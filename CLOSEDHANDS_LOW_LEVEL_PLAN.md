# ClosedHands — Full Low-Level Implementation Plan

**Goal:** Personal fork of the published grok-build / harness tree that is **as solid, good, fast, and accurate as Grok** for software engineering in a workspace. Everything a Grok-class coding agent can do in-repo must be possible here.

**Non-goals (v1):** Multiplayer, marketplace, cloud telemetry product surface, full crate rename of every `xai-grok-*` package, Python OpenHands rewrite.

**North star loop:**
```
user task → plan → tools (edit/shell/search/…) → observe → fix → verify → done
```
Multi-agent is an **escalation**, not the default path.

**Base tree:** `aalhadxx/closedhands` @ main (derived from monorepo publish + your commits).  
**Primary binary package today:** `crates/codegen/xai-grok-pager-bin` → artifact `xai-grok-pager` (must become `closedhands`).  
**Config root:** `~/.closedhands` (and project `.closedhands/`).

---

## 0. Security freeze (Day 0 — before any feature work)

### 0.1 Rotate & revoke (human actions)
| Secret | Action |
|--------|--------|
| Ollama Cloud API key in `closedhands.toml`, `.closedhands/config.toml`, `CLOSEDHANDS_PLAN.md` | Revoke in Ollama dashboard; create new; **never commit** |
| Mangools `x-access-token` in `closedhands.toml`, `sidecar/mcp_mangools.py` | Revoke in Mangools; create new |
| Any GitHub PAT pasted in chat / CI | Revoke at github.com/settings/tokens immediately |
| Any other key ever committed | Assume burned; rotate |

### 0.2 Remove secrets from tree
| File | Change |
|------|--------|
| `closedhands.toml` | Delete from repo **or** replace with `closedhands.toml.example` using placeholders only |
| `.closedhands/config.toml` | Same → `config.toml.example` |
| `sidecar/mcp_mangools.py` | Read token from `os.environ["MANGOOLS_TOKEN"]` or config; no literal |
| `CLOSEDHANDS_PLAN.md` / this plan | Examples only: `api_key = "ollama-..."` placeholders |
| `.gitignore` | Append: |
| | `closedhands.toml` |
| | `.closedhands/config.toml` |
| | `.closedhands/auth.json` |
| | `.closedhands/**/credentials*` |
| | `.env` |
| | `.env.*` |
| | `!.closedhands/config.toml.example` |
| | `!.closedhands/personas/**` |

### 0.3 History purge
```bash
# After local clean commit removing secrets from HEAD:
git filter-repo --invert-paths \
  --path closedhands.toml \
  --path .closedhands/config.toml \
  --path sidecar/mcp_mangools.py
# Or use BFG on blobs matching key prefixes.
# Force-push main only after purge; notify if anyone else cloned.
```
Re-add **example** files without secrets in a fresh commit.

### 0.4 Acceptance
- [ ] `git log -p -S '<old-ollama-key-prefix>' --all` empty (old Ollama key gone from history)
- [ ] `git log -p -S '<old-mangools-token-prefix>' --all` empty (Mangools)
- [ ] `git log -p -S 'ghp_' --all` empty (any leaked PAT patterns)
- [ ] Fresh clone has zero live secrets; only `*.example` templates

---

## 1. Product contract (what “done” means)

### 1.1 User-visible binary
| Item | Target |
|------|--------|
| CLI name | `closedhands` / `closedhands.exe` |
| Version | `closedhands --version` |
| First run | No forced xAI browser auth; API key or Ollama login path only |
| Config | `~/.closedhands/config.toml` + optional project `.closedhands/` |
| Default model | `kimi-k2.7-code:cloud` via `https://ollama.com/v1` (overridable) |

### 1.2 Default agent mode (speed + accuracy)
Single agent, **full toolset**, verify loop. This is the Grok-class path.

