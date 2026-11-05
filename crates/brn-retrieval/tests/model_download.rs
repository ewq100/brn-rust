#![cfg(feature = "native")]
use brn_retrieval::{
    Error,
    native::download::{ModelInstallError, download_model},
};
use std::sync::atomic::AtomicBool;

#[test]
fn cancelled_pinned_install_does_not_fetch_or_create_assets() {
    let data = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let target = data.path().join("model");
    assert!(matches!(
        download_model(&target, &AtomicBool::new(true), |_, _| {}),
        Err(Error::Cancelled)
    ));
    assert!(!target.exists());
    assert_eq!(std::fs::read_dir(data.path()).unwrap().count(), 0);
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn occupied_target_is_refused_without_network_or_overwrite() {
    let data = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let target = data.path().join("model");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("model.onnx"), b"synthetic sentinel").unwrap();
    assert!(matches!(
        download_model(&target, &AtomicBool::new(false), |_, _| {}),
        Err(Error::ModelInstall(ModelInstallError::TargetOccupied))
    ));
    assert_eq!(
        std::fs::read(target.join("model.onnx")).unwrap(),
        b"synthetic sentinel"
    );
    assert_eq!(std::fs::read_dir(data.path()).unwrap().count(), 1);
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
#[test]
fn unsupported_pinned_install_preserves_existing_and_absent_destinations() {
    let data = tempfile::tempdir().unwrap();
    let occupied = data.path().join("model");
    std::fs::create_dir(&occupied).unwrap();
    std::fs::write(occupied.join("sentinel"), b"keep").unwrap();
    for target in [occupied.clone(), data.path().join("absent-parent/model")] {
        assert!(matches!(
            download_model(&target, &AtomicBool::new(false), |_, _| panic!(
                "unsupported progress"
            )),
            Err(Error::ModelInstall(
                ModelInstallError::ExclusiveInstallUnavailable
            ))
        ));
    }
    assert_eq!(std::fs::read(occupied.join("sentinel")).unwrap(), b"keep");
    assert_eq!(std::fs::read_dir(&occupied).unwrap().count(), 1);
    assert_eq!(std::fs::read_dir(data.path()).unwrap().count(), 1);
}
