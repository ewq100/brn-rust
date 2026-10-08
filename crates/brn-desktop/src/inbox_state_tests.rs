use super::*;
use brn_workflow::{
    app::AppConfig,
    app_worker::AppWorker,
    inbox::{CaptureInboxRequest, InboxItem, InboxKind, InboxOriginal},
    inbox_processing::{
        InboxCandidateRequest, InboxProcessBatch, InboxProcessOutcome, InboxSourceRequest,
    },
    proposals::DraftNoteChange,
};
use std::{fs, os::unix::fs::PermissionsExt};

struct Fixture(tempfile::TempDir);
impl Fixture {
    fn new() -> Self {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        fs::set_permissions(owner.path(), fs::Permissions::from_mode(0o700)).unwrap();
        fs::create_dir(owner.path().join("data")).unwrap();
        fs::create_dir(owner.path().join("vault")).unwrap();
        fs::write(
            owner.path().join("vault/untouched.md"),
            "\u{feff}Untouched õ\r\n",
        )
        .unwrap();
        Self(owner)
    }
    fn worker(&self) -> AppWorker {
        let worker = AppWorker::start(
            self.0.path().join("data"),
            AppConfig {
                vault_root: Some(self.0.path().join("vault")),
                credentials_dir: Some(self.0.path().join("credentials")),
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
        worker
    }
    fn unchanged(&self) {
        assert_eq!(
            fs::read(self.0.path().join("vault/untouched.md")).unwrap(),
            "\u{feff}Untouched õ\r\n".as_bytes()
        );
        assert_eq!(
            fs::read_dir(self.0.path().join("vault")).unwrap().count(),
            1
        );
        assert_eq!(
            fs::read_dir(self.0.path().join("credentials"))
                .unwrap()
                .count(),
            0
        );
    }
}
fn state() -> AiState {
    AiState {
        ready: true,
        vault_bound: true,
        ..Default::default()
    }
}
fn request(kind: InboxKind, text: &str) -> CaptureInboxRequest {
    CaptureInboxRequest {
        id: Uuid::new_v4(),
        kind,
        title: "Exact synthetic source õ".into(),
        original_name: Some("synthetic-copy.txt".into()),
        text: text.into(),
    }
}
fn reply(worker: &AppWorker, command: (Uuid, AppCommand)) -> (Uuid, AppEvent) {
    worker.submit(command.0, command.1).unwrap();
    loop {
        let event = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
        if event.0 == command.0 {
            return event;
        }
    }
}
fn settle(worker: &AppWorker, state: &mut AiState, command: (Uuid, AppCommand)) {
    let operation = command.0;
    worker.submit(operation, command.1).unwrap();
    loop {
        let (id, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
        let followups = state.apply(id, event);
        for (next, command) in followups {
            worker.submit(next, command).unwrap();
        }
        if id == operation && !state.pending.contains_key(&operation) {
            break;
        }
    }
}
fn capture(worker: &AppWorker, state: &mut AiState, request: CaptureInboxRequest) -> InboxItem {
    let command = state.capture_inbox(request.clone()).unwrap();
    assert_eq!(command.0, request.id);
    assert!(matches!(&command.1, AppCommand::CaptureInbox(actual) if actual == &request));
    assert!(state.capture_pending());
    settle(worker, state, command);
    assert!(!state.capture_pending());
    let item = state.inbox_queue.capture_result.clone().unwrap();
    request.validate_receipt(&item).unwrap();
    item
}
fn process(worker: &AppWorker, state: &mut AiState, items: Vec<InboxItem>) -> InboxProcessBatch {
    let command = state.process_inbox_items(items.clone()).unwrap();
    let AppCommand::ProcessInbox(request) = &command.1 else {
        panic!("process command");
    };
    assert_eq!(command.0, request.id);
    assert_eq!(request.items, items);
    assert!(state.processing_pending());
    settle(worker, state, command);
    assert!(!state.processing_pending());
    let batch = state.inbox_queue.batch.clone().unwrap();
    assert_eq!(batch.pending_count(), 0);
    batch.validate().unwrap();
    batch
}
fn source_request(batch: &InboxProcessBatch) -> InboxSourceRequest {
    InboxSourceRequest {
        candidate: InboxCandidateRequest {
            batch_id: batch.request.id,
            index: 0,
        },
        proposal_id: Uuid::new_v4(),
        note_id: Uuid::new_v4(),
        path: "source.md".into(),
        title: "Review exact original".into(),
    }
}

#[test]
fn successful_capture_updates_inventory_and_selection_without_manual_refresh() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let mut state = state();
    let open = state.open_inbox().unwrap();
    settle(&worker, &mut state, open);
    state.draft = crate::draft::DraftForm::new(None);
    state.draft.as_mut().unwrap().text = "Unfinished owner input õ".into();
    let retained = state.draft.as_ref().unwrap().id;
    let item = capture(
        &worker,
        &mut state,
        request(InboxKind::Text, "Public imported body"),
    );
    while state.inbox_loading() {
        let (id, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
        for (id, command) in state.apply(id, event) {
            worker.submit(id, command).unwrap();
        }
    }
    assert!(
        state
            .inbox_queue
            .page
            .as_ref()
            .unwrap()
            .entries
            .iter()
            .any(|entry| entry.item == item)
    );
    assert_eq!(state.inbox_queue.selected.as_ref().unwrap().item, item);
    assert_eq!(state.draft.as_ref().unwrap().id, retained);
    assert_eq!(
        state.draft.as_ref().unwrap().text,
        "Unfinished owner input õ"
    );
    fixture.unchanged();
    worker.shutdown().unwrap();
}

#[test]
fn actual_worker_capture_progress_preview_and_prepared_review_keep_exact_source() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let mut state = state();
    let exact = "\u{feff}---\r\nbrn_id: invalid\r\nbrn_kind: knowledge\r\n---\r\n# õ 日本語\0\r\n";
    let item = capture(&worker, &mut state, request(InboxKind::Markdown, exact));
    let open = state.open_inbox().unwrap();
    settle(&worker, &mut state, open);
    assert_eq!(
        state.inbox_queue.page.as_ref().unwrap().entries[0].item,
        item
    );
    let selected = state.select_inbox(item.capture.id).unwrap();
    settle(&worker, &mut state, selected);
    assert_eq!(
        state.inbox_queue.selected.as_ref().unwrap().original,
        InboxOriginal::Available { text: exact.into() }
    );
    let command = state.process_inbox_items(vec![item.clone()]).unwrap();
    let id = command.0;
    worker.submit(id, command.1).unwrap();
    let (queued_id, AppEvent::InboxProcessing(queued)) =
        worker.recv_event_timeout(Duration::from_secs(10)).unwrap()
    else {
        panic!("initial batch snapshot");
    };
    assert_eq!(queued_id, id);
    assert!(queued.pending_count() > 0);
    state.apply(id, AppEvent::InboxProcessing(queued));
    assert!(
        state.processing_pending(),
        "progress is not a terminal reply"
    );
    while state.processing_pending() {
        let (next, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
        state.apply(next, event);
    }
    let preview = state.preview_inbox_candidate(0).unwrap();
    settle(&worker, &mut state, preview);
    assert_eq!(state.inbox_queue.preview.as_ref().unwrap().markdown, exact);
    let batch = state.inbox_queue.batch.clone().unwrap();
    let source = source_request(&batch);
    let prepare = state.prepare_inbox_source(source.clone()).unwrap();
    assert!(state.source_pending());
    settle(&worker, &mut state, prepare);
    assert!(!state.source_pending());
    let prepared = state.inbox_queue.prepared.as_ref().unwrap().clone();
    source.validate_draft(&prepared).unwrap();
    assert!(
        matches!(&prepared.changes[0], DraftNoteChange::Create { text, .. } if text.ends_with(exact))
    );
    assert!(
        state.draft.is_none(),
        "the source acknowledgement must leave opening the form to the guarded transition"
    );
    assert!(state.open_inbox_source_draft());
    assert!(state.inbox_queue.prepared.is_none());
    assert!(state.draft.as_ref().unwrap().is_prepared_source());
    let create = state.create_draft().unwrap();
    settle(&worker, &mut state, create);
    assert_eq!(state.last_draft_request.as_ref(), Some(&prepared));
    fixture.unchanged();
    let (_, AppEvent::InboxItem(retained)) = reply(
        &worker,
        (Uuid::new_v4(), AppCommand::InboxItem(item.capture.id)),
    ) else {
        panic!("retained original");
    };
    assert_eq!(retained.item, item);
    assert_eq!(
        retained.original,
        InboxOriginal::Available { text: exact.into() }
    );
    worker.shutdown().unwrap();
}

#[test]
fn stale_reads_preparation_and_forged_replies_cannot_replace_current_input() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let mut state = state();
    let first = capture(
        &worker,
        &mut state,
        request(InboxKind::Markdown, "# first\r\n"),
    );
    let second = capture(
        &worker,
        &mut state,
        request(InboxKind::Email, "second exact\r\n"),
    );
    let page = state.open_inbox().unwrap();
    let old_page = reply(&worker, page);
    let fresh = state.refresh_inbox().unwrap();
    state.apply(old_page.0, old_page.1);
    assert!(state.inbox_queue.page.is_none());
    settle(&worker, &mut state, fresh);
    let old_read = state.select_inbox(first.capture.id).unwrap();
    let new_read = state.select_inbox(second.capture.id).unwrap();
    let (old_id, old_event) = reply(&worker, old_read);
    state.apply(old_id, old_event);
    assert!(state.inbox_queue.selected.is_none());
    let (new_id, AppEvent::InboxItem(read)) = reply(&worker, new_read) else {
        panic!("new selected reply");
    };
    let mut forged = read.clone();
    forged.item.capture.id = Uuid::new_v4();
    state.apply(new_id, AppEvent::InboxItem(forged));
    assert!(state.pending.contains_key(&new_id));
    assert!(state.inbox_queue.selected.is_none());
    state.apply(new_id, AppEvent::InboxItem(read));
    assert_eq!(state.inbox_queue.selected.as_ref().unwrap().item, second);
    let batch = process(&worker, &mut state, vec![first.clone()]);
    let source = source_request(&batch);
    let stale = state.prepare_inbox_source(source.clone()).unwrap();
    let (id, stale_event) = reply(&worker, stale);
    let new_selection = state.select_inbox(second.capture.id).unwrap();
    state.apply(id, stale_event);
    assert!(state.inbox_queue.prepared.is_none());
    settle(&worker, &mut state, new_selection);
    state.draft = crate::draft::DraftForm::new(None);
    state.draft.as_mut().unwrap().title = "Retained unfinished input".into();
    state.draft.as_mut().unwrap().text = "Exact unsent 👋\r\n".into();
    let draft_id = state.draft.as_ref().unwrap().id;
    let prepare = state.prepare_inbox_source(source.clone()).unwrap();
    let (id, AppEvent::InboxSourceDraft(draft)) = reply(&worker, prepare) else {
        panic!("source draft");
    };
    let mut forged = draft.clone();
    forged
        .inbox_source
        .as_mut()
        .unwrap()
        .original
        .received_at_ms += 1;
    state.apply(id, AppEvent::InboxSourceDraft(forged));
    assert!(state.pending.contains_key(&id));
    assert!(state.inbox_queue.prepared.is_none());
    state.apply(id, AppEvent::InboxSourceDraft(draft));
    assert_eq!(state.draft.as_ref().unwrap().id, draft_id);
    assert_eq!(state.draft.as_ref().unwrap().text, "Exact unsent 👋\r\n");
    source
        .validate_draft(state.inbox_queue.prepared.as_ref().unwrap())
        .unwrap();
    assert!(!state.open_inbox_source_draft());
    fixture.unchanged();
    worker.shutdown().unwrap();
}

#[test]
fn exact_failed_requests_retry_and_terminal_cancel_is_correlated_without_regression() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let mut state = state();
    let exact = request(InboxKind::Text, "Exact retry\r\nõ");
    let capture = state.capture_inbox(exact.clone()).unwrap();
    state.apply(
        capture.0,
        AppEvent::Failed(brn_workflow::WorkflowError::msg("synthetic admission loss")),
    );
    let retry = state.retry_capture().unwrap();
    assert_eq!(retry.0, exact.id);
    assert!(matches!(&retry.1, AppCommand::CaptureInbox(request) if request == &exact));
    settle(&worker, &mut state, retry);
    let item = state.inbox_queue.capture_result.clone().unwrap();
    let open = state.open_inbox().unwrap();
    settle(&worker, &mut state, open);
    let command = state.process_inbox_items(vec![item.clone()]).unwrap();
    let AppCommand::ProcessInbox(request) = &command.1 else {
        panic!("process request");
    };
    let captured = request.clone();
    state.apply(
        command.0,
        AppEvent::Failed(brn_workflow::WorkflowError::msg("synthetic admission loss")),
    );
    assert!(state.can_retry_process());
    let refresh = state.refresh_inbox().unwrap();
    settle(&worker, &mut state, refresh);
    assert!(state.inbox_queue.error.is_none());
    assert!(
        state.can_retry_process(),
        "inventory success must not hide the retained failed batch"
    );
    let retry = state.retry_process().unwrap();
    assert!(!state.can_retry_process());
    assert_eq!(retry.0, captured.id);
    assert!(matches!(&retry.1, AppCommand::ProcessInbox(request) if request == &captured));
    worker.submit(retry.0, retry.1).unwrap();
    let cancel = state.cancel_inbox_batch().unwrap();
    let cancel_id = cancel.0;
    worker.submit(cancel.0, cancel.1).unwrap();
    loop {
        let (id, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
        state.apply(id, event);
        if id == cancel_id && !state.pending.contains_key(&cancel_id) {
            break;
        }
    }
    assert!(!state.processing_pending());
    let terminal = state.inbox_queue.batch.clone().unwrap();
    assert_eq!(terminal.request, captured);
    assert_eq!(terminal.pending_count(), 0);
    let (_, AppEvent::InboxProcessing(replayed)) = reply(
        &worker,
        (captured.id, AppCommand::ProcessInbox(captured.clone())),
    ) else {
        panic!("historical process reply");
    };
    assert_eq!(*replayed, terminal);
    state.apply(captured.id, AppEvent::InboxProcessing(replayed));
    assert_eq!(state.inbox_queue.batch.as_ref(), Some(&terminal));
    let (_, AppEvent::InboxItem(retained)) = reply(
        &worker,
        (Uuid::new_v4(), AppCommand::InboxItem(item.capture.id)),
    ) else {
        panic!("retained original");
    };
    assert_eq!(retained.item, item);
    assert_eq!(
        retained.original,
        InboxOriginal::Available { text: exact.text }
    );
    fixture.unchanged();
    worker.shutdown().unwrap();
}

#[test]
fn malformed_progress_is_ignored_and_receipts_do_not_hijack_closed_navigation() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let mut state = state();
    let open = state.open_inbox().unwrap();
    settle(&worker, &mut state, open);
    let request = request(InboxKind::Email, "Source retained across navigation\r\n");
    let command = state.capture_inbox(request.clone()).unwrap();
    state.close_inbox();
    settle(&worker, &mut state, command);
    assert!(!state.inbox_queue.visible);
    assert!(state.inbox_queue.selected.is_none());
    let item = state.inbox_queue.capture_result.clone().unwrap();
    let process = state.process_inbox_items(vec![item.clone()]).unwrap();
    let id = process.0;
    worker.submit(id, process.1).unwrap();
    let (_, AppEvent::InboxProcessing(initial)) =
        worker.recv_event_timeout(Duration::from_secs(10)).unwrap()
    else {
        panic!("queued progress");
    };
    let mut forged = initial.clone();
    forged.request.items[0].capture.title = "Changed request".into();
    state.apply(id, AppEvent::InboxProcessing(forged));
    assert!(state.processing_pending());
    assert!(state.inbox_queue.batch.is_none());
    state.apply(id, AppEvent::InboxProcessing(initial.clone()));
    assert!(state.processing_pending());
    while state.processing_pending() {
        let (id, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
        state.apply(id, event);
    }
    let terminal = state.inbox_queue.batch.clone().unwrap();
    state.apply(id, AppEvent::InboxProcessing(initial));
    assert_eq!(state.inbox_queue.batch.as_ref(), Some(&terminal));
    assert!(!state.inbox_queue.visible);
    assert!(state.inbox_queue.selected.is_none());
    assert!(state.inbox_queue.preview.is_none());
    fixture.unchanged();
    worker.shutdown().unwrap();
}

#[test]
fn complete_operational_preview_remains_visible_when_source_wrapper_exceeds_limit() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let mut state = state();
    let exact = "x".repeat(brn_workflow::MAX_NOTE_BYTES);
    let item = capture(&worker, &mut state, request(InboxKind::Markdown, &exact));
    let open = state.open_inbox().unwrap();
    settle(&worker, &mut state, open);
    let batch = process(&worker, &mut state, vec![item]);
    let command = state.preview_inbox_candidate(0).unwrap();
    let (id, AppEvent::InboxCandidate(preview)) = reply(&worker, command) else {
        panic!("complete operational preview");
    };
    let mut forged = preview.clone();
    forged.markdown.replace_range(0..1, "y");
    state.apply(id, AppEvent::InboxCandidate(forged));
    assert!(state.pending.contains_key(&id));
    assert!(state.inbox_queue.preview.is_none());
    state.apply(id, AppEvent::InboxCandidate(preview));
    assert_eq!(state.inbox_queue.preview.as_ref().unwrap().markdown, exact);
    let prepare = state.prepare_inbox_source(source_request(&batch)).unwrap();
    settle(&worker, &mut state, prepare);
    assert!(state.inbox_queue.source_error.is_some());
    assert!(state.inbox_queue.prepared.is_none());
    assert!(state.draft.is_none());
    assert_eq!(state.inbox_queue.preview.as_ref().unwrap().markdown, exact);
    fixture.unchanged();
    worker.shutdown().unwrap();
}

#[test]
fn binary_reads_verify_complete_proofs_and_queue_durable_unsupported_failure() {
    use brn_workflow::inbox::CaptureBinaryInboxRequest;
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let request = CaptureBinaryInboxRequest {
        id: Uuid::new_v4(),
        title: "Binary original".into(),
        original_name: Some("Synthetic.pdf".into()),
        bytes: b"Binary may also be valid UTF-8\r\n".to_vec(),
    };
    let (_, AppEvent::InboxCaptured(item)) = reply(
        &worker,
        (request.id, AppCommand::CaptureBinaryInbox(request.clone())),
    ) else {
        panic!("binary receipt")
    };
    let mut state = state();
    let open = state.open_inbox().unwrap();
    settle(&worker, &mut state, open);
    let select = state.select_inbox(request.id).unwrap();
    let id = select.0;
    let (_, AppEvent::InboxItem(read)) = reply(&worker, select) else {
        panic!("binary original")
    };
    let mut forged = read.clone();
    forged.original = InboxOriginal::Available {
        text: String::from_utf8(request.bytes.clone()).unwrap(),
    };
    state.apply(id, AppEvent::InboxItem(forged));
    assert!(state.inbox_queue.selected.is_none());
    assert!(state.pending.contains_key(&id));
    let mut forged = read.clone();
    if let InboxOriginal::AvailableBinary { sha256, .. } = &mut forged.original {
        sha256[0] ^= 1;
    }
    state.apply(id, AppEvent::InboxItem(forged));
    assert!(state.inbox_queue.selected.is_none());
    state.apply(id, AppEvent::InboxItem(read.clone()));
    assert_eq!(state.inbox_queue.selected.as_ref().unwrap().item, *item);
    assert!(!state.pending.contains_key(&id));
    let batch = process(&worker, &mut state, vec![(*item).clone()]);
    assert!(matches!(
        &batch.entries[0].outcome,
        InboxProcessOutcome::Failed { code } if code == "intake_invalid"
    ));
    assert!(state.preview_inbox_candidate(0).is_none());
    let poll = reply(
        &worker,
        (
            Uuid::new_v4(),
            AppCommand::InboxProcessing(batch.request.id),
        ),
    );
    assert!(matches!(poll.1, AppEvent::InboxProcessing(retained) if *retained == batch));
    assert!(!state.processing_pending());
    assert!(!state.can_retry_process());
    assert!(
        state
            .capture_inbox(CaptureInboxRequest {
                id: Uuid::new_v4(),
                kind: InboxKind::Binary,
                title: "No text coercion".into(),
                original_name: None,
                text: String::from_utf8(request.bytes.clone()).unwrap()
            })
            .is_none()
    );
    fixture.unchanged();
    worker.shutdown().unwrap();
}

// Independent tiny Stored ZIP fixture; complete bytes reach the real AppWorker.
fn synthetic_docx() -> Vec<u8> {
    let parts = [
        (
            "[Content_Types].xml",
            r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#,
        ),
        (
            "_rels/.rels",
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
        ),
        (
            "word/document.xml",
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>First õ 日本語</w:t></w:r></w:p><w:p><w:r><w:t>Second preserved</w:t></w:r></w:p></w:body></w:document>"#,
        ),
    ];
    let mut bytes = Vec::new();
    let mut central = Vec::new();
    for (name, text) in parts {
        let payload = text.as_bytes();
        let mut crc = u32::MAX;
        for byte in payload {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                crc = (crc >> 1) ^ (0xedb88320 & 0u32.wrapping_sub(crc & 1));
            }
        }
        crc = !crc;
        let offset = bytes.len() as u32;
        bytes.extend(0x04034b50u32.to_le_bytes());
        for value in [20u16, 0, 0, 0, 0] {
            bytes.extend(value.to_le_bytes());
        }
        for value in [crc, payload.len() as u32, payload.len() as u32] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend((name.len() as u16).to_le_bytes());
        bytes.extend(0u16.to_le_bytes());
        bytes.extend(name.as_bytes());
        bytes.extend(payload);
        central.extend(0x02014b50u32.to_le_bytes());
        for value in [20u16, 20, 0, 0, 0, 0] {
            central.extend(value.to_le_bytes());
        }
        for value in [crc, payload.len() as u32, payload.len() as u32] {
            central.extend(value.to_le_bytes());
        }
        for value in [name.len() as u16, 0, 0, 0, 0] {
            central.extend(value.to_le_bytes());
        }
        central.extend(0u32.to_le_bytes());
        central.extend(offset.to_le_bytes());
        central.extend(name.as_bytes());
    }
    let offset = bytes.len() as u32;
    let len = central.len() as u32;
    bytes.extend(central);
    bytes.extend(0x06054b50u32.to_le_bytes());
    for value in [0u16, 0, 3, 3] {
        bytes.extend(value.to_le_bytes());
    }
    bytes.extend(len.to_le_bytes());
    bytes.extend(offset.to_le_bytes());
    bytes.extend(0u16.to_le_bytes());
    bytes
}

