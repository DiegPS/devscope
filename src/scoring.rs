use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScoreEntry {
    pub visits: u32,
    pub opens: u32,
    pub last_used: Option<i64>,
}

pub type ScoreMap = HashMap<String, ScoreEntry>;

/// Tokenize a folder/project name by splitting on separators
/// (`-`, `_`, ` `, `.`) and camelCase boundaries.
pub fn tokenize(name: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    for ch in name.chars() {
        if ch == '-' || ch == '_' || ch == ' ' || ch == '.' {
            if !current.is_empty() {
                tokens.push(current.to_lowercase());
                current.clear();
            }
        } else if ch.is_uppercase()
            && !current.is_empty()
            && !current.ends_with(|c: char| c.is_uppercase())
        {
            tokens.push(current.to_lowercase());
            current.clear();
            current.push(ch);
        } else {
            current.push(ch);
        }
    }

    if !current.is_empty() {
        tokens.push(current.to_lowercase());
    }

    tokens
}

/// Check if a query matches a project name using token-aware matching.
/// Returns true if the query matches via any of these strategies:
/// - Compact-name substring (alphanumeric only, no separators)
/// - Subsequence match (characters appear in order in compact name)
/// - Token prefix match (each query token is a prefix of some name token)
pub fn matches_name(name: &str, query: &str) -> bool {
    let name_lower = name.to_lowercase();
    let query_lower = query.to_lowercase();

    let name_compact: String = name_lower.chars().filter(|c| c.is_alphanumeric()).collect();

    if name_compact.contains(&query_lower) {
        return true;
    }

    if is_subsequence(&query_lower, &name_compact) {
        return true;
    }

    let name_tokens = tokenize(name);
    let query_tokens = tokenize(&query_lower);

    if !query_tokens.is_empty()
        && query_tokens
            .iter()
            .all(|qt| name_tokens.iter().any(|nt| nt.starts_with(qt)))
    {
        return true;
    }

    false
}

/// Check if `query` is a subsequence of `text` (chars in order, not necessarily consecutive).
fn is_subsequence(query: &str, text: &str) -> bool {
    let mut text_chars = text.chars();
    for qc in query.chars() {
        loop {
            match text_chars.next() {
                Some(tc) if tc == qc => break,
                Some(_) => continue,
                None => return false,
            }
        }
    }
    true
}

/// Compute a ranking score for a project.
/// Combines text match quality with usage frequency and recency.
/// Higher score = better match.
pub fn compute_score(name: &str, query: &str, entry: &ScoreEntry) -> f64 {
    if query.is_empty() {
        return frecency_score(entry);
    }

    let name_lower = name.to_lowercase();
    let query_lower = query.to_lowercase();
    let name_tokens = tokenize(name);
    let query_tokens = tokenize(&query_lower);

    let mut score = 0.0;

    if name_lower == query_lower {
        score += 100.0;
    } else if name_lower.starts_with(&query_lower) {
        score += 50.0;
    } else {
        let name_compact: String = name_lower.chars().filter(|c| c.is_alphanumeric()).collect();
        if name_compact.starts_with(&query_lower) {
            score += 30.0;
        } else if name_compact.contains(&query_lower) {
            score += 20.0;
        } else if is_subsequence(&query_lower, &name_compact) {
            score += 10.0;
        }
    }

    for qt in &query_tokens {
        let mut best: f64 = 0.0;
        for nt in &name_tokens {
            if nt == qt {
                best = best.max(10.0);
            } else if nt.starts_with(qt) {
                best = best.max(7.0);
            } else if nt.contains(qt) {
                best = best.max(4.0);
            }
        }
        score += best;
    }

    score += frecency_score(entry) * 3.0;

    score
}

/// Score based purely on frequency + recency (no search query active).
fn frecency_score(entry: &ScoreEntry) -> f64 {
    let total = (entry.visits + entry.opens) as f64;
    let usage_bonus = if total > 0.0 { (1.0 + total).ln() } else { 0.0 };
    let recency_bonus = recency_bonus(entry);
    usage_bonus + recency_bonus
}

fn recency_bonus(entry: &ScoreEntry) -> f64 {
    let Some(last_used) = entry.last_used else {
        return 0.0;
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let age_hours = (now - last_used) as f64 / 3600.0;

    if age_hours < 1.0 {
        1.0
    } else if age_hours < 24.0 {
        0.7
    } else if age_hours < 168.0 {
        0.4
    } else if age_hours < 720.0 {
        0.2
    } else {
        0.05
    }
}
