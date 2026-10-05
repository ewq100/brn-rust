use brn_store::{
    Error, WorkStore,
    files::{FileFingerprint, VaultIdentity, VaultRecord},
    work::{
        actions::{ActionData, ActionOrigin, ActionRecord, ActionState},
        proposal_apply::{ApplyJournal, ApplyMember, ApprovalRequest},
        proposal_rewrite::{RewriteOutcome, RewriteSpec, RewriteStatus, validate_result},
        proposals::*,
    },
};
use rusqlite::{Connection, params};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fmt::Write;
use uuid::Uuid;

fn fixture() -> (tempfile::TempDir, WorkStore) {
    let dir = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let (store, _) = WorkStore::open(dir.path()).unwrap();
    (dir, store)
}

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn hex(bytes: &[u8]) -> String {
    let mut text = String::new();
    for byte in bytes {
        write!(text, "{byte:02x}").unwrap();
    }
    text
}

fn data() -> ActionData {
    ActionData {
        title: "  Küsi Annalt λ\r\n".into(),
        description: "\u{feff}Exact English ja Eesti. „Tähtaeg on reede.“\r\n".into(),
        state: ActionState::Waiting,
        owner: Some("  Anna Õun  ".into()),
        related_person: Some(Uuid::new_v4()),
        related_project: Some(Uuid::new_v4()),
        sources: vec![Uuid::new_v4()],
        thread: Some(Uuid::new_v4()),
        due_on: Some("2028-02-29".into()),
        follow_up_on: Some("2026-10-08".into()),
        dependencies: vec![Uuid::new_v4()],
        parent: None,
        follows_up: None,
        priority: None,
    }
}

fn before(id: Uuid) -> ActionRecord {
    let data = data();
    ActionRecord {
        origin: ActionOrigin {
            id,
            proposal: ProposalStamp {
                id: Uuid::new_v4(),
                version: 2,
            },
            data: data.clone(),
            created_at_ms: 1000,
        },
        version: 1,
        data,
        updated_at_ms: 1000,
        waiting_since_ms: Some(1000),
        completed_at_ms: None,
    }
}

fn draft() -> ProposalDraft {
    ProposalDraft {
        inbox_source: None,
        id: Uuid::new_v4(),
        group_id: Some(Uuid::new_v4()),
        session_id: Some(Uuid::new_v4()),
        vault: None,
        title: "Exact Action review".into(),
        changes: vec![],
        sources: vec![],
        action_changes: vec![
            ActionChange::Create {
                id: Uuid::new_v4(),
                data: data(),
            },
            ActionChange::Replace {
                before: Box::new(before(Uuid::new_v4())),
                data: data(),
            },
        ],
    }
}

fn edit(record: &ProposalRecord) -> ProposalEdit {
    ProposalEdit {
        expected: record.stamp(),
        title: record.draft.title.clone(),
        texts: record
            .draft
            .changes
            .iter()
            .map(|c| c.text().map(str::to_owned))
            .collect(),
        action_data: record
            .draft
            .action_changes
            .iter()
            .map(|c| c.data().clone())
            .collect(),
    }
}

fn with_note(draft: &mut ProposalDraft) {
    let parent = VaultIdentity {
        device: 1,
        inode: 1,
    };
    draft.vault = Some(VaultRecord {
        id: Uuid::new_v4(),
        root: "/synthetic/vault".into(),
        identity: parent.clone(),
    });
    draft.changes.push(NoteChange::Create {
        path: "note.md".into(),
        parent,
        text: "Exact note\r\n".into(),
    });
}

