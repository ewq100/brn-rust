//! Genuine Stored ZIP32 OOXML, observed through the owned Inbox file boundary.
use super::*;
use crate::{
    app::AppConfig,
    inbox::CaptureBinaryInboxRequest,
    proposal_apply::{ApplyOutcome, ApprovalRequest},
    proposals::{DraftNoteChange, DraftRequest},
};
use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::PathBuf,
};

const DOCX: &[u8] = include_bytes!("fixtures/basic-text.docx");
const BODY: &str = "<!-- docx-story: body body -->\n\n<!-- docx-export:0 -->\nFirst õ 日本語\n\n<!-- docx-export:1 -->\nSecond preserved\n";

#[test]
fn docx_wide_numbered_lists_keep_parentage_in_the_actual_markdown_parser() {
    use markdown::mdast::Node;
    // Saved legacy markup remains readable without linking the retired converter.
    let payload: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/wide-lists.legacy.json")).unwrap();
    let body = payload["body"].as_str().unwrap().to_owned();
    let tree = markdown::to_mdast(&body, &markdown::ParseOptions::default()).unwrap();
    let [Node::List(parents)] = tree.children().unwrap().as_slice() else {
        panic!("one ordered parent list: {tree:?}")
    };
    assert!(parents.ordered);
    assert_eq!(parents.start, Some(99));
    assert_eq!(parents.children.len(), 2);
    for (parent, wording) in parents.children.iter().zip(["Before", "After"]) {
        let Node::ListItem(parent) = parent else {
            panic!("parent item")
        };
        let [Node::Paragraph(label), Node::List(children)] = parent.children.as_slice() else {
            panic!("child stays inside its parent: {parent:?}")
        };
        assert_eq!(label.children[0].to_string(), wording);
        assert!(children.ordered);
        assert_eq!(children.start, Some(1000));
        let [Node::ListItem(child)] = children.children.as_slice() else {
            panic!("one ordered child")
        };
        let [Node::Paragraph(label), Node::List(grandchildren)] = child.children.as_slice() else {
            panic!("grandchild stays inside its child: {child:?}")
        };
        assert_eq!(label.children[0].to_string(), "Child");
        assert!(!grandchildren.ordered);
        assert_eq!(grandchildren.start, None);
        let [Node::ListItem(grandchild)] = grandchildren.children.as_slice() else {
            panic!("one bullet grandchild")
        };
        let [Node::Paragraph(label)] = grandchild.children.as_slice() else {
            panic!("grandchild wording")
        };
        assert_eq!(label.children[0].to_string(), "Grandchild õ");
    }
}

