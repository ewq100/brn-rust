#![cfg(target_os = "macos")]
use brn_workflow::{
    ErrorKind,
    app::{App, AppConfig},
    app_worker::{AppCommand, AppEvent, AppWorker},
    knowledge::{LinkRequest, NoteLinkOutcome},
    proposal_apply::{ApplyOutcome, ApprovalRequest, UndoRequest},
    proposals::{DraftNoteChange, DraftRequest, ProposalEdit, ProposalRecord, ProposalState},
};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, time::Duration};
use uuid::Uuid;

const CURRENT_ID: &str = "11111111-1111-4111-8111-111111111111";
const TARGET_ID: &str = "22222222-2222-4222-8222-222222222222";
const OTHER_ID: &str = "33333333-3333-4333-8333-333333333333";
const CURRENT: &str = "current.md";
const TARGET: &str = "archive/source.md";
const ORIGINAL: &str = "\u{feff}---\r\nbrn_id: 11111111-1111-4111-8111-111111111111\r\ncustom: untouched\r\n...\r\n# Õ\r\nBody\nlast\r";

fn note(id: &str, body: &str) -> String {
    format!("---\nbrn_id: {id}\n---\n{body}")
}
struct Fixture {
    _owner: tempfile::TempDir,
    data: PathBuf,
    vault: PathBuf,
    credentials: PathBuf,
    target: String,
}
impl Fixture {
    fn new() -> Self {
        Self::in_parent(&std::env::temp_dir().canonicalize().unwrap())
    }
    fn in_parent(parent: &std::path::Path) -> Self {
        let owner = tempfile::tempdir_in(parent).unwrap();
        let data = owner.path().join("data");
        let vault = owner.path().join("vault");
        let credentials = owner.path().join("task.credentials");
        fs::create_dir(&data).unwrap();
        fs::create_dir(&vault).unwrap();
        fs::create_dir(vault.join("archive")).unwrap();
        let target = format!("---\nbrn_id: {TARGET_ID}\nbrn_kind: source\n---\nExact source õ\r\n");
        fs::write(vault.join(CURRENT), ORIGINAL).unwrap();
        fs::write(vault.join(TARGET), &target).unwrap();
        Self {
            _owner: owner,
            data,
            vault,
            credentials,
            target,
        }
    }
    fn config(&self) -> AppConfig {
        AppConfig {
            vault_root: Some(self.vault.clone()),
            credentials_dir: Some(self.credentials.clone()),
            model_dir: None,
        }
    }
    fn app(&self) -> App {
        App::open(&self.data, self.config()).unwrap()
    }
    fn request(&self) -> LinkRequest {
        LinkRequest {
            path: CURRENT.into(),
            target_note_id: TARGET_ID.parse().unwrap(),
            expected_target_sha256: Sha256::digest(self.target.as_bytes()).into(),
            proposal_id: Uuid::new_v4(),
            title: "Link exact source".into(),
            label: "Õ [õ] *source* \\ &".into(),
        }
    }
    fn unchanged(&self) {
        assert_eq!(
            fs::read(self.vault.join(CURRENT)).unwrap(),
            ORIGINAL.as_bytes()
        );
        assert_eq!(
            fs::read(self.vault.join(TARGET)).unwrap(),
            self.target.as_bytes()
        );
    }
}

