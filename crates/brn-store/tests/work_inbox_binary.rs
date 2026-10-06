use brn_store::{
    WorkStore,
    work::{
        inbox::*,
        inbox_processing::*,
        inbox_source::{InboxSourceBinding, convert_original, read_provenance},
    },
};
use sha2::{Digest, Sha256};
use std::sync::atomic::AtomicBool;
use uuid::Uuid;

fn hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn fixture() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
}
fn capture(dir: &std::path::Path, bytes: &[u8]) -> InboxCapture {
    let id = Uuid::new_v4();
    let path = dir.join(format!("{id}.bin"));
    std::fs::write(&path, bytes).unwrap();
    InboxCapture {
        id,
        kind: InboxKind::Binary,
        title: "Opaque synthetic copy õ".into(),
        original_name: Some("../supplied.pdf".into()),
        copy: InboxCopy {
            directory: dir.to_owned(),
            // Store receives symbolic proof only; the Workflow tests qualify
            // actual owned file identity and bytes on the supported platform.
            directory_device: 1,
            directory_inode: 2,
            file_device: 1,
            file_inode: 3,
            byte_len: bytes.len() as u64,
            sha256: hash(bytes),
        },
    }
}
#[test]
fn binary_catalog_is_metadata_only_exact_bounded_and_restartable() {
    let data = fixture();
    let copies = fixture();
    let bytes = vec![0x80; MAX_INBOX_BINARY_BYTES];
    let capture = capture(copies.path(), &bytes);
    capture.validate().unwrap();
    assert_eq!(capture.copy_name(), format!("{}.bin", capture.id));
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let item = store.capture_inbox(&capture).unwrap();
    assert_eq!(store.capture_inbox(&capture).unwrap(), item);
    assert!(serde_json::to_vec(&item).unwrap().len() < 2048);
    for mutation in 0..5 {
        let mut fork = capture.clone();
        match mutation {
            0 => fork.copy.sha256[0] ^= 1,
            1 => fork.original_name = None,
            2 => fork.title.push('x'),
            3 => fork.copy.file_inode += 1,
            _ => fork.copy.byte_len -= 1,
        }
        assert!(store.capture_inbox(&fork).is_err());
    }
    let mut overflow = capture.clone();
    overflow.copy.byte_len += 1;
    assert!(overflow.validate().is_err());
    let mut text = capture.clone();
    text.kind = InboxKind::Text;
    assert!(text.validate().is_err());
    for kind in [
        InboxKind::Text,
        InboxKind::Markdown,
        InboxKind::Email,
        InboxKind::Teams,
    ] {
        text.kind = kind;
        assert_eq!(text.copy_name(), format!("{}.txt", text.id));
        text.copy.byte_len = 1;
        text.validate().unwrap();
    }
    drop(store);
    let (store, _) = WorkStore::open(data.path()).unwrap();
    assert_eq!(store.inbox_item(capture.id).unwrap(), Some(item));
    assert_eq!(
        std::fs::read(copies.path().join(capture.copy_name())).unwrap(),
        bytes
    );
}
#[test]
fn binary_processing_source_and_text_shaped_provenance_refuse_before_writes() {
    let data = fixture();
    let copies = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let item = store
        .capture_inbox(&capture(copies.path(), b"UTF-8-looking body"))
        .unwrap();
    let request = ProcessInboxRequest {
        id: Uuid::new_v4(),
        items: vec![item.clone()],
    };
    assert!(
        request
            .validate()
            .unwrap_err()
            .to_string()
            .contains("Binary")
    );
    assert!(store.process_inbox(&request).is_err());
    assert!(store.inbox_processing(request.id).unwrap().is_none());
    let forged = InboxProcessBatch {
        request,
        queued_at_ms: 1,
        entries: vec![InboxProcessEntry {
            outcome: InboxProcessOutcome::Converted {
                format: InboxConversionFormat::LiteralTextV1,
                byte_len: 64,
                sha256: hash(b"fake"),
            },
            started_at_ms: Some(1),
            finished_at_ms: Some(1),
        }],
    };
    assert!(forged.validate().is_err());
    assert_eq!(
        convert_original(InboxKind::Binary, "body", &AtomicBool::new(false)),
        Err(InboxProcessOutcome::Failed {
            code: "binary_unsupported".into()
        })
    );
    assert_eq!(
        convert_original(InboxKind::Binary, "body", &AtomicBool::new(true)),
        Err(InboxProcessOutcome::Cancelled)
    );
    let binding = InboxSourceBinding {
        original: item,
        batch_id: Uuid::new_v4(),
        index: 0,
        note_id: Uuid::new_v4(),
        format: InboxConversionFormat::LiteralTextV1,
        byte_len: 64,
        sha256: hash(b"fake"),
    };
    assert!(
        binding
            .validate()
            .unwrap_err()
            .to_string()
            .contains("Binary")
    );
    assert!(binding.markdown("fake").is_err());
    let text = format!(
        "---\nbrn_inbox_source: {}\n---\nbody\n",
        serde_json::to_string(&binding.provenance()).unwrap()
    );
    assert!(
        read_provenance(&text)
            .unwrap_err()
            .to_string()
            .contains("Binary")
    );
}
#[test]
fn old_text_capture_literal_wire_and_new_kind_are_distinct() {
    let capture = InboxCapture {
        id: Uuid::from_u128(1),
        kind: InboxKind::Email,
        title: "Text".into(),
        original_name: None,
        copy: InboxCopy {
            directory: "/synthetic/inbox".into(),
            directory_device: 1,
            directory_inode: 2,
            file_device: 1,
            file_inode: 3,
            byte_len: 1,
            sha256: [0; 32],
        },
    };
    let literal = concat!(
        "{\"id\":\"00000000-0000-0000-0000-000000000001\",\"kind\":\"email\",\"title\":\"Text\",\"original_name\":null,",
        "\"copy\":{\"directory\":\"/synthetic/inbox\",\"directory_device\":1,\"directory_inode\":2,\"file_device\":1,\"file_inode\":3,\"byte_len\":1,\"sha256\":[",
        "0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}}"
    );
    assert_eq!(serde_json::to_string(&capture).unwrap(), literal);
    assert_eq!(
        serde_json::from_str::<InboxCapture>(literal).unwrap(),
        capture
    );
    assert_eq!(
        serde_json::to_string(&InboxKind::Binary).unwrap(),
        "\"binary\""
    );
    let mut empty = capture;
    empty.kind = InboxKind::Binary;
    empty.copy.byte_len = 0;
    assert!(empty.validate().is_err());
    empty.copy.sha256 = hash(&[]);
    empty.validate().unwrap();
}
