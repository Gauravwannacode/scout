//! Proves HTML-to-PDF actually works before the resume feature depends on it.
//!
//! `cargo run --bin pdfcheck`
//!
//! A resume that silently renders as a blank page would be the worst possible
//! failure — it goes to a stranger with a job to offer. So this checks the
//! bytes, not just that the call returned Ok.

use printpdf::*;
use std::collections::BTreeMap;

const SAMPLE: &str = r#"
<html>
  <head><style>
    body { font-family: sans-serif; font-size: 11px; color: #1a1a1a; }
    h1 { font-size: 22px; margin-bottom: 2px; }
    .sub { font-size: 10px; color: #555555; margin-bottom: 14px; }
    h2 { font-size: 12px; margin-top: 14px; margin-bottom: 6px; }
    .role { font-weight: bold; font-size: 11px; }
    .meta { font-size: 9px; color: #666666; }
    p { margin-top: 2px; margin-bottom: 8px; line-height: 1.4; }
  </style></head>
  <body>
    <h1>Gaurav</h1>
    <div class="sub">Second-year CS student &middot; TypeScript, React, Python, Rust</div>
    <h2>PROJECTS</h2>
    <div class="role">Scout &mdash; local-first desktop news and opportunity app</div>
    <div class="meta">Rust, Tauri, React, TypeScript, SQLite</div>
    <p>Reads 28 sources, clusters duplicate coverage, and ranks stories on
    significance and reach. Background scheduler fires OS alarms with the
    window closed.</p>
    <div class="role">Tribal Knowledge Capture &mdash; deployed SaaS</div>
    <div class="meta">Next.js, Supabase, pgvector, Groq, Gemini</div>
    <p>Turns recorded interviews into searchable, versioned guides with
    semantic search over embedded sections.</p>
  </body>
</html>
"#;

fn main() {
    let images = BTreeMap::new();
    let fonts = BTreeMap::new();
    let options = GeneratePdfOptions::default();
    let mut warnings = Vec::new();

    let doc = match PdfDocument::from_html(SAMPLE, &images, &fonts, &options, &mut warnings) {
        Ok(d) => d,
        Err(e) => {
            println!("FAILED to render: {e}");
            std::process::exit(1);
        }
    };

    if !warnings.is_empty() {
        println!("{} render warning(s):", warnings.len());
        for w in warnings.iter().take(5) {
            println!("  {w:?}");
        }
    }

    let mut save_warnings = Vec::new();
    let bytes = doc.save(&PdfSaveOptions::default(), &mut save_warnings);

    let out = std::env::temp_dir().join("scout-pdfcheck.pdf");
    std::fs::write(&out, &bytes).expect("could not write the pdf");

    // A PDF that exists but is blank is the failure worth catching. Real
    // content means a meaningful byte count and a readable header.
    let header_ok = bytes.starts_with(b"%PDF");
    println!("\nwrote {} ({} bytes)", out.display(), bytes.len());
    println!("  %PDF header: {}", if header_ok { "yes" } else { "NO" });
    println!(
        "  plausibly non-empty: {}",
        if bytes.len() > 2000 { "yes" } else { "NO — suspiciously small" }
    );

    if !header_ok || bytes.len() < 2000 {
        std::process::exit(1);
    }
}
