//! Tailored resumes and the cold emails they accompany.
//!
//! Split so that only one place can introduce a claim: `facts` holds what is
//! true, `store` persists it, `research` reads the company, `tailor` chooses
//! what to show them, `render` draws it, and `outreach` sequences the lot.
//! Everything downstream of `facts` can only select, never invent.

pub mod facts;
pub mod outreach;
pub mod render;
pub mod research;
pub mod store;
pub mod tailor;

use facts::{Fact, Header};

#[tauri::command]
pub fn get_resume() -> store::ResumeData {
    store::load()
}

#[tauri::command]
pub fn save_resume(header: Header, facts: Vec<Fact>) -> Result<(), String> {
    store::save(&store::ResumeData { header, facts })
}

/// Restores the seeded fact base, for when an edit goes wrong.
#[tauri::command]
pub fn reset_resume() -> store::ResumeData {
    let d = store::ResumeData::default();
    let _ = store::save(&d);
    d
}

#[tauri::command]
pub async fn draft_outreach(
    company: String,
    url: String,
    title: String,
    summary: Option<String>,
) -> Result<outreach::Draft, String> {
    outreach::draft(&company, &url, &title, summary.as_deref()).await
}
