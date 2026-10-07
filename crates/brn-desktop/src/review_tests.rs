use super::*;
use brn_workflow::proposals::{
    CommentTarget, NoteChange, ProposalEdit, ProposalRecord, ProposalState,
};
use serde_json::json;
use std::time::{Duration, Instant};
use uuid::Uuid;

pub(crate) fn fixture() -> ProposalRecord {
    // The before/source bytes are empty; this is their actual SHA-256.
    let empty = [
        227, 176, 196, 66, 152, 252, 28, 20, 154, 251, 244, 200, 153, 111, 185, 36, 39, 174, 65,
        228, 100, 155, 147, 76, 164, 149, 153, 27, 120, 82, 184, 85,
    ];
    let parent = json!({"device":1,"inode":1});
    let fingerprint = |inode| json!({"device":1,"inode":inode,"len":0,"sha256":empty});
    let record: ProposalRecord = serde_json::from_value(json!({
        "draft": {
            "id":Uuid::new_v4(), "group_id":Uuid::new_v4(), "session_id":Uuid::new_v4(),
            "vault":{"id":Uuid::new_v4(),"root":"/synthetic/review-vault","identity":parent},
            "title":"\u{feff}日本語 review\r\nλ",
            "changes":[
                {"kind":"create","path":"new.md","parent":parent,"text":"\u{feff}# 日本語\r\nrepeat λ repeat λ\r\n"},
                {"kind":"replace","path":"existing.md","parent":parent,"before":fingerprint(2),"before_text":"","text":"\u{feff}Замена\r\nλ"},
                {"kind":"trash","path":"trash.md","parent":parent,"before":fingerprint(3),"before_text":""}
            ],
            "sources":[{"path":"source.md","fingerprint":fingerprint(99)}]
        },
        "version":2, "state":"draft", "created_at_ms":1, "updated_at_ms":2,
        "comments":[{"id":Uuid::new_v4(),"text":"Review λ\r\n","target":{"kind":"proposal"}}]
    })).unwrap();
    brn_workflow::proposals::validate_review_edit(&record, &full_edit(&record)).unwrap();
    record
}

fn full_edit(record: &ProposalRecord) -> ProposalEdit {
    ProposalEdit {
        action_data: Vec::new(),
        expected: record.stamp(),
        title: record.draft.title.clone(),
        texts: record
            .draft
            .changes
            .iter()
            .map(|change| change.text().map(str::to_owned))
            .collect(),
    }
}

fn reply(baseline: &ProposalRecord, edit: &ProposalEdit, version: u64) -> ProposalRecord {
    let mut record = baseline.clone();
    record.version = version;
    record.updated_at_ms += 1;
    record.draft.title = edit.title.clone();
    for (change, text) in record.draft.changes.iter_mut().zip(&edit.texts) {
        match change {
            NoteChange::Create { text: current, .. }
            | NoteChange::Replace { text: current, .. } => {
                *current = text.clone().unwrap();
            }
            NoteChange::Trash { .. }
            | NoteChange::CreateAsset { .. }
            | NoteChange::ReplaceAsset { .. }
            | NoteChange::TrashAsset { .. } => assert!(text.is_none()),
        }
    }
    record
}

