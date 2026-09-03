//! The programmes worth planning a year around.
//!
//! GSoC, Smart India Hackathon and their peers do not reliably appear in a
//! daily news sweep. They are announced once, discussed for a week, and then
//! go quiet until the window opens — which is exactly when it is too late to
//! start preparing. A feed-driven app will therefore always miss them.
//!
//! So they are curated. Every entry has a permanent official URL (all verified
//! live) and the months its application window normally falls in. Exact dates
//! shift every year, which is why the UI says "typically" and always links to
//! the official page rather than pretending to know this year's calendar.

use crate::sources::{RawItem, ReachSignals, SourceResult};
use chrono::{Datelike, Utc};

pub struct Flagship {
    pub name: &'static str,
    pub org: &'static str,
    pub url: &'static str,
    /// Months the application window normally falls in, 1-12. Several entries
    /// run more than one round a year.
    pub windows: &'static [u32],
    pub why: &'static str,
}

/// Curated deliberately small. These are the ones actually worth a second-year
/// student's time; a longer list would dilute the signal into another feed.
pub const FLAGSHIPS: &[Flagship] = &[
    Flagship {
        name: "Google Summer of Code",
        org: "Google",
        url: "https://summerofcode.withgoogle.com/",
        // Orgs announced in February, contributor proposals late March.
        windows: &[2, 3, 4],
        why: "Paid, remote, open to students with no degree. Start reading org codebases in January — proposals are won before the window opens.",
    },
    Flagship {
        name: "Smart India Hackathon",
        org: "Government of India",
        url: "https://sih.gov.in/",
        windows: &[8, 9, 10],
        why: "India's largest hackathon, and your college nominates teams — find out internally who is organising before registration closes.",
    },
    Flagship {
        name: "Outreachy",
        org: "Software Freedom Conservancy",
        url: "https://www.outreachy.org/",
        // Two rounds a year.
        windows: &[1, 2, 8, 9],
        why: "Paid remote internships in open source. The contribution period during the application itself is what decides selection.",
    },
    Flagship {
        name: "LFX Mentorship",
        org: "Linux Foundation",
        url: "https://mentorship.lfx.linuxfoundation.org/",
        // Three terms a year.
        windows: &[1, 2, 5, 6, 9, 10],
        why: "Three terms a year, so the next window is never far off. Paid, remote, and CNCF projects take a lot of mentees.",
    },
    Flagship {
        name: "MLH Fellowship",
        org: "Major League Hacking",
        url: "https://fellowship.mlh.io/",
        windows: &[1, 2, 5, 6, 9, 10],
        why: "Twelve weeks building open source with a stipend. No degree requirement, and it runs several batches a year.",
    },
    Flagship {
        name: "Hacktoberfest",
        org: "DigitalOcean",
        url: "https://hacktoberfest.com/",
        windows: &[9, 10],
        why: "The easiest first open-source contribution of the year. Low stakes, but four merged PRs is a real portfolio line.",
    },
    Flagship {
        name: "Google Season of Docs",
        org: "Google",
        url: "https://developers.google.com/season-of-docs",
        windows: &[2, 3, 4],
        why: "Less competitive than GSoC and paid. Worth knowing about if writing comes easier to you than algorithms.",
    },
];

/// How near this programme's next window is.
#[derive(Debug, PartialEq, Eq)]
pub enum Window {
    OpenNow,
    /// Months until the window opens.
    In(u32),
}

pub fn window_status(windows: &[u32], month: u32) -> Window {
    if windows.contains(&month) {
        return Window::OpenNow;
    }
    // Smallest forward distance, wrapping through December.
    let nearest = windows
        .iter()
        .map(|&m| (m + 12 - month) % 12)
        .min()
        .unwrap_or(0);
    Window::In(nearest)
}

fn describe(windows: &[u32], month: u32) -> String {
    match window_status(windows, month) {
        Window::OpenNow => "Window is usually open around now — check the site".to_string(),
        Window::In(1) => "Opens in about a month".to_string(),
        Window::In(n) => format!("Opens in about {n} months"),
    }
}

/// These are not fetched — they are known. The adapter shape is kept so they
/// flow through clustering, scoring and the UI like anything else.
pub async fn flagships() -> SourceResult {
    let month = Utc::now().month();

    let items = FLAGSHIPS
        .iter()
        .map(|f| RawItem {
            kind: "grant".into(),
            title: format!("{} — {}", f.name, f.org),
            org: Some(f.org.to_string()),
            url: f.url.to_string(),
            summary: Some(format!("{} {}", describe(f.windows, month), f.why)),
            published_at: None,
            // Deliberately absent. Real dates move every year, and a wrong
            // deadline is worse than none — it would fire a reminder for a
            // date that does not exist.
            deadline_at: None,
            location: Some("Remote".into()),
            is_online: Some(true),
            source: "flagship".into(),
            external_id: f.name.to_lowercase().replace(' ', "-"),
            signals: ReachSignals {
                points: None,
                comments: None,
                primary: true,
            },
        })
        .collect();

    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_month_inside_the_window_reads_as_open() {
        assert_eq!(window_status(&[3, 4], 3), Window::OpenNow);
        assert_eq!(window_status(&[3, 4], 4), Window::OpenNow);
    }

    #[test]
    fn distance_is_counted_forward_to_the_next_window() {
        assert_eq!(window_status(&[3, 4], 1), Window::In(2));
        assert_eq!(window_status(&[9, 10], 7), Window::In(2));
    }

    #[test]
    fn the_year_wraps_rather_than_going_negative() {
        // December looking at a February window is two months away, not ten.
        assert_eq!(window_status(&[2, 3], 12), Window::In(2));
        assert_eq!(window_status(&[1], 11), Window::In(2));
    }

    #[test]
    fn a_programme_with_several_rounds_finds_the_nearest() {
        // Outreachy runs in Jan-Feb and Aug-Sep; in June the August round is
        // nearer than next January.
        assert_eq!(window_status(&[1, 2, 8, 9], 6), Window::In(2));
    }

    #[test]
    fn every_flagship_has_a_real_https_url_and_a_reason() {
        for f in FLAGSHIPS {
            assert!(f.url.starts_with("https://"), "{} has a bad url", f.name);
            assert!(!f.why.is_empty(), "{} has no reason given", f.name);
            assert!(!f.windows.is_empty(), "{} has no window", f.name);
            assert!(
                f.windows.iter().all(|m| (1..=12).contains(m)),
                "{} has a month outside 1-12",
                f.name
            );
        }
    }

    #[tokio::test]
    async fn the_set_is_produced_without_a_network_call() {
        let items = flagships().await.expect("flagships never fail");
        assert_eq!(items.len(), FLAGSHIPS.len());
        // No deadline: a wrong one would fire a reminder for a date that does
        // not exist.
        assert!(items.iter().all(|i| i.deadline_at.is_none()));
    }
}
