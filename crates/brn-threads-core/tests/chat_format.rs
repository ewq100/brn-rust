use brn_threads_core::Note;

#[test]
fn private_note_scope_survives_serialization_without_changing_legacy_bytes() {
    let ordinary = Note::working("Reference", "Readable content");
    let legacy = serde_json::to_value(&ordinary).unwrap();
    assert!(legacy.get("chat").is_none());
    let mut scoped = legacy.clone();
    scoped["chat"] = serde_json::json!("chat-a");
    let saved: Note = serde_json::from_value(scoped.clone()).unwrap();
    assert_eq!(serde_json::to_value(saved).unwrap(), scoped);
    assert_eq!(serde_json::to_value(ordinary).unwrap(), legacy);
}
