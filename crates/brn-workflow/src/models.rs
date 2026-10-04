//! Consent and owned installation work; the application lane activates only after tools drain.
use crate::{ErrorKind, Result, WorkflowError, app::App};
use std::{
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

pub const MODEL_SOURCE: &str = "https://huggingface.co/Xenova/paraphrase-multilingual-MiniLM-L12-v2/resolve/2c4055b12046f11709e9df2c122e59ffbdc2f900/";
pub const MODEL_BYTES: u64 = 135_392_488;
pub const MODEL_RELATIVE_DIR: &str = "models/multilingual-minilm-l12-v2";
pub const MODEL_COST: &str = "Approximately 129 MiB of network transfer and installed storage";
pub const MODEL_DECISION_KEY: &str =
    "model.download_decision.2c4055b12046f11709e9df2c122e59ffbdc2f900";

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadDecision {
    Approved,
    Declined,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelDownloadPrompt {
    pub source: String,
    pub bytes: u64,
    pub cost: String,
    pub destination: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ModelInstallReport {
    pub directory: PathBuf,
    pub downloaded_bytes: u64,
}

/// Non-cloneable fresh approval, consumed by one owned blocking install job.
pub struct ModelInstallRequest {
    target: PathBuf,
}

impl ModelInstallRequest {
    #[cfg(test)]
    pub(crate) fn test_request(target: PathBuf) -> Self {
        Self { target }
    }
    pub fn target(&self) -> &Path {
        &self.target
    }

    pub fn install(
        self,
        cancel: &AtomicBool,
        progress: impl Fn(u64, u64),
    ) -> Result<ModelInstallReport> {
        #[cfg(feature = "native-retrieval")]
        {
            self.install_with(cancel, progress, |target, cancel, progress| {
                let report =
                    brn_retrieval::native::download::download_model(target, cancel, progress)
                        .map_err(|error| {
                            if matches!(error, brn_retrieval::Error::Cancelled) {
                                WorkflowError::cancelled()
                            } else {
                                WorkflowError::typed(
                                    ErrorKind::ModelDownloadFailed,
                                    "model installation failed",
                                )
                            }
                        })?;
                Ok(ModelInstallReport {
                    directory: report.directory,
                    downloaded_bytes: report.downloaded_bytes,
                })
            })
        }
        #[cfg(not(feature = "native-retrieval"))]
        {
            let _ = (cancel, progress);
            Err(unavailable())
        }
    }

    #[cfg(feature = "native-retrieval")]
    pub(super) fn install_with(
        self,
        cancel: &AtomicBool,
        progress: impl Fn(u64, u64),
        install: impl FnOnce(&Path, &AtomicBool, &dyn Fn(u64, u64)) -> Result<ModelInstallReport>,
    ) -> Result<ModelInstallReport> {
        install(&self.target, cancel, &progress)
    }
}

impl App {
    pub fn model_download_decision(&self) -> Result<Option<DownloadDecision>> {
        self.work_store()
            .setting(MODEL_DECISION_KEY)?
            .map(|decision| match decision.as_str() {
                "approved" => Ok(DownloadDecision::Approved),
                "declined" => Ok(DownloadDecision::Declined),
                _ => Err(WorkflowError::typed(
                    ErrorKind::ModelInvalid,
                    "stored model consent is invalid",
                )),
            })
            .transpose()
    }

    pub fn model_download_prompt(&self) -> Result<Option<ModelDownloadPrompt>> {
        #[cfg(not(feature = "native-retrieval"))]
        {
            Ok(None)
        }
        #[cfg(feature = "native-retrieval")]
        {
            if self.model_installed() || self.model_download_decision()?.is_some() {
                return Ok(None);
            }
            Ok(Some(ModelDownloadPrompt {
                source: MODEL_SOURCE.into(),
                bytes: MODEL_BYTES,
                cost: MODEL_COST.into(),
                destination: self.work_store().data_dir().join(MODEL_RELATIVE_DIR),
            }))
        }
    }

    /// Never runs network work, even for persisted approval. Every retry needs a new explicit action.
    pub fn prepare_model_download(
        &mut self,
        consent: bool,
        target: &Path,
    ) -> Result<Option<ModelInstallRequest>> {
        if !consent {
            self.work_store_mut()
                .set_setting(MODEL_DECISION_KEY, "declined")?;
            return Ok(None);
        }
        #[cfg(not(feature = "native-retrieval"))]
        {
            let _ = target;
            Err(unavailable())
        }
        #[cfg(feature = "native-retrieval")]
        {
            self.validate_model_target(target)?;
            self.work_store_mut()
                .set_setting(MODEL_DECISION_KEY, "approved")?;
            Ok(Some(ModelInstallRequest {
                target: target.to_owned(),
            }))
        }
    }

    /// Caller emits ModelDownloaded before this and ModelInstalled only on success.
    /// Drop idle chat/tool handles before calling; an active snapshot is never swapped.
    pub fn activate_model(&mut self, directory: &Path) -> Result<()> {
        self.activate_model_cancellable(directory, &AtomicBool::new(false))
    }

    pub(crate) fn activate_model_cancellable(
        &mut self,
        directory: &Path,
        cancel: &AtomicBool,
    ) -> Result<()> {
        self.activate_model_with(directory, cancel, |data, directory| {
            crate::app::load_model(data, Some(directory))
        })
    }

    pub(crate) fn activate_model_with(
        &mut self,
        directory: &Path,
        cancel: &AtomicBool,
        load: impl FnOnce(&Path, &Path) -> Result<Option<crate::library::SharedEmbedder>>,
    ) -> Result<()> {
        if cancel.load(Ordering::Acquire) {
            return Err(WorkflowError::cancelled());
        }
        self.validate_model_target(directory)?;
        self.ensure_tools_drained()?;
        let embedder = load(self.work_store().data_dir(), directory)?.ok_or_else(|| {
            WorkflowError::typed(
                ErrorKind::ModelInvalid,
                "downloaded model directory is missing",
            )
        })?;
        // Loading is synchronous; cancellation before publication leaves the old model intact.
        if cancel.load(Ordering::Acquire) {
            return Err(WorkflowError::cancelled());
        }
        self.activate_embedder(embedder)?;
        self.work_store_mut().set_setting(
            "model.directory",
            directory.to_str().ok_or_else(|| {
                WorkflowError::typed(ErrorKind::ModelInvalid, "model directory is not UTF-8")
            })?,
        )?;
        Ok(())
    }

    fn validate_model_target(&self, target: &Path) -> Result<()> {
        let invalid = || {
            WorkflowError::typed(
                ErrorKind::ModelInvalid,
                "model destination must be absolute, outside repositories and the vault, without symlinks",
            )
        };
        if !target.is_absolute()
            || target.file_name().is_none()
            || target
                .components()
                .any(|c| !matches!(c, Component::RootDir | Component::Normal(_)))
        {
            return Err(invalid());
        }
        for ancestor in target.ancestors() {
            match ancestor.join(".git").symlink_metadata() {
                Ok(_) => return Err(invalid()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => return Err(invalid()),
            }
            match ancestor.symlink_metadata() {
                Ok(meta) if meta.file_type().is_symlink() => return Err(invalid()),
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => return Err(invalid()),
            }
        }
        if self
            .vault_root()
            .is_some_and(|root| target.starts_with(root) || root.starts_with(target))
        {
            return Err(invalid());
        }
        Ok(())
    }
}

#[cfg(not(feature = "native-retrieval"))]
fn unavailable() -> WorkflowError {
    WorkflowError::typed(
        ErrorKind::SemanticUnavailableInBuild,
        "SEMANTIC_UNAVAILABLE_IN_BUILD",
    )
}
