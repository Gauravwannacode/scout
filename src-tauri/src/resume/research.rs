//! Reading a company before writing to them.
//!
//! A cold email that could have been sent to anyone gets deleted. The
//! difference between that and one worth answering is usually a single
//! specific observation, and that has to come from somewhere — so this fetches
//! whatever page the opening points at and pulls the readable text out.
//!
//! This is the one part of the feature that can fail harmlessly. No research
//! means a weaker email, not a broken one, and the caller says so rather than
//! pretending the page was read.

use crate::sources::http::{clean_text, get_text};
use scraper::{Html, Selector};

/// How much page text the model sees. Enough for a landing page's pitch;
/// short enough to leave the token budget for the tailoring itself.
const MAX_CHARS: usize = 3500;

/// Pulls the human-readable pitch out of a page.
///
/// Deliberately crude: headings and paragraphs only, no attempt to understand
/// the site's structure. A landing page says what it does in those tags, and
/// anything cleverer would break on the next redesign.
fn readable(html: &str) -> String {
    let doc = Html::parse_document(html);
    let mut out = String::new();

    for sel in ["h1", "h2", "h3", "p", "li"] {
        let Ok(selector) = Selector::parse(sel) else {
            continue;
        };
        for el in doc.select(&selector) {
            let text = clean_text(&el.text().collect::<String>(), 400);
            // Single words and nav labels are noise.
            if text.split_whitespace().count() < 4 {
                continue;
            }
            out.push_str(&text);
            out.push('\n');
            if out.len() > MAX_CHARS {
                return out;
            }
        }
    }
    out
}

#[derive(Debug, Clone)]
pub struct Research {
    pub text: String,
    /// True when the page could not be read. The caller shows this rather
    /// than letting the model invent a company from its name alone.
    pub failed: bool,
    pub note: String,
}

/// Gathers what can be learned about the company behind an opening.
///
/// `summary` is what Scout already knows from the source that surfaced it,
/// which is often the most useful part — a Launch HN post says more about a
/// company in two lines than its landing page does in ten.
pub async fn research(url: &str, title: &str, summary: Option<&str>) -> Research {
    let mut parts = vec![format!("Opening: {title}")];
    if let Some(s) = summary.filter(|s| !s.trim().is_empty()) {
        parts.push(format!("What Scout knows: {s}"));
    }

    if url.trim().is_empty() {
        return Research {
            text: parts.join("\n"),
            failed: true,
            note: "No link to read — the email will rely on the listing alone.".into(),
        };
    }

    match get_text(url).await {
        Ok(html) => {
            let body = readable(&html);
            if body.trim().is_empty() {
                Research {
                    text: parts.join("\n"),
                    failed: true,
                    note: format!("Opened {url} but found no readable text — likely a JavaScript-rendered page."),
                }
            } else {
                parts.push(format!("From their page:\n{body}"));
                Research {
                    text: parts.join("\n"),
                    failed: false,
                    note: format!("Read {url}"),
                }
            }
        }
        Err(e) => Research {
            text: parts.join("\n"),
            failed: true,
            note: format!("Could not open {url} ({e}) — the email will rely on the listing alone."),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readable_text_keeps_sentences_and_drops_nav_labels() {
        let html = r#"<html><body>
            <nav><li>Home</li><li>Pricing</li></nav>
            <h1>We build infrastructure for real-time data</h1>
            <p>Our platform ingests millions of events per second and serves them back.</p>
            <li>Short</li>
        </body></html>"#;
        let out = readable(html);
        assert!(out.contains("real-time data"));
        assert!(out.contains("millions of events"));
        // Nav items are one or two words and must not survive.
        assert!(!out.contains("Pricing"));
        assert!(!out.contains("Short"));
    }

    #[test]
    fn the_extract_is_capped() {
        let long = "<p>".to_string() + &"a very long sentence about things ".repeat(600) + "</p>";
        assert!(readable(&long).len() <= MAX_CHARS + 400);
    }

    #[tokio::test]
    async fn a_missing_url_is_reported_not_faked() {
        // The failure that matters: with no page read, the model must be told
        // so, rather than left to invent a company from its name.
        let r = research("", "Some startup", Some("Raised a seed round")).await;
        assert!(r.failed);
        assert!(r.text.contains("Raised a seed round"));
        assert!(r.note.contains("No link"));
    }

    #[tokio::test]
    async fn what_scout_already_knows_is_always_included() {
        let r = research("", "Launch HN: Foo", Some("Two founders, YC W25")).await;
        assert!(r.text.contains("Launch HN: Foo"));
        assert!(r.text.contains("Two founders"));
    }
}
