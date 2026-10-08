//! Genuine saved Source/PNG inputs over the production owned worker. Providers
//! are replaced only at the existing execution seam; no credentials or calls.
use super::*;
use crate::{
    inbox::CaptureBinaryInboxRequest,
    inbox_actions::InboxAnalysisPurpose,
    proposals::{DraftRequest, NoteChange, SourceVersion},
};
use std::{fs, os::unix::fs::MetadataExt};

const DOCX: &[u8] = include_bytes!("../inbox_processing/fixtures/inline-png.docx");
const PNG: &[u8] = include_bytes!("../inbox_processing/fixtures/inline.png");
const RESULT: &str = r#"{"description":"A tentative illustration õ. <script> [data](bad)","uncertainty":"The image alone does not establish its meaning."}"#;

#[test]
fn visual_inspection_returns_actual_png_and_refuses_forged_or_lost_evidence() {
    let f = Fixture::new();
    let mut w = f.start(Hooks::default());
    let (source, asset) = captured_visual(&w, &f);
    let AppEvent::InboxVisualEvidence(evidence) = reply(
        &w,
        AppCommand::InboxVisualEvidence(source.source.source.path.clone()),
    ) else {
        panic!("complete inspection")
    };
    evidence.validate().unwrap();
    assert_eq!(*evidence.source, source.source);
    assert_eq!(evidence.asset, asset);
    assert_eq!(evidence.bytes, PNG);
    assert_eq!(
        evidence.visual_proof().unwrap().sha256,
        asset.fingerprint.sha256
    );
    let wire = serde_json::to_vec(&evidence).unwrap();
    let roundtrip: crate::inbox_actions::InboxVisualEvidence =
        serde_json::from_slice(&wire).unwrap();
    assert_eq!(roundtrip, *evidence);
    for mutate in 0..4 {
        let mut forged = (*evidence).clone();
        match mutate {
            0 => forged.bytes[20] ^= 1,
            1 => forged.asset.path = "another.png".into(),
            2 => forged.asset.fingerprint.sha256[0] ^= 1,
            _ => forged.source.text.push_str("changed"),
        }
        assert!(forged.validate().is_err());
    }
    fs::write(
        f.base.path().join("vault").join("duplicate.md"),
        &source.raw,
    )
    .unwrap();
    assert!(matches!(
        reply(&w, AppCommand::InboxVisualEvidence("source.md".into())),
        AppEvent::Failed(_)
    ));
    fs::remove_file(f.base.path().join("vault").join("duplicate.md")).unwrap();
    fs::write(f.base.path().join("vault").join(&asset.path), b"not a PNG").unwrap();
    assert!(matches!(
        reply(&w, AppCommand::InboxVisualEvidence("source.md".into())),
        AppEvent::Failed(_)
    ));
    assert_eq!(
        fs::read_to_string(f.base.path().join("vault").join("source.md")).unwrap(),
        source.raw
    );
    w.shutdown().unwrap();
}

