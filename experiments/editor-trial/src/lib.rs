use similar::TextDiff;
use std::io::Read;
use std::ops::Range;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Anchor {
    Resolved(Range<usize>),
    Unresolved(&'static str),
}

#[derive(Clone, Debug)]
pub struct Comment {
    pub id: usize,
    pub body: String,
    pub original_quote: String,
    pub original_range: Range<usize>,
    pub anchor: Anchor,
    original_document: String,
}

#[derive(Clone, Debug)]
pub struct PendingSelection {
    range: Range<usize>,
    quote: String,
    document_snapshot: String,
}

impl PendingSelection {
    pub fn quote(&self) -> &str {
        &self.quote
    }
    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }
}

pub const MAX_DOCUMENT_BYTES: usize = 2 * 1024 * 1024;
pub const BUILTIN_DOCUMENT: &str = include_str!("../fixtures/long-document.md");

pub fn load_document_from_args<'a>(
    args: impl IntoIterator<Item = &'a str>,
) -> Result<(String, String), String> {
    let mut args = args.into_iter();
    let Some(path) = args.next() else {
        return Ok(("Built-in Markdown sample".into(), BUILTIN_DOCUMENT.into()));
    };
    if args.next().is_some() {
        return Err("Expected at most one UTF-8 Markdown file path.".into());
    }
    let metadata =
        std::fs::metadata(path).map_err(|error| format!("Cannot open {path}: {error}"))?;
    if !metadata.is_file() {
        return Err(format!("{path} is not a regular file."));
    }
    if metadata.len() > MAX_DOCUMENT_BYTES as u64 {
        return Err(format!(
            "{path} is larger than the {MAX_DOCUMENT_BYTES}-byte trial limit."
        ));
    }
    let file = std::fs::File::open(path).map_err(|error| format!("Cannot open {path}: {error}"))?;
    let mut bytes = Vec::new();
    file.take(MAX_DOCUMENT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Cannot read {path}: {error}"))?;
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err(format!(
            "{path} is larger than the {MAX_DOCUMENT_BYTES}-byte trial limit."
        ));
    }
    let text = String::from_utf8(bytes).map_err(|_| format!("{path} is not valid UTF-8."))?;
    Ok((path.to_owned(), text))
}

pub struct Document {
    baseline: String,
    text: String,
    comments: Vec<Comment>,
}

