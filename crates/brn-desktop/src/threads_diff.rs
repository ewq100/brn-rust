//! Tracked-change presentation of one exact whole-note candidate. Display only:
//! approval always binds the complete candidate, never these segments.
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Mark {
    Deleted,
    Inserted,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Tracked {
    /// The candidate interleaved with deleted current text.
    pub(super) text: String,
    pub(super) marks: Vec<(Range<usize>, Mark)>,
    pub(super) added: usize,
    pub(super) removed: usize,
}
impl Tracked {
    fn push(&mut self, text: &str, mark: Option<Mark>) {
        if text.is_empty() {
            return;
        }
        let start = self.text.len();
        self.text.push_str(text);
        if let Some(mark) = mark {
            match self.marks.last_mut() {
                Some((range, last)) if *last == mark && range.end == start => {
                    range.end = self.text.len()
                }
                _ => self.marks.push((start..self.text.len(), mark)),
            }
        }
    }
    /// The candidate text with deleted segments removed.
    pub(super) fn candidate(&self) -> String {
        let mut out = String::new();
        let mut at = 0;
        for (range, mark) in &self.marks {
            out.push_str(&self.text[at..range.start]);
            if *mark == Mark::Inserted {
                out.push_str(&self.text[range.clone()]);
            }
            at = range.end;
        }
        out.push_str(&self.text[at..]);
        out
    }
}

const MAX_CELLS: usize = 4_000_000;
const WORD_SIMILARITY: f32 = 0.6;

fn paragraphs(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\n' && bytes.get(i + 1) == Some(&b'\n') {
            let mut end = i;
            while end < bytes.len() && bytes[end] == b'\n' {
                end += 1;
            }
            out.push(&text[start..end]);
            start = end;
            i = end;
        } else {
            i += 1;
        }
    }
    if start < text.len() {
        out.push(&text[start..]);
    }
    out
}

fn tokens(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut space = None;
    for (index, ch) in text.char_indices() {
        let is_space = ch.is_whitespace();
        if space.is_some_and(|previous| previous != is_space) {
            out.push(&text[start..index]);
            start = index;
        }
        space = Some(is_space);
    }
    if start < text.len() {
        out.push(&text[start..]);
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    Same(usize, usize),
    Delete(usize),
    Insert(usize),
}

/// Longest common subsequence script, or `None` beyond the work bound.
fn script<T: PartialEq>(a: &[T], b: &[T]) -> Option<Vec<Op>> {
    let (n, m) = (a.len(), b.len());
    if (n + 1).saturating_mul(m + 1) > MAX_CELLS {
        return None;
    }
    let width = m + 1;
    let mut table = vec![0u32; (n + 1) * width];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            table[i * width + j] = if a[i] == b[j] {
                table[(i + 1) * width + j + 1] + 1
            } else {
                table[(i + 1) * width + j].max(table[i * width + j + 1])
            };
        }
    }
    let (mut i, mut j, mut ops) = (0, 0, Vec::new());
    while i < n && j < m {
        if a[i] == b[j] {
            ops.push(Op::Same(i, j));
            i += 1;
            j += 1;
        } else if table[(i + 1) * width + j] >= table[i * width + j + 1] {
            ops.push(Op::Delete(i));
            i += 1;
        } else {
            ops.push(Op::Insert(j));
            j += 1;
        }
    }
    ops.extend((i..n).map(Op::Delete));
    ops.extend((j..m).map(Op::Insert));
    Some(ops)
}

fn similarity(a: &str, b: &str) -> f32 {
    let a: Vec<_> = a.split_whitespace().collect();
    let b: Vec<_> = b.split_whitespace().collect();
    if a.is_empty() || b.is_empty() {
        return 0.;
    }
    let Some(ops) = script(&a, &b) else {
        return 0.;
    };
    let same = ops.iter().filter(|op| matches!(op, Op::Same(..))).count();
    same as f32 / a.len().max(b.len()) as f32
}

