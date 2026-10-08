//! Client correlation tests. Actual PNG evidence comes from genuine approved DOCX;
//! symbolic retained replies below test presentation, not workflow authority.
use super::*;
use brn_workflow::{
    app::AppConfig,
    app_worker::AppWorker,
    inbox::CaptureBinaryInboxRequest,
    inbox_actions::{InboxActionAnalysis, InboxAnalysisPurpose, InboxVisualEvidence},
    inbox_processing::InboxCandidateRequest,
    proposals::{DraftNoteChange, DraftRequest, ProposalSource},
};
use std::fs;

pub(crate) fn visual_fixture() -> (tempfile::TempDir, InboxVisualEvidence) {
    let (owner, evidence, _) = visual_fixture_with_preview();
    (owner, evidence)
}
pub(crate) fn visual_fixture_with_preview() -> (
    tempfile::TempDir,
    InboxVisualEvidence,
    brn_workflow::inbox_processing::InboxConversionPreview,
) {
    let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    fs::create_dir(owner.path().join("data")).unwrap();
    fs::create_dir(owner.path().join("vault")).unwrap();
    let mut worker = AppWorker::start(
        owner.path().join("data"),
        AppConfig {
            vault_root: Some(owner.path().join("vault")),
            credentials_dir: Some(owner.path().join("credentials")),
            model_dir: None,
        },
    )
    .unwrap();
    assert!(matches!(
        worker
            .recv_event_timeout(Duration::from_secs(10))
            .unwrap()
            .1,
        AppEvent::Ready { .. }
    ));
    let capture = CaptureBinaryInboxRequest {
        id: Uuid::new_v4(),
        title: "Genuine inline PNG".into(),
        original_name: Some("synthetic.docx".into()),
        bytes: include_bytes!("../../brn-workflow/src/inbox_processing/fixtures/inline-png.docx")
            .to_vec(),
    };
    let AppEvent::InboxCaptured(item) =
        reply(&worker, capture.id, AppCommand::CaptureBinaryInbox(capture))
    else {
        panic!("capture")
    };
    // Historical-record witness: seed a canonical previously saved Source and asset.
    // New imports use maintained plural extraction and cannot produce this legacy profile.
    let original = *item;
    let png = include_bytes!("../../brn-workflow/src/inbox_processing/fixtures/inline.png");
    let asset_name = format!(
        "brn-inbox-image-{}-1.png",
        brn_intake::hex(&original.capture.copy.sha256)
    );
    let payload: serde_json::Value = serde_json::from_str(include_str!(
        "../../brn-workflow/src/inbox_processing/fixtures/inline-png.legacy.json"
    ))
    .unwrap();
    let body = payload["body"].as_str().unwrap().to_owned();
    let mut metadata = payload["visual"].clone();
    metadata.as_object_mut().unwrap().remove("bytes");
    metadata["converted_byte_len"] = body.len().into();
    metadata["converted_sha256"] = serde_json::json!(brn_intake::digest(body.as_bytes()));
    let proof: brn_workflow::inbox_processing::InboxSourceVisual =
        serde_json::from_value(metadata).unwrap();
    let binding = brn_workflow::inbox_processing::InboxSourceBinding {
        extraction: None,
        visual: Some(proof.clone()),
        batch_id: Uuid::new_v4(),
        index: 0,
        original: original.clone(),
        format: brn_workflow::inbox_processing::InboxConversionFormat::DocxInlinePngV1,
        byte_len: body.len() as u64,
        sha256: brn_intake::digest(body.as_bytes()),
        note_id: Uuid::new_v4(),
    };
    fs::write(
        owner.path().join("vault/source.md"),
        binding.markdown(&body).unwrap(),
    )
    .unwrap();
    fs::write(owner.path().join("vault").join(&asset_name), png).unwrap();
    let preview = brn_workflow::inbox_processing::InboxConversionPreview {
        extraction: None,
        visual: Some(brn_workflow::inbox_processing::InboxVisualPreview {
            proof,
            bytes: png.to_vec(),
        }),
        request: InboxCandidateRequest {
            batch_id: binding.batch_id,
            index: 0,
        },
        original,
        format: binding.format,
        markdown: body,
        needs_semantic_review: true,
    };
    let AppEvent::InboxVisualEvidence(evidence) = reply(
        &worker,
        Uuid::new_v4(),
        AppCommand::InboxVisualEvidence("source.md".into()),
    ) else {
        panic!("actual PNG")
    };
    worker.shutdown().unwrap();
    let evidence = *evidence;
    evidence.validate().unwrap();
    assert_eq!(
        evidence.bytes,
        include_bytes!("../../brn-workflow/src/inbox_processing/fixtures/inline.png")
    );
    assert_eq!(preview.visual.as_ref().unwrap().bytes, evidence.bytes);
    (owner, evidence, preview)
}
fn reply(worker: &AppWorker, id: Uuid, command: AppCommand) -> AppEvent {
    worker.submit(id, command).unwrap();
    loop {
        let (received, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
        if received == id {
            return event;
        }
    }
}
fn state(source: &ProposalSource) -> AiState {
    let mut state = AiState {
        ready: true,
        vault_bound: true,
        selection: Some(Selection {
            provider: Provider::Chatgpt,
            model: "gpt-6-luna".into(),
        }),
        effort: Some(ReasoningEffort::High),
        generation: 8,
        ..Default::default()
    };
    state.open_inbox().unwrap();
    let (id, _) = state
        .inspect_inbox_analysis_source(source.source.path.clone())
        .unwrap();
    assert!(
        state
            .apply(id, AppEvent::ProposalSource(Box::new(source.clone())))
            .is_empty()
    );
    state
}
fn loaded(evidence: &InboxVisualEvidence) -> AiState {
    let mut state = state(&evidence.source);
    let (id, command) = state.inspect_inbox_visual().unwrap();
    assert!(matches!(command, AppCommand::InboxVisualEvidence(ref path) if path == "source.md"));
    assert!(
        state
            .apply(
                id,
                AppEvent::InboxVisualEvidence(Box::new(evidence.clone()))
            )
            .is_empty()
    );
    assert!(state.inbox_analysis.request.is_none());
    state
}
pub(crate) fn visual_analysis(request: &InboxActionRequest) -> InboxActionAnalysis {
    let mut request_without_asset = request.clone();
    request_without_asset.purpose = InboxAnalysisPurpose::KnowledgeAndActions;
    request_without_asset.visual_asset = None;
    let mut record = super::inbox_analysis_state_tests::symbolic_analysis(&request_without_asset);
    record.job.capture.purpose = request.purpose;
    record.job.capture.visual_asset = request.visual_asset.clone();
    record.turn.as_mut().unwrap().answer = r#"{"description":"Tentative illustration õ","uncertainty":"The 1 × 1 PNG cannot establish details."}"#.into();
    record.job.validate().unwrap();
    record
}
pub(crate) fn annotation(record: &InboxActionAnalysis) -> DraftRequest {
    let capture = &record.job.capture;
    let mut draft: DraftRequest = serde_json::from_value(serde_json::json!({
        "id": Uuid::new_v4(), "group_id": capture.id,
        "session_id": record.turn.as_ref().unwrap().conversation_id,
        "title": "Review tentative visual interpretation", "changes": [], "sources": [capture.source],
        "inbox_visual": {"analysis_id": capture.id, "note_id": capture.note_id().unwrap(),
            "source": capture.source, "source_text": capture.source_text,
            "asset": capture.visual_asset, "description": "Tentative illustration õ",
            "uncertainty": "The 1 × 1 PNG cannot establish details."}
    })).unwrap();
    draft.changes = vec![DraftNoteChange::Replace {
        path: capture.source.as_ref().unwrap().path.clone(),
        expected: capture.source.as_ref().unwrap().fingerprint.clone(),
        text: draft
            .inbox_visual
            .as_ref()
            .unwrap()
            .candidate_text()
            .unwrap(),
    }];
    draft.validate().unwrap();
    draft
}
pub(crate) fn annotation_record(draft: &DraftRequest) -> ProposalRecord {
    let binding = draft.inbox_visual.as_ref().unwrap();
    serde_json::from_value(serde_json::json!({
        "draft": {"id":draft.id,"group_id":draft.group_id,"session_id":draft.session_id,
            "vault":null,"title":draft.title,"inbox_visual":binding,"sources":draft.sources,
            "changes":[{"kind":"replace","path":binding.source.path,
                "parent":{"device":binding.source.fingerprint.device,"inode":1},
                "before":binding.source.fingerprint,"before_text":binding.source_text,
                "text":binding.candidate_text().unwrap()}]},
        "version":1,"state":"draft","comments":[],"created_at_ms":1,"updated_at_ms":1
    }))
    .unwrap()
}
fn completed(state: &mut AiState) -> InboxActionAnalysis {
    let (_, AppCommand::AnalyzeInboxActions(request)) = state.interpret_inbox_visual().unwrap()
    else {
        panic!("explicit interpretation")
    };
    assert_eq!(request.purpose, InboxAnalysisPurpose::VisualInterpretation);
    assert_eq!(
        request.visual_asset.as_ref(),
        state.inbox_analysis.visual.as_ref().map(|v| &v.asset)
    );
    let record = visual_analysis(&request);
    let commands = state.apply(
        request.id,
        AppEvent::Chat(ChatEvent::Finished {
            id: request.id,
            generation: request.generation,
            turn: record.turn.clone().unwrap(),
        }),
    );
    assert!(!state.can_prepare_visual_annotation());
    let (id, _) = commands
        .into_iter()
        .find(|(_, c)| matches!(c, AppCommand::InboxActionAnalysis(_)))
        .unwrap();
    assert!(
        state
            .apply(id, AppEvent::InboxActionAnalysis(Box::new(record.clone())))
            .is_empty()
    );
    record
}
#[test]
fn checked_png_reply_requires_operation_path_full_source_asset_payload_and_current_view() {
    let (_owner, evidence) = visual_fixture();
    let mut state = state(&evidence.source);
    let (id, _) = state.inspect_inbox_visual().unwrap();
    assert!(!state.can_interpret_inbox_visual());
    assert!(
        state
            .apply(
                Uuid::new_v4(),
                AppEvent::InboxVisualEvidence(Box::new(evidence.clone()))
            )
            .is_empty()
    );
    for mode in 0..5 {
        let mut forged = evidence.clone();
        match mode {
            0 => forged.source.source.path = "other.md".into(),
            1 => forged.source.text.push('x'),
            2 => forged.asset.fingerprint.sha256[0] ^= 1,
            3 => forged.bytes[0] ^= 1,
            _ => forged.asset.path = "other.png".into(),
        }
        assert!(
            state
                .apply(id, AppEvent::InboxVisualEvidence(Box::new(forged)))
                .is_empty()
        );
        assert!(state.inbox_analysis.visual.is_none());
        assert!(state.pending.contains_key(&id));
    }
    state
        .inspect_inbox_analysis_source("other.md".into())
        .unwrap();
    state.apply(
        id,
        AppEvent::InboxVisualEvidence(Box::new(evidence.clone())),
    );
    assert!(state.inbox_analysis.visual.is_none());
    assert!(!state.pending.contains_key(&id));
    let mut state = loaded(&evidence);
    let (id, _) = state.inspect_inbox_visual().unwrap();
    state.close_inbox();
    state.open_inbox().unwrap();
    state.apply(id, AppEvent::InboxVisualEvidence(Box::new(evidence)));
    assert!(state.inbox_analysis.visual.is_none());
    assert!(!state.can_interpret_inbox_visual());
}
#[test]
fn completed_visual_requires_separate_prepare_create_and_review_without_automatic_effects() {
    let (owner, evidence) = visual_fixture();
    let mut state = loaded(&evidence);
    let record = completed(&mut state);
    assert!(state.can_prepare_visual_annotation());
    assert!(state.inbox_analysis.annotation.is_none());
    assert!(state.inbox_analysis.annotation_review.is_none());
    let draft = annotation(&record);
    let (id, command) = state.prepare_visual_annotation().unwrap();
    assert!(
        matches!(command, AppCommand::PrepareInboxVisualAnnotation(capture) if capture == record.job.capture.id)
    );
    for mode in 0..4 {
        let mut forged = draft.clone();
        let binding = forged.inbox_visual.as_mut().unwrap();
        match mode {
            0 => binding.analysis_id = Uuid::new_v4(),
            1 => binding.asset.fingerprint.inode += 1,
            2 => binding.description.push_str(" forged"),
            _ => {
                if let DraftNoteChange::Replace { text, .. } = &mut forged.changes[0] {
                    *text = text.replace("Tentative illustration õ", "Owner reviewed alternative");
                }
                forged.validate().unwrap();
            }
        }
        state.apply(id, AppEvent::InboxVisualDraft(Box::new(forged)));
        assert!(state.inbox_analysis.annotation.is_none());
    }
    assert!(
        state
            .apply(id, AppEvent::InboxVisualDraft(Box::new(draft.clone())))
            .is_empty()
    );
    assert_eq!(state.inbox_analysis.annotation.as_ref(), Some(&draft));
    assert!(state.inbox_analysis.annotation_review.is_none());
    let (create_id, command) = state.create_visual_annotation().unwrap();
    assert!(matches!(command, AppCommand::CreateProposal(ref captured) if captured == &draft));
    let proposal = annotation_record(&draft);
    let mut forged = proposal.clone();
    if let brn_workflow::proposals::NoteChange::Replace { text, .. } = &mut forged.draft.changes[0]
    {
        text.push_str("forged");
    }
    state.apply(create_id, AppEvent::Proposal(forged));
    assert!(state.inbox_analysis.annotation_review.is_none());
    let mut newer = proposal.clone();
    newer.version = 2;
    newer.draft.title = "Owner reviewed wording".into();
    if let brn_workflow::proposals::NoteChange::Replace { text, .. } = &mut newer.draft.changes[0] {
        *text = text.replace("Tentative illustration õ", "Owner reviewed alternative");
    }
    assert!(state.apply(create_id, AppEvent::Proposal(newer)).is_empty());
    assert_eq!(state.inbox_analysis.annotation_review, Some(draft.id));
    assert!(state.review.is_none());
    assert!(state.create_visual_annotation().is_none());
    assert_eq!(
        fs::read(owner.path().join("vault/source.md")).unwrap(),
        evidence.source.text.as_bytes()
    );
    assert_eq!(
        fs::read(owner.path().join("vault").join(&evidence.asset.path)).unwrap(),
        evidence.bytes
    );
}
#[test]
fn visual_capture_purpose_asset_and_late_generation_cannot_enable_annotation() {
    let (_owner, evidence) = visual_fixture();
    let mut state = loaded(&evidence);
    let record = completed(&mut state);
    let draft = annotation(&record);
    for mode in 0..4 {
        let mut wrong = record.clone();
        match mode {
            0 => wrong.job.capture.purpose = InboxAnalysisPurpose::KnowledgeAndActions,
            1 => wrong.job.capture.visual_asset = None,
            2 => {
                wrong
                    .job
                    .capture
                    .visual_asset
                    .as_mut()
                    .unwrap()
                    .fingerprint
                    .inode += 1
            }
            _ => wrong.turn.as_mut().unwrap().status = WorkTurnStatus::Interrupted,
        }
        state.inbox_analysis.record = Some(wrong);
        assert!(!state.can_prepare_visual_annotation());
        assert!(state.prepare_visual_annotation().is_none());
    }
    state.inbox_analysis.record = Some(record);
    let (id, _) = state.prepare_visual_annotation().unwrap();
    state.inspect_inbox_analysis(Uuid::new_v4()).unwrap();
    state.apply(id, AppEvent::InboxVisualDraft(Box::new(draft)));
    assert!(state.inbox_analysis.annotation.is_none());
    assert!(!state.pending.contains_key(&id));
}

#[test]
fn retained_visual_reply_requires_exact_purpose_asset_and_capture_before_preparation() {
    let (_owner, evidence) = visual_fixture();
    let mut state = loaded(&evidence);
    let record = completed(&mut state);
    let (id, _) = state.inspect_inbox_analysis(record.job.capture.id).unwrap();
    for mode in 0..4 {
        let mut forged = record.clone();
        match mode {
            0 => forged.job.capture.purpose = InboxAnalysisPurpose::KnowledgeAndActions,
            1 => forged.job.capture.visual_asset = None,
            2 => {
                forged
                    .job
                    .capture
                    .visual_asset
                    .as_mut()
                    .unwrap()
                    .fingerprint
                    .inode += 1
            }
            _ => {
                forged
                    .job
                    .capture
                    .source
                    .as_mut()
                    .unwrap()
                    .fingerprint
                    .inode += 1
            }
        }
        assert!(
            state
                .apply(id, AppEvent::InboxActionAnalysis(Box::new(forged)))
                .is_empty()
        );
        assert!(state.inbox_analysis.record.is_none());
        assert!(state.pending.contains_key(&id));
        assert!(!state.can_prepare_visual_annotation());
    }
    assert!(
        state
            .apply(id, AppEvent::InboxActionAnalysis(Box::new(record)))
            .is_empty()
    );
    assert!(state.can_prepare_visual_annotation());
}
