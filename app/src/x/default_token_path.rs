use std::path::PathBuf;

pub fn default_token_path() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join("x.json"))
}
