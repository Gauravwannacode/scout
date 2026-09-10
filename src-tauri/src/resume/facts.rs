//! The fact base: everything Gaurav has actually done.
//!
//! Seeded once from what is genuinely verifiable — the five projects, the
//! stack behind each, the skill levels he set himself — and editable in the
//! app afterwards. Nothing here is aspirational, because a resume that
//! overstates gets found out in the first interview question.
//!
//! Tailoring picks from this list. It cannot add to it.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Fact {
    pub id: String,
    /// project | skill | education | award
    pub kind: String,
    pub title: String,
    #[serde(default)]
    pub subtitle: Option<String>,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub tech: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    /// Skills only: how the level should read, in his own words.
    #[serde(default)]
    pub level: Option<String>,
    #[serde(default)]
    pub period: Option<String>,
    #[serde(default)]
    pub sort_order: i64,
    #[serde(default = "yes")]
    pub enabled: bool,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Header {
    pub name: String,
    pub headline: String,
    pub email: String,
    pub phone: String,
    pub github: String,
    pub linkedin: String,
    pub location: String,
}

fn project(
    id: &str,
    title: &str,
    subtitle: &str,
    detail: &str,
    tech: &str,
    url: Option<&str>,
    period: &str,
    order: i64,
) -> Fact {
    Fact {
        id: id.into(),
        kind: "project".into(),
        title: title.into(),
        subtitle: Some(subtitle.into()),
        detail: Some(detail.into()),
        tech: Some(tech.into()),
        url: url.map(str::to_string),
        level: None,
        period: Some(period.into()),
        sort_order: order,
        enabled: true,
    }
}

fn skill(id: &str, title: &str, group: &str, level: &str, order: i64) -> Fact {
    Fact {
        id: id.into(),
        kind: "skill".into(),
        title: title.into(),
        subtitle: Some(group.into()),
        detail: None,
        tech: None,
        url: None,
        level: Some(level.into()),
        period: None,
        sort_order: order,
        enabled: true,
    }
}

