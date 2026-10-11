use crate::*;
use std::path::PathBuf;
impl Workspace {
    pub fn settings(&self) -> AppResult<WorkspaceSettings> {
        self.records()?
            .into_iter()
            .find_map(|record| {
                if let RecordData::Settings(settings) = record.data {
                    Some(settings)
                } else {
                    None
                }
            })
            .ok_or_else(|| AppError::Invalid("Workspace configuration is missing".into()))
    }
    pub(crate) fn bootstrap_settings(&mut self) -> AppResult<()> {
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
        let auth = std::env::var_os("CODEX_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
            .map(|home| home.join("auth.json"));
        let settings = WorkspaceSettings {
            provider: "Codex".into(),
            model: "gpt-6.1-sol".into(),
            effort: "medium".into(),
            auth_file: auth.map(|path| path.to_string_lossy().into_owned()),
            credentials_dir: None,
            maintenance: true,
            review_first: false,
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
    pub fn pause(&mut self, paused: bool) -> AppResult<ApplyOutcome> {
        let mut settings = self.settings()?;
        settings.maintenance = !paused;
        self.configure(settings)
    }
}
