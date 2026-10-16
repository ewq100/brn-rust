//! Document toolbar and comment placement values; shell Layout remains host-owned.
/// Where a Note's comments sit: beside the text at a fixed margin width, or
/// docked below it when the Note has no room.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarginMode {
    Wide,
    Narrow,
    Stacked,
}
impl MarginMode {
    pub const WIDE: f32 = 280.;
    pub const NARROW: f32 = 220.;
    /// Width the margin adds to the Note's minimum.
    pub fn width(self) -> f32 {
        match self {
            Self::Wide => Self::WIDE,
            Self::Narrow => Self::NARROW,
            Self::Stacked => 0.,
        }
    }
}

/// The width each Note toolbar arrangement needs on one row, padding and
/// borders included. The renderer measures them for the open note from its
/// labels and its status chip and comment count (`note_toolbar_needs`), so
/// Edit note and Connections fold only when this note's row doesn't fit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoteNeeds {
    pub full: f32,
    pub format_menu: f32,
    pub compact: f32,
}
impl NoteNeeds {
    /// The widest row in the test text system, a "Protected · v12" chip and
    /// a 12-comment toggle (797, 752 and 543 pt), plus about 12 pt.
    pub const WIDEST: Self = Self {
        full: NoteFit::FULL,
        format_menu: NoteFit::FORMAT_MENU,
        compact: NoteFit::COMPACT,
    };
}

/// How much of the Note toolbar fits on its one row, by the Note panel's
/// width and what the open note's row needs (`NoteNeeds`). Icons needs
/// 315 pt, under the Note's 360 pt minimum.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum NoteFit {
    /// Format and Outline are icon triggers, and the status chip shows its
    /// icon and version.
    Icons,
    /// Edit note and Connections move into the Note ⋯.
    Compact,
    /// The five formatting buttons fold into Format ▾.
    FormatMenu,
    #[default]
    Full,
}
impl NoteFit {
    pub const FULL: f32 = 810.;
    pub const FORMAT_MENU: f32 = 765.;
    pub const COMPACT: f32 = 555.;
    pub fn for_width(width: f32, needs: NoteNeeds) -> Self {
        if width >= needs.full {
            Self::Full
        } else if width >= needs.format_menu {
            Self::FormatMenu
        } else if width >= needs.compact {
            Self::Compact
        } else {
            Self::Icons
        }
    }
    pub fn format_menu(self) -> bool {
        self < Self::Full
    }
    pub fn actions_in_menu(self) -> bool {
        self < Self::FormatMenu
    }
    pub fn icons(self) -> bool {
        self == Self::Icons
    }
}

/// How much of the review bar fits on its one row, by the Note panel's
/// width. Each threshold is the measured width its arrangement needs on a
/// protected whole-note review with a 12-comment toggle (915 and 658 pt,
/// padding and borders included) plus a few points for wider counts; the
/// icon arrangement needs 405 pt, so 900×600 Write (Note 406 pt) keeps one
/// row. The exact-text footer below the review never folds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReviewFit {
    /// The primary reads Apply or Approve, the Proposal label drops its
    /// shape and the Protected chip shows only its lock.
    Icons,
    /// Keep for later and Dismiss move into the Note ⋯, and the +a −r counts
    /// into the Proposal label's tooltip.
    Compact,
    #[default]
    Full,
}
impl ReviewFit {
    pub const FULL: f32 = 920.;
    pub const COMPACT: f32 = 670.;
    pub fn for_width(width: f32) -> Self {
        if width >= Self::FULL {
            Self::Full
        } else if width >= Self::COMPACT {
            Self::Compact
        } else {
            Self::Icons
        }
    }
    pub fn actions_in_menu(self) -> bool {
        self < Self::Full
    }
    pub fn icons(self) -> bool {
        self == Self::Icons
    }
}