impl Document {
    pub fn new(text: String) -> Self {
        Self {
            baseline: text.clone(),
            text,
            comments: Vec::new(),
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn baseline(&self) -> &str {
        &self.baseline
    }
    pub fn comments(&self) -> &[Comment] {
        &self.comments
    }
    pub fn is_modified(&self) -> bool {
        self.text != self.baseline
    }

    pub fn capture_selection(&self, range: Range<usize>) -> Result<PendingSelection, String> {
        let quote = self
            .text
            .get(range.clone())
            .filter(|quote| !quote.is_empty())
            .ok_or_else(|| "Select a nonempty UTF-8 passage before capturing it.".to_owned())?;
        Ok(PendingSelection {
            range,
            quote: quote.to_owned(),
            document_snapshot: self.text.clone(),
        })
    }

    pub fn add_comment(
        &mut self,
        pending: PendingSelection,
        body: String,
    ) -> Result<usize, String> {
        if pending.document_snapshot != self.text {
            return Err("The document changed after capture. Select the passage again.".into());
        }
        if body.trim().is_empty() {
            return Err("Enter a comment before adding it.".into());
        }
        let id = self.comments.len() + 1;
        let anchor = Anchor::Resolved(pending.range.clone());
        self.comments.push(Comment {
            id,
            body,
            original_quote: pending.quote,
            original_range: pending.range,
            anchor,
            original_document: self.text.clone(),
        });
        Ok(id)
    }

    pub fn replace_text(&mut self, next: String) {
        if next == self.text {
            return;
        }
        let previous = std::mem::replace(&mut self.text, next);
        for comment in &mut self.comments {
            if self.text == comment.original_document {
                comment.anchor = Anchor::Resolved(comment.original_range.clone());
                continue;
            }
            let Anchor::Resolved(range) = &comment.anchor else {
                continue;
            };
            if quote_occurrences(&previous, &comment.original_quote) != 1
                || quote_occurrences(&self.text, &comment.original_quote) != 1
            {
                comment.anchor = Anchor::Unresolved("The passage is duplicated or missing.");
                continue;
            }
            let (old_edit, new_edit) = changed_span(&previous, &self.text);
            if old_edit.end <= range.start {
                let shifted_start = (range.start as isize + new_edit.len() as isize
                    - old_edit.len() as isize) as usize;
                let shifted = shifted_start..shifted_start + range.len();
                if self.text.get(shifted.clone()) == Some(comment.original_quote.as_str()) {
                    comment.anchor = Anchor::Resolved(shifted);
                } else {
                    comment.anchor = Anchor::Unresolved("The selected passage changed.");
                }
            } else if old_edit.start >= range.end {
                if self.text.get(range.clone()) != Some(comment.original_quote.as_str()) {
                    comment.anchor = Anchor::Unresolved("The selected passage changed.");
                }
            } else {
                comment.anchor = Anchor::Unresolved("An edit touched the selected passage.");
            }
        }
    }

    pub fn unified_diff(&self) -> String {
        if !self.is_modified() {
            return "No changes from opened document.".into();
        }
        TextDiff::from_lines(&self.baseline, &self.text)
            .unified_diff()
            .header("opened", "current")
            .to_string()
    }
}

fn quote_occurrences(haystack: &str, needle: &str) -> usize {
    haystack
        .char_indices()
        .filter(|(i, _)| haystack[*i..].starts_with(needle))
        .take(2)
        .count()
}

fn changed_span(previous: &str, next: &str) -> (Range<usize>, Range<usize>) {
    let mut prefix = 0;
    for (a, b) in previous.chars().zip(next.chars()) {
        if a != b {
            break;
        }
        prefix += a.len_utf8();
    }
    let mut old_end = previous.len();
    let mut new_end = next.len();
    while old_end > prefix && new_end > prefix {
        let a = previous[..old_end].chars().next_back().unwrap();
        let b = next[..new_end].chars().next_back().unwrap();
        if a != b {
            break;
        }
        old_end -= a.len_utf8();
        new_end -= b.len_utf8();
    }
    (prefix..old_end, prefix..new_end)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anchored(text: &str, range: Range<usize>) -> Document {
        let mut document = Document::new(text.to_owned());
        let selection = document.capture_selection(range).unwrap();
        document.add_comment(selection, "review".into()).unwrap();
        document
    }

    #[test]
    fn edit_before_shifts_anchor_and_edit_after_preserves_it() {
        let mut document = anchored("alpha beta gamma", 6..10);
        document.replace_text("X alpha beta gamma".into());
        assert_eq!(document.comments()[0].anchor, Anchor::Resolved(8..12));
        document.replace_text("X alpha beta gamma!".into());
        assert_eq!(document.comments()[0].anchor, Anchor::Resolved(8..12));
    }

    #[test]
    fn insertion_boundaries_stay_outside_quote() {
        let mut document = anchored("one beta two", 4..8);
        document.replace_text("one Xbeta two".into());
        assert_eq!(document.comments()[0].anchor, Anchor::Resolved(5..9));
        document.replace_text("one XbetaY two".into());
        assert_eq!(document.comments()[0].anchor, Anchor::Resolved(5..9));
    }

    #[test]
    fn overlap_and_deletion_invalidate_anchor() {
        let mut document = anchored("one beta two", 4..8);
        document.replace_text("one bEta two".into());
        assert!(matches!(
            document.comments()[0].anchor,
            Anchor::Unresolved(_)
        ));
        let mut document = anchored("one beta two", 4..8);
        document.replace_text("one two".into());
        assert!(matches!(
            document.comments()[0].anchor,
            Anchor::Unresolved(_)
        ));
    }

    #[test]
    fn duplicate_quote_never_reattaches() {
        let mut document = anchored("beta one", 0..4);
        document.replace_text("beta one beta".into());
        assert!(matches!(
            document.comments()[0].anchor,
            Anchor::Unresolved(_)
        ));
    }

    #[test]
    fn duplicate_quote_is_anchored_at_capture_but_unresolved_after_edit() {
        let mut document = anchored("beta one beta", 0..4);
        assert_eq!(document.comments()[0].anchor, Anchor::Resolved(0..4));
        document.replace_text("beta one beta!".into());
        assert!(matches!(
            document.comments()[0].anchor,
            Anchor::Unresolved(_)
        ));
    }

    #[test]
    fn cli_rejects_non_regular_file() {
        assert!(load_document_from_args(["/dev/null"]).is_err());
    }

    #[test]
    fn invalid_utf8_boundaries_and_stale_capture_are_rejected() {
        let mut document = Document::new("é beta".into());
        assert!(document.capture_selection(1..2).is_err());
        let selection = document.capture_selection(3..7).unwrap();
        document.replace_text("é betas".into());
        assert!(document.add_comment(selection, "note".into()).is_err());
    }

    #[test]
    fn exact_full_buffer_restore_reattaches_original_and_redo_invalidates() {
        let original = "one beta two";
        let mut document = anchored(original, 4..8);
        document.replace_text("one bEta two".into());
        document.replace_text(original.into());
        assert_eq!(document.comments()[0].anchor, Anchor::Resolved(4..8));
        document.replace_text("one bEta two".into());
        assert!(matches!(
            document.comments()[0].anchor,
            Anchor::Unresolved(_)
        ));
    }

    #[test]
    fn cli_rejects_extra_arguments_and_invalid_utf8() {
        assert!(load_document_from_args(["file.md", "extra.md"]).is_err());
        let path = std::env::temp_dir().join(format!("brn-invalid-{}.md", std::process::id()));
        std::fs::write(&path, [0xff]).unwrap();
        assert!(load_document_from_args([path.to_str().unwrap()]).is_err());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn cli_loads_supplied_file_and_rejects_oversized_input() {
        let path = std::env::temp_dir().join(format!("brn-source-{}.md", std::process::id()));
        std::fs::write(&path, "# hello\n").unwrap();
        assert_eq!(
            load_document_from_args([path.to_str().unwrap()]).unwrap().1,
            "# hello\n"
        );
        std::fs::write(&path, vec![b'a'; MAX_DOCUMENT_BYTES + 1]).unwrap();
        assert!(load_document_from_args([path.to_str().unwrap()]).is_err());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn diff_preserves_newline_only_changes() {
        let mut document = Document::new("first\nsecond\n".into());
        document.replace_text("first\nsecond".into());
        let diff = document.unified_diff();
        assert!(
            diff.contains("\\ No newline at end of file") || diff.contains("-second\n+second"),
            "{diff}"
        );
    }
}
