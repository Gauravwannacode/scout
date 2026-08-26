//! The daily brief — the "anchor script" the whole product is named after.
//!
//! Per-item `why_line`s answer "why does this one matter". They do not answer
//! "what happened today", because nothing reads the day as a whole. This does:
//! one short passage over the top stories and openings together, so the
//! connections between them are visible.
//!
//! Gemini is preferred when a key is configured — its longer context reads the
//! day in one piece. Without one it falls back to Groq, which is already
//! configured for scoring, so the brief works out of the box rather than being
//! gated behind a second signup.

use super::ScoredItem;
use crate::settings::Settings;
use crate::sources::http::client;
use serde::Deserialize;

const GROQ_URL: &str = "https://api.groq.com/openai/v1/chat/completions";
const GEMINI_BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";

/// Tried in order. Google retires free-tier ids without notice — `2.5-flash`
/// already 404s for keys that once used it — so a single hardcoded model is a
/// guaranteed future outage.
const GEMINI_MODELS: &[&str] = &["gemini-flash-latest", "gemini-3.6-flash", "gemini-3.5-flash"];
const GROQ_MODELS: &[&str] = &["openai/gpt-oss-120b", "openai/gpt-oss-20b"];

/// How many items the brief reads. Enough to see the shape of the day without
/// spending the token budget that scoring needs.
const BRIEF_ITEMS: usize = 14;

const PROMPT: &str = "\
You write a short daily briefing for a second-year computer science student who \
wants remote work, hackathons worth entering, and open-source contributions.

Write 3 to 5 sentences of flowing prose. No lists, no headings, no preamble.

Open with the single most important thing that happened, and say plainly why it \
matters. If something big is barely being covered, say so — that is the most \
valuable thing you can tell him. Then, if there is an opportunity worth acting \
on this week, name it and say what to do. If the day is quiet, say that in one \
sentence rather than inflating it; a quiet day is useful information.

Address him as 'you'. No hype, no filler, never open with 'Today's briefing'.";

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}
#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}
#[derive(Debug, Deserialize)]
struct Message {
    content: String,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    #[serde(default)]
    candidates: Vec<Candidate>,
}
#[derive(Debug, Deserialize)]
struct Candidate {
    content: Content,
}
#[derive(Debug, Deserialize)]
struct Content {
    #[serde(default)]
    parts: Vec<Part>,
}
#[derive(Debug, Deserialize)]
struct Part {
    #[serde(default)]
    text: String,
}

