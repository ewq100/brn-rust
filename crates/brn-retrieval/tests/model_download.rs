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
