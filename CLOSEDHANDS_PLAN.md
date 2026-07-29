# ClosedHands — Low-Level Implementation Plan

**Status:** Draft v1  
**Base:** Fork of `grok-build` (Apache 2.0) + `moe.py` logic + OpenHands SDK  
**Backend:** Ollama Cloud (`https://ollama.com/v1`)  
**Primary Model:** `kimi-k2.7-code:cloud` (all phases)  
**Branding:** ClosedHands everywhere. No SpaceXAI. No telemetry. No login wall.

---

## 0. Architecture Decision: Hybrid Rust + Python

| Layer | Language | Why |
|-------|----------|-----|
| **TUI / PTY / Markdown / Workspace** | Rust (from grok-build) | 89 crates of proven terminal UI. Don't rewrite. |
| **MOE Brain / Orchestration** | Python (from moe.py + OpenHands SDK) | Agent logic, LLM calls, artifact schemas already work. |
| **IPC** | HTTP localhost (JSON) | Fast to wire, easy to debug with `curl`. |

**Why not pure Rust?** Re-implementing OpenHands TaskToolSet + MOE pipeline in Rust = 3+ months.  
**Why not pure Python?** You want Grok's TUI quality (Ratatui + PTY + markdown). Python `textual` is close but not native-terminal.

---

## 1. Phase Breakdown

### Phase 0: Fork & Rebrand (Days 1–2)

**Repo setup:**
```bash
cd D:\coding\ai-ide-research
cp -r grok-build closedhands
cd closedhands
git remote remove origin
git init
git remote add origin https://github.com/aalhadxx/closedhands.git
```

**Global string replacements:**
- `grok` → `closedhands` (binary name, config paths)
- `Grok` → `ClosedHands` (UI strings, docs)
- `SpaceXAI` → remove / replace with author name
- `xai-grok` → `ch` (crate prefixes)

**Strip non-essential crates:**
| Crate | Action | Why |
|-------|--------|-----|
| `xai-grok-auth` | Replace | Remove OAuth/device-auth. Load `XAI_API_KEY` from env or `~/.closedhands/config.toml`. |
| `xai-grok-telemetry` | Delete | No tracking. |
| `xai-grok-announcements` | Delete | No marketing. |
| `xai-grok-mixpanel` | Delete | No analytics. |
| `xai-grok-update` | Delete | No auto-updater (for now). |
| `xai-grok-voice` | Delete | Out of scope. |
| `xai-grok-plugin-marketplace` | Delete | Out of scope. |

**Keep essential crates:**
- `xai-grok-pager*` (TUI renderer)
- `xai-ratatui-inline`, `xai-ratatui-textarea` (widgets)
- `xai-grok-markdown`, `xai-grok-markdown-core` (MD → TUI)
- `xai-grok-shell*`, `ptyctl*` (PTY / shell)
- `xai-grok-workspace*` (file ops)
- `xai-grok-config` (rework for Ollama)
- `xai-grok-http` (rework for Ollama)
- `xai-grok-models` (rework for Ollama)
- `xai-grok-memory` (keep for conversation state)
- `xai-grok-tools`, `xai-grok-tools-api` (keep tool defs)
- `xai-tty-utils`, `xai-grok-pager-render` (terminal glue)

**Config file:**
```toml
# ~/.closedhands/config.toml
[llm]
api_key = "fde3a83697d24f4681940a9d9ff57e89.tdBZqMcqhTVxUKUbEPDWc1lE"
base_url = "https://ollama.com/v1"
model = "kimi-k2.7-code:cloud"

[moe]
plan_model = "deepseek-v4-pro:cloud"
code_model = "kimi-k2.7-code:cloud"
review_model = "kimi-k2.7-code:cloud"
fast_model = "deepseek-v4-flash:cloud"
max_retries = 2
```

---

### Phase 1: Backend Swap — Rust TUI → Ollama (Days 3–5)

**Goal:** The Rust TUI can chat with Ollama Cloud instead of SpaceXAI.

**Files to modify:**

1. **`crates/codegen/xai-grok-http/src/lib.rs`** (or equivalent)
   - Rip out xAI-specific headers, auth flows, base URL construction.
   - Replace with generic OpenAI-compatible client:
     - `base_url = config.base_url` (default `https://ollama.com/v1`)
     - `Authorization: Bearer {api_key}`
     - Endpoint: `/chat/completions`
   - Reuse `async-openai` dependency (already in workspace) or `reqwest` directly.