struct Fixture {
    _owner: tempfile::TempDir,
    data: PathBuf,
    vault: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = owner.path().join("data");
        let vault = owner.path().join("vault");
        fs::create_dir(&data).unwrap();
        fs::create_dir(&vault).unwrap();
        Self {
            _owner: owner,
            data,
            vault,
        }
    }
    fn app(&self) -> App {
        App::open(
            &self.data,
            AppConfig {
                vault_root: Some(self.vault.clone()),
                credentials_dir: None,
                model_dir: None,
            },
        )
        .unwrap()
    }
    fn queue(&self, app: &mut App, bytes: &[u8]) -> (crate::inbox::InboxItem, ProcessInboxRequest) {
        let capture = CaptureBinaryInboxRequest {
            id: Uuid::new_v4(),
            title: "Genuine synthetic DOCX õ".into(),
            // The parser must use actual package bytes rather than this label.
            original_name: Some("label.txt".into()),
            bytes: bytes.to_vec(),
        };
        let item = app.capture_binary_inbox(&capture).unwrap();
        capture.validate_receipt(&item).unwrap();
        let process = ProcessInboxRequest {
            limits: None,

            id: Uuid::new_v4(),
            items: vec![item.clone()],
        };
        app.process_inbox(&process).unwrap();
        (item, process)
    }
    fn prepare(&self, app: &mut App) -> (crate::inbox::InboxItem, DraftRequest) {
        let (item, process) = self.queue(app, DOCX);
        let done = app
            .advance_inbox_processing(process.id, &AtomicBool::new(false))
            .unwrap();
        assert_eq!(
            done.entries[0].outcome,
            InboxProcessOutcome::Converted {
                format: InboxConversionFormat::MaintainedExtractionV1,
                byte_len: BODY.len() as u64,
                sha256: digest(BODY.as_bytes())
            }
        );
        assert_eq!(app.process_inbox(&process).unwrap(), done);
        let candidate = InboxCandidateRequest {
            batch_id: process.id,
            index: 0,
        };
        let preview = app.inbox_candidate(&candidate).unwrap();
        preview.validate_receipt(&done).unwrap();
        assert_eq!(preview.markdown, BODY);
        assert!(preview.needs_semantic_review);
        let request = InboxSourceRequest {
            candidate,
            proposal_id: Uuid::new_v4(),
            note_id: Uuid::new_v4(),
            path: "source.md".into(),
            title: "Review complete DOCX Source".into(),
        };
        let draft = app.prepare_inbox_source(&request).unwrap();
        request.validate_draft(&draft).unwrap();
        (item, draft)
    }
}
fn text(draft: &DraftRequest) -> &str {
    let [DraftNoteChange::Create { text, .. }] = draft.changes.as_slice() else {
        panic!("Source Create")
    };
    text
}
fn original(item: &crate::inbox::InboxItem) -> PathBuf {
    item.capture.copy.directory.join(item.capture.copy_name())
}
fn private_write(path: &std::path::Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

#[test]
fn docx_fresh_conversion_source_approval_preserves_complete_original() {
    let f = Fixture::new();
    let mut app = f.app();
    let (item, draft) = f.prepare(&mut app);
    let path = original(&item);
    let inode = path.metadata().unwrap().ino();
    assert!(text(&draft).ends_with(BODY));
    assert!(!f.vault.join("source.md").exists());
    let record = app.create_proposal(&draft).unwrap();
    let request = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: record.stamp(),
    };
    assert_eq!(
        app.approve_proposal(&request).unwrap().outcome,
        ApplyOutcome::Applied
    );
    assert_eq!(
        fs::read(f.vault.join("source.md")).unwrap(),
        text(&draft).as_bytes()
    );
    assert_eq!(fs::read(&path).unwrap(), DOCX);
    assert_eq!(path.metadata().unwrap().ino(), inode);
    assert_eq!(
        app.note_provenance("source.md").unwrap().inbox_source,
        Some(draft.inbox_source.as_ref().unwrap().provenance())
    );
    assert!(app.preview_inbox_removal(item.capture.id).is_err());
    assert!(
        app.inbox_original_operations(item.capture.id)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn retained_docx_preview_survives_original_damage_but_unfinished_approval_refuses() {
    for damage in 0..5 {
        let f = Fixture::new();
        let mut app = f.app();
        let (item, draft) = f.prepare(&mut app);
        let record = app.create_proposal(&draft).unwrap();
        let path = original(&item);
        match damage {
            0 => fs::remove_file(&path).unwrap(),
            1 => private_write(&path, b"changed exact original"),
            2 => {
                fs::rename(&path, path.with_extension("kept")).unwrap();
                private_write(&path, DOCX);
            }
            3 => fs::hard_link(&path, path.with_extension("alias")).unwrap(),
            _ => fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap(),
        }
        let binding = draft.inbox_source.as_ref().unwrap();
        // Historical evidence is immutable and remains inspectable; new effects
        // still require the original's exact physical identity below.
        assert!(
            app.inbox_candidate(&InboxCandidateRequest {
                batch_id: binding.batch_id,
                index: binding.index,
            })
            .is_ok()
        );
        assert_eq!(
            app.approve_proposal(&ApprovalRequest {
                operation_id: Uuid::new_v4(),
                expected: record.stamp()
            })
            .unwrap_err()
            .kind,
            ErrorKind::ContextStale
        );
        assert!(
            app.create_proposal(&draft).is_ok(),
            "exact historical creation replay remains available"
        );
        assert!(!f.vault.join("source.md").exists());
    }
}

#[test]
fn docx_binary_byte_observation_refuses_midread_same_bytes_new_inode() {
    let f = Fixture::new();
    let mut app = f.app();
    let (item, draft) = f.prepare(&mut app);
    let path = original(&item);
    let before = path.metadata().unwrap().ino();
    crate::files::inbox::OBSERVE_HOOK.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move || {
            fs::rename(&path, path.with_extension("kept")).unwrap();
            private_write(&path, DOCX);
        }));
    });
    assert!(app.create_proposal(&draft).is_err());
    assert_ne!(original(&item).metadata().unwrap().ino(), before);
    assert!(app.proposals(None).unwrap().is_empty());
    assert!(!f.vault.join("source.md").exists());
}

#[test]
fn docx_forged_self_consistent_receipt_and_source_body_are_not_fresh_authority() {
    let f = Fixture::new();
    let mut app = f.app();
    let (item, process) = f.queue(&mut app, DOCX);
    app.work_store_mut()
        .start_inbox_processing(process.id, 0)
        .unwrap();
    let fake = "Invented output with consistent hash\n";
    app.work_store_mut()
        .finish_inbox_processing(
            process.id,
            0,
            InboxProcessOutcome::Converted {
                format: InboxConversionFormat::DocxTextV1,
                byte_len: fake.len() as u64,
                sha256: digest(fake.as_bytes()),
            },
        )
        .unwrap();
    let candidate = InboxCandidateRequest {
        batch_id: process.id,
        index: 0,
    };
    assert_eq!(
        app.inbox_candidate(&candidate).unwrap_err().kind,
        ErrorKind::OperationConflict
    );
    assert!(
        app.prepare_inbox_source(&InboxSourceRequest {
            candidate,
            proposal_id: Uuid::new_v4(),
            note_id: Uuid::new_v4(),
            path: "source.md".into(),
            title: "Forged Source".into()
        })
        .is_err()
    );
    let binding = InboxSourceBinding {
        extraction: None,
        visual: None,
        batch_id: process.id,
        index: 0,
        original: item,
        format: InboxConversionFormat::DocxTextV1,
        byte_len: fake.len() as u64,
        sha256: digest(fake.as_bytes()),
        note_id: Uuid::new_v4(),
    };
    let draft = DraftRequest {
        intake: None,
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        title: "Consistent forged wrapper".into(),
        changes: vec![DraftNoteChange::Create {
            path: "source.md".into(),
            text: binding.markdown(fake).unwrap(),
        }],
        sources: vec![],
        action_changes: vec![],
        inbox_source: Some(Box::new(binding)),
        inbox_visual: None,
        inbox_knowledge: None,
    };
    draft.validate().unwrap();
    assert!(app.create_proposal(&draft).is_err());
    assert!(app.proposals(None).unwrap().is_empty());
    assert!(!f.vault.join("source.md").exists());
}