#[test]
#[ignore = "requires an explicitly owned case-sensitive APFS fixture parent"]
fn distinct_case_sensitive_uuid_aliases_survive_the_after_state_overlay() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let parent = PathBuf::from(
        std::env::var_os("BRN_LINK_CASE_FIXTURE_PARENT")
            .expect("explicit synthetic case-sensitive fixture parent"),
    )
    .canonicalize()
    .unwrap();
    let metadata = fs::metadata(&parent).unwrap();
    assert_eq!(metadata.permissions().mode() & 0o777, 0o700);
    // SAFETY: geteuid has no arguments or memory effects.
    assert_eq!(metadata.uid(), unsafe { libc::geteuid() });
    for replace in [true, false] {
        let f = Fixture::in_parent(&parent);
        fs::rename(f.vault.join(TARGET), f.vault.join("TARGET.md")).unwrap();
        fs::write(f.vault.join("case-probe"), "lower").unwrap();
        fs::write(f.vault.join("CASE-PROBE"), "upper").unwrap();
        assert_ne!(
            fs::metadata(f.vault.join("case-probe")).unwrap().ino(),
            fs::metadata(f.vault.join("CASE-PROBE")).unwrap().ino(),
            "requires a case-sensitive filesystem"
        );
        if replace {
            fs::write(f.vault.join("target.md"), &f.target).unwrap();
        }
        let mut app = f.app();
        let mut draft = raw(
            &mut app,
            format!("{ORIGINAL}\n[new](brn://note/{TARGET_ID})\n"),
        );
        let after = format!("{}\nExact reviewed target", f.target);
        draft.changes.push(if replace {
            DraftNoteChange::Replace {
                path: "target.md".into(),
                expected: app.proposal_source("target.md").unwrap().source.fingerprint,
                text: after,
            }
        } else {
            DraftNoteChange::Create {
                path: "target.md".into(),
                text: after,
            }
        });
        let record = app.create_proposal(&draft).unwrap();
        refused(&mut app, &record);
        assert_eq!(
            fs::read(f.vault.join(CURRENT)).unwrap(),
            ORIGINAL.as_bytes()
        );
        assert_eq!(
            fs::read_to_string(f.vault.join("TARGET.md")).unwrap(),
            f.target
        );
        if replace {
            assert_eq!(
                fs::read_to_string(f.vault.join("target.md")).unwrap(),
                f.target
            );
        } else {
            assert!(!f.vault.join("target.md").exists());
        }
    }
}
fn approve(app: &mut App, record: &ProposalRecord) -> ApprovalRequest {
    let request = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: record.stamp(),
    };
    assert_eq!(
        app.approve_proposal(&request).unwrap().outcome,
        ApplyOutcome::Applied
    );
    request
}
fn raw(app: &mut App, text: String) -> DraftRequest {
    let source = app.proposal_source(CURRENT).unwrap();
    DraftRequest {
        intake: None,
        inbox_visual: None,
        inbox_knowledge: None,
        inbox_source: None,
        action_changes: Vec::new(),
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        title: "Complete ordinary draft".into(),
        changes: vec![DraftNoteChange::Replace {
            path: CURRENT.into(),
            expected: source.source.fingerprint,
            text,
        }],
        sources: vec![],
    }
}
fn refused(app: &mut App, record: &ProposalRecord) {
    let request = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: record.stamp(),
    };
    assert!(app.approve_proposal(&request).is_err());
    assert!(
        app.work_store()
            .proposal_apply(request.operation_id)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        app.proposal(record.draft.id).unwrap().state,
        ProposalState::Draft
    );
}

