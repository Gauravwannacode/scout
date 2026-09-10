//! Renders both templates from the real fact base, so the design can be
//! judged before the feature is wired into the UI.
//!
//! `cargo run --bin resumecheck`

use app_lib::resume::{facts, render};

fn main() {
    let all = facts::seed();
    let header = facts::seed_header();

    // A plausible tailoring for an infrastructure startup: systems work
    // first, the web and rendering projects left out entirely.
    let infra = [
        "proj-jos",
        "proj-market",
        "proj-bharatgpt",
        "proj-scout",
        "proj-pipeline",
        "proj-factorio",
        "sk-rust",
        "sk-c",
        "sk-cpp",
        "sk-python",
        "sk-ts",
        "sk-sqlite",
        "sk-linux",
        "sk-git",
        "edu-pes",
    ];

    // And for an AI lab: the model work leads, systems supports it.
    let ai = [
        "proj-jos",
        "proj-market",
        "proj-bharatgpt",
        "proj-factorio",
        "proj-quantum",
        "proj-pipeline",
        "sk-python",
        "sk-pytorch",
        "sk-ml",
        "sk-llm",
        "sk-prompt",
        "sk-qiskit",
        "sk-rust",
        "edu-pes",
        "edu-pesuio",
    ];

    for (name, ids, template, headline) in [
        (
            "clean",
            &infra[..],
            "clean",
            "Second-year CS student · systems work in Rust, C and Python",
        ),
        (
            "bold",
            &ai[..],
            "bold",
            "Second-year CS student · trains models and builds the systems around them",
        ),
    ] {
        let chosen: Vec<&facts::Fact> = ids
            .iter()
            .filter_map(|id| all.iter().find(|f| f.id == *id))
            .collect();

        let html = render::to_html(&header, &chosen, headline, template);
        match render::to_pdf(&html) {
            Ok(bytes) => {
                let out = std::env::temp_dir().join(format!("scout-resume-{name}.pdf"));
                std::fs::write(&out, &bytes).expect("could not write");
                println!(
                    "{:<6} {:>3} facts · {:>6} bytes · {}",
                    name,
                    chosen.len(),
                    bytes.len(),
                    out.display()
                );
            }
            Err(e) => println!("{name}: FAILED — {e}"),
        }
    }
}
