//! Actual synthetic PresentationML through capture, retained evidence and exact approval.
use super::*;
use crate::{
    app::AppConfig,
    inbox::{CaptureBinaryInboxRequest, InboxItem},
    proposal_apply::{ApplyOutcome, ApprovalRequest},
    proposals::{DraftNoteChange, DraftRequest},
};
use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::PathBuf,
    process::Command,
};

const PPTX: &[u8] = include_bytes!("../../../brn-intake/tests/fixtures/quay.pptx");
const PNG: &[u8] = include_bytes!("../../../brn-intake/tests/fixtures/quay-map.png");
const JPEG: &[u8] = include_bytes!("../../../brn-intake/tests/fixtures/quay-map.jpg");
const SOURCE: &str = "sources/quay.md";

struct Fixture {
    owner: tempfile::TempDir,
    data: PathBuf,
    vault: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = owner.path().join("data");
        let vault = owner.path().join("vault");
        fs::create_dir(&data).unwrap();
        fs::create_dir_all(vault.join("sources")).unwrap();
        Self { owner, data, vault }
    }
    fn app(&self) -> App {
        open(&self.data, &self.vault)
    }
    fn prepare(
        &self,
        app: &mut App,
    ) -> (
        InboxItem,
        ProcessInboxRequest,
        InboxProcessBatch,
        InboxConversionPreview,
        DraftRequest,
    ) {
        let capture = CaptureBinaryInboxRequest {
            id: Uuid::new_v4(),
            title: "Quay authored slide evidence".into(),
            original_name: Some("quay.pptx".into()),
            bytes: PPTX.to_vec(),
        };
        let item = app.capture_binary_inbox(&capture).unwrap();
        capture.validate_receipt(&item).unwrap();
        assert_eq!(app.capture_binary_inbox(&capture).unwrap(), item);
        let process = ProcessInboxRequest {
            id: Uuid::new_v4(),
            limits: None,
            items: vec![item.clone()],
        };
        app.process_inbox(&process).unwrap();
        let terminal = app
            .advance_inbox_processing(process.id, &AtomicBool::new(false))
            .unwrap();
        assert!(
            matches!(
                terminal.entries[0].outcome,
                InboxProcessOutcome::Converted {
                    format: InboxConversionFormat::MaintainedExtractionV1,
                    ..
                }
            ),
            "{:?}",
            terminal.entries[0].outcome
        );
        assert_eq!(app.process_inbox(&process).unwrap(), terminal);
        let candidate = InboxCandidateRequest {
            batch_id: process.id,
            index: 0,
        };
        let preview = app.inbox_candidate(&candidate).unwrap();
        preview.validate_receipt(&terminal).unwrap();
        assert!(preview.needs_semantic_review);
        let request = InboxSourceRequest {
            candidate,
            proposal_id: Uuid::new_v4(),
            note_id: Uuid::new_v4(),
            path: SOURCE.into(),
            title: "Quay slide and presenter-note review".into(),
        };
        let draft = app.prepare_inbox_source(&request).unwrap();
        request.validate_draft(&draft).unwrap();
        assert_eq!(app.prepare_inbox_source(&request).unwrap(), draft);
        (item, process, terminal, preview, draft)
    }
}
fn open(data: &std::path::Path, vault: &std::path::Path) -> App {
    App::open(
        data,
        AppConfig {
            vault_root: Some(vault.into()),
            credentials_dir: Some(data.with_file_name("empty-credentials")),
            model_dir: None,
        },
    )
    .unwrap()
}
fn original(item: &InboxItem) -> PathBuf {
    item.capture.copy.directory.join(item.capture.copy_name())
}
fn assert_installed(vault: &std::path::Path, draft: &DraftRequest, installed: bool) {
    assert_eq!(
        draft.changes.len(),
        3,
        "one whole Source and two unique complete assets"
    );
    for change in &draft.changes {
        let (path, bytes): (&str, &[u8]) = match change {
            DraftNoteChange::Create { path, text } => {
                assert_eq!(path, SOURCE);
                (path, text.as_bytes())
            }
            DraftNoteChange::CreateAsset { path, bytes } => (path, bytes),
            _ => panic!("PPTX Source cannot add other consequences"),
        };
        if installed {
            assert_eq!(fs::read(vault.join(path)).unwrap(), bytes);
        } else {
            assert!(!vault.join(path).exists());
        }
    }
}