#[test]
fn action_only_review_edit_reject_restart_and_creation_replay_preserve_exact_bindings() {
    let (dir, mut store) = fixture();
    let initial = draft();
    let first = store.create_proposal(&initial).unwrap();
    assert_eq!(first.draft, initial);
    assert_eq!(first.version, 1);
    let commented = store
        .add_proposal_comment(&CommentRequest {
            expected: first.stamp(),
            comment: ReviewComment {
                id: Uuid::new_v4(),
                text: "Whole Action review note\r\n".into(),
                target: CommentTarget::Proposal,
            },
        })
        .unwrap();
    let mut request = edit(&commented);
    request.title = "Reviewed title".into();
    request.action_data[0]
        .description
        .push_str("  Täpne parandus.\r\n");
    request.action_data[1].state = ActionState::Blocked;
    request.action_data[1].owner = None;
    let edited = store.rewrite_proposal(&request).unwrap();
    assert_eq!(edited.version, commented.version + 1);
    assert_eq!(edited.comments, commented.comments);
    assert_eq!(
        edited.draft.action_changes[0].id(),
        initial.action_changes[0].id()
    );
    assert_eq!(
        edited.draft.action_changes[1].id(),
        initial.action_changes[1].id()
    );
    assert!(
        matches!((&edited.draft.action_changes[1], &initial.action_changes[1]),
        (ActionChange::Replace { before: a, .. }, ActionChange::Replace { before: b, .. }) if a == b)
    );
    assert_eq!(store.edit_proposal(&edit(&edited)).unwrap(), edited);
    assert!(matches!(
        store.edit_proposal(&request),
        Err(Error::StateChanged(_))
    ));
    assert_eq!(store.create_proposal(&initial).unwrap(), edited);
    let mut conflicting = initial.clone();
    conflicting.action_changes[0]
        .data_mut()
        .description
        .push('x');
    assert!(matches!(
        store.create_proposal(&conflicting),
        Err(Error::OperationConflict(_))
    ));
    let rejected = store.reject_proposal(edited.stamp()).unwrap();
    assert_eq!(rejected.state, ProposalState::Rejected);
    assert_eq!(rejected.draft, edited.draft);
    assert!(
        store
            .action_list(&Default::default())
            .unwrap()
            .entries
            .is_empty()
    );
    drop(store);
    let (store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(store.proposal(initial.id).unwrap(), Some(rejected.clone()));
    assert_eq!(store.proposals(initial.group_id).unwrap(), [rejected]);
    assert!(
        store
            .action_list(&Default::default())
            .unwrap()
            .entries
            .is_empty()
    );
    assert!(!dir.path().join("credentials").exists());
}

#[test]
fn mixed_notes_and_source_proofs_require_a_vault_but_action_only_does_not() {
    let (_dir, mut store) = fixture();
    let mut mixed = draft();
    with_note(&mut mixed);
    let full = store.create_proposal(&mixed).unwrap();
    let mut request = edit(&full);
    request.texts[0] = Some("Changed exact note\r\n".into());
    request.action_data[0].due_on = None;
    let result = store.edit_proposal(&request).unwrap();
    assert_eq!(result.draft.vault, mixed.vault);
    assert_eq!(
        result.draft.changes[0].text(),
        Some("Changed exact note\r\n")
    );
    mixed.id = Uuid::new_v4();
    mixed.vault = None;
    assert!(store.create_proposal(&mixed).is_err());
    let mut source_only = draft();
    source_only.sources.push(SourceVersion {
        path: "source.md".into(),
        fingerprint: FileFingerprint {
            device: 1,
            inode: 2,
            len: 1,
            sha256: digest(b"x"),
        },
    });
    assert!(store.create_proposal(&source_only).is_err());
    with_note(&mut source_only);
    source_only.changes.clear();
    assert!(store.create_proposal(&source_only).is_ok());
}

#[test]
fn action_domains_duplicates_completed_baselines_and_candidates_refuse_without_admission() {
    let (_dir, mut store) = fixture();
    for case in 0..9 {
        let mut bad = draft();
        match case {
            0 => {
                bad.action_changes[0] = ActionChange::Create {
                    id: Uuid::nil(),
                    data: data(),
                }
            }
            1 => bad.action_changes[0].data_mut().state = ActionState::Completed,
            2 => bad.action_changes[1].data_mut().state = ActionState::Completed,
            3 => bad.action_changes[0].data_mut().dependencies = vec![bad.action_changes[0].id()],
            4 => bad.action_changes[0].data_mut().due_on = Some("2026-02-29".into()),
            5 => bad.action_changes[0].data_mut().sources = vec![Uuid::nil()],
            6 => {
                let id = bad.action_changes[1].id();
                bad.action_changes[0] = ActionChange::Create { id, data: data() };
            }
            7 => {
                if let ActionChange::Replace { before, .. } = &mut bad.action_changes[1] {
                    before.version = 0;
                }
            }
            _ => {
                if let ActionChange::Replace { before, .. } = &mut bad.action_changes[1] {
                    before.version = 2;
                    before.data.state = ActionState::Completed;
                    before.waiting_since_ms = None;
                    before.completed_at_ms = Some(1000);
                    before.validate().unwrap();
                }
            }
        }
        assert!(store.create_proposal(&bad).is_err(), "case {case}");
        assert_eq!(store.proposal(bad.id).unwrap(), None);
    }
    let initial = store.create_proposal(&draft()).unwrap();
    for case in 0..4 {
        let mut bad = edit(&initial);
        match case {
            0 => {
                bad.action_data.pop();
            }
            1 => bad.action_data[0].state = ActionState::Completed,
            2 => bad.action_data[1].parent = Some(initial.draft.action_changes[1].id()),
            _ => bad.action_data[0].description = "x".repeat(64 * 1024 + 1),
        }
        assert!(store.edit_proposal(&bad).is_err());
        assert_eq!(
            store.proposal(initial.draft.id).unwrap(),
            Some(initial.clone())
        );
    }
}

#[test]
fn combined_members_and_full_serialized_action_baselines_share_the_existing_budget() {
    let (_dir, mut store) = fixture();
    let mut maximum = draft();
    maximum.action_changes = (0..64)
        .map(|_| ActionChange::Create {
            id: Uuid::new_v4(),
            data: data(),
        })
        .collect();
    store.create_proposal(&maximum).unwrap();
    maximum.id = Uuid::new_v4();
    with_note(&mut maximum);
    assert!(store.create_proposal(&maximum).is_err());
    maximum.action_changes.pop();
    store.create_proposal(&maximum).unwrap();
    let mut full = draft();
    full.action_changes.clear();
    for _ in 0..8 {
        let mut before = before(Uuid::new_v4());
        before.data.description = "\0".repeat(64 * 1024);
        before.origin.data = before.data.clone();
        full.action_changes.push(ActionChange::Replace {
            data: before.data.clone(),
            before: Box::new(before),
        });
    }
    assert!(serde_json::to_vec(&full.action_changes).unwrap().len() > MAX_PROPOSAL_BYTES);
    assert!(
        store.create_proposal(&full).is_err(),
        "origin, current baseline and candidate must all count"
    );
    full.action_changes.pop();
    with_note(&mut full);
    let action_bytes: usize = full
        .action_changes
        .iter()
        .map(|change| serde_json::to_vec(change).unwrap().len())
        .sum();
    let fixed = action_bytes + full.title.len() + "/synthetic/vault".len() + "note.md".len();
    let remaining = MAX_PROPOSAL_BYTES - fixed;
    assert!(remaining <= brn_store::MAX_NOTE_BYTES);
    if let NoteChange::Create { text, .. } = &mut full.changes[0] {
        *text = "x".repeat(remaining);
    }
    let exact = store.create_proposal(&full).unwrap();
    assert!(
        store
            .add_proposal_comment(&CommentRequest {
                expected: exact.stamp(),
                comment: ReviewComment {
                    id: Uuid::new_v4(),
                    text: "x".into(),
                    target: CommentTarget::Proposal
                }
            })
            .is_err()
    );
    assert_eq!(store.proposal(full.id).unwrap(), Some(exact));
}

#[test]
fn action_json_is_strict_and_whole_proposal_comments_do_not_invent_action_anchors() {
    let initial = draft();
    let mut value = serde_json::to_value(&initial).unwrap();
    value["action_changes"][0]["unexpected"] = json!(true);
    assert!(serde_json::from_value::<ProposalDraft>(value).is_err());
    let encoded = serde_json::to_string(&initial.action_changes[0]).unwrap();
    assert!(
        serde_json::from_str::<ActionChange>(&encoded.replacen(
            "\"kind\":\"create\"",
            "\"kind\":\"create\",\"kind\":\"create\"",
            1
        ))
        .is_err()
    );
    let mut value = serde_json::to_value(&initial).unwrap();
    value["action_changes"] = json!(null);
    assert!(serde_json::from_value::<ProposalDraft>(value).is_err());
    let (_dir, mut store) = fixture();
    let record = store.create_proposal(&initial).unwrap();
    assert!(
        store
            .add_proposal_comment(&CommentRequest {
                expected: record.stamp(),
                comment: ReviewComment {
                    id: Uuid::new_v4(),
                    text: "No guessed Action text range".into(),
                    target: CommentTarget::Text(TextAnchor {
                        change_index: 0,
                        start: 0,
                        end: 1,
                        quote: " ".into()
                    })
                }
            })
            .is_err()
    );
    assert_eq!(store.proposal(initial.id).unwrap(), Some(record));
}

#[test]
fn unbound_action_only_and_mixed_apply_refuse_admission_or_recovery() {
    for mixed in [false, true] {
        let (dir, mut store) = fixture();
        let mut draft = draft();
        if mixed {
            with_note(&mut draft);
        }
        let record = store.create_proposal(&draft).unwrap();
        let request = ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: record.stamp(),
        };
        assert!(store.begin_proposal_apply(&request).is_err());
        assert_eq!(store.proposal_apply(request.operation_id).unwrap(), None);
        assert_eq!(store.proposal(draft.id).unwrap(), Some(record.clone()));
        let members = record
            .draft
            .changes
            .iter()
            .map(|c| {
                let id = Uuid::new_v4();
                ApplyMember {
                    id,
                    staging: std::path::Path::new(c.path())
                        .with_file_name(format!(".brn-{id}.stage")),
                }
            })
            .collect();
        let journal = ApplyJournal {
            request,
            approved: record.clone(),
            creation_sha256: digest(&serde_json::to_vec(&draft).unwrap()),
            members,
            prepared: None,
            receipt: None,
            observations: None,
            no_effects: false,
            undo: None,
            repair: None,
            started_at_ms: record.updated_at_ms,
            action_records: Vec::new(),
        };
        assert!(
            journal.validate().is_err(),
            "central mirror/recovery validation must reject unbound Action members"
        );
        assert!(store.restore_proposal_apply(&journal).is_err());
        assert_eq!(store.proposal(draft.id).unwrap(), Some(record));
        assert!(
            store
                .action_list(&Default::default())
                .unwrap()
                .entries
                .is_empty()
        );
        let raw = Connection::open(dir.path().join("brn.sqlite")).unwrap();
        for table in ["proposal_applies", "proposal_rewrites", "actions"] {
            assert_eq!(
                raw.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                0
            );
        }
    }
}

