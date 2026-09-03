use super::http::get_json;
use super::{RawItem, ReachSignals, SourceResult};
use chrono::{DateTime, Utc};
use serde::Deserialize;

const FRONT_PAGE: &str = "https://hn.algolia.com/api/v1/search?tags=front_page&hitsPerPage=30";

/// Stories that are gaining traction but have not peaked: enough points to
/// show someone cared, few enough that the crowd has not arrived. This is the
/// main way a big-but-unnoticed story gets caught early — a front-page-only
/// diet would always be late.
const RISING: &str = "https://hn.algolia.com/api/v1/search_by_date?tags=story\
                      &numericFilters=points%3E5,points%3C40&hitsPerPage=40";

#[derive(Debug, Deserialize)]
struct Response {
    hits: Vec<Hit>,
}

#[derive(Debug, Deserialize)]
struct Hit {
    #[serde(rename = "objectID")]
    object_id: String,
    title: Option<String>,
    url: Option<String>,
    points: Option<u32>,
    num_comments: Option<u32>,
    created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    story_text: Option<String>,
}

fn to_items(hits: Vec<Hit>, source: &str) -> Vec<RawItem> {
    hits.into_iter()
        .filter_map(|h| {
            let title = h.title?;
            // Ask HN / poll entries have no URL; link to the discussion instead.
            let url = h.url.unwrap_or_else(|| {
                format!("https://news.ycombinator.com/item?id={}", h.object_id)
            });
            Some(RawItem {
                kind: "news".into(),
                title,
                org: None,
                url,
                summary: h.story_text.map(|s| super::http::clean_text(&s, 400)),
                published_at: h.created_at,
                deadline_at: None,
                location: None,
                is_online: None,
                source: source.into(),
                external_id: h.object_id,
                signals: ReachSignals {
                    points: h.points,
                    comments: h.num_comments,
                    // HN links to the story rather than being it.
                    primary: false,
                },
            })
        })
        .collect()
}

pub async fn front_page() -> SourceResult {
    let res: Response = get_json(FRONT_PAGE).await?;
    Ok(to_items(res.hits, "hn-front"))
}

pub async fn rising() -> SourceResult {
    let res: Response = get_json(RISING).await?;
    Ok(to_items(res.hits, "hn-rising"))
}
