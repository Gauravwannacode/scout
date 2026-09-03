use super::cluster::Cluster;
use crate::settings::Settings;
use crate::sources::http::client;
use serde::{Deserialize, Serialize};

const GROQ_URL: &str = "https://api.groq.com/openai/v1/chat/completions";
/// Tried in order. Groq's free catalogue is retired and replaced without
/// notice — `llama-3.3-70b-versatile` worked until it simply 404'd — so a
/// single hardcoded model is a guaranteed future outage. On `model_not_found`
/// the scorer moves to the next entry instead of failing the run.
///
/// Reasoning-style models that emit <think> blocks are deliberately excluded:
/// they bury the JSON and waste the daily token budget.
const MODELS: &[&str] = &[
    "openai/gpt-oss-120b",
    "openai/gpt-oss-20b",
    "groq/compound",
];

/// Items per model call. Large enough to keep the number of requests small,
/// small enough that one malformed response costs little.
const BATCH: usize = 20;

/// How many items the model is actually asked to judge, per group.
///
/// Groq's free tier is capped at 8000 *tokens per minute*, so a 500-story
/// sweep cannot be model-scored — the first two batches exhaust every key and
/// the remaining 95% silently fall back to the heuristic.
///
/// Scoring everything was never useful anyway: he reads one lead story, a
/// handful in the picker, and a list of openings. The cheap heuristic ranks
/// the full set, and only the top candidates get the expensive judgement.
const NEWS_CANDIDATES: usize = 60;
const OPENING_CANDIDATES: usize = 40;

/// Paced to stay under the tokens-per-minute cap. A batch costs roughly 2.5k
/// tokens, so three per minute is about the ceiling.
const BATCH_PAUSE_SECS: u64 = 21;

/// How long to wait out a rate limit before retrying a batch. The free tier
/// limit is per minute, so a short wait usually clears it — retiring the key
/// immediately would waste a key that is about to work again.
const RATE_LIMIT_BACKOFF_SECS: u64 = 25;

#[derive(Debug, Clone, Serialize)]
pub struct Scored {
    /// 0-100, how big a deal this is. The sort key.
    pub significance: u32,
    /// One second-person sentence on why it matters to him.
    pub why_line: String,
}

const SYSTEM_PROMPT: &str = "\
You rate AI and software news for a second-year computer science student who is \
looking for remote internships and gigs, enters hackathons, and contributes to open source.

For each item return:
- significance: 0-100, how big a deal this is in AI and software generally. \
Judge the news itself, NOT how popular it is and NOT how well it suits this \
particular reader. A frontier model release, a major framework breaking change, \
a large acquisition, or a genuine research result scores high. Routine patch \
notes, listicles, opinion pieces and marketing score low.
- why_line: one sentence, max 30 words, addressed to the reader as 'you', \
saying concretely why this matters to them or what to do about it. No hype, \
no filler, never restate the headline.

