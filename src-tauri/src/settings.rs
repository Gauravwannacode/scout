use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Local, single-user configuration. Keys live in a plain file on this machine
/// — there is no server, so nothing is transmitted anywhere except to the
/// model providers when a scoring run happens.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Several keys are supported because Groq's free tier has a small daily
    /// token budget. When one is exhausted the scorer moves to the next
    /// rather than dropping the whole run to the heuristic.
    pub groq_api_keys: Vec<String>,
    pub gemini_api_key: String,
}

impl Settings {
    /// Non-empty keys, trimmed and de-duplicated, in configured order.
    pub fn groq_keys(&self) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        self.groq_api_keys
            .iter()
            .map(|k| k.trim())
            .filter(|k| !k.is_empty())
            .filter(|k| seen.insert(k.to_string()))
            .map(str::to_string)
            .collect()
    }

    pub fn has_groq(&self) -> bool {
        !self.groq_keys().is_empty()
    }
}

/// `%APPDATA%\dev.gaurav.scout` on Windows, the XDG config dir elsewhere.
/// Resolved without an AppHandle so the probe binary can read it too.
pub fn config_dir() -> Option<PathBuf> {
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config"))
    }?;
    Some(base.join("dev.gaurav.scout"))
}

pub fn settings_path() -> Option<PathBuf> {
    Some(config_dir()?.join("settings.json"))
}

/// Environment variables win over the file, which makes testing and one-off
/// runs easy without touching saved configuration.
pub fn load() -> Settings {
    let mut s = settings_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|raw| serde_json::from_str::<Settings>(&raw).ok())
        .unwrap_or_default();

    // Accept a comma-separated list so multiple keys can be supplied at once.
    if let Ok(k) = std::env::var("GROQ_API_KEY") {
        let keys: Vec<String> = k
            .split(',')
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .collect();
        if !keys.is_empty() {
            s.groq_api_keys = keys;
        }
    }
    if let Ok(k) = std::env::var("GEMINI_API_KEY") {
        if !k.trim().is_empty() {
            s.gemini_api_key = k;
        }
    }
    s
}

pub fn save(s: &Settings) -> Result<(), String> {
    let path = settings_path().ok_or("cannot resolve a config directory")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(s).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_and_duplicate_keys_are_ignored() {
        let s = Settings {
            groq_api_keys: vec![
                "  gsk_one  ".into(),
                "".into(),
                "gsk_one".into(),
                "gsk_two".into(),
            ],
            gemini_api_key: String::new(),
        };
        assert_eq!(s.groq_keys(), vec!["gsk_one", "gsk_two"]);
        assert!(s.has_groq());
    }

    #[test]
    fn no_usable_keys_reads_as_unconfigured() {
        let s = Settings {
            groq_api_keys: vec!["   ".into()],
            gemini_api_key: String::new(),
        };
        assert!(!s.has_groq());
    }
}
