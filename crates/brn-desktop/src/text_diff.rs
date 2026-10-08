//! Word-level comparison of two proposal versions for the review "Changes" view.
//!
//! Presentation only: the result marks what changed between two texts the app
//! has seen; it never edits, approves or re-anchors anything.

use std::ops::Range;

/// One changed region. `new` is the byte range in the newer text (empty when
/// text was only removed); `old` is the removed text (empty when only added).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub new: Range<usize>,
    pub old: String,
}

impl Change {
    pub fn added(&self) -> bool {
        self.old.trim().is_empty()
    }
    pub fn removed(&self) -> bool {
        self.new.is_empty()
    }
}

/// Largest comparison table, in token pairs, after trimming the shared start and end.
const MAX_CELLS: usize = 4_000_000;

fn tokens(text: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut iter = text.char_indices().peekable();
    while let Some((start, ch)) = iter.next() {
        let class = |c: char| {
            if c.is_alphanumeric() || c == '\'' || c == '’' {
                0
            } else if c.is_whitespace() {
                1
            } else {
                2
            }
        };
        let kind = class(ch);
        let mut end = start + ch.len_utf8();
        if kind != 2 {
            while let Some(&(next, c)) = iter.peek() {
                if class(c) != kind {
                    break;
                }
                end = next + c.len_utf8();
                iter.next();
            }
        }
        out.push(start..end);
    }
    out
}

/// Changes from `old` to `new`, or `None` when the texts are too large to compare.
pub fn diff(old: &str, new: &str) -> Option<Vec<Change>> {
    let a = tokens(old);
    let b = tokens(new);
    let same = |i: usize, j: usize| old[a[i].clone()] == new[b[j].clone()];
    let mut prefix = 0;
    while prefix < a.len() && prefix < b.len() && same(prefix, prefix) {
        prefix += 1;
    }
    let mut suffix = 0;
    while suffix < a.len() - prefix
        && suffix < b.len() - prefix
        && same(a.len() - 1 - suffix, b.len() - 1 - suffix)
    {
        suffix += 1;
    }
    let (n, m) = (a.len() - prefix - suffix, b.len() - prefix - suffix);
    if n.saturating_mul(m) > MAX_CELLS {
        return None;
    }
    // lcs[i][j]: common tokens of a[prefix+i..] and b[prefix+j..].
    let width = m + 1;
    let mut lcs = vec![0u32; (n + 1) * width];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[i * width + j] = if same(prefix + i, prefix + j) {
                lcs[(i + 1) * width + j + 1] + 1
            } else {
                lcs[(i + 1) * width + j].max(lcs[i * width + j + 1])
            };
        }
    }
    let new_at = |j: usize| {
        if prefix + j < b.len() {
            b[prefix + j].start
        } else {
            new.len()
        }
    };
    let mut changes = Vec::new();
    let (mut i, mut j) = (0, 0);
    // Start of the current changed run: (old token index, new token index).
    let mut open: Option<(usize, usize)> = None;
    let mut flush = |open: &mut Option<(usize, usize)>, i: usize, j: usize| {
        if let Some((oi, nj)) = open.take() {
            let removed: String = (oi..i).map(|k| &old[a[prefix + k].clone()]).collect();
            let range = new_at(nj)..new_at(j);
            let added = &new[range.clone()];
            if !(removed.trim().is_empty() && added.trim().is_empty()) {
                changes.push(Change {
                    new: range,
                    old: removed,
                });
            }
        }
    };
    while i < n || j < m {
        if i < n && j < m && same(prefix + i, prefix + j) {
            flush(&mut open, i, j);
            i += 1;
            j += 1;
        } else {
            open.get_or_insert((i, j));
            if j < m && (i == n || lcs[i * width + j + 1] >= lcs[(i + 1) * width + j]) {
                j += 1;
            } else {
                i += 1;
            }
        }
    }
    flush(&mut open, i, j);
    Some(changes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn show(old: &str, new: &str) -> Vec<(String, String)> {
        diff(old, new)
            .unwrap()
            .into_iter()
            .map(|c| (c.old, new[c.new].to_owned()))
            .collect()
    }

    #[test]
    fn identical_texts_have_no_changes() {
        assert!(
            diff("How we buy paint.", "How we buy paint.")
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn replaced_words_insertions_and_removals_are_located_in_the_new_text() {
        assert_eq!(
            show(
                "larger orders need a manager's approval.",
                "larger orders need Märt's approval."
            ),
            vec![("a manager's".into(), "Märt's".into())]
        );
        assert_eq!(
            show(
                "Photograph the labels.",
                "Photograph the labels. Keep the receipt."
            ),
            vec![(String::new(), " Keep the receipt.".into())]
        );
        let removed = diff(
            "Check batch numbers carefully now.",
            "Check batch numbers now.",
        )
        .unwrap();
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].old.trim(), "carefully");
        assert!(removed[0].removed());
    }

    #[test]
    fn multibyte_text_keeps_char_boundaries() {
        let changes = show("Värvikoda ütleb 5 päeva", "Värvikoda ütleb 7 päeva");
        assert_eq!(changes, vec![("5".into(), "7".into())]);
    }

    #[test]
    fn oversized_comparisons_are_refused() {
        let old: String = (0..2100).map(|i| format!("a{i} ")).collect();
        let new: String = (0..2100).map(|i| format!("b{i} ")).collect();
        assert!(diff(&old, &new).is_none());
    }
}
