use super::*;

const FULL: &str = "\u{feff}---\r\ncustom: λ\r\n---\r\n本文 日本語 õ🦀\r\n";
const LATER: &str = "\u{feff}Later full body Ελληνικά\r\nλ\r";

fn prepared() -> DraftRequest {
    serde_json::from_value(serde_json::json!({
        "id": "00000000-0000-4000-8000-000000000001",
        "group_id": "00000000-0000-4000-8000-000000000002",
        "session_id": "00000000-0000-4000-8000-000000000003",
        "title": "Full prepared title 日本語\r\n",
        "changes": [{
            "kind": "replace", "path": "consumer.md", "text": FULL,
            "expected": {"device": 11, "inode": 21, "len": 101, "sha256": vec![1u8; 32]}
        }],
        "sources": [
            {"path": "consumer.md", "fingerprint": {"device": 11, "inode": 21, "len": 101, "sha256": vec![1u8; 32]}},
            {"path": "archive/tõend.md", "fingerprint": {"device": 12, "inode": 22, "len": 202, "sha256": vec![2u8; 32]}}
        ]
    })).unwrap()
}

#[test]
fn full_prepared_bindings_survive_title_and_body_edits_without_fabricated_capture() {
    let request = prepared();
    let mut form = DraftForm::from_prepared(request.clone()).unwrap();
    assert_eq!(form.id, request.id);
    assert_eq!(form.session_id, request.session_id);
    assert_eq!(form.path, "consumer.md");
    assert_eq!(form.kind, DraftKind::Replace);
    assert_eq!(form.title, "Full prepared title 日本語\r\n");
    assert_eq!(form.text.as_bytes(), FULL.as_bytes());
    assert!(form.source.is_none());
    assert!(form.source_operation.is_none());
    assert_eq!(form.prepared_request(), Some(&request));
    assert!(!form.can_leave());
    form.edit(
        "\u{feff}New full title λ\r\n".into(),
        "consumer.md".into(),
        LATER.into(),
        DraftKind::Replace,
    );
    let actual = form.request().unwrap();
    assert_eq!(actual.id, request.id);
    assert_eq!(actual.group_id, request.group_id);
    assert_eq!(actual.session_id, request.session_id);
    assert_eq!(actual.sources, request.sources);
    assert_eq!(actual.title, "\u{feff}New full title λ\r\n");
    let DraftNoteChange::Replace {
        path,
        expected,
        text,
    } = &actual.changes[0]
    else {
        panic!("replace")
    };
    assert_eq!(path, "consumer.md");
    assert_eq!(expected, &request.sources[0].fingerprint);
    assert_eq!(text.as_bytes(), LATER.as_bytes());
    assert_eq!(form.prepared_request(), Some(&request));
}

#[test]
fn prepared_binding_edits_are_refused_without_losing_the_full_retained_form() {
    for (path, kind) in [
        ("another.md", DraftKind::Replace),
        ("consumer.md", DraftKind::Create),
        ("consumer.md", DraftKind::Trash),
    ] {
        let request = prepared();
        let mut form = DraftForm::from_prepared(request.clone()).unwrap();
        let generation = form.generation;
        let binding = form.binding_generation;
        form.edit("Later title".into(), path.into(), LATER.into(), kind);
        assert_eq!(form.path, "consumer.md");
        assert_eq!(form.kind, DraftKind::Replace);
        assert_eq!(form.title, request.title);
        assert_eq!(form.text.as_bytes(), FULL.as_bytes());
        assert_eq!(form.generation, generation);
        assert_eq!(form.binding_generation, binding);
        assert!(
            form.error
                .as_ref()
                .is_some_and(|error| error.contains("fixed"))
        );
        assert_eq!(form.request().unwrap(), request);
    }
}

