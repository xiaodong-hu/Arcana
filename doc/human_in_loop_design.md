# Human-in-Loop Interaction Design

## Overview

Arcana's authority system ensures the agent never performs destructive actions without explicit human approval. All file mutations, system commands, and network operations are gated through the authority program.

## Approval Panel

When the agent requests a mutation that requires approval, the TUI displays an approval panel with four options:

```
┌─ Authority ──────────────────────────────────────────┐
│ write requires approval: src/main.rs                  │
│                                                       │
│ ❯ Yes, single permission                             │
│   Trust, always allow in this session                 │
│   Human interrupt (wait for edit)                     │
│   Reject and abort                                    │
│                                                       │
│ ↑↓ select │ Enter confirm │ Esc reject                │
└───────────────────────────────────────────────────────┘
```

### Options

| Option | Behavior |
|--------|----------|
| **Yes, single permission** | Approve this one mutation only. Next mutation will ask again. |
| **Trust, always allow in this session** | Approve all mutations for the remainder of this session. Dangerous — use with caution. |
| **Human interrupt (wait for edit)** | Pause the agent. The diff is written to a temp file. User can edit in their `$EDITOR`, modify the changes, then resume. Agent continues with the human-modified version. |
| **Reject and abort** | Reject the mutation. Agent is informed and must find an alternative approach. |

### Keybindings

| Key | Action |
|-----|--------|
| `↑` / `↓` | Move selection |
| `Enter` | Confirm selection |
| `Esc` | Reject (shortcut) |

---

## Diff Review Panel

Before applying file mutations, the agent shows a full diff review panel:

```
┌─ Diff Review: src/main.rs ───────────────────────────┐
│ @@ -10,6 +10,8 @@                                     │
│  use std::io;                                          │
│ +use std::path::PathBuf;                               │
│ +use crate::config::Config;                            │
│  fn main() {                                           │
│ -    println!("old");                                   │
│ +    println!("new");                                   │
│  }                                                     │
│                                                        │
│ ❯ Accept  │  Edit in $EDITOR  │  Reject    1/42 (2%)  │
│ ↑↓/j/k scroll │ ←→ select │ Enter confirm │ Esc reject│
└───────────────────────────────────────────────────────┘
```

### Features

- **Full diff display** — shows the complete unified diff with syntax coloring (green = added, red = removed, cyan = headers)
- **Scrollable** — `j`/`k` or `↑`/`↓` to scroll through large diffs
- **External editor** — "Edit in $EDITOR" opens the diff in neovim/vim/vscode for manual modification
- **Accept/Reject** — approve or deny the changes

### External Editor Flow

When "Edit in $EDITOR" is selected:

1. The proposed file content is written to a temporary file
2. The user's `$EDITOR` (from config or env) is launched with that file
3. User modifies the content as desired
4. On editor exit, the modified content is applied instead of the original proposal
5. Agent is informed of the human modifications

### Keybindings

| Key | Action |
|-----|--------|
| `↑` / `↓` / `j` / `k` | Scroll diff |
| `←` / `→` | Select footer option |
| `Enter` | Confirm selected action |
| `Esc` | Reject changes |
| `Tab` | Open in editor (shortcut) |
| `PgUp` / `PgDn` | Page scroll |

---

## Integration with Authority System

The approval and diff review panels integrate with `~/.arcana/authority.toml`:

- Commands in `[commands.allow]` execute without approval
- Commands in `[commands.confirm]` always show the approval panel
- File writes within `[filesystem.writable]` scope show diff review
- File writes to `[filesystem.deny]` paths are always rejected
- Network requests to `[network.deny]` hosts are always rejected

### Session Trust

When "Trust, always allow in this session" is selected:
- A session-level flag is set for that operation type
- Subsequent similar operations proceed without asking
- Trust resets when the session ends
- Trust is **never** persisted to disk (security)

---

## Workflow Example

```
User: "Add error handling to main.rs"

Agent: [plans changes]
       [generates diff for main.rs]

TUI:   ┌─ Diff Review: src/main.rs ─┐
       │ [shows full diff]            │
       │ ❯ Accept │ Edit │ Reject     │
       └──────────────────────────────┘

User:  [selects "Edit in $EDITOR"]
       [modifies the proposed changes in neovim]
       [saves and exits editor]

Agent: [applies human-modified version]
       [continues with next step]
```