use brn_threads_app::Comment;
use std::ops::Range;
/// Why a comment has no place in the shown version, worded for what is
/// true: the service unresolved it (its passage was edited away), or it is
/// still tied to the earlier version it was written on and nothing
/// re-anchored it. `status` is the card's line; `reason` follows
/// "unavailable:" in labels and tooltips.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unlocated {
    pub status: String,
    pub reason: String,
}
pub fn unlocated_reason(comment: &Comment, version: u64) -> Unlocated {
    let tied = comment.mapped_version.unwrap_or(comment.base_version);
    let next = "Exclude it, or select the passage and comment again.";
    if !comment.unresolved && tied != version {
        Unlocated {
            status: format!("Tied to v{tied}, not re-anchored to v{version}. {next}"),
            reason: format!("the comment is tied to v{tied}, not v{version}"),
        }
    } else {
        Unlocated {
            status: format!("Passage not found in v{version}. {next}"),
            reason: format!("the passage is not in v{version}"),
        }
    }
}
/// Only an exact supported mapping supplies a current location. Repeated quotes
/// and changed revisions never trigger a guessed relocation.
pub fn comment_location(comment: &Comment, version: u64, text: &str) -> Option<Range<usize>> {
    brn_threads_app::exact_writing_range(comment, version, text).map(|(start, end)| start..end)
}
/// Navigation only, never a second source-of-truth representation of Markdown.
pub fn sections(text: &str) -> Vec<(String, Range<usize>)> {
    let mut offset = 0;
    let mut fence: Option<(char, usize)> = None;
    let mut out = Vec::new();
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        let marker = trimmed.chars().next().unwrap_or(' ');
        let run = trimmed.chars().take_while(|c| *c == marker).count();
        if (marker == '`' || marker == '~') && run >= 3 {
            if fence.is_some_and(|(c, n)| c == marker && run >= n) {
                fence = None;
            } else if fence.is_none() {
                fence = Some((marker, run));
            }
        } else if fence.is_none() {
            let hashes = line.chars().take_while(|c| *c == '#').count();
            if (1..=6).contains(&hashes) && line.as_bytes().get(hashes) == Some(&b' ') {
                let title = line[hashes + 1..]
                    .trim()
                    .trim_end_matches('#')
                    .trim()
                    .to_owned();
                if !title.is_empty() {
                    out.push((title, offset + hashes + 1..offset + line.trim_end().len()));
                }
            }
        }
        offset += line.len();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn note_toolbar_folds_in_order_as_the_note_narrows() {
        for (width, fit) in [
            (1200., NoteFit::Full),
            (NoteFit::FULL, NoteFit::Full),
            (NoteFit::FULL - 1., NoteFit::FormatMenu),
            (NoteFit::FORMAT_MENU, NoteFit::FormatMenu),
            (NoteFit::FORMAT_MENU - 1., NoteFit::Compact),
            (NoteFit::COMPACT, NoteFit::Compact),
            (NoteFit::COMPACT - 1., NoteFit::Icons),
            (360., NoteFit::Icons),
        ] {
            assert_eq!(NoteFit::for_width(width, NoteNeeds::WIDEST), fit, "{width}");
        }
        // A plainer note's narrower row keeps Edit note and Connections at
        // a width where the widest row folds them.
        let plain = NoteNeeds {
            full: 740.,
            format_menu: 690.,
            compact: 500.,
        };
        assert_eq!(NoteFit::for_width(700., plain), NoteFit::FormatMenu);
        assert_eq!(
            NoteFit::for_width(700., NoteNeeds::WIDEST),
            NoteFit::Compact
        );
        assert!(!NoteFit::Full.format_menu());
        assert!(NoteFit::FormatMenu.format_menu() && !NoteFit::FormatMenu.actions_in_menu());
        assert!(NoteFit::Compact.actions_in_menu() && !NoteFit::Compact.icons());
        assert!(NoteFit::Icons.icons() && NoteFit::Icons.actions_in_menu());
    }
    #[test]
    fn review_bar_folds_in_order_as_the_note_narrows() {
        for (width, fit) in [
            (1200., ReviewFit::Full),
            (ReviewFit::FULL, ReviewFit::Full),
            (ReviewFit::FULL - 1., ReviewFit::Compact),
            (ReviewFit::COMPACT, ReviewFit::Compact),
            (ReviewFit::COMPACT - 1., ReviewFit::Icons),
            (360., ReviewFit::Icons),
        ] {
            assert_eq!(ReviewFit::for_width(width), fit, "{width}");
        }
        assert!(!ReviewFit::Full.actions_in_menu());
        assert!(ReviewFit::Compact.actions_in_menu() && !ReviewFit::Compact.icons());
        assert!(ReviewFit::Icons.icons() && ReviewFit::Icons.actions_in_menu());
    }
}
