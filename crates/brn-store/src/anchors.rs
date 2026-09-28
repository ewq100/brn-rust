//! Exact UTF-8 anchor mapping. Ambiguity is sticky except for an exact immutable snapshot.
use crate::{Result, hash, invalid};
use serde::{Deserialize, Serialize};
use std::ops::Range;

pub const MAX_EDIT_STEPS: usize = 4096;
pub const MAX_TRACE_REPLACEMENT_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_TEXT_BYTES: usize = crate::MAX_DRAFT_BYTES;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEdit {
    pub start: usize,
    pub end: usize,
    pub replacement: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditTrace {
    Steps(Vec<TextEdit>),
    HistoryLost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AmbiguityReason {
    Touched,
    Duplicate,
    MissingUnsupported,
    BoundaryAmbiguity,
    ConflictingSnapshot,
    HistoryLimit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnchorState {
    Anchored { start: usize, end: usize },
    Deleted,
    Ambiguous { reason: AmbiguityReason },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalAnchor {
    pub text: String,
    pub sha256: [u8; 32],
    pub range: Range<usize>,
    pub quote: String,
}

impl OriginalAnchor {
    pub fn new(text: &str, range: Range<usize>, quote: &str) -> Result<Self> {
        validate_range(text, range.start, range.end, quote)?;
        Ok(Self {
            text: text.into(),
            sha256: hash(text.as_bytes()),
            range,
            quote: quote.into(),
        })
    }

    fn validate(&self) -> Result<()> {
        if self.text.len() > MAX_TEXT_BYTES || hash(self.text.as_bytes()) != self.sha256 {
            return Err(invalid("invalid original anchor content"));
        }
        validate_range(&self.text, self.range.start, self.range.end, &self.quote)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryReference {
    pub text: String,
    pub sha256: [u8; 32],
    pub state: AnchorState,
}

impl RecoveryReference {
    pub fn new(text: &str, state: AnchorState) -> Self {
        Self {
            text: text.into(),
            sha256: hash(text.as_bytes()),
            state,
        }
    }

    fn validate(&self, quote: &str) -> Result<()> {
        if self.text.len() > MAX_TEXT_BYTES || hash(self.text.as_bytes()) != self.sha256 {
            return Err(invalid("invalid checkpoint recovery content"));
        }
        if let AnchorState::Anchored { start, end } = self.state {
            validate_range(&self.text, start, end, quote)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnchorProjection {
    pub state: AnchorState,
    pub original: OriginalAnchor,
    pub checkpoints: Vec<RecoveryReference>,
}

fn validate_range(text: &str, start: usize, end: usize, quote: &str) -> Result<()> {
    if quote.is_empty()
        || start >= end
        || end > text.len()
        || !text.is_char_boundary(start)
        || !text.is_char_boundary(end)
        || text.get(start..end) != Some(quote)
    {
        return Err(invalid("anchor range does not match exact quote"));
    }
    Ok(())
}

/// Longest shared UTF-8-character prefix and non-overlapping suffix.
pub fn derive_edit(before: &str, after: &str) -> Option<TextEdit> {
    if before == after {
        return None;
    }
    let mut start = 0;
    let mut b = before.chars();
    let mut a = after.chars();
    loop {
        match (b.next(), a.next()) {
            (Some(x), Some(y)) if x == y => start += x.len_utf8(),
            _ => break,
        }
    }
    let (mut end, mut after_end) = (before.len(), after.len());
    while end > start && after_end > start {
        let x = before[..end].chars().next_back().unwrap();
        let y = after[..after_end].chars().next_back().unwrap();
        if x != y {
            break;
        }
        end -= x.len_utf8();
        after_end -= y.len_utf8();
    }
    Some(TextEdit {
        start,
        end,
        replacement: after[start..after_end].into(),
    })
}

/// Apply a recorded edit to an intermediate text, with exact byte and size checks.
pub fn apply_edit(before: &str, edit: &TextEdit) -> Result<String> {
    if edit.start > edit.end
        || edit.end > before.len()
        || !before.is_char_boundary(edit.start)
        || !before.is_char_boundary(edit.end)
    {
        return Err(invalid("invalid edit byte range"));
    }
    let len = before
        .len()
        .checked_sub(edit.end - edit.start)
        .and_then(|n| n.checked_add(edit.replacement.len()))
        .ok_or_else(|| invalid("edit size overflow"))?;
    if len > MAX_TEXT_BYTES {
        return Err(invalid("intermediate draft exceeds limit"));
    }
    let mut after = String::with_capacity(len);
    after.push_str(&before[..edit.start]);
    after.push_str(&edit.replacement);
    after.push_str(&before[edit.end..]);
    Ok(after)
}

fn occurrence_count(text: &str, quote: &str) -> usize {
    if quote.is_empty() {
        return 0;
    }
    let mut count = 0;
    let mut at = 0;
    while at <= text.len() {
        let Some(offset) = text[at..].find(quote) else {
            break;
        };
        let found = at + offset;
        count += 1;
        if count == 2 {
            break;
        }
        let next = text[found..].chars().next().unwrap().len_utf8();
        at = found + next;
    }
    count
}

fn shift(
    start: usize,
    end: usize,
    edit_start: usize,
    edit_end: usize,
    added: usize,
) -> Result<AnchorState> {
    let removed = edit_end - edit_start;
    let before = if edit_start == edit_end {
        edit_start <= start
    } else {
        edit_end <= start
    };
    if before {
        let moved = |n: usize| {
            n.checked_sub(removed)
                .and_then(|v| v.checked_add(added))
                .ok_or_else(|| invalid("anchor offset overflow"))
        };
        return Ok(AnchorState::Anchored {
            start: moved(start)?,
            end: moved(end)?,
        });
    }
    if edit_start >= end {
        return Ok(AnchorState::Anchored { start, end });
    }
    Ok(AnchorState::Ambiguous {
        reason: AmbiguityReason::Touched,
    })
}

/// Map one canonical replacement. Equivalent placements must agree on identity.
pub fn map_anchor(
    before: &str,
    after: &str,
    state: &AnchorState,
    original_quote: &str,
    edit: &TextEdit,
) -> Result<AnchorState> {
    if apply_edit(before, edit)? != after || derive_edit(before, after).as_ref() != Some(edit) {
        return Err(invalid("edit is not the canonical replacement"));
    }
    let AnchorState::Anchored { start, end } = *state else {
        return Ok(state.clone());
    };
    validate_range(before, start, end, original_quote)?;
    if occurrence_count(before, original_quote) != 1 || occurrence_count(after, original_quote) > 1
    {
        return Ok(AnchorState::Ambiguous {
            reason: AmbiguityReason::Duplicate,
        });
    }
    let no_quote_after = occurrence_count(after, original_quote) == 0;
    let removed = edit.end - edit.start;
    let added = edit.replacement.len();
    let mut common_suffix = 0;
    let mut b = before.chars().rev();
    let mut a = after.chars().rev();
    while let (Some(x), Some(y)) = (b.next(), a.next()) {
        if x != y {
            break;
        }
        common_suffix += x.len_utf8();
    }
    let common_prefix = edit.start;
    let min_start = before
        .len()
        .saturating_sub(common_suffix)
        .saturating_sub(removed);
    let mut outcome: Option<AnchorState> = None;
    for candidate_start in min_start..=common_prefix {
        let Some(candidate_end) = candidate_start.checked_add(removed) else {
            continue;
        };
        let Some(after_end) = candidate_start.checked_add(added) else {
            continue;
        };
        if candidate_end > before.len()
            || after_end > after.len()
            || !before.is_char_boundary(candidate_start)
            || !before.is_char_boundary(candidate_end)
            || !after.is_char_boundary(candidate_start)
            || !after.is_char_boundary(after_end)
        {
            continue;
        }
        // The common prefix/suffix lengths already establish equality for
        // every candidate, avoiding quadratic scans over repeated text.
        let mapped =
            if added == 0 && candidate_start <= start && candidate_end >= end && no_quote_after {
                AnchorState::Deleted
            } else {
                shift(start, end, candidate_start, candidate_end, added)?
            };
        if let Some(old) = &outcome {
            if old != &mapped {
                return Ok(AnchorState::Ambiguous {
                    reason: AmbiguityReason::BoundaryAmbiguity,
                });
            }
        } else {
            outcome = Some(mapped);
        }
    }
    let mapped = outcome.ok_or_else(|| invalid("no valid edit alignment"))?;
    if let AnchorState::Anchored { start, end } = mapped {
        if after.get(start..end) == Some(original_quote) {
            return Ok(mapped);
        }
        return Ok(AnchorState::Ambiguous {
            reason: AmbiguityReason::Touched,
        });
    }
    Ok(mapped)
}

fn recover(text: &str, projection: &AnchorProjection, state: AnchorState) -> AnchorState {
    if text == projection.original.text {
        return AnchorState::Anchored {
            start: projection.original.range.start,
            end: projection.original.range.end,
        };
    }
    let mut candidate = None;
    for reference in &projection.checkpoints {
        if reference.text == text {
            if let AnchorState::Anchored { .. } = reference.state {
                if let Some(old) = &candidate {
                    if old != &reference.state {
                        return AnchorState::Ambiguous {
                            reason: AmbiguityReason::ConflictingSnapshot,
                        };
                    }
                } else {
                    candidate = Some(reference.state.clone());
                }
            }
        }
    }
    candidate.unwrap_or(state)
}

/// Replay the exact edit history from the acknowledged text, then recover only at immutable snapshots.
pub fn replay_trace(
    initial: &str,
    trace: &EditTrace,
    final_text: &str,
    projections: &[AnchorProjection],
) -> Result<Vec<AnchorState>> {
    if initial.len() > MAX_TEXT_BYTES || final_text.len() > MAX_TEXT_BYTES {
        return Err(invalid("draft exceeds limit"));
    }
    for projection in projections {
        projection.original.validate()?;
        for reference in &projection.checkpoints {
            reference.validate(&projection.original.quote)?;
        }
        if let AnchorState::Anchored { start, end } = projection.state {
            validate_range(initial, start, end, &projection.original.quote)?;
        }
    }
    let mut states: Vec<_> = projections.iter().map(|p| p.state.clone()).collect();
    match trace {
        EditTrace::HistoryLost => {
            for state in &mut states {
                *state = AnchorState::Ambiguous {
                    reason: AmbiguityReason::HistoryLimit,
                };
            }
        }
        EditTrace::Steps(steps) => {
            if steps.len() > MAX_EDIT_STEPS {
                return Err(invalid("edit trace step limit exceeded"));
            }
            let mut replacement_bytes = 0usize;
            let mut current = initial.to_owned();
            for edit in steps {
                replacement_bytes = replacement_bytes
                    .checked_add(edit.replacement.len())
                    .ok_or_else(|| invalid("edit trace size overflow"))?;
                if replacement_bytes > MAX_TRACE_REPLACEMENT_BYTES {
                    return Err(invalid("edit trace byte limit exceeded"));
                }
                let next = apply_edit(&current, edit)?;
                if derive_edit(&current, &next).as_ref() != Some(edit) {
                    return Err(invalid("noncanonical edit trace step"));
                }
                for (state, projection) in states.iter_mut().zip(projections) {
                    *state = map_anchor(&current, &next, state, &projection.original.quote, edit)?;
                    *state = recover(&next, projection, state.clone());
                }
                current = next;
            }
            if current != final_text {
                return Err(invalid("edit trace does not reconstruct submitted text"));
            }
        }
    }
    for (state, projection) in states.iter_mut().zip(projections) {
        *state = recover(final_text, projection, state.clone());
    }
    Ok(states)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anchored(start: usize, end: usize) -> AnchorState {
        AnchorState::Anchored { start, end }
    }

    fn mapped(before: &str, after: &str, state: AnchorState, quote: &str) -> AnchorState {
        let edit = derive_edit(before, after).expect("changed text");
        map_anchor(before, after, &state, quote, &edit).unwrap()
    }

    #[test]
    fn before_after_and_boundary_insertions_preserve_exact_range() {
        assert_eq!(
            mapped("one TWO tail", "z one TWO tail", anchored(4, 7), "TWO"),
            anchored(6, 9)
        );
        assert_eq!(
            mapped("one TWO tail", "one TWO tail!", anchored(4, 7), "TWO"),
            anchored(4, 7)
        );
        assert_eq!(
            mapped("a TWO b", "a xTWO b", anchored(2, 5), "TWO"),
            anchored(3, 6)
        );
        assert_eq!(
            mapped("a TWO b", "a TWOx b", anchored(2, 5), "TWO"),
            anchored(2, 5)
        );
        assert_eq!(
            mapped("xx TWO yy", "TWO yy", anchored(3, 6), "TWO"),
            anchored(0, 3)
        );
        assert_eq!(
            mapped("xx TWO yy", "xx TWO", anchored(3, 6), "TWO"),
            anchored(3, 6)
        );
    }

    #[test]
    fn overlap_and_complete_deletion_are_distinct() {
        assert_eq!(
            mapped("a TWO b", "a TW b", anchored(2, 5), "TWO"),
            AnchorState::Ambiguous {
                reason: AmbiguityReason::Touched
            }
        );
        assert_eq!(
            mapped("a TWO b", "a  b", anchored(2, 5), "TWO"),
            AnchorState::Deleted
        );
        assert_eq!(
            mapped("a TWO b", "a X b", anchored(2, 5), "TWO"),
            AnchorState::Ambiguous {
                reason: AmbiguityReason::Touched
            }
        );
        assert_eq!(
            mapped("a TWO b", "a XWO b", anchored(2, 5), "TWO"),
            AnchorState::Ambiguous {
                reason: AmbiguityReason::Touched
            }
        );
    }

    #[test]
    fn duplicate_quotes_and_overlapping_occurrences_never_choose_identity() {
        assert_eq!(
            mapped("TWO and TWO", "z TWO and TWO", anchored(0, 3), "TWO"),
            AnchorState::Ambiguous {
                reason: AmbiguityReason::Duplicate
            }
        );
        assert_eq!(
            mapped("TWO tail", "TWO tail TWO", anchored(0, 3), "TWO"),
            AnchorState::Ambiguous {
                reason: AmbiguityReason::Duplicate
            }
        );
        assert_eq!(
            mapped("aaaa", "aaaa!", anchored(0, 3), "aaa"),
            AnchorState::Ambiguous {
                reason: AmbiguityReason::Duplicate
            }
        );
    }

    #[test]
    fn repeated_character_alignment_never_chooses_a_boundary() {
        assert_eq!(
            mapped("aaabc", "aaaabc", anchored(2, 5), "abc"),
            AnchorState::Ambiguous {
                reason: AmbiguityReason::BoundaryAmbiguity
            }
        );
        assert_eq!(
            mapped("aaabc", "aabc", anchored(2, 5), "abc"),
            AnchorState::Ambiguous {
                reason: AmbiguityReason::BoundaryAmbiguity
            }
        );
    }

    #[test]
    fn unicode_crlf_and_final_newline_remain_exact_bytes() {
        assert_eq!(
            mapped(
                "🙂e\u{301}\r\n猫\n",
                "é🙂e\u{301}\r\n猫\n",
                anchored(9, 12),
                "猫"
            ),
            anchored(11, 14)
        );
        assert_eq!(
            mapped(
                "🙂e\u{301}\r\n猫\n",
                "🙂e\u{301}\r\n猫\n!",
                anchored(9, 12),
                "猫"
            ),
            anchored(9, 12)
        );
        assert_eq!(
            derive_edit("a\r\n", "a\n"),
            Some(TextEdit {
                start: 1,
                end: 2,
                replacement: String::new()
            })
        );
    }

    #[test]
    fn broad_multi_region_edit_and_invalid_ranges_fail_conservatively() {
        assert_eq!(
            mapped("A TWO middle B", "X TWO middle Y", anchored(2, 5), "TWO"),
            AnchorState::Ambiguous {
                reason: AmbiguityReason::Touched
            }
        );
        assert_eq!(derive_edit("same", "same"), None);
        assert!(
            map_anchor(
                "a🙂b",
                "a🙂b!",
                &anchored(2, 5),
                "🙂",
                &TextEdit {
                    start: 6,
                    end: 6,
                    replacement: "!".into()
                }
            )
            .is_err()
        );
        assert!(
            map_anchor(
                "abc",
                "axc",
                &anchored(0, 1),
                "a",
                &TextEdit {
                    start: 2,
                    end: 2,
                    replacement: "x".into()
                }
            )
            .is_err()
        );
    }

    #[test]
    fn trace_delete_reinsert_recovers_exact_original_snapshot() {
        let original = OriginalAnchor::new("a TWO b", 2..5, "TWO").unwrap();
        let projection = AnchorProjection {
            state: anchored(2, 5),
            original,
            checkpoints: vec![],
        };
        let trace = EditTrace::Steps(vec![
            TextEdit {
                start: 2,
                end: 5,
                replacement: String::new(),
            },
            TextEdit {
                start: 2,
                end: 2,
                replacement: "TWO".into(),
            },
        ]);
        assert_eq!(
            replay_trace("a TWO b", &trace, "a TWO b", &[projection]).unwrap(),
            vec![anchored(2, 5)]
        );
    }

    #[test]
    fn history_lost_recovers_only_exact_original_or_checkpoint() {
        let original = OriginalAnchor::new("a TWO b", 2..5, "TWO").unwrap();
        let checkpoint = RecoveryReference::new("x TWO b", anchored(2, 5));
        let projection = AnchorProjection {
            state: anchored(2, 5),
            original,
            checkpoints: vec![checkpoint],
        };
        assert_eq!(
            replay_trace(
                "a TWO b",
                &EditTrace::HistoryLost,
                "x TWO b",
                &[projection.clone()]
            )
            .unwrap(),
            vec![anchored(2, 5)]
        );
        assert_eq!(
            replay_trace("a TWO b", &EditTrace::HistoryLost, "z TWO b", &[projection]).unwrap(),
            vec![AnchorState::Ambiguous {
                reason: AmbiguityReason::HistoryLimit
            }]
        );
    }

    #[test]
    fn delete_then_reinsert_in_different_document_does_not_reattach() {
        let projection = AnchorProjection {
            state: anchored(2, 5),
            original: OriginalAnchor::new("a TWO b", 2..5, "TWO").unwrap(),
            checkpoints: vec![],
        };
        let trace = EditTrace::Steps(vec![
            TextEdit {
                start: 2,
                end: 5,
                replacement: String::new(),
            },
            TextEdit {
                start: 0,
                end: 1,
                replacement: "z".into(),
            },
            TextEdit {
                start: 2,
                end: 2,
                replacement: "TWO".into(),
            },
        ]);
        assert_eq!(
            replay_trace("a TWO b", &trace, "z TWO b", &[projection]).unwrap(),
            vec![AnchorState::Deleted]
        );
    }

    #[test]
    fn replay_noop_and_exact_checkpoint_recover_but_conflict_stays_ambiguous() {
        let original = OriginalAnchor::new("a TWO b", 2..5, "TWO").unwrap();
        let projection = AnchorProjection {
            state: anchored(2, 5),
            original: original.clone(),
            checkpoints: vec![],
        };
        assert_eq!(
            replay_trace(
                "a TWO b",
                &EditTrace::Steps(vec![]),
                "a TWO b",
                &[projection]
            )
            .unwrap(),
            vec![anchored(2, 5)]
        );
        let checkpoint = "TWO x TWO";
        let projection = AnchorProjection {
            state: AnchorState::Deleted,
            original,
            checkpoints: vec![
                RecoveryReference::new(checkpoint, anchored(0, 3)),
                RecoveryReference::new(checkpoint, anchored(6, 9)),
            ],
        };
        assert_eq!(
            replay_trace("a  b", &EditTrace::HistoryLost, checkpoint, &[projection]).unwrap(),
            vec![AnchorState::Ambiguous {
                reason: AmbiguityReason::ConflictingSnapshot
            }]
        );
    }

    #[test]
    fn malformed_trace_and_limits_reject_without_partial_mapping() {
        let original = OriginalAnchor::new("a TWO b", 2..5, "TWO").unwrap();
        let projection = AnchorProjection {
            state: anchored(2, 5),
            original,
            checkpoints: vec![],
        };
        assert!(
            replay_trace(
                "a TWO b",
                &EditTrace::Steps(vec![TextEdit {
                    start: 0,
                    end: 1,
                    replacement: "z".into()
                }]),
                "wrong",
                &[projection.clone()]
            )
            .is_err()
        );
        assert!(
            replay_trace(
                "a TWO b",
                &EditTrace::Steps(vec![TextEdit {
                    start: 1,
                    end: 1,
                    replacement: "x".repeat(MAX_TEXT_BYTES)
                }]),
                "x",
                &[projection.clone()]
            )
            .is_err()
        );
        assert!(
            replay_trace(
                "a TWO b",
                &EditTrace::Steps(vec![
                    TextEdit {
                        start: 0,
                        end: 0,
                        replacement: String::new()
                    };
                    MAX_EDIT_STEPS + 1
                ]),
                "a TWO b",
                &[projection.clone()]
            )
            .is_err()
        );
        assert!(
            replay_trace(
                "a TWO b",
                &EditTrace::Steps(vec![TextEdit {
                    start: 0,
                    end: 0,
                    replacement: "x".repeat(MAX_TRACE_REPLACEMENT_BYTES + 1)
                }]),
                "a TWO b",
                &[projection.clone()]
            )
            .is_err()
        );
        let mut tampered = projection;
        tampered.original.sha256 = [0; 32];
        assert!(replay_trace("a TWO b", &EditTrace::HistoryLost, "a TWO b", &[tampered]).is_err());
    }
}
