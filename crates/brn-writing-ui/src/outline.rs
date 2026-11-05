//! Article outline: the headings of the displayed document, observed in the
//! reading view's own Markdown parse and bound to one document generation.
//!
//! No second parser and no text scan. The reading `TextView` already turns
//! the document's mdast into its rendered blocks; a block parser registered
//! first in its `MarkdownExtensions` sees every block node of that very parse
//! (and declines it, so rendering is unchanged). Each parse is recorded with
//! a copy of the source it parsed. An outline is only offered when a complete
//! recorded parse has exactly the source the view has committed, which is in
//! turn the source the panel last handed it; anything else is "updating".
//! Targets carry the document identity, its content generation (committed
//! version or open edit buffer) and a hash of the parsed text, so a target
//! from an earlier generation is refused instead of guessing a section.
use gpui_kit::base::{MarkdownExtensions, MarkdownParseContext, RenderedText, markdown_ast::Node};
use std::{
    collections::VecDeque,
    hash::{DefaultHasher, Hash, Hasher},
    ops::Range,
    sync::{Arc, Mutex, MutexGuard},
};

/// The content generation an outline describes.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Generation {
    /// The committed note at this version.
    Committed { version: u64 },
    /// The owner's open edit buffer at this buffer generation.
    Buffer { session: String, generation: u64 },
}

/// What one outline was built from: document, generation and exact text.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Stamp {
    pub document: String,
    pub generation: Generation,
    pub source_hash: u64,
}
impl Stamp {
    pub fn new(document: impl Into<String>, generation: Generation, source: &str) -> Self {
        let mut hasher = DefaultHasher::new();
        source.hash(&mut hasher);
        Self {
            document: document.into(),
            generation,
            source_hash: hasher.finish(),
        }
    }
}

/// One heading of the displayed document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineEntry {
    /// Heading level 1–6, as written.
    pub level: u8,
    /// The heading's rendered text (Markdown emphasis resolved).
    pub label: String,
    /// The heading's source byte range, markers included.
    pub source: Range<usize>,
    /// The heading's range in the view's rendered text.
    pub rendered: Range<usize>,
    /// The top-level block holding the heading (the reading list's item).
    pub block: usize,
    /// Inside a list or quote rather than a top-level block.
    pub nested: bool,
}

/// A request to go to one entry of one outline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineTarget {
    pub stamp: Stamp,
    pub index: usize,
    pub source: Range<usize>,
}

/// The outline of the displayed document, or why there is none yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutlineState {
    Ready {
        stamp: Stamp,
        entries: Vec<OutlineEntry>,
    },
    /// The view has not yet committed a parse of the current text.
    Updating,
}
impl OutlineState {
    pub fn entries(&self) -> &[OutlineEntry] {
        match self {
            Self::Ready { entries, .. } => entries,
            Self::Updating => &[],
        }
    }
    /// The entry `target` names, only while this outline is the one it was
    /// taken from and the entry still has the same source range.
    pub fn resolve(&self, target: &OutlineTarget) -> Option<&OutlineEntry> {
        let Self::Ready { stamp, entries } = self else {
            return None;
        };
        (stamp == &target.stamp)
            .then(|| entries.get(target.index))
            .flatten()
            .filter(|entry| entry.source == target.source)
    }
    pub fn target(&self, index: usize) -> Option<OutlineTarget> {
        let Self::Ready { stamp, entries } = self else {
            return None;
        };
        entries.get(index).map(|entry| OutlineTarget {
            stamp: stamp.clone(),
            index,
            source: entry.source.clone(),
        })
    }
}

/// The section the reader is in: the last heading whose block (`blocks`,
/// one per entry) starts at or before the first visible block.
pub fn current_section(blocks: &[usize], first_visible_block: usize) -> Option<usize> {
    blocks
        .iter()
        .rposition(|block| *block <= first_visible_block)
        .or_else(|| (!blocks.is_empty()).then_some(0))
}

#[derive(Debug)]
struct RawHeading {
    level: u8,
    span: Range<usize>,
    block: usize,
    nested: bool,
    text: String,
}

/// One parse the view ran, in parser call order.
struct Capture {
    /// The parsed text's address, length and document offset: one parse
    /// passes the same `&str` to every block.
    key: (usize, usize, usize),
    offset: usize,
    source: String,
    blocks: usize,
    last_top: Option<Range<usize>>,
    top_end: usize,
    headings: Vec<RawHeading>,
    /// Every non-blank byte of the source belongs to a block already seen.
    complete: bool,
}

/// Records the headings of each parse its reading view runs. Clones share
/// the record; give each `TextViewState` its own collector.
#[derive(Clone, Default)]
pub struct HeadingCollector(Arc<Mutex<VecDeque<Capture>>>);

/// Parses kept for matching; a view commits one of its last few.
const KEPT: usize = 6;