fn captured_visual(worker: &AppWorker, owner: &Fixture) -> (SourceFixture, SourceVersion) {
    let capture = CaptureBinaryInboxRequest {
        id: Uuid::new_v4(),
        title: "Synthetic visual document".into(),
        original_name: Some("document.docx".into()),
        bytes: DOCX.to_vec(),
    };
    let AppEvent::InboxCaptured(original) = reply_at(
        worker,
        capture.id,
        AppCommand::CaptureBinaryInbox(capture.clone()),
    ) else {
        panic!("capture")
    };
    capture.validate_receipt(&original).unwrap();
    // Read compatibility uses retained historical bytes, never the new converter.
    let golden: Value = serde_json::from_str(include_str!(
        "../inbox_processing/fixtures/inline-png.legacy.json"
    ))
    .unwrap();
    let body = golden["body"].as_str().unwrap();
    let mut visual = golden["visual"].clone();
    visual.as_object_mut().unwrap().remove("bytes");
    visual["converted_byte_len"] = (body.len() as u64).into();
    visual["converted_sha256"] = serde_json::to_value(brn_intake::digest(body.as_bytes())).unwrap();
    let proof: crate::inbox_processing::InboxSourceVisual = serde_json::from_value(visual).unwrap();
    let note_id = Uuid::new_v4();
    let binding = crate::inbox_processing::InboxSourceBinding {
        extraction: None,
        visual: Some(proof.clone()),
        batch_id: Uuid::new_v4(),
        index: 0,
        original: (*original).clone(),
        format: crate::inbox_processing::InboxConversionFormat::DocxInlinePngV1,
        byte_len: body.len() as u64,
        sha256: brn_intake::digest(body.as_bytes()),
        note_id,
    };
    let asset_path = proof
        .asset_path("source.md", &original.capture.copy.sha256)
        .unwrap();
    fs::write(
        owner.base.path().join("vault/source.md"),
        binding.markdown(body).unwrap(),
    )
    .unwrap();
    fs::write(owner.base.path().join("vault").join(&asset_path), PNG).unwrap();
    let AppEvent::ProposalSource(source) = reply(
        worker,
        AppCommand::ProposalEvidenceSource("source.md".into()),
    ) else {
        panic!("source proof");
    };
    let AppEvent::ProposalAsset(asset) = reply(worker, AppCommand::ProposalAsset(asset_path))
    else {
        panic!("asset proof");
    };

    (
        SourceFixture {
            raw: source.text.clone(),
            source: *source,
            note_id,
            original: *original,
        },
        SourceVersion {
            path: asset.path,
            fingerprint: asset.fingerprint,
        },
    )
}
fn visual_request(source: &SourceFixture, asset: &SourceVersion) -> InboxActionRequest {
    let mut request = request(source);
    request.purpose = InboxAnalysisPurpose::VisualInterpretation;
    request.visual_asset = Some(asset.clone());
    request
}
fn hooks(calls: Arc<AtomicUsize>, result: AiAnswer) -> Hooks {
    Hooks {
        visual: Some(Arc::new(move |ask, image, _, _| {
            calls.fetch_add(1, Ordering::SeqCst);
            assert_eq!(image.png_bytes(), PNG);
            let input: Value =
                serde_json::from_str(ask.question.rsplit_once("\n\n").unwrap().1).unwrap();
            assert_eq!(input["source_path"], "source.md");
            assert!(
                input["source_text"]
                    .as_str()
                    .unwrap()
                    .contains("Exact caption")
            );
            assert!(!ask.question.contains("propose_actions"));
            let result = result.clone();
            Box::pin(async move { result })
        })),
        answer: Some(Arc::new(|_, _, _, _, _| {
            panic!("visual must not use ordinary Ask")
        })),
        proposal_answer: Some(Arc::new(|_, _, _, _, _, _| {
            panic!("visual must not receive proposal tools")
        })),
        ..Hooks::default()
    }
}
fn completed() -> AiAnswer {
    AiAnswer {
        text: RESULT.into(),
        terminal: AiTerminal::Completed,
    }
}
fn prepared(worker: &AppWorker, id: Uuid) -> DraftRequest {
    let AppEvent::InboxVisualDraft(draft) =
        reply(worker, AppCommand::PrepareInboxVisualAnnotation(id))
    else {
        panic!("complete visual draft")
    };
    draft.validate().unwrap();
    *draft
}
fn candidate(draft: &DraftRequest) -> &str {
    let [crate::proposals::DraftNoteChange::Replace { text, .. }] = draft.changes.as_slice() else {
        panic!("one annotation Replace")
    };
    text
}

