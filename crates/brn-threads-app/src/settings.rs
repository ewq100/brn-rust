use crate::*;
use std::path::PathBuf;

/// The owner's caveat shown wherever the helper model is chosen.
pub const HELPER_TESTED_NOTE: &str = "Experimental: helper sub-agents are tested only with gpt-6-luna at high effort; other models may give unreliable results. Best for long-document digests; runs that delegate are slower";
/// Whether a workspace created now gets the derived Codex sign-in file.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum DefaultCredentials {
    /// `CODEX_HOME/auth.json`, else `HOME/.codex/auth.json` (desktop).
    Derive,
    /// No credential source until an explicit `configure` (the CLI).
    None,
}
impl Workspace {
    pub fn settings(&self) -> AppResult<WorkspaceSettings> {
        self.store
            .workspace_settings()?
            .ok_or_else(|| AppError::Invalid("Workspace configuration is missing".into()))
    }
    pub(crate) fn bootstrap_settings(&mut self, credentials: DefaultCredentials) -> AppResult<()> {
        if self
            .records()?
            .iter()
            .any(|record| matches!(record.data, RecordData::Settings(_)))
        {
            return Ok(());
        }
        let op = self
            .store
            .allocate_operation("workspace-bootstrap-settings")?;
        if self.store.prepared(&op).is_ok() {
            applied_receipt(
                self.store
                    .apply(&op, &HostAuthority::owner("workspace bootstrap"))?,
            )?;
            return Ok(());
        }
        let auth = match credentials {
            DefaultCredentials::Derive => std::env::var_os("CODEX_HOME")
                .map(PathBuf::from)
                .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
                .map(|home| home.join("auth.json")),
            DefaultCredentials::None => None,
        };
        let settings = WorkspaceSettings {
            document_conversion: Default::default(),
            mcp: None,
            skills: vec![],
            provider: "Codex".into(),
            model: "gpt-6.1-sol".into(),
            effort: "medium".into(),
            auth_file: auth.map(|path| path.to_string_lossy().into_owned()),
            credentials_dir: None,
            maintenance: false,
            review_first: false,
            delegation_chosen: false,
            backup_directory: None,
            theme: None,
            topics_note: None,
            helper: None,
        };
        let request = ChangeRequest {
            reason: "Initial workspace configuration".into(),
            writes: vec![Put {
                id: op.creation_id(0),
                expected_version: None,
                archived: false,
                data: RecordData::Settings(settings),
            }],
            inputs: vec![],
        };
        self.store
            .prepare_owner(&op, &request, &HostAuthority::owner("workspace bootstrap"))?;
        applied_receipt(
            self.store
                .apply(&op, &HostAuthority::owner("workspace bootstrap"))?,
        )?;
        Ok(())
    }
    pub fn configure(&mut self, settings: WorkspaceSettings) -> AppResult<ApplyOutcome> {
        if settings.provider.trim().is_empty()
            || settings.model.trim().is_empty()
            || settings.effort.trim().is_empty()
        {
            return Err(AppError::Invalid(
                "Choose a provider, model and reasoning effort".into(),
            ));
        }
        for path in [&settings.auth_file, &settings.credentials_dir]
            .into_iter()
            .flatten()
        {
            if !Path::new(path).is_absolute() {
                return Err(AppError::Invalid(
                    "Credential locations must be absolute paths".into(),
                ));
            }
        }
        if let Some(helper) = &settings.helper {
            validate_helper(helper)?;
        }
        let binding = format!("{:x}", Sha256::digest(serde_json::to_vec(&settings)?));
        let op = self
            .store
            .allocate_bound_operation(&format!("configure:{}", uuid::Uuid::new_v4()), &binding)?;
        // Each settings change below stops Working Runs; a stopped Run's free
        // lease file goes with it, and a live one is kept.
        let outcome = self.store.configure_workspace(
            &op,
            &settings,
            &HostAuthority::owner("owner configuration"),
        )?;
        Ok(self.remove_leases_ended_by(outcome))
    }
    pub fn choose_document_conversion(
        &mut self,
        settings: DocumentConversionSettings,
    ) -> AppResult<ApplyOutcome> {
        let op = self
            .store
            .allocate_operation(&format!("document-conversion:{}", uuid::Uuid::new_v4()))?;
        let outcome = self.store.choose_document_conversion(
            &op,
            &settings,
            &HostAuthority::owner("owner document conversion choice"),
        )?;
        Ok(self.remove_leases_ended_by(outcome))
    }
    pub fn choose_delegation(&mut self, choice: DelegationChoice) -> AppResult<ApplyOutcome> {
        let binding = format!("{:x}", Sha256::digest(serde_json::to_vec(&choice)?));
        let op = self.store.allocate_bound_operation(
            &format!("delegation-choice:{}", uuid::Uuid::new_v4()),
            &binding,
        )?;
        let outcome = self.store.choose_delegation(
            &op,
            choice,
            &HostAuthority::owner("owner delegation choice"),
        )?;
        Ok(self.remove_leases_ended_by(outcome))
    }
    pub fn choose_backup_directory(&mut self, directory: &Path) -> AppResult<ApplyOutcome> {
        let op = self
            .store
            .allocate_operation(&format!("backup-directory:{}", uuid::Uuid::new_v4()))?;
        let outcome = self.store.choose_backup_directory(
            &op,
            directory,
            &HostAuthority::owner("owner backup folder selection"),
        )?;
        Ok(self.remove_leases_ended_by(outcome))
    }