### 1.3 Escalation modes
| Slash | When | Behavior |
|-------|------|----------|
| (default chat) | Always | Single agent + tools |
| `/debate <brief>` | Hard reasoning / design | Harper ∥ Benjamin ∥ Lucas → leader synthesis |
| `/closedhands <prompt>` | Full product build pipeline | Tool-using stages + real verify (or remove until ready) |

### 1.4 Capability parity checklist (Grok-class)
| Capability | Required in ClosedHands |
|------------|-------------------------|
| Multi-file read/edit | file + apply_patch tools |
| Shell / tests / build | terminal tool + sandbox |
| Web search / fetch | search tools wired to provider |
| Subagents / parallel work | `ChannelBackend` + personas |
| Long sessions | compaction + resume (upstream) |
| Workspace trust | folder trust + worktrees (later) |
| Headless / CI | stdio / headless entry (upstream) |
| ACP editor embed | keep ACP; rename user strings only |

---

## 2. Repository map (where to touch)

### 2.1 Hot paths for v1
| Area | Path |
|------|------|
| Binary / main | `crates/codegen/xai-grok-pager-bin/` |
| TUI | `crates/codegen/xai-grok-pager/` |
| Agent runtime / slash | `crates/codegen/xai-grok-shell/` |
| Slash dispatch | `.../session/slash_commands.rs` |
| Slash exec + pipelines | `.../session/acp_session_impl/slash_exec.rs` |
| Debate | `.../session/acp_session_impl/debate.rs` |
| Session module glue | `.../session/acp_session.rs` |
| Config load / paths | `crates/codegen/xai-grok-config/`, `xai-grok-config-types/` |
| Models default | `crates/codegen/xai-grok-models/default_models.json` |
| Sampler / HTTP | `crates/codegen/xai-grok-sampler/` |
| Auth | `crates/codegen/xai-grok-auth/`, pager user-guide `02-authentication.md` |
| Tools / task backend | `crates/codegen/xai-grok-tools/src/implementations/grok_build/task/` |
| Personas | `.closedhands/personas/*.toml` |
| CI | `.github/workflows/` |
| Docs | `README.md`, `SECURITY.md`, `CONTRIBUTING.md` |
| Hermetic protoc | `bin/protoc` (restore Unix DotSlash; keep Windows separate) |

### 2.2 Do not touch in v1 (unless compile forces it)
- Full rename of all `xai-grok-*` crate directories  
- Mixpanel / telemetry **deletion** (disable by default instead)  
- Mermaid third_party stack  
- Bazel-only paths  

---

## 3. Phase A — Identity & config (solid foundation)

**Duration:** 1–2 days  
**Depends on:** §0 complete  

### A.1 Config schema (canonical)

Create `closedhands.toml.example` and `.closedhands/config.toml.example`:

```toml
[llm]
# Prefer env CLOSEDHANDS_API_KEY over committing secrets
api_key = ""   # or set via environment
base_url = "https://ollama.com/v1"
model = "kimi-k2.7-code:cloud"

[agent]
default_mode = "single"          # single | debate
subagents_enabled = true
max_debate_rounds = 4
debate_round_timeout_secs = 120

[tools]
shell = true
network = true
# capability defaults for subagents
default_capability_mode = "all"

[telemetry]
enabled = false
```

**Env precedence (implement if missing):**
1. `CLOSEDHANDS_API_KEY` / `OLLAMA_API_KEY`  
2. `CLOSEDHANDS_BASE_URL`  
3. `CLOSEDHANDS_MODEL`  
4. File config  
5. Built-in defaults  

### A.2 Path / home resolution
| Symbol / API | Action |
|--------------|--------|
| `GROK_HOME` | Accept as deprecated alias → `CLOSEDHANDS_HOME` |
| `grok_home()` helpers | Resolve: `CLOSEDHANDS_HOME` > `GROK_HOME` > `~/.closedhands` |
| Files: `xai-grok-config/src/paths.rs`, `xai-grok-tools/.../grok_home`, `xai-fast-worktree` | Single resolution function; unit tests for precedence |