#[test]
fn pptx_actual_sources_assets_exact_approval_restart_and_helper_free_replay() {
    let f = Fixture::new();
    let mut app = f.app();
    let (item, process, terminal, preview, draft) = f.prepare(&mut app);
    let inode = original(&item).metadata().unwrap().ino();
    assert_eq!(
        inode, item.capture.copy.file_inode,
        "processing preserves the captured Original inode"
    );
    let extraction = preview.extraction.as_ref().unwrap();
    extraction.validate().unwrap();
    assert_eq!(extraction.original_sha256, digest(PPTX));
    assert_eq!(extraction.sources[0].bytes, PPTX);
    assert_eq!(extraction.sources[0].status, "partial");
    assert!(!extraction.gaps.is_empty());
    let slides: Vec<_> = extraction
        .sources
        .iter()
        .filter(|s| s.media_type.ends_with("presentationml.slide+xml"))
        .collect();
    assert_eq!(slides.len(), 2);
    assert!(slides[0].locator.contains("ppt/slides/slide9.xml"));
    assert!(slides[1].locator.contains("ppt/slides/slide2.xml"));
    assert!(slides[0].name.contains("hidden"));
    // Independent hashes of the exact original ZIP members, not reconstructed XML.
    for (part, len, sha) in [
        (
            "ppt/slides/slide9.xml",
            3473,
            "578e7d2f86c73f1090110d2f3c33339137f31c2de9ff256bbee1ef9643c9fa91",
        ),
        (
            "ppt/slides/slide2.xml",
            1675,
            "005a41358b5572b189fb112d5176bbb81715a23efbef04bcb9a65663bddcd2cc",
        ),
        (
            "ppt/notesSlides/notesSlide1.xml",
            1280,
            "0bd0a68070940c80eb30d480ad4d6ae45b9ce0e75cd9a97b93dbc10916b907db",
        ),
        (
            "ppt/notesSlides/notesSlide2.xml",
            1277,
            "58979bbae22732af3bd8b66f6b8dd44979981a3ee940feb6247e903e2568f5ef",
        ),
    ] {
        let source = extraction
            .sources
            .iter()
            .find(|s| s.locator.ends_with(part))
            .unwrap();
        assert_eq!(source.bytes.len(), len);
        assert_eq!(brn_intake::hex(&digest(&source.bytes)), sha);
        assert_eq!(source.status, "partial");
    }
    let duplicate: Vec<_> = extraction
        .sources
        .iter()
        .filter(|s| s.text.contains("Dock review is pending."))
        .collect();
    assert_eq!(
        duplicate.len(),
        2,
        "repeated wording remains independently attributed"
    );
    assert_ne!(duplicate[0].id, duplicate[1].id);
    assert_eq!(duplicate[0].parent.as_ref(), Some(&slides[0].id));
    assert_eq!(duplicate[1].parent.as_ref(), Some(&slides[1].id));
    assert_eq!(
        preview.markdown.matches("Dock review is pending.").count(),
        2
    );
    let nested = extraction
        .sources
        .iter()
        .find(|s| {
            s.text
                .contains("Inspect pier A\nbefore cargo arrival.\nDisplay date: 14 October")
        })
        .unwrap();
    assert!(nested.name.contains("hidden"));
    let group = extraction
        .sources
        .iter()
        .find(|s| Some(&s.id) == nested.parent.as_ref())
        .unwrap();
    assert_eq!(group.parent.as_ref(), Some(&slides[0].id));
    for (row, values) in [
        (1, ["Pier", "Owner", "State"]),
        (2, ["A", "Leena", "Pending"]),
    ] {
        for (column, value) in values.into_iter().enumerate() {
            let cell = extraction
                .sources
                .iter()
                .find(|s| {
                    s.locator
                        .ends_with(&format!("/table/row={row}/cell={}", column + 1))
                })
                .unwrap();
            assert!(cell.text.lines().any(|line| line == value));
        }
    }
    for (slide, wording) in slides.iter().zip([
        "Notes only: Leena must confirm the crane by 14 October.",
        "Do not treat a berth count as arrival authorization.",
    ]) {
        let notes = extraction
            .sources
            .iter()
            .find(|s| {
                s.media_type.ends_with("presentationml.notesSlide+xml")
                    && s.parent.as_ref() == Some(&slide.id)
            })
            .unwrap();
        assert!(notes.text.contains(wording));
        assert!(notes.name.to_lowercase().contains("notes"));
    }
    assert_eq!(extraction.assets.len(), 2);
    assert!(extraction.assets.iter().any(|a| a.bytes == PNG));
    assert!(extraction.assets.iter().any(|a| a.bytes == JPEG));
    assert_eq!(extraction.occurrences.len(), 3);
    for occurrence in &extraction.occurrences {
        assert!(
            extraction
                .sources
                .iter()
                .any(|s| s.id == occurrence.source_id && s.locator.contains("shape="))
        );
        assert!(
            extraction
                .assets
                .iter()
                .any(|a| a.id == occurrence.asset_id)
        );
        let interval = &preview.markdown[occurrence.start..occurrence.end];
        assert!(interval.trim().starts_with("!["));
        let asset = extraction
            .assets
            .iter()
            .find(|a| a.id == occurrence.asset_id)
            .unwrap();
        assert!(interval.contains(&brn_intake::asset_file_name(asset).unwrap()));
        assert!(occurrence.locator.contains("ppt/slides/"));
    }
    assert_eq!(
        extraction
            .occurrences
            .iter()
            .filter(|o| o.asset_id == extraction.occurrences[0].asset_id)
            .count(),
        2
    );
    let binding = draft.inbox_source.as_ref().unwrap();
    assert_eq!(binding.original, item);
    let proof = binding.extraction.as_ref().unwrap();
    let snapshot = app.retained_intake(proof.snapshot_id).unwrap();
    assert_eq!(snapshot.extraction, *extraction);
    assert_eq!(snapshot.digest().unwrap(), proof.snapshot_sha256);
    assert_eq!(
        app.retained_intakes_for_item(item.capture.id).unwrap(),
        vec![snapshot.clone()]
    );
    let body = extraction
        .materialize_for_source(&binding.note_id.to_string())
        .unwrap();
    assert_eq!(binding.sha256, digest(body.markdown.as_bytes()));
    for asset in &body.assets {
        let path = format!(
            "sources/{}",
            brn_intake::asset_file_name_for_source(asset, &binding.note_id.to_string()).unwrap()
        );
        assert!(draft.changes.iter().any(|c| matches!(c, DraftNoteChange::CreateAsset { path: p, bytes } if p == &path && bytes == &asset.bytes)));
    }
    assert_installed(&f.vault, &draft, false);
    let record = app.create_proposal(&draft).unwrap();
    let approval = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: record.stamp(),
    };
    let mut stale = approval.clone();
    stale.operation_id = Uuid::new_v4();
    stale.expected.version += 1;
    assert!(app.approve_proposal(&stale).is_err());
    assert_installed(&f.vault, &draft, false);
    let receipt = app.approve_proposal(&approval).unwrap();
    assert_eq!(receipt.outcome, ApplyOutcome::Applied);
    assert_eq!(app.approve_proposal(&approval).unwrap(), receipt);
    assert_installed(&f.vault, &draft, true);
    assert_eq!(
        app.note_provenance(SOURCE).unwrap().inbox_source,
        Some(binding.provenance())
    );
    assert!(app.preview_inbox_removal(item.capture.id).is_err());
    assert!(
        app.inbox_original_operations(item.capture.id)
            .unwrap()
            .is_empty()
    );
    assert_eq!(fs::read(original(&item)).unwrap(), PPTX);
    assert_eq!(original(&item).metadata().unwrap().ino(), inode);
    let proof = Replay {
        item,
        process,
        terminal,
        preview,
        draft,
        approval,
        snapshot,
        receipt: serde_json::to_value(receipt).unwrap(),
        inode,
    };
    fs::write(
        f.owner.path().join("replay.json"),
        serde_json::to_vec(&proof).unwrap(),
    )
    .unwrap();
    drop(app);
    // Child-only environment: prove replay/discovery never needs the helper,
    // without racing or modifying the parent's/shared executable environment.
    let _guard = crate::SUBPROCESS_FIXTURES.lock().unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "inbox_processing::pptx_tests::pptx_helper_unavailable_replay_child",
            "--ignored",
            "--test-threads=1",
        ])
        .env("BRN_PPTX_REPLAY_BASE", f.owner.path())
        .env(
            "BRN_INTAKE_HELPER",
            f.owner.path().join("deliberately-unavailable-helper"),
        )
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"),
        "the exact isolated replay witness must run"
    );
}