#[test]
fn docx_worker_preview_binds_format_complete_bytes_and_exact_source_review() {
    use brn_workflow::inbox::CaptureBinaryInboxRequest;
    use brn_workflow::inbox_processing::InboxConversionFormat;
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let request = CaptureBinaryInboxRequest {
        id: Uuid::new_v4(),
        title: "Actual DOCX with misleading label".into(),
        original_name: Some("synthetic.pdf".into()),
        bytes: synthetic_docx(),
    };
    let (_, AppEvent::InboxCaptured(item)) = reply(
        &worker,
        (request.id, AppCommand::CaptureBinaryInbox(request.clone())),
    ) else {
        panic!("binary capture")
    };
    request.validate_receipt(&item).unwrap();
    let mut state = state();
    let open = state.open_inbox().unwrap();
    settle(&worker, &mut state, open);
    let batch = process(&worker, &mut state, vec![(*item).clone()]);
    assert!(matches!(
        batch.entries[0].outcome,
        InboxProcessOutcome::Converted {
            format: InboxConversionFormat::MaintainedExtractionV1,
            ..
        }
    ));
    let command = state.preview_inbox_candidate(0).unwrap();
    let (id, AppEvent::InboxCandidate(preview)) = reply(&worker, command) else {
        panic!("DOCX preview")
    };
    assert_eq!(
        preview.markdown,
        "<!-- docx-story: body body -->\n\n<!-- docx-export:0 -->\nFirst õ 日本語\n\n<!-- docx-export:1 -->\nSecond preserved\n"
    );
    assert!(preview.extraction.is_some());
    assert!(preview.needs_semantic_review);
    for alteration in 0..5 {
        let mut forged = preview.clone();
        match alteration {
            0 => forged.markdown.push_str("Clipped or invented"),
            1 => forged.needs_semantic_review = false,
            2 => forged.original.capture.copy.sha256[0] ^= 1,
            3 => forged.request.batch_id = Uuid::new_v4(),
            _ => forged.format = InboxConversionFormat::LiteralTextV1,
        }
        state.apply(id, AppEvent::InboxCandidate(forged));
        assert!(state.inbox_queue.preview.is_none());
        assert!(state.pending.contains_key(&id));
    }
    state.apply(id, AppEvent::InboxCandidate(preview));
    let source = source_request(&batch);
    let command = state.prepare_inbox_source(source.clone()).unwrap();
    settle(&worker, &mut state, command);
    let prepared = state.inbox_queue.prepared.as_ref().unwrap().clone();
    source.validate_draft(&prepared).unwrap();
    assert_eq!(
        prepared.inbox_source.as_ref().unwrap().format,
        InboxConversionFormat::MaintainedExtractionV1
    );
    assert!(
        matches!(&prepared.changes[0], DraftNoteChange::Create { text, .. }
        if text.contains("First õ 日本語") && text.ends_with("Second preserved\n"))
    );
    assert!(state.draft.is_none());
    assert!(state.open_inbox_source_draft());
    let command = state.create_draft().unwrap();
    settle(&worker, &mut state, command);
    assert_eq!(state.last_draft_request.as_ref(), Some(&prepared));
    fixture.unchanged();
    assert_eq!(
        fs::read(
            fixture
                .0
                .path()
                .join("data/inbox")
                .join(item.capture.copy_name())
        )
        .unwrap(),
        request.bytes
    );
    worker.shutdown().unwrap();
}