### A.3 Binary rename (user-facing)
| Step | Detail |
|------|--------|
| Add bin name | In `xai-grok-pager-bin/Cargo.toml`: `[[bin]] name = "closedhands"` (keep or alias old name during transition) |
| `default-run` | `closedhands` |
| CI artifact | Upload `closedhands.exe` / `closedhands` |
| Install docs | README only documents `closedhands` |
| `authors` | `["ClosedHands Contributors"]` on binary package |

Optional: `[[bin]] name = "xai-grok-pager"` as hidden alias for one release.

### A.4 Auth path cleanup
| File / area | Change |
|-------------|--------|
| `02-authentication.md` | Ollama + API key only; remove `console.x.ai`, SuperGrok, wrong HackerOne |
| `SECURITY.md` | Personal contact or private security email — **not** `hackerone.com/x` |
| Browser login | If Ollama device flow exists, keep; else API-key-first |
| Defaults in code | `cli_models.rs` already has ollama.com — audit remaining xAI OAuth defaults |

### A.5 Acceptance A
- [ ] `cargo build -p xai-grok-pager-bin --release` produces `closedhands`  
- [ ] Config without secrets loads; missing key → clear error  
- [ ] `SECURITY.md` / auth guide have zero xAI security endpoints  
- [ ] No live secrets in tree  

---

## 4. Phase B — Default single-agent loop (fast + accurate)

**Duration:** 2–4 days  
**Depends on:** A  

### B.1 Prove tools on Ollama path
Manual + automated smoke:

```text
1. closedhands (or cargo run -p xai-grok-pager-bin)
2. Prompt: "Create /tmp/ch-smoke/hello.txt with hello and run cat on it"
3. Expect: file tool + shell tool, real file exists
4. Prompt: "In this repo, run cargo check -p xai-grok-config and fix any error you introduce"
5. Expect: edit → check → stop on green
```

**If tools fail:** trace `xai-grok-sampler` auth headers (Bearer vs x-api-key), `base_url` join, model id in `default_models.json`.

### B.2 System prompt / agent identity
| File | Change |
|------|--------|
| `xai-grok-agent/templates/prompt.md` | Identity = ClosedHands coding agent; prefer tools; verify with commands |
| `subagent_prompt.md` | Same; no “Grok” product name |
| Agent display names | `session_config.rs` / models list already partially updated — finish |

### B.3 Accuracy harness (non-negotiable)
Add project-local skill or slash later; for v1 enforce in default prompt:

1. After code changes → run project’s test/build command  
2. Do not claim pass without tool output  
3. Prefer minimal diffs  

Optional code: post-turn hook in shell that surfaces “no verification run” warning when edits occurred without shell tool (feature flag).

### B.4 Disable product noise by default
| Feature | Default |
|---------|---------|
| Telemetry / mixpanel | off |
| Marketplace / update upsell | off or no-op |
| Announcements to x.ai | off |
| Share session to x.ai | off or stub |

### B.5 Acceptance B
- [ ] Cold start → tool edit + shell in < N seconds first tool call (measure)  
- [ ] End-to-end “fix compile error you introduced” succeeds 3/3 trials  
- [ ] No xAI network calls in default config (tcpdump or log audit)  

---

## 5. Phase C — Multi-agent: `/debate` (depth)

**Duration:** 2–3 days  
**Depends on:** B (subagents proven)  

### C.1 Files
- `crates/codegen/xai-grok-shell/src/session/acp_session_impl/debate.rs`  
- `slash_exec.rs` → `BuiltinAction::ClosedHandsDebate`  
- `slash_commands.rs` registration  
- `.closedhands/personas/{harper,benjamin,lucas}.toml`  

