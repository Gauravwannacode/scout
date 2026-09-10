//! Drafts a real outreach email end to end, so the output can be judged
//! before it is ever sent to anyone.
//!
//! `cargo run --bin outreachcheck -- "Company" "https://their.site"`

use app_lib::resume::outreach;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let company = args.get(1).cloned().unwrap_or_else(|| "Groq".to_string());
    let url = args
        .get(2)
        .cloned()
        .unwrap_or_else(|| "https://groq.com".to_string());

    println!("Researching {company} at {url}…\n");

    match outreach::draft(&company, &url, &company, None).await {
        Ok(d) => {
            println!("UNDERSTOOD:  {}", d.understanding);
            println!("RESEARCH:    {}", d.research_note);
            println!("TEMPLATE:    {}", d.template);
            println!("RESUME:      {}", d.resume_path.as_deref().unwrap_or("(none)"));
            if !d.dropped.is_empty() {
                println!("DROPPED:     {:?}  <-- the fact base refused these", d.dropped);
            }
            println!("\nSELECTED ({}):", d.included.len());
            for t in &d.included {
                println!("  · {t}");
            }
            println!("\n--- SUBJECT ---\n{}", d.subject);
            println!("\n--- EMAIL ---\n{}", d.body);
        }
        Err(e) => println!("FAILED: {e}"),
    }
}
