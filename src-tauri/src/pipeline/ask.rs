//! The advisor.
//!
//! The point is not a chatbot — it is a second opinion grounded in what Scout
//! actually collected today. General web chat cannot tell him that the biggest
//! story of the day is worth *skipping*, because it does not know what else is
//! on his plate or which hackathon closes on Friday.
//!
//! Context is supplied by the caller rather than read here: the database
//! belongs to the frontend plugin, and a second reader competing for the same
//! file is a worse trade than passing a digest across the boundary.

use crate::settings::Settings;
use crate::sources::http::client;
use serde::{Deserialize, Serialize};

const GROQ_URL: &str = "https://api.groq.com/openai/v1/chat/completions";
const MODELS: &[&str] = &["openai/gpt-oss-120b", "openai/gpt-oss-20b"];

/// gpt-oss reasons before answering and that reasoning counts against the
/// budget. Too low and `content` comes back empty with finish_reason
/// "length" — a 200 response carrying nothing.
const MAX_TOKENS: u32 = 1400;

/// Enough to hold a short back-and-forth without letting an old thread crowd
/// out today's context.
const MAX_HISTORY: usize = 8;

const SYSTEM: &str = "\
You advise a second-year computer science student. He works in TypeScript, \
React, Python and C, is at university full time, and wants remote paid work, \
hackathons worth entering, and open-source contributions that build a record. \
He has no degree yet and cannot relocate.

You are given what his news reader collected today, plus his current tasks. \
Answer only from that. If the answer is not in the context, say so plainly \
rather than inventing something.

Be specific and short — a few sentences, or a handful of terse lines when he \
asks for a plan. Name actual items from the context. Say what to skip as \
readily as what to do; his time is the scarce resource, and telling him to \
ignore something is often the most useful answer. Never pad, never flatter, \
never restate the question back at him.";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Turn {
    /// "user" or "assistant".
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}
#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
    #[serde(default)]
    finish_reason: Option<String>,
}
#[derive(Debug, Deserialize)]
struct Message {
    #[serde(default)]
    content: String,
}

/// Answers a question against today's collected context.
///
/// Errors are returned as text rather than swallowed: an advisor that silently
/// stops answering is worse than one that says why it cannot.
pub async fn ask(
    question: &str,
    context: &str,
    history: &[Turn],
    settings: &Settings,
) -> Result<String, String> {
    let question = question.trim();
    if question.is_empty() {
        return Err("Ask something first.".into());
    }

    let keys = settings.groq_keys();
    if keys.is_empty() {
        return Err("No Groq API key set — add one in Settings.".into());
    }

    let mut messages = vec![
        serde_json::json!({ "role": "system", "content": SYSTEM }),
        serde_json::json!({
            "role": "system",
            "content": format!("Here is what Scout collected today:\n\n{context}")
        }),
    ];
    // Only the tail of the conversation, so a long thread cannot push today's
    // context out of the window.
    for turn in history.iter().rev().take(MAX_HISTORY).rev() {
        let role = if turn.role == "assistant" { "assistant" } else { "user" };
        messages.push(serde_json::json!({ "role": role, "content": turn.content }));
    }
    messages.push(serde_json::json!({ "role": "user", "content": question }));

    let mut last_error = String::new();

    for (n, key) in keys.iter().enumerate() {
        for model in MODELS {
            let body = serde_json::json!({
                "model": model,
                "temperature": 0.3,
                "max_tokens": MAX_TOKENS,
                "reasoning_effort": "low",
                "messages": messages,
            });

            let res = match client().post(GROQ_URL).bearer_auth(key).json(&body).send().await {
                Ok(r) => r,
                Err(e) => {
                    last_error = format!("network error: {e}");
                    continue;
                }
            };

            let status = res.status();
            if !status.is_success() {
                let detail: String = res.text().await.unwrap_or_default().chars().take(120).collect();
                last_error = if status.as_u16() == 429 {
                    format!("key {} is rate limited — try again in a minute", n + 1)
                } else {
                    format!("HTTP {status}: {detail}")
                };
                continue;
            }

            let parsed = match res.json::<ChatResponse>().await {
                Ok(p) => p,
                Err(e) => {
                    last_error = format!("unreadable reply: {e}");
                    continue;
                }
            };

            let Some(choice) = parsed.choices.first() else {
                last_error = "the model returned nothing".into();
                continue;
            };

            let text = choice.message.content.trim();
            if text.is_empty() {
                // Almost always the reasoning-ate-the-budget case.
                last_error = match choice.finish_reason.as_deref() {
                    Some("length") => "the answer was cut off before it started".into(),
                    other => format!("empty reply ({})", other.unwrap_or("no reason given")),
                };
                continue;
            }

            return Ok(text.to_string());
        }
    }

    Err(last_error)
}

#[tauri::command]
pub async fn ask_advisor(
    question: String,
    context: String,
    history: Vec<Turn>,
) -> Result<String, String> {
    ask(&question, &context, &history, &crate::settings::load()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn an_empty_question_is_refused_without_a_network_call() {
        let s = Settings::default();
        assert!(ask("   ", "ctx", &[], &s).await.is_err());
    }

    #[tokio::test]
    async fn a_missing_key_says_so_rather_than_failing_silently() {
        let s = Settings::default();
        let err = ask("what should I do?", "ctx", &[], &s).await.unwrap_err();
        assert!(err.contains("Settings"), "got: {err}");
    }

    #[test]
    fn history_is_trimmed_to_the_most_recent_turns() {
        let history: Vec<Turn> = (0..20)
            .map(|i| Turn {
                role: "user".into(),
                content: format!("turn {i}"),
            })
            .collect();
        let kept: Vec<&Turn> = history.iter().rev().take(MAX_HISTORY).rev().collect();
        assert_eq!(kept.len(), MAX_HISTORY);
        // The tail, not the head — the recent turns are the relevant ones.
        assert_eq!(kept.last().unwrap().content, "turn 19");
    }
}