#[test]
fn full_exact_snapshot_coalesces_and_guards_leaving_until_acknowledged() {
    let baseline = fixture();
    let mut review = ProposalReview::new(baseline.clone());
    let now = Instant::now();
    assert_eq!(review.title(), baseline.draft.title);
    assert_eq!(review.text(0), baseline.draft.changes[0].text());
    assert_eq!(review.text(2), None);
    assert!(review.can_leave() && review.can_mutate());
    assert!(!review.wants_recovery(now, true));
    review
        .edit_title("\u{feff}新しい title\r\nλ".into(), now)
        .unwrap();
    review
        .edit_text(1, "\u{feff}Точный новый текст\r\nλ".into(), now)
        .unwrap();
    assert!(review.dirty() && !review.can_leave() && !review.can_mutate());
    assert!(!review.wants_recovery(now + Duration::from_millis(499), false));
    assert!(review.wants_recovery(now + Duration::from_millis(500), false));
    assert!(review.wants_recovery(now, true));
    let (id, edit) = review.prepare_edit().unwrap();
    assert_eq!(edit.expected, baseline.stamp());
    assert_eq!(edit.title, "\u{feff}新しい title\r\nλ");
    assert_eq!(
        edit.texts,
        vec![
            baseline.draft.changes[0].text().map(str::to_owned),
            Some("\u{feff}Точный новый текст\r\nλ".into()),
            None
        ]
    );
    assert!(review.pending());
    assert!(review.prepare_edit().is_none());
    assert!(!review.discard_local());
    assert!(!review.wants_recovery(now + Duration::from_secs(1), true));
    let acknowledged = reply(&baseline, &edit, baseline.version + 1);
    assert!(review.acknowledge_edit(id, acknowledged.clone()));
    assert_eq!(review.record, acknowledged);
    assert_eq!(review.record.comments, baseline.comments);
    assert_eq!(review.record.draft.sources, baseline.draft.sources);
    assert!(!review.dirty() && !review.pending());
    assert!(review.can_leave() && review.can_mutate());
}

#[test]
fn late_acknowledgement_preserves_every_later_keystroke_and_uses_the_new_stamp() {
    let baseline = fixture();
    let mut review = ProposalReview::new(baseline.clone());
    let now = Instant::now();
    review
        .edit_text(0, "\u{feff}Submitted 日本語\r\nλ".into(), now)
        .unwrap();
    let (id, submitted) = review.prepare_edit().unwrap();
    review
        .edit_title(
            "\u{feff}Later title λ\r\n".into(),
            now + Duration::from_millis(100),
        )
        .unwrap();
    review
        .edit_text(
            0,
            "\u{feff}Later typing 日本語\r\nλ".into(),
            now + Duration::from_millis(200),
        )
        .unwrap();
    review
        .edit_text(
            1,
            "\u{feff}Later Замена\r\nλ".into(),
            now + Duration::from_millis(300),
        )
        .unwrap();
    let acknowledged = reply(&baseline, &submitted, baseline.version + 1);
    assert!(review.acknowledge_edit(id, acknowledged.clone()));
    assert_eq!(review.record, acknowledged);
    assert_eq!(review.title(), "\u{feff}Later title λ\r\n");
    assert_eq!(review.text(0), Some("\u{feff}Later typing 日本語\r\nλ"));
    assert_eq!(review.text(1), Some("\u{feff}Later Замена\r\nλ"));
    assert!(review.dirty() && !review.pending() && !review.can_leave());
    assert!(!review.wants_recovery(now + Duration::from_millis(799), false));
    assert!(review.wants_recovery(now + Duration::from_millis(800), false));
    let (next_id, next) = review.prepare_edit().unwrap();
    assert_ne!(next_id, id);
    assert_eq!(next.expected, acknowledged.stamp());
    assert_eq!(next.title, review.title());
    assert_eq!(next.texts, review.texts());
    assert!(!review.acknowledge_edit(id, acknowledged.clone()));
    assert!(review.pending());
    assert!(review.acknowledge_edit(next_id, reply(&acknowledged, &next, baseline.version + 2)));
    assert!(review.can_leave() && review.can_mutate());
}

#[test]
fn typing_back_to_baseline_needs_an_exact_noop_acknowledgement() {
    let baseline = fixture();
    let now = Instant::now();
    let mut review = ProposalReview::new(baseline.clone());
    review.edit_title("temporary".into(), now).unwrap();
    review
        .edit_title(baseline.draft.title.clone(), now)
        .unwrap();
    assert!(review.dirty());
    let (id, edit) = review.prepare_edit().unwrap();
    assert_eq!(edit, full_edit(&baseline));
    assert!(review.acknowledge_edit(id, baseline.clone()));
    assert!(review.can_leave());

    review.edit_title("temporary".into(), now).unwrap();
    review
        .edit_title(baseline.draft.title.clone(), now)
        .unwrap();
    let (id, edit) = review.prepare_edit().unwrap();
    assert!(!review.acknowledge_edit(id, reply(&baseline, &edit, baseline.version + 1)));
    assert!(review.dirty() && review.error.is_some() && !review.pending());
    assert!(!review.wants_recovery(now + Duration::from_secs(2), true));
}

