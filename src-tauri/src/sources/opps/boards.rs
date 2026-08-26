use crate::sources::http::{clean_text, get_json};
use crate::sources::{RawItem, ReachSignals, SourceResult};
use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;

// RemoteOK's terms require a visible link back to them, which the UI provides
// via the item's own url. Keep that in mind before changing how items link.
const REMOTEOK: &str = "https://remoteok.com/api";
const ARBEITNOW: &str = "https://www.arbeitnow.com/api/job-board-api";
const HIMALAYAS: &str = "https://himalayas.app/jobs/api?limit=50";

#[derive(Debug, Deserialize)]
struct RemoteOkRow {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    position: Option<String>,
    #[serde(default)]
    company: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    date: Option<DateTime<Utc>>,
    #[serde(default)]
    tags: Vec<String>,
}

pub async fn remoteok() -> SourceResult {
    let rows: Vec<RemoteOkRow> = get_json(REMOTEOK).await?;

    let items = rows
        .into_iter()
        // The first element is a legal/ToS notice, not a job. It has no
        // position field, so filtering on that removes it naturally.
        .filter_map(|r| {
            let position = r.position?;
            let id = r.id?;
            let company = r.company.unwrap_or_else(|| "Unknown".into());
            Some(RawItem {
                kind: "job".into(),
                title: format!("{position} at {company}"),
                org: Some(company),
                url: r.url.unwrap_or_else(|| "https://remoteok.com".into()),
                summary: r
                    .description
                    .map(|d| clean_text(&d, 400))
                    .filter(|d| !d.is_empty()),
                published_at: r.date,
                deadline_at: None,
                source: "remoteok".into(),
                external_id: id,
                signals: ReachSignals {
                    points: None,
                    comments: None,
                    primary: false,
                },
            })
            .map(|mut item| {
                if !r.tags.is_empty() {
                    let tags = r.tags.join(", ");
                    item.summary = Some(match item.summary {
                        Some(s) => format!("{s} · {tags}"),
                        None => tags,
                    });
                }
                item
            })
        })
        .collect();

    Ok(items)
}

#[derive(Debug, Deserialize)]
struct ArbeitnowResponse {
    data: Vec<ArbeitnowJob>,
}

#[derive(Debug, Deserialize)]
struct ArbeitnowJob {
    slug: String,
    title: String,
    company_name: String,
    description: Option<String>,
    url: String,
    #[serde(default)]
    remote: bool,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    created_at: Option<i64>,
}

pub async fn arbeitnow() -> SourceResult {
    let res: ArbeitnowResponse = get_json(ARBEITNOW).await?;

    let items = res
        .data
        .into_iter()
        .filter(|j| j.remote)
        .map(|j| RawItem {
            kind: "job".into(),
            title: format!("{} at {}", j.title, j.company_name),
            org: Some(j.company_name),
            url: j.url,
            summary: j
                .description
                .map(|d| clean_text(&d, 400))
                .filter(|d| !d.is_empty())
                .map(|d| {
                    if j.tags.is_empty() {
                        d
                    } else {
                        format!("{d} · {}", j.tags.join(", "))
                    }
                }),
            published_at: j
                .created_at
                .and_then(|ts| Utc.timestamp_opt(ts, 0).single()),
            deadline_at: None,
            source: "arbeitnow".into(),
            external_id: j.slug,
            signals: ReachSignals {
                points: None,
                comments: None,
                primary: false,
            },
        })
        .collect();

    Ok(items)
}

#[derive(Debug, Deserialize)]
struct HimalayasResponse {
    jobs: Vec<HimalayasJob>,
}

#[derive(Debug, Deserialize)]
struct HimalayasJob {
    title: String,
    #[serde(rename = "companyName")]
    company_name: String,
    excerpt: Option<String>,
    #[serde(rename = "applicationLink")]
    application_link: Option<String>,
    guid: String,
    #[serde(rename = "pubDate", default)]
    pub_date: Option<i64>,
    #[serde(rename = "expiryDate", default)]
    expiry_date: Option<i64>,
    /// Himalayas tags roles with experience level, which is the single most
    /// useful field here — it is how entry-level work is found directly.
    #[serde(default)]
    seniority: Vec<String>,
}

pub async fn himalayas() -> SourceResult {
    let res: HimalayasResponse = get_json(HIMALAYAS).await?;

    let items = res
        .jobs
        .into_iter()
        .map(|j| {
            let level = if j.seniority.is_empty() {
                String::new()
            } else {
                format!(" · {}", j.seniority.join(", "))
            };
            RawItem {
                kind: "job".into(),
                title: format!("{} at {}", j.title, j.company_name),
                org: Some(j.company_name),
                url: j.application_link.unwrap_or_else(|| j.guid.clone()),
                summary: j
                    .excerpt
                    .map(|e| clean_text(&e, 380))
                    .map(|e| format!("{e}{level}")),
                published_at: j.pub_date.and_then(|ts| Utc.timestamp_opt(ts, 0).single()),
                deadline_at: j
                    .expiry_date
                    .and_then(|ts| Utc.timestamp_opt(ts, 0).single()),
                source: "himalayas".into(),
                external_id: j.guid,
                signals: ReachSignals {
                    points: None,
                    comments: None,
                    primary: false,
                },
            }
        })
        .collect();

    Ok(items)
}
