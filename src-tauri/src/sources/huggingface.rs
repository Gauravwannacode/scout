use super::http::{clean_text, get_json};
use super::{RawItem, ReachSignals, SourceResult};
use chrono::{DateTime, Utc};
use serde::Deserialize;

const DAILY_PAPERS: &str = "https://huggingface.co/api/daily_papers?limit=30";
const TRENDING: &str = "https://huggingface.co/api/models?sort=trendingScore&limit=25";

#[derive(Debug, Deserialize)]
struct DailyPaper {
    paper: Paper,
    #[serde(rename = "publishedAt")]
    published_at: Option<DateTime<Utc>>,
    #[serde(default)]
    title: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Paper {
    id: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    upvotes: Option<u32>,
    #[serde(rename = "numComments", default)]
    num_comments: Option<u32>,
}

pub async fn daily_papers() -> SourceResult {
    let rows: Vec<DailyPaper> = get_json(DAILY_PAPERS).await?;

    let items = rows
        .into_iter()
        .filter_map(|row| {
            let title = row.paper.title.or(row.title)?;
            Some(RawItem {
                kind: "paper".into(),
                title: clean_text(&title, 300),
                org: None,
                url: format!("https://huggingface.co/papers/{}", row.paper.id),
                summary: row.paper.summary.map(|s| clean_text(&s, 500)),
                published_at: row.published_at,
                deadline_at: None,
                location: None,
                is_online: None,
                source: "hf-papers".into(),
                external_id: row.paper.id,
                signals: ReachSignals {
                    points: row.paper.upvotes,
                    comments: row.paper.num_comments,
                    primary: true,
                },
            })
        })
        .collect();

    Ok(items)
}

#[derive(Debug, Deserialize)]
struct Model {
    id: String,
    #[serde(default)]
    likes: Option<u32>,
    #[serde(rename = "createdAt", default)]
    created_at: Option<DateTime<Utc>>,
    #[serde(rename = "pipeline_tag", default)]
    pipeline_tag: Option<String>,
}

pub async fn trending_models() -> SourceResult {
    let rows: Vec<Model> = get_json(TRENDING).await?;

    let items = rows
        .into_iter()
        .map(|m| {
            let summary = m
                .pipeline_tag
                .as_ref()
                .map(|t| format!("Trending {t} model on Hugging Face."));
            RawItem {
                kind: "release".into(),
                title: format!("{} is trending on Hugging Face", m.id),
                org: m.id.split('/').next().map(str::to_string),
                url: format!("https://huggingface.co/{}", m.id),
                summary,
                published_at: m.created_at,
                deadline_at: None,
                location: None,
                is_online: None,
                source: "hf-models".into(),
                external_id: m.id,
                signals: ReachSignals {
                    points: m.likes,
                    comments: None,
                    primary: true,
                },
            }
        })
        .collect();

    Ok(items)
}