#[test]
fn plural_mail_source_preparation_accepts_exact_source_materialization() {
    use brn_workflow::inbox::CaptureBinaryInboxRequest;
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let request = CaptureBinaryInboxRequest {
        id: Uuid::new_v4(),
        title: "Plural public email".into(),
        original_name: Some("plural.eml".into()),
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../experiments/architecture-reassessment/p1-office-mime/fixtures/plural.eml"
        ))
        .to_vec(),
    };
    let (_, AppEvent::InboxCaptured(item)) = reply(
        &worker,
        (request.id, AppCommand::CaptureBinaryInbox(request.clone())),
    ) else {
        panic!("capture")
    };
    let mut state = state();
    let open = state.open_inbox().unwrap();
    settle(&worker, &mut state, open);
    let batch = process(&worker, &mut state, vec![*item]);
    let preview = state.preview_inbox_candidate(0).unwrap();
    settle(&worker, &mut state, preview);
    let extraction = state
        .inbox_queue
        .preview
        .as_ref()
        .unwrap()
        .extraction
        .as_ref()
        .unwrap()
        .clone();
    assert!(extraction.occurrences.len() > 1);
    let source = source_request(&batch);
    let command = state.prepare_inbox_source(source.clone()).unwrap();
    settle(&worker, &mut state, command);
    let prepared = state
        .inbox_queue
        .prepared
        .as_ref()
        .expect("source-specific filenames must pass native response qualification")
        .clone();
    source.validate_draft(&prepared).unwrap();
    let body = extraction
        .materialize_for_source(&source.note_id.to_string())
        .unwrap()
        .markdown;
    assert!(
        matches!(&prepared.changes[0],DraftNoteChange::Create{text,..} if text.ends_with(&body))
    );
    assert_eq!(prepared.changes.len(), 1 + extraction.assets.len());
    assert!(state.open_inbox_source_draft());
    let command = state.create_draft().unwrap();
    settle(&worker, &mut state, command);
    assert_eq!(state.last_draft_request.as_ref(), Some(&prepared));
    fixture.unchanged();
    worker.shutdown().unwrap();
}

#[path = "inbox_guided_tests.rs"]
mod guided_tests;