### C.2 Behavioral spec
```
Input: brief
Round r = 0..max_rounds-1:
  spawn harper, benjamin, lucas in parallel (same parent_session_id)
  each gets: brief + transcript + role instructions from persona file
  collect outputs; parse `To [All|Name]:` lines into DebateMessage
  if any message starts with CONSENSUS: → break
  if no messages → break
Leader:
  persona leader / grok-leader renamed to "closedhands-leader"
  synthesis prompt WITHOUT branding "You are Grok"
  parent_session_id = real session id (not empty string)  # fix if still empty
Output: leader final text + optional transcript artifact
```

### C.3 Low-level code changes
| Location | Change |
|----------|--------|
| `build_agent_prompt` | “You are {agent} on the ClosedHands team. The leader synthesizes the final answer.” |
| `build_leader_prompt` | “You are the ClosedHands leader…” — drop Grok |
| `DebateConfig::default` | `max_rounds: 4` (latency); configurable from config.toml |
| Leader `parent_session_id` | Use `self.config.session_id` not `String::new()` |
| Persona loading | Prefer file instructions from `.closedhands/personas/{name}.toml` over hardcoded only |
| Capability | Explore-like: Harper network+read; Benjamin read; Lucas read; Leader read — **optional** tool allowlists via `runtime_overrides.capability_mode` |
| Isolation | v1 shared workspace; v2 worktrees (`isolation` override) |
| Tests | Keep extract/consensus unit tests; add test for leader prompt not containing "Grok" |

### C.4 Latency / cost guards
- Default max_rounds = 3–4  
- `round_timeout_secs` = 90–120  
- Abort debate if single-agent would suffice: optional classifier later  
- Log token usage per agent if sampler exposes it  

### C.5 Acceptance C
- [ ] `/debate design a rate limiter` returns coherent synthesis  
- [ ] Parallelism: three spawns overlap (trace logs)  
- [ ] No “Grok” in debate prompts  
- [ ] Unit tests pass: `cargo test -p xai-grok-shell debate` (or package that owns module)  

---

## 6. Phase D — `/closedhands` pipeline (optional product path)

**Duration:** 3–5 days  
**Depends on:** B; ideally C  

### D.1 Decision gate
If you need **speed**: default chat only; demote `/closedhands` until stages use tools.  
If you need **full build factory**: implement D.2.

### D.2 Stage machine (tool-using)

| # | Stage | Persona | Must use tools? | Exit criterion |
|---|-------|---------|-----------------|----------------|
| 1 | Research | harper | yes (search/read) | `01_evidence.md` + citations |
| 2 | Logic | benjamin | read evidence | `02_logic.md` |
| 3 | Challenge | lucas | read | `03_challenge.md` |
| 4 | Synthesis | leader | no tools ok | `04_synthesis.md` |
| 5 | Implement | coder | **yes** edit/shell | files on disk, not only markdown |
| 6 | Review | reviewer | read diff | `06_review.md` tagged issues |
| 7 | Fix | fixer | **yes** edit | issues addressed |
| 8 | Test | tester | **yes** shell | real test command output in `08_tests.md` |
| 9 | Deploy | deployer | **yes** only if `vercel`/config present | URL or skip with reason |

### D.3 Implementation notes (`run_closedhands_pipeline`)
| Current | Target |
|---------|--------|
| Prompt-only spawns | Spawns with capability_mode allowing tools; prompts **require** tool use for 5/7/8 |
| `05_code.md` only | Coder writes real project files under cwd or `artifacts/app` |
| Fake deploy | If no Vercel token: stage returns `SKIPPED: no deploy credentials` |
| Fail-fast | On stage error: stop; don’t continue to Deployer |
| Progress UI | Keep host turn messages |
| Artifacts | Keep `.closedhands/artifacts/` |

### D.4 Persona TOML upgrades
Add optional fields (extend parser in `config/mod.rs` personas loader):

```toml
instructions = """..."""
tools = ["read", "search"]   # or capability_mode = "read-only"
model = "kimi-k2.7-code:cloud"
```

### D.5 Acceptance D
- [ ] `/closedhands todo app` produces runnable files + test output on disk  
- [ ] Pipeline stops on test failure  
- [ ] No stage claims success without tool evidence when tools required  

