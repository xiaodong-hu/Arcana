<p align="center">
  <pre>
     ╔═══════════════════════════════════════════════════════════╗
     ║                                                           ║
     ║              ░█▀█░█▀▄░█▀▀░█▀█░█▀█░█▀█                   ║
     ║              ░█▀█░█▀▄░█░░░█▀█░█░█░█▀█                   ║
     ║              ░▀░▀░▀░▀░▀▀▀░▀░▀░▀░▀░▀░▀                   ║
     ║                                                           ║
     ║          Memory · Skills · Authority · Agents             ║
     ║                                                           ║
     ╚═══════════════════════════════════════════════════════════╝
  </pre>
</p>

<p align="center">
  <strong>A sovereign AI agent that remembers, learns, and operates under your authority.</strong>
</p>

<p align="center">
  <em>Not another chatbot wrapper. A full autonomous agent runtime with memory persistence, skill composition, sub-agent orchestration, and cryptographic authority control — all in your terminal.</em>
</p>

---

## Why Arcana

Every existing coding agent is a **stateless parrot** — it forgets everything the moment you close the terminal. Arcana is different:

| Problem | Arcana's Answer |
|---------|-----------------|
| Agents forget context between sessions | **Persistent memory** — semantic knowledge store survives across sessions |
| No control over what agents do | **Authority system** — every file write is recorded, reviewable, recoverable |
| One model fits all | **Hybrid LLM routing** — different models for different agent roles |
| Skills are hardcoded | **Composable skill modules** — trigger-based, hot-loadable, user-extensible |
| Sub-agents are fire-and-forget | **Orchestrated sub-agents** — checkpointed, freezable, resumable |

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              TERMINAL                                    │
│                                                                         │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │                        arcana (TUI)                               │  │
│  │  ┌─────────────────────────────────────────────────────────────┐  │  │
│  │  │ Status: ⚗ deepseek-v4-pro │ [████░░░░░░] 8.2K/1M          │  │  │
│  │  ├─────────────────────────────────────────────────────────────┤  │  │
│  │  │ Viewport (streaming responses, thinking blocks, diffs)      │  │  │
│  │  ├─────────────────────────────────────────────────────────────┤  │  │
│  │  │ Composer (multiline input)                                  │  │  │
│  │  └─────────────────────────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│                              │                                           │
│                    Unix Socket IPC                                       │
│                              │                                           │
│  ┌───────────┐  ┌───────────┴───────────┐  ┌─────────────────────────┐ │
│  │  Skills   │  │  Authority & Record   │  │  Sub-Agent Orchestrator │ │
│  │  Daemon   │  │  Daemon               │  │                         │ │
│  │           │  │                       │  │  ┌─────┐ ┌─────┐       │ │
│  │ triggers  │  │  • Permission gate    │  │  │Agent│ │Agent│ ...   │ │
│  │ manifests │  │  • Git-like recording │  │  │  1  │ │  2  │       │ │
│  │ hot-load  │  │  • Crash recovery     │  │  └─────┘ └─────┘       │ │
│  └───────────┘  └───────────────────────┘  └─────────────────────────┘ │
│                              │                                           │
│                    ┌─────────┴─────────┐                                │
│                    │   Memory System   │                                 │
│                    │                   │                                 │
│                    │  • Knowledge DB   │                                 │
│                    │  • Error patterns │                                 │
│                    │  • Session recall │                                 │
│                    │  • Embeddings     │                                 │
│                    └───────────────────┘                                 │
└─────────────────────────────────────────────────────────────────────────┘
```

### Agent Hierarchy

```
                    ┌──────────────────┐
                    │    Main Agent    │  ← deepseek-v4-pro (configurable)
                    │  plans, reasons  │
                    └────────┬─────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
     ┌────────▼───┐  ┌──────▼─────┐  ┌────▼────────┐
     │Query Agent │  │ Sub-Agent  │  │ Sub-Agent   │  ← deepseek-v4-flash
     │(persistent)│  │ (spawned)  │  │ (spawned)   │    (configurable)
     │ shares ctx │  │ scoped fs  │  │ scoped fs   │
     └────────────┘  └────────────┘  └─────────────┘
```

### Data Flow

```
User Input ──► Skill Triggers ──► Main Agent ──► Authority Gate ──► File System
                                       │                │
                                       │           git_record/
                                       │          (every mutation)
                                       ▼
                                  Memory Store
                              (knowledge, errors,
                               session history)