#[test]
fn action_only_and_mixed_owned_rewrite_capture_full_edits_without_real_mutation() {
    for mixed in [false, true] {
        let (dir, mut store) = fixture();
        let mut draft = draft();
        if mixed {
            with_note(&mut draft);
        }
        let original = store.create_proposal(&draft).unwrap();
        let record = store
            .add_proposal_comment(&CommentRequest {
                expected: original.stamp(),
                comment: ReviewComment {
                    id: Uuid::new_v4(),
                    text: "Whole Action instruction õ\r\n".into(),
                    target: CommentTarget::Proposal,
                },
            })
            .unwrap();
        let spec = RewriteSpec {
            id: Uuid::new_v4(),
            expected: record.stamp(),
            provider: "chatgpt".into(),
            model: "synthetic".into(),
            effort: "high".into(),
        };
        let (running, capture) = store.begin_proposal_rewrite(&spec).unwrap();
        assert_eq!(capture, Some(record.clone()));
        assert_eq!(
            running.capture_sha256,
            digest(&serde_json::to_vec(&record).unwrap())
        );
        let mut result = edit(&record);
        result.title = "Full revised review õ\r\n".into();
        for (index, data) in result.action_data.iter_mut().enumerate() {
            data.title = format!("\u{feff}Revised {index} λ\r\n");
            data.description.push_str("Whole suggestion 🦀\r\n");
            data.state = ActionState::Blocked;
            data.owner = Some("  New owner Õ  ".into());
            data.related_person = Some(Uuid::new_v4());
            data.related_project = Some(Uuid::new_v4());
            data.sources = vec![Uuid::new_v4(), Uuid::new_v4()];
            data.thread = Some(Uuid::new_v4());
            data.due_on = Some("2028-03-01".into());
            data.follow_up_on = Some("2028-03-02".into());
            data.dependencies = vec![Uuid::new_v4(), Uuid::new_v4()];
            data.parent = Some(Uuid::new_v4());
            data.follows_up = Some(Uuid::new_v4());
            data.priority = Some(brn_store::work::actions::ActionPriority::High);
        }
        if mixed {
            result.texts[0] = Some("\u{feff}Full note õ\r\n".into());
        }
        validate_result(&record, &result).unwrap();
        let outcome = RewriteOutcome::Completed(result.clone());
        let finished = store.finish_proposal_rewrite(spec.id, &outcome).unwrap();
        assert_eq!(finished.status, RewriteStatus::Completed);
        let revised = store.proposal(record.draft.id).unwrap().unwrap();
        assert_eq!(finished.result_stamp, Some(revised.stamp()));
        assert_eq!(revised.version, record.version + 1);
        assert_eq!(revised.draft.vault, record.draft.vault);
        assert_eq!(revised.draft.sources, record.draft.sources);
        assert_eq!(revised.comments, record.comments);
        assert_eq!(edit(&revised).action_data, result.action_data);
        for (old, new) in record
            .draft
            .action_changes
            .iter()
            .zip(&revised.draft.action_changes)
        {
            assert_eq!(old.id(), new.id());
            match (old, new) {
                (ActionChange::Create { .. }, ActionChange::Create { .. }) => {}
                (
                    ActionChange::Replace { before: a, .. },
                    ActionChange::Replace { before: b, .. },
                ) => assert_eq!(a, b),
                _ => panic!("immutable member kind changed"),
            }
        }
        assert!(
            store
                .action_list(&Default::default())
                .unwrap()
                .entries
                .is_empty()
        );
        assert_eq!(
            store.begin_proposal_rewrite(&spec).unwrap(),
            (finished.clone(), None)
        );
        drop(store);
        let (mut store, _) = WorkStore::open(dir.path()).unwrap();
        assert_eq!(store.proposal(record.draft.id).unwrap(), Some(revised));
        assert_eq!(
            store.finish_proposal_rewrite(spec.id, &outcome).unwrap(),
            finished
        );
        assert!(
            store
                .action_list(&Default::default())
                .unwrap()
                .entries
                .is_empty()
        );
        let raw = Connection::open(dir.path().join("brn.sqlite")).unwrap();
        let bytes: Vec<u8> = raw
            .query_row(
                "SELECT job_json FROM proposal_rewrites WHERE id=?1",
                [spec.id.to_string()],
                |r| r.get(0),
            )
            .unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(!text.contains("Whole Action instruction") && !text.contains("Whole suggestion"));
    }
}