---

## 7. Phase E — Build, CI, protoc, ship

**Duration:** 1–2 days  
**Depends on:** A (binary name); can parallelize with B  

### E.1 Restore Linux hermetic protoc
| Action | Detail |
|--------|--------|
| Restore `bin/protoc` DotSlash wrapper from pre-`b2a9b96` or upstream | Unix builds work |
| Keep `protoc.exe` only on Windows CI path or document `PROTOC` env | |
| README | Accurate build requirements |

### E.2 Workflows
**`.github/workflows/ci.yml`**
```yaml
on: [push, pull_request]
jobs:
  linux:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - dtolnay/rust-toolchain (1.92.0)
      - install protoc / dotslash as needed
      - cargo build -p xai-grok-pager-bin
      - cargo test -p xai-grok-shell --debate-or-relevant
      # optional: cargo check -p xai-grok-config
  windows:
    # existing cross-compile job; artifact closedhands.exe
```

### E.3 README truth table
| Claim | Must be true |
|-------|--------------|
| Prebuilt releases | Only if GH Releases exist; else “build from source” |
| Binary name | `closedhands` |
| Auth | API key / Ollama |
| Screenshot | Real terminal capture, not placeholder |

### E.4 Acceptance E
- [ ] Linux CI green on main  
- [ ] Windows artifact named `closedhands.exe`  
- [ ] Fresh clone → documented steps → TUI starts  

---

## 8. Phase F — Performance & accuracy tuning

**Duration:** ongoing after B  

### F.1 Speed
| Lever | Action |
|-------|--------|
| Default path | Never auto-run 9-stage pipeline |
| Debate | Cap rounds; parallel agents only |
| Model | Single primary code model; don’t fan-out 8×  
| Compaction | Keep upstream compaction thresholds sane for 256k ctx |
| Cold start | Lazy-load heavy features |

### F.2 Accuracy
| Lever | Action |
|-------|--------|
| Verify loop | Shell after edits |
| Smaller diffs | Prompt + reviewer tags |
| Debate | Use for design; implement in single agent after synthesis |
| Regression | Save 10 tasks; weekly pass-rate |

### F.3 Golden tasks (personal bench)
1. Add function + unit test in a sample Rust crate  
2. Fix failing test  
3. Multi-file refactor rename  
4. `/debate` API design → single agent implements  
5. Search docs question with citation  

Track: pass/fail, tool calls, wall time, tokens.

---

## 9. Phase G — Deferred (explicitly not v1)

| Item | Why defer |
|------|-----------|
| Rename all crates `xai-grok-*` → `closedhands-*` | Massive churn; breaks sync |
| Python `moed.py` + OpenHands SDK | Duplicates Rust runtime; latency/complexity |
| Full telemetry excision | Disable default enough |
| Git worktree isolation per agent | Need after parallel write races hurt |
| Marketplace / voice / share | Out of scope |
| Upstream monorepo sync automation | Manual pin `SOURCE_REV` until hard-fork decision |

**Hard-fork decision point:** after Phase E, stop merging “Synced from monorepo” or script controlled merges with conflict policy.

---

## 10. File-level task backlog (execution order)

### Sprint 0 — Security
1. Rotate all keys / PATs  
2. Replace secret files with examples  
3. Update `.gitignore`  
4. `git filter-repo` / BFG  
5. Force-push; verify clone  

### Sprint 1 — Identity
6. Binary `closedhands` + authors  
7. Config examples + env precedence  
8. Home path alias  
9. SECURITY + auth docs  
10. README honest  

### Sprint 2 — Agent core
11. Ollama smoke tools  
12. Prompt identity ClosedHands  
13. Telemetry off default  
14. Golden task 1–3 pass  

### Sprint 3 — Debate
15. Prompt rebrand in `debate.rs`  
16. session_id on leader  
17. Persona file injection  
18. Config caps for rounds  
19. Tests  

