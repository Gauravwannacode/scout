use crate::sources::http::{clean_text, get_json};
use crate::sources::{RawItem, ReachSignals, SourceResult};
use chrono::{DateTime, Utc};
use serde::Deserialize;

/// A company that launched days ago, whose founders are in the thread right
/// now answering everyone. No listing exists, so there is no queue.
const LAUNCH_HN: &str =
    "https://hn.algolia.com/api/v1/search?query=Launch%20HN&tags=story&hitsPerPage=25";

/// Show HN skews to solo builders and tiny teams — reachable people, and often
/// the origin of an open-source project worth contributing to early.
const SHOW_HN: &str = "https://hn.algolia.com/api/v1/search_by_date?tags=show_hn&hitsPerPage=30";

/// The monthly hiring thread. Comments are the actual postings, and many are
/// remote and explicitly open to juniors.
const WHO_IS_HIRING: &str =
    "https://hn.algolia.com/api/v1/search?query=Ask%20HN%20Who%20is%20hiring&tags=story&hitsPerPage=3";

#[derive(Debug, Deserialize)]
struct Response {
    hits: Vec<Hit>,
}

#[derive(Debug, Deserialize)]
struct Hit {
    #[serde(rename = "objectID")]
    object_id: String,
    title: Option<String>,
    points: Option<u32>,
    num_comments: Option<u32>,
    created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    story_text: Option<String>,
}

fn hn_discussion(id: &str) -> String {
    format!("https://news.ycombinator.com/item?id={id}")
}

fn to_items(hits: Vec<Hit>, source: &str, kind: &str, note: &str) -> Vec<RawItem> {
    hits.into_iter()
        .filter_map(|h| {
            let title = h.title?;
            let summary = h
                .story_text
                .map(|s| clean_text(&s, 300))
                .filter(|s| !s.is_empty())
                .map(|s| format!("{note} {s}"))
                .unwrap_or_else(|| note.to_string());

            Some(RawItem {
                kind: kind.into(),
                title,
                org: None,
                // Always link the discussion: the comments are where the
                // founders are, which is the whole reason to be here early.
                url: hn_discussion(&h.object_id),
                summary: Some(summary),
                published_at: h.created_at,
                deadline_at: None,
                source: source.into(),
                external_id: h.object_id,
                signals: ReachSignals {
                    points: h.points,
                    comments: h.num_comments,
                    primary: true,
                },
            })
        })
        .collect()
}

pub async fn launch_hn() -> SourceResult {
    let res: Response = get_json(LAUNCH_HN).await?;
    // The query is fuzzy, so keep only genuine Launch HN posts.
    let hits = res
        .hits
        .into_iter()
        .filter(|h| {
            h.title
                .as_deref()
                .is_some_and(|t| t.to_lowercase().starts_with("launch hn"))
        })
        .collect();
    Ok(to_items(
        hits,
        "launch-hn",
        "company",
        "Just launched — founders are answering in the thread.",
    ))
}

pub async fn show_hn() -> SourceResult {
    let res: Response = get_json(SHOW_HN).await?;
    Ok(to_items(
        res.hits,
        "show-hn",
        "company",
        "Small team or solo builder, reachable directly.",
    ))
}

pub async fn who_is_hiring() -> SourceResult {
    let res: Response = get_json(WHO_IS_HIRING).await?;

    // Only the current month's thread is useful; older ones are stale roles.
    let newest = res
        .hits
        .into_iter()
        .filter(|h| {
            h.title
                .as_deref()
                .is_some_and(|t| t.to_lowercase().contains("who is hiring"))
        })
        .max_by_key(|h| h.created_at);

    let Some(thread) = newest else {
        return Ok(Vec::new());
    };

    // The thread itself is the item. Its comments are the postings; pulling
    // and filtering those individually is a later refinement, and linking the
    // thread is already actionable today.
    Ok(vec![RawItem {
        kind: "job".into(),
        title: thread
            .title
            .clone()
            .unwrap_or_else(|| "Who is hiring?".into()),
        org: None,
        url: hn_discussion(&thread.object_id),
        summary: Some(format!(
            "This month's hiring thread — {} postings, many remote and open to juniors.",
            thread.num_comments.unwrap_or(0)
        )),
        published_at: thread.created_at,
        deadline_at: None,
        source: "hn-hiring".into(),
        external_id: thread.object_id,
        signals: ReachSignals {
            points: thread.points,
            comments: thread.num_comments,
            primary: true,
        },
    }])
}