#[test]
fn acknowledgement_refuses_wrong_correlation_contents_versions_and_all_bindings() {
    let baseline = fixture();
    let now = Instant::now();
    let mut review = ProposalReview::new(baseline.clone());
    review.edit_title("new".into(), now).unwrap();
    let (id, edit) = review.prepare_edit().unwrap();
    let good = reply(&baseline, &edit, baseline.version + 1);
    assert!(!review.acknowledge_edit(Uuid::new_v4(), good.clone()));
    assert!(!review.fail_edit(Uuid::new_v4(), "unrelated".into()));
    assert!(review.pending() && review.error.is_none());
    assert!(review.acknowledge_edit(id, good.clone()));

    let mut bad_replies = vec![];
    let mut wrong = good.clone();
    wrong.draft.id = Uuid::new_v4();
    bad_replies.push(wrong);
    let mut wrong = good.clone();
    wrong.draft.group_id = None;
    bad_replies.push(wrong);
    let mut wrong = good.clone();
    wrong.draft.session_id = None;
    bad_replies.push(wrong);
    let mut wrong = good.clone();
    wrong.draft.vault.as_mut().unwrap().root = "/synthetic/other".into();
    bad_replies.push(wrong);
    let mut wrong = good.clone();
    wrong.draft.vault.as_mut().unwrap().identity.inode += 1;
    bad_replies.push(wrong);
    let mut wrong = good.clone();
    wrong.draft.vault.as_mut().unwrap().id = Uuid::new_v4();
    bad_replies.push(wrong);
    let mut wrong = good.clone();
    wrong.draft.sources[0].fingerprint.inode += 1;
    bad_replies.push(wrong);
    let mut wrong = good.clone();
    wrong.draft.sources[0].path = "other-source.md".into();
    bad_replies.push(wrong);
    let mut wrong = good.clone();
    wrong.created_at_ms = 0;
    bad_replies.push(wrong);
    let mut wrong = good.clone();
    wrong.updated_at_ms = 1;
    bad_replies.push(wrong);
    let mut wrong = good.clone();
    wrong.version = baseline.version;
    bad_replies.push(wrong);
    let mut wrong = good.clone();
    wrong.version += 1;
    bad_replies.push(wrong);
    let mut wrong = good.clone();
    wrong.state = ProposalState::Rejected;
    bad_replies.push(wrong);
    let mut wrong = good.clone();
    wrong.draft.title = "different".into();
    bad_replies.push(wrong);
    let mut wrong = good.clone();
    if let NoteChange::Create { text, .. } = &mut wrong.draft.changes[0] {
        text.push('!');
    }
    bad_replies.push(wrong);
    let mut wrong = good.clone();
    wrong.draft.changes.pop();
    bad_replies.push(wrong);
    let mut wrong = good.clone();
    wrong.draft.changes.swap(0, 1);
    bad_replies.push(wrong);
    let mut wrong = good.clone();
    if let NoteChange::Replace { path, .. } = &mut wrong.draft.changes[1] {
        *path = "other.md".into();
    }
    bad_replies.push(wrong);
    let mut wrong = good.clone();
    if let NoteChange::Create { parent, .. } = &mut wrong.draft.changes[0] {
        parent.inode += 1;
    }
    bad_replies.push(wrong);
    let mut wrong = good.clone();
    if let NoteChange::Replace { before, .. } = &mut wrong.draft.changes[1] {
        before.inode += 1;
    }
    bad_replies.push(wrong);
    let mut wrong = good.clone();
    if let NoteChange::Trash { before_text, .. } = &mut wrong.draft.changes[2] {
        before_text.push('!');
    }
    bad_replies.push(wrong);
    let mut wrong = good.clone();
    wrong.comments[0].text = "x".repeat(16 * 1024 + 1);
    bad_replies.push(wrong);
    for wrong in bad_replies {
        let mut review = ProposalReview::new(baseline.clone());
        review.edit_title("new".into(), now).unwrap();
        let (id, _) = review.prepare_edit().unwrap();
        assert!(!review.acknowledge_edit(id, wrong));
        assert_eq!(review.record, baseline);
        assert_eq!(review.title(), "new");
        assert!(review.dirty() && !review.pending() && review.error.is_some());
        assert!(!review.wants_recovery(now + Duration::from_secs(1), true));
    }
}