### Sprint 4 — Pipeline or cut
20. Either toolify `/closedhands` stages **or** document as experimental and hide from README  
21. Fail-fast + real test stage  

### Sprint 5 — CI/ship
22. Restore `bin/protoc`  
23. Linux CI  
24. Windows artifact name  
25. Optional release workflow  

---

## 11. Testing matrix

| Level | Command / action | Phase |
|-------|------------------|-------|
| Unit | `debate` extract/consensus tests | C |
| Unit | config path precedence | A |
| Package | `cargo test -p xai-grok-config` | A |
| Package | `cargo check -p xai-grok-pager-bin` | A–E |
| Integration | Manual TUI tool smoke | B |
| Integration | `/debate` manual | C |
| Integration | `/closedhands` mini app | D |
| CI | build + selected tests | E |

---

## 12. Risk register

| Risk | Impact | Mitigation |
|------|--------|------------|
| Secrets already scraped | Account abuse | Rotate immediately; purge history |
| Ollama model weak on tools | Accuracy < Grok | Tool-use prompt tuning; try alternate model id; reduce parallel agents |
| Subagents disabled at runtime | `/debate` dead | Ensure feature flags / `subagent_event_tx` always set in interactive mode |
| Brand strip incomplete | Confusion | User-facing only in v1; crates keep names |
| Windows-only protoc | Linux build break | Restore DotSlash protoc |
| 9-stage pipeline latency | Feels worse than Grok | Default single-agent |
| Upstream sync overwrites | Lost forks | Hard-fork or merge policy |
| ACP `x.ai/*` method names | Cosmetic / compat | Keep wire names; hide from UI |

---

## 13. Definition of “as good as Grok” (exit criteria)

All must pass on your machine + clean CI:

1. **Solid:** no secrets in git; config local; binary `closedhands`; builds Linux+Windows  
2. **Good:** TUI usable; docs match behavior; ClosedHands identity consistent in user paths  
3. **Fast:** default path = single agent; p50 simple edit+test task within acceptable wall time (set your budget, e.g. ≤ 2× interactive ChatGPT/Grok coding for same task)  
4. **Accurate:** ≥ 8/10 golden tasks pass with real tool verification  
5. **Capable:** edit, shell, search, subagents, `/debate` synthesis all work on Ollama  
6. **Honest:** README/plan match code; no fake Deployer success  

---

## 14. Relationship to existing `CLOSEDHANDS_PLAN.md`

| Old plan | This plan |
|----------|-----------|
| Python MOE sidecar first | **Defer**; Rust single-agent first |
| Phase 0 brand strip as primary | Brand **after/with** secrets + binary; depth over string replace |
| OpenHands SDK | Optional later; not on critical path |
| Path C debate | Keep; fix identity + session_id + caps |
| 8-agent pipeline | Only if tool-using; else experimental |

**Replace operational guidance with this document.** Keep old plan as historical notes only after scrubbing secrets from it.

---

## 15. Immediate next commands (after key rotation)

```bash
cd /path/to/closedhands

# 1) Working tree clean of secrets
cp closedhands.toml closedhands.toml.example  # then scrub values
# edit example to placeholders; remove real closedhands.toml from tracking

# 2) gitignore update + commit on a private branch first

# 3) history purge (destructive)
# git filter-repo ...

# 4) verify
git grep -i 'api_key.*=' -- '*.toml' '*.py' '*.md' || true

# 5) build
cargo build -p xai-grok-pager-bin
```

---

## 16. Success snapshot

When finished, a stranger cloning your **secret-free** repo can:

```bash
export CLOSEDHANDS_API_KEY=...
cargo run -p xai-grok-pager-bin --release
# binary: closedhands
# default: single agent, tools on, Ollama
# /debate for hard problems
# accuracy from real test runs, not persona fan-fiction
```

That is the full low-level plan: security → identity → single-agent parity → debate depth → optional pipeline → CI/ship → tune.

**End of plan.**
