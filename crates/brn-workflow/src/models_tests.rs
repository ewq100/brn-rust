use crate::{
    ErrorKind,
    app::{App, AppConfig},
    library::{Embedder, SearchMode, SharedEmbedder},
};
use std::sync::atomic::AtomicBool;

struct SyntheticModel;
impl Embedder for SyntheticModel {
    fn identity(&self) -> &str {
        "synthetic-downloaded-model"
    }
    fn dimension(&self) -> usize {
        2
    }
    fn embed(&mut self, texts: &[&str]) -> brn_retrieval::Result<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|_| vec![1.0, 0.5]).collect())
    }
}

#[test]
fn synthetic_owned_install_job_defers_activation_until_tool_handles_are_drained() {
    let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = base.path().join("data");
    let vault = base.path().join("vault");
    std::fs::create_dir(&data).unwrap();
    std::fs::create_dir(&vault).unwrap();
    std::fs::write(vault.join("a.md"), b"apple").unwrap();
    let mut app = App::open(
        &data,
        AppConfig {
            vault_root: Some(vault.clone()),
            credentials_dir: Some(base.path().join("credentials")),
            model_dir: None,
        },
    )
    .unwrap();
    let target = data.join("models/minilm");
    let job = app.prepare_model_download(true, &target).unwrap().unwrap();
    let report = job
        .install_with(
            &AtomicBool::new(false),
            |_, _| {},
            |target, _, _| {
                std::fs::create_dir_all(target).unwrap();
                std::fs::write(target.join("synthetic-assets"), b"not onnx").unwrap();
                Ok(crate::models::ModelInstallReport {
                    directory: target.to_owned(),
                    downloaded_bytes: 8,
                })
            },
        )
        .unwrap();
    assert_eq!(report.directory, target);
    assert!(!app.model_installed());
    let tools = app.tools().unwrap();
    assert_eq!(
        app.activate_embedder(SharedEmbedder::new(Box::new(SyntheticModel)))
            .unwrap_err()
            .kind,
        ErrorKind::ToolsBusy
    );
    assert!(!app.model_installed());
    drop(tools);
    app.activate_embedder(SharedEmbedder::new(Box::new(SyntheticModel)))
        .unwrap();
    assert_eq!(
        app.activate_model(&target).unwrap_err().kind,
        ErrorKind::ModelInvalid
    );
    assert!(
        app.model_installed(),
        "failed replacement never discards the old loaded model"
    );
    app.embed_pending(2).unwrap();
    assert!(
        !app.search("apple", SearchMode::Hybrid, 10)
            .unwrap()
            .keyword_only
    );
    assert!(
        !brn_ai::ReadTools::search_notes(&*app.tools().unwrap(), "apple", 10)
            .unwrap()
            .keyword_only
    );
    assert_eq!(std::fs::read(vault.join("a.md")).unwrap(), b"apple");
}
