use super::*;
use brn_workflow::inbox_processing::InboxSourceBinding;

const ORIGINAL: &str = "\u{feff}# Evidence 日本語 λ\r\nExact original 🦀\r\n";
// SHA-256 of the complete synthetic ORIGINAL bytes, including BOM and CRLF.
const HASH: [u8; 32] = [
    118, 251, 229, 241, 46, 168, 74, 116, 125, 133, 203, 37, 181, 123, 75, 19, 71, 24, 108, 47,
    185, 69, 185, 26, 227, 98, 94, 44, 169, 164, 32, 36,
];

fn prepared_source() -> DraftRequest {
    let binding: InboxSourceBinding = serde_json::from_value(serde_json::json!({
        "batch_id": "00000000-0000-4000-8000-000000000004", "index": 2,
        "original": {
            "capture": {
                "id": "00000000-0000-4000-8000-000000000005", "kind": "markdown",
                "title": "Evidence 日本語 λ", "original_name": "Original 🦀.md",
                "copy": {"directory": "/synthetic/intake", "directory_device": 11,
                    "directory_inode": 21, "file_device": 11, "file_inode": 22,
                    "byte_len": ORIGINAL.len(), "sha256": HASH}
            }, "received_at_ms": 100
        },
        "format": "verbatim_markdown_v1", "byte_len": ORIGINAL.len(), "sha256": HASH,
        "note_id": "00000000-0000-4000-8000-000000000006"
    }))
    .unwrap();
    DraftRequest {
        intake: None,
        id: Uuid::parse_str("00000000-0000-4000-8000-000000000001").unwrap(),
        group_id: Some(Uuid::parse_str("00000000-0000-4000-8000-000000000002").unwrap()),
        session_id: Some(Uuid::parse_str("00000000-0000-4000-8000-000000000003").unwrap()),
        title: "Prepared Source 日本語 λ".into(),
        changes: vec![DraftNoteChange::Create {
            path: "sources/資料 λ.md".into(),
            text: binding.markdown(ORIGINAL).unwrap(),
        }],
        sources: vec![],
        action_changes: vec![],
        inbox_visual: None,
        inbox_knowledge: None,
        inbox_source: Some(Box::new(binding)),
    }
}

fn text(request: &DraftRequest) -> &str {
    let DraftNoteChange::Create { text, .. } = &request.changes[0] else {
        panic!("Source Create")
    };
    text
}

fn record(request: &DraftRequest) -> ProposalRecord {
    serde_json::from_value(serde_json::json!({
        "draft": {
            "id": request.id, "group_id": request.group_id, "session_id": request.session_id,
            "vault": {"id": "00000000-0000-4000-8000-000000000007", "root": "/synthetic/vault", "identity": {"device": 11, "inode": 30}},
            "title": "Later persisted review title λ",
            "changes": [{"kind": "create", "path": "sources/資料 λ.md", "parent": {"device": 11, "inode": 31}, "text": text(request)}],
            "sources": [], "inbox_source": request.inbox_source
        },
        "version": 2, "state": "draft", "comments": [], "created_at_ms": 1, "updated_at_ms": 2
    })).unwrap()
}

#[test]
fn source_preparation_retains_the_whole_request_and_only_edits_title() {
    let request = prepared_source();
    let mut form = DraftForm::from_inbox_source(request.clone()).unwrap();
    assert!(form.is_prepared_source());
    assert_eq!(form.kind, DraftKind::Create);
    assert_eq!(form.prepared_request(), Some(&request));
    assert_eq!(form.request().unwrap(), request);
    assert!(form.source.is_none());
    assert!(!form.can_leave());
    let generation = form.generation;
    let binding_generation = form.binding_generation;
    form.edit(
        "\u{feff}Later title 日本語\r\nλ\r".into(),
        form.path.clone(),
        form.text.clone(),
        DraftKind::Create,
    );
    let mut expected = request.clone();
    expected.title = form.title.clone();
    assert_eq!(form.request().unwrap(), expected);
    assert_eq!(form.prepared_request(), Some(&request));
    assert_eq!(form.generation, generation + 1);
    assert_eq!(form.binding_generation, binding_generation);
    assert_eq!(form.text.as_bytes(), text(&request).as_bytes());
}

#[test]
fn source_constructor_refuses_unbound_or_invalid_body_destination_and_kind() {
    let request = prepared_source();
    let mut invalid = Vec::new();
    let mut missing = request.clone();
    missing.inbox_source = None;
    invalid.push(missing);
    let mut changed = request.clone();
    if let DraftNoteChange::Create { text, .. } = &mut changed.changes[0] {
        text.push_str("Different original");
    }
    invalid.push(changed);
    let mut changed = request.clone();
    if let DraftNoteChange::Create { text, .. } = &mut changed.changes[0] {
        *text = text.replace("brn_kind: source", "brn_kind: note");
    }
    invalid.push(changed);
    let mut changed = request.clone();
    if let DraftNoteChange::Create { path, .. } = &mut changed.changes[0] {
        *path = "../rebound.md".into();
    }
    invalid.push(changed);
    let mut changed = request.clone();
    changed.changes = vec![DraftNoteChange::Replace {
        path: "sources/資料 λ.md".into(),
        expected: brn_workflow::editor::FileFingerprint {
            device: 11,
            inode: 32,
            len: 49,
            sha256: HASH,
        },
        text: text(&request).into(),
    }];
    invalid.push(changed);
    let mut changed = request.clone();
    changed.changes.push(changed.changes[0].clone());
    invalid.push(changed);
    for changed in invalid {
        assert!(DraftForm::from_inbox_source(changed).is_err());
    }
    assert!(DraftForm::from_prepared(request).is_err());
}

