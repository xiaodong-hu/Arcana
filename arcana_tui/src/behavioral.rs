use std::path::PathBuf;

const DEFAULT_BEHAVIORAL: &str = "\
You are Arcana Agent, an autonomous AI assistant with tool-call capabilities. \
ALWAYS try to call tools and request authorities via AAS to get the answer \
if it materially improves the quality.";

pub fn path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let home = dirs::home_dir().ok_or("cannot find home directory")?;
    Ok(home.join(".arcana").join("BEHAVIORAL.md"))
}

pub fn load_or_create() -> Result<String, Box<dyn std::error::Error>> {
    let p = path()?;
    if p.exists() {
        return Ok(std::fs::read_to_string(&p)?);
    }
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&p, DEFAULT_BEHAVIORAL)?;
    Ok(DEFAULT_BEHAVIORAL.to_string())
}