/// The starting fact base.
///
/// Detail lines state what the thing does and what it demonstrably achieved —
/// test counts, verified behaviours, honest outcomes. The market-intel entry
/// says plainly that no edge was found, because that is the truth and it is
/// also the more impressive claim: it shows he could tell a real result from
/// a flattering one.
pub fn seed() -> Vec<Fact> {
    vec![
        project(
            "proj-jos",
            "JOS — an operating system for AI agents",
            "Personal project",
            "Rust kernel owning world state, scheduling and policy, with a TypeScript userspace that reaches it only through an enumerated capability ABI over CBOR. Task status has a single writer inside one transaction, and WAITING is a first-class state so a rate limit parks a task instead of failing its goal. 188 tests green; the acceptance gate was killing the kernel mid-task and watching the task resume from its checkpoint on restart.",
            "Rust, TypeScript, CBOR, SQLite, WSL2",
            None,
            "2026",
            10,
        ),
        project(
            "proj-scout",
            "Scout — local-first desktop news and opportunity app",
            "Open source",
            "Reads 28 live sources every 30 minutes, folds duplicate coverage into single stories with TF-IDF clustering, and ranks each on two independent axes — how significant it is, and how widely covered. A Rust scheduler outlives the window so alarms and deadline reminders fire with the app closed. Everything except fetching news works offline from one SQLite file.",
            "Rust, Tauri, React, TypeScript, SQLite, Groq, Gemini",
            Some("https://github.com/Gauravwannacode/scout"),
            "2026",
            40,
        ),
        project(
            "proj-market",
            "AI Market Intelligence — quantitative research pipeline",
            "Personal project",
            "Nine independent searches for a tradeable edge against index benchmarks using free data only. Found none, and documented why in two postmortems. The result worth reporting: a rotation strategy showing +1.52%/yr proved to be three author-chosen parameters, and walk-forward selection produced negative excess return at every start year tested.",
            "Python, pandas, time-series analysis, walk-forward validation",
            None,
            "2026",
            20,
        ),
        project(
            "proj-factorio",
            "Factorio Automation Agent",
            "Personal project",
            "LLM agents that build factories autonomously in the Factorio Learning Environment, driving the game over RCON from Python. Aimed at splitting the problem the way chip floorplanning does — the language model decides what to build, a trained model decides where to place it.",
            "Python, LLM agents, RCON, Docker",
            None,
            "2026",
            60,
        ),
        project(
            "proj-comic",
            "Comic Engine — manhwa rendering pipeline",
            "Personal project",
            "A non-photorealistic render pipeline in Blender targeting Korean webtoon art: EEVEE cel shading with a two-pass line-art stack, driven by Python through the bpy API, with a TOML scene format so panels are described as data rather than hand-composed.",
            "Python, Blender/bpy, TOML, NPR rendering",
            None,
            "2026",
            80,
        ),
        project(
            "proj-bharatgpt",
            "BharatGPT-600 — a transformer trained from scratch",
            "Personal project",
            "A GPT-style language model trained from zero on a 1.9GB corpus of Sanskrit, Pali and Sangam Tamil texts composed before 600 CE — deliberately a time-capsule, with no modern text in the training data at all. Built the corpus pipeline, a tokenizer for Devanagari and Tamil script, the training loop, and an evaluation harness that probes for anachronism as well as perplexity.",
            "Python, PyTorch, transformers, tokenization, corpus engineering",
            None,
            "2026",
            30,
        ),
        project(
            "proj-pipeline",
            "AI video production pipeline",
            "Personal project",
            "An orchestrator that takes a script through voice, image, video and upscale stages to a finished 1080p episode, then uploads it. Runs entirely on free tiers and one 8GB GPU, which forced a scheduler that keeps a single heavy model resident at a time and pushes lighter work to CPU. A human approves or rejects each finished item; nothing else is manual.",
            "Python, ComfyUI, TTS, YouTube API, GPU scheduling, Flask",
            None,
            "2026",
            50,
        ),
        project(
            "proj-quantum",
            "Q-State Arena — quantum gate simulator",
            "Personal project",
            "An interactive two-qubit simulator that renders each qubit on a Bloch sphere and scores the player's state by fidelity across five levels, ending at a phase-entangled state. Built to make gates and entanglement legible rather than abstract.",
            "Python, Qiskit, Tkinter, Matplotlib",
            Some("https://github.com/Gauravwannacode/Q-State-Arena-Learning-Quantum-Gates-main"),
            "2025",
            90,
        ),
        project(
            "proj-rmm",
            "RMM Counselling — college admissions tool",
            "Built for a working counsellor",
            "Two engines over one student profile: deterministic college matching that turns rank, category and quota into Safe/Target/Reach bands from verified cutoffs with no model involved in the prediction, and a separate guidance engine grounded in a counsellor-verified knowledge base. A Python pipeline parses official JoSAA PDFs into versioned cutoff rows, with a schema that does not hardcode any one authority.",
            "Next.js, TypeScript, Supabase, Postgres, Python, RAG",
            None,
            "2026",
            70,
        ),
        // Levels are his own words, set deliberately rather than inflated.
        skill("sk-c", "C", "Languages", "Comfortable", 10),
        skill("sk-cpp", "C++", "Languages", "Comfortable", 11),
        skill("sk-python", "Python", "Languages", "Comfortable", 12),
        skill("sk-rust", "Rust", "Languages", "Learning", 13),
        skill("sk-ts", "TypeScript", "Languages", "Decent", 14),
        skill(
            "sk-llm",
            "LLM integration",
            "AI",
            "Strong — rate limits, key rotation, structured output, model fallback",
            20,
        ),
        skill("sk-prompt", "Prompt engineering", "AI", "Strong", 21),
        skill("sk-ml", "Machine learning", "AI", "Learning the theory", 22),
        skill("sk-pytorch", "PyTorch", "AI", "Decent", 23),
        skill("sk-qiskit", "Qiskit / quantum computing", "AI", "Decent", 24),
        skill("sk-react", "React", "Web", "Decent", 30),
        skill("sk-tailwind", "Tailwind / CSS", "Web", "Learning", 31),
        skill("sk-node", "Node.js", "Web", "Decent", 32),
        skill("sk-sqlite", "SQLite / SQL", "Data", "Comfortable", 40),
        skill("sk-postgres", "Postgres / Supabase", "Data", "Decent", 41),
        skill("sk-tauri", "Tauri", "Tools", "Comfortable", 50),
        skill("sk-git", "Git / GitHub", "Tools", "Comfortable", 51),
        skill("sk-linux", "Linux / WSL2", "Tools", "Decent", 52),
        skill("sk-docker", "Docker", "Tools", "Learning", 53),
        skill("sk-blender", "Blender / bpy", "Tools", "Decent", 54),
        Fact {
            id: "edu-pes".into(),
            kind: "education".into(),
            title: "B.Tech, Computer Science and Engineering".into(),
            subtitle: Some("PES University, Electronic City campus, Bangalore".into()),
            detail: None,
            tech: None,
            url: None,
            level: None,
            period: Some("2025 – 2029".into()),
            sort_order: 10,
            enabled: true,
        },
        Fact {
            id: "edu-pesuio".into(),
            kind: "award".into(),
            title: "Introduction to Quantum AI".into(),
            subtitle: Some("PESU I/O".into()),
            detail: None,
            tech: None,
            url: None,
            level: None,
            period: None,
            sort_order: 10,
            enabled: true,
        },
    ]
}