#[test]
fn failed_and_invalid_edits_retain_text_until_explicit_retry_or_discard() {
    let baseline = fixture();
    let now = Instant::now();
    let mut review = ProposalReview::new(baseline.clone());
    review
        .edit_text(0, "\u{feff}Retained λ\r\n".into(), now)
        .unwrap();
    let (id, _) = review.prepare_edit().unwrap();
    assert!(!review.retry());
    assert!(review.fail_edit(id, "Synthetic storage failure".into()));
    assert_eq!(review.text(0), Some("\u{feff}Retained λ\r\n"));
    assert_eq!(review.error.as_deref(), Some("Synthetic storage failure"));
    assert!(!review.wants_recovery(now + Duration::from_secs(2), true));
    review.edit_title(String::new(), now).unwrap();
    assert!(!review.wants_recovery(now + Duration::from_secs(2), true));
    assert!(review.retry());
    assert!(review.prepare_edit().is_none()); // Blank local titles are allowed, queueing is not.
    assert!(review.error.is_some() && review.dirty() && !review.pending());
    assert_eq!(review.title(), "");
    review.edit_title("Repaired title λ".into(), now).unwrap();
    assert!(review.prepare_edit().is_none());
    assert!(review.retry());
    let (id, edit) = review.prepare_edit().unwrap();
    assert!(review.acknowledge_edit(id, reply(&baseline, &edit, baseline.version + 1)));
    assert!(!review.retry());
    assert!(review.can_leave());
    review.edit_title("discard me".into(), now).unwrap();
    assert!(review.discard_local());
    assert_eq!(review.title(), "Repaired title λ");
    assert!(review.can_mutate());
}

#[test]
fn byte_limits_and_generation_overflow_refuse_atomically_before_typing_changes() {
    let baseline = fixture();
    let mut review = ProposalReview::new(baseline.clone());
    let now = Instant::now();
    assert!(review.edit_title("λ".repeat(257), now).is_err());
    assert!(
        review
            .edit_text(0, "x".repeat(MAX_NOTE_BYTES + 1), now)
            .is_err()
    );
    assert!(
        review
            .edit_text(2, "Trash is not editable".into(), now)
            .is_err()
    );
    assert!(review.edit_text(3, "out of range".into(), now).is_err());
    assert!(
        review
            .edit_text(usize::MAX, "out of range".into(), now)
            .is_err()
    );
    assert_eq!(review.title(), baseline.draft.title);
    assert_eq!(review.texts(), full_edit(&baseline).texts);
    assert!(review.can_leave());
    review.edit_title("λ".repeat(256), now).unwrap();
    review
        .edit_text(0, "x".repeat(MAX_NOTE_BYTES), now)
        .unwrap();
    assert!(review.prepare_edit().is_some());
    let mut exhausted = ProposalReview::new(baseline.clone());
    exhausted.generation = u64::MAX;
    exhausted.acknowledged_generation = u64::MAX;
    assert!(exhausted.edit_title("different".into(), now).is_err());
    assert!(exhausted.edit_text(0, "different".into(), now).is_err());
    assert_eq!(exhausted.generation, u64::MAX);
    assert_eq!(exhausted.title(), baseline.draft.title);
    assert_eq!(exhausted.texts(), full_edit(&baseline).texts);
    assert!(exhausted.can_leave());
}

