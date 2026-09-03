pub mod arxiv;
pub mod dates;
pub mod feeds;
pub mod github;
pub mod hn;
pub mod http;
pub mod huggingface;
pub mod json_feeds;
pub mod opps;

use chrono::{DateTime, Utc};
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// One story as a source reported it, before scoring or clustering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawItem {
    pub kind: String,
    pub title: String,
    pub org: Option<String>,
    pub url: String,
    pub summary: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    /// When applications or submissions close. Only openings carry this.
    pub deadline_at: Option<DateTime<Utc>>,
    /// Where it physically happens, when that is known. Devpost reports
    /// everything as "Online"; the Indian platforms carry real cities, which
    /// is what makes "near me" possible at all.
    pub location: Option<String>,
    /// None when the source does not say.
    pub is_online: Option<bool>,
    pub source: String,
    /// Stable per-source id. Combined with `source` it is the dedupe key.
    pub external_id: String,
    /// Observable audience metrics. These feed `reach`; no LLM is involved.
    pub signals: ReachSignals,
}

/// Everything we can cheaply observe about how much attention an item already has.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReachSignals {
    pub points: Option<u32>,
    pub comments: Option<u32>,
    /// True for the thing itself (a paper, a vendor announcement, a release),
    /// false for coverage *about* it. Reposts carry less weight.
    pub primary: bool,
}

#[derive(Debug)]
pub enum SourceError {
    Network(String),
    Parse(String),
}

impl std::fmt::Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceError::Network(m) => write!(f, "network: {m}"),
            SourceError::Parse(m) => write!(f, "parse: {m}"),
        }
    }
}

impl SourceError {
    /// Distinguishes "the machine is offline" from "this source is broken".
    /// Only the former should put the whole run into its quiet offline state.
    pub fn is_offline(&self) -> bool {
        matches!(self, SourceError::Network(_))
    }
}

impl From<reqwest::Error> for SourceError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_connect() || e.is_timeout() {
            SourceError::Network(e.to_string())
        } else {
            SourceError::Parse(e.to_string())
        }
    }
}

pub type SourceResult = Result<Vec<RawItem>, SourceError>;

/// Result of one full sweep across every registered source.
#[derive(Debug, Default, Serialize)]
pub struct FetchReport {
    pub items: Vec<RawItem>,
    pub counts: HashMap<String, usize>,
    pub errors: HashMap<String, String>,
    /// Set when every source failed to connect — the machine has no network.
    pub offline: bool,
}

/// A single source's fetch, boxed so the registry can hold heterogeneous futures.
pub type Fetcher = fn() -> futures::future::BoxFuture<'static, SourceResult>;

macro_rules! source {
    ($name:expr, $f:path) => {
        ($name, (|| Box::pin($f()) as _) as Fetcher)
    };
}

/// Every news source, in one list. Adding one is a single line here plus a module.
pub fn news_registry() -> Vec<(&'static str, Fetcher)> {
    vec![
        source!("hn-front", hn::front_page),
        source!("hn-rising", hn::rising),
        source!("arxiv", arxiv::fetch),
        source!("hf-papers", huggingface::daily_papers),
        source!("hf-models", huggingface::trending_models),
        source!("lobsters", json_feeds::lobsters),
        source!("devto", json_feeds::devto),
        source!("openai", feeds::openai),
        source!("googleai", feeds::google_ai),
        source!("deepmind", feeds::deepmind),
        source!("verge", feeds::verge),
        source!("ars", feeds::ars),
        source!("techcrunch", feeds::techcrunch),
        source!("simonwillison", feeds::simon_willison),
        source!("anthropic", feeds::anthropic),
        source!("gh-releases", github::releases),
    ]
}

/// Runs every source concurrently.
///
/// A source that fails is recorded and skipped — one broken adapter must never
/// cost us the other fifteen. Each gets its own timeout so a hanging host
/// cannot stall the sweep.
pub async fn fetch_all() -> FetchReport {
    let mut registry = news_registry();
    registry.extend(opps::registry());

    let futures = registry.into_iter().map(|(name, f)| async move {
        let outcome = tokio::time::timeout(Duration::from_secs(25), f()).await;
        let result = match outcome {
            Ok(r) => r,
            Err(_) => Err(SourceError::Network("timed out after 25s".into())),
        };
        (name, result)
    });

    let results = join_all(futures).await;

    let mut report = FetchReport::default();
    let mut network_failures = 0usize;
    let total = results.len();

    for (name, result) in results {
        match result {
            Ok(items) => {
                report.counts.insert(name.to_string(), items.len());
                report.items.extend(items);
            }
            Err(e) => {
                if e.is_offline() {
                    network_failures += 1;
                }
                report.counts.insert(name.to_string(), 0);
                report.errors.insert(name.to_string(), e.to_string());
            }
        }
    }

    // If nothing could connect, this is an offline run rather than sixteen
    // broken adapters — the UI says so quietly instead of showing errors.
    report.offline = total > 0 && network_failures == total;
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_errors_are_offline_but_parse_errors_are_not() {
        // The distinction decides whether the UI shows a quiet "last updated"
        // line or surfaces a genuinely broken adapter.
        assert!(SourceError::Network("connect refused".into()).is_offline());
        assert!(!SourceError::Parse("HTTP 403".into()).is_offline());
    }

    #[test]
    fn a_broken_source_does_not_mark_the_run_offline() {
        // One adapter returning 403 while others succeed is a source bug,
        // not a lost network. Mislabelling it would hide a real failure.
        let mut report = FetchReport::default();
        let results: Vec<(&str, SourceResult)> = vec![
            ("hn-front", Ok(vec![])),
            ("devpost", Err(SourceError::Parse("HTTP 403".into()))),
        ];
        let total = results.len();
        let mut network_failures = 0;
        for (name, result) in results {
            match result {
                Ok(items) => {
                    report.counts.insert(name.to_string(), items.len());
                }
                Err(e) => {
                    if e.is_offline() {
                        network_failures += 1;
                    }
                    report.errors.insert(name.to_string(), e.to_string());
                }
            }
        }
        report.offline = total > 0 && network_failures == total;
        assert!(!report.offline);
        assert_eq!(report.errors.len(), 1);
    }

    #[test]
    fn clean_text_strips_markup_and_collapses_space() {
        let messy = "<p>Hello   <b>there</b>\n\n world</p>";
        assert_eq!(http::clean_text(messy, 100), "Hello there world");
    }

    #[test]
    fn clean_text_truncates_on_a_word_boundary() {
        let long = "the quick brown fox jumps over the lazy dog";
        let out = http::clean_text(long, 15);
        assert!(out.ends_with('…'));
        assert!(out.len() <= 18);
        assert!(!out.contains("jumps"));
    }

    #[test]
    fn every_registered_source_has_a_unique_name() {
        // Duplicate names would silently overwrite each other's counts in the
        // report, hiding a dead adapter.
        let names: Vec<&str> = news_registry().into_iter().map(|(n, _)| n).collect();
        let unique: std::collections::HashSet<&&str> = names.iter().collect();
        assert_eq!(names.len(), unique.len(), "duplicate source name in registry");
    }
}
