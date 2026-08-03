use std::collections::HashMap;
use tokio::time::{timeout, Duration};

use xai_grok_tools::implementations::grok_build::task::backend::{ChannelBackend, SubagentBackend};
use xai_grok_tools::implementations::grok_build::task::types::SubagentRequest;

/// Message in the shared debate transcript.
#[derive(Debug, Clone)]
pub struct DebateMessage {
    pub from: String,
    pub to: Vec<String>,
    pub content: String,
    pub round: usize,
}

/// Configuration for the debate pipeline.
pub struct DebateConfig {
    pub brief: String,
    pub session_id: String,
    pub max_rounds: usize,
    pub round_timeout_secs: u64,
    pub agents: Vec<String>,
}

impl Default for DebateConfig {
    fn default() -> Self {
        Self {
            brief: String::new(),
            session_id: String::new(),
            max_rounds: 4,
            round_timeout_secs: 120,
            agents: vec![
                "harper".to_string(),
                "benjamin".to_string(),
                "lucas".to_string(),
            ],
        }
    }
}

/// Round-based multi-agent debate orchestrator.
///
/// Spawns specialists in parallel each round, collects their chatroom-style
/// messages, injects the transcript into the next round, and forces a leader
/// synthesis when consensus is reached or rounds exhaust.
pub struct DebatePipeline<'a> {
    backend: &'a ChannelBackend,
    config: DebateConfig,
}

impl<'a> DebatePipeline<'a> {
    pub fn new(backend: &'a ChannelBackend, config: DebateConfig) -> Self {
        Self { backend, config }
    }

    /// Execute the full debate → synthesis pipeline.
    pub async fn execute(&self) -> anyhow::Result<String> {
        let mut transcript: Vec<DebateMessage> = Vec::new();

        for round in 0..self.config.max_rounds {
            tracing::info!(round, "starting debate round");

            // Spawn all agents in parallel for this round.
            let mut handles: HashMap<String, _> = HashMap::new();
            for agent in &self.config.agents {
                let prompt = self.build_agent_prompt(agent, &transcript, round);
                let id = format!("closedhands-debate-{}-{}-{}", agent, round, uuid::Uuid::new_v4());
                let req = SubagentRequest {
                    id,
                    prompt,
                    description: format!("Debate round {round} for {agent}"),
                    subagent_type: "general-purpose".to_string(),
                    parent_session_id: self.config.session_id.clone(),
                    parent_prompt_id: None,
                    resume_from: None,
                    cwd: None,
                    runtime_overrides: xai_grok_tools::implementations::grok_build::task::types::SubagentRuntimeOverrides {
                        persona: Some(agent.clone()),
                        ..Default::default()
                    },
                    run_in_background: false,
                    surface_completion: true,
                    await_to_completion: true,
                    fork_context: false,
                    owner: xai_grok_tools::implementations::grok_build::task::types::SubagentOwner::Task,
                    cancel_token: tokio_util::sync::CancellationToken::new(),
                };
                handles.insert(agent.clone(), self.backend.spawn(req));
            }

            // Await all results concurrently (true parallelism).
            let mut round_messages: Vec<DebateMessage> = Vec::new();
            for (agent, handle) in handles {
                let deadline = Duration::from_secs(self.config.round_timeout_secs);
                match timeout(deadline, handle).await {
                    Ok(Ok(result)) => {
                        let output = result.output.to_string();
                        let msgs = Self::extract_messages(&output, &agent, round);
                        round_messages.extend(msgs);
                    }
                    Ok(Err(e)) => {
                        tracing::warn!(%agent, error = %e, "agent failed");
                    }
                    Err(_) => {
                        tracing::warn!(%agent, "agent timed out");
                    }
                }
            }

            if round_messages.is_empty() {
                tracing::info!(round, "no new messages — ending debate");
                break;
            }

            transcript.extend(round_messages);

            if Self::consensus_reached(&transcript) {
                tracing::info!(round, "consensus detected");
                break;
            }
        }

        // Leader synthesis.
        let leader_prompt = self.build_leader_prompt(&transcript);
        let leader_id = format!("closedhands-debate-leader-{}", uuid::Uuid::new_v4());
        let leader_req = SubagentRequest {
            id: leader_id,
            prompt: leader_prompt,
            description: "Leader synthesis for debate".to_string(),
            subagent_type: "general-purpose".to_string(),
            parent_session_id: self.config.session_id.clone(),
            parent_prompt_id: None,
            resume_from: None,
            cwd: None,
            runtime_overrides: xai_grok_tools::implementations::grok_build::task::types::SubagentRuntimeOverrides {
                persona: Some("leader".to_string()),
                ..Default::default()
            },
            run_in_background: false,
            surface_completion: true,
            await_to_completion: true,
            fork_context: false,
            owner: xai_grok_tools::implementations::grok_build::task::types::SubagentOwner::Task,
            cancel_token: tokio_util::sync::CancellationToken::new(),
        };

        let deadline = Duration::from_secs(self.config.round_timeout_secs * 2);
        let answer = timeout(deadline, self.backend.spawn(leader_req))
            .await
            .map_err(|_| anyhow::anyhow!("leader synthesis timed out"))??;

        Ok(answer.output.to_string())
    }

