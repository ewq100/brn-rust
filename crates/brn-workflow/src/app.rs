//! The simple application's one owner of user work and the current vault.
use crate::{
    ErrorKind, Result, WorkflowError,
    ai_tools::{AiTools, note_page_scoped, validate_hits_scoped},
    library::{
        EmbeddingProgress, KnowledgeScope, Library, RefreshReport, SearchMode, SearchResults,
        SharedEmbedder, saved_metadata,
    },
    vault::{self, NoteText, VaultPath},
};
use brn_ai::{Auth, ModelOption, NotePage, Provider, ReasoningEffort, Selection};
use brn_store::{
    OpenReport, WorkStore,
    work::{WorkConversation, WorkTurn},
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct AppConfig {
    pub vault_root: Option<PathBuf>,
    /// None reopens the saved non-secret location, or derives the safe sibling.
    pub credentials_dir: Option<PathBuf>,
    pub model_dir: Option<PathBuf>,
}

pub struct App {
    // Readers close before the writer checkpoints; the owner lock is released last.
    tools: Option<Arc<AiTools>>,
    library: Option<Library>,
    auth: Arc<Auth>,
    pub(crate) root: Option<PathBuf>,
    pub(crate) editor: crate::editor::EditorState,
    embedder: Option<SharedEmbedder>,
    report: OpenReport,
    pub(crate) store: WorkStore,
    pub(crate) apply_records: Option<crate::files::recovery::ApplyRecoveryFiles>,
    pub(crate) completion_uncertain: bool,
    pub(crate) inbox: crate::inbox::InboxState,
}

impl App {
    pub fn open(data_dir: &Path, config: AppConfig) -> Result<Self> {
        Self::open_with_model_loader(data_dir, config, load_model)
    }

    pub(crate) fn open_with_model_loader(
        data_dir: &Path,
        config: AppConfig,
        load: impl FnOnce(&Path, Option<&Path>) -> Result<Option<SharedEmbedder>>,
    ) -> Result<Self> {
        if !data_dir.is_absolute() {
            return Err(WorkflowError::msg("data directory must be absolute"));
        }
        let data_dir = data_dir
            .canonicalize()
            .map_err(|e| WorkflowError::msg(e.to_string()))?;
        if let Some(root) = &config.vault_root
            && root
                .try_exists()
                .map_err(|e| WorkflowError::msg(e.to_string()))?
        {
            validate_vault_separation(&data_dir, &root.canonicalize().map_err(|_| unavailable())?)?;
        }
        let (mut store, report) = WorkStore::open(&data_dir)?;
        let apply_records = crate::proposal_apply::restore_application_records(
            &mut store,
            config.vault_root.as_deref(),
        )?;
        crate::action_completion::restore_action_completions(&mut store, apply_records.as_ref())?;
        let inbox = crate::inbox::restore_inbox_captures(&mut store)?;
        let stored_root = store.setting("vault.root")?.map(PathBuf::from);
        if let (Some(stored), Some(requested)) = (&stored_root, &config.vault_root) {
            let requested = if requested
                .try_exists()
                .map_err(|e| WorkflowError::msg(e.to_string()))?
            {
                requested.canonicalize().map_err(|_| unavailable())?
            } else {
                requested.to_owned()
            };
            if *stored != requested {
                return Err(WorkflowError::typed(
                    ErrorKind::WorkspaceModeConflict,
                    "this data folder is already bound to another vault",
                ));
            }
        }
        let requested_root = config.vault_root.clone().or(stored_root.clone());
        let credentials_dir = config
            .credentials_dir
            .or(store.setting("ai.credentials_dir")?.map(PathBuf::from))
            .map(Ok)
            .unwrap_or_else(|| default_credentials_dir(&data_dir))?;
        validate_credentials(&credentials_dir, requested_root.as_deref(), &data_dir)?;
        let auth = Arc::new(Auth::open(&credentials_dir)?);
        #[cfg(feature = "native-retrieval")]
        let saved_model = store.setting("model.directory")?.map(PathBuf::from);
        #[cfg(not(feature = "native-retrieval"))]
        let saved_model: Option<PathBuf> = None;
        let embedder = load(
            &data_dir,
            config.model_dir.as_deref().or(saved_model.as_deref()),
        )?;
        let mut app = Self {
            store,
            auth,
            library: None,
            root: stored_root,
            tools: None,
            embedder,
            report,
            editor: crate::editor::EditorState::default(),
            apply_records,
            completion_uncertain: false,
            inbox,
        };
        app.store.set_setting(
            "ai.credentials_dir",
            credentials_dir.to_str().ok_or_else(|| {
                WorkflowError::typed(
                    ErrorKind::UnsafeCredentials,
                    "credential path must be UTF-8",
                )
            })?,
        )?;
        app.reconcile_startup_proposals()?;
        if let Some(root) = requested_root.as_deref()
            && root
                .try_exists()
                .map_err(|e| WorkflowError::msg(e.to_string()))?
        {
            app.bind_vault(root)?;
        }
        Ok(app)
    }

    pub fn open_report(&self) -> &OpenReport {
        &self.report
    }
    pub fn auth(&self) -> Arc<Auth> {
        self.auth.clone()
    }
    pub fn work_store(&self) -> &WorkStore {
        &self.store
    }
    pub fn work_store_mut(&mut self) -> &mut WorkStore {
        &mut self.store
    }
    pub fn vault_root(&self) -> Option<&Path> {
        self.root.as_deref()
    }
    pub fn model_installed(&self) -> bool {
        self.embedder.is_some()
    }

    pub(crate) fn ensure_tools_drained(&self) -> Result<()> {
        if self
            .tools
            .as_ref()
            .is_some_and(|tools| Arc::strong_count(tools) > 1)
        {
            return Err(WorkflowError::typed(
                ErrorKind::ToolsBusy,
                "drain and detach current read tools before activating another model",
            ));
        }
        Ok(())
    }

    pub(crate) fn activate_embedder(&mut self, embedder: SharedEmbedder) -> Result<()> {
        self.ensure_tools_drained()?;
        let tools = if let Some(root) = self.library.as_ref().and(self.root.as_ref()) {
            Some(Arc::new(AiTools::open(
                root,
                &self.store.data_dir().join("index.sqlite"),
                Some(embedder.clone()),
            )?))
        } else {
            None
        };
        if let Some(library) = self.library.as_mut() {
            library.replace_embedder(embedder.clone())?;
        }
        if let Some(tools) = &tools {
            tools.set_current_blocked(self.current_evidence_blocked()?);
        }
        self.tools = tools;
        self.embedder = Some(embedder);
        Ok(())
    }

    pub fn bind_vault(&mut self, root: &Path) -> Result<()> {
        let meta = root.symlink_metadata().map_err(|_| unavailable())?;
        if !meta.is_dir() || meta.file_type().is_symlink() {
            return Err(unavailable());
        }
        let root = root.canonicalize().map_err(|_| unavailable())?;
        validate_vault_separation(self.store.data_dir(), &root)?;
        if self.root.as_ref().is_some_and(|bound| *bound != root) {
            return Err(WorkflowError::typed(
                ErrorKind::WorkspaceModeConflict,
                "this data folder is already bound to another vault",
            ));
        }
        // Recheck on first binding: history/settings may have opened without a vault.
        validate_credentials(self.auth_dir(), Some(&root), self.store.data_dir())?;
        if self.library.is_some() {
            return Ok(());
        }
        let index = self.store.data_dir().join("index.sqlite");
        let mut library = Library::open_shared(&root, &index, self.embedder.clone())?;
        let blocked = self.current_evidence_blocked()?;
        if !blocked {
            library.refresh()?;
        }
        let tools = Arc::new(AiTools::open(&root, &index, self.embedder.clone())?);
        let root_text = root.to_str().ok_or_else(unavailable)?;
        self.store.set_setting("vault.root", root_text)?;
        tools.set_current_blocked(blocked);
        self.root = Some(root);
        self.library = Some(library);
        self.tools = Some(tools);
        Ok(())
    }

    fn auth_dir(&self) -> &Path {
        self.auth.credentials_dir()
    }

    pub(crate) fn require_vault(&self) -> Result<&Path> {
        let root = self.root.as_deref().ok_or_else(|| {
            WorkflowError::typed(
                ErrorKind::VaultNotBound,
                "choose a vault before reading notes or asking AI",
            )
        })?;
        if !root
            .symlink_metadata()
            .is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink())
        {
            return Err(unavailable());
        }
        if self.library.is_none() {
            return Err(unavailable());
        }
        Ok(root)
    }

    pub(crate) fn set_current_tool_barrier(&self, blocked: bool) {
        if let Some(tools) = &self.tools {
            tools.set_current_blocked(blocked);
        }
    }

    pub(crate) fn guarded_tools(&self) -> Result<Arc<AiTools>> {
        self.require_vault()?;
        self.tools.clone().ok_or_else(unavailable)
    }

    pub fn tools(&self) -> Result<Arc<AiTools>> {
        self.require_current_evidence()?;
        self.guarded_tools()
    }

    /// Call on startup, explicit Refresh, focus and after application writes.
    pub fn refresh(&mut self) -> Result<RefreshReport> {
        self.require_current_evidence()?;
        self.require_vault()?;
        Ok(self.library.as_mut().ok_or_else(unavailable)?.refresh()?)
    }

    pub(crate) fn cache_relationships(
        &mut self,
        edges: &[brn_retrieval::note_index::NoteEdge],
        request: &crate::knowledge::RelationshipRequest,
    ) -> Result<brn_retrieval::note_index::EdgePage> {
        self.require_current_evidence()?;
        self.require_vault()?;
        Ok(self
            .library
            .as_mut()
            .ok_or_else(unavailable)?
            .relationship_page(edges, request.scope, request.offset, request.limit)?)
    }

    pub fn embed_pending(&mut self, batch: usize) -> Result<Option<EmbeddingProgress>> {
        self.require_current_evidence()?;
        self.require_vault()?;
        Ok(self
            .library
            .as_mut()
            .ok_or_else(unavailable)?
            .embed_pending(batch)?)
    }

    pub fn select(&mut self, selection: Selection) -> Result<()> {
        self.validate_selection(&selection)?;
        let value = serde_json::to_string(&selection)
            .map_err(|_| WorkflowError::msg("could not encode AI selection"))?;
        self.store.set_setting("ai.selection", &value)?;
        Ok(())
    }

    pub fn selection(&self) -> Result<Option<Selection>> {
        self.store
            .setting("ai.selection")?
            .map(|value| {
                let selection: Selection = serde_json::from_str(&value).map_err(|_| {
                    WorkflowError::typed(ErrorKind::ModelRefused, "stored AI selection is invalid")
                })?;
                // Refreshing discovery must not erase the user's explicit saved choice.
                // New requests still use validate_selection for current admission.
                selection.validate()?;
                Ok(selection)
            })
            .transpose()
    }

    /// Explicit user choice, independent of the provider/model selection.
    pub fn select_effort(&mut self, effort: ReasoningEffort) -> Result<()> {
        self.store.set_setting("ai.effort", effort.as_str())?;
        Ok(())
    }

    pub fn effort(&self) -> Result<Option<ReasoningEffort>> {
        self.store
            .setting("ai.effort")?
            .map(|value| match value.as_str() {
                "low" => Ok(ReasoningEffort::Low),
                "medium" => Ok(ReasoningEffort::Medium),
                "high" => Ok(ReasoningEffort::High),
                _ => Err(WorkflowError::typed(
                    ErrorKind::SelectionRequired,
                    "stored reasoning effort is invalid; choose low, medium or high",
                )),
            })
            .transpose()
    }

    /// Records only the result of an explicit model discovery action; never calls a provider.
    pub fn record_models(&mut self, provider: Provider, models: &[ModelOption]) -> Result<()> {
        for model in models {
            Selection {
                provider,
                model: model.id.clone(),
            }
            .validate()?;
        }
        let value = serde_json::to_string(models)
            .map_err(|_| WorkflowError::msg("could not encode discovered models"))?;
        self.store.set_setting(model_key(provider), &value)?;
        Ok(())
    }

    /// Validates explicit turn selections without persistence, credential access or discovery.
    pub fn validate_selection(&self, selection: &Selection) -> Result<()> {
        selection.validate()?;
        if selection.provider == Provider::Copilot {
            let models = self
                .store
                .setting(model_key(Provider::Copilot))?
                .ok_or_else(|| {
                    WorkflowError::typed(
                        ErrorKind::ModelRefused,
                        "discover Copilot models explicitly before selecting one",
                    )
                })?;
            let models: Vec<ModelOption> = serde_json::from_str(&models).map_err(|_| {
                WorkflowError::typed(ErrorKind::ModelRefused, "stored model discovery is invalid")
            })?;
            if !models.iter().any(|m| m.id == selection.model) {
                return Err(WorkflowError::typed(
                    ErrorKind::ModelRefused,
                    "selected model was not discovered for Copilot",
                ));
            }
        }
        Ok(())
    }

    pub fn notes(&mut self, folder: Option<&str>, cursor: Option<&str>) -> Result<NotePage> {
        self.notes_scoped(folder, cursor, KnowledgeScope::Current)
    }

    pub fn notes_scoped(
        &mut self,
        folder: Option<&str>,
        cursor: Option<&str>,
        scope: KnowledgeScope,
    ) -> Result<NotePage> {
        self.refresh()?;
        Ok(note_page_scoped(
            self.require_vault()?,
            self.library
                .as_ref()
                .ok_or_else(unavailable)?
                .notes_scoped(scope)?,
            folder,
            cursor,
            scope,
        )?)
    }

    pub fn note(&self, path: &str) -> Result<NoteText> {
        self.note_scoped(path, KnowledgeScope::Current)
    }

    pub fn note_scoped(&self, path: &str, scope: KnowledgeScope) -> Result<NoteText> {
        self.require_current_evidence()?;
        let root = self.require_vault()?;
        if scope == KnowledgeScope::Current {
            VaultPath::parse(path)
                .map_err(|e| WorkflowError::typed(ErrorKind::ToolRejected, e.to_string()))?;
        }
        let path = vault::EvidencePath::parse(path)
            .map_err(|e| WorkflowError::typed(ErrorKind::ToolRejected, e.to_string()))?;
        let note = vault::read_evidence(root, &path)
            .map_err(|e| WorkflowError::typed(ErrorKind::ToolRejected, e.to_string()))?;
        let metadata = saved_metadata(&note.text, path.as_str());
        if metadata.issue.is_some() || !scope.includes(metadata.source, metadata.history) {
            return Err(WorkflowError::typed(
                ErrorKind::ToolRejected,
                "The saved note is not eligible for the requested knowledge scope.",
            ));
        }
        Ok(note)
    }

    pub fn search(&mut self, query: &str, mode: SearchMode, limit: usize) -> Result<SearchResults> {
        self.search_scoped(query, mode, limit, KnowledgeScope::Current)
    }

    pub fn search_scoped(
        &mut self,
        query: &str,
        mode: SearchMode,
        limit: usize,
        scope: KnowledgeScope,
    ) -> Result<SearchResults> {
        self.require_current_evidence()?;
        self.refresh()?;
        let results = self
            .library
            .as_mut()
            .ok_or_else(unavailable)?
            .search_scoped(query, mode, limit, scope)?;
        validate_hits_scoped(self.require_vault()?, &results.hits, scope)?;
        Ok(results)
    }

    pub fn conversations(&self) -> Result<Vec<WorkConversation>> {
        Ok(self.store.conversations()?)
    }
    pub fn turns(&self, conversation: uuid::Uuid) -> Result<Vec<WorkTurn>> {
        Ok(self.store.turns(conversation)?)
    }
}