#[test]
fn preparation_preserves_exact_prefix_and_captures_both_full_sources_without_admission() {
    let f = Fixture::new();
    let mut app = f.app();
    let request = f.request();
    let draft = app.prepare_note_link(&request).unwrap();
    assert_eq!(draft, app.prepare_note_link(&request).unwrap());
    assert_eq!(draft.id, request.proposal_id);
    assert_eq!(draft.title, request.title);
    assert_eq!(draft.sources.len(), 2);
    assert_eq!(
        draft.sources[0],
        app.proposal_source(CURRENT).unwrap().source
    );
    let captured = app
        .capture_citation(&brn_workflow::knowledge::CitationRequest {
            note_id: request.target_note_id,
            expected_sha256: request.expected_target_sha256,
            start_byte: f.target.find("Exact").unwrap(),
            end_byte: f.target.find("Exact").unwrap() + 5,
        })
        .unwrap();
    assert_eq!(draft.sources[1], captured.source);
    let DraftNoteChange::Replace { text, .. } = &draft.changes[0] else {
        panic!("Replace")
    };
    assert_eq!(
        text,
        &format!("{ORIGINAL}\r\n\r\n[Õ \\[õ\\] \\*source\\* \\\\ \\&](brn://note/{TARGET_ID})\r\n")
    );
    f.unchanged();
    assert!(app.work_store().proposals(None).unwrap().is_empty());
    assert!(app.work_store().editors().unwrap().is_empty());
    assert_eq!(fs::read_dir(&f.credentials).unwrap().count(), 0);
    let record = app.create_proposal(&draft).unwrap();
    f.unchanged();
    approve(&mut app, &record);
    assert_eq!(
        app.note_links(CURRENT).unwrap().links[0].outcome,
        NoteLinkOutcome::Resolved
    );
    assert!(app.prepare_note_link(&request).is_err());
}

#[test]
fn preparation_refuses_existing_path_links_self_stale_aliases_and_hidden_markdown() {
    let f = Fixture::new();
    let mut app = f.app();
    let mut request = f.request();
    request.expected_target_sha256[0] ^= 1;
    assert_eq!(
        app.prepare_note_link(&request).unwrap_err().kind,
        ErrorKind::ContextStale
    );
    request = f.request();
    fs::copy(f.vault.join(TARGET), f.vault.join("alias.md")).unwrap();
    assert!(app.prepare_note_link(&request).is_err());
    fs::remove_file(f.vault.join("alias.md")).unwrap();
    fs::copy(f.vault.join(CURRENT), f.vault.join("alias.md")).unwrap();
    assert!(app.prepare_note_link(&request).is_err());
    fs::remove_file(f.vault.join("alias.md")).unwrap();
    request.target_note_id = CURRENT_ID.parse().unwrap();
    request.expected_target_sha256 = Sha256::digest(ORIGINAL.as_bytes()).into();
    assert!(app.prepare_note_link(&request).is_err());
    for body in [
        "[already](archive/source.md)",
        "```\nunfinished",
        "<!-- unfinished",
    ] {
        fs::write(f.vault.join(CURRENT), note(CURRENT_ID, body)).unwrap();
        assert!(app.prepare_note_link(&f.request()).is_err(), "{body}");
    }
    assert!(app.work_store().proposals(None).unwrap().is_empty());
}

#[test]
fn preparation_requires_managed_current_consumer_eligible_target_and_complete_inspection() {
    let f = Fixture::new();
    let mut app = f.app();
    for text in [
        "unmanaged".into(),
        note(CURRENT_ID, "body").replace("---\nbody", "brn_kind: source\n---\nbody"),
        ORIGINAL.replace("custom: untouched", "brn_state: history"),
        ORIGINAL.replace("custom: untouched", "brn_kind: invalid"),
    ] {
        fs::write(f.vault.join(CURRENT), text).unwrap();
        assert!(app.prepare_note_link(&f.request()).is_err());
    }
    fs::write(f.vault.join(CURRENT), ORIGINAL).unwrap();
    fs::write(f.vault.join("unknown.md"), "---\nbrn_id: broken\n---\nbody").unwrap();
    assert!(app.prepare_note_link(&f.request()).is_err());
    fs::remove_file(f.vault.join("unknown.md")).unwrap();
    let malformed = f.target.replace("brn_kind: source", "brn_kind: invalid");
    fs::write(f.vault.join(TARGET), &malformed).unwrap();
    let mut request = f.request();
    request.expected_target_sha256 = Sha256::digest(malformed.as_bytes()).into();
    assert!(app.prepare_note_link(&request).is_err());
}

