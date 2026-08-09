use anyhow::{bail, Result};

pub(super) fn parse_openai_response(v: serde_json::Value) -> Result<String> {
    let content = v
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c0| c0.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .map(|s| s.trim().to_string());
    match content {
        Some(c) if !c.is_empty() => Ok(c),
        _ => bail!("OpenAI response did not contain choices[0].message.content"),
    }
}