#[test]
fn retained_extraction_approval_survives_disposable_processing_rows_without_reconversion() {
    let f = Fixture::new();
    let mut app = f.app();
    let (_, draft) = f.prepare(&mut app);
    let record = app.create_proposal(&draft).unwrap();
    let raw = rusqlite::Connection::open(f.data.join("brn.sqlite")).unwrap();
    raw.execute("DELETE FROM inbox_processing", []).unwrap();
    raw.execute("DELETE FROM inbox_items", []).unwrap();
    drop(raw);
    assert_eq!(
        app.approve_proposal(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: record.stamp()
        })
        .unwrap()
        .outcome,
        ApplyOutcome::Applied
    );
    assert_eq!(
        fs::read(f.vault.join("source.md")).unwrap(),
        text(&draft).as_bytes()
    );
}

#[test]
fn docx_terminal_source_recovers_fresh_sql_without_original_or_processing() {
    let f = Fixture::new();
    let mut app = f.app();
    let (item, draft) = f.prepare(&mut app);
    let record = app.create_proposal(&draft).unwrap();
    let request = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: record.stamp(),
    };
    let receipt = app.approve_proposal(&request).unwrap();
    drop(app);
    fs::remove_file(original(&item)).unwrap();
    fs::remove_file(f.data.join("brn.sqlite")).unwrap();
    fs::remove_dir_all(f.data.join("backups")).unwrap();
    let mut app = f.app();
    assert_eq!(app.approve_proposal(&request).unwrap(), receipt);
    assert_eq!(
        app.create_proposal(&draft).unwrap().draft.inbox_source,
        draft.inbox_source
    );
    assert!(
        app.inbox_processing(draft.inbox_source.as_ref().unwrap().batch_id)
            .is_err()
    );
    // A fresh catalog may not adopt a capture whose original is unqualified.
    // Terminal Source history still imports independently of that catalog.
    assert_eq!(
        app.inbox_item(item.capture.id).unwrap_err().kind,
        ErrorKind::NotFound
    );
    assert!(
        app.inbox_items(&crate::inbox::InboxListRequest::default())
            .unwrap()
            .issues
            .iter()
            .any(|issue| issue.item_id == Some(item.capture.id))
    );
    assert!(
        !original(&item).exists(),
        "historical import never resurrects original bytes"
    );
    assert_eq!(
        fs::read(f.vault.join("source.md")).unwrap(),
        text(&draft).as_bytes()
    );
}

#[test]
fn docx_queue_cancel_interrupt_and_original_damage_have_durable_outcomes() {
    let f = Fixture::new();
    let mut app = f.app();
    let (_, cancelled) = f.queue(&mut app, DOCX);
    let done = app
        .advance_inbox_processing(cancelled.id, &AtomicBool::new(true))
        .unwrap();
    assert_eq!(done.entries[0].outcome, InboxProcessOutcome::Cancelled);
    assert_eq!(app.process_inbox(&cancelled).unwrap(), done);
    let (_, interrupted) = f.queue(&mut app, DOCX);
    app.work_store_mut()
        .start_inbox_processing(interrupted.id, 0)
        .unwrap();
    drop(app);
    let mut app = f.app();
    assert_eq!(
        app.inbox_processing(interrupted.id).unwrap().entries[0].outcome,
        InboxProcessOutcome::Interrupted
    );
    for missing in [false, true] {
        let (item, process) = f.queue(&mut app, DOCX);
        if missing {
            fs::remove_file(original(&item)).unwrap();
        } else {
            private_write(&original(&item), b"changed");
        }
        let done = app
            .advance_inbox_processing(process.id, &AtomicBool::new(false))
            .unwrap();
        assert!(
            matches!(&done.entries[0].outcome, InboxProcessOutcome::Failed { code } if code == if missing { "original_missing" } else { "original_changed" })
        );
        assert_eq!(app.process_inbox(&process).unwrap(), done);
    }
}

#[test]
fn docx_original_loss_during_apply_keeps_existing_eligibility_fences() {
    for (phase, expected) in [
        ("prepared", ApplyOutcome::NotApplied),
        ("member", ApplyOutcome::Uncertain),
    ] {
        let f = Fixture::new();
        let mut app = f.app();
        let (item, draft) = f.prepare(&mut app);
        let record = app.create_proposal(&draft).unwrap();
        let request = ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: record.stamp(),
        };
        let path = original(&item);
        crate::proposal_apply::APPLY_HOOK.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(move |step, _| {
                if step == phase {
                    fs::remove_file(&path).unwrap();
                }
            }));
        });
        let result = app.approve_proposal(&request);
        crate::proposal_apply::APPLY_HOOK.with(|hook| *hook.borrow_mut() = None);
        assert!(result.is_err());
        let journal = app
            .work_store()
            .proposal_apply(request.operation_id)
            .unwrap()
            .unwrap();
        assert_eq!(journal.receipt.unwrap().outcome, expected);
        assert_eq!(f.vault.join("source.md").exists(), phase == "member");
        drop(app);
        let mut app = f.app();
        assert_eq!(
            app.reconcile_proposal(request.operation_id)
                .unwrap()
                .outcome,
            expected
        );
    }
}