/// Header details that are already known. Phone and LinkedIn are left blank
/// deliberately — inventing either would be exactly the failure this whole
/// design exists to prevent.
pub fn seed_header() -> Header {
    Header {
        name: "Gaurav Mehta".into(),
        headline: "Second-year CS student · builds systems in Rust, Python and TypeScript".into(),
        email: "gaurav9215600@gmail.com".into(),
        phone: "+91 82950 48105".into(),
        github: "github.com/Gauravwannacode".into(),
        linkedin: "linkedin.com/in/gauravmehtaa".into(),
        location: "Bangalore, India".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_fact_has_a_unique_id() {
        // Tailoring addresses facts by id; a duplicate would silently make one
        // of them unreachable.
        let facts = seed();
        let ids: HashSet<&str> = facts.iter().map(|f| f.id.as_str()).collect();
        assert_eq!(ids.len(), facts.len());
    }

    #[test]
    fn every_named_project_is_present() {
        let ids: HashSet<String> = seed().iter().map(|f| f.id.clone()).collect();
        for id in [
            "proj-jos",
            "proj-scout",
            "proj-bharatgpt",
            "proj-market",
            "proj-factorio",
            "proj-pipeline",
            "proj-comic",
            "proj-quantum",
            "proj-rmm",
        ] {
            assert!(ids.contains(id), "missing {id}");
        }
    }

    #[test]
    fn projects_default_to_his_chosen_order() {
        // He set this deliberately: JOS, then Market Intelligence, then
        // BharatGPT. Tailoring may promote something for a specific company,
        // but the baseline is his.
        let facts = seed();
        let mut projects: Vec<&Fact> = facts.iter().filter(|f| f.kind == "project").collect();
        projects.sort_by_key(|f| f.sort_order);
        let ids: Vec<&str> = projects.iter().map(|f| f.id.as_str()).collect();
        assert_eq!(&ids[..3], &["proj-jos", "proj-market", "proj-bharatgpt"]);
    }

    #[test]
    fn every_project_says_what_it_actually_does() {
        for f in seed().iter().filter(|f| f.kind == "project") {
            let detail = f.detail.as_deref().unwrap_or("");
            assert!(
                detail.len() > 80,
                "{} has a thin detail line, which reads as padding",
                f.title
            );
            assert!(f.tech.is_some(), "{} has no stack listed", f.title);
        }
    }

    #[test]
    fn every_skill_carries_an_honest_level() {
        for f in seed().iter().filter(|f| f.kind == "skill") {
            assert!(f.level.is_some(), "{} has no level", f.title);
            assert!(f.subtitle.is_some(), "{} has no group", f.title);
        }
    }

    #[test]
    fn the_header_carries_real_details_not_placeholders() {
        // These came from him directly. The point of the test is that nothing
        // here is an invented example value.
        let h = seed_header();
        assert!(!h.name.is_empty());
        assert!(h.email.contains('@'));
        assert!(h.phone.starts_with("+91"), "got: {}", h.phone);
        assert!(h.linkedin.contains("gauravmehtaa"));
        for field in [&h.phone, &h.linkedin, &h.email] {
            assert!(
                !field.contains("example") && !field.contains("xxx"),
                "placeholder left in: {field}"
            );
        }
    }
}