#[test]
fn separate_prepared_form_keeps_both_sources_and_later_input_under_a_fresh_uuid() {
    let request = prepared();
    let mut form = DraftForm::from_prepared(request.clone()).unwrap();
    let initial = form.prepare().unwrap();
    assert!(form.separate().is_none());
    form.failed(initial.operation, "synthetic failure".into());
    let retry = form.prepare().unwrap();
    assert_eq!(retry.request, initial.request);
    assert_ne!(retry.operation, initial.operation);
    form.failed(retry.operation, "synthetic retry failure".into());
    form.edit(
        "Later full title λ\r\n".into(),
        "consumer.md".into(),
        LATER.into(),
        DraftKind::Replace,
    );
    assert!(form.prepare().is_none());
    let mut separate = form.separate().unwrap();
    assert_ne!(separate.id, form.id);
    assert_eq!(separate.title, "Later full title λ\r\n");
    assert_eq!(separate.text.as_bytes(), LATER.as_bytes());
    assert_eq!(separate.session_id, request.session_id);
    assert!(separate.source.is_none());
    assert!(separate.submitted.is_none());
    assert!(!separate.pending);
    assert!(separate.result.is_none());
    let template = separate.prepared_request().unwrap();
    assert_eq!(template.id, separate.id);
    assert_eq!(template.sources, request.sources);
    assert_eq!(template.group_id, request.group_id);
    assert_eq!(template.session_id, request.session_id);
    let submitted = separate.prepare().unwrap();
    assert_eq!(submitted.request.id, separate.id);
    assert_eq!(submitted.request.sources, request.sources);
    assert_eq!(submitted.request.group_id, request.group_id);
    assert_eq!(submitted.request.session_id, request.session_id);
    assert_eq!(submitted.request.title, "Later full title λ\r\n");
    let DraftNoteChange::Replace { expected, text, .. } = &submitted.request.changes[0] else {
        panic!("replace")
    };
    assert_eq!(expected, &request.sources[0].fingerprint);
    assert_eq!(text.as_bytes(), LATER.as_bytes());
    assert_eq!(form.submitted.as_ref().unwrap().request, initial.request);
}

#[test]
fn prepared_constructor_rejects_incomplete_mismatched_oversized_and_nonreplace_requests() {
    let request = prepared();
    let mut invalid = vec![];
    for field in 0..4 {
        let mut changed = request.clone();
        let fp = &mut changed.sources[0].fingerprint;
        match field {
            0 => fp.device += 1,
            1 => fp.inode += 1,
            2 => fp.len += 1,
            _ => fp.sha256 = [9; 32],
        }
        invalid.push(changed);
    }
    for count in 0..2 {
        let mut changed = request.clone();
        changed.sources.truncate(count);
        invalid.push(changed);
    }
    let mut changed = request.clone();
    let mut extra = changed.sources[1].clone();
    extra.path = "another-source.md".into();
    changed.sources.push(extra);
    invalid.push(changed);
    let mut changed = request.clone();
    changed.sources[1].path = "Consumer.MD".into();
    invalid.push(changed);
    let mut changed = request.clone();
    changed.sources[0].path = "another-consumer.md".into();
    invalid.push(changed);
    let mut changed = request.clone();
    changed.changes = vec![DraftNoteChange::Create {
        path: "consumer.md".into(),
        text: FULL.into(),
    }];
    invalid.push(changed);
    let mut changed = request.clone();
    changed.changes = vec![DraftNoteChange::Trash {
        path: "consumer.md".into(),
        expected: request.sources[0].fingerprint.clone(),
    }];
    invalid.push(changed);
    let mut changed = request.clone();
    changed.changes.push(DraftNoteChange::Create {
        path: "another.md".into(),
        text: FULL.into(),
    });
    invalid.push(changed);
    let mut changed = request.clone();
    changed.changes.clear();
    invalid.push(changed);
    for path in ["../outside.md", "note.txt", ".hidden.md"] {
        let mut changed = request.clone();
        if let DraftNoteChange::Replace {
            path: destination, ..
        } = &mut changed.changes[0]
        {
            *destination = path.into();
        }
        changed.sources[0].path = path.into();
        invalid.push(changed);
    }
    let mut changed = request.clone();
    changed.sources[1].path = "../private.md".into();
    invalid.push(changed);
    let mut changed = request.clone();
    changed.id = Uuid::nil();
    invalid.push(changed);
    let mut changed = request.clone();
    changed.group_id = Some(Uuid::nil());
    invalid.push(changed);
    let mut changed = request.clone();
    changed.session_id = Some(Uuid::nil());
    invalid.push(changed);
    for title in [" \r\n".into(), "a".repeat(513)] {
        let mut changed = request.clone();
        changed.title = title;
        invalid.push(changed);
    }
    let mut changed = request.clone();
    if let DraftNoteChange::Replace { text, .. } = &mut changed.changes[0] {
        *text = "a".repeat(MAX_NOTE_BYTES + 1);
    }
    invalid.push(changed);
    let mut changed = request.clone();
    changed.sources[1].fingerprint.len = MAX_NOTE_BYTES as u64 + 1;
    invalid.push(changed);
    for (index, request) in invalid.into_iter().enumerate() {
        assert!(
            DraftForm::from_prepared(request).is_err(),
            "invalid case {index}"
        );
    }
    let mut reversed = request;
    reversed.sources.reverse();
    assert_eq!(
        DraftForm::from_prepared(reversed.clone())
            .unwrap()
            .request()
            .unwrap(),
        reversed
    );
}

