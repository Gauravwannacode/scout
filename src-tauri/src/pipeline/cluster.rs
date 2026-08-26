use crate::sources::RawItem;
use std::collections::{HashMap, HashSet};

/// A single story, possibly carried by several outlets.
#[derive(Debug, Clone)]
pub struct Cluster {
    /// The item we show. Prefers a primary source over coverage about it.
    pub lead: RawItem,
    /// Every item that turned out to be the same story, lead included.
    pub members: Vec<RawItem>,
}

impl Cluster {
    /// Independent sources carrying this story. This *is* the reach signal —
    /// obtained free as a by-product of de-duplicating.
    pub fn corroborations(&self) -> usize {
        self.members
            .iter()
            .map(|m| m.source.as_str())
            .collect::<HashSet<_>>()
            .len()
    }

    pub fn best_points(&self) -> Option<u32> {
        self.members.iter().filter_map(|m| m.signals.points).max()
    }

    pub fn best_comments(&self) -> Option<u32> {
        self.members.iter().filter_map(|m| m.signals.comments).max()
    }

    pub fn has_primary(&self) -> bool {
        self.members.iter().any(|m| m.signals.primary)
    }
}

/// Words too common in tech headlines to tell two stories apart.
const STOPWORDS: &[&str] = &[
    "the", "a", "an", "and", "or", "but", "of", "to", "in", "on", "for", "with", "at", "by",
    "from", "as", "is", "are", "was", "were", "be", "been", "it", "its", "this", "that", "these",
    "those", "how", "why", "what", "when", "new", "now", "can", "will", "you", "your", "we",
    "our", "us", "into", "about", "after", "before", "more", "most", "up", "down", "out", "over",
    "show", "hn", "ask", "s", "t",
];

/// Crude suffix stripping, not a real stemmer.
///
/// Headlines about one event vary mostly by verb form — "launches" / "launch",
/// "reasoning" / "reason". Without this they look like unrelated words and the
/// same story fails to merge. A full stemmer would be overkill for titles.
fn stem(word: &str) -> String {
    for suffix in ["ing", "es", "s"] {
        if word.len() > suffix.len() + 3 && word.ends_with(suffix) {
            return word[..word.len() - suffix.len()].to_string();
        }
    }
    word.to_string()
}

fn tokenize(text: &str) -> Vec<String> {
    let stop: HashSet<&str> = STOPWORDS.iter().copied().collect();
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 2 && !stop.contains(w))
        .map(stem)
        .collect()
}

/// Strips protocol, `www.`, query strings and trailing slashes so the same
/// article shared with different tracking parameters collapses to one key.
fn canonical_url(url: &str) -> String {
    let no_proto = url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("www.");
    let no_query = no_proto.split(['?', '#']).next().unwrap_or(no_proto);
    no_query.trim_end_matches('/').to_lowercase()
}

/// Inverse document frequency over the corpus. Rare words — company names,
/// model names, the specific verb — carry the signal; common ones do not.
fn build_idf(docs: &[Vec<String>]) -> HashMap<String, f32> {
    let n = docs.len().max(1) as f32;
    let mut df: HashMap<String, usize> = HashMap::new();
    for doc in docs {
        for term in doc.iter().collect::<HashSet<_>>() {
            *df.entry(term.clone()).or_insert(0) += 1;
        }
    }
    df.into_iter()
        .map(|(term, count)| {
            // Smoothed IDF, always >= 1. The textbook ln(n/df) form collapses
            // to zero for any term present in every document, which silently
            // deletes exactly the shared words that prove two headlines are
            // the same story. Rare terms still dominate; common ones merely
            // count for less instead of nothing.
            let idf = ((n + 1.0) / (1.0 + count as f32)).ln() + 1.0;
            (term, idf)
        })
        .collect()
}

/// L2-normalised IDF-weighted term vector.
fn vectorize(tokens: &[String], idf: &HashMap<String, f32>) -> HashMap<String, f32> {
    let mut v: HashMap<String, f32> = HashMap::new();
    for t in tokens {
        let w = idf.get(t).copied().unwrap_or(0.0);
        if w > 0.0 {
            *v.entry(t.clone()).or_insert(0.0) += w;
        }
    }
    let norm: f32 = v.values().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for val in v.values_mut() {
            *val /= norm;
        }
    }
    v
}

fn cosine(a: &HashMap<String, f32>, b: &HashMap<String, f32>) -> f32 {
    // Iterate the smaller vector; headline vectors are tiny either way.
    let (small, large) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    small
        .iter()
        .filter_map(|(k, v)| large.get(k).map(|w| v * w))
        .sum()
}

/// Titles this similar are treated as the same story. Tuned for headlines:
/// high enough that two different stories about one company stay apart, low
/// enough that differently-worded coverage of one event merges.
const SIMILARITY_THRESHOLD: f32 = 0.52;

