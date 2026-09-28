#![cfg(feature = "native")]
use brn_retrieval_trial::native::reopen;
use std::time::{SystemTime, UNIX_EPOCH};
fn unique_path(kind: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "brn-retrieval-{kind}-{}-{nonce}",
        std::process::id()
    ))
}

#[tokio::test]
async fn missing_state_is_rejected_without_download_or_fallback() {
    let path = unique_path("missing");
    assert!(reopen(&path).await.is_err());
}

#[tokio::test]
async fn incomplete_and_incompatible_states_fail_closed() {
    let root = unique_path("corrupt");
    std::fs::create_dir(&root).unwrap();
    assert!(reopen(&root).await.is_err());
    std::fs::write(root.join("COMPLETE"), b"complete").unwrap();
    std::fs::write(root.join("manifest.json"), b"{\"format\":999,\"model\":\"Qdrant/all-MiniLM-L6-v2-onnx\",\"dimensions\":384,\"pooling\":\"mean\",\"runtime\":\"fastembed-7.1.0\",\"docs_sha256\":\"\",\"files_sha256\":{},\"db_sha256\":{}}").unwrap();
    assert!(reopen(&root).await.is_err());
    std::fs::remove_dir_all(&root).unwrap();
}

#[tokio::test]
async fn build_rejects_existing_path_before_model_download() {
    let root = unique_path("existing");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("sentinel"), b"untouched").unwrap();
    assert!(brn_retrieval_trial::native::build(&root).await.is_err());
    assert_eq!(std::fs::read(root.join("sentinel")).unwrap(), b"untouched");
    std::fs::remove_dir_all(&root).unwrap();
}
