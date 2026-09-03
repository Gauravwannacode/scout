use super::http::{clean_text, get_text};
use super::{RawItem, ReachSignals, SourceError, SourceResult};
use scraper::{Html, Selector};

/// Most news sources are just RSS or Atom, so they share one implementation.
/// `primary` separates a vendor announcing its own work (OpenAI, DeepMind)
/// from press writing *about* it (Verge, Ars) — reposts carry less weight
/// when measuring how crowded a story is.
async fn parse_feed(
    url: &str,
    source: &'static str,
    kind: &str,
    org: Option<&str>,
    primary: bool,
) -> SourceResult {
    let body = get_text(url).await?;
    let feed = feed_rs::parser::parse(body.as_bytes())
        .map_err(|e| SourceError::Parse(format!("{source}: {e}")))?;

    let items = feed
        .entries
        .into_iter()
        .take(25)
        .filter_map(|e| {
            let title = e.title.map(|t| clean_text(&t.content, 300))?;
            if title.is_empty() {
                return None;
            }
            let link = e.links.first().map(|l| l.href.clone())?;
            let summary = e
                .summary
                .map(|s| s.content)
                .or_else(|| e.content.and_then(|c| c.body))
                .map(|s| clean_text(&s, 400))
                .filter(|s| !s.is_empty());

            Some(RawItem {
                kind: kind.to_string(),
                title,
                org: org.map(str::to_string),
                url: link,
                summary,
                published_at: e.published.or(e.updated),
                deadline_at: None,
                location: None,
                is_online: None,
                source: source.to_string(),
                external_id: e.id,
                signals: ReachSignals {
                    points: None,
                    comments: None,
                    primary,
                },
            })
        })
        .collect();

    Ok(items)
}

pub async fn openai() -> SourceResult {
    parse_feed(
        "https://openai.com/blog/rss.xml",
        "openai",
        "news",
        Some("OpenAI"),
        true,
    )
    .await
}

pub async fn google_ai() -> SourceResult {
    parse_feed(
        "https://blog.google/technology/ai/rss/",
        "googleai",
        "news",
        Some("Google"),
        true,
    )
    .await
}

pub async fn deepmind() -> SourceResult {
    parse_feed(
        "https://deepmind.google/blog/rss.xml",
        "deepmind",
        "news",
        Some("DeepMind"),
        true,
    )
    .await
}

pub async fn verge() -> SourceResult {
    parse_feed(
        "https://www.theverge.com/rss/ai-artificial-intelligence/index.xml",
        "verge",
        "news",
        Some("The Verge"),
        false,
    )
    .await
}

pub async fn ars() -> SourceResult {
    parse_feed(
        "https://feeds.arstechnica.com/arstechnica/technology-lab",
        "ars",
        "news",
        Some("Ars Technica"),
        false,
    )
    .await
}

pub async fn techcrunch() -> SourceResult {
    parse_feed(
        "https://techcrunch.com/feed/",
        "techcrunch",
        "news",
        Some("TechCrunch"),
        false,
    )
    .await
}

pub async fn simon_willison() -> SourceResult {
    parse_feed(
        "https://simonwillison.net/atom/everything/",
        "simonwillison",
        "news",
        Some("Simon Willison"),
        false,
    )
    .await
}

/// Anthropic publishes no feed of any kind, so this is the one scrape in the
/// news set. Kept deliberately loose: it takes any link under /news/ and reads
/// its visible text, so a class-name change on their site degrades to zero
/// items (a visible failure in fetch_run) rather than wrong ones.
pub async fn anthropic() -> SourceResult {
    let body = get_text("https://www.anthropic.com/news").await?;
    let doc = Html::parse_document(&body);
    let link_sel =
        Selector::parse("a[href*='/news/']").map_err(|e| SourceError::Parse(e.to_string()))?;

    let mut items = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for el in doc.select(&link_sel) {
        let Some(href) = el.value().attr("href") else {
            continue;
        };
        // Skip the index page itself and any category links.
        let slug = href.trim_end_matches('/');
        if slug.ends_with("/news") || slug.is_empty() {
            continue;
        }
        if !seen.insert(slug.to_string()) {
            continue;
        }

        let title = clean_text(&el.text().collect::<String>(), 300);
        if title.len() < 12 {
            continue;
        }

        let url = if href.starts_with("http") {
            href.to_string()
        } else {
            format!("https://www.anthropic.com{href}")
        };

        items.push(RawItem {
            kind: "news".into(),
            title,
            org: Some("Anthropic".into()),
            url,
            summary: None,
            // The index does not carry reliable dates; the item is dated when
            // we first see it, which is accurate enough for a daily sweep.
            published_at: None,
            deadline_at: None,
            location: None,
            is_online: None,
            source: "anthropic".into(),
            external_id: slug.to_string(),
            signals: ReachSignals {
                points: None,
                comments: None,
                primary: true,
            },
        });

        if items.len() >= 20 {
            break;
        }
    }

    Ok(items)
}