#[test]
fn visual_owned_worker_uses_real_image_and_requires_separate_exact_annotation_review() {
    let f = Fixture::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let mut w = f.start(hooks(calls.clone(), completed()));
    let (source, asset) = captured_visual(&w, &f);
    let r = visual_request(&source, &asset);
    let turn = analyze(&w, &r).unwrap();
    assert_eq!(turn.status, WorkTurnStatus::Completed);
    assert_eq!(turn.answer, RESULT);
    assert!(analysis(&w, r.id).proposals.is_empty());
    assert_eq!(
        fs::read_to_string(f.base.path().join("vault/source.md")).unwrap(),
        source.source.text
    );
    let draft = prepared(&w, r.id);
    let binding = draft.inbox_visual.as_ref().unwrap();
    assert_eq!(binding.asset, asset);
    assert_eq!(binding.source, source.source.source);
    assert_eq!(candidate(&draft), binding.candidate_text().unwrap());
    assert!(candidate(&draft).contains("Tentative model interpretation"));
    assert!(candidate(&draft).contains("&lt;script&gt;"));
    let AppEvent::Proposal(record) = reply(&w, AppCommand::CreateProposal(draft.clone())) else {
        panic!("annotation review")
    };
    let NoteChange::Replace { text, .. } = &record.draft.changes[0] else {
        panic!("annotation")
    };
    let mut edit = ProposalEdit {
        expected: record.stamp(),
        title: record.draft.title.clone(),
        texts: vec![Some(text.replacen(
            "A tentative illustration",
            "A carefully reviewed illustration",
            1,
        ))],
        action_data: vec![],
    };
    let AppEvent::Proposal(edited) = reply(&w, AppCommand::EditProposal(edit.clone())) else {
        panic!("owner edit")
    };
    assert_eq!(
        fs::read_to_string(f.base.path().join("vault/source.md")).unwrap(),
        source.source.text
    );
    edit.expected = edited.stamp();
    edit.texts[0] = Some(edit.texts[0].as_ref().unwrap().replacen(
        "Exact caption",
        "Invented caption",
        1,
    ));
    assert!(matches!(
        reply(&w, AppCommand::EditProposal(edit)),
        AppEvent::Failed(_)
    ));
    let approval = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: edited.stamp(),
    };
    assert!(
        matches!(reply_at(&w, approval.operation_id, AppCommand::ApproveProposal(approval.clone())),
        AppEvent::ProposalApplied(receipt) if receipt.outcome == ApplyOutcome::Applied)
    );
    let saved = fs::read_to_string(f.base.path().join("vault/source.md")).unwrap();
    let provenance = brn_store::work::inbox_source::read_provenance(&saved).unwrap();
    assert_eq!(
        provenance,
        brn_store::work::inbox_source::read_provenance(&source.source.text).unwrap()
    );
    assert!(saved.contains("A carefully reviewed illustration"));
    assert_eq!(
        fs::read(f.base.path().join("vault").join(&asset.path)).unwrap(),
        PNG
    );
    assert_eq!(
        fs::read(
            source
                .original
                .capture
                .copy
                .directory
                .join(source.original.capture.copy_name())
        )
        .unwrap(),
        DOCX
    );
    assert!(matches!(
        reply(
            &w,
            AppCommand::PreviewInboxRemoval(source.original.capture.id)
        ),
        AppEvent::Failed(_)
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    // Completed analysis, original creation and newer review replay after loss.
    fs::remove_file(f.base.path().join("vault").join(&asset.path)).unwrap();
    let mut replay = r.clone();
    replay.generation += 1;
    assert_eq!(
        serde_json::to_value(analyze(&w, &replay).unwrap()).unwrap(),
        serde_json::to_value(&turn).unwrap()
    );
    let rebuilt = prepared(&w, r.id);
    assert_eq!(rebuilt, draft);
    assert!(
        matches!(reply(&w, AppCommand::CreateProposal(rebuilt)), AppEvent::Proposal(p) if p == edited || p.state == ProposalState::Applied)
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    w.shutdown().unwrap();
    no_credentials(&f);
}

#[test]
fn visual_fresh_source_asset_and_identity_damage_refuse_before_provider_or_reservation() {
    for damage in 0..5 {
        let f = Fixture::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let mut w = f.start(hooks(calls.clone(), completed()));
        let (source, asset) = captured_visual(&w, &f);
        let r = visual_request(&source, &asset);
        let path =
            f.base
                .path()
                .join("vault")
                .join(if damage < 2 { "source.md" } else { &asset.path });
        match damage {
            0 => fs::write(&path, "changed Source").unwrap(),
            1 | 3 => {
                let bytes = fs::read(&path).unwrap();
                let inode = path.metadata().unwrap().ino();
                fs::rename(&path, path.with_extension("kept")).unwrap();
                fs::write(&path, bytes).unwrap();
                assert_ne!(path.metadata().unwrap().ino(), inode);
            }
            2 => fs::write(&path, b"changed image").unwrap(),
            _ => fs::write(
                f.base.path().join("vault/duplicate.md"),
                &source.source.text,
            )
            .unwrap(),
        }
        assert_eq!(analyze(&w, &r).unwrap_err().kind, ErrorKind::ContextStale);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(
            matches!(reply(&w, AppCommand::InboxActionAnalysis(r.id)), AppEvent::Failed(e) if e.kind == ErrorKind::NotFound)
        );
        w.shutdown().unwrap();
        no_credentials(&f);
    }
}

#[test]
fn visual_purpose_and_asset_proof_are_exact_and_do_not_broaden_old_behaviors() {
    let f = Fixture::new();
    let mut w = f.start(Hooks::default());
    let (source, asset) = captured_visual(&w, &f);
    let valid = visual_request(&source, &asset);
    valid.validate().unwrap();
    let mut wrong = valid.clone();
    wrong.visual_asset = None;
    assert!(wrong.validate().is_err());
    wrong = valid.clone();
    wrong.purpose = InboxAnalysisPurpose::Actions;
    assert!(wrong.validate().is_err());
    wrong = valid.clone();
    wrong.visual_asset.as_mut().unwrap().path = "other.png".into();
    assert!(wrong.validate().is_err());
    wrong = valid.clone();
    wrong.visual_asset.as_mut().unwrap().fingerprint.sha256[0] ^= 1;
    assert!(wrong.validate().is_err());
    let old = request(&source);
    old.validate().unwrap();
    assert!(
        serde_json::to_value(old.capture())
            .unwrap()
            .get("visual_asset")
            .is_none()
    );
    w.shutdown().unwrap();
    no_credentials(&f);
}

#[test]
fn visual_failed_interrupted_or_malformed_output_cannot_prepare_a_durable_annotation() {
    for answer in [
        AiAnswer {
            text: RESULT.into(),
            terminal: AiTerminal::Failed(AiError::new(AiErrorKind::ModelRefused)),
        },
        AiAnswer {
            text: RESULT.into(),
            terminal: AiTerminal::Interrupted,
        },
        AiAnswer {
            text: r#"{"description":"partial"}"#.into(),
            terminal: AiTerminal::Completed,
        },
        AiAnswer {
            text: r#"{"description":"x","uncertainty":"u","authority":"approved"}"#.into(),
            terminal: AiTerminal::Completed,
        },
        AiAnswer {
            text: r#"{"description":"","uncertainty":"u"}"#.into(),
            terminal: AiTerminal::Completed,
        },
        AiAnswer {
            text: "x".repeat(16 * 1024 + 1),
            terminal: AiTerminal::Completed,
        },
    ] {
        let f = Fixture::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let mut w = f.start(hooks(calls, answer));
        let (source, asset) = captured_visual(&w, &f);
        let r = visual_request(&source, &asset);
        let turn = analyze(&w, &r).unwrap();
        if turn.status != WorkTurnStatus::Completed {
            assert!(turn.answer.is_empty());
        }
        assert!(matches!(
            reply(&w, AppCommand::PrepareInboxVisualAnnotation(r.id)),
            AppEvent::Failed(_)
        ));
        assert!(analysis(&w, r.id).proposals.is_empty());
        assert_eq!(
            fs::read_to_string(f.base.path().join("vault/source.md")).unwrap(),
            source.source.text
        );
        assert_eq!(
            fs::read(f.base.path().join("vault").join(&asset.path)).unwrap(),
            PNG
        );
        w.shutdown().unwrap();
        no_credentials(&f);
    }
}

#[test]
fn visual_annotation_receipt_recovers_genuine_capture_without_chat_and_fences_provider_reexecution()
{
    let f = Fixture::new();
    let mut w = f.start(hooks(Arc::new(AtomicUsize::new(0)), completed()));
    let (source, asset) = captured_visual(&w, &f);
    let r = visual_request(&source, &asset);
    analyze(&w, &r).unwrap();
    let draft = prepared(&w, r.id);
    let AppEvent::Proposal(record) = reply(&w, AppCommand::CreateProposal(draft)) else {
        panic!("review")
    };
    let approval = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: record.stamp(),
    };
    let AppEvent::ProposalApplied(receipt) = reply_at(
        &w,
        approval.operation_id,
        AppCommand::ApproveProposal(approval.clone()),
    ) else {
        panic!("approval")
    };
    assert_eq!(receipt.outcome, ApplyOutcome::Applied);
    w.shutdown().unwrap();
    let fresh = f.base.path().join("fresh-data");
    fs::create_dir(&fresh).unwrap();
    for entry in fs::read_dir(f.base.path().join("data")).unwrap() {
        let entry = entry.unwrap();
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with(".brn-apply-")
        {
            fs::copy(entry.path(), fresh.join(entry.file_name())).unwrap();
        }
    }
    fs::remove_file(f.base.path().join("vault").join(&asset.path)).unwrap();
    fs::remove_file(
        source
            .original
            .capture
            .copy
            .directory
            .join(source.original.capture.copy_name()),
    )
    .unwrap();
    let mut app = crate::app::App::open(&fresh, f.config()).unwrap();
    let job = app.store.inbox_action(r.id).unwrap().unwrap();
    assert_eq!(job.capture, r.capture());
    assert!(app.store.turn(r.id).unwrap().is_none());
    assert!(matches!(
        app.store.begin_inbox_action_turn(&job),
        Err(brn_store::Error::StateChanged(_))
    ));
    assert_eq!(app.approve_proposal(&approval).unwrap(), receipt);
    assert_eq!(
        app.proposal(record.draft.id).unwrap().state,
        ProposalState::Applied
    );
    no_credentials(&f);
}

#[test]
fn visual_cancellation_drains_the_existing_owned_lane_without_annotation_or_partial_output() {
    let f = Fixture::new();
    let (started, observed) = mpsc::channel();
    let hook = Arc::new(
        move |_: AskRequest,
              image: brn_ai::VisualImage,
              cancel: CancellationToken,
              _: Arc<dyn Fn(AiEvent) + Send + Sync>| {
            assert_eq!(image.png_bytes(), PNG);
            started.send(()).unwrap();
            Box::pin(async move {
                cancel.cancelled().await;
                completed()
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = AiAnswer> + Send>>
        },
    );
    let mut w = f.start(Hooks {
        visual: Some(hook),
        ..Hooks::default()
    });
    let (source, asset) = captured_visual(&w, &f);
    let r = visual_request(&source, &asset);
    w.submit(r.id, AppCommand::AnalyzeInboxActions(Box::new(r.clone())))
        .unwrap();
    observed.recv_timeout(Duration::from_secs(10)).unwrap();
    w.submit(Uuid::new_v4(), AppCommand::CancelTurn(r.id))
        .unwrap();
    let turn = finish(&w, &r).unwrap();
    assert_eq!(turn.status, WorkTurnStatus::Interrupted);
    assert!(turn.answer.is_empty());
    assert!(matches!(
        reply(&w, AppCommand::PrepareInboxVisualAnnotation(r.id)),
        AppEvent::Failed(_)
    ));
    assert!(analysis(&w, r.id).proposals.is_empty());
    assert_eq!(
        fs::read_to_string(f.base.path().join("vault/source.md")).unwrap(),
        source.source.text
    );
    w.shutdown().unwrap();
    no_credentials(&f);
}

#[test]
fn visual_annotation_late_asset_substitution_refuses_before_installing_model_text() {
    let f = Fixture::new();
    let mut w = f.start(hooks(Arc::new(AtomicUsize::new(0)), completed()));
    let (source, asset) = captured_visual(&w, &f);
    let r = visual_request(&source, &asset);
    analyze(&w, &r).unwrap();
    let draft = prepared(&w, r.id);
    let AppEvent::Proposal(record) = reply(&w, AppCommand::CreateProposal(draft)) else {
        panic!("review")
    };
    w.shutdown().unwrap();
    let mut app = crate::app::App::open(&f.base.path().join("data"), f.config()).unwrap();
    let path = f.base.path().join("vault").join(&asset.path);
    let old_inode = path.metadata().unwrap().ino();
    crate::proposal_apply::APPLY_HOOK.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move |phase, index| {
            if phase == "prepared" && index == 0 {
                fs::rename(&path, path.with_extension("kept")).unwrap();
                fs::write(&path, PNG).unwrap();
                assert_ne!(path.metadata().unwrap().ino(), old_inode);
            }
        }))
    });
    let result = app.approve_proposal(&ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: record.stamp(),
    });
    crate::proposal_apply::APPLY_HOOK.with(|hook| *hook.borrow_mut() = None);
    assert!(result.is_err());
    assert_eq!(
        fs::read_to_string(f.base.path().join("vault/source.md")).unwrap(),
        source.source.text
    );
    assert_eq!(
        fs::read(f.base.path().join("vault").join(&asset.path)).unwrap(),
        PNG
    );
    no_credentials(&f);
}

