use crate::sources::http::{clean_text, get_json};
use crate::sources::{RawItem, ReachSignals, SourceResult};
use chrono::{DateTime, Utc};
use futures::future::join_all;
use serde::Deserialize;

/// Languages he actually works in. Searching everything would bury the ones
/// he could realistically pick up this weekend.
const LANGUAGES: &[&str] = &["typescript", "python", "rust"];

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    items: Vec<Issue>,
}

#[derive(Debug, Deserialize)]
struct Issue {
    id: u64,
    title: String,
    html_url: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    comments: Option<u32>,
    #[serde(default)]
    created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    repository_url: Option<String>,
    #[serde(default)]
    pull_request: Option<serde_json::Value>,
}

fn repo_name(repository_url: &str) -> String {
    // "https://api.github.com/repos/owner/name" -> "owner/name"
    repository_url
        .rsplit("/repos/")
        .next()
        .unwrap_or(repository_url)
        .to_string()
}

async fn for_language(language: &str) -> Vec<RawItem> {
    let url = format!(
        "https://api.github.com/search/issues?q=label:%22good+first+issue%22+state:open+language:{language}+no:assignee&sort=created&order=desc&per_page=12"
    );

    let Ok(res) = get_json::<SearchResponse>(&url).await else {
        // Unauthenticated GitHub search is rate limited hard; one language
        // failing should not take the others down.
        return Vec::new();
    };

    res.items
        .into_iter()
        // Search returns PRs alongside issues; only issues are contributable here.
        .filter(|i| i.pull_request.is_none())
        .map(|i| {
            let repo = i
                .repository_url
                .as_deref()
                .map(repo_name)
                .unwrap_or_default();
            RawItem {
                kind: "oss".into(),
                title: format!("{} — {}", repo, i.title),
                org: repo.split('/').next().map(str::to_string),
                url: i.html_url,
                summary: i
                    .body
                    .map(|b| clean_text(&b, 350))
                    .filter(|b| !b.is_empty())
                    .map(|b| format!("Good first issue in {language}. {b}"))
                    .or_else(|| Some(format!("Good first issue in {language}, unassigned."))),
                published_at: i.created_at,
                deadline_at: None,
                location: None,
                is_online: None,
                source: "gh-issues".into(),
                external_id: i.id.to_string(),
                signals: ReachSignals {
                    points: None,
                    comments: i.comments,
                    primary: true,
                },
            }
        })
        .collect()
}

pub async fn good_first_issues() -> SourceResult {
    let results = join_all(LANGUAGES.iter().map(|l| for_language(l))).await;
    Ok(results.into_iter().flatten().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_name_is_extracted_from_the_api_url() {
        assert_eq!(
            repo_name("https://api.github.com/repos/rust-lang/rust"),
            "rust-lang/rust"
        );
    }
}
