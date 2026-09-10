//! Where the fact base lives.
//!
//! A JSON file beside `settings.json`, owned by Rust — not SQLite. Tailoring
//! runs in Rust and needs the facts on every call, and the database belongs to
//! the frontend plugin; passing the whole fact base across the boundary for
//! each draft would be work for nothing. The file is small and edited rarely,
//! so a read-modify-write is the right shape.

use super::facts::{seed, seed_header, Fact, Header};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumeData {
    pub header: Header,
    pub facts: Vec<Fact>,
}

impl Default for ResumeData {
    fn default() -> Self {
        Self {
            header: seed_header(),
            facts: seed(),
        }
    }
}

fn path() -> Option<PathBuf> {
    Some(crate::settings::config_dir()?.join("resume.json"))
}

/// Reads the fact base, seeding it on first use.
///
/// A corrupt file falls back to the seed rather than failing: losing edits is
/// bad, but a resume feature that refuses to open is worse, and the original
/// is still on disk to recover by hand.
pub fn load() -> ResumeData {
    let Some(p) = path() else {
        return ResumeData::default();
    };
    match std::fs::read_to_string(&p) {
        Ok(raw) => match serde_json::from_str(&raw) {
            Ok(d) => d,
            Err(e) => {
                log::warn!("resume.json is unreadable, using the seed: {e}");
                ResumeData::default()
            }
        },
        // First run.
        Err(_) => {
            let d = ResumeData::default();
            let _ = save(&d);
            d
        }
    }
}

pub fn save(data: &ResumeData) -> Result<(), String> {
    let p = path().ok_or("could not resolve the config directory")?;
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
    std::fs::write(&p, json).map_err(|e| e.to_string())
}

/// Facts in display order, skipping anything switched off.
pub fn enabled_facts(data: &ResumeData) -> Vec<Fact> {
    let mut facts: Vec<Fact> = data.facts.iter().filter(|f| f.enabled).cloned().collect();
    facts.sort_by_key(|f| f.sort_order);
    facts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_the_seeded_fact_base() {
        let d = ResumeData::default();
        assert_eq!(d.facts.len(), seed().len());
        assert_eq!(d.header.name, "Gaurav Mehta");
    }

    #[test]
    fn disabled_facts_are_excluded_and_order_is_respected() {
        let mut d = ResumeData::default();
        for f in d.facts.iter_mut() {
            if f.id == "proj-jos" {
                f.enabled = false;
            }
        }
        let facts = enabled_facts(&d);
        assert!(!facts.iter().any(|f| f.id == "proj-jos"));

        let projects: Vec<&str> = facts
            .iter()
            .filter(|f| f.kind == "project")
            .map(|f| f.id.as_str())
            .collect();
        // JOS is gone, so Market Intelligence leads.
        assert_eq!(projects.first(), Some(&"proj-market"));
    }

    #[test]
    fn a_round_trip_through_json_keeps_every_field() {
        let d = ResumeData::default();
        let json = serde_json::to_string(&d).unwrap();
        let back: ResumeData = serde_json::from_str(&json).unwrap();
        assert_eq!(back.facts.len(), d.facts.len());
        let jos = back.facts.iter().find(|f| f.id == "proj-jos").unwrap();
        assert!(jos.detail.as_deref().unwrap_or("").contains("188 tests"));
    }
}