#[test]
fn fresh_approval_refuses_uncaptured_new_stable_targets_from_inline_and_reference_links() {
    let f = Fixture::new();
    let mut app = f.app();
    for suffix in [
        format!("\n[new](brn://note/{TARGET_ID})\n"),
        format!("\n[new][õ]\n\n[Õ]: BRN://note/{TARGET_ID}#part\n"),
    ] {
        let draft = raw(&mut app, format!("{ORIGINAL}{suffix}"));
        let record = app.create_proposal(&draft).unwrap();
        refused(&mut app, &record);
        f.unchanged();
    }
}

#[test]
fn fresh_approval_rechecks_target_aliases_removal_and_same_size_content_changes() {
    for mutation in ["alias", "content", "removed"] {
        let f = Fixture::new();
        let mut app = f.app();
        let draft = app.prepare_note_link(&f.request()).unwrap();
        let record = app.create_proposal(&draft).unwrap();
        match mutation {
            "alias" => {
                fs::copy(f.vault.join(TARGET), f.vault.join("alias.md")).unwrap();
            }
            "content" => {
                fs::write(f.vault.join(TARGET), f.target.replace("Exact", "Other")).unwrap();
            }
            _ => {
                fs::remove_file(f.vault.join(TARGET)).unwrap();
            }
        }
        refused(&mut app, &record);
        assert_eq!(
            fs::read(f.vault.join(CURRENT)).unwrap(),
            ORIGINAL.as_bytes()
        );
    }
}

#[test]
fn review_edits_and_rewrite_cannot_redirect_to_an_uncaptured_uuid() {
    for rewrite in [false, true] {
        let f = Fixture::new();
        fs::write(f.vault.join("other.md"), note(OTHER_ID, "Other")).unwrap();
        let mut app = f.app();
        let draft = app.prepare_note_link(&f.request()).unwrap();
        let record = app.create_proposal(&draft).unwrap();
        let after = record.draft.changes[0]
            .text()
            .unwrap()
            .replace(TARGET_ID, OTHER_ID);
        let edit = ProposalEdit {
            action_data: Vec::new(),
            expected: record.stamp(),
            title: record.draft.title,
            texts: vec![Some(after)],
        };
        let edited = if rewrite {
            app.rewrite_proposal(&edit)
        } else {
            app.edit_proposal(&edit)
        }
        .unwrap();
        refused(&mut app, &edited);
        f.unchanged();
    }
}

#[test]
fn same_draft_target_replacement_uses_proven_filesystem_namespace_aliases() {
    let f = Fixture::new();
    fs::rename(f.vault.join(TARGET), f.vault.join("target.md")).unwrap();
    let mut app = f.app();
    let target = app.proposal_source("TARGET.md").unwrap();
    let mut draft = raw(
        &mut app,
        format!("{ORIGINAL}\n[new](brn://note/{TARGET_ID})\n"),
    );
    draft.changes.push(DraftNoteChange::Replace {
        path: "TARGET.md".into(),
        expected: target.source.fingerprint,
        text: format!("{}\nExact reviewed replacement", target.text),
    });
    let record = app.create_proposal(&draft).unwrap();
    approve(&mut app, &record);
    assert_eq!(
        app.note_links(CURRENT).unwrap().links[0].outcome,
        NoteLinkOutcome::Resolved
    );
    assert!(
        fs::read_to_string(f.vault.join("target.md"))
            .unwrap()
            .ends_with("Exact reviewed replacement")
    );
}

#[test]
fn unrelated_edits_retain_more_than_public_link_output_limits() {
    let f = Fixture::new();
    let old = format!(
        "{ORIGINAL}\n{}",
        format!("[historical](brn://note/{OTHER_ID})\n").repeat(4100)
    );
    fs::write(f.vault.join(CURRENT), &old).unwrap();
    let mut app = f.app();
    let draft = raw(&mut app, format!("{old}\nUnrelated approved text"));
    let record = app.create_proposal(&draft).unwrap();
    approve(&mut app, &record);
    assert!(
        fs::read_to_string(f.vault.join(CURRENT))
            .unwrap()
            .ends_with("Unrelated approved text")
    );
}

