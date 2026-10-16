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
        self.records()?
            .into_iter()
            .find_map(|record| {
                if !record.archived
                    && let RecordData::Settings(settings) = record.data
                {
                    Some(settings)
                } else {
                    None
                }
            })
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
        Ok(self.store.configure_workspace(
            &op,
            &settings,
            &HostAuthority::owner("owner configuration"),
        )?)
    }
    pub fn choose_delegation(&mut self, choice: DelegationChoice) -> AppResult<ApplyOutcome> {
        let binding = format!("{:x}", Sha256::digest(serde_json::to_vec(&choice)?));
        let op = self.store.allocate_bound_operation(
            &format!("delegation-choice:{}", uuid::Uuid::new_v4()),
            &binding,
        )?;
        Ok(self.store.choose_delegation(
            &op,
            choice,
            &HostAuthority::owner("owner delegation choice"),
        )?)
    }
    pub fn choose_backup_directory(&mut self, directory: &Path) -> AppResult<ApplyOutcome> {
        let op = self
            .store
            .allocate_operation(&format!("backup-directory:{}", uuid::Uuid::new_v4()))?;
        Ok(self.store.choose_backup_directory(
            &op,
            directory,
            &HostAuthority::owner("owner backup folder selection"),
        )?)
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

    /// Save the appearance preference; deferred while any run is working.
    pub fn choose_theme(&mut self, theme: &str) -> AppResult<ApplyOutcome> {
        let op = self
            .store
            .allocate_operation(&format!("theme:{}", uuid::Uuid::new_v4()))?;
        Ok(self
            .store
            .choose_theme(&op, theme, &HostAuthority::owner("owner theme choice"))?)
    }

    pub fn pause(&mut self, paused: bool) -> AppResult<ApplyOutcome> {
        let binding = format!("{:x}", Sha256::digest(serde_json::to_vec(&paused)?));
        let op = self
            .store
            .allocate_bound_operation(&format!("pause:{}", uuid::Uuid::new_v4()), &binding)?;
        Ok(self.store.pause_workspace(
            &op,
            paused,
            &HostAuthority::owner("owner maintenance pause"),
        )?)
    }
}

/// The helper is an explicit owner selection with its own explicit credentials.
fn validate_helper(helper: &brn_threads_core::HelperSettings) -> AppResult<()> {
    let invalid = |message: &str| Err(AppError::Invalid(message.into()));
    let codex = ["codex", "chatgpt"]
        .iter()
        .any(|name| helper.provider.eq_ignore_ascii_case(name));
    if !codex && !helper.provider.eq_ignore_ascii_case("copilot") {
        return invalid("Choose Codex or Copilot for the helper");
    }
    let model = helper.model.trim();
    if model.is_empty()
        || model.len() > 128
        || !model
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.:/".contains(&b))
    {
        return invalid("Choose a helper model");
    }
    if !["low", "medium", "high"].contains(&helper.effort.as_str()) {
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
    match (codex, &helper.auth_file, &helper.credentials_dir) {
        (true, Some(_), None) | (true, None, Some(_)) | (false, None, Some(_)) => Ok(()),
        (false, Some(_), _) => invalid("Codex sign-in files apply only to a Codex helper"),
        _ => invalid("Choose one credential location for the helper"),
    }
}
