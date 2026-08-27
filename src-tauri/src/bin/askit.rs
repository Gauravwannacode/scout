//! Exercises the advisor against a real day, without the UI.
//!
//! `cargo run --bin askit -- "what should I spend this week on?"`
//!
//! Runs the full pipeline first so the context is genuine — an advisor tested
//! on invented input proves nothing about whether it stays grounded.

use app_lib::pipeline::{self, ask};

#[tokio::main]
async fn main() {
    let question = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "What should I spend this week on?".to_string());

    println!("Collecting today first…\n");
    let result = pipeline::run().await;
    if result.offline {
        println!("Offline — nothing to advise on.");
        return;
    }

    let opening_kinds = ["job", "internship", "hackathon", "oss", "grant", "company"];
    let (openings, stories): (Vec<_>, Vec<_>) = result
        .items
        .iter()
        .partition(|i| opening_kinds.contains(&i.kind.as_str()));

    let label = |badge: &str| match badge {
        "legendary" => " [big, and barely covered]",
        "worth-knowing" => " [big, widely covered]",
        _ => "",
    };

    let mut context = String::new();
    if let Some(b) = &result.brief {
        context.push_str(&format!("TODAY'S BRIEF:\n{b}\n\n"));
    }
    context.push_str("NEWS (most significant first):\n");
    for i in stories.iter().take(10) {
        context.push_str(&format!(
            "- {}{} — {}. {}\n",
            i.title,
            label(&i.badge),
            i.source,
            i.why_line.as_deref().unwrap_or("")
        ));
    }
    context.push_str("\nOPENINGS:\n");
    for i in openings.iter().take(10) {
        context.push_str(&format!(
            "- [{}] {} — {}. {}\n",
            i.kind,
            i.title,
            i.source,
            i.why_line.as_deref().unwrap_or("")
        ));
    }
    context.push_str("\nHIS OPEN TASKS:\n- Hackathon build block\n- Message the founder\n");

    println!("Context: {} chars\n", context.len());
    println!("Q: {question}\n");

    match ask::ask(&question, &context, &[], &app_lib::settings::load()).await {
        Ok(answer) => println!("A: {answer}"),
        Err(e) => println!("FAILED: {e}"),
    }
}