#[test]
fn docx_malformed_actual_package_failure_is_durable_and_never_has_a_candidate() {
    let f = Fixture::new();
    let mut app = f.app();
    let mut damaged = DOCX.to_vec();
    let start = damaged
        .windows(b"First".len())
        .position(|w| w == b"First")
        .unwrap();
    damaged[start] = b'X'; // Exact capture observes this; ZIP CRC must still refuse it.
    let (item, process) = f.queue(&mut app, &damaged);
    let terminal = app
        .advance_inbox_processing(process.id, &AtomicBool::new(false))
        .unwrap();
    assert!(
        matches!(&terminal.entries[0].outcome, InboxProcessOutcome::Failed { code } if code == "intake_invalid")
    );
    assert!(
        app.inbox_candidate(&InboxCandidateRequest {
            batch_id: process.id,
            index: 0
        })
        .is_err()
    );
    assert_eq!(fs::read(original(&item)).unwrap(), damaged);
    drop(app);
    let mut app = f.app();
    assert_eq!(app.process_inbox(&process).unwrap(), terminal);
    assert!(app.proposals(None).unwrap().is_empty());
}

// Fixed ZIP32 timestamps/order; actual OOXML relationship/drawing and complete
// 1x1 RGBA PNG. Stored and Deflate packages contain identical original wording.
const VISUAL_DOCX: &[u8] = include_bytes!("fixtures/inline-png.docx");
const VISUAL_DEFLATE: &[u8] = include_bytes!("fixtures/inline-png-deflate.docx");
const INLINE_PNG: &[u8] = include_bytes!("fixtures/inline.png");
const ALTERNATE_PNG: &[u8] = include_bytes!("fixtures/alternate-inline.png");
const VISUAL_PATH: &str = "sources/visual.md";

impl Fixture {
    fn prepare_visual(
        &self,
        app: &mut App,
        bytes: &[u8],
    ) -> (crate::inbox::InboxItem, DraftRequest) {
        fs::create_dir_all(self.vault.join("sources")).unwrap();
        let (item, process) = self.queue(app, bytes);
        let terminal = app
            .advance_inbox_processing(process.id, &AtomicBool::new(false))
            .unwrap();
        assert!(matches!(
            terminal.entries[0].outcome,
            InboxProcessOutcome::Converted {
                format: InboxConversionFormat::MaintainedExtractionV1,
                ..
            }
        ));
        assert_eq!(app.process_inbox(&process).unwrap(), terminal);
        let candidate = InboxCandidateRequest {
            batch_id: process.id,
            index: 0,
        };
        let preview = app.inbox_candidate(&candidate).unwrap();
        preview.validate_receipt(&terminal).unwrap();
        assert!(preview.needs_semantic_review);
        let extraction = preview.extraction.as_ref().unwrap();
        assert_eq!(extraction.assets.len(), 1);
        assert_eq!(extraction.occurrences.len(), 1);
        let image_asset = &extraction.assets[0];
        assert_eq!(image_asset.bytes, INLINE_PNG);
        assert_eq!((image_asset.width, image_asset.height), (1, 1));
        let occurrence = &extraction.occurrences[0];
        assert!(occurrence.locator.contains("word/media/picture.png"));
        assert!(preview.markdown[occurrence.start..occurrence.end].starts_with("!["));
        assert!(
            extraction
                .gaps
                .iter()
                .any(|gap| gap.contains("titles unavailable"))
        );
        let request = InboxSourceRequest {
            candidate,
            proposal_id: Uuid::new_v4(),
            note_id: Uuid::new_v4(),
            path: VISUAL_PATH.into(),
            title: "Whole original and visual Source review".into(),
        };
        let draft = app.prepare_inbox_source(&request).unwrap();
        request.validate_draft(&draft).unwrap();
        let binding = draft.inbox_source.as_ref().unwrap();
        assert_eq!(binding.original, item);
        assert!(binding.visual.is_none());
        assert!(binding.extraction.is_some());
        assert_eq!(binding.byte_len, preview.markdown.len() as u64);
        assert_eq!(binding.sha256, digest(preview.markdown.as_bytes()));
        let (text, asset, payload) = visual_members(&draft);
        assert_eq!(text, binding.markdown(&preview.markdown).unwrap());
        assert_eq!(payload, INLINE_PNG);
        assert_eq!(
            asset,
            format!(
                "sources/{}",
                brn_intake::asset_file_name(image_asset).unwrap()
            )
        );
        assert!(!self.vault.join(VISUAL_PATH).exists());
        assert!(!self.vault.join(asset).exists());
        assert_eq!(fs::read(original(&item)).unwrap(), bytes);
        (item, draft)
    }
    fn crash_visual(&self, phase: &str, member: usize, mode: &str) {
        let _guard = crate::SUBPROCESS_FIXTURES.lock().unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "inbox_processing::docx_tests::docx_visual_crash_child",
                "--ignored",
                "--test-threads=1",
            ])
            .env("BRN_DOCX_VISUAL_TEST_BASE", self._owner.path())
            .env("BRN_DOCX_VISUAL_TEST_PHASE", phase)
            .env("BRN_DOCX_VISUAL_TEST_MEMBER", member.to_string())
            .env("BRN_DOCX_VISUAL_TEST_MODE", mode)
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(86),
            "{phase}/{member}/{mode}: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    fn assert_visual(&self, draft: &DraftRequest, installed: bool) {
        let (text, asset, bytes) = visual_members(draft);
        if installed {
            assert_eq!(
                fs::read(self.vault.join(VISUAL_PATH)).unwrap(),
                text.as_bytes()
            );
            assert_eq!(fs::read(self.vault.join(asset)).unwrap(), bytes);
        } else {
            assert!(!self.vault.join(VISUAL_PATH).exists());
            assert!(!self.vault.join(asset).exists());
        }
    }
}
fn visual_members(draft: &DraftRequest) -> (&str, &str, &[u8]) {
    let [
        DraftNoteChange::Create { path, text },
        DraftNoteChange::CreateAsset { path: asset, bytes },
    ] = draft.changes.as_slice()
    else {
        panic!("one exact Source Create plus one ordinary asset Create");
    };
    assert_eq!(path, VISUAL_PATH);
    (text, asset, bytes)
}
fn visual_approval(app: &mut App, draft: &DraftRequest) -> ApprovalRequest {
    let record = app.create_proposal(draft).unwrap();
    ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: record.stamp(),
    }
}
fn visual_request_file(f: &Fixture, name: &str, request: &impl serde::Serialize) {
    fs::write(
        f._owner.path().join(name),
        serde_json::to_vec(request).unwrap(),
    )
    .unwrap();
}