#[test]
fn separate_prepared_form_retains_invalid_full_typing_for_explicit_correction() {
    let mut form = DraftForm::from_prepared(prepared()).unwrap();
    let oversized = "λ".repeat(MAX_NOTE_BYTES / 2 + 1);
    form.edit(
        String::new(),
        "consumer.md".into(),
        oversized.clone(),
        DraftKind::Replace,
    );
    assert!(form.prepare().is_none());
    let mut separate = form.separate().unwrap();
    assert_eq!(separate.title, "");
    assert_eq!(separate.text, oversized);
    assert!(separate.request().is_err());
    separate.edit(
        "Corrected".into(),
        "consumer.md".into(),
        FULL.into(),
        DraftKind::Replace,
    );
    assert!(separate.prepare().is_some());
}

#[test]
fn ordinary_forms_keep_existing_creation_capture_and_separate_behavior() {
    let mut ordinary = DraftForm::new(None).unwrap();
    assert!(ordinary.can_leave());
    assert!(ordinary.prepared_request().is_none());
    ordinary.edit(
        "Ordinary".into(),
        "new.md".into(),
        FULL.into(),
        DraftKind::Create,
    );
    assert!(ordinary.request().unwrap().sources.is_empty());
    ordinary.edit(
        "Existing".into(),
        "consumer.md".into(),
        FULL.into(),
        DraftKind::Replace,
    );
    assert!(ordinary.request().is_err());
    ordinary.source = Some(ProposalSource {
        source: prepared().sources[0].clone(),
        text: "Captured full source\r\n".into(),
    });
    ordinary.session_id = prepared().session_id;
    let initial = ordinary.prepare().unwrap();
    assert!(ordinary.separate().is_none());
    ordinary.failed(initial.operation, "synthetic failure".into());
    let separate = ordinary.separate().unwrap();
    assert!(separate.prepared_request().is_none());
    assert_eq!(separate.source, ordinary.source);
    assert_eq!(separate.title, ordinary.title);
    assert_eq!(separate.path, ordinary.path);
    assert_eq!(separate.kind, ordinary.kind);
    assert_eq!(separate.text.as_bytes(), FULL.as_bytes());
    assert_eq!(separate.session_id, ordinary.session_id);
    assert_eq!(
        separate.request().unwrap().sources,
        vec![prepared().sources[0].clone()]
    );
    assert_ne!(separate.id, ordinary.id);
}

#[test]
fn prepared_current_ack_requires_both_source_bindings_and_preserves_later_full_typing() {
    let request = prepared();
    let mut form = DraftForm::from_prepared(request.clone()).unwrap();
    let submitted = form.prepare().unwrap();
    form.edit(
        "Later local title λ\r\n".into(),
        "consumer.md".into(),
        LATER.into(),
        DraftKind::Replace,
    );
    let record: ProposalRecord = serde_json::from_value(serde_json::json!({
        "draft": {
            "id": request.id, "group_id": request.group_id, "session_id": request.session_id,
            "vault": {"id": "00000000-0000-4000-8000-000000000004", "root": "/synthetic/vault", "identity": {"device": 11, "inode": 30}},
            "title": "Later persisted review 日本語\r\n",
            "changes": [{"kind": "replace", "path": "consumer.md", "parent": {"device": 11, "inode": 30}, "before": request.sources[0].fingerprint, "before_text": "Trusted full before text\r\n", "text": "Later persisted body λ\r\n"}],
            "sources": request.sources
        },
        "version": 2, "state": "draft", "comments": [], "created_at_ms": 1, "updated_at_ms": 2
    })).unwrap();
    let mut wrong = record.clone();
    wrong.draft.sources[1].fingerprint.sha256 = [9; 32];
    assert!(!form.created(submitted.operation, wrong));
    assert!(form.pending);
    assert!(!form.created(Uuid::new_v4(), record.clone()));
    assert!(form.created(submitted.operation, record.clone()));
    assert!(!form.pending);
    assert_eq!(form.title, "Later local title λ\r\n");
    assert_eq!(form.text.as_bytes(), LATER.as_bytes());
    assert_eq!(form.submitted.as_ref().unwrap().request, submitted.request);
    assert_eq!(form.result, Some((submitted.generation, record)));
    assert!(!form.can_leave());
    let current = form.request().unwrap();
    assert_eq!(current.sources, request.sources);
    assert_eq!(current.group_id, request.group_id);
    assert_eq!(current.session_id, request.session_id);
    assert_eq!(current.title, "Later local title λ\r\n");
}