#[derive(Serialize, Deserialize)]
struct Replay {
    item: InboxItem,
    process: ProcessInboxRequest,
    terminal: InboxProcessBatch,
    preview: InboxConversionPreview,
    draft: DraftRequest,
    approval: ApprovalRequest,
    snapshot: IntakeSnapshot,
    receipt: serde_json::Value,
    inode: u64,
}
#[test]
#[ignore = "isolated child entered by PPTX replay test"]
fn pptx_helper_unavailable_replay_child() {
    let base = PathBuf::from(std::env::var_os("BRN_PPTX_REPLAY_BASE").unwrap());
    let proof: Replay =
        serde_json::from_slice(&fs::read(base.join("replay.json")).unwrap()).unwrap();
    assert!(!PathBuf::from(std::env::var_os("BRN_INTAKE_HELPER").unwrap()).exists());
    let mut app = open(&base.join("data"), &base.join("vault"));
    assert_eq!(app.process_inbox(&proof.process).unwrap(), proof.terminal);
    assert_eq!(
        app.inbox_candidate(
            &proof
                .draft
                .inbox_source
                .as_ref()
                .map(|s| InboxCandidateRequest {
                    batch_id: s.batch_id,
                    index: s.index
                })
                .unwrap()
        )
        .unwrap(),
        proof.preview
    );
    assert_eq!(
        app.retained_intake(proof.snapshot.id).unwrap(),
        proof.snapshot
    );
    assert_eq!(
        app.retained_intakes_for_item(proof.item.capture.id)
            .unwrap(),
        vec![proof.snapshot]
    );
    assert_eq!(
        app.create_proposal(&proof.draft)
            .unwrap()
            .draft
            .inbox_source,
        proof.draft.inbox_source
    );
    assert_eq!(
        serde_json::to_value(app.approve_proposal(&proof.approval).unwrap()).unwrap(),
        proof.receipt
    );
    assert_installed(&base.join("vault"), &proof.draft, true);
    assert_eq!(fs::read(original(&proof.item)).unwrap(), PPTX);
    assert_eq!(original(&proof.item).metadata().unwrap().ino(), proof.inode);
}

