use crate::sources::RawItem;
use chrono::{DateTime, Utc};

/// Phrases that mean the role is not open to a second-year student. Matched
/// against title and summary before any model sees the item, which keeps the
/// token budget on things he could actually take.
const SENIOR_MARKERS: &[&str] = &[
    "senior ",
    "sr. ",
    "sr ",
    "staff ",
    "principal ",
    "lead ",
    "head of",
    "director",
    "vp of",
    "vice president",
    "manager",
    "architect",
    "10+ years",
    "8+ years",
    "7+ years",
    "6+ years",
    "5+ years",
    "4+ years",
    "minimum of 5 years",
    "minimum of 4 years",
    "phd required",
    "ph.d. required",
];

/// Hard credential gates. He asked specifically for work that does not require
/// a finished degree or a certificate.
const CREDENTIAL_GATES: &[&str] = &[
    "bachelor's degree required",
    "bachelors degree required",
    "master's degree required",
    "masters degree required",
    "degree is required",
    "must have a degree",
    "certification required",
    "certified professional required",
    "security clearance",
];

/// Roles that are not software work at all. Boards like RemoteOK carry plenty
/// of retail and support listings that match nothing he wants.
const OFF_TOPIC: &[&str] = &[
    "retail store",
    "sales representative",
    "account executive",
    "customer support agent",
    "call center",
    "truck driver",
    "nurse",
    "recruiter",
    "insurance agent",
    "real estate",
];

fn haystack(item: &RawItem) -> String {
    format!(
        "{} {}",
        item.title.to_lowercase(),
        item.summary.as_deref().unwrap_or("").to_lowercase()
    )
}

/// Why an opening was dropped. Returned rather than a bare bool so a run can
/// explain itself — a filter that silently eats everything is worse than none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rejection {
    Expired,
    TooSenior,
    CredentialGate,
    OffTopic,
}

impl Rejection {
    pub fn as_str(&self) -> &'static str {
        match self {
            Rejection::Expired => "deadline passed",
            Rejection::TooSenior => "too senior",
            Rejection::CredentialGate => "degree or certificate gate",
            Rejection::OffTopic => "not software work",
        }
    }
}

/// Decides whether an opening is worth showing him.
///
/// Only openings are filtered. News is never dropped — a story being about
/// senior work does not make it less worth knowing.
pub fn reject_reason(item: &RawItem, now: DateTime<Utc>) -> Option<Rejection> {
    if !crate::sources::opps::is_opening(item) {
        return None;
    }

    if let Some(deadline) = item.deadline_at {
        if deadline < now {
            return Some(Rejection::Expired);
        }
    }

    let text = haystack(item);

    // Title-anchored so that a body mentioning "our senior engineers" does not
    // disqualify an otherwise junior role.
    let title = item.title.to_lowercase();
    if SENIOR_MARKERS.iter().any(|m| title.contains(m)) {
        return Some(Rejection::TooSenior);
    }
    // Year requirements are usually in the body, and they are unambiguous.
    if SENIOR_MARKERS
        .iter()
        .filter(|m| m.contains("years") || m.contains("phd") || m.contains("ph.d"))
        .any(|m| text.contains(m))
    {
        return Some(Rejection::TooSenior);
    }
    if CREDENTIAL_GATES.iter().any(|g| text.contains(g)) {
        return Some(Rejection::CredentialGate);
    }
    if OFF_TOPIC.iter().any(|o| text.contains(o)) {
        return Some(Rejection::OffTopic);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::ReachSignals;
    use chrono::Duration;

    fn opening(kind: &str, title: &str, summary: &str) -> RawItem {
        RawItem {
            kind: kind.into(),
            title: title.into(),
            org: None,
            url: "https://example.com".into(),
            summary: Some(summary.into()),
            published_at: None,
            deadline_at: None,
            location: None,
            is_online: None,
            source: "test".into(),
            external_id: "1".into(),
            signals: ReachSignals::default(),
        }
    }

    #[test]
    fn senior_roles_are_dropped() {
        let item = opening("job", "Senior Backend Engineer", "Remote");
        assert_eq!(reject_reason(&item, Utc::now()), Some(Rejection::TooSenior));
    }

    #[test]
    fn year_requirements_in_the_body_are_caught() {
        let item = opening("job", "Backend Engineer", "We need 5+ years of Go.");
        assert_eq!(reject_reason(&item, Utc::now()), Some(Rejection::TooSenior));
    }

    #[test]
    fn degree_gates_are_dropped() {
        let item = opening("job", "Junior Developer", "A bachelor's degree required.");
        assert_eq!(
            reject_reason(&item, Utc::now()),
            Some(Rejection::CredentialGate)
        );
    }

    #[test]
    fn non_software_roles_are_dropped() {
        let item = opening("job", "Retail Store Associate", "Great team!");
        assert_eq!(reject_reason(&item, Utc::now()), Some(Rejection::OffTopic));
    }

    #[test]
    fn expired_openings_are_dropped() {
        let mut item = opening("hackathon", "Some hackathon", "Online");
        item.deadline_at = Some(Utc::now() - Duration::days(2));
        assert_eq!(reject_reason(&item, Utc::now()), Some(Rejection::Expired));
    }

    #[test]
    fn a_suitable_junior_remote_role_survives() {
        let item = opening(
            "job",
            "Backend Engineer (Junior)",
            "Remote, no degree needed, we care about what you have built.",
        );
        assert_eq!(reject_reason(&item, Utc::now()), None);
    }

    #[test]
    fn news_is_never_filtered_however_senior_it_sounds() {
        // The filter exists to protect his time applying, not to censor news.
        let item = opening("news", "Senior engineers are quitting over AI mandates", "");
        assert_eq!(reject_reason(&item, Utc::now()), None);
    }

    #[test]
    fn a_body_mentioning_senior_staff_does_not_disqualify_a_junior_role() {
        let item = opening(
            "job",
            "Junior Developer",
            "You will pair with our senior engineers regularly.",
        );
        assert_eq!(reject_reason(&item, Utc::now()), None);
    }
}
