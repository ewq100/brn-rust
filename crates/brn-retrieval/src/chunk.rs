//! Splits note text into passages of at most 1600 bytes, preferring to break after whitespace.
const MAX_PASSAGE: usize = 1600;

/// Half-open byte ranges that cover `text` in order, each on UTF-8 boundaries.
pub(crate) fn passages(text: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut start = 0;
    while start < text.len() {
        let mut end = (start + MAX_PASSAGE).min(text.len());
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        if end < text.len() {
            let slice = &text[start..end];
            if let Some((offset, c)) = slice
                .char_indices()
                .rev()
                .find(|(i, c)| *i >= MAX_PASSAGE / 2 && c.is_whitespace())
            {
                end = start + offset + c.len_utf8();
            }
        }
        ranges.push((start, end));
        start = end;
    }
    ranges
}
