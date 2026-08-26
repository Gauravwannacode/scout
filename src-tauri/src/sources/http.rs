use super::SourceError;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, USER_AGENT};
use std::sync::OnceLock;
use std::time::Duration;

/// Some hosts (notably Devpost) reject requests without a browser-shaped
/// user agent, and several feeds are only served to clients that ask for it.
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                  (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

/// One shared client for the whole app: connection pooling matters when
/// sixteen sources fire at once.
pub fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(UA));
        reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(20))
            .gzip(true)
            .build()
            .expect("http client builds")
    })
}

/// GET returning a string body, with status checked.
pub async fn get_text(url: &str) -> Result<String, SourceError> {
    let res = client().get(url).send().await?;
    let status = res.status();
    if !status.is_success() {
        return Err(SourceError::Parse(format!("HTTP {status}")));
    }
    Ok(res.text().await?)
}

/// GET returning parsed JSON.
pub async fn get_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, SourceError> {
    let res = client()
        .get(url)
        .header(ACCEPT, "application/json")
        .send()
        .await?;
    let status = res.status();
    if !status.is_success() {
        return Err(SourceError::Parse(format!("HTTP {status}")));
    }
    res.json::<T>()
        .await
        .map_err(|e| SourceError::Parse(e.to_string()))
}

/// Collapses whitespace and strips HTML tags from a summary blob.
/// Feed descriptions routinely carry markup we never want to render.
pub fn clean_text(input: &str, max_chars: usize) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    let mut last_space = true;

    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if in_tag => {
                let _ = c;
            }
            c if c.is_whitespace() => {
                if !last_space {
                    out.push(' ');
                    last_space = true;
                }
            }
            c => {
                out.push(c);
                last_space = false;
            }
        }
    }

    let trimmed = out.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let cut: String = trimmed.chars().take(max_chars).collect();
    match cut.rfind(' ') {
        Some(i) => format!("{}…", &cut[..i]),
        None => format!("{cut}…"),
    }
}