#[test]
fn owned_action_rewrite_invalid_results_are_whole_refusals_and_later_comments_win() {
    let (_dir, mut store) = fixture();
    let original = store.create_proposal(&draft()).unwrap();
    for case in 0..5 {
        let spec = RewriteSpec {
            id: Uuid::new_v4(),
            expected: original.stamp(),
            provider: "chatgpt".into(),
            model: "synthetic".into(),
            effort: "high".into(),
        };
        store.begin_proposal_rewrite(&spec).unwrap();
        let mut result = edit(&original);
        result.action_data[0].title = "Must not install partially".into();
        match case {
            0 => result.action_data.clear(),
            1 => {
                result.action_data.pop();
            }
            2 => {
                result.action_data.push(data());
            }
            3 => result.action_data[1].state = ActionState::Completed,
            _ => result.action_data[1].sources = vec![Uuid::nil()],
        }
        assert!(validate_result(&original, &result).is_err());
        assert!(
            store
                .finish_proposal_rewrite(spec.id, &RewriteOutcome::Completed(result))
                .is_err()
        );
        assert_eq!(
            store.proposal(original.draft.id).unwrap(),
            Some(original.clone())
        );
        assert_eq!(
            store.proposal_rewrite(spec.id).unwrap().unwrap().status,
            RewriteStatus::Running
        );
        store
            .finish_proposal_rewrite(spec.id, &RewriteOutcome::Failed("tool_rejected".into()))
            .unwrap();
    }
    let spec = RewriteSpec {
        id: Uuid::new_v4(),
        expected: original.stamp(),
        provider: "chatgpt".into(),
        model: "synthetic".into(),
        effort: "high".into(),
    };
    store.begin_proposal_rewrite(&spec).unwrap();
    let later = store
        .add_proposal_comment(&CommentRequest {
            expected: original.stamp(),
            comment: ReviewComment {
                id: Uuid::new_v4(),
                text: "Newer exact instruction".into(),
                target: CommentTarget::Proposal,
            },
        })
        .unwrap();
    let mut result = edit(&original);
    result.action_data[0].description = "Late suggestion".into();
    let finished = store
        .finish_proposal_rewrite(spec.id, &RewriteOutcome::Completed(result))
        .unwrap();
    assert_eq!(finished.status, RewriteStatus::Stale);
    assert_eq!(store.proposal(original.draft.id).unwrap(), Some(later));
    assert!(
        store
            .action_list(&Default::default())
            .unwrap()
            .entries
            .is_empty()
    );
}