#[test]
fn opaque_legacy_layout_body_edits_are_preserved_but_new_uuid_links_cannot_bypass_proofs() {
    let f = Fixture::new();
    let legacy = "---\n{custom: [opaque, values]}\n---\nlegacy words\n";
    fs::write(f.vault.join(CURRENT), legacy).unwrap();
    let mut app = f.app();
    let draft = raw(&mut app, format!("{legacy}Explicit body edit\n"));
    let record = app.create_proposal(&draft).unwrap();
    approve(&mut app, &record);
    assert_eq!(
        fs::read_to_string(f.vault.join(CURRENT)).unwrap(),
        format!("{legacy}Explicit body edit\n")
    );
    let draft = raw(
        &mut app,
        format!("{legacy}\n[new](brn://note/{TARGET_ID})\n"),
    );
    let record = app.create_proposal(&draft).unwrap();
    refused(&mut app, &record);
    let historical = format!("{legacy}\n[historical](brn://note/{OTHER_ID})\n");
    fs::write(f.vault.join(CURRENT), &historical).unwrap();
    let draft = raw(
        &mut app,
        format!("{historical}Retain historical relationship\n"),
    );
    let record = app.create_proposal(&draft).unwrap();
    approve(&mut app, &record);
    assert!(
        fs::read_to_string(f.vault.join(CURRENT))
            .unwrap()
            .ends_with("Retain historical relationship\n")
    );
}

#[test]
fn opaque_metadata_uuid_never_supplies_historical_body_link_authority() {
    let f = Fixture::new();
    let legacy = format!(
        "---\n{{custom: \"[opaque](brn://note/{TARGET_ID})\"}}\n---\nOriginal body without links\n"
    );
    fs::write(f.vault.join(CURRENT), &legacy).unwrap();
    let mut app = f.app();
    let draft = raw(
        &mut app,
        note(
            CURRENT_ID,
            &format!("Repaired header\n[new](brn://note/{TARGET_ID})\n"),
        ),
    );
    let record = app.create_proposal(&draft).unwrap();
    refused(&mut app, &record);
    assert_eq!(fs::read_to_string(f.vault.join(CURRENT)).unwrap(), legacy);
}

#[test]
fn exact_same_draft_creation_and_replacement_can_supply_new_targets() {
    for existing in [false, true] {
        let f = Fixture::new();
        if existing {
            fs::write(f.vault.join("new.md"), "Unmanaged before").unwrap();
        }
        let mut app = f.app();
        let mut draft = raw(
            &mut app,
            format!("{ORIGINAL}\n[new](brn://note/{OTHER_ID})\n"),
        );
        let text = note(OTHER_ID, "Exact reviewed target");
        let target = if existing {
            DraftNoteChange::Replace {
                path: "new.md".into(),
                expected: app.proposal_source("new.md").unwrap().source.fingerprint,
                text: text.clone(),
            }
        } else {
            DraftNoteChange::Create {
                path: "new.md".into(),
                text: text.clone(),
            }
        };
        draft.changes.push(target);
        let record = app.create_proposal(&draft).unwrap();
        approve(&mut app, &record);
        assert_eq!(fs::read_to_string(f.vault.join("new.md")).unwrap(), text);
        assert_eq!(
            app.note_links(CURRENT).unwrap().links[0].outcome,
            NoteLinkOutcome::Resolved
        );
    }
}