#[test]
fn full_result_validation_rejects_aggregate_overflow_without_losing_local_members() {
    let mut baseline = fixture();
    let member = baseline.draft.changes[0].clone();
    baseline.draft.changes = (0..8)
        .map(|index| {
            let mut change = member.clone();
            if let NoteChange::Create { path, text, .. } = &mut change {
                *path = format!("part-{index}.md");
                *text = String::new();
            }
            change
        })
        .collect();
    brn_workflow::proposals::validate_review_edit(&baseline, &full_edit(&baseline)).unwrap();
    let mut review = ProposalReview::new(baseline.clone());
    let now = Instant::now();
    for index in 0..8 {
        review
            .edit_text(index, "x".repeat(MAX_NOTE_BYTES), now)
            .unwrap();
    }
    assert!(review.prepare_edit().is_none());
    assert!(review.error.is_some() && review.dirty() && !review.pending());
    assert!(
        review
            .texts()
            .iter()
            .all(|text| text.as_ref().unwrap().len() == MAX_NOTE_BYTES)
    );
    assert_eq!(review.record, baseline);
    assert!(!review.wants_recovery(now + Duration::from_secs(1), true));
}

#[test]
fn newer_rewrite_conflict_retains_local_bytes_until_explicit_discard() {
    let baseline = fixture();
    let now = Instant::now();
    let mut review = ProposalReview::new(baseline.clone());
    review
        .edit_text(0, "\u{feff}Local 日本語\r\nλ".into(), now)
        .unwrap();
    let (id, submitted) = review.prepare_edit().unwrap();
    review
        .edit_title("Later local title λ".into(), now)
        .unwrap();
    let mut remote_edit = full_edit(&baseline);
    remote_edit.title = "Remote Rewrite λ".into();
    remote_edit.texts[1] = Some("\u{feff}Remote Замена\r\nλ".into());
    let mut remote = reply(&baseline, &remote_edit, baseline.version + 2);
    remote.comments[0].text = "Latest review work λ".into();
    assert!(review.observe(remote.clone()));
    assert_eq!(review.observed, Some(remote.clone()));
    assert_eq!(review.record, baseline);
    assert_eq!(review.text(0), Some("\u{feff}Local 日本語\r\nλ"));
    assert_eq!(review.title(), "Later local title λ");
    assert!(review.error.is_some());
    assert!(!review.retry() && !review.discard_local());
    assert!(!review.wants_recovery(now + Duration::from_secs(2), true));
    assert!(review.acknowledge_edit(id, reply(&baseline, &submitted, baseline.version + 1)));
    assert_eq!(review.observed, Some(remote.clone()));
    assert!(!review.can_mutate() && !review.can_leave());
    assert!(review.prepare_edit().is_none());
    assert!(!review.retry());
    assert!(!review.observe(baseline.clone()));
    assert_eq!(review.observed, Some(remote.clone()));
    assert!(review.discard_local());
    assert_eq!(review.record, remote);
    assert_eq!(review.title(), "Remote Rewrite λ");
    assert_eq!(review.text(1), Some("\u{feff}Remote Замена\r\nλ"));
    assert!(review.observed.is_none() && review.error.is_none());
    assert!(review.can_mutate() && review.can_leave());
}

#[test]
fn identical_worker_observation_and_ack_clear_only_their_own_snapshot() {
    let baseline = fixture();
    let now = Instant::now();
    let mut review = ProposalReview::new(baseline.clone());
    review.edit_title("Queued exact title".into(), now).unwrap();
    let (id, edit) = review.prepare_edit().unwrap();
    let acknowledged = reply(&baseline, &edit, baseline.version + 1);
    assert!(review.observe(acknowledged.clone()));
    assert!(review.observed.is_some());
    assert!(review.acknowledge_edit(id, acknowledged.clone()));
    assert!(review.observed.is_none() && review.error.is_none() && review.can_leave());

    // A same-version conflicting payload cannot count as the matching ACK.
    review
        .edit_title("Second queued title".into(), now)
        .unwrap();
    let (id, edit) = review.prepare_edit().unwrap();
    let exact = reply(&acknowledged, &edit, acknowledged.version + 1);
    let mut other = exact.clone();
    other.draft.title = "Different same-version observation".into();
    assert!(review.observe(other.clone()));
    assert!(review.acknowledge_edit(id, exact));
    assert_eq!(review.observed, Some(other.clone()));
    assert_eq!(review.title(), "Second queued title");
    assert!(!review.can_leave());
    assert!(review.discard_local());
    assert_eq!(review.record, other);
}