fn annotation_crash(f: &Fixture, phase: &str) {
    let _guard = crate::SUBPROCESS_FIXTURES.lock().unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "simple_worker_tests::inbox_actions::visual_tests::visual_annotation_crash_child",
            "--ignored",
            "--test-threads=1",
        ])
        .env("BRN_VISUAL_ANNOTATION_TEST_BASE", f.base.path())
        .env("BRN_VISUAL_ANNOTATION_TEST_PHASE", phase)
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(86),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn copy_approval_records(from: &std::path::Path, to: &std::path::Path) {
    fs::create_dir(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with(".brn-apply-")
        {
            fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
        }
    }
}

#[test]
#[ignore = "private subprocess entry exercised by visual annotation recovery"]
fn visual_annotation_crash_child() {
    let base =
        std::path::PathBuf::from(std::env::var_os("BRN_VISUAL_ANNOTATION_TEST_BASE").unwrap());
    let phase = std::env::var("BRN_VISUAL_ANNOTATION_TEST_PHASE").unwrap();
    let mut app = crate::app::App::open(
        &base.join("data"),
        crate::app::AppConfig {
            vault_root: Some(base.join("vault")),
            credentials_dir: Some(base.join("credentials")),
            model_dir: None,
        },
    )
    .unwrap();
    crate::proposal_apply::APPLY_HOOK.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move |step, index| {
            if step == phase && index == 0 {
                std::process::exit(86);
            }
        }));
    });
    let request =
        serde_json::from_slice(&fs::read(base.join("annotation-approval.json")).unwrap()).unwrap();
    app.approve_proposal(&request).unwrap();
    panic!("selected annotation checkpoint was not reached");
}