#[test]
fn same_draft_trash_or_duplicate_targets_are_refused_before_any_member_effect() {
    for duplicate in [false, true] {
        let f = Fixture::new();
        // A current target allows a typed Trash/Replace member.
        fs::rename(f.vault.join(TARGET), f.vault.join("target.md")).unwrap();
        let mut app = f.app();
        let capture = app.proposal_source("target.md").unwrap();
        let mut draft = raw(
            &mut app,
            format!("{ORIGINAL}\n[new](brn://note/{TARGET_ID})\n"),
        );
        draft.sources.push(capture.source.clone());
        draft.changes.push(if duplicate {
            DraftNoteChange::Create {
                path: "duplicate.md".into(),
                text: capture.text.clone(),
            }
        } else {
            DraftNoteChange::Trash {
                path: "target.md".into(),
                expected: capture.source.fingerprint,
            }
        });
        let record = app.create_proposal(&draft).unwrap();
        refused(&mut app, &record);
        assert_eq!(
            fs::read(f.vault.join(CURRENT)).unwrap(),
            ORIGINAL.as_bytes()
        );
        assert_eq!(
            fs::read_to_string(f.vault.join("target.md")).unwrap(),
            f.target
        );
        assert!(!f.vault.join("duplicate.md").exists());
    }
}

#[test]
fn historical_links_and_exact_undo_do_not_require_current_targets_or_repeat_completed_effects() {
    let f = Fixture::new();
    let old = format!("{ORIGINAL}\n[old](brn://note/{OTHER_ID})\n");
    fs::write(f.vault.join(CURRENT), &old).unwrap();
    let mut app = f.app();
    let draft = raw(&mut app, ORIGINAL.into());
    let record = app.create_proposal(&draft).unwrap();
    let approval = approve(&mut app, &record);
    let undo = UndoRequest {
        operation_id: Uuid::new_v4(),
        target_operation_id: approval.operation_id,
        trash_member: None,
    };
    assert_eq!(
        app.undo_proposal(&undo).unwrap().outcome,
        ApplyOutcome::Applied
    );
    assert_eq!(fs::read_to_string(f.vault.join(CURRENT)).unwrap(), old);
    let draft = raw(&mut app, format!("{old}\nEdited unrelated text"));
    let record = app.create_proposal(&draft).unwrap();
    let approval = approve(&mut app, &record);
    fs::write(f.vault.join(CURRENT), "Later explicit text").unwrap();
    assert_eq!(
        app.approve_proposal(&approval).unwrap().outcome,
        ApplyOutcome::Applied
    );
    assert_eq!(
        fs::read_to_string(f.vault.join(CURRENT)).unwrap(),
        "Later explicit text"
    );
}

#[test]
fn actual_worker_preparation_survives_restart_index_loss_and_obeys_uncertain_work_fence() {
    let f = Fixture::new();
    let mut expected = None;
    for _ in 0..2 {
        let mut worker = AppWorker::start(f.data.clone(), f.config()).unwrap();
        assert!(matches!(
            worker
                .recv_event_timeout(Duration::from_secs(20))
                .unwrap()
                .1,
            AppEvent::Ready {
                vault_bound: true,
                ..
            }
        ));
        let id = Uuid::new_v4();
        let mut request = f.request();
        request.proposal_id = Uuid::parse_str("44444444-4444-4444-8444-444444444444").unwrap();
        worker
            .submit(id, AppCommand::PrepareNoteLink(request))
            .unwrap();
        let (actual, event) = worker.recv_event_timeout(Duration::from_secs(20)).unwrap();
        assert_eq!(actual, id);
        let AppEvent::NoteLinkDraft(draft) = event else {
            panic!("complete link draft")
        };
        if let Some(expected) = &expected {
            assert_eq!(&*draft, expected);
        }
        expected = Some(*draft);
        worker.shutdown().unwrap();
        fs::remove_file(f.data.join("index.sqlite")).unwrap();
    }
    let mut app = f.app();
    let record = app.create_proposal(expected.as_ref().unwrap()).unwrap();
    app.work_store_mut()
        .begin_proposal_apply(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: record.stamp(),
        })
        .unwrap();
    assert_eq!(
        app.prepare_note_link(&f.request()).unwrap_err().kind,
        ErrorKind::SaveUncertain
    );
    f.unchanged();
}