/// Groups items into distinct stories.
///
/// Two passes: identical canonical URLs merge outright, then IDF-weighted
/// cosine over titles catches the same event written up differently. Nothing
/// is discarded — a merged member still counts toward `corroborations`, which
/// is exactly the "how many people already know this" measurement.
pub fn cluster(items: Vec<RawItem>) -> Vec<Cluster> {
    if items.is_empty() {
        return Vec::new();
    }

    let docs: Vec<Vec<String>> = items.iter().map(|i| tokenize(&i.title)).collect();
    let idf = build_idf(&docs);
    let vectors: Vec<HashMap<String, f32>> =
        docs.iter().map(|d| vectorize(d, &idf)).collect();

    let mut clusters: Vec<(HashMap<String, f32>, Vec<usize>)> = Vec::new();
    let mut by_url: HashMap<String, usize> = HashMap::new();

    for (i, item) in items.iter().enumerate() {
        let url_key = canonical_url(&item.url);

        // Exact same article, shared twice.
        if let Some(&ci) = by_url.get(&url_key) {
            clusters[ci].1.push(i);
            continue;
        }

        // Otherwise find the closest existing cluster above the threshold.
        let mut best: Option<(usize, f32)> = None;
        for (ci, (centroid, _)) in clusters.iter().enumerate() {
            let sim = cosine(&vectors[i], centroid);
            if sim >= SIMILARITY_THRESHOLD && best.map_or(true, |(_, b)| sim > b) {
                best = Some((ci, sim));
            }
        }

        match best {
            Some((ci, _)) => {
                // Fold the new member into the centroid rather than keeping the
                // first headline as a fixed reference. Each outlet phrases the
                // same event differently, so an averaged centroid recognises
                // later wordings that the first headline alone would miss.
                let (centroid, members) = &mut clusters[ci];
                members.push(i);
                let n = members.len() as f32;
                for (term, weight) in &vectors[i] {
                    *centroid.entry(term.clone()).or_insert(0.0) += *weight;
                }
                let norm: f32 = centroid.values().map(|x| x * x).sum::<f32>().sqrt();
                if norm > 0.0 {
                    for value in centroid.values_mut() {
                        *value /= norm;
                    }
                }
                let _ = n;
            }
            None => {
                clusters.push((vectors[i].clone(), vec![i]));
                by_url.insert(url_key, clusters.len() - 1);
            }
        }
    }

    clusters
        .into_iter()
        .map(|(_, indices)| {
            let mut members: Vec<RawItem> =
                indices.into_iter().map(|i| items[i].clone()).collect();

            // Show the source closest to the thing itself. A vendor's own post
            // beats a writeup of it; failing that, the fullest headline.
            members.sort_by(|a, b| {
                b.signals
                    .primary
                    .cmp(&a.signals.primary)
                    .then_with(|| b.title.len().cmp(&a.title.len()))
            });

            Cluster {
                lead: members[0].clone(),
                members,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::ReachSignals;

    fn item(source: &str, title: &str, url: &str, primary: bool) -> RawItem {
        RawItem {
            kind: "news".into(),
            title: title.into(),
            org: None,
            url: url.into(),
            summary: None,
            published_at: None,
            deadline_at: None,
            source: source.into(),
            external_id: format!("{source}-{title}"),
            signals: ReachSignals {
                points: None,
                comments: None,
                primary,
            },
        }
    }

    #[test]
    fn the_same_story_across_outlets_becomes_one_cluster() {
        // This is the headline behaviour: four outlets, one story, and the
        // count of four is what tells us the story is already crowded.
        let items = vec![
            item("verge", "OpenAI launches GPT-6 with new reasoning modes", "verge.com/a", false),
            item("ars", "OpenAI launches GPT-6, adding new reasoning modes", "arstechnica.com/b", false),
            item("techcrunch", "OpenAI launches GPT-6 with reasoning modes", "techcrunch.com/c", false),
            item("openai", "Introducing GPT-6: new reasoning modes launch", "openai.com/d", true),
        ];
        let clusters = cluster(items);
        assert_eq!(clusters.len(), 1, "four reports of one story must merge");
        assert_eq!(clusters[0].corroborations(), 4);
    }

    #[test]
    fn the_primary_source_leads_its_cluster() {
        let items = vec![
            item("verge", "OpenAI launches GPT-6 with new reasoning modes", "verge.com/a", false),
            item("openai", "OpenAI launches GPT-6 with new reasoning modes", "openai.com/d", true),
        ];
        let clusters = cluster(items);
        assert_eq!(clusters.len(), 1);
        assert_eq!(
            clusters[0].lead.source, "openai",
            "the vendor's own post should lead over coverage of it"
        );
    }

    #[test]
    fn unrelated_stories_stay_apart() {
        let items = vec![
            item("verge", "OpenAI launches GPT-6 with new reasoning modes", "a.com/1", false),
            item("ars", "Rust 1.99 stabilises async closures at long last", "b.com/2", false),
            item("hn-front", "A minimalist vi-like text editor written in Zig", "c.com/3", false),
        ];
        assert_eq!(cluster(items).len(), 3);
    }

    #[test]
    fn two_stories_about_one_company_stay_apart() {
        // The failure mode that a naive keyword match would hit: both mention
        // the same company, but they are not the same news.
        let items = vec![
            item("verge", "OpenAI launches GPT-6 with new reasoning modes", "a.com/1", false),
            item("ars", "OpenAI faces lawsuit over training data in Germany", "b.com/2", false),
        ];
        assert_eq!(cluster(items).len(), 2);
    }

    #[test]
    fn the_same_url_merges_regardless_of_headline() {
        let items = vec![
            item("hn-front", "Some completely different wording here", "https://x.com/post?utm=1", false),
            item("lobsters", "Another unrelated phrasing entirely", "http://www.x.com/post/", false),
        ];
        let clusters = cluster(items);
        assert_eq!(clusters.len(), 1, "tracking params and www must not defeat the merge");
    }

    #[test]
    fn empty_input_is_handled() {
        assert!(cluster(Vec::new()).is_empty());
    }
}