#[test]
fn clean_observations_adopt_full_review_and_stale_or_foreign_records_do_nothing() {
    let baseline = fixture();
    let mut review = ProposalReview::new(baseline.clone());
    let mut edit = full_edit(&baseline);
    edit.title = "Clean external title λ".into();
    edit.texts[0] = Some("\u{feff}External 日本語\r\nλ".into());
    let mut newer = reply(&baseline, &edit, baseline.version + 1);
    newer.comments[0].text = "External review comment".into();
    assert!(review.observe(newer.clone()));
    assert_eq!(review.record, newer);
    assert_eq!(review.text(0), Some("\u{feff}External 日本語\r\nλ"));
    assert!(review.can_mutate());
    assert!(!review.observe(baseline));
    let mut foreign = newer.clone();
    foreign.draft.id = Uuid::new_v4();
    assert!(!review.observe(foreign));
    let mut changed_binding = newer.clone();
    changed_binding.draft.sources.clear();
    assert!(!review.observe(changed_binding));
    let mut changed_binding = newer.clone();
    changed_binding.draft.changes.swap(0, 1);
    assert!(!review.observe(changed_binding));
    assert_eq!(review.record, newer);
    assert!(review.observed.is_none() && review.error.is_none() && review.can_leave());
}

#[test]
fn exact_utf8_selection_never_searches_trims_or_reanchors_repeated_text() {
    let baseline = fixture();
    let review = ProposalReview::new(baseline.clone());
    let text = baseline.draft.changes[0].text().unwrap();
    let first = text.find("repeat λ").unwrap();
    let second = text.rfind("repeat λ").unwrap();
    assert_ne!(first, second);
    for start in [first, second] {
        let target = selection_target(&review, 0, start..start + "repeat λ".len()).unwrap();
        assert_eq!(
            target,
            CommentTarget::Text(TextAnchor {
                change_index: 0,
                start,
                end: start + "repeat λ".len(),
                quote: "repeat λ".into()
            })
        );
    }
    assert_eq!(
        selection_target(&review, 0, 0..text.len()),
        Some(CommentTarget::Text(TextAnchor {
            change_index: 0,
            start: 0,
            end: text.len(),
            quote: text.into()
        }))
    );
    let lambda = text.find('λ').unwrap();
    assert!(selection_target(&review, 0, lambda..lambda + 1).is_none());
    assert!(selection_target(&review, 0, lambda + 1..lambda + 2).is_none());
    assert!(selection_target(&review, 0, 1..3).is_none()); // Inside BOM.
    assert!(selection_target(&review, 0, 0..0).is_none());
    assert!(selection_target(&review, 0, std::ops::Range { start: 4, end: 3 }).is_none());
    assert!(selection_target(&review, 0, 0..text.len() + 1).is_none());
    assert!(selection_target(&review, 0, 0..usize::MAX).is_none());
    assert!(selection_target(&review, 2, 0..1).is_none()); // Trash has before-evidence only.
    assert!(selection_target(&review, 3, 0..1).is_none());
    let mut dirty = ProposalReview::new(baseline.clone());
    dirty.edit_title("dirty".into(), Instant::now()).unwrap();
    assert!(selection_target(&dirty, 0, first..first + 1).is_none());
    let (id, edit) = dirty.prepare_edit().unwrap();
    assert!(selection_target(&dirty, 0, first..first + 1).is_none());
    assert!(dirty.fail_edit(id, "Synthetic failure".into()));
    assert!(selection_target(&dirty, 0, first..first + 1).is_none());
    assert!(dirty.retry());
    let (id, _) = dirty.prepare_edit().unwrap();
    assert!(dirty.acknowledge_edit(id, reply(&baseline, &edit, baseline.version + 1)));
    assert!(selection_target(&dirty, 0, first..first + 1).is_some());
}