    /// Reload and validate the preference for this workspace, then publish a fresh backup.
    pub fn backup_to_saved_directory(&self) -> AppResult<PathBuf> {
        let settings = self.settings()?;
        let directory = settings
            .backup_directory
            .ok_or_else(|| AppError::Invalid("Choose a backup folder first".into()))?;
        let directory = self
            .store
            .validate_backup_directory(Path::new(&directory))?;
        let destination = directory.join(format!("BRN-backup-{}", uuid::Uuid::new_v4()));
        self.backup(&destination)?;
        Ok(destination)
    }

    /// Save the appearance preference into the versioned Settings record;
    /// deferred while any run is working. Retained for callers that need an
    /// ordinary settings revision; the desktop appearance preference now uses
    /// the non-fencing [`Workspace::save_appearance`] path instead.
    pub fn choose_theme(&mut self, theme: &str) -> AppResult<ApplyOutcome> {
        let op = self
            .store
            .allocate_operation(&format!("theme:{}", uuid::Uuid::new_v4()))?;
        Ok(self
            .store
            .choose_theme(&op, theme, &HostAuthority::owner("owner theme choice"))?)
    }

    /// The owner's saved appearance preference key, or `None` when none has
    /// been saved through the presentation path. For a workspace last written
    /// before this path existed, the host may fall back to the legacy
    /// [`WorkspaceSettings::theme`] value once; see
    /// [`Workspace::adopt_legacy_theme`].
    pub fn appearance(&self) -> AppResult<Option<String>> {
        Ok(self.store.presentation_theme()?)
    }

    /// Save the owner's appearance preference. Presentation only: it never
    /// touches the versioned Settings record, never fences or interrupts a
    /// working run, and persists durably before returning (THEME-02).
    pub fn save_appearance(&mut self, theme: &str) -> AppResult<()> {
        Ok(self.store.save_presentation_theme(theme)?)
    }

    /// The desktop's last workspace location key, if any. Host
    /// presentation state only: restoring it must never resume work.
    pub fn last_location(&self) -> AppResult<Option<String>> {
        Ok(self.store.presentation_location()?)
    }

    /// Save the desktop's last workspace location key. Presentation only,
    /// like [`Workspace::save_appearance`].
    pub fn save_last_location(&mut self, location: &str) -> AppResult<()> {
        Ok(self.store.save_presentation_location(location)?)
    }

    /// The owner's favorite chat models as `(provider, model)` pairs.
    /// Presentation only: a favorite never selects a model or grants a route.
    pub fn favorite_models(&self) -> AppResult<Vec<(String, String)>> {
        Ok(self.store.presentation_model_favorites()?)
    }

    /// Replace the favorite chat models. Presentation only, like
    /// [`Workspace::save_appearance`].
    pub fn save_favorite_models(&mut self, favorites: &[(String, String)]) -> AppResult<()> {
        Ok(self.store.save_presentation_model_favorites(favorites)?)
    }

    /// One-time migration: if no presentation preference has been saved yet but
    /// the workspace carries a legacy [`WorkspaceSettings::theme`], adopt it as
    /// the presentation preference so an owner's earlier choice is not reset.
    /// Returns the adopted key, if any. Never overwrites an existing
    /// presentation preference.
    pub fn adopt_legacy_theme(&mut self) -> AppResult<Option<String>> {
        if self.store.presentation_theme()?.is_some() {
            return Ok(None);
        }
        let Some(legacy) = self.settings()?.theme else {
            return Ok(None);
        };
        self.store.save_presentation_theme(&legacy)?;
        Ok(Some(legacy))
    }

    pub fn pause(&mut self, paused: bool) -> AppResult<ApplyOutcome> {
        let binding = format!("{:x}", Sha256::digest(serde_json::to_vec(&paused)?));
        let op = self
            .store
            .allocate_bound_operation(&format!("pause:{}", uuid::Uuid::new_v4()), &binding)?;
        let outcome = self.store.pause_workspace(
            &op,
            paused,
            &HostAuthority::owner("owner maintenance pause"),
        )?;
        Ok(self.remove_leases_ended_by(outcome))
    }
}

/// The helper is an explicit owner selection with its own explicit credentials.
fn validate_helper(helper: &brn_threads_core::HelperSettings) -> AppResult<()> {
    let invalid = |message: &str| Err(AppError::Invalid(message.into()));
    let Some(provider) = brn_ai::Provider::from_setting(&helper.provider) else {
        return invalid("Choose Codex or Copilot for the helper");
    };
    // Saved helper validation has always checked trimmed syntax while keeping
    // the original bytes. Runtime selection still validates the exact model.
    if (brn_ai::Selection {
        provider,
        model: helper.model.trim().to_owned(),
    })
    .validate()
    .is_err()
    {
        return invalid("Choose a helper model");
    }
    if brn_ai::ReasoningEffort::from_setting(&helper.effort).is_none() {
        return invalid("Choose low, medium or high helper effort");
    }
    for path in [&helper.auth_file, &helper.credentials_dir]
        .into_iter()
        .flatten()
    {
        if !Path::new(path).is_absolute() {
            return invalid("Credential locations must be absolute paths");
        }
    }
    match (
        provider == brn_ai::Provider::Chatgpt,
        &helper.auth_file,
        &helper.credentials_dir,
    ) {
        (true, Some(_), None) | (true, None, Some(_)) | (false, None, Some(_)) => Ok(()),
        (false, Some(_), _) => invalid("Codex sign-in files apply only to a Codex helper"),
        _ => invalid("Choose one credential location for the helper"),
    }
}