2. **`crates/codegen/xai-grok-models/src/lib.rs`**
   - Replace model enum with Ollama model strings.
   - Default: `kimi-k2.7-code:cloud`.
   - Remove Grok 4.5, Grok 3, etc.

3. **`crates/codegen/xai-grok-auth/src/lib.rs`**
   - Delete OAuth, device auth, browser login.
   - New logic: read `XAI_API_KEY` env var → read `~/.closedhands/config.toml` → fail if missing.

4. **`crates/codegen/xai-grok-config/src/lib.rs`**
   - Parse new TOML schema (see Phase 0).
   - Expose `api_key`, `base_url`, `model` to rest of app.

**Validation:**
```bash
cd closedhands
cargo build --release
./target/release/closedhands -p "hi. reply pong"
# Expect: pong from kimi-k2.7-code:cloud via Ollama
```

---

### Phase 2: MOE Sidecar — Python Daemon (Days 6–10)

**Goal:** The Rust TUI spawns a Python daemon that runs the Plan→Code→Review→Fix pipeline.

**Why a sidecar?** `moe.py` already works. OpenHands SDK is Python. Don't rewrite in Rust yet.

**Architecture:**
```
┌─────────────────────────────────────────────┐
│  Rust TUI (closedhands)                     │
│  - Ratatui rendering                        │
│  - PTY shell                                │
│  - Keyboard input                           │
└──────┬──────────────────────────────────────┘
       │ HTTP POST localhost:8787/task
       │ JSON: { "task": "...", "workspace": "..." }
       ▼
┌─────────────────────────────────────────────┐
│  Python Daemon (closedhands-moed)            │
│  - FastAPI or Flask (single file)            │
│  - Runs moe.py pipeline                     │
│  - Streams SSE or returns JSON artifacts    │
└──────┬──────────────────────────────────────┘
       │ HTTP POST https://ollama.com/v1/chat/completions
       ▼
           Ollama Cloud
```

**Python daemon spec:** `D:\coding\ai-ide-research\closedhands\sidecar\moed.py`

```python
from fastapi import FastAPI
from pydantic import BaseModel
import asyncio
import json
import urllib.request

app = FastAPI()

CONFIG = {
    "api_key": "...",
    "base_url": "https://ollama.com/v1",
    "model": "kimi-k2.7-code:cloud",
}

class TaskRequest(BaseModel):
    task: str
    workspace: str

@app.post("/task")
async def run_task(req: TaskRequest):
    # Run Plan → Code → Review → Fix
    # Stream progress via SSE or return final artifacts
    ...

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="127.0.0.1", port=8787)
```

**Rust TUI changes:**
- New crate: `ch-moe-client` (or add to `xai-grok-agent`)
- On user submit: `POST localhost:8787/task` with JSON body.
- Read SSE stream or poll for artifacts.
- Render artifacts in TUI panels.

**TUI Panels (new):**
| Panel | Content |
|-------|---------|
| `PlanPanel` | Editable markdown from `01_plan.md` |
| `AgentPanel` | Timeline: `[PLAN] => [CODE] => [REVIEW] => [FIX]` |
| `CodePanel` | Syntax-highlighted code blocks from `04_fixed.md` |
| `ReviewPanel` | Severity chips (Critical / Warning / Info) |
| `TerminalPanel` | Existing PTY for running tests |

---

### Phase 3: OpenHands SDK Integration (Days 11–14)

**Goal:** Replace bare `urllib.request` in `moed.py` with OpenHands SDK primitives.

**What to use from OpenHands SDK:**

| SDK Primitive | MOE Use |
|---------------|---------|
| `Agent` + `Conversation` | Each specialist = agent with own EventLog |
| `Skills` | Expert system prompts (planner, coder, reviewer) |
| `TaskToolSet` | Register `explorer`, `implementer`, `tester`, `reviewer` |
| `Tools allowlists` | Least-privilege per expert |
| `Condenser` | Long-horizon memory compression |
| `Stuck detector` | Kill infinite loops |
| `Workspace` | Docker / local sandbox for code execution |

**Installation:**
```bash
py -3.12 -m pip install openhands-ai  # or local SDK from your fork
```

**Integration:**
- `moed.py` imports `openhands` and uses `Agent` class.
- Each MOE phase spawns an `Agent` with a specific `Skill`.
- Artifacts are typed Python dataclasses, serialized to JSON for Rust TUI.

---

### Phase 4: Human Checkpoints & TUI Polish (Days 15–18)

