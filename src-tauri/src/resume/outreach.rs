//! Turning an opening into a draft: research, tailor, render, hand over.
//!
//! Nothing here sends anything. The last step writes a PDF and returns text
//! for a `mailto:` link, so the final action is always his — a cold email that
//! goes out without being read by the person whose name is on it is a mistake
//! waiting to happen, and the review step costs seconds.

use super::{facts::Fact, render, research, store, tailor};
use crate::settings;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    pub company: String,
    pub subject: String,
    pub body: String,
    pub template: String,
    pub research_note: String,
    /// What the model understood them to be. Shown so he can catch a
    /// misreading before it reaches a founder.
    pub understanding: String,
    /// Titles of the facts that made the cut, in order.
    pub included: Vec<String>,
    /// Ids the model invented, which were dropped. Normally empty; when it is
    /// not, that is worth seeing.
    pub dropped: Vec<String>,
    pub resume_path: Option<String>,
    /// True when the company's page could not be read.
    pub research_failed: bool,
}

fn slug(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_lowercase()
        .chars()
        .take(40)
        .collect()
}

/// Where finished resumes go. A real folder rather than the temp directory,
/// because he will want to find them again.
fn output_dir() -> Option<std::path::PathBuf> {
    let dir = settings::config_dir()?.join("resumes");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// Researches the company, tailors the resume, renders the PDF, and returns
/// the email for review.
pub async fn draft(
    company: &str,
    url: &str,
    title: &str,
    summary: Option<&str>,
) -> Result<Draft, String> {
    let data = store::load();
    let facts = store::enabled_facts(&data);
    if facts.is_empty() {
        return Err("Your fact base is empty — nothing to build a resume from.".into());
    }

    let cfg = settings::load();
    let found = research::research(url, title, summary).await;
    let (tailored, dropped) = tailor::tailor(&facts, company, &found.text, &cfg).await?;

    if tailored.include.is_empty() {
        return Err("The model selected no facts — nothing was written.".into());
    }

    // Only facts that survived validation reach the page.
    let chosen: Vec<&Fact> = tailored
        .include
        .iter()
        .filter_map(|id| facts.iter().find(|f| &f.id == id))
        .collect();

    let html = render::to_html(
        &data.header,
        &chosen,
        &tailored.headline,
        &tailored.template,
    );

    // A failed render must not lose the email, which is the slower half to
    // reproduce — report it and hand back the draft without an attachment.
    let (resume_path, render_error) = match render::to_pdf(&html) {
        Ok(bytes) => match output_dir() {
            Some(dir) => {
                let path = dir.join(format!("Gaurav Mehta — {}.pdf", slug(company)));
                match std::fs::write(&path, &bytes) {
                    Ok(_) => (Some(path.to_string_lossy().to_string()), None),
                    Err(e) => (None, Some(format!("could not save the PDF: {e}"))),
                }
            }
            None => (None, Some("could not resolve an output folder".into())),
        },
        Err(e) => (None, Some(e)),
    };

    let mut note = found.note.clone();
    if let Some(e) = render_error {
        note.push_str(&format!(" · resume not attached: {e}"));
    }

    Ok(Draft {
        company: company.to_string(),
        subject: tailored.email_subject,
        body: tailored.email_body,
        template: tailored.template,
        research_note: note,
        understanding: tailored.research_note,
        included: chosen
            .iter()
            .map(|f| f.title.clone())
            .collect(),
        dropped,
        resume_path,
        research_failed: found.failed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_company_name_becomes_a_safe_filename() {
        assert_eq!(slug("Play.ht (YC W23)"), "play-ht--yc-w23");
        assert_eq!(slug("///"), "");
        assert_eq!(slug("Acme"), "acme");
    }

    #[test]
    fn a_long_name_is_truncated_rather_than_breaking_the_path() {
        let s = slug(&"x".repeat(200));
        assert!(s.len() <= 40);
    }
}
