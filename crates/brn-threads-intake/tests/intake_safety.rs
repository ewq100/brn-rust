#[cfg(any(target_os = "macos", all(target_os = "linux", target_arch = "x86_64")))]
use brn_threads_intake::*;
#[cfg(any(target_os = "macos", all(target_os = "linux", target_arch = "x86_64")))]
use std::sync::atomic::AtomicBool;

#[cfg(any(target_os = "macos", all(target_os = "linux", target_arch = "x86_64")))]
#[test]
fn restart_expiry_removes_only_owned_old_runs_and_does_not_follow_links() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let converter = Converter {
        temp_root: root.path().canonicalize().unwrap().join("transient"),
        limits: Default::default(),
    };
    std::fs::create_dir_all(&converter.temp_root).unwrap();
    let abandoned = converter
        .temp_root
        .join(format!("threads-intake-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&abandoned).unwrap();
    std::fs::write(abandoned.join("input.pdf"), b"excluded-canary").unwrap();
    let dir = std::fs::File::open(&abandoned).unwrap();
    dir.set_times(std::fs::FileTimes::new().set_modified(std::time::SystemTime::UNIX_EPOCH))
        .unwrap();
    let external = root.path().join("external");
    std::fs::create_dir(&external).unwrap();
    std::fs::write(external.join("source"), b"keep").unwrap();
    symlink(
        &external,
        converter
            .temp_root
            .join(format!("threads-intake-{}", uuid::Uuid::new_v4())),
    )
    .unwrap();
    std::fs::create_dir(converter.temp_root.join("unrelated")).unwrap();
    converter
        .convert_document(
            ConversionRequest {
                kind: DocumentKind::Text,
                intent: ImportIntent::FullNote,
                bytes: b"current cleanup witness",
                source_reference: "synthetic://cleanup",
            },
            &DocumentHelpers::discover().expect("declared helpers required"),
            "xberg",
            &AtomicBool::new(false),
        )
        .unwrap();
    assert_eq!(std::fs::read_dir(&converter.temp_root).unwrap().count(), 2);
    assert!(!abandoned.exists());
    assert!(external.join("source").exists());
    assert!(converter.temp_root.join("unrelated").exists());
}

#[test]
fn source_file_read_is_bounded_and_cancelled_before_conversion() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("input.txt");
    std::fs::write(&path, b"00123\n").unwrap();
    let running = std::sync::atomic::AtomicBool::new(false);
    assert_eq!(
        brn_threads_intake::read_input_file(&path, 6, &running).unwrap(),
        b"00123\n"
    );
    assert_eq!(
        brn_threads_intake::read_input_file(&path, 5, &running),
        Err(brn_threads_intake::ConversionError::Budget)
    );
    assert_eq!(
        brn_threads_intake::read_input_file(&path, 6, &std::sync::atomic::AtomicBool::new(true)),
        Err(brn_threads_intake::ConversionError::Cancelled)
    );
    assert_eq!(
        brn_threads_intake::read_input_file(root.path(), 6, &running),
        Err(brn_threads_intake::ConversionError::Budget)
    );
}