Return ONLY a JSON object of the form {\"items\": [...]}, with one entry per \
input item in the same order, each having keys \"significance\" and \"why_line\".";

/// Openings are judged on a completely different question to news.
///
/// "How big a deal is this in AI and software" is the wrong question to ask of
/// a hackathon — what matters is whether he can realistically take it on and
/// get something out of it. Scoring both with the news prompt ranked openings
/// by how newsworthy they sounded, which is close to meaningless here.
const OPENING_PROMPT: &str = "You rate opportunities for a second-year computer science student. He is at university full time, works in TypeScript, React, Python and C, and wants remote paid work, hackathons worth entering, and open-source contributions that build a track record. He does not have a degree yet and cannot relocate.

For each item return:
- significance: 0-100, how worth HIS time this is. Score high when the work is remote and genuinely open to a student with no degree, when a hackathon has real prizes and a field small enough to place in, when a company is early enough that a direct message reaches a founder, or when an open-source issue is a realistic first contribution. Score low when it needs years of experience, a finished degree, on-site presence, or when the deadline is too close to do anything useful. An item closing in under two days is rarely worth starting.
- why_line: one sentence, max 30 words, addressed to him as 'you', saying what to actually do about it and why it suits him. Be concrete. Never restate the title.

Return ONLY a JSON object of the form {\"items\": [...]}, with one entry per input item in the same order, each having keys \"significance\" and \"why_line\".";

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

/// Distinguishes "this key is spent" from "something is actually broken".
/// Only the former is worth retrying on another key.
#[derive(Debug)]
pub enum ScoreError {
    KeyExhausted(String),
    /// This model id is gone from the provider's catalogue.
    ModelUnavailable(String),
    Other(String),
}

impl std::fmt::Display for ScoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScoreError::KeyExhausted(m) => write!(f, "key exhausted: {m}"),
            ScoreError::ModelUnavailable(m) => write!(f, "model unavailable: {m}"),
            ScoreError::Other(m) => write!(f, "{m}"),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ScoreRow {
    #[serde(default)]
    significance: Option<f32>,
    #[serde(default)]
    why_line: Option<String>,
}

/// Models sometimes wrap JSON in prose or fences despite instructions.
/// Pulling the outermost array is more reliable than trusting the format.
fn extract_json_array(text: &str) -> Option<&str> {
    let start = text.find('[')?;
    let end = text.rfind(']')?;
    if end > start {
        Some(&text[start..=end])
    } else {
        None
    }
}

#[derive(Debug, Deserialize)]
struct ScoreEnvelope {
    #[serde(default)]
    items: Vec<ScoreRow>,
}

/// Reads the rows out of whatever shape came back.
///
/// JSON mode should always give `{"items": [...]}`, but a model that ignores
/// it still tends to emit a usable array — worth salvaging rather than
/// discarding twenty scored items over a formatting slip.
fn parse_rows(content: &str) -> Option<Vec<ScoreRow>> {
    if let Ok(envelope) = serde_json::from_str::<ScoreEnvelope>(content) {
        if !envelope.items.is_empty() {
            return Some(envelope.items);
        }
    }
    let array = extract_json_array(content)?;
    serde_json::from_str::<Vec<ScoreRow>>(array).ok()
}

async fn score_batch(
    batch: &[&Cluster],
    key: &str,
    model: &str,
    prompt: &str,
) -> Result<Vec<Scored>, ScoreError> {
    let listing: String = batch
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let summary = c
                .lead
                .summary
                .as_deref()
                .map(|s| format!("\n   {}", &s.chars().take(240).collect::<String>()))
                .unwrap_or_default();
            // Deadlines are decision-relevant for openings: the model is
            // told to discount anything closing too soon to act on.
            let closing = c
                .lead
                .deadline_at
                .map(|d| {
                    let days = (d - chrono::Utc::now()).num_days();
                    if days < 0 {
                        " (closed)".to_string()
                    } else {
                        format!(" (closes in {days} days)")
                    }
                })
                .unwrap_or_default();
            format!(
                "{}. [{}] {}{}{}",
                i + 1,
                c.lead.source,
                c.lead.title,
                closing,
                summary
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let body = serde_json::json!({
        "model": model,
        "temperature": 0.2,
        // Without this the model intermittently wraps the JSON in prose and
        // the whole batch is thrown away. Constrained decoding needs an
        // object at the root, which is why the schema nests under "items".
        "response_format": { "type": "json_object" },
        "messages": [
            { "role": "system", "content": prompt },
            { "role": "user", "content": format!("Rate these {} items:\n\n{listing}", batch.len()) }
        ]
    });

    let res = client()
        .post(GROQ_URL)
        .bearer_auth(key)
        .json(&body)
        .send()
        .await
        .map_err(|e| ScoreError::Other(format!("groq request failed: {e}")))?;

    let status = res.status();
    if !status.is_success() {
        let detail = res.text().await.unwrap_or_default();
        let detail: String = detail.chars().take(200).collect();
        // 429 and 413 mean this key is spent or the request is too large for
        // its tier — both are worth retrying on a different key.
        if status.as_u16() == 429 || status.as_u16() == 413 {
            return Err(ScoreError::KeyExhausted(format!("HTTP {status}: {detail}")));
        }
        if status.as_u16() == 404 || detail.contains("model_not_found") {
            return Err(ScoreError::ModelUnavailable(format!("{model}: {detail}")));
        }
        return Err(ScoreError::Other(format!("groq HTTP {status}: {detail}")));
    }

    let parsed: ChatResponse = res
        .json()
        .await
        .map_err(|e| ScoreError::Other(format!("groq response not JSON: {e}")))?;
    let content = parsed
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .ok_or_else(|| ScoreError::Other("groq returned no choices".into()))?;

    let rows = parse_rows(&content).ok_or_else(|| {
        let preview: String = content.chars().take(120).collect();
        ScoreError::Other(format!("unreadable score JSON: {preview}"))
    })?;

    // A short reply must not silently misalign scores with items, so pad
    // rather than zip — index i always belongs to batch item i.
    Ok((0..batch.len())
        .map(|i| {
            let row = rows.get(i);
            Scored {
                significance: row
                    .and_then(|r| r.significance)
                    .unwrap_or(0.0)
                    .clamp(0.0, 100.0)
                    .round() as u32,
                why_line: row
                    .and_then(|r| r.why_line.clone())
                    .unwrap_or_default()
                    .trim()
                    .to_string(),
            }
        })
        .collect())
}

/// Rough significance without a model, used when no API key is configured or
/// the call fails.
///
/// It is a stand-in, not an equal: it can tell a vendor announcement from a
/// blog post, but it cannot tell an important paper from a dull one. The UI
/// says so rather than passing these off as real judgements.
pub fn heuristic_significance(c: &Cluster) -> Scored {
    if crate::sources::opps::is_opening(&c.lead) {
        return heuristic_opening(c);
    }

    let mut score: f32 = 24.0;

    score += match c.lead.source.as_str() {
        "openai" | "deepmind" | "googleai" | "anthropic" => 30.0,
        "arxiv" | "hf-papers" => 18.0,
        "gh-releases" => 16.0,
        "hn-front" => 14.0,
        "verge" | "ars" | "techcrunch" => 10.0,
        _ => 4.0,
    };

    if let Some(points) = c.best_points() {
        score += (points as f32 + 1.0).ln() * 3.5;
    }
    // Several outlets bothering to cover something is weak evidence it matters.
    score += (c.corroborations().saturating_sub(1) as f32) * 5.0;

    Scored {
        significance: score.clamp(0.0, 100.0).round() as u32,
        why_line: String::new(),
    }
}

/// Fallback ranking for openings when no model is available.
///
/// Leans on source, because source is the strongest cheap proxy for how
/// crowded something is: a company that has not posted a listing anywhere is
/// worth more of his attention than the same role on a job board seen by
/// thousands.
fn heuristic_opening(c: &Cluster) -> Scored {
    let mut score: f32 = 30.0;

    score += match c.lead.source.as_str() {
        // No listing exists yet — he would be first, not in a queue.
        "yc" | "launch-hn" => 30.0,
        "show-hn" => 20.0,
        // Real prizes, and registrant counts small enough to place in.
        "devpost" | "devfolio" | "unstop" => 18.0,
        // A concrete first contribution.
        "gh-issues" => 16.0,
        "hn-hiring" => 14.0,
        // Aggregated boards: everyone has already seen these.
        _ => 4.0,
    };

    if let Some(deadline) = c.lead.deadline_at {
        let days = (deadline - chrono::Utc::now()).num_days();
        // Too soon to start is nearly as useless as already closed.
        score += match days {
            d if d < 0 => -30.0,
            0..=1 => -15.0,
            2..=6 => 8.0,
            7..=30 => 12.0,
            _ => 4.0,
        };
    }

    Scored {
        significance: score.clamp(0.0, 100.0).round() as u32,
        why_line: String::new(),
    }
}

pub struct ScoreOutcome {
    pub scores: Vec<Scored>,
    /// True when every score came from the heuristic rather than the model.
    pub used_fallback: bool,
    /// How many items the model actually scored. Less than the total means a
    /// partial fallback, which would otherwise degrade the ranking silently —
    /// news succeeding is not evidence that openings did.
    pub model_scored: usize,
    pub error: Option<String>,
}

/// Scores every cluster, falling back to the heuristic per batch on failure.
///
/// A model outage degrades the ranking rather than emptying the app: the user
/// still gets a brief, and the UI can tell them the scores are provisional.
pub async fn score_all(clusters: &[Cluster], settings: &Settings) -> ScoreOutcome {
    if !settings.has_groq() {
        return ScoreOutcome {
            scores: clusters.iter().map(heuristic_significance).collect(),
            used_fallback: true,
            model_scored: 0,
            error: Some("no Groq API key configured".into()),
        };
    }

    // News and openings answer different questions, so they are scored in
    // separate passes with separate prompts, then put back in the caller's
    // order. Indices are carried through so nothing can be misaligned.
    let (mut opening_idx, mut news_idx): (Vec<usize>, Vec<usize>) = (0..clusters.len())
        .partition(|&i| crate::sources::opps::is_opening(&clusters[i].lead));

    // Rank cheaply first, then spend the model only on the top of each group.
    let heuristics: Vec<Scored> = clusters.iter().map(heuristic_significance).collect();
    let by_heuristic = |a: &usize, b: &usize| {
        heuristics[*b].significance.cmp(&heuristics[*a].significance)
    };
    news_idx.sort_by(by_heuristic);
    opening_idx.sort_by(by_heuristic);
    news_idx.truncate(NEWS_CANDIDATES);
    opening_idx.truncate(OPENING_CANDIDATES);

    let keys = settings.groq_keys();
    let mut scores = Vec::with_capacity(clusters.len());
    let mut first_error: Option<String> = None;
    let mut any_model_scores = false;

    // Index of the key currently in use. Groq's free tier has a small daily
    // budget, so an exhausted key advances this rather than ending the run.
    let mut key_index = 0usize;
    // Index into MODELS. Advanced permanently when a model id turns out to be
    // retired, so the rest of the run does not keep asking for a dead model.
    let mut model_index = 0usize;

    // Start from the heuristic everywhere; the model overwrites only what it
    // is asked to judge.
    let mut by_index: Vec<Option<Scored>> =
        heuristics.iter().cloned().map(Some).collect();
    let mut model_scored = 0usize;

    for (indices, prompt) in [
        (&news_idx, SYSTEM_PROMPT),
        (&opening_idx, OPENING_PROMPT),
    ] {
        for chunk in indices.chunks(BATCH) {
            let refs: Vec<&Cluster> = chunk.iter().map(|&i| &clusters[i]).collect();
            let mut batch_done = false;
            let mut backed_off = false;

            if model_scored > 0 {
                tokio::time::sleep(std::time::Duration::from_secs(BATCH_PAUSE_SECS)).await;
            }

            // Try each remaining key, and each remaining model, once per batch.
            while key_index < keys.len() && model_index < MODELS.len() && !batch_done {
                match score_batch(&refs, &keys[key_index], MODELS[model_index], prompt).await {
                    Ok(batch_scores) => {
                        any_model_scores = true;
                        model_scored += chunk.len();
                        for (&i, scored) in chunk.iter().zip(batch_scores) {
                            by_index[i] = Some(scored);
                        }
                        batch_done = true;
                    }
                    Err(ScoreError::KeyExhausted(msg)) => {
                        if !backed_off {
                            // The cap is per minute, so wait it out once before
                            // giving up on a key that is about to recover.
                            backed_off = true;
                            tokio::time::sleep(std::time::Duration::from_secs(
                                RATE_LIMIT_BACKOFF_SECS,
                            ))
                            .await;
                            continue;
                        }
                        first_error.get_or_insert(format!("key {} spent ({msg})", key_index + 1));
                        // Retire this key for the rest of the run and retry the
                        // same batch on the next one.
                        key_index += 1;
                    }
                    Err(ScoreError::ModelUnavailable(msg)) => {
                        first_error.get_or_insert(msg);
                        model_index += 1;
                    }
                    Err(other) => {
                        // A genuine fault is not fixed by another key or model.
                        first_error.get_or_insert(other.to_string());
                        break;
                    }
                }
            }

            // Nothing to do on failure: the slot already holds its heuristic.
        }
    }

    // Every slot was seeded with a heuristic score, so none can be empty.
    scores.extend(
        by_index
            .into_iter()
            .enumerate()
            .map(|(i, s)| s.unwrap_or_else(|| heuristics[i].clone())),
    );

    ScoreOutcome {
        scores,
        used_fallback: !any_model_scores,
        model_scored,
        error: first_error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_is_recovered_from_a_fenced_reply() {
        let reply = "Here you go:\n```json\n[{\"significance\": 80}]\n```\nHope that helps!";
        assert_eq!(extract_json_array(reply), Some("[{\"significance\": 80}]"));
    }

    #[test]
    fn a_reply_with_no_array_is_rejected_rather_than_guessed() {
        assert_eq!(extract_json_array("I cannot do that"), None);
    }

    use crate::sources::{RawItem, ReachSignals};
    use chrono::{Duration, Utc};

    fn cluster_of(kind: &str, source: &str, deadline_days: Option<i64>) -> Cluster {
        let lead = RawItem {
            kind: kind.into(),
            title: "Something".into(),
            org: None,
            url: "https://example.com".into(),
            summary: None,
            published_at: None,
            deadline_at: deadline_days.map(|d| Utc::now() + Duration::days(d)),
            location: None,
            is_online: None,
            source: source.into(),
            external_id: "1".into(),
            signals: ReachSignals::default(),
        };
        Cluster {
            members: vec![lead.clone()],
            lead,
        }
    }

    #[test]
    fn an_opening_is_scored_by_the_opening_heuristic_not_the_news_one() {
        // A YC company is worthless as *news* — no points, no coverage — but
        // it is the most valuable kind of opening he can get.
        let yc = heuristic_significance(&cluster_of("company", "yc", None));
        let board = heuristic_significance(&cluster_of("job", "remoteok", None));
        assert!(
            yc.significance > board.significance,
            "a company with no listing must outrank a job board everyone reads"
        );
    }

    #[test]
    fn an_opening_closing_tomorrow_is_discounted() {
        let soon = heuristic_significance(&cluster_of("hackathon", "devpost", Some(1)));
        let roomy = heuristic_significance(&cluster_of("hackathon", "devpost", Some(14)));
        assert!(
            roomy.significance > soon.significance,
            "too soon to start is nearly as useless as already closed"
        );
    }

    #[test]
    fn an_expired_opening_scores_near_the_floor() {
        let expired = heuristic_significance(&cluster_of("hackathon", "devpost", Some(-3)));
        let open = heuristic_significance(&cluster_of("hackathon", "devpost", Some(14)));
        assert!(expired.significance < open.significance);
    }

    #[test]
    fn news_still_routes_through_the_news_heuristic() {
        // A vendor announcement should score well as news; the opening
        // heuristic knows nothing about "openai" and would score it near zero.
        let news = heuristic_significance(&cluster_of("news", "openai", None));
        assert!(news.significance > 40, "got {}", news.significance);
    }

    #[test]
    fn scores_are_clamped_into_range() {
        let rows: Vec<ScoreRow> =
            serde_json::from_str(r#"[{"significance": 900}, {"significance": -5}]"#).unwrap();
        let clamped: Vec<u32> = rows
            .iter()
            .map(|r| r.significance.unwrap_or(0.0).clamp(0.0, 100.0).round() as u32)
            .collect();
        assert_eq!(clamped, vec![100, 0]);
    }
}