#[test]
fn store_returned_unresolved_anchor_is_preserved_exactly_without_local_reanchoring() {
    let mut baseline = fixture();
    let text = baseline.draft.changes[0].text().unwrap();
    let start = text.find("repeat λ").unwrap();
    let old_anchor = TextAnchor {
        change_index: 0,
        start,
        end: start + "repeat λ".len(),
        quote: "repeat λ".into(),
    };
    let mut comment = baseline.comments[0].clone();
    comment.id = Uuid::new_v4();
    comment.target = CommentTarget::Text(old_anchor.clone());
    baseline.comments.push(comment);
    brn_workflow::proposals::validate_review_edit(&baseline, &full_edit(&baseline)).unwrap();
    let mut review = ProposalReview::new(baseline.clone());
    review
        .edit_text(0, "\u{feff}λ repeat λ\r\n".into(), Instant::now())
        .unwrap();
    let (id, edit) = review.prepare_edit().unwrap();
    assert_eq!(review.record.comments, baseline.comments);
    let mut acknowledged = reply(&baseline, &edit, baseline.version + 1);
    acknowledged.comments[1].target = CommentTarget::Unresolved(old_anchor.clone());
    assert!(review.acknowledge_edit(id, acknowledged.clone()));
    assert_eq!(review.record.comments, acknowledged.comments);
    assert_eq!(
        review.record.comments[1].target,
        CommentTarget::Unresolved(old_anchor)
    );
    review
        .edit_text(0, "More changes λ".into(), Instant::now())
        .unwrap();
    assert_eq!(review.record.comments, acknowledged.comments);
}

#[test]
fn non_draft_states_block_typing_recovery_mutations_and_selection() {
    let baseline = fixture();
    for state in [
        ProposalState::Rejected,
        ProposalState::Applying,
        ProposalState::Uncertain,
        ProposalState::Applied,
    ] {
        let mut record = baseline.clone();
        record.state = state;
        record.version = 4;
        record.updated_at_ms = 4;
        if state == ProposalState::Applied {
            record.comments.clear();
        }
        let mut review = ProposalReview::new(record.clone());
        assert!(review.can_leave() && !review.can_mutate());
        assert!(
            review
                .edit_title("different".into(), Instant::now())
                .is_err()
        );
        assert!(
            review
                .edit_text(0, "different".into(), Instant::now())
                .is_err()
        );
        assert!(review.prepare_edit().is_none());
        assert!(!review.wants_recovery(Instant::now(), true));
        assert!(selection_target(&review, 0, 0..3).is_none());
        assert_eq!(review.record, record);

        let mut dirty = ProposalReview::new(baseline.clone());
        dirty
            .edit_title("Retain pending title".into(), Instant::now())
            .unwrap();
        assert!(dirty.observe(record.clone()));
        assert_eq!(dirty.title(), "Retain pending title");
        assert!(dirty.edit_title("blocked".into(), Instant::now()).is_err());
        assert!(
            dirty
                .edit_text(0, "blocked".into(), Instant::now())
                .is_err()
        );
        assert!(!dirty.can_leave() && !dirty.retry());
        assert!(dirty.discard_local());
        assert_eq!(dirty.record, record);
        assert!(dirty.can_leave() && !dirty.can_mutate());
    }
}

#[test]
fn matching_newer_acknowledgement_cannot_leave_an_older_observation_to_discard_into() {
    let baseline = fixture();
    let now = Instant::now();
    let mut review = ProposalReview::new(baseline.clone());
    review
        .edit_title("New submitted title".into(), now)
        .unwrap();
    let (id, edit) = review.prepare_edit().unwrap();
    let mut old = baseline.clone();
    old.draft.title = "Differing old observation".into();
    assert!(review.observe(old));
    assert!(review.observed.is_some());
    let acknowledged = reply(&baseline, &edit, baseline.version + 1);
    review
        .edit_text(0, "\u{feff}Later typing λ\r\n".into(), now)
        .unwrap();
    assert!(review.acknowledge_edit(id, acknowledged.clone()));
    assert!(review.observed.is_none());
    assert_eq!(review.text(0), Some("\u{feff}Later typing λ\r\n"));
    assert!(review.discard_local());
    assert_eq!(review.record, acknowledged);
    assert_eq!(review.title(), "New submitted title");
    assert!(review.can_mutate());
}