impl HeadingCollector {
    fn lock(&self) -> MutexGuard<'_, VecDeque<Capture>> {
        self.0.lock().unwrap_or_else(|poison| poison.into_inner())
    }

    /// Register the observer ahead of every other block parser, so it sees
    /// each block before a plugin claims it. It always declines.
    pub fn extensions(&self, extensions: MarkdownExtensions) -> MarkdownExtensions {
        let collector = self.clone();
        extensions.block_parser(move |node, cx| {
            collector.observe(node, cx);
            None
        })
    }

    /// Called by the renderer's parser for every block node in preorder:
    /// each top-level block, then its nested blocks.
    pub fn observe(&self, node: &Node, cx: &MarkdownParseContext<'_>) {
        let Some(position) = node.position() else {
            return;
        };
        let (start, end) = (position.start.offset, position.end.offset);
        let source = cx.source();
        let key = (source.as_ptr() as usize, source.len(), cx.offset());
        let within = |capture: &Capture| {
            capture
                .last_top
                .as_ref()
                .is_some_and(|top| start >= top.start && end <= top.end)
        };
        let mut captures = self.lock();
        // A node before the end of the blocks seen so far that is not
        // inside the last one starts another parse of a same-sized text.
        let fresh = captures.back().is_none_or(|capture| {
            capture.key != key || (!within(capture) && start < capture.top_end)
        });
        if fresh {
            captures.push_back(Capture {
                key,
                offset: cx.offset(),
                source: source.to_owned(),
                blocks: 0,
                last_top: None,
                top_end: 0,
                headings: vec![],
                complete: false,
            });
            while captures.len() > KEPT {
                captures.pop_front();
            }
        }
        let Some(capture) = captures.back_mut() else {
            return;
        };
        if within(capture) {
            // A nested block of the top-level block walked below.
            return;
        }
        let block = capture.blocks;
        capture.blocks += 1;
        let offset = capture.offset;
        walk(node, block, false, offset, &mut capture.headings);
        capture.last_top = Some(start..end);
        capture.top_end = end;
        capture.complete = source.get(end..).is_none_or(|rest| rest.trim().is_empty());
    }

    /// The headings of `rendered`, when a complete recorded parse produced
    /// exactly its source; `None` while that parse has not been recorded.
    pub fn entries(&self, rendered: &RenderedText) -> Option<Vec<OutlineEntry>> {
        let source = rendered.source();
        if source.trim().is_empty() {
            return Some(vec![]);
        }
        let captures = self.lock();
        let capture = captures
            .iter()
            .rev()
            .find(|capture| capture.complete && capture.offset == 0 && capture.source == source)?;
        Some(
            capture
                .headings
                .iter()
                .filter_map(|heading| {
                    // A heading that renders no text has nothing to show.
                    let rendered_range = rendered.range_for_source(heading.span.clone())?;
                    let shown = rendered
                        .as_str()
                        .get(rendered_range.clone())
                        .map(one_line)
                        .filter(|label| !label.is_empty())
                        .unwrap_or_else(|| one_line(&heading.text));
                    Some(OutlineEntry {
                        level: heading.level,
                        label: shown,
                        source: heading.span.clone(),
                        rendered: rendered_range,
                        block: heading.block,
                        nested: heading.nested,
                    })
                })
                .collect(),
        )
    }
}

fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn walk(node: &Node, block: usize, nested: bool, offset: usize, out: &mut Vec<RawHeading>) {
    if let Node::Heading(heading) = node {
        if let Some(position) = node.position() {
            out.push(RawHeading {
                level: heading.depth,
                span: offset + position.start.offset..offset + position.end.offset,
                block,
                nested,
                text: node.to_string(),
            });
        }
        return;
    }
    for child in node.children().into_iter().flatten() {
        walk(child, block, true, offset, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(level: u8, block: usize) -> OutlineEntry {
        OutlineEntry {
            level,
            label: format!("H{level}"),
            source: block * 10..block * 10 + 5,
            rendered: 0..1,
            block,
            nested: false,
        }
    }

    #[test]
    fn the_current_section_is_the_last_heading_at_or_above_the_top_block() {
        let blocks = [0, 4, 9];
        assert_eq!(current_section(&blocks, 0), Some(0));
        assert_eq!(current_section(&blocks, 3), Some(0));
        assert_eq!(current_section(&blocks, 4), Some(1));
        assert_eq!(current_section(&blocks, 20), Some(2));
        assert_eq!(current_section(&[], 3), None);
        // Text before the first heading still reads as its section.
        assert_eq!(current_section(&[2], 0), Some(0));
    }

    #[test]
    fn a_target_resolves_only_against_its_own_document_generation_and_range() {
        let stamp = Stamp::new(
            "note-a",
            Generation::Committed { version: 3 },
            "# A\n\n## B\n",
        );
        let state = OutlineState::Ready {
            stamp: stamp.clone(),
            entries: vec![entry(1, 0), entry(2, 1)],
        };
        let target = state.target(1).unwrap();
        assert_eq!(state.resolve(&target).map(|e| e.block), Some(1));
        // Another document, a newer version, an unsaved buffer or different
        // text never resolves an old target.
        for other in [
            Stamp::new(
                "note-b",
                Generation::Committed { version: 3 },
                "# A\n\n## B\n",
            ),
            Stamp::new(
                "note-a",
                Generation::Committed { version: 4 },
                "# A\n\n## B\n",
            ),
            Stamp::new(
                "note-a",
                Generation::Buffer {
                    session: "s".into(),
                    generation: 1,
                },
                "# A\n\n## B\n",
            ),
            Stamp::new(
                "note-a",
                Generation::Committed { version: 3 },
                "# A\n\n## C\n",
            ),
        ] {
            let newer = OutlineState::Ready {
                stamp: other,
                entries: vec![entry(1, 0), entry(2, 1)],
            };
            assert_eq!(newer.resolve(&target), None);
        }
        // A moved entry with the same index is refused too.
        let mut moved = vec![entry(1, 0), entry(2, 1)];
        moved[1].source = 40..45;
        let shifted = OutlineState::Ready {
            stamp,
            entries: moved,
        };
        assert_eq!(shifted.resolve(&target), None);
        assert_eq!(OutlineState::Updating.resolve(&target), None);
        assert!(OutlineState::Updating.target(0).is_none());
    }
}