#[test]
fn docx_visual_source_and_asset_require_whole_exact_approval_and_keep_original_provenance() {
    for package in [VISUAL_DOCX, VISUAL_DEFLATE] {
        let f = Fixture::new();
        let mut app = f.app();
        let (item, draft) = f.prepare_visual(&mut app, package);
        let original_inode = original(&item).metadata().unwrap().ino();
        let approval = visual_approval(&mut app, &draft);
        f.assert_visual(&draft, false);
        let mut stale = approval.clone();
        stale.operation_id = Uuid::new_v4();
        stale.expected.version += 1;
        assert!(app.approve_proposal(&stale).is_err());
        f.assert_visual(&draft, false);
        let receipt = app.approve_proposal(&approval).unwrap();
        assert_eq!(receipt.outcome, ApplyOutcome::Applied);
        assert_eq!(app.approve_proposal(&approval).unwrap(), receipt);
        f.assert_visual(&draft, true);
        let source = app.proposal_evidence_source(VISUAL_PATH).unwrap();
        let asset = app.proposal_asset(visual_members(&draft).1).unwrap();
        assert_eq!(source.text, visual_members(&draft).0);
        assert_eq!(asset.fingerprint.sha256, digest(INLINE_PNG));
        assert_eq!(
            app.note_provenance(VISUAL_PATH).unwrap().inbox_source,
            Some(draft.inbox_source.as_ref().unwrap().provenance())
        );
        assert_eq!(fs::read(original(&item)).unwrap(), package);
        assert_eq!(original(&item).metadata().unwrap().ino(), original_inode);
        assert!(app.preview_inbox_removal(item.capture.id).is_err());
        assert!(
            app.inbox_original_operations(item.capture.id)
                .unwrap()
                .is_empty()
        );
        drop(app);
        let mut app = f.app();
        assert_eq!(app.approve_proposal(&approval).unwrap(), receipt);
        assert_eq!(app.proposal_asset(visual_members(&draft).1).unwrap(), asset);
        assert_eq!(
            app.proposal_evidence_source(VISUAL_PATH).unwrap().source,
            source.source
        );
        f.assert_visual(&draft, true);
    }
}

#[test]
fn snapshot_source_self_consistent_forgery_is_not_retained_authority() {
    for forgery in 0..4 {
        let f = Fixture::new();
        let mut app = f.app();
        let (_, mut draft) = f.prepare_visual(&mut app, VISUAL_DOCX);
        let binding = draft.inbox_source.as_mut().unwrap();
        match forgery {
            0 => binding.extraction.as_mut().unwrap().snapshot_id = Uuid::new_v4(),
            1 => binding.extraction.as_mut().unwrap().snapshot_sha256 = [7; 32],
            2 => {
                binding.extraction.as_mut().unwrap().assets[0].sha256 = digest(ALTERNATE_PNG);
                binding.extraction.as_mut().unwrap().assets[0].byte_len =
                    ALTERNATE_PNG.len() as u64;
                let DraftNoteChange::CreateAsset { bytes, .. } = &mut draft.changes[1] else {
                    panic!("asset")
                };
                *bytes = ALTERNATE_PNG.to_vec();
            }
            _ => {
                let snapshot = app
                    .work_store()
                    .intake_snapshot(binding.extraction.as_ref().unwrap().snapshot_id)
                    .unwrap()
                    .unwrap();
                let body = snapshot.extraction.markdown.replace("First", "Invented");
                binding.byte_len = body.len() as u64;
                binding.sha256 = digest(body.as_bytes());
                let text = binding.markdown(&body).unwrap();
                let DraftNoteChange::Create { text: out, .. } = &mut draft.changes[0] else {
                    panic!("source")
                };
                *out = text;
            }
        }
        // Wrapper metadata is regenerated to make each forgery self-consistent.
        if forgery != 3 {
            let original = app
                .work_store()
                .intake_snapshot_for(binding.batch_id, binding.index)
                .unwrap()
                .unwrap();
            let text = binding.markdown(&original.extraction.markdown).unwrap();
            let DraftNoteChange::Create { text: out, .. } = &mut draft.changes[0] else {
                panic!("source")
            };
            *out = text;
        }
        assert!(app.create_proposal(&draft).is_err());
        assert!(!f.vault.join(VISUAL_PATH).exists());
    }
}

