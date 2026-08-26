use crate::sources::dates::{parse_devpost_range, parse_iso};
use crate::sources::http::{clean_text, client, get_json};
use crate::sources::{RawItem, ReachSignals, SourceError, SourceResult};
use serde::Deserialize;

const DEVPOST: &str = "https://devpost.com/api/hackathons?status[]=open&order_by=deadline";
const UNSTOP: &str =
    "https://unstop.com/api/public/opportunity/search-result?opportunity=hackathons&per_page=30";
const DEVFOLIO: &str = "https://api.devfolio.co/api/search/hackathons";

#[derive(Debug, Deserialize)]
struct DevpostResponse {
    hackathons: Vec<DevpostHackathon>,
}

#[derive(Debug, Deserialize)]
struct DevpostHackathon {
    id: u64,
    title: String,
    url: String,
    #[serde(default)]
    displayed_location: Option<DevpostLocation>,
    #[serde(default)]
    submission_period_dates: Option<String>,
    #[serde(default)]
    prize_amount: Option<String>,
    #[serde(default)]
    registrations_count: Option<u32>,
    #[serde(default)]
    organization_name: Option<String>,
    #[serde(default)]
    invite_only: bool,
    #[serde(default)]
    time_left_to_submission: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DevpostLocation {
    #[serde(default)]
    location: Option<String>,
}

/// Devpost returns 403 to a plain GET. It requires the XHR header and a
/// matching Referer — without both, this adapter silently returns nothing.
pub async fn devpost() -> SourceResult {
    let res = client()
        .get(DEVPOST)
        .header("X-Requested-With", "XMLHttpRequest")
        .header("Referer", "https://devpost.com/hackathons")
        .header("Accept", "application/json")
        .send()
        .await?;

    let status = res.status();
    if !status.is_success() {
        return Err(SourceError::Parse(format!("devpost HTTP {status}")));
    }

    let body: DevpostResponse = res
        .json()
        .await
        .map_err(|e| SourceError::Parse(format!("devpost json: {e}")))?;

    let items = body
        .hackathons
        .into_iter()
        .filter(|h| !h.invite_only)
        .map(|h| {
            let location = h
                .displayed_location
                .and_then(|l| l.location)
                .unwrap_or_else(|| "Online".into());
            let prize = h.prize_amount.as_deref().unwrap_or("");
            // Registrations are the crowding signal: a big prize with a small
            // field is a far better bet than the reverse.
            let entrants = h
                .registrations_count
                .map(|c| format!("{c} registered"))
                .unwrap_or_default();
            let closing = h.time_left_to_submission.as_deref().unwrap_or("");

            let summary = clean_text(
                &[
                    location.as_str(),
                    prize,
                    entrants.as_str(),
                    closing,
                    h.submission_period_dates.as_deref().unwrap_or(""),
                ]
                .iter()
                .filter(|s| !s.is_empty())
                .cloned()
                .collect::<Vec<_>>()
                .join(" · "),
                400,
            );

            RawItem {
                kind: "hackathon".into(),
                title: h.title,
                org: h.organization_name,
                url: h.url,
                summary: Some(summary),
                published_at: None,
                // Drives the reminder sweep and the "closes in N days" line.
                deadline_at: h
                    .submission_period_dates
                    .as_deref()
                    .and_then(parse_devpost_range),
                source: "devpost".into(),
                external_id: h.id.to_string(),
                signals: ReachSignals {
                    // Registrations are the audience size for a hackathon.
                    points: h.registrations_count,
                    comments: None,
                    primary: true,
                },
            }
        })
        .collect();

    Ok(items)
}

#[derive(Debug, Deserialize)]
struct UnstopResponse {
    data: UnstopInner,
}

#[derive(Debug, Deserialize)]
struct UnstopInner {
    data: Vec<UnstopItem>,
}

#[derive(Debug, Deserialize)]
struct UnstopItem {
    id: u64,
    title: String,
    public_url: String,
    /// Unstop sends 1/0 here, not a JSON boolean — deserialising as bool
    /// fails the whole response.
    #[serde(default)]
    regn_open: Option<i32>,
    #[serde(default)]
    organisation: Option<UnstopOrg>,
    #[serde(default)]
    region: Option<String>,
    /// Unstop keeps the registration window nested rather than alongside the
    /// other fields; `end_regn_dt` is when applications actually close.
    #[serde(rename = "regnRequirements", default)]
    regn: Option<UnstopRegn>,
}

#[derive(Debug, Deserialize)]
struct UnstopRegn {
    #[serde(default)]
    end_regn_dt: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UnstopOrg {
    #[serde(default)]
    name: Option<String>,
}

pub async fn unstop() -> SourceResult {
    let res: UnstopResponse = get_json(UNSTOP).await?;

    let items = res
        .data
        .data
        .into_iter()
        .filter(|i| i.regn_open.unwrap_or(1) != 0)
        .map(|i| RawItem {
            kind: "hackathon".into(),
            title: i.title,
            org: i.organisation.and_then(|o| o.name),
            url: format!("https://unstop.com/{}", i.public_url.trim_start_matches('/')),
            summary: i.region.map(|r| format!("Unstop · {r}")),
            published_at: None,
            deadline_at: i.regn.and_then(|r| r.end_regn_dt).as_deref().and_then(parse_iso),
            source: "unstop".into(),
            external_id: i.id.to_string(),
            signals: ReachSignals {
                points: None,
                comments: None,
                primary: true,
            },
        })
        .collect();

    Ok(items)
}

#[derive(Debug, Deserialize)]
struct DevfolioResponse {
    hits: DevfolioHits,
}

#[derive(Debug, Deserialize)]
struct DevfolioHits {
    hits: Vec<DevfolioHit>,
}

#[derive(Debug, Deserialize)]
struct DevfolioHit {
    #[serde(rename = "_source")]
    source: DevfolioSource,
}

#[derive(Debug, Deserialize)]
struct DevfolioSource {
    #[serde(default)]
    uuid: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    slug: Option<String>,
    #[serde(default)]
    desc: Option<String>,
    #[serde(default)]
    starts_at: Option<String>,
    #[serde(default)]
    ends_at: Option<String>,
    #[serde(default)]
    is_online: Option<bool>,
}

/// Devfolio search is an Elasticsearch proxy, so it needs a POST body and the
/// results are nested two levels deep under `hits.hits[]._source`.
pub async fn devfolio() -> SourceResult {
    let res = client()
        .post(DEVFOLIO)
        .json(&serde_json::json!({
            "type": "application_open",
            "from": 0,
            "size": 40
        }))
        .send()
        .await?;

    let status = res.status();
    if !status.is_success() {
        return Err(SourceError::Parse(format!("devfolio HTTP {status}")));
    }

    let body: DevfolioResponse = res
        .json()
        .await
        .map_err(|e| SourceError::Parse(format!("devfolio json: {e}")))?;

    let items = body
        .hits
        .hits
        .into_iter()
        .filter_map(|h| {
            let s = h.source;
            let name = s.name?;
            let slug = s.slug.clone().unwrap_or_else(|| name.to_lowercase());
            let mode = match s.is_online {
                Some(true) => "Online",
                Some(false) => "In person",
                None => "",
            };
            let window = match (s.starts_at.as_deref(), s.ends_at.as_deref()) {
                (Some(a), Some(b)) => format!("{} → {}", &a[..10.min(a.len())], &b[..10.min(b.len())]),
                _ => String::new(),
            };
            let summary = [
                s.desc.as_deref().unwrap_or(""),
                mode,
                window.as_str(),
            ]
            .iter()
            .filter(|p| !p.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" · ");

            Some(RawItem {
                kind: "hackathon".into(),
                title: name,
                org: None,
                url: format!("https://{slug}.devfolio.co"),
                summary: Some(clean_text(&summary, 400)),
                published_at: None,
                // Submission close — the date he would actually be working to.
                deadline_at: s.ends_at.as_deref().and_then(parse_iso),
                source: "devfolio".into(),
                external_id: s.uuid.unwrap_or(slug),
                signals: ReachSignals {
                    points: None,
                    comments: None,
                    primary: true,
                },
            })
        })
        .collect();

    Ok(items)
}
