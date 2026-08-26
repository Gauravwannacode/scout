pub mod boards;
pub mod gh_opps;
pub mod hackathons;
pub mod hn_opps;
pub mod yc;

use crate::sources::{Fetcher, RawItem};

/// Kinds that belong on the Openings section rather than the news feed.
pub const OPENING_KINDS: &[&str] = &[
    "job",
    "internship",
    "hackathon",
    "oss",
    "grant",
    "company",
];

pub fn is_opening(item: &RawItem) -> bool {
    OPENING_KINDS.contains(&item.kind.as_str())
}

macro_rules! source {
    ($name:expr, $f:path) => {
        ($name, (|| Box::pin($f()) as _) as Fetcher)
    };
}

/// Opening sources, ordered by how early they catch a role.
///
/// YC and Launch HN come first deliberately: they surface companies that are
/// hiring before any listing exists, which is the one thing a job board can
/// never provide. The boards below them are coverage, not edge.
pub fn registry() -> Vec<(&'static str, Fetcher)> {
    vec![
        source!("yc", yc::companies),
        source!("launch-hn", hn_opps::launch_hn),
        source!("show-hn", hn_opps::show_hn),
        source!("hn-hiring", hn_opps::who_is_hiring),
        source!("gh-issues", gh_opps::good_first_issues),
        source!("devpost", hackathons::devpost),
        source!("devfolio", hackathons::devfolio),
        source!("unstop", hackathons::unstop),
        source!("remoteok", boards::remoteok),
        source!("arbeitnow", boards::arbeitnow),
        source!("himalayas", boards::himalayas),
    ]
}