#[test]
fn docx_visual_original_damage_and_asset_occupancy_refuse_before_effects_and_at_late_checks() {
    for damage in 0..5 {
        let f = Fixture::new();
        let mut app = f.app();
        let (item, draft) = f.prepare_visual(&mut app, VISUAL_DOCX);
        let approval = visual_approval(&mut app, &draft);
        let path = original(&item);
        match damage {
            0 => fs::remove_file(&path).unwrap(),
            1 => private_write(&path, b"changed original"),
            2 => {
                fs::rename(&path, path.with_extension("kept")).unwrap();
                private_write(&path, VISUAL_DOCX);
            }
            3 => fs::hard_link(&path, path.with_extension("alias")).unwrap(),
            _ => fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap(),
        }
        assert_eq!(
            app.approve_proposal(&approval).unwrap_err().kind,
            ErrorKind::ContextStale
        );
        f.assert_visual(&draft, false);
        assert!(
            app.create_proposal(&draft).is_ok(),
            "historical draft replay does not re-convert"
        );
    }
    for same_bytes in [false, true] {
        let f = Fixture::new();
        let mut app = f.app();
        let (item, draft) = f.prepare_visual(&mut app, VISUAL_DOCX);
        let approval = visual_approval(&mut app, &draft);
        let source = f.vault.join(VISUAL_PATH);
        let occupant = if same_bytes {
            visual_members(&draft).0.as_bytes()
        } else {
            b"Unrelated Source occupant"
        };
        fs::write(&source, occupant).unwrap();
        let inode = source.metadata().unwrap().ino();
        assert!(app.approve_proposal(&approval).is_err());
        assert_eq!(fs::read(&source).unwrap(), occupant);
        assert_eq!(source.metadata().unwrap().ino(), inode);
        assert!(!f.vault.join(visual_members(&draft).1).exists());
        assert_eq!(fs::read(original(&item)).unwrap(), VISUAL_DOCX);
    }
    for phase in ["before-admission", "prepared", "member"] {
        let f = Fixture::new();
        let mut app = f.app();
        let (item, draft) = f.prepare_visual(&mut app, VISUAL_DOCX);
        let approval = visual_approval(&mut app, &draft);
        let asset = f.vault.join(visual_members(&draft).1);
        let inode = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        if phase == "before-admission" {
            fs::write(&asset, INLINE_PNG).unwrap(); // Equal bytes still belong to another occupant.
            inode.store(
                asset.metadata().unwrap().ino(),
                std::sync::atomic::Ordering::SeqCst,
            );
        } else {
            let occupant = asset.clone();
            let observed_inode = inode.clone();
            crate::proposal_apply::APPLY_HOOK.with(|hook| {
                *hook.borrow_mut() = Some(Box::new(move |step, member| {
                    if step == phase && member == 0 {
                        fs::write(&occupant, INLINE_PNG).unwrap();
                        observed_inode.store(
                            occupant.metadata().unwrap().ino(),
                            std::sync::atomic::Ordering::SeqCst,
                        );
                    }
                }));
            });
        }
        assert!(app.approve_proposal(&approval).is_err(), "{phase}");
        crate::proposal_apply::APPLY_HOOK.with(|hook| *hook.borrow_mut() = None);
        assert_eq!(fs::read(&asset).unwrap(), INLINE_PNG);
        assert_eq!(
            asset.metadata().unwrap().ino(),
            inode.load(std::sync::atomic::Ordering::SeqCst)
        );
        assert_eq!(f.vault.join(VISUAL_PATH).exists(), phase == "member");
        assert_eq!(fs::read(original(&item)).unwrap(), VISUAL_DOCX);
        if phase != "before-admission" {
            let outcome = app
                .work_store()
                .proposal_apply(approval.operation_id)
                .unwrap()
                .unwrap()
                .receipt
                .unwrap()
                .outcome;
            assert_eq!(
                outcome,
                if phase == "member" {
                    ApplyOutcome::Uncertain
                } else {
                    ApplyOutcome::NotApplied
                }
            );
            drop(app);
            let mut app = f.app();
            assert_eq!(
                app.reconcile_proposal(approval.operation_id)
                    .unwrap()
                    .outcome,
                outcome
            );
            assert_eq!(fs::read(&asset).unwrap(), INLINE_PNG);
            assert_eq!(
                asset.metadata().unwrap().ino(),
                inode.load(std::sync::atomic::Ordering::SeqCst)
            );
        }
    }
    for (phase, member) in [("prepared", 0), ("member", 0), ("member", 1)] {
        let f = Fixture::new();
        let mut app = f.app();
        let (item, draft) = f.prepare_visual(&mut app, VISUAL_DOCX);
        let approval = visual_approval(&mut app, &draft);
        let path = original(&item);
        crate::proposal_apply::APPLY_HOOK.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(move |step, index| {
                if step == phase && index == member {
                    fs::remove_file(&path).unwrap();
                }
            }));
        });
        assert!(app.approve_proposal(&approval).is_err());
        crate::proposal_apply::APPLY_HOOK.with(|hook| *hook.borrow_mut() = None);
        let outcome = app
            .work_store()
            .proposal_apply(approval.operation_id)
            .unwrap()
            .unwrap()
            .receipt
            .unwrap()
            .outcome;
        assert_eq!(
            outcome,
            if phase == "prepared" {
                ApplyOutcome::NotApplied
            } else {
                ApplyOutcome::Uncertain
            }
        );
        assert_eq!(f.vault.join(VISUAL_PATH).exists(), phase == "member");
        assert_eq!(
            f.vault.join(visual_members(&draft).1).exists(),
            phase == "member" && member == 1
        );
    }
}