#[test]
fn pptx_changed_original_and_same_bytes_replacement_refuse_fresh_approval() {
    for replacement in [false, true] {
        let f = Fixture::new();
        let mut app = f.app();
        let (item, _, terminal, preview, draft) = f.prepare(&mut app);
        let record = app.create_proposal(&draft).unwrap();
        let path = original(&item);
        if replacement {
            fs::rename(&path, path.with_extension("retained-original")).unwrap();
            fs::write(&path, PPTX).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
            assert_eq!(fs::read(&path).unwrap(), PPTX);
            assert_ne!(path.metadata().unwrap().ino(), item.capture.copy.file_inode);
        } else {
            let mut changed = PPTX.to_vec();
            changed[0] ^= 1;
            fs::write(&path, changed).unwrap();
        }
        let candidate = InboxCandidateRequest {
            batch_id: terminal.request.id,
            index: 0,
        };
        assert_eq!(app.inbox_candidate(&candidate).unwrap(), preview);
        let error = app
            .approve_proposal(&ApprovalRequest {
                operation_id: Uuid::new_v4(),
                expected: record.stamp(),
            })
            .unwrap_err();
        assert_eq!(error.kind, ErrorKind::ContextStale);
        assert_installed(&f.vault, &draft, false);
        assert_eq!(app.create_proposal(&draft).unwrap(), record);
        drop(app);
        assert_eq!(f.app().inbox_candidate(&candidate).unwrap(), preview);
    }
}

#[test]
fn pptx_retained_approval_without_queue_then_terminal_mirror_recovery() {
    let f = Fixture::new();
    let mut app = f.app();
    let (item, _, _, preview, draft) = f.prepare(&mut app);
    let record = app.create_proposal(&draft).unwrap();
    let raw = rusqlite::Connection::open(f.data.join("brn.sqlite")).unwrap();
    raw.execute("DELETE FROM inbox_processing", []).unwrap();
    raw.execute("DELETE FROM inbox_items", []).unwrap();
    drop(raw);
    let approval = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: record.stamp(),
    };
    let receipt = app.approve_proposal(&approval).unwrap();
    assert_eq!(receipt.outcome, ApplyOutcome::Applied);
    assert_installed(&f.vault, &draft, true);
    let snapshot_id = draft
        .inbox_source
        .as_ref()
        .unwrap()
        .extraction
        .as_ref()
        .unwrap()
        .snapshot_id;
    let snapshot = app.retained_intake(snapshot_id).unwrap();
    assert_eq!(snapshot.extraction, preview.extraction.unwrap());
    drop(app);
    fs::remove_file(original(&item)).unwrap();
    fs::remove_file(f.data.join("brn.sqlite")).unwrap();
    fs::remove_dir_all(f.data.join("backups")).unwrap();
    let mut app = f.app();
    assert_eq!(app.approve_proposal(&approval).unwrap(), receipt);
    assert_eq!(app.create_proposal(&draft).unwrap().draft, record.draft);
    assert_eq!(app.retained_intake(snapshot_id).unwrap(), snapshot);
    assert_installed(&f.vault, &draft, true);
    assert!(app.inbox_processing(snapshot.batch_id).is_err());
    assert!(
        !original(&item).exists(),
        "recovery never resurrects or invents fresh original authority"
    );
}
