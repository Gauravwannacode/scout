use super::http::{clean_text, get_text};
use super::{RawItem, ReachSignals, SourceError, SourceResult};
use chrono::{DateTime, Utc};

/// arXiv serves Atom, not JSON. Newest first across the two categories that
/// carry almost all the AI work worth knowing about.
const URL: &str = "http://export.arxiv.org/api/query?\
                   search_query=cat:cs.AI+OR+cat:cs.LG\
                   &sortBy=submittedDate&sortOrder=descending&max_results=40";

pub async fn fetch() -> SourceResult {
    let body = get_text(URL).await?;
    let feed = feed_rs::parser::parse(body.as_bytes())
        .map_err(|e| SourceError::Parse(format!("arxiv atom: {e}")))?;

    let items = feed
        .entries
        .into_iter()
        .filter_map(|e| {
            let title = e.title.map(|t| clean_text(&t.content, 300))?;
            let url = e
                .links
                .iter()
                .find(|l| l.media_type.as_deref() == Some("text/html"))
                .or_else(|| e.links.first())
                .map(|l| l.href.clone())?;

            let published: Option<DateTime<Utc>> = e.published.or(e.updated);

            Some(RawItem {
                kind: "paper".into(),
                title,
                org: None,
                url,
                summary: e.summary.map(|s| clean_text(&s.content, 500)),
                published_at: published,
                deadline_at: None,
                source: "arxiv".into(),
                external_id: e.id,
                signals: ReachSignals {
                    points: None,
                    comments: None,
                    // A paper is the primary artefact, not coverage of one.
                    primary: true,
                },
            })
        })
        .collect();

    Ok(items)
}