#[test]
#[ignore = "private subprocess entry exercised by genuine visual Source apply/repair/Undo"]
fn docx_visual_crash_child() {
    let base = PathBuf::from(std::env::var_os("BRN_DOCX_VISUAL_TEST_BASE").unwrap());
    let phase = std::env::var("BRN_DOCX_VISUAL_TEST_PHASE").unwrap();
    let member: usize = std::env::var("BRN_DOCX_VISUAL_TEST_MEMBER")
        .unwrap()
        .parse()
        .unwrap();
    let mut app = App::open(
        &base.join("data"),
        AppConfig {
            vault_root: Some(base.join("vault")),
            credentials_dir: None,
            model_dir: None,
        },
    )
    .unwrap();
    crate::proposal_apply::APPLY_HOOK.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move |step, index| {
            if step == phase && index == member {
                std::process::exit(86);
            }
        }));
    });
    match std::env::var("BRN_DOCX_VISUAL_TEST_MODE").unwrap().as_str() {
        "apply" => {
            let request =
                serde_json::from_slice(&fs::read(base.join("visual-approval.json")).unwrap())
                    .unwrap();
            app.approve_proposal(&request).unwrap();
        }
        "undo" => {
            let request =
                serde_json::from_slice(&fs::read(base.join("visual-undo.json")).unwrap()).unwrap();
            app.undo_proposal(&request).unwrap();
        }
        "repair" => {
            let request =
                serde_json::from_slice(&fs::read(base.join("visual-repair.json")).unwrap())
                    .unwrap();
            app.repair_proposal(&request).unwrap();
        }
        _ => panic!("private fixture mode"),
    }
    panic!("selected real mixed-family checkpoint not reached");
}

#[test]
fn docx_visual_actual_crashes_finish_restore_and_interrupted_undo_keep_whole_endpoints() {
    use crate::proposal_apply::{RepairDirection, RepairRequest, UndoRequest};
    for (phase, member, expected) in [
        ("prepared", 0, ApplyOutcome::NotApplied),
        ("member", 0, ApplyOutcome::Uncertain),
        ("member", 1, ApplyOutcome::Applied),
        ("completion", 0, ApplyOutcome::Applied),
    ] {
        for direction in [RepairDirection::Finish, RepairDirection::Restore] {
            let f = Fixture::new();
            let mut app = f.app();
            let (item, draft) = f.prepare_visual(&mut app, VISUAL_DOCX);
            let approval = visual_approval(&mut app, &draft);
            visual_request_file(&f, "visual-approval.json", &approval);
            drop(app);
            f.crash_visual(phase, member, "apply");
            let mut app = f.app();
            let receipt = app.reconcile_proposal(approval.operation_id).unwrap();
            assert_eq!(receipt.outcome, expected, "{phase}/{member}");
            assert_eq!(app.approve_proposal(&approval).unwrap(), receipt);
            let installed = if expected == ApplyOutcome::Uncertain {
                assert!(app.tools().is_err());
                let preview = app.preview_proposal_repair(approval.operation_id).unwrap();
                let repair = RepairRequest {
                    id: Uuid::new_v4(),
                    operation_id: approval.operation_id,
                    expected: preview.expected,
                    direction,
                };
                let repaired = app.repair_proposal(&repair).unwrap();
                assert_eq!(app.repair_proposal(&repair).unwrap(), repaired);
                direction == RepairDirection::Finish
            } else {
                expected == ApplyOutcome::Applied
            };
            f.assert_visual(&draft, installed);
            assert_eq!(fs::read(original(&item)).unwrap(), VISUAL_DOCX);
            assert!(app.tools().is_ok());
            if installed {
                let undo = UndoRequest {
                    operation_id: Uuid::new_v4(),
                    target_operation_id: approval.operation_id,
                    trash_member: None,
                };
                visual_request_file(&f, "visual-undo.json", &undo);
                drop(app);
                f.crash_visual("member", 0, "undo");
                let mut app = f.app();
                assert_eq!(
                    app.reconcile_proposal(undo.operation_id).unwrap().outcome,
                    ApplyOutcome::Uncertain
                );
                let preview = app.preview_proposal_repair(undo.operation_id).unwrap();
                let repair = RepairRequest {
                    id: Uuid::new_v4(),
                    operation_id: undo.operation_id,
                    expected: preview.expected,
                    direction: RepairDirection::Finish,
                };
                assert_eq!(
                    app.repair_proposal(&repair).unwrap().outcome,
                    Some(ApplyOutcome::Applied)
                );
                f.assert_visual(&draft, false);
                assert_eq!(fs::read(original(&item)).unwrap(), VISUAL_DOCX);
            }
        }
    }
}

