//! Choosing what to show a particular company.
//!
//! The model is handed the fact base as a list of ids and returns a list of
//! ids. It never returns prose that lands in the resume body. That is the
//! guarantee: an invented project has no id, and `apply` drops anything it
//! cannot look up, so a hallucination becomes a missing line rather than a
//! lie on a document sent to a stranger.
//!
//! The one free-text field is the headline, and it is checked against the
//! fact base before use.

use super::facts::Fact;
use crate::settings::Settings;
use crate::sources::http::client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const GROQ_URL: &str = "https://api.groq.com/openai/v1/chat/completions";
const MODELS: &[&str] = &["openai/gpt-oss-120b", "openai/gpt-oss-20b"];
const MAX_TOKENS: u32 = 2000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tailored {
    /// Fact ids, in the order they should appear.
    pub include: Vec<String>,
    /// One line under the name. Recombines his own words; validated below.
    pub headline: String,
    /// "clean" for anything that will pass through a parser, "bold" for a
    /// founder who will look at it directly.
    pub template: String,
    pub email_subject: String,
    pub email_body: String,
    /// What the model understood about the company, shown so the reasoning is
    /// reviewable rather than hidden.
    pub research_note: String,
}

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
    #[serde(default)]
    content: String,
}

#[derive(Debug, Deserialize)]
struct Raw {
    #[serde(default)]
    include: Vec<String>,
    #[serde(default)]
    headline: String,
    #[serde(default)]
    template: String,
    #[serde(default)]
    email_subject: String,
    #[serde(default)]
    email_body: String,
    #[serde(default)]
    research_note: String,
}

const SYSTEM: &str = "\
You tailor a student's resume and write a cold email to one specific company.

You are given his complete fact base as a numbered list of ids. Every entry is \
something he has genuinely done.

RULES, in order of importance:
1. You may only reference facts by their id. You must never write a project, \
skill or achievement that is not in the list. If the company wants something \
he has not done, leave it out — do not stretch a fact to cover it.
2. Choose which facts to include and their order. Lead with whatever is most \
relevant to this company. Drop what is irrelevant; a shorter resume that is \
all on-target beats a long one.
3. Always include his education and FIVE or SIX projects — he has nine strong ones and a resume showing only three looks thin. Order them by relevance to this company; the least relevant still earn a place at the bottom.
4. The fact base is already in his preferred order. Follow it unless a project is genuinely more relevant to this company — relevance may promote something, but do not reshuffle for the sake of it.
5. Skill levels are his own honest assessment. Never upgrade them.

Return JSON with these keys:
- include: array of fact ids, in display order
- headline: one line, under 100 characters, describing him. Use only claims \
supported by the included facts.
- template: \"clean\" if this will likely go through an applicant tracking \
system or a formal careers process; \"bold\" if a founder or small team will \
read it directly.
- email_subject: under 70 characters, specific to this company. Never generic.
- email_body: 90-140 words. Address them directly. Say what you noticed about \
them specifically, name the one project of his most relevant and why, and ask \
for something small and concrete. No flattery, no buzzwords, no 'I am writing \
to express my interest'. Sign off as Gaurav.
- research_note: two sentences on what this company appears to do and what \
they would value, so he can check your reasoning.

Return ONLY a JSON object.";