/// Renders the day into the compact listing the model reads.
fn digest(items: &[ScoredItem]) -> String {
    items
        .iter()
        .take(BRIEF_ITEMS)
        .map(|i| {
            let badge = match i.badge.as_str() {
                "legendary" => " [BIG AND UNCOVERED]",
                "worth-knowing" => " [widely covered]",
                _ => "",
            };
            let closing = i
                .deadline_at
                .as_deref()
                .map(|d| format!(" (closes {})", &d[..10.min(d.len())]))
                .unwrap_or_default();
            format!(
                "- [{}] {}{badge}{closing}\n  {}",
                i.kind,
                i.title,
                i.why_line.as_deref().unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

async fn via_gemini(listing: &str, key: &str) -> Option<String> {
    for model in GEMINI_MODELS {
        let url = format!("{GEMINI_BASE}/{model}:generateContent?key={key}");
        let body = serde_json::json!({
            "systemInstruction": { "parts": [{ "text": PROMPT }] },
            "contents": [{ "parts": [{ "text": format!("Here is today:\n\n{listing}") }] }],
            "generationConfig": { "temperature": 0.4, "maxOutputTokens": 400 }
        });

        let Ok(res) = client().post(&url).json(&body).send().await else {
            continue;
        };
        if !res.status().is_success() {
            // A retired model id is the common case; try the next one.
            continue;
        }
        let Ok(parsed) = res.json::<GeminiResponse>().await else {
            continue;
        };
        let text = parsed
            .candidates
            .first()
            .map(|c| {
                c.content
                    .parts
                    .iter()
                    .map(|p| p.text.as_str())
                    .collect::<String>()
            })
            .unwrap_or_default();
        if !text.trim().is_empty() {
            return Some(text.trim().to_string());
        }
    }
    None
}

async fn via_groq(listing: &str, keys: &[String], why: &mut Vec<String>) -> Option<String> {
    if keys.is_empty() {
        why.push("no Groq key configured".into());
        return None;
    }
    for (n, key) in keys.iter().enumerate() {
        for model in GROQ_MODELS {
            let body = serde_json::json!({
                "model": model,
                "temperature": 0.4,
                // gpt-oss reasons before answering, and that reasoning counts
                // against max_tokens. At 400 the reasoning consumed the whole
                // budget and `content` came back empty with finish_reason
                // "length" — a 200 response carrying nothing. Low effort plus
                // real headroom leaves room for the actual brief.
                "max_tokens": 1200,
                "reasoning_effort": "low",
                "messages": [
                    { "role": "system", "content": PROMPT },
                    { "role": "user", "content": format!("Here is today:\n\n{listing}") }
                ]
            });

            let res = match client().post(GROQ_URL).bearer_auth(key).json(&body).send().await {
                Ok(r) => r,
                Err(e) => {
                    why.push(format!("key{} {model}: request failed ({e})", n + 1));
                    continue;
                }
            };
            let status = res.status();
            if !status.is_success() {
                // Rate limited or retired model: move on, but say which.
                let detail: String = res
                    .text()
                    .await
                    .unwrap_or_default()
                    .chars()
                    .take(90)
                    .collect();
                why.push(format!("key{} {model}: HTTP {status} {detail}", n + 1));
                continue;
            }
            let parsed = match res.json::<ChatResponse>().await {
                Ok(p) => p,
                Err(e) => {
                    why.push(format!("key{} {model}: bad JSON ({e})", n + 1));
                    continue;
                }
            };
            let text = parsed
                .choices
                .first()
                .map(|c| c.message.content.trim().to_string())
                .unwrap_or_default();
            if !text.is_empty() {
                return Some(text);
            }
            why.push(format!("key{} {model}: empty reply", n + 1));
        }
    }
    None
}

/// Writes the day's brief, or `None` when there is nothing to say and no way
/// to say it. A missing brief is a quiet absence in the UI, never an error.
pub async fn write_brief(
    items: &[ScoredItem],
    settings: &Settings,
) -> (Option<String>, Option<String>) {
    if items.is_empty() {
        return (None, None);
    }
    let listing = digest(items);
    let mut why: Vec<String> = Vec::new();

    let gemini = settings.gemini_api_key.trim();
    if !gemini.is_empty() {
        if let Some(text) = via_gemini(&listing, gemini).await {
            return (Some(text), None);
        }
        why.push("gemini failed".into());
    }

    match via_groq(&listing, &settings.groq_keys(), &mut why).await {
        Some(text) => (Some(text), None),
        // A missing brief is quiet in the UI, but the reason must never be —
        // silent degradation is how a broken feature goes unnoticed for weeks.
        None => (None, Some(why.join("; "))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(kind: &str, title: &str, badge: &str, deadline: Option<&str>) -> ScoredItem {
        ScoredItem {
            id: "1".into(),
            kind: kind.into(),
            title: title.into(),
            org: None,
            url: "https://example.com".into(),
            summary: None,
            published_at: None,
            deadline_at: deadline.map(str::to_string),
            source: "test".into(),
            external_id: "1".into(),
            significance: 80,
            reach: 20,
            badge: badge.into(),
            why_line: Some("Because it matters.".into()),
            corroborations: 1,
            first_seen_at: "2026-08-26T00:00:00Z".into(),
        }
    }

    #[test]
    fn the_digest_flags_a_big_but_uncovered_story() {
        // This flag is the single most valuable thing the brief can relay, so
        // it must survive into the text the model reads.
        let d = digest(&[item("news", "A quiet breakthrough", "legendary", None)]);
        assert!(d.contains("[BIG AND UNCOVERED]"), "got: {d}");
    }

    #[test]
    fn the_digest_marks_widely_covered_stories_differently() {
        let d = digest(&[item("news", "Everyone has this", "worth-knowing", None)]);
        assert!(d.contains("[widely covered]"), "got: {d}");
        assert!(!d.contains("BIG AND UNCOVERED"));
    }

    #[test]
    fn the_digest_carries_a_deadline_when_there_is_one() {
        let d = digest(&[item(
            "hackathon",
            "Some hackathon",
            "radar",
            Some("2026-09-13T11:59:00+05:30"),
        )]);
        assert!(d.contains("closes 2026-09-13"), "got: {d}");
    }

    #[test]
    fn the_digest_is_capped_so_it_cannot_eat_the_token_budget() {
        let many: Vec<ScoredItem> = (0..60)
            .map(|i| item("news", &format!("Story {i}"), "radar", None))
            .collect();
        let lines = digest(&many).lines().filter(|l| l.starts_with("- ")).count();
        assert_eq!(lines, BRIEF_ITEMS);
    }

    #[tokio::test]
    async fn an_empty_day_produces_no_brief_and_makes_no_call() {
        let settings = Settings::default();
        let (text, err) = write_brief(&[], &settings).await;
        assert!(text.is_none());
        assert!(err.is_none(), "an empty day is not a failure");
    }
}