    /// Build the prompt for a specialist in a given round.
    fn build_agent_prompt(
        &self,
        agent: &str,
        transcript: &[DebateMessage],
        round: usize,
    ) -> String {
        let history = Self::format_transcript(transcript);
        let brief = &self.config.brief;

        format!(
            r#"# Multi-Agent Debate — Round {round}

You are {agent}. You are collaborating with Harper, Benjamin, and Lucas.
Grok (the leader) will synthesize the final answer. Your job is to contribute your unique perspective so the team reaches the best possible conclusion.

## Brief
{brief}

## Previous Messages
{history}

## Instructions
1. Analyze the brief from your perspective.
2. Communicate using the chatroom format below.
3. Challenge weak claims, cite evidence, and build on others' ideas.
4. If you believe consensus is reached, say `CONSENSUS: <one-sentence summary>`.

## Chatroom Format
Write exactly one message per line in this format:
  To [All]: <your contribution>
  To [Name]: <directed response>

After your message(s), include a `### Analysis` section with your private reasoning (not broadcast)."#,
            round = round,
            agent = agent,
            brief = brief,
            history = if history.is_empty() {
                "(none yet — you are opening the debate.)".to_string()
            } else {
                history
            },
        )
    }

    /// Build the leader synthesis prompt.
    fn build_leader_prompt(&self, transcript: &[DebateMessage]) -> String {
        let history = Self::format_transcript(transcript);
        let brief = &self.config.brief;

        format!(
            r#"# Leader Synthesis

You are the ClosedHands leader. Synthesize the following multi-agent debate into a single, coherent, high-quality final answer.

## Brief
{brief}

## Full Transcript
{history}

## Instructions
- Resolve conflicts using the strongest evidence.
- Preserve unique insights from each specialist.
- Drop weak or unsupported claims.
- Produce the final answer directly. No meta-commentary. No "As leader..." preamble."#,
            brief = brief,
            history = history,
        )
    }

    /// Parse agent output for `To [Name]: message` lines.
    fn extract_messages(output: &str, from: &str, round: usize) -> Vec<DebateMessage> {
        let mut messages = Vec::new();

        for line in output.lines() {
            let line = line.trim();
            if let Some(stripped) = line.strip_prefix("To ") {
                if let Some((targets_str, content)) = stripped.split_once(':') {
                    let targets: Vec<String> = targets_str
                        .split(',')
                        .map(|s| s.trim().to_lowercase())
                        .collect();
                    messages.push(DebateMessage {
                        from: from.to_string(),
                        to: targets,
                        content: content.trim().to_string(),
                        round,
                    });
                }
            }
        }

        messages
    }

    /// Check if any agent has declared consensus in the last round.
    fn consensus_reached(transcript: &[DebateMessage]) -> bool {
        transcript
            .iter()
            .rev()
            .take(5)
            .any(|m| m.content.to_uppercase().starts_with("CONSENSUS:"))
    }

    /// Format transcript as human-readable history.
    fn format_transcript(transcript: &[DebateMessage]) -> String {
        if transcript.is_empty() {
            return String::new();
        }

        transcript
            .iter()
            .map(|m| {
                let to = if m.to.contains(&"all".to_string()) {
                    "All".to_string()
                } else {
                    m.to.join(", ")
                };
                format!("[{} → {}] {}", m.from, to, m.content)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_basic_messages() {
        let output = r#"
To All: I think we should use tokio::join for parallelism.
To Benjamin: Can you verify the type constraints?
### Analysis
This is my private reasoning.
"#;
        let msgs = DebatePipeline::extract_messages(output, "harper", 0);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].to, vec!["all"]);
        assert_eq!(msgs[0].content, "I think we should use tokio::join for parallelism.");
        assert_eq!(msgs[1].to, vec!["benjamin"]);
    }

    #[test]
    fn consensus_detection() {
        let msgs = vec![
            DebateMessage {
                from: "lucas".into(),
                to: vec!["all".into()],
                content: "CONSENSUS: Use round-based debate with transcript injection.".into(),
                round: 2,
            },
        ];
        assert!(DebatePipeline::consensus_reached(&msgs));
    }

    #[test]
    fn format_transcript_empty() {
        assert_eq!(DebatePipeline::format_transcript(&[]), "");
    }
}
