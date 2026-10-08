//! Saved legacy intents exercise real exact-proof Restore without any DOCX converter.
use super::*;
use crate::{app::AppConfig, inbox::CaptureBinaryInboxRequest};
use brn_store::work::{
    inbox_processing::InboxConversionFormat, inbox_source::InboxSourceBinding,
    inbox_visual::InboxSourceVisual, proposals::ProposalDraft,
};
use std::{fs, path::PathBuf, sync::atomic::AtomicBool};
const DOCX: &[u8] = include_bytes!("../inbox_processing/fixtures/inline-png.docx");
const PNG: &[u8] = include_bytes!("../inbox_processing/fixtures/inline.png");
fn app(base: &Path) -> App {
    App::open(
        &base.join("data"),
        AppConfig {
            vault_root: Some(base.join("vault")),
            credentials_dir: None,
            model_dir: None,
        },
    )
    .unwrap()
}
fn fixture() -> tempfile::TempDir {
    let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    fs::create_dir(base.path().join("data")).unwrap();
    fs::create_dir(base.path().join("vault")).unwrap();
    base
}
/// Construct an actual saved V1 draft from retained baseline payload, not parser output.
fn legacy(app: &mut App) -> brn_store::work::proposals::ProposalRecord {
    let original = app
        .capture_binary_inbox(&CaptureBinaryInboxRequest {
            id: Uuid::new_v4(),
            title: "Legacy evidence".into(),
            original_name: Some("legacy.docx".into()),
            bytes: DOCX.to_vec(),
        })
        .unwrap();
    let payload: serde_json::Value = serde_json::from_str(include_str!(
        "../inbox_processing/fixtures/inline-png.legacy.json"
    ))
    .unwrap();
    let body = payload["body"].as_str().unwrap();
    let mut value = payload["visual"].clone();
    value.as_object_mut().unwrap().remove("bytes");
    value["converted_byte_len"] = serde_json::json!(body.len());
    value["converted_sha256"] = serde_json::json!(brn_intake::digest(body.as_bytes()));
    let visual: InboxSourceVisual = serde_json::from_value(value).unwrap();
    let asset = visual
        .asset_path("legacy.md", &original.capture.copy.sha256)
        .unwrap();
    let binding = InboxSourceBinding {
        extraction: None,
        visual: Some(visual),
        batch_id: Uuid::new_v4(),
        index: 0,
        original,
        format: InboxConversionFormat::DocxInlinePngV1,
        byte_len: body.len() as u64,
        sha256: brn_intake::digest(body.as_bytes()),
        note_id: Uuid::new_v4(),
    };
    let text = binding.markdown(body).unwrap();
    app.editor_files().unwrap();
    let vault: VaultRecord =
        serde_json::from_str(&app.store.setting("vault.editor_identity").unwrap().unwrap())
            .unwrap();
    let parent = app
        .editor
        .files
        .as_ref()
        .unwrap()
        .parent_identity(Path::new("legacy.md"))
        .unwrap();
    let draft = ProposalDraft {
        intake: None,
        inbox_source: Some(Box::new(binding)),
        inbox_visual: None,
        inbox_knowledge: None,
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        vault: Some(vault),
        title: "Retained legacy review".into(),
        changes: vec![
            NoteChange::Create {
                path: "legacy.md".into(),
                parent: parent.clone(),
                text,
            },
            NoteChange::CreateAsset {
                path: asset,
                parent,
                bytes: PNG.to_vec(),
            },
        ],
        sources: vec![],
        action_changes: vec![],
    };
    // This simulates loading work created by the old executable. New workflow admission refuses it.
    app.store.create_proposal(&draft).unwrap()
}
#[test]
fn legacy_unapproved_draft_requires_new_extraction_and_renewed_review() {
    let base = fixture();
    let mut app = app(base.path());
    let record = legacy(&mut app);
    let approval = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: record.stamp(),
    };
    assert!(
        app.approve_proposal(&approval)
            .unwrap_err()
            .message
            .contains("Legacy DOCX")
    );
    assert_eq!(app.proposal(record.draft.id).unwrap(), record);
    assert!(
        app.store
            .proposal_apply(approval.operation_id)
            .unwrap()
            .is_none()
    );
    let old = record.draft.inbox_source.as_ref().unwrap();
    let process = crate::inbox_processing::ProcessInboxRequest {
        limits: None,
        id: Uuid::new_v4(),
        items: vec![old.original.clone()],
    };
    app.process_inbox(&process).unwrap();
    app.advance_inbox_processing(process.id, &AtomicBool::new(false))
        .unwrap();
    let fresh = app
        .prepare_inbox_source(&crate::inbox_processing::InboxSourceRequest {
            candidate: crate::inbox_processing::InboxCandidateRequest {
                batch_id: process.id,
                index: 0,
            },
            proposal_id: Uuid::new_v4(),
            note_id: Uuid::new_v4(),
            path: "renewed.md".into(),
            title: "Renewed extraction review".into(),
        })
        .unwrap();
    assert!(fresh.inbox_source.as_ref().unwrap().extraction.is_some());
    let fresh = app.create_proposal(&fresh).unwrap();
    assert_ne!(fresh.stamp(), record.stamp());
    assert!(!base.path().join("vault/renewed.md").exists());
}
#[test]
#[ignore = "private historical source-only process-crash entry"]
fn legacy_partial_crash_child() {
    let base = PathBuf::from(std::env::var_os("BRN_P2_LEGACY_BASE").unwrap());
    let mut app = app(&base);
    let record = legacy(&mut app);
    let request = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: record.stamp(),
    };
    fs::write(
        base.join("legacy-approval.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    app.application_records().unwrap();
    let mut journal = app.store.begin_proposal_apply(&request).unwrap();
    let mirror = app
        .apply_records
        .as_ref()
        .unwrap()
        .write(&journal, None)
        .unwrap();
    let files = app.editor.files.as_ref().unwrap();
    let mut prepared = Vec::new();
    for (change, member) in journal.approved.draft.changes.iter().zip(&journal.members) {
        let result = match change {
            NoteChange::Create { path, text, .. } => {
                files.prepare_copy(member.id, &member.staging, Path::new(path), text.as_bytes())
            }
            NoteChange::CreateAsset { path, bytes, .. } => {
                files.prepare_asset_copy(member.id, &member.staging, Path::new(path), bytes)
            }
            _ => panic!("legacy fixture members"),
        }
        .unwrap();
        prepared.push(result.fingerprint);
    }
    journal = app
        .store
        .record_proposal_prepared(request.operation_id, &prepared)
        .unwrap();
    app.apply_records
        .as_ref()
        .unwrap()
        .write(&journal, Some(&mirror))
        .unwrap();
    let member = &journal.members[0];
    let change = &journal.approved.draft.changes[0];
    install_member(
        files,
        &PreparedFile {
            relative: member.staging.clone(),
            fingerprint: prepared[0].clone(),
        },
        Path::new(change.path()),
        change,
    )
    .unwrap();
    files.flush_artifact(Path::new(change.path())).unwrap();
    std::process::exit(86);
}
#[test]
fn legacy_source_only_crash_blocks_finish_and_restores_exact_members() {
    for damage in [false, true] {
        let base = fixture();
        let _guard = crate::SUBPROCESS_FIXTURES.lock().unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "proposal_apply::legacy_intake_tests::legacy_partial_crash_child",
                "--ignored",
                "--test-threads=1",
            ])
            .env("BRN_P2_LEGACY_BASE", base.path())
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(86),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let approval: ApprovalRequest =
            serde_json::from_slice(&fs::read(base.path().join("legacy-approval.json")).unwrap())
                .unwrap();
        let mut app = app(base.path());
        assert_eq!(
            app.reconcile_proposal(approval.operation_id)
                .unwrap()
                .outcome,
            ApplyOutcome::Uncertain
        );
        assert!(app.tools().is_err());
        let journal = app
            .store
            .proposal_apply(approval.operation_id)
            .unwrap()
            .unwrap();
        assert!(base.path().join("vault/legacy.md").exists());
        let asset = journal.approved.draft.changes[1].path();
        assert!(!base.path().join("vault").join(asset).exists());
        let preview = app.preview_proposal_repair(approval.operation_id).unwrap();
        let finish = RepairRequest {
            id: Uuid::new_v4(),
            operation_id: approval.operation_id,
            expected: preview.expected,
            direction: RepairDirection::Finish,
        };
        assert!(
            app.repair_proposal(&finish)
                .unwrap_err()
                .message
                .contains("Legacy DOCX")
        );
        if damage {
            fs::remove_file(base.path().join("vault").join(&journal.members[1].staging)).unwrap();
            assert!(app.preview_proposal_repair(approval.operation_id).is_err());
            assert!(base.path().join("vault/legacy.md").exists());
            assert!(app.tools().is_err());
        } else {
            let restore = RepairRequest {
                id: Uuid::new_v4(),
                direction: RepairDirection::Restore,
                ..finish
            };
            assert_eq!(
                app.repair_proposal(&restore).unwrap().outcome,
                Some(ApplyOutcome::NotApplied)
            );
            assert!(!base.path().join("vault/legacy.md").exists());
            assert!(app.tools().is_ok());
            let original = &journal
                .approved
                .draft
                .inbox_source
                .as_ref()
                .unwrap()
                .original;
            assert_eq!(
                fs::read(
                    original
                        .capture
                        .copy
                        .directory
                        .join(original.capture.copy_name())
                )
                .unwrap(),
                DOCX
            );
        }
    }
}
