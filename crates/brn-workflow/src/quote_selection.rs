//! Resolve semantic quote selections against exact saved body bytes.
use crate::{ErrorKind, Result, WorkflowError};
use brn_store::note_identity;
use std::ops::Range;

/// Occurrences are 1-based, ordered by their start byte, including overlaps.
/// Without a selector, exactly one body occurrence is required. No normalized
/// matching, metadata matches or model-supplied offsets establish evidence.
pub(crate) fn resolve(text: &str, quote: &str, occurrence: Option<usize>) -> Result<Range<usize>> {
    if occurrence.is_some_and(|n| n == 0 || n > brn_store::work::MAX_NOTE_BYTES) {
        return Err(refused(ErrorKind::QuoteOccurrenceInvalid));
    }
    if quote.is_empty() || quote.len() > 16 * 1024 {
        return Err(refused(ErrorKind::ToolRejected));
    }
    let body = note_identity::body_start(text)?;
    let needle = quote.as_bytes();
    // KMP bounds work by saved-body plus quotation size, including highly
    // repetitive overlapping text. Each accepted start is a UTF-8 boundary
    // because both the whole text and the exact nonempty needle are UTF-8.
    let mut prefix = vec![0; needle.len()];
    for i in 1..needle.len() {
        let mut matched = prefix[i - 1];
        while matched > 0 && needle[i] != needle[matched] {
            matched = prefix[matched - 1];
        }
        if needle[i] == needle[matched] {
            matched += 1;
        }
        prefix[i] = matched;
    }
    let mut matched = 0;
    let mut count = 0;
    let mut unique = None;
    for (offset, &byte) in text.as_bytes()[body..].iter().enumerate() {
        while matched > 0 && byte != needle[matched] {
            matched = prefix[matched - 1];
        }
        if byte == needle[matched] {
            matched += 1;
        }
        if matched == needle.len() {
            count += 1;
            let end = body + offset + 1;
            let range = end - needle.len()..end;
            if occurrence == Some(count) {
                return Ok(range);
            }
            if occurrence.is_none() {
                if unique.is_some() {
                    return Err(refused(ErrorKind::QuoteAmbiguous));
                }
                unique = Some(range);
            }
            matched = prefix[matched - 1];
        }
    }
    if count == 0 {
        Err(refused(ErrorKind::QuoteNotFound))
    } else if occurrence.is_some() {
        Err(refused(ErrorKind::QuoteOccurrenceInvalid))
    } else {
        Ok(unique.expect("one exact occurrence"))
    }
}

fn refused(kind: ErrorKind) -> WorkflowError {
    let message = match kind {
        ErrorKind::QuoteNotFound => "The quotation was not found in the saved note body.",
        ErrorKind::QuoteAmbiguous => "The quotation has multiple body occurrences; select one.",
        ErrorKind::QuoteOccurrenceInvalid => "The selected quotation occurrence is unavailable.",
        _ => "The quotation input is invalid.",
    };
    WorkflowError::typed(kind, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_body_bytes_keep_bom_crlf_unicode_and_exclude_metadata() {
        let text = "\u{feff}---\r\ntitle: Blue õ 🦀\r\n---\r\nBlue õ 🦀\r\n";
        let wording = "Blue õ 🦀\r\n";
        let range = resolve(text, wording, None).unwrap();
        assert_eq!(range.start, text.rfind("Blue").unwrap());
        assert_eq!(text.get(range), Some(wording));
        for wording in ["title:", "Blue õ 🦀\n", "BLUE õ 🦀", "🦀\r"] {
            if wording == "🦀\r" {
                assert_eq!(
                    text.get(resolve(text, wording, None).unwrap()),
                    Some(wording)
                );
            } else {
                assert_eq!(
                    resolve(text, wording, None).unwrap_err().kind,
                    ErrorKind::QuoteNotFound
                );
            }
        }
        assert_eq!(resolve("\u{feff}õ🦀", "õ🦀", None).unwrap(), 3..9);
    }

    #[test]
    fn repeated_overlapping_occurrences_require_explicit_selection() {
        for (text, quote, ranges) in [
            ("aaaa", "aaa", vec![0..3, 1..4]),
            ("õõõ", "õõ", vec![0..4, 2..6]),
            ("ababa", "aba", vec![0..3, 2..5]),
        ] {
            assert_eq!(
                resolve(text, quote, None).unwrap_err().kind,
                ErrorKind::QuoteAmbiguous
            );
            for (index, range) in ranges.iter().enumerate() {
                assert_eq!(resolve(text, quote, Some(index + 1)).unwrap(), *range);
            }
            for occurrence in [0, ranges.len() + 1, usize::MAX] {
                assert_eq!(
                    resolve(text, quote, Some(occurrence)).unwrap_err().kind,
                    ErrorKind::QuoteOccurrenceInvalid
                );
            }
        }
        assert_eq!(
            resolve("body", "absent", Some(1)).unwrap_err().kind,
            ErrorKind::QuoteNotFound
        );
    }

    #[test]
    fn maximum_repeated_body_and_quote_keep_all_overlapping_matches() {
        let text = "a".repeat(brn_store::work::MAX_NOTE_BYTES);
        let quote = "a".repeat(16 * 1024);
        let count = text.len() - quote.len() + 1;
        assert_eq!(
            resolve(&text, &quote, Some(count)).unwrap(),
            count - 1..text.len()
        );
        assert_eq!(
            resolve(&text, &quote, Some(count + 1)).unwrap_err().kind,
            ErrorKind::QuoteOccurrenceInvalid
        );
    }
}
