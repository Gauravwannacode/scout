//! Checks the news adapters and the full pipeline against live data.
//!
//! `cargo run --bin probe`            — sources only
//! `cargo run --bin probe -- pipeline` — fetch, cluster, score, rank
//!
//! Any source reporting 0 items is a failure, not an empty day: all of these
//! were confirmed to return data when the source list was chosen, so a zero
//! means the adapter broke or the endpoint changed shape.

use app_lib::{pipeline, sources};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let full = std::env::args().any(|a| a == "pipeline");
    if full {
        run_pipeline().await;
    } else {
        run_sources().await;
    }
}

async fn run_sources() {
    println!("Sweeping every news source…\n");
    let started = std::time::Instant::now();
    let report = sources::fetch_all().await;

    let mut names: Vec<&String> = report.counts.keys().collect();
    names.sort();

    let mut zero = Vec::new();
    for name in names {
        let count = report.counts[name];
        match report.errors.get(name) {
            Some(err) => {
                println!("  {name:<16} FAIL  {err}");
                zero.push(name.clone());
            }
            None if count == 0 => {
                println!("  {name:<16} ZERO  parsed nothing (adapter or endpoint changed?)");
                zero.push(name.clone());
            }
            None => println!("  {name:<16} ok    {count} items"),
        }
    }

    println!(
        "\n{} items in {:.1}s",
        report.items.len(),
        started.elapsed().as_secs_f32()
    );

    if report.offline {
        println!("\nEvery source failed to connect — this machine is offline.");
        return;
    }

    if !zero.is_empty() {
        println!("\n{} source(s) returned nothing: {}", zero.len(), zero.join(", "));
        std::process::exit(1);
    }
    println!("\nAll sources returned data.");
}

async fn run_pipeline() {
    println!("Running the full pipeline…\n");
    let started = std::time::Instant::now();
    let result = pipeline::run().await;

    if result.offline {
        println!("Offline — nothing fetched. The app keeps working; news just pauses.");
        return;
    }

    println!(
        "{} raw items clustered into {} distinct stories ({:.1}s)",
        result.raw_count,
        result.story_count,
        started.elapsed().as_secs_f32()
    );

    if result.model_scored < result.story_count {
        println!(
            "  {} of {} scored by the model; {} fell back to the heuristic",
            result.model_scored,
            result.story_count,
            result.story_count - result.model_scored
        );
    }
    if let Some(err) = &result.score_error {
        println!("  scoring reported: {err}");
    }

    if result.provisional_scores {
        println!(
            "\n  NOTE: scores are provisional (heuristic, not the model){}",
            result
                .score_error
                .as_ref()
                .map(|e| format!(" — {e}"))
                .unwrap_or_default()
        );
    }

    if !result.filtered.is_empty() {
        let mut reasons: Vec<_> = result.filtered.iter().collect();
        reasons.sort_by(|a, b| b.1.cmp(a.1));
        let summary: Vec<String> = reasons.iter().map(|(r, n)| format!("{n} {r}")).collect();
        println!("
Openings filtered out: {}", summary.join(" · "));
    }

    let openings: Vec<_> = result
        .items
        .iter()
        .filter(|i| app_lib::sources::opps::OPENING_KINDS.contains(&i.kind.as_str()))
        .collect();
    let dated = openings.iter().filter(|o| o.deadline_at.is_some()).count();
    println!(
        "
{} openings survived the filter ({dated} with a real deadline)",
        openings.len()
    );
    for o in openings.iter().take(8) {
        println!("  [{:<10}] {:<3} {}", o.source, o.significance, truncate(&o.title, 62));
        if let Some(w) = &o.why_line {
            println!("       └─ {}", truncate(w, 88));
        }
    }

    let legendary = result.items.iter().filter(|i| i.badge == "legendary").count();
    let worth = result.items.iter().filter(|i| i.badge == "worth-knowing").count();
    println!("\nBadges: {legendary} legendary · {worth} worth knowing · {} radar",
        result.items.len() - legendary - worth);

    println!("\nTop 12 by significance:\n");
    println!("  {:<4} {:<5} {:<4} {:<14} {}", "SIG", "REACH", "SRC", "BADGE", "TITLE");
    for item in result.items.iter().take(12) {
        println!(
            "  {:<4} {:<5} {:<4} {:<14} {}",
            item.significance,
            item.reach,
            item.corroborations,
            item.badge,
            truncate(&item.title, 68)
        );
        if let Some(why) = &item.why_line {
            println!("       └─ {}", truncate(why, 90));
        }
    }

    // The clustering payoff: stories several outlets ran, folded into one.
    let mut merged: Vec<_> = result.items.iter().filter(|i| i.corroborations > 1).collect();
    merged.sort_by(|a, b| b.corroborations.cmp(&a.corroborations));
    if merged.is_empty() {
        println!("\nNo cross-source duplicates found in this batch.");
    } else {
        println!("\nMerged stories (the reach measurement, obtained free):");
        for item in merged.iter().take(6) {
            println!(
                "  {} sources · reach {:<3} {}",
                item.corroborations,
                item.reach,
                truncate(&item.title, 66)
            );
        }
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_string();
    }
    s.chars().take(n).collect::<String>() + "…"
}