/// Text-only earlier terminal pairs, retaining failed/interrupted partial answers.
pub fn model_history(turns: &[WorkTurn]) -> Vec<brn_ai::HistoryPair> {
    let terminal = turns
        .iter()
        .filter(|turn| turn.status != brn_store::work::WorkTurnStatus::Running)
        .collect::<Vec<_>>();
    terminal[terminal.len().saturating_sub(20)..]
        .iter()
        .map(|turn| brn_ai::HistoryPair {
            question: turn.question.clone(),
            answer: turn.answer.clone(),
        })
        .collect()
}

fn unavailable() -> WorkflowError {
    WorkflowError::typed(
        ErrorKind::VaultUnavailable,
        "bound vault folder is unavailable",
    )
}
fn model_key(provider: Provider) -> &'static str {
    match provider {
        Provider::Copilot => "ai.models.copilot",
        Provider::Chatgpt => "ai.models.chatgpt",
    }
}

fn validate_vault_separation(data: &Path, root: &Path) -> Result<()> {
    if data.starts_with(root) || root.starts_with(data) {
        return Err(WorkflowError::typed(
            ErrorKind::WorkspaceModeConflict,
            "vault and data folders must not overlap",
        ));
    }
    Ok(())
}

/// A safe sibling default, not a cache under the repository or vault.
pub fn default_credentials_dir(data_dir: &Path) -> Result<PathBuf> {
    let data = data_dir
        .canonicalize()
        .map_err(|e| WorkflowError::msg(e.to_string()))?;
    let parent = data
        .parent()
        .ok_or_else(|| WorkflowError::msg("data folder has no parent"))?;
    let name = data
        .file_name()
        .ok_or_else(|| WorkflowError::msg("data folder has no name"))?;
    let mut credential_name = name.to_os_string();
    credential_name.push(".credentials");
    let credentials = parent.join(credential_name);
    validate_credentials(&credentials, None, &data)?;
    Ok(credentials)
}

