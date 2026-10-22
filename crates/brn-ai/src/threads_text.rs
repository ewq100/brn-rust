//! Small byte-exact text primitives shared by local edits and research.

/// Return the byte range of a nonempty quote only when it occurs exactly once.
/// Searching after the first character counts overlapping occurrences too.
pub(super) fn unique_range(text: &str, quote: &str) -> Option<std::ops::Range<usize>> {
    if quote.is_empty() {
        return None;
    }
    let start = text.find(quote)?;
    let next = start + text[start..].chars().next()?.len_utf8();
    if text[next..].contains(quote) {
        None
    } else {
        Some(start..start + quote.len())
    }
}

#[cfg(test)]
mod tests {
    use super::unique_range;

    #[test]
    fn exact_unique_ranges_count_overlapping_utf8_matches() {
        assert_eq!(unique_range("before Ω quote after", "Ω quote"), Some(7..15));
        assert_eq!(unique_range("abc quote", "quote"), Some(4..9));
        assert_eq!(unique_range("aaa", "aa"), None);
        assert_eq!(unique_range("Ω Ω", "Ω"), None);
        assert_eq!(unique_range("ΩΩΩ", "ΩΩ"), None);
        assert_eq!(unique_range("text", ""), None);
        assert_eq!(unique_range("text", "longer"), None);
    }
}