#[test]
fn source_form_refuses_editing_bound_body_path_and_kind_without_losing_input() {
    let request = prepared_source();
    for change in 0..4 {
        let mut form = DraftForm::from_inbox_source(request.clone()).unwrap();
        let generation = form.generation;
        let binding_generation = form.binding_generation;
        form.edit(
            "Attempted title".into(),
            if change == 0 {
                "other.md".into()
            } else {
                form.path.clone()
            },
            if change == 1 {
                format!("{}changed", form.text)
            } else {
                form.text.clone()
            },
            match change {
                2 => DraftKind::Replace,
                3 => DraftKind::Trash,
                _ => DraftKind::Create,
            },
        );
        assert_eq!(form.request().unwrap(), request);
        assert_eq!(form.generation, generation);
        assert_eq!(form.binding_generation, binding_generation);
        assert!(form.error.as_ref().unwrap().contains("fixed"));
    }
    // Public presentation fields cannot bypass shared exact-byte validation.
    for change in 0..4 {
        let mut form = DraftForm::from_inbox_source(request.clone()).unwrap();
        match change {
            0 => form.path = "other.md".into(),
            1 => form.kind = DraftKind::Replace,
            2 => form.text.push_str("changed"),
            _ => {
                form.text = form
                    .text
                    .replace("brn_state: current", "brn_state: history")
            }
        }
        assert!(form.request().is_err());
        assert!(form.prepare().is_none());
        assert!(form.submitted.is_none());
        assert_eq!(form.prepared_request(), Some(&request));
    }
}

#[test]
fn source_retry_and_separate_copy_keep_proof_and_the_original_note_uuid() {
    let request = prepared_source();
    let mut form = DraftForm::from_inbox_source(request.clone()).unwrap();
    let initial = form.prepare().unwrap();
    assert!(form.separate().is_none());
    form.failed(Uuid::new_v4(), "stale failure".into());
    assert!(form.pending);
    form.failed(initial.operation, "synthetic failure".into());
    let retry = form.prepare().unwrap();
    assert_ne!(retry.operation, initial.operation);
    assert_eq!(retry.request, initial.request);
    form.failed(retry.operation, "synthetic retry failure".into());
    form.edit(
        "Later local title λ".into(),
        form.path.clone(),
        form.text.clone(),
        form.kind,
    );
    assert!(form.prepare().is_none());
    let mut copy = form.separate().unwrap();
    assert_ne!(copy.id, form.id);
    assert!(copy.is_prepared_source());
    assert_eq!(copy.title, form.title);
    assert_eq!(copy.text.as_bytes(), text(&request).as_bytes());
    let actual = copy.prepare().unwrap().request;
    let mut expected = request.clone();
    expected.id = copy.id;
    expected.title = form.title.clone();
    assert_eq!(actual, expected);
    assert_eq!(actual.inbox_source, request.inbox_source);
    assert_eq!(copy.prepared_request().unwrap().id, copy.id);
}

#[test]
fn source_late_ack_requires_the_entire_binding_and_keeps_later_title() {
    let request = prepared_source();
    let mut form = DraftForm::from_inbox_source(request.clone()).unwrap();
    let submitted = form.prepare().unwrap();
    form.edit(
        "Later local title λ".into(),
        form.path.clone(),
        form.text.clone(),
        form.kind,
    );
    let record = record(&request);
    for change in 0..5 {
        let mut wrong = record.clone();
        let binding = wrong.draft.inbox_source.as_mut().unwrap();
        match change {
            0 => binding.original.capture.copy.sha256[0] ^= 1,
            1 => binding.sha256[0] ^= 1,
            2 => binding.note_id = Uuid::new_v4(),
            3 => binding.index += 1,
            _ => binding.original.received_at_ms += 1,
        }
        assert!(!form.created(submitted.operation, wrong));
        assert!(form.pending);
    }
    assert!(!form.created(Uuid::new_v4(), record.clone()));
    assert!(form.created(submitted.operation, record.clone()));
    assert_eq!(form.title, "Later local title λ");
    assert_eq!(form.text.as_bytes(), text(&request).as_bytes());
    assert_eq!(form.submitted.as_ref().unwrap().request, request);
    assert_eq!(form.result, Some((submitted.generation, record)));
    assert!(!form.can_leave());
    assert_eq!(form.request().unwrap().inbox_source, request.inbox_source);
}