const OLD_DRAFT: &[u8]=br#"{"id":"00000000-0000-4000-8000-000000000001","group_id":null,"session_id":null,"vault":{"id":"00000000-0000-4000-8000-000000000002","root":"/synthetic/vault","identity":{"device":1,"inode":1}},"title":"Original","changes":[{"kind":"create","path":"note.md","parent":{"device":1,"inode":1},"text":"exact\r\n"}],"sources":[]}"#;
const OLD_EDIT: &[u8]=br#"{"expected":{"id":"00000000-0000-4000-8000-000000000001","version":1},"title":"Changed","texts":["changed\r\n"]}"#;
const OLD_OUTCOME: &[u8]=br#"{"completed":{"expected":{"id":"00000000-0000-4000-8000-000000000001","version":1},"title":"Changed","texts":["changed\r\n"]}}"#;
const OLD_JOURNAL: &[u8]=br#"{"request":{"operation_id":"00000000-0000-4000-8000-000000000004","expected":{"id":"00000000-0000-4000-8000-000000000001","version":1}},"approved":{"draft":{"id":"00000000-0000-4000-8000-000000000001","group_id":null,"session_id":null,"vault":{"id":"00000000-0000-4000-8000-000000000002","root":"/synthetic/vault","identity":{"device":1,"inode":1}},"title":"Original","changes":[{"kind":"create","path":"note.md","parent":{"device":1,"inode":1},"text":"exact\r\n"}],"sources":[]},"version":1,"state":"draft","comments":[],"created_at_ms":1000,"updated_at_ms":1000},"creation_sha256":[158,218,54,11,232,204,32,198,223,255,148,72,123,31,66,76,114,215,107,76,227,152,29,226,66,138,149,36,58,126,108,9],"members":[{"id":"00000000-0000-4000-8000-000000000003","staging":".brn-00000000-0000-4000-8000-000000000003.stage"}],"prepared":null,"receipt":null,"observations":null,"started_at_ms":1000}"#;