#[test]
fn visual_annotation_crash_recovery_finish_and_restore_preserve_authority_and_undo_history() {
    use crate::proposal_apply::{RepairDirection, RepairRequest, UndoRequest};
    for direction in [RepairDirection::Finish, RepairDirection::Restore] {
        let f = Fixture::new();
        let mut w = f.start(hooks(Arc::new(AtomicUsize::new(0)), completed()));
        let (source, asset) = captured_visual(&w, &f);
        let r = visual_request(&source, &asset);
        analyze(&w, &r).unwrap();
        let draft = prepared(&w, r.id);
        let proposed = candidate(&draft).to_owned();
        let AppEvent::Proposal(record) = reply(&w, AppCommand::CreateProposal(draft)) else {
            panic!("review")
        };
        w.shutdown().unwrap();
        let approval = ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: record.stamp(),
        };
        fs::write(
            f.base.path().join("annotation-approval.json"),
            serde_json::to_vec(&approval).unwrap(),
        )
        .unwrap();
        annotation_crash(&f, "member");
        assert_eq!(
            fs::read_to_string(f.base.path().join("vault/source.md")).unwrap(),
            proposed
        );
        let image_path = f.base.path().join("vault").join(&asset.path);
        let kept = image_path.with_extension("retained");
        fs::rename(&image_path, &kept).unwrap();
        fs::write(&image_path, PNG).unwrap();
        assert_ne!(
            image_path.metadata().unwrap().ino(),
            asset.fingerprint.inode
        );
        let fresh = f.base.path().join("recovered-data");
        copy_approval_records(&f.base.path().join("data"), &fresh);
        let mut app = crate::app::App::open(&fresh, f.config()).unwrap();
        assert_eq!(
            app.store.inbox_action(r.id).unwrap().unwrap().capture,
            r.capture()
        );
        assert!(app.store.turn(r.id).unwrap().is_none());
        let uncertain = app.reconcile_proposal(approval.operation_id).unwrap();
        assert_eq!(uncertain.outcome, ApplyOutcome::Uncertain);
        let preview = app.preview_proposal_repair(approval.operation_id).unwrap();
        let refused = RepairRequest {
            id: Uuid::new_v4(),
            operation_id: approval.operation_id,
            expected: preview.expected,
            direction: RepairDirection::Finish,
        };
        assert!(app.repair_proposal(&refused).is_err());
        assert_eq!(
            fs::read_to_string(f.base.path().join("vault/source.md")).unwrap(),
            proposed
        );
        if direction == RepairDirection::Finish {
            fs::remove_file(&image_path).unwrap();
            fs::rename(&kept, &image_path).unwrap();
        }
        let repair = RepairRequest {
            id: Uuid::new_v4(),
            operation_id: approval.operation_id,
            expected: preview.expected,
            direction,
        };
        let receipt = app.repair_proposal(&repair).unwrap();
        assert_eq!(
            receipt.outcome,
            Some(if direction == RepairDirection::Finish {
                ApplyOutcome::Applied
            } else {
                ApplyOutcome::NotApplied
            })
        );
        assert_eq!(app.repair_proposal(&repair).unwrap(), receipt);
        let settled = app.approve_proposal(&approval).unwrap();
        fs::remove_file(&image_path).unwrap();
        fs::remove_file(
            source
                .original
                .capture
                .copy
                .directory
                .join(source.original.capture.copy_name()),
        )
        .unwrap();
        if direction == RepairDirection::Finish {
            let undo = UndoRequest {
                operation_id: Uuid::new_v4(),
                target_operation_id: approval.operation_id,
                trash_member: None,
            };
            let undone = app.undo_proposal(&undo).unwrap();
            assert_eq!(undone.outcome, ApplyOutcome::Applied);
            assert_eq!(app.undo_proposal(&undo).unwrap(), undone);
        }
        assert_eq!(
            fs::read_to_string(f.base.path().join("vault/source.md")).unwrap(),
            source.raw
        );
        assert_eq!(app.repair_proposal(&repair).unwrap(), receipt);
        assert_eq!(app.approve_proposal(&approval).unwrap(), settled);
        assert!(
            app.store
                .begin_inbox_action_turn(&app.store.inbox_action(r.id).unwrap().unwrap())
                .is_err()
        );
        drop(app);
        let later = f.base.path().join("later-data");
        copy_approval_records(&fresh, &later);
        let mut app = crate::app::App::open(&later, f.config()).unwrap();
        assert_eq!(app.repair_proposal(&repair).unwrap(), receipt);
        assert_eq!(app.approve_proposal(&approval).unwrap(), settled);
        assert_eq!(
            fs::read_to_string(f.base.path().join("vault/source.md")).unwrap(),
            source.raw
        );
        assert!(app.store.turn(r.id).unwrap().is_none());
        no_credentials(&f);
    }
}
