use super::http::{clean_text, get_json};
use super::{RawItem, ReachSignals, SourceResult};
use chrono::{DateTime, Utc};
use futures::future::join_all;
use serde::Deserialize;

/// Repos whose releases actually change what he should be writing. Kept short
/// on purpose: a long list would drown the brief in routine patch notes.
const WATCHED: &[&str] = &[
    "facebook/react",
    "vercel/next.js",
    "microsoft/TypeScript",
    "tauri-apps/tauri",
    "langchain-ai/langchain",
    "ggml-org/llama.cpp",
    "huggingface/transformers",
    "rust-lang/rust",
];

#[derive(Debug, Deserialize)]
struct Release {
    id: u64,
    name: Option<String>,
    tag_name: String,
    html_url: String,
    body: Option<String>,
    published_at: Option<DateTime<Utc>>,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    draft: bool,
}

async fn for_repo(repo: &str) -> Vec<RawItem> {
    let url = format!("https://api.github.com/repos/{repo}/releases?per_page=3");
    let Ok(rows) = get_json::<Vec<Release>>(&url).await else {
        // A single repo failing is not worth failing the source over —
        // unauthenticated GitHub rate limits are hit easily.
        return Vec::new();
    };

    rows.into_iter()
        .filter(|r| !r.draft && !r.prerelease)
        .map(|r| {
            let label = r.name.filter(|n| !n.is_empty()).unwrap_or_else(|| r.tag_name.clone());
            RawItem {
                kind: "release".into(),
                title: format!("{repo} released {label}"),
                org: repo.split('/').next().map(str::to_string),
                url: r.html_url,
                summary: r.body.map(|b| clean_text(&b, 400)).filter(|b| !b.is_empty()),
                published_at: r.published_at,
                deadline_at: None,
                location: None,
                is_online: None,
                source: "gh-releases".into(),
                external_id: r.id.to_string(),
                signals: ReachSignals {
                    points: None,
                    comments: None,
                    primary: true,
                },
            }
        })
        .collect()
}

pub async fn releases() -> SourceResult {
    let results = join_all(WATCHED.iter().map(|r| for_repo(r))).await;
    Ok(results.into_iter().flatten().collect())
}
