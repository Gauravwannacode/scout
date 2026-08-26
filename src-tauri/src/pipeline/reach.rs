use super::cluster::Cluster;
use chrono::{DateTime, Duration, Utc};

/// How widely known a story already is, 0-100.
///
/// This never decides ordering — `significance` does. Reach only chooses the
/// badge, so that a big story everyone is covering still reaches him, while a
/// big story nobody has noticed gets flagged.
///
/// Deliberately arithmetic: no model call, so it works with the network off
/// and costs nothing to run over every item.
pub fn reach_score(c: &Cluster, now: DateTime<Utc>) -> u32 {
    let mut score = 0.0f32;

    // How many independent outlets carried it. The strongest signal by far —
    // four outlets means the crowd has already arrived.
    let sources = c.corroborations();
    score += match sources {
        0 | 1 => 0.0,
        2 => 22.0,
        3 => 38.0,
        4 => 50.0,
        _ => 60.0,
    };

    // Audience on the aggregators, compressed — the gap between 5 and 50
    // points matters much more than between 500 and 1000.
    if let Some(points) = c.best_points() {
        score += (points as f32 + 1.0).ln() * 6.0;
    }
    if let Some(comments) = c.best_comments() {
        score += (comments as f32 + 1.0).ln() * 3.0;
    }

    // Something published days ago has had time to spread; something from an
    // hour ago has not, whatever its eventual audience.
    if let Some(published) = c.lead.published_at {
        let age = now.signed_duration_since(published);
        score += if age < Duration::hours(3) {
            0.0
        } else if age < Duration::hours(12) {
            6.0
        } else if age < Duration::days(1) {
            12.0
        } else if age < Duration::days(3) {
            20.0
        } else {
            26.0
        };
    } else {
        // No date is most often an index scrape; assume mid-range rather than
        // treating it as brand new and over-flagging it.
        score += 12.0;
    }

    // A vendor's own announcement is the origin of a story, not evidence that
    // it has spread. Coverage *about* something implies an audience already.
    if !c.has_primary() {
        score += 8.0;
    }

    score.clamp(0.0, 100.0).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::{RawItem, ReachSignals};

    fn item(source: &str, points: Option<u32>, primary: bool, age_hours: i64) -> RawItem {
        RawItem {
            kind: "news".into(),
            title: "A story".into(),
            org: None,
            url: format!("https://{source}.com/x"),
            summary: None,
            published_at: Some(Utc::now() - Duration::hours(age_hours)),
            deadline_at: None,
            source: source.into(),
            external_id: source.into(),
            signals: ReachSignals {
                points,
                comments: None,
                primary,
            },
        }
    }

    fn cluster_of(members: Vec<RawItem>) -> Cluster {
        Cluster {
            lead: members[0].clone(),
            members,
        }
    }

    #[test]
    fn a_lone_fresh_story_scores_low() {
        let c = cluster_of(vec![item("arxiv", None, true, 2)]);
        assert!(
            reach_score(&c, Utc::now()) < 35,
            "one source, hours old, must read as uncrowded"
        );
    }

    #[test]
    fn a_widely_covered_story_scores_high() {
        let c = cluster_of(vec![
            item("verge", Some(400), false, 20),
            item("ars", None, false, 20),
            item("techcrunch", None, false, 20),
            item("hn-front", Some(900), false, 20),
        ]);
        assert!(
            reach_score(&c, Utc::now()) > 70,
            "four outlets and 900 points is crowded"
        );
    }

    #[test]
    fn more_sources_always_means_more_reach() {
        let one = cluster_of(vec![item("verge", Some(10), false, 5)]);
        let three = cluster_of(vec![
            item("verge", Some(10), false, 5),
            item("ars", Some(10), false, 5),
            item("techcrunch", Some(10), false, 5),
        ]);
        assert!(reach_score(&three, Utc::now()) > reach_score(&one, Utc::now()));
    }

    #[test]
    fn scores_stay_in_range_under_absurd_input() {
        let c = cluster_of(vec![item("hn-front", Some(u32::MAX), false, 900)]);
        assert!(reach_score(&c, Utc::now()) <= 100);
    }
}
