//! Short matching context for a search result: a bounded window of the best
//! passage around the first query word, with ranges to highlight.
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Snippet {
    /// Whitespace-collapsed text, with `…` where the passage was cut.
    pub text: String,
    /// Byte ranges in `text` of words matching the query.
    pub highlights: Vec<Range<usize>>,
}

fn words(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(move |w| (w.as_ptr() as usize - text.as_ptr() as usize, w))
}

/// Query terms, as the index splits them.
fn terms(query: &str) -> Vec<String> {
    query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// Exact, or the same English stem by a shared prefix (`deploying`,
/// `deployment`), approximating the stemmed lane for display only.
fn matches(word: &str, terms: &[String]) -> bool {
    let word = word.to_lowercase();
    terms.iter().any(|term| {
        if *term == word {
            return true;
        }
        let shortest = term.chars().count().min(word.chars().count());
        if shortest < 4 {
            return false;
        }
        let shared = term
            .chars()
            .zip(word.chars())
            .take_while(|(a, b)| a == b)
            .count();
        shared >= 4.max(shortest.saturating_sub(3))
    })
}

fn floor(text: &str, mut i: usize) -> usize {
    i = i.min(text.len());
    while !text.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// At most `max_bytes` (plus ellipses) of `passage` around the first word
/// matching `query`; the beginning when no word matches (a title match).
pub fn snippet(passage: &str, query: &str, max_bytes: usize) -> Snippet {
    let flat: String = passage.split_whitespace().collect::<Vec<_>>().join(" ");
    let terms = terms(query);
    let first = words(&flat)
        .find(|(_, w)| matches(w, &terms))
        .map_or(0, |(at, _)| at);
    let mut start = floor(&flat, first.saturating_sub(max_bytes / 3));
    if start > 0 {
        // Begin at a word boundary when one is near.
        if let Some(space) = flat[start..first].find(' ') {
            start += space + 1;
        }
    }
    let mut end = floor(&flat, start + max_bytes);
    if end < flat.len()
        && let Some(space) = flat[start..end].rfind(' ')
        && space > 0
    {
        end = start + space;
    }
    let prefix = if start > 0 { "…" } else { "" };
    let suffix = if end < flat.len() { "…" } else { "" };
    let body = &flat[start..end];
    let highlights = words(body)
        .filter(|(_, w)| matches(w, &terms))
        .map(|(at, w)| prefix.len() + at..prefix.len() + at + w.len())
        .collect();
    Snippet {
        text: format!("{prefix}{body}{suffix}"),
        highlights,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centres_on_the_first_match_and_highlights_stems() {
        let passage = format!(
            "{} The deployment of valves went well.\n\nLater deploying again. {}",
            "lead ".repeat(200),
            "tail ".repeat(200)
        );
        let s = snippet(&passage, "Deploying", 120);
        assert!(s.text.starts_with('…') && s.text.ends_with('…'));
        assert!(s.text.len() <= 120 + 2 * '…'.len_utf8());
        let marked: Vec<&str> = s.highlights.iter().map(|r| &s.text[r.clone()]).collect();
        assert_eq!(marked, ["deployment", "deploying"]);
    }

    #[test]
    fn title_only_matches_show_the_beginning_and_never_split_characters() {
        let s = snippet("õun õun õun õun", "missing", 6);
        assert_eq!(s.text, "õun…");
        assert!(s.highlights.is_empty());
        let s = snippet("short text", "text", 300);
        assert_eq!(s.text, "short text");
        assert_eq!(&s.text[s.highlights[0].clone()], "text");
    }

    #[test]
    fn bounded_for_any_input() {
        for (i, passage) in ["", "a", "ä ö ü", "x".repeat(5000).as_str()]
            .into_iter()
            .enumerate()
        {
            let s = snippet(passage, "x ä", 50 + i);
            assert!(s.text.len() <= 50 + i + 2 * '…'.len_utf8(), "{passage}");
            for r in &s.highlights {
                assert!(s.text.is_char_boundary(r.start) && s.text.is_char_boundary(r.end));
            }
        }
    }
}
