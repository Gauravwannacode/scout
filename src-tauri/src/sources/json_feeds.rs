use super::http::{clean_text, get_json};
use super::{RawItem, ReachSignals, SourceResult};
use chrono::{DateTime, Utc};
use serde::Deserialize;

const LOBSTERS: &str = "https://lobste.rs/hottest.json";
const DEVTO: &str = "https://dev.to/api/articles?per_page=30&top=1";

#[derive(Debug, Deserialize)]
struct LobstersStory {
    short_id: String,
    title: String,
    url: String,
    comments_url: String,
    score: Option<i32>,
    comment_count: Option<u32>,
    created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    description_plain: Option<String>,
}

pub async fn lobsters() -> SourceResult {
    let rows: Vec<LobstersStory> = get_json(LOBSTERS).await?;

    let items = rows
        .into_iter()
        .map(|s| {
            // Text-only posts carry an empty `url`; the discussion is the story.
            let url = if s.url.is_empty() { s.comments_url } else { s.url };
            RawItem {
                kind: "news".into(),
                title: clean_text(&s.title, 300),
                org: None,
                url,
                summary: s
                    .description_plain
                    .map(|d| clean_text(&d, 400))
                    .filter(|d| !d.is_empty()),
                published_at: s.created_at,
                deadline_at: None,
                location: None,
                is_online: None,
                source: "lobsters".into(),
                external_id: s.short_id,
                signals: ReachSignals {
                    points: s.score.map(|v| v.max(0) as u32),
                    comments: s.comment_count,
                    primary: false,
                },
            }
        })
        .collect();

    Ok(items)
}

#[derive(Debug, Deserialize)]
struct DevtoArticle {
    id: u64,
    title: String,
    description: Option<String>,
    url: String,
    published_at: Option<DateTime<Utc>>,
    positive_reactions_count: Option<u32>,
    comments_count: Option<u32>,
    #[serde(default)]
    organization: Option<DevtoOrg>,
}

#[derive(Debug, Deserialize)]
struct DevtoOrg {
    name: String,
}

pub async fn devto() -> SourceResult {
    let rows: Vec<DevtoArticle> = get_json(DEVTO).await?;

    let items = rows
        .into_iter()
        .map(|a| RawItem {
            kind: "news".into(),
            title: clean_text(&a.title, 300),
            org: a.organization.map(|o| o.name),
            url: a.url,
            summary: a
                .description
                .map(|d| clean_text(&d, 400))
                .filter(|d| !d.is_empty()),
            published_at: a.published_at,
            deadline_at: None,
            location: None,
            is_online: None,
            source: "devto".into(),
            external_id: a.id.to_string(),
            signals: ReachSignals {
                points: a.positive_reactions_count,
                comments: a.comments_count,
                primary: false,
            },
        })
        .collect();

    Ok(items)
}