#[test]
fn historical_markdown_json_creation_and_rewrite_hashes_stay_exact_after_reopen_and_replay() {
    let draft: ProposalDraft = serde_json::from_slice(OLD_DRAFT).unwrap();
    let edit: ProposalEdit = serde_json::from_slice(OLD_EDIT).unwrap();
    let outcome: RewriteOutcome = serde_json::from_slice(OLD_OUTCOME).unwrap();
    for (bytes, expected, actual) in [
        (
            OLD_DRAFT,
            "9eda360be8cc20c6dfff94487b1f424c72d76b4ce3981de2428a95243a7e6c09",
            serde_json::to_vec(&draft).unwrap(),
        ),
        (
            OLD_EDIT,
            "5150ddc998ef9d736a7922deb1c5dbc8e233e32627b1d5c9ad5ef943f4992965",
            serde_json::to_vec(&edit).unwrap(),
        ),
        (
            OLD_OUTCOME,
            "99a279ee5b082ac94b7f58bf5197cad88f40e69105a735251da45a05fa03f4c8",
            serde_json::to_vec(&outcome).unwrap(),
        ),
    ] {
        assert_eq!(actual, bytes);
        assert_eq!(hex(&digest(bytes)), expected);
    }
    assert!(draft.action_changes.is_empty());
    assert!(edit.action_data.is_empty());
    let (dir, mut store) = fixture();
    let initial = store.create_proposal(&draft).unwrap();
    let spec = RewriteSpec {
        id: Uuid::new_v4(),
        expected: initial.stamp(),
        provider: "copilot".into(),
        model: "synthetic".into(),
        effort: "high".into(),
    };
    store.begin_proposal_rewrite(&spec).unwrap();
    let job = store.finish_proposal_rewrite(spec.id, &outcome).unwrap();
    assert_eq!(job.outcome_sha256, Some(digest(OLD_OUTCOME)));
    let updated = store.proposal(draft.id).unwrap().unwrap();
    assert_eq!(updated.version, 2);
    assert_eq!(updated.draft.changes[0].text(), Some("changed\r\n"));
    drop(store);
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(
        store.finish_proposal_rewrite(spec.id, &outcome).unwrap(),
        job
    );
    assert_eq!(store.create_proposal(&draft).unwrap(), updated);
    let raw = Connection::open(dir.path().join("brn.sqlite")).unwrap();
    let (creation, bytes): (Vec<u8>, Vec<u8>) = raw
        .query_row(
            "SELECT creation_sha256,record_json FROM proposals WHERE id=?1",
            [draft.id.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(creation, digest(OLD_DRAFT));
    assert!(!String::from_utf8(bytes).unwrap().contains("action_changes"));
}

#[test]
fn historical_journal_bytes_and_digest_survive_restore_restart_and_replay() {
    let journal: ApplyJournal = serde_json::from_slice(OLD_JOURNAL).unwrap();
    journal.validate().unwrap();
    assert_eq!(serde_json::to_vec(&journal).unwrap(), OLD_JOURNAL);
    assert_eq!(
        hex(&digest(OLD_JOURNAL)),
        "f48a08cd27e541df25751f824c1be7a81097dae84cee67870f88bdfc609d1677"
    );
    let (dir, mut store) = fixture();
    assert_eq!(store.restore_proposal_apply(&journal).unwrap(), journal);
    drop(store);
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(store.restore_proposal_apply(&journal).unwrap(), journal);
    assert_eq!(
        store.begin_proposal_apply(&journal.request).unwrap(),
        journal
    );
    let raw = Connection::open(dir.path().join("brn.sqlite")).unwrap();
    let (bytes, hash): (Vec<u8>, Vec<u8>) = raw
        .query_row(
            "SELECT journal_json,journal_sha256 FROM proposal_applies WHERE operation_id=?1",
            params![journal.request.operation_id.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(bytes, OLD_JOURNAL);
    assert_eq!(hash, digest(OLD_JOURNAL));
}