fn validate_credentials(credentials: &Path, root: Option<&Path>, data: &Path) -> Result<()> {
    let unsafe_path = || {
        WorkflowError::typed(
            ErrorKind::UnsafeCredentials,
            "credentials must be outside repositories, the data folder and the vault",
        )
    };
    if !credentials.is_absolute() {
        return Err(unsafe_path());
    }
    let parent = credentials.parent().ok_or_else(unsafe_path)?;
    let resolved = parent
        .canonicalize()
        .map_err(|_| unsafe_path())?
        .join(credentials.file_name().ok_or_else(unsafe_path)?);
    for ancestor in resolved.ancestors() {
        match ancestor.join(".git").symlink_metadata() {
            Ok(_) => return Err(unsafe_path()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(unsafe_path()),
        }
    }
    let data = data.canonicalize().map_err(|_| unsafe_path())?;
    if resolved.starts_with(&data) || data.starts_with(&resolved) {
        return Err(unsafe_path());
    }
    if let Some(root) = root {
        let root = if root.try_exists().map_err(|_| unsafe_path())? {
            root.canonicalize().map_err(|_| unsafe_path())?
        } else {
            root.to_owned()
        };
        if resolved.starts_with(&root) || root.starts_with(&resolved) {
            return Err(unsafe_path());
        }
    }
    Ok(())
}

pub(crate) fn load_model(data: &Path, selected: Option<&Path>) -> Result<Option<SharedEmbedder>> {
    #[cfg(not(feature = "native-retrieval"))]
    {
        let _ = data;
        if selected.is_some() {
            return Err(WorkflowError::typed(
                ErrorKind::SemanticUnavailableInBuild,
                "SEMANTIC_UNAVAILABLE_IN_BUILD",
            ));
        }
        Ok(None)
    }
    #[cfg(feature = "native-retrieval")]
    {
        let default = data.join(crate::models::MODEL_RELATIVE_DIR);
        let path = selected.unwrap_or(&default);
        if !path.is_absolute() {
            return Err(WorkflowError::typed(
                ErrorKind::ModelInvalid,
                "model directory must be absolute",
            ));
        }
        match path.symlink_metadata() {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(_) => Err(WorkflowError::typed(
                ErrorKind::ModelInvalid,
                "could not inspect model directory",
            )),
            Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => {
                let model = crate::library::LocalEmbedder::open(path).map_err(|_| {
                    WorkflowError::typed(
                        ErrorKind::ModelInvalid,
                        "installed embedding model could not be loaded",
                    )
                })?;
                Ok(Some(SharedEmbedder::new(Box::new(model))))
            }
            Ok(_) => Err(WorkflowError::typed(
                ErrorKind::ModelInvalid,
                "model directory is not a regular directory",
            )),
        }
    }
}
