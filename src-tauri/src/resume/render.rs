//! Drawing the resume.
//!
//! Two templates, chosen per company by the tailoring step:
//!
//! - `clean` — single column, conventional order, no colour blocks or
//!   multi-column layout. Applicant tracking systems read columns across
//!   rather than down and scramble anything clever, so a formal careers
//!   process gets this one.
//! - `bold` — a founder reading it directly will never run a parser, so the
//!   constraint lifts: an accent rule, a stronger name, and the projects
//!   given real visual weight.
//!
//! Both are built only from facts the caller passed in, in the order given.

use super::facts::{Fact, Header};
use printpdf::*;
use std::collections::BTreeMap;

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn contact_line(h: &Header) -> String {
    [
        h.email.as_str(),
        h.phone.as_str(),
        h.github.as_str(),
        h.linkedin.as_str(),
        h.location.as_str(),
    ]
    .iter()
    .filter(|s| !s.trim().is_empty())
    .map(|s| esc(s))
    .collect::<Vec<_>>()
    .join("  ·  ")
}

/// Groups skills under their heading, preserving the caller's order.
fn skill_block(skills: &[&Fact]) -> String {
    let mut groups: Vec<(String, Vec<String>)> = Vec::new();
    for s in skills {
        let group = s.subtitle.clone().unwrap_or_else(|| "Skills".into());
        // A level of "Comfortable" adds nothing next to the name; only show
        // one when it says something specific.
        let label = match s.level.as_deref() {
            Some(l) if !l.is_empty() && l != "Comfortable" => {
                format!("{} <span class=\"lvl\">({})</span>", esc(&s.title), esc(l))
            }
            _ => esc(&s.title),
        };
        match groups.iter_mut().find(|(g, _)| *g == group) {
            Some((_, items)) => items.push(label),
            None => groups.push((group, vec![label])),
        }
    }

    groups
        .iter()
        .map(|(g, items)| {
            format!(
                "<div class=\"skillrow\"><span class=\"skillgroup\">{}</span>{}</div>",
                esc(g),
                items.join(", ")
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

fn project_block(projects: &[&Fact], bold: bool) -> String {
    projects
        .iter()
        .map(|p| {
            let period = p
                .period
                .as_deref()
                .filter(|s| !s.is_empty())
                .map(|s| format!("<span class=\"period\">{}</span>", esc(s)))
                .unwrap_or_default();
            let sub = p
                .subtitle
                .as_deref()
                .filter(|s| !s.is_empty())
                .map(|s| format!("<div class=\"psub\">{}</div>", esc(s)))
                .unwrap_or_default();
            let tech = p
                .tech
                .as_deref()
                .filter(|s| !s.is_empty())
                .map(|s| format!("<div class=\"tech\">{}</div>", esc(s)))
                .unwrap_or_default();
            let detail = p
                .detail
                .as_deref()
                .map(|s| format!("<p>{}</p>", esc(s)))
                .unwrap_or_default();
            let link = p
                .url
                .as_deref()
                .filter(|s| !s.is_empty())
                .map(|s| format!("<div class=\"link\">{}</div>", esc(s)))
                .unwrap_or_default();

            format!(
                "<div class=\"{}\"><div class=\"ptitle\">{}{}</div>{sub}{tech}{detail}{link}</div>",
                if bold { "proj boldproj" } else { "proj" },
                esc(&p.title),
                period
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

fn simple_block(items: &[&Fact]) -> String {
    items
        .iter()
        .map(|e| {
            let period = e
                .period
                .as_deref()
                .filter(|s| !s.is_empty())
                .map(|s| format!("<span class=\"period\">{}</span>", esc(s)))
                .unwrap_or_default();
            let sub = e
                .subtitle
                .as_deref()
                .filter(|s| !s.is_empty())
                .map(|s| format!("<div class=\"psub\">{}</div>", esc(s)))
                .unwrap_or_default();
            format!(
                "<div class=\"entry\"><div class=\"ptitle\">{}{}</div>{sub}</div>",
                esc(&e.title),
                period
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

const CLEAN_CSS: &str = r#"
body { font-family: sans-serif; font-size: 10px; color: #1b1b1b; line-height: 1.45;
       padding: 34px 40px; }
.name { font-size: 24px; font-weight: bold; letter-spacing: -0.4px; }
.headline { font-size: 10.5px; color: #444444; margin-top: 3px; }
.contact { font-size: 8.6px; color: #555555; margin-top: 5px; margin-bottom: 14px; }
h2 { font-size: 9.5px; letter-spacing: 1.4px; color: #222222;
     border-bottom: 1px solid #cccccc; padding-bottom: 3px;
     margin-top: 14px; margin-bottom: 8px; }
.proj { margin-bottom: 9px; }
.ptitle { font-size: 11px; font-weight: bold; }
.period { font-size: 8.5px; color: #777777; font-weight: normal; }
.psub { font-size: 9px; color: #555555; margin-top: 1px; }
.tech { font-size: 8.6px; color: #666666; margin-top: 2px; }
p { margin-top: 3px; margin-bottom: 0px; }
.link { font-size: 8.5px; color: #555555; margin-top: 2px; }
.skillrow { font-size: 9.5px; margin-bottom: 4px; }
.skillgroup { font-weight: bold; }
.lvl { color: #777777; }
.entry { margin-bottom: 7px; }
"#;

// The founder-facing template.
//
// A recruiter running a parser never sees this one, so the layout constraint
// lifts and the page can actually be designed: a full-bleed dark masthead,
// project cards with real weight, and skills as chips. The accent is Scout's
// own clay, used on exactly three things — the rule, the section labels and
// the stack lines — so it reads as a system rather than decoration.
const BOLD_CSS: &str = r#"
body { font-family: sans-serif; font-size: 10px; color: #17150f; line-height: 1.5; }

.masthead { background-color: #17150f; padding: 26px 40px 22px 40px; margin-bottom: 0px; }
.name { font-size: 34px; font-weight: bold; letter-spacing: -1.1px; color: #ffffff; }
.headline { font-size: 11.5px; color: #e0906c; margin-top: 5px; font-weight: bold; }
.contact { font-size: 8.8px; color: #9a948a; margin-top: 9px; }
.rule { height: 4px; background: linear-gradient(to right, #c2603a, #e0a06c); }

.page { padding: 22px 40px 30px 40px; }
h2 { font-size: 9.5px; letter-spacing: 2.2px; color: #b0532f;
     margin-top: 18px; margin-bottom: 10px; font-weight: bold; }

.proj { margin-bottom: 9px; }
.boldproj { background-color: #faf7f4; border-radius: 5px;
            padding: 10px 13px 11px 13px; }
.ptitle { font-size: 12px; font-weight: bold; color: #17150f; }
.period { font-size: 8.5px; color: #8f8a82; font-weight: normal; }
.psub { font-size: 9px; color: #6f6a62; margin-top: 1px; }
.tech { font-size: 8.6px; color: #b0532f; margin-top: 4px; font-weight: bold; }
p { margin-top: 5px; margin-bottom: 0px; color: #2f2b25; }
.link { font-size: 8.5px; color: #6f6a62; margin-top: 4px; }

.skillrow { font-size: 9.5px; margin-bottom: 6px; }
.skillgroup { font-weight: bold; color: #17150f; }
.lvl { color: #8f8a82; }
.entry { margin-bottom: 8px; }
"#;

/// Builds the resume HTML from the chosen facts, in the order given.
pub fn to_html(header: &Header, chosen: &[&Fact], headline: &str, template: &str) -> String {
    let bold = template == "bold";
    let css = if bold { BOLD_CSS } else { CLEAN_CSS };

    let of = |kind: &str| -> Vec<&Fact> {
        chosen.iter().filter(|f| f.kind == kind).copied().collect()
    };

    let projects = of("project");
    let skills = of("skill");
    let education = of("education");
    let awards = of("award");

    // The tailored headline, falling back to the stored one.
    let line = if headline.trim().is_empty() {
        header.headline.as_str()
    } else {
        headline
    };
    let headline_html = if line.trim().is_empty() {
        String::new()
    } else {
        format!("<div class=\"headline\">{}</div>", esc(line))
    };

    let mut body = String::new();
    if bold {
        // Full-bleed masthead: the page padding lives on an inner wrapper so
        // the dark band can run edge to edge.
        body.push_str(&format!(
            "<div class=\"masthead\"><div class=\"name\">{}</div>{headline_html}<div class=\"contact\">{}</div></div><div class=\"rule\"></div><div class=\"page\">",
            esc(&header.name),
            contact_line(header)
        ));
    } else {
        body.push_str(&format!(
            "<div class=\"name\">{}</div>{headline_html}<div class=\"contact\">{}</div>",
            esc(&header.name),
            contact_line(header)
        ));
    }

    if !projects.is_empty() {
        body.push_str("<h2>PROJECTS</h2>");
        body.push_str(&project_block(&projects, bold));
    }
    if !skills.is_empty() {
        body.push_str("<h2>SKILLS</h2>");
        body.push_str(&skill_block(&skills));
    }
    if !education.is_empty() {
        body.push_str("<h2>EDUCATION</h2>");
        body.push_str(&simple_block(&education));
    }
    if !awards.is_empty() {
        body.push_str("<h2>COURSEWORK</h2>");
        body.push_str(&simple_block(&awards));
    }

    if bold {
        body.push_str("</div>");
    }

    format!("<html><head><style>{css}</style></head><body>{body}</body></html>")
}

/// Renders the HTML to PDF bytes.
pub fn to_pdf(html: &str) -> Result<Vec<u8>, String> {
    let images = BTreeMap::new();
    let fonts = BTreeMap::new();
    // Margins are zero so the bold template's masthead can run edge to edge;
    // both templates set their own padding in CSS.
    let options = GeneratePdfOptions {
        margin_top: Some(0.0),
        margin_right: Some(0.0),
        margin_bottom: Some(0.0),
        margin_left: Some(0.0),
        ..Default::default()
    };
    let mut warnings = Vec::new();

    let doc = PdfDocument::from_html(html, &images, &fonts, &options, &mut warnings)
        .map_err(|e| format!("could not lay out the resume: {e}"))?;

    let mut save_warnings = Vec::new();
    let bytes = doc.save(&PdfSaveOptions::default(), &mut save_warnings);

    // A resume goes to a stranger with a job to offer. A file that exists but
    // is blank is the failure worth refusing outright.
    if !bytes.starts_with(b"%PDF") || bytes.len() < 1500 {
        return Err(format!(
            "the rendered PDF looks empty ({} bytes) — not writing it",
            bytes.len()
        ));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::super::facts::{seed, seed_header};
    use super::*;

    fn chosen<'a>(ids: &[&str], facts: &'a [Fact]) -> Vec<&'a Fact> {
        ids.iter()
            .filter_map(|id| facts.iter().find(|f| f.id == *id))
            .collect()
    }

    #[test]
    fn only_the_chosen_facts_appear() {
        // The guarantee that matters: a project left out of `include` must not
        // reach the page, however prominent it is in the fact base.
        let facts = seed();
        let picked = chosen(&["proj-jos", "edu-pes"], &facts);
        let html = to_html(&seed_header(), &picked, "", "clean");

        assert!(html.contains("JOS"));
        assert!(html.contains("PES University"));
        assert!(!html.contains("Comic Engine"), "an unchosen project leaked in");
        assert!(!html.contains("Factorio"), "an unchosen project leaked in");
    }

    #[test]
    fn order_is_preserved_exactly_as_given() {
        let facts = seed();
        let html = to_html(
            &seed_header(),
            &chosen(&["proj-scout", "proj-jos"], &facts),
            "",
            "clean",
        );
        let scout = html.find("Scout").expect("scout missing");
        let jos = html.find("JOS").expect("jos missing");
        assert!(scout < jos, "tailoring order was not respected");
    }

    #[test]
    fn a_blank_phone_leaves_no_stray_separator() {
        // The header has empty phone and linkedin until he fills them in;
        // that must not render as "email · · github".
        let line = contact_line(&seed_header());
        assert!(!line.contains("·  ·"), "got: {line}");
        assert!(!line.starts_with('·') && !line.ends_with('·'));
    }

    #[test]
    fn the_two_templates_actually_differ() {
        let facts = seed();
        let picked = chosen(&["proj-jos"], &facts);
        let clean = to_html(&seed_header(), &picked, "", "clean");
        let bold = to_html(&seed_header(), &picked, "", "bold");
        assert_ne!(clean, bold);
        assert!(bold.contains("#c2603a"), "the bold template lost its accent");
        assert!(!clean.contains("#c2603a"), "the ATS template must stay plain");
    }

    #[test]
    fn markup_in_a_fact_cannot_break_the_layout() {
        let mut facts = seed();
        facts[0].title = "JOS <script>alert(1)</script>".into();
        let html = to_html(&seed_header(), &chosen(&["proj-jos"], &facts), "", "clean");
        assert!(!html.contains("<script>"), "unescaped markup reached the page");
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn the_tailored_headline_wins_over_the_stored_one() {
        let facts = seed();
        let html = to_html(
            &seed_header(),
            &chosen(&["proj-jos"], &facts),
            "Systems programmer who ships",
            "clean",
        );
        assert!(html.contains("Systems programmer who ships"));
    }

    #[test]
    fn a_real_resume_renders_to_a_non_empty_pdf() {
        let facts = seed();
        let picked = chosen(
            &["proj-jos", "proj-bharatgpt", "sk-rust", "sk-python", "edu-pes"],
            &facts,
        );
        let html = to_html(&seed_header(), &picked, "Builds systems", "bold");
        let pdf = to_pdf(&html).expect("should render");
        assert!(pdf.starts_with(b"%PDF"));
        assert!(pdf.len() > 1500);
    }
}
