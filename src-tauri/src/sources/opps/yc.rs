use crate::sources::http::{clean_text, get_json};
use crate::sources::{RawItem, ReachSignals, SourceResult};
use serde::Deserialize;

/// The highest-value opening source in the whole app.
///
/// A company that just joined a batch is hiring months before it posts on any
/// board — often before it has a careers page at all. By the time a listing
/// exists there is a queue; here there is not. Nothing on a job board can
/// substitute for this.
const URL: &str = "https://api.ycombinator.com/v0.1/companies?page=1";

#[derive(Debug, Deserialize)]
struct Response {
    companies: Vec<Company>,
}

#[derive(Debug, Deserialize)]
struct Company {
    id: u64,
    name: String,
    #[serde(default)]
    website: Option<String>,
    #[serde(rename = "oneLiner", default)]
    one_liner: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    batch: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(rename = "teamSize", default)]
    team_size: Option<u32>,
    #[serde(default)]
    industries: Vec<String>,
}

/// Batches recent enough that the company is still small and reachable.
/// Anything older has an HR process and a careers page, which is exactly the
/// queue this source exists to skip.
fn is_recent_batch(batch: &str) -> bool {
    // Batch labels look like "Summer 2026", "W26", "S25".
    let b = batch.to_lowercase();
    ["2026", "2025", "w26", "s26", "f26", "x26", "w25", "s25"]
        .iter()
        .any(|marker| b.contains(marker))
}

pub async fn companies() -> SourceResult {
    let res: Response = get_json(URL).await?;

    let items = res
        .companies
        .into_iter()
        .filter(|c| {
            // Dead companies are not hiring.
            c.status.as_deref().unwrap_or("Active") != "Inactive"
                && c.batch.as_deref().is_some_and(is_recent_batch)
                // Above roughly 50 people the informal route is gone.
                && c.team_size.unwrap_or(0) <= 50
        })
        .map(|c| {
            let batch = c.batch.clone().unwrap_or_default();
            let size = c
                .team_size
                .map(|t| format!("{t} people"))
                .unwrap_or_else(|| "size unknown".into());
            let pitch = c.one_liner.clone().unwrap_or_default();
            let industries = if c.industries.is_empty() {
                String::new()
            } else {
                format!(" · {}", c.industries.join(", "))
            };

            RawItem {
                kind: "company".into(),
                title: format!("{} ({batch}) is early enough to approach directly", c.name),
                org: Some(c.name.clone()),
                url: c
                    .url
                    .clone()
                    .or_else(|| c.website.clone())
                    .unwrap_or_else(|| "https://www.ycombinator.com/companies".into()),
                summary: Some(clean_text(
                    &format!("{pitch} — {size}{industries}"),
                    400,
                )),
                published_at: None,
                deadline_at: None,
                location: None,
                is_online: None,
                source: "yc".into(),
                external_id: c.id.to_string(),
                signals: ReachSignals {
                    points: None,
                    comments: None,
                    // No aggregator has this as a job yet, which is the point.
                    primary: true,
                },
            }
        })
        .collect();

    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_batches_are_recognised_in_both_label_styles() {
        assert!(is_recent_batch("Summer 2026"));
        assert!(is_recent_batch("W26"));
        assert!(is_recent_batch("S25"));
    }

    #[test]
    fn old_batches_are_excluded() {
        // A 2015 company has an HR process; the informal route is gone.
        assert!(!is_recent_batch("Winter 2015"));
        assert!(!is_recent_batch("S12"));
    }
}