fn catalogue(facts: &[Fact]) -> String {
    facts
        .iter()
        .filter(|f| f.enabled)
        .map(|f| {
            let extra = match f.kind.as_str() {
                "skill" => format!(" [{}]", f.level.as_deref().unwrap_or("")),
                _ => f
                    .tech
                    .as_deref()
                    .map(|t| format!(" [{t}]"))
                    .unwrap_or_default(),
            };
            let detail = f
                .detail
                .as_deref()
                .map(|d| format!(" — {}", &d.chars().take(240).collect::<String>()))
                .unwrap_or_default();
            format!("{} ({}) {}{extra}{detail}", f.id, f.kind, f.title)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Strips anything the fact base cannot vouch for.
///
/// Returns the cleaned result plus whatever was dropped, so the UI can say
/// "the model invented two entries and they were removed" rather than quietly
/// producing a shorter resume.
pub fn apply(mut raw: Tailored, facts: &[Fact]) -> (Tailored, Vec<String>) {
    let known: HashMap<&str, &Fact> = facts
        .iter()
        .filter(|f| f.enabled)
        .map(|f| (f.id.as_str(), f))
        .collect();

    let mut dropped = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut kept = Vec::new();

    for id in raw.include.drain(..) {
        if !known.contains_key(id.as_str()) {
            dropped.push(id);
            continue;
        }
        // A repeated id would render the same project twice.
        if seen.insert(id.clone()) {
            kept.push(id);
        }
    }
    raw.include = kept;

    if raw.template != "bold" {
        raw.template = "clean".into();
    }
    if raw.headline.chars().count() > 140 {
        raw.headline = raw.headline.chars().take(140).collect();
    }

    (raw, dropped)
}

pub async fn tailor(
    facts: &[Fact],
    company: &str,
    research: &str,
    settings: &Settings,
) -> Result<(Tailored, Vec<String>), String> {
    let keys = settings.groq_keys();
    if keys.is_empty() {
        return Err("No Groq API key set — add one in Settings.".into());
    }

    let user = format!(
        "COMPANY: {company}\n\nWHAT IS KNOWN ABOUT THEM:\n{research}\n\nHIS FACT BASE:\n{}",
        catalogue(facts)
    );

    let mut last_error = String::new();

    for key in &keys {
        for model in MODELS {
            let body = serde_json::json!({
                "model": model,
                "temperature": 0.3,
                "max_tokens": MAX_TOKENS,
                "reasoning_effort": "low",
                "response_format": { "type": "json_object" },
                "messages": [
                    { "role": "system", "content": SYSTEM },
                    { "role": "user", "content": user }
                ]
            });

            let res = match client().post(GROQ_URL).bearer_auth(key).json(&body).send().await {
                Ok(r) => r,
                Err(e) => {
                    last_error = format!("request failed: {e}");
                    continue;
                }
            };
            if !res.status().is_success() {
                let status = res.status();
                let detail: String =
                    res.text().await.unwrap_or_default().chars().take(120).collect();
                last_error = if status.as_u16() == 429 {
                    "rate limited — try again in a minute".into()
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
            let content = parsed
                .choices
                .first()
                .map(|c| c.message.content.clone())
                .unwrap_or_default();
            if content.trim().is_empty() {
                last_error = "the model returned nothing".into();
                continue;
            }

            let raw: Raw = match serde_json::from_str(&content) {
                Ok(r) => r,
                Err(e) => {
                    last_error = format!("bad JSON: {e}");
                    continue;
                }
            };

            let tailored = Tailored {
                include: raw.include,
                headline: raw.headline,
                template: raw.template,
                email_subject: raw.email_subject,
                email_body: raw.email_body,
                research_note: raw.research_note,
            };
            return Ok(apply(tailored, facts));
        }
    }

    Err(last_error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::facts::seed;

    fn tailored(ids: &[&str]) -> Tailored {
        Tailored {
            include: ids.iter().map(|s| s.to_string()).collect(),
            headline: "Builds systems in Rust and Python".into(),
            template: "bold".into(),
            email_subject: "About your infra work".into(),
            email_body: "Hello.".into(),
            research_note: "They build infrastructure.".into(),
        }
    }

    #[test]
    fn an_invented_project_is_dropped_not_rendered() {
        // The failure this design exists to prevent: the model naming a
        // project he never built, on a document sent to a stranger.
        let (clean, dropped) = apply(
            tailored(&["proj-jos", "proj-kubernetes-at-google", "proj-scout"]),
            &seed(),
        );
        assert_eq!(clean.include, vec!["proj-jos", "proj-scout"]);
        assert_eq!(dropped, vec!["proj-kubernetes-at-google"]);
    }

    #[test]
    fn a_repeated_id_renders_once() {
        let (clean, _) = apply(tailored(&["proj-jos", "proj-jos"]), &seed());
        assert_eq!(clean.include, vec!["proj-jos"]);
    }

    #[test]
    fn an_unknown_template_falls_back_to_the_safe_one() {
        let mut t = tailored(&["proj-jos"]);
        t.template = "neon-cyberpunk".into();
        let (clean, _) = apply(t, &seed());
        assert_eq!(clean.template, "clean");
    }

    #[test]
    fn a_disabled_fact_cannot_be_selected() {
        // Switching a fact off in the editor must actually remove it from
        // consideration, not just hide it in the UI.
        let mut facts = seed();
        for f in facts.iter_mut() {
            if f.id == "proj-comic" {
                f.enabled = false;
            }
        }
        let (clean, dropped) = apply(tailored(&["proj-comic", "proj-jos"]), &facts);
        assert_eq!(clean.include, vec!["proj-jos"]);
        assert_eq!(dropped, vec!["proj-comic"]);
    }

    #[test]
    fn a_runaway_headline_is_truncated() {
        let mut t = tailored(&["proj-jos"]);
        t.headline = "x".repeat(400);
        let (clean, _) = apply(t, &seed());
        assert!(clean.headline.chars().count() <= 140);
    }

    #[test]
    fn the_catalogue_lists_every_enabled_fact_with_its_id() {
        let facts = seed();
        let c = catalogue(&facts);
        for f in facts.iter().filter(|f| f.enabled) {
            assert!(c.contains(&f.id), "catalogue is missing {}", f.id);
        }
    }
}