```

---

## Features

### Hybrid LLM Configuration

Assign different models to different roles. Use your most powerful model where it matters, cheap models where it doesn't:

```toml
[agents.main]
provider = "deepseek"
model = "deepseek-v4-pro"

[agents.main.thinking]
enabled = true
reasoning_effort = "max"

[agents.query]
provider = "deepseek"
model = "deepseek-v4-pro"

[agents.sub]
provider = "deepseek"
model = "deepseek-v4-flash"    # Fast & cheap for parallel work
```

### Authority & Recording

Every file mutation is gated and recorded. Full git-like history of agent actions:

```
.arcana/git_record/
├── objects/          # Content-addressed blobs
├── actions.jsonl     # Append-only action log
├── snapshots/        # Periodic full snapshots
└── HEAD              # Current sequence number
```

Recover any state: `arcana recover . --to-seq 42`

### Persistent Memory

Knowledge survives across sessions. The agent learns your codebase, your patterns, your mistakes:

- **Knowledge store** — semantic search over accumulated project understanding
- **Error patterns** — never repeat the same mistake twice
- **Session memory** — resume exactly where you left off

### Composable Skills

Hot-loadable, trigger-based skill modules:

```toml
# ~/.arcana/skills/user/my-skill/manifest.toml
[skill]
name = "deploy-checker"
trigger = { pattern = "deploy|ship|release" }
mode = "inject"    # inject context when triggered
```

### Per-Response Telemetry

Every LLM response shows exactly what it cost:

```
Expense: 0.0031 ( 1.2K in / 847 out )
Time: 2.4s
```

---

## Quick Start

```bash
# Install (from source)
cd arcana_tui && cargo build --release
cp target/release/arcana ~/.local/bin/

# First-time setup
arcana onboard

# Start working
cd your-project
arcana
```

### Key Commands

```bash
arcana                          # Interactive session
arcana -q "explain main.rs"    # Single-shot query
arcana --model deepseek-v4-flash  # Override model
arcana config show              # View configuration
arcana config edit              # Edit config in $EDITOR
arcana --reset                  # Factory reset
arcana check                    # System health check
arcana resume --last            # Resume previous session
```

### Keybindings

| Key | Action |
|-----|--------|
| `Ctrl+T` | Toggle tasks panel |
| `Ctrl+S` | Toggle skills panel |
| `Ctrl+A` | Toggle agents panel |
| `?` | Open query agent overlay |
| `Ctrl+C` | Interrupt / clear |
| `Ctrl+D` | End session |
| `Ctrl+Shift+P` | Freeze all agents |

---

## Configuration

Config lives at `~/.arcana/config.toml`. Created automatically on first launch.

```bash
arcana config show    # Print current config
arcana config edit    # Open in $EDITOR
arcana config path    # Print file path
```

See [doc/agent_usage.md](doc/agent_usage.md) §7 for the full configuration reference.

---

## Project Structure

```
Arcana-Agent/
├── arcana_tui/              # Terminal UI (ratatui + crossterm)
├── authority_and_recording/ # Permission gate + mutation recording
├── human_in_loop_interaction/ # Diff review, session management
├── subagent_system/         # Sub-agent orchestration + checkpointing
├── skills_modules/          # Skill daemon, triggers, manifests
├── memory_system/           # Knowledge DB, embeddings, semantic search
└── doc/                     # Design documents
    ├── agent_usage.md       # User manual
    ├── tui_design.md        # TUI architecture
    ├── agent_running_design.md        # Agent runtime design
    └── authority_and_recording_design.md  # Authority system design
```

---

## Design Philosophy

1. **The agent works for you, not the other way around.** Authority is non-negotiable — every destructive action requires explicit approval or pre-configured trust.

2. **Memory is not optional.** An agent that forgets is just an expensive autocomplete. Arcana accumulates understanding over time.

3. **Composition over monoliths.** Skills, sub-agents, and memory layers are independent, hot-swappable modules communicating over unix sockets.

4. **Transparency over magic.** Every token spent, every file touched, every decision made — visible, recorded, recoverable.

---

## Documentation

| Document | Contents |
|----------|----------|
| [Agent Usage Manual](doc/agent_usage.md) | CLI commands, keybindings, configuration, workflows |
| [TUI Design](doc/tui_design.md) | Terminal interface architecture, rendering, streaming |
| [Agent Runtime](doc/agent_running_design.md) | Agent lifecycle, context management, LLM integration |
| [Authority & Recording](doc/authority_and_recording_design.md) | Permission system, mutation recording, crash recovery |

---

## License

MIT