**Goal:** The IDE feels like Grok Build but with MOE superpowers.

**New TUI features:**
1. **Plan Approval Gate**
   - After Phase 1 (Plan), TUI pauses.
   - User can edit the plan in-place (Ratatui textarea).
   - Press `Ctrl+A` to approve, `Ctrl+R` to reject and regenerate.

2. **Agent Activity Timeline**
   - Sidebar showing which agent is running, which is idle.
   - Color-coded: green = done, yellow = running, red = failed.

3. **Cost Chips**
   - Per-agent token usage and cost (from Ollama response headers).
   - Total cost for the task.

4. **Diff Viewer**
   - Before/after split pane for `PatchSet`.
   - Reuse `similar` crate (already in grok-build workspace deps).

5. **Keybindings**
   - `Ctrl+P` — focus Plan panel
   - `Ctrl+C` — focus Code panel
   - `Ctrl+R` — focus Review panel
   - `Ctrl+T` — focus Terminal panel
   - `Ctrl+Enter` — submit task
   - `Ctrl+Q` — quit

---

### Phase 5: Build & Ship (Days 19–21)

**Release profile:**
```bash
cargo build --profile release-dist
```

**Distribution:**
- Single binary: `closedhands.exe` (Windows)
- Sidecar: `moed.py` + `requirements.txt`
- Config wizard: `closedhands init` (generates `~/.closedhands/config.toml`)

**Installer (Windows):**
```powershell
# install.ps1
irm https://raw.githubusercontent.com/aalhadxx/closedhands/main/install.ps1 | iex
```

**Docs:**
- `README.md`: "ClosedHands — a self-hosted MOE coding IDE. Forked from grok-build, powered by Ollama."
- `AGENTS.md`: How the 5 specialists work.
- `ARCHITECTURE.md`: Rust TUI + Python sidecar diagram.

---

## 2. File Inventory (What Lives Where)

| Component | Path | Language |
|-----------|------|----------|
| TUI entrypoint | `crates/codegen/xai-grok-pager-bin/src/main.rs` | Rust |
| TUI widgets | `crates/codegen/xai-ratatui-inline/src/` | Rust |
| Markdown renderer | `crates/codegen/xai-grok-markdown/src/` | Rust |
| Shell / PTY | `crates/codegen/ptyctl/src/` | Rust |
| HTTP client (Ollama) | `crates/codegen/xai-grok-http/src/` | Rust |
| Config loader | `crates/codegen/xai-grok-config/src/` | Rust |
| MOE daemon | `sidecar/moed.py` | Python |
| MOE pipeline | `sidecar/moe/` (refactor moe.py) | Python |
| OpenHands SDK integration | `sidecar/openhands_bridge.py` | Python |
| Build script | `Cargo.toml` (workspace root) | Rust |

---

## 3. Critical Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Backend** | Ollama Cloud | You already have API key, working config, and `kimi-k2.7-code:cloud` is proven. |
| **Models** | All phases use `kimi-k2.7-code:cloud` | You insisted. No mixing. |
| **Auth** | API key in config file | No login wall. No OAuth. Copy-paste and go. |
| **Telemetry** | None | Delete all Mixpanel, tracing to external sinks. Keep local logs only. |
| **MOE topology** | Supervisor + 4 workers | Orchestrator (in Python) routes to Explore/Implement/Test/Review. |
| **Communication** | HTTP JSON between Rust ↔ Python | Simple. Debug with browser/curl. |
| **Parallelism v1** | Only Explore + Review parallel | Implementer stays sequential (one writer). |
| **Isolation** | Git worktrees (later) | v1 uses shared workspace. v2 adds worktrees for parallel implementers. |

---

## 4. Immediate Next Step

If you want to start **right now**:

```bash
cd D:\coding\ai-ide-research
cp -r grok-build closedhands
cd closedhands
# Delete telemetry crates
rm -rf crates/codegen/xai-grok-telemetry
rm -rf crates/codegen/xai-grok-mixpanel
rm -rf crates/codegen/xai-grok-announcements
rm -rf crates/codegen/xai-grok-update
rm -rf crates/codegen/xai-grok-voice
rm -rf crates/codegen/xai-grok-plugin-marketplace
# Remove from Cargo.toml workspace members list
git init
git add .
git commit -m "ClosedHands initial fork from grok-build"
```

Then open `crates/codegen/xai-grok-http/src/lib.rs` and start ripping out the SpaceXAI base URL.

---

**End of plan.** Say the word and I generate the actual file edits for Phase 0.
