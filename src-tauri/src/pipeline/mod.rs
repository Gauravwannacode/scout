pub mod ask;
pub mod brief;
pub mod cluster;
pub mod prefilter;
pub mod reach;
pub mod score;

use crate::settings;
use crate::sources;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A story after clustering and scoring, ready for the database and the UI.
///
/// Deserialize matters as much as Serialize here: a background run parks these
/// on disk when the window is closed, and reads them back when it opens.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoredItem {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub org: Option<String>,
    pub url: String,
    pub summary: Option<String>,
    pub published_at: Option<String>,
    pub deadline_at: Option<String>,
    /// Where it happens. "Near me" filtering depends entirely on this.
    pub location: Option<String>,
    pub is_online: Option<bool>,
    pub source: String,
    pub external_id: String,
    pub significance: u32,
    pub reach: u32,
    pub badge: String,
    pub why_line: Option<String>,
    pub corroborations: u32,
    pub first_seen_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineResult {
    pub items: Vec<ScoredItem>,
    pub counts: HashMap<String, usize>,
    pub errors: HashMap<String, String>,
    pub offline: bool,
    /// True when significance came from the heuristic, not the model. The UI
    /// must say so rather than presenting these as considered judgements.
    pub provisional_scores: bool,
    pub score_error: Option<String>,
    /// Items scored by the model. Below `story_count` means some fell back to
    /// the heuristic even though others succeeded.
    pub model_scored: usize,
    /// Distinct stories after clustering, versus raw items fetched.
    pub raw_count: usize,
    pub story_count: usize,
    /// Openings dropped by the rules pass, keyed by reason.
    pub filtered: HashMap<String, usize>,
    /// The day read as a whole. None when there was nothing to say, or no
    /// model available to say it — a quiet absence, never an error.
    pub brief: Option<String>,
    /// Why no brief was written, when one was expected.
    pub brief_error: Option<String>,
}

/// Big *and* barely covered. The rare combination worth interrupting him for.
const LEGENDARY_SIGNIFICANCE: u32 = 70;
const LEGENDARY_REACH: u32 = 35;

/// Significance decides the badge floor; reach only ever decides which of the
/// two upper badges applies. Nothing is demoted for being popular.
pub fn badge_for(significance: u32, reach: u32) -> &'static str {
    if significance >= LEGENDARY_SIGNIFICANCE && reach <= LEGENDARY_REACH {
        "legendary"
    } else if significance >= 55 {
        "worth-knowing"
    } else {
        "radar"
    }
}

/// Fetch, cluster, measure reach, score significance, assign badges.
///
/// Ordering is by significance alone. Reach is computed first only because
/// clustering produces it as a by-product.
pub async fn run() -> PipelineResult {
    let report = sources::fetch_all().await;
    let raw_count = report.items.len();

    if report.offline {
        return PipelineResult {
            items: Vec::new(),
            counts: report.counts,
            errors: report.errors,
            offline: true,
            provisional_scores: false,
            score_error: None,
            model_scored: 0,
            raw_count,
            story_count: 0,
            filtered: HashMap::new(),
            brief: None,
            brief_error: None,
        };
    }

    let now = Utc::now();

    // Drop openings he cannot take before anything is clustered or scored.
    // News is never filtered here. Counting the reasons keeps the filter
    // honest: a rule that quietly eats everything shows up in the report.
    let mut filtered: HashMap<String, usize> = HashMap::new();
    let kept: Vec<_> = report
        .items
        .into_iter()
        .filter(|item| match prefilter::reject_reason(item, now) {
            Some(reason) => {
                *filtered.entry(reason.as_str().to_string()).or_insert(0) += 1;
                false
            }
            None => true,
        })
        .collect();

    let clusters = cluster::cluster(kept);
    let cfg = settings::load();
    let outcome = score::score_all(&clusters, &cfg).await;

    let mut items: Vec<ScoredItem> = clusters
        .iter()
        .zip(outcome.scores.iter())
        .map(|(c, s)| {
            let reach = reach::reach_score(c, now);
            let significance = s.significance;
            ScoredItem {
                id: uuid::Uuid::new_v4().to_string(),
                kind: c.lead.kind.clone(),
                title: c.lead.title.clone(),
                org: c.lead.org.clone(),
                url: c.lead.url.clone(),
                summary: c.lead.summary.clone(),
                published_at: c.lead.published_at.map(|d| d.to_rfc3339()),
                deadline_at: c.lead.deadline_at.map(|d| d.to_rfc3339()),
                location: c.lead.location.clone(),
                is_online: c.lead.is_online,
                source: c.lead.source.clone(),
                external_id: c.lead.external_id.clone(),
                significance,
                reach,
                badge: badge_for(significance, reach).to_string(),
                why_line: Some(s.why_line.clone()).filter(|w| !w.is_empty()),
                corroborations: c.corroborations() as u32,
                first_seen_at: now.to_rfc3339(),
            }
        })
        .collect();

    // Significance is the sort key, full stop.
    items.sort_by(|a, b| b.significance.cmp(&a.significance));

    // Written after ranking so the brief reads the day in the order he will.
    let (brief, brief_error) = brief::write_brief(&items, &cfg).await;

    PipelineResult {
        story_count: items.len(),
        items,
        counts: report.counts,
        errors: report.errors,
        offline: false,
        provisional_scores: outcome.used_fallback,
        score_error: outcome.error,
        model_scored: outcome.model_scored,
        raw_count,
        filtered,
        brief,
        brief_error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn big_and_quiet_is_legendary() {
        assert_eq!(badge_for(84, 12), "legendary");
    }

    #[test]
    fn a_popular_big_story_is_never_demoted_below_worth_knowing() {
        // The correction that matters most: reach must not bury a big story.
        assert_eq!(badge_for(91, 88), "worth-knowing");
    }

    #[test]
    fn a_quiet_but_minor_story_is_not_legendary() {
        // Obscurity alone earns nothing — it must be big as well.
        assert_eq!(badge_for(30, 5), "radar");
    }

    #[test]
    fn significance_alone_decides_order() {
        let mut items = vec![(50u32, 5u32), (91, 88), (70, 30)];
        items.sort_by(|a, b| b.0.cmp(&a.0));
        assert_eq!(
            items[0],
            (91, 88),
            "the biggest story leads even though everyone has it"
        );
    }

    #[test]
    fn badge_boundaries_are_inclusive_as_documented() {
        assert_eq!(badge_for(70, 35), "legendary");
        assert_eq!(badge_for(69, 35), "worth-knowing");
        assert_eq!(badge_for(70, 36), "worth-knowing");
    }
}