#[test]
fn docx_visual_terminal_older_and_fresh_sql_replay_do_not_resurrect_undone_files_or_original() {
    use crate::proposal_apply::UndoRequest;
    for fresh in [false, true] {
        let f = Fixture::new();
        let mut app = f.app();
        let (item, draft) = f.prepare_visual(&mut app, VISUAL_DOCX);
        let approval = visual_approval(&mut app, &draft);
        drop(app);
        let earlier = fs::read(f.data.join("brn.sqlite")).unwrap();
        let mut app = f.app();
        let applied = app.approve_proposal(&approval).unwrap();
        assert_eq!(applied.outcome, ApplyOutcome::Applied);
        let asset = app.proposal_asset(visual_members(&draft).1).unwrap();
        drop(app);
        fs::write(f.data.join("brn.sqlite"), &earlier).unwrap();
        let mut app = f.app();
        assert_eq!(app.approve_proposal(&approval).unwrap(), applied);
        assert_eq!(app.proposal_asset(visual_members(&draft).1).unwrap(), asset);
        f.assert_visual(&draft, true);
        let undo = UndoRequest {
            operation_id: Uuid::new_v4(),
            target_operation_id: approval.operation_id,
            trash_member: None,
        };
        let undone = app.undo_proposal(&undo).unwrap();
        assert_eq!(undone.outcome, ApplyOutcome::Applied);
        f.assert_visual(&draft, false);
        drop(app);
        fs::remove_file(original(&item)).unwrap();
        if fresh {
            fs::remove_file(f.data.join("brn.sqlite")).unwrap();
            fs::remove_dir_all(f.data.join("backups")).unwrap();
        } else {
            fs::write(f.data.join("brn.sqlite"), earlier).unwrap();
        }
        let mut app = f.app();
        assert_eq!(app.approve_proposal(&approval).unwrap(), applied);
        assert_eq!(app.undo_proposal(&undo).unwrap(), undone);
        assert_eq!(
            app.create_proposal(&draft).unwrap().draft.inbox_source,
            draft.inbox_source
        );
        f.assert_visual(&draft, false);
        assert!(!original(&item).exists());
    }
}

#[test]
fn docx_visual_interrupted_finish_and_restore_resume_exact_owned_members() {
    use crate::proposal_apply::{RepairDirection, RepairRequest};
    for direction in [RepairDirection::Finish, RepairDirection::Restore] {
        for phase in ["repair-member", "repair-synced"] {
            let f = Fixture::new();
            let mut app = f.app();
            let (item, draft) = f.prepare_visual(&mut app, VISUAL_DOCX);
            let approval = visual_approval(&mut app, &draft);
            visual_request_file(&f, "visual-approval.json", &approval);
            drop(app);
            f.crash_visual("member", 0, "apply");
            let mut app = f.app();
            assert_eq!(
                app.reconcile_proposal(approval.operation_id)
                    .unwrap()
                    .outcome,
                ApplyOutcome::Uncertain
            );
            let preview = app.preview_proposal_repair(approval.operation_id).unwrap();
            let repair = RepairRequest {
                id: Uuid::new_v4(),
                operation_id: approval.operation_id,
                expected: preview.expected,
                direction,
            };
            visual_request_file(&f, "visual-repair.json", &repair);
            drop(app);
            let member = if phase == "repair-member" && direction == RepairDirection::Restore {
                0
            } else {
                1
            };
            f.crash_visual(phase, member, "repair");
            let mut app = f.app();
            let settled = app.reconcile_proposal(approval.operation_id).unwrap();
            if settled.outcome == ApplyOutcome::Uncertain {
                let preview = app.preview_proposal_repair(approval.operation_id).unwrap();
                let resume = RepairRequest {
                    id: Uuid::new_v4(),
                    operation_id: approval.operation_id,
                    expected: preview.expected,
                    direction,
                };
                let result = app.repair_proposal(&resume).unwrap();
                assert_eq!(app.repair_proposal(&resume).unwrap(), result);
            }
            let installed = direction == RepairDirection::Finish;
            f.assert_visual(&draft, installed);
            assert_eq!(
                app.reconcile_proposal(approval.operation_id)
                    .unwrap()
                    .outcome,
                if installed {
                    ApplyOutcome::Applied
                } else {
                    ApplyOutcome::NotApplied
                }
            );
            assert_eq!(fs::read(original(&item)).unwrap(), VISUAL_DOCX);
            assert!(app.tools().is_ok());
            drop(app);
            let app = f.app();
            f.assert_visual(&draft, installed);
            assert!(app.tools().is_ok());
        }
    }
}