fn words(out: &mut Tracked, current: &str, candidate: &str) {
    let (a, b) = (tokens(current), tokens(candidate));
    let Some(ops) = script(&a, &b) else {
        out.push(current, Some(Mark::Deleted));
        out.push(candidate, Some(Mark::Inserted));
        return;
    };
    let (mut deleted, mut inserted) = (String::new(), String::new());
    let flush = |out: &mut Tracked, deleted: &mut String, inserted: &mut String| {
        out.push(deleted, Some(Mark::Deleted));
        out.push(inserted, Some(Mark::Inserted));
        deleted.clear();
        inserted.clear();
    };
    for op in ops {
        match op {
            Op::Same(i, _) => {
                flush(out, &mut deleted, &mut inserted);
                out.push(a[i], None);
            }
            Op::Delete(i) => deleted.push_str(a[i]),
            Op::Insert(j) => inserted.push_str(b[j]),
        }
    }
    flush(out, &mut deleted, &mut inserted);
}

fn block(out: &mut Tracked, deleted: &[&str], inserted: &[&str]) {
    let mut paired = 0;
    while paired < deleted.len()
        && paired < inserted.len()
        && similarity(deleted[paired], inserted[paired]) >= WORD_SIMILARITY
    {
        paired += 1;
    }
    if paired == deleted.len().min(inserted.len()) && paired > 0 {
        for (current, candidate) in deleted.iter().zip(inserted) {
            words(out, current, candidate);
        }
        for current in &deleted[paired..] {
            out.push(current, Some(Mark::Deleted));
        }
        for candidate in &inserted[paired..] {
            out.push(candidate, Some(Mark::Inserted));
        }
    } else {
        for current in deleted {
            out.push(current, Some(Mark::Deleted));
        }
        // Display-only separation; `candidate()` drops it with the deletion.
        if !inserted.is_empty() && deleted.last().is_some_and(|last| !last.ends_with('\n')) {
            out.push("\n\n", Some(Mark::Deleted));
        }
        for candidate in inserted {
            out.push(candidate, Some(Mark::Inserted));
        }
    }
    out.removed += deleted.len();
    out.added += inserted.len();
}

/// Word-level changes for lightly edited paragraphs; whole delete/insert
/// paragraph blocks for heavy rewrites or beyond the work bound.
pub(super) fn tracked(current: &str, candidate: &str) -> Tracked {
    let mut out = Tracked::default();
    let (a, b) = (paragraphs(current), paragraphs(candidate));
    let Some(ops) = script(&a, &b) else {
        block(&mut out, &a, &b);
        return out;
    };
    let (mut deleted, mut inserted): (Vec<&str>, Vec<&str>) = (vec![], vec![]);
    for op in ops {
        match op {
            Op::Same(i, _) => {
                if !deleted.is_empty() || !inserted.is_empty() {
                    block(&mut out, &deleted, &inserted);
                    deleted.clear();
                    inserted.clear();
                }
                out.push(a[i], None);
            }
            Op::Delete(i) => deleted.push(a[i]),
            Op::Insert(j) => inserted.push(b[j]),
        }
    }
    if !deleted.is_empty() || !inserted.is_empty() {
        block(&mut out, &deleted, &inserted);
    }
    out
}

