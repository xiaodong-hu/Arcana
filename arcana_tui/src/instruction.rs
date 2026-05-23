use std::path::PathBuf;

const DEFAULT_INSTRUCTION: &str = r#"# Arcana Authority Instruction

This file is human-maintained first-line authority guidance for Arcana agents.
It describes the authority API shape only. Concrete permissions, allowed tools,
blocked commands, filesystem rules, and network rules are supplied by the Rust
authority program from its structured configuration.

## Required Authority Rule

All privileged operations must go through the Arcana authority program. The agent
must not infer permission from this markdown file. When in doubt, ask the
authority program or the human.

## Authority APIs Exposed To Agents

- `query`: ask the authority program whether an operation or path is available.
- `read`: request file content through the authority program.
- `write`: request a recorded file write through the authority program.
- `delete`: request a recorded file deletion through the authority program.
- `rename`: request a recorded file rename through the authority program.
- `exec`: request command execution through the authority program.
- `fetch`: request web access through the authority program.
- `register_tool`: request runtime tool registration through the authority program.
- `instruction`: request this human-maintained instruction text.

## Configuration Boundary

The source of truth for allowlists, denylists, tool permissions, command
permissions, filesystem permissions, and web permissions is the structured
authority configuration managed by the Rust authority program. Agents should not
read or modify `~/.arcana/authority.toml` directly.
"#;

pub fn path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let home = dirs::home_dir().ok_or("cannot find home directory")?;
    Ok(home.join(".arcana").join("INSTRUCTION.md"))
}

pub fn load_or_create() -> Result<String, Box<dyn std::error::Error>> {
    let path = path()?;
    if path.exists() {
        return Ok(std::fs::read_to_string(path)?);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, DEFAULT_INSTRUCTION)?;
    Ok(DEFAULT_INSTRUCTION.to_string())
}