/// A short excerpt of the first changed paragraphs for a proposal card.
pub(super) fn preview(current: &str, candidate: &str, paragraphs_shown: usize) -> Tracked {
    let full = tracked(current, candidate);
    let mut out = Tracked {
        added: full.added,
        removed: full.removed,
        ..Tracked::default()
    };
    let mut shown = 0;
    for paragraph in self::paragraphs(&full.text) {
        if shown == paragraphs_shown {
            break;
        }
        let start = paragraph.as_ptr() as usize - full.text.as_ptr() as usize;
        let range = start..start + paragraph.len();
        if !full
            .marks
            .iter()
            .any(|(mark, _)| mark.start < range.end && range.start < mark.end)
        {
            continue;
        }
        shown += 1;
        let mut at = range.start;
        for (mark, kind) in &full.marks {
            let (start, end) = (mark.start.max(range.start), mark.end.min(range.end));
            if start >= end {
                continue;
            }
            out.push(&full.text[at..start], None);
            out.push(&full.text[start..end], Some(*kind));
            at = end;
        }
        out.push(full.text[at..range.end].trim_end_matches('\n'), None);
        out.push("\n", None);
    }
    let trimmed = out.text.trim_end().len();
    out.text.truncate(trimmed);
    out.marks.retain_mut(|(range, _)| {
        range.end = range.end.min(trimmed);
        range.start < range.end
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_edit_marks_words_and_reconstructs_both_texts() {
        let current = "# Title\n\nKeep this line.\n\nThe quick brown fox jumps over the dog.\n";
        let candidate =
            "# Title\n\nKeep this line.\n\nThe quick red fox jumps over the lazy dog.\n";
        let tracked = tracked(current, candidate);
        assert_eq!(tracked.candidate(), candidate);
        let deleted: Vec<_> = tracked
            .marks
            .iter()
            .filter(|(_, mark)| *mark == Mark::Deleted)
            .map(|(range, _)| &tracked.text[range.clone()])
            .collect();
        assert_eq!(deleted, ["brown"]);
        let inserted: Vec<_> = tracked
            .marks
            .iter()
            .filter(|(_, mark)| *mark == Mark::Inserted)
            .map(|(range, _)| tracked.text[range.clone()].trim())
            .collect();
        assert_eq!(inserted, ["red", "lazy"]);
        assert_eq!((tracked.added, tracked.removed), (1, 1));
    }

    #[test]
    fn heavy_rewrite_uses_whole_paragraph_blocks() {
        let current = "Alpha beta gamma delta.\n\nUnchanged.";
        let candidate = "Completely different wording here now.\n\nUnchanged.";
        let tracked = tracked(current, candidate);
        assert_eq!(
            tracked
                .marks
                .iter()
                .map(|(_, mark)| *mark)
                .collect::<Vec<_>>(),
            [Mark::Deleted, Mark::Inserted]
        );
        assert_eq!(
            &tracked.text[tracked.marks[0].0.clone()],
            "Alpha beta gamma delta.\n\n"
        );
        assert_eq!(tracked.candidate(), candidate);
    }

    #[test]
    fn identical_and_unicode_text_have_no_marks_and_valid_boundaries() {
        let text = "Õun ja päike.\n\nÜks lõik.";
        assert!(tracked(text, text).marks.is_empty());
        let changed = tracked(text, "Õun ja kuu.\n\nÜks lõik.");
        for (range, _) in &changed.marks {
            assert!(changed.text.is_char_boundary(range.start));
            assert!(changed.text.is_char_boundary(range.end));
        }
        assert_eq!(changed.candidate(), "Õun ja kuu.\n\nÜks lõik.");
        let added = tracked("", "New note text.");
        assert_eq!(added.marks, [(0..14, Mark::Inserted)]);
    }

    #[test]
    fn oversized_inputs_fall_back_to_blocks_and_preview_is_bounded() {
        let current = (0..2100)
            .map(|i| format!("p{i}"))
            .collect::<Vec<_>>()
            .join("\n\n");
        let candidate = (0..2100)
            .map(|i| format!("q{i}"))
            .collect::<Vec<_>>()
            .join("\n\n");
        let tracked = tracked(&current, &candidate);
        assert_eq!(tracked.candidate(), candidate);
        assert_eq!((tracked.added, tracked.removed), (2100, 2100));
        let preview = preview(
            "One.\n\nTwo words here.\n\nThree.\n\nFour.",
            "One.\n\nTwo new words here.\n\nThree.\n\nFour more.",
            1,
        );
        assert_eq!(preview.text, "Two new words here.");
        assert_eq!(preview.marks.len(), 1);
        assert_eq!((preview.added, preview.removed), (2, 2));
    }
}
