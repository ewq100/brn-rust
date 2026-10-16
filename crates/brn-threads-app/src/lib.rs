//! Shared product use cases. Every canonical write delegates to the checked core.
pub use brn_threads_core::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

mod assets;
mod chat;
mod converge;
mod decisions;
mod export;
mod intake;
mod keyed;
mod knowledge;
mod projects;
mod refresh;
pub mod relations;
mod review;
mod review_groups;
pub mod search;
mod settings;
pub use settings::HELPER_TESTED_NOTE;
mod steering;
mod topics;
mod writing;
pub use brn_retrieval::Error as RetrievalError;
pub use export::ExportManifest;
pub use intake::import_report;
pub use knowledge::{
    FINDINGS_PER_KIND, Finding, FindingKind, KnowledgeCheck, REVIEW_MESSAGE_BYTES,
};
pub use review_groups::{ReviewGroup, ReviewGroups};
pub use topics::{TopicEntry, Topics};
pub use writing::{WritingComment, WritingCommentInput};
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Core(#[from] brn_threads_core::Error),
    #[error(transparent)]
    Search(#[from] brn_retrieval::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Invalid(String),
    /// A service is not running or could not finish; not the caller's fault.
    #[error("{0}")]
    Unavailable(String),
}
pub type AppResult<T> = std::result::Result<T, AppError>;
/// One matching note with its best passage. `start..end` is a byte range in
/// the saved Markdown at `version`; `quote` is that passage.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchHit {
    pub note: String,
    pub version: u64,
    pub title: String,
    pub start: usize,
    pub end: usize,
    pub quote: String,
    /// Headings above the passage, joined with ` › `.
    #[serde(default)]
    pub heading_path: String,
    /// Bounded matching context from `quote`.
    #[serde(default)]
    pub snippet: String,
    /// Byte ranges in `snippet` to highlight.
    #[serde(default)]
    pub highlights: Vec<std::ops::Range<usize>>,
    /// Matching passages in the note.
    #[serde(default)]
    pub matches: usize,
    /// Links into the note from notes visible in the same context.
    #[serde(default)]
    pub inbound_links: usize,
    #[serde(default)]
    pub protected: bool,
    #[serde(default)]
    pub imported: bool,
    /// Imported, or cited by a visible `source` link.
    #[serde(default)]
    pub evidence: bool,
}

/// One page of search results with exact counts.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchPage {
    pub hits: Vec<SearchHit>,
    /// Every visible matching note.
    pub total: usize,
    /// Position of the first hit among the ranked matches.
    pub offset: usize,
    /// Matches pages can reach; refine the query to reach the rest.
    pub reachable: usize,
    /// Pass back with the same limit for the next page.
    pub cursor: Option<String>,
    /// Dense coverage: `none`, `partial(n/m)`, `complete` or `failed: …`.
    #[serde(default)]
    pub semantic: String,
}

impl From<search::SearchResults> for SearchPage {
    fn from(results: search::SearchResults) -> Self {
        Self {
            total: results.total,
            offset: results.offset,
            reachable: results.reachable,
            cursor: results.next,
            semantic: results.semantic,
            hits: results
                .matches
                .into_iter()
                .map(|m| SearchHit {
                    note: m.note,
                    version: m.version,
                    title: m.title,
                    start: m.start,
                    end: m.end,
                    quote: m.quote,
                    heading_path: m.heading_path,
                    snippet: m.snippet.text,
                    highlights: m.snippet.highlights,
                    matches: m.passages,
                    inbound_links: m.inbound_links,
                    protected: m.protected,
                    imported: m.imported,
                    evidence: m.evidence,
                })
                .collect(),
        }
    }
}

/// Downloads the pinned embedding model into the fresh absolute `target`
/// folder: an explicit owner action, verified by size and digest. Returns
/// the downloaded byte count.
#[cfg(feature = "native-retrieval")]
pub fn install_embedding_model(
    target: &Path,
    cancel: &std::sync::atomic::AtomicBool,
    progress: impl Fn(u64, u64),
) -> AppResult<u64> {
    Ok(brn_retrieval::native::download::download_model(target, cancel, progress)?.downloaded_bytes)
}

/// Names the folder of an installed embedding model for semantic search.
pub const EMBEDDING_MODEL_ENV: &str = "BRN_EMBEDDING_MODEL_DIR";

/// Results [`Workspace::search`] returns.
pub const SEARCH_LIMIT: usize = 30;

pub struct Workspace {
    pub store: Store,
    search: Option<search::SearchHandle>,
    embedder: Option<Box<dyn brn_retrieval::embedding::Embedder + Send>>,
}
impl Workspace {
    pub fn open(data: impl AsRef<Path>) -> AppResult<Self> {
        Self::open_with(data, settings::DefaultCredentials::Derive)
    }
    /// `open`, except that a workspace created now records no credential
    /// source; one that already exists keeps its stored Settings.
    pub fn open_without_default_credentials(data: impl AsRef<Path>) -> AppResult<Self> {
        Self::open_with(data, settings::DefaultCredentials::None)
    }
    fn open_with(
        data: impl AsRef<Path>,
        credentials: settings::DefaultCredentials,
    ) -> AppResult<Self> {
        let mut workspace = Self {
            store: Store::open(data)?,
            search: None,
            embedder: None,
        };
        workspace.bootstrap_settings(credentials)?;
        workspace.bootstrap_chat_control()?;
        Ok(workspace)
    }
    pub fn records(&self) -> AppResult<Vec<Record>> {
        Ok(self.store.records()?)
    }
    pub fn notes(&self) -> AppResult<Vec<Record>> {
        Ok(self
            .records()?
            .into_iter()
            .filter(|r| {
                !r.archived && matches!(&r.data, RecordData::Note(n) if n.superseded_by.is_none() && n.chat.is_none())
            })
            .collect())
    }
    pub fn threads(&self) -> AppResult<Vec<Record>> {
        Ok(self
            .records()?
            .into_iter()
            .filter(|r| !r.archived && matches!(r.data, RecordData::Thread(_)))
            .collect())
    }
    pub fn needs_you(&self) -> AppResult<Vec<Record>> {
        Ok(self
            .records()?
            .into_iter()
            .filter(|r| matches!(&r.data,RecordData::Thread(t) if !t.attention.is_empty()))
            .collect())
    }
    pub fn messages(&self, thread: &str) -> AppResult<Vec<Record>> {
        let mut rows: Vec<_> = self
            .records()?
            .into_iter()
            .filter(|r| !r.archived && matches!(&r.data,RecordData::Message(m) if m.thread==thread))
            .collect();
        // Receipt insertion order is durable and total even when wall-clock
        // timestamps tie or the clock moves backwards. Message edits retain creation order.
        let mut order = BTreeMap::new();
        for (receipt_order, receipt) in self.history()?.into_iter().enumerate() {
            for (write_order, write) in receipt.writes.into_iter().enumerate() {
                order
                    .entry(write.after.id)
                    .or_insert((receipt_order, write_order));
            }
        }
        rows.sort_by_key(|row| {
            order
                .get(&row.id)
                .copied()
                .unwrap_or((usize::MAX, usize::MAX))
        });
        Ok(rows)
    }
    pub fn history(&self) -> AppResult<Vec<Receipt>> {
        Ok(self.store.receipts()?)
    }
    pub fn candidates(&self) -> AppResult<Vec<Prepared>> {
        Ok(self.store.candidates()?)
    }
    pub fn owner_change(&mut self, key: &str, request: &ChangeRequest) -> AppResult<ApplyOutcome> {
        let op = self.store.allocate_operation(key)?;
        let authority = HostAuthority::owner("owner interaction");
        self.store.prepare_owner(&op, request, &authority)?;
        Ok(self.store.apply(&op, &authority)?)
    }
    pub fn new_note(&mut self, title: &str, markdown: &str) -> AppResult<Record> {
        first_record(self.new_note_with_key(&uuid::Uuid::new_v4().to_string(), title, markdown)?)
    }
    /// Owner note creation under a caller-chosen host key. An identical retry
    /// replays the receipt; the same key with different content is ImmutableRequest.
    pub fn new_note_with_key(
        &mut self,
        key: &str,
        title: &str,
        markdown: &str,
    ) -> AppResult<Receipt> {
        let op = self.store.allocate_operation(&format!("new-note:{key}"))?;
        let request = ChangeRequest {
            reason: "Owner created a note".into(),
            writes: vec![Put {
                id: op.creation_id(0),
                expected_version: None,
                archived: false,
                data: RecordData::Note(Note::working(title, markdown)),
            }],
            inputs: vec![],
        };
        self.store
            .prepare_owner(&op, &request, &HostAuthority::owner("owner"))?;
        applied_receipt(self.store.apply(&op, &HostAuthority::owner("owner"))?)
    }
    pub fn new_thread(&mut self, title: &str) -> AppResult<Record> {
        first_record(self.new_thread_with_key(&uuid::Uuid::new_v4().to_string(), title)?)
    }
    /// Owner thread creation under a caller-chosen host key; replays like
    /// [`Workspace::new_note_with_key`].
    pub fn new_thread_with_key(&mut self, key: &str, title: &str) -> AppResult<Receipt> {
        let op = self
            .store
            .allocate_operation(&format!("new-thread:{key}"))?;
        let request = ChangeRequest {
            reason: "Owner started a thread".into(),
            writes: vec![Put {
                id: op.creation_id(0),
                expected_version: None,
                archived: false,
                data: RecordData::Thread(Thread {
                    private_context: false,
                    context_epoch: 0,
                    title: title.into(),
                    state: ThreadState::Open,
                    attention: vec![],
                }),
            }],
            inputs: vec![],
        };
        self.store
            .prepare_owner(&op, &request, &HostAuthority::owner("owner"))?;
        applied_receipt(self.store.apply(&op, &HostAuthority::owner("owner"))?)
    }
    pub fn post_message(&mut self, thread: &str, text: &str) -> AppResult<Receipt> {
        let op = self
            .store
            .allocate_operation(&format!("owner-message:{}", uuid::Uuid::new_v4()))?;
        let request = ChangeRequest {
            reason: "Owner message".into(),
            writes: vec![Put {
                id: op.creation_id(0),
                expected_version: None,
                archived: false,
                data: RecordData::Message(Message {
                    thread: thread.into(),
                    role: "owner".into(),
                    text: text.into(),
                }),
            }],
            inputs: vec![],
        };
        self.store
            .prepare_owner(&op, &request, &HostAuthority::owner("owner"))?;
        applied_receipt(self.store.apply(&op, &HostAuthority::owner("owner"))?)
    }
    pub fn resolve_thread(&mut self, id: &str) -> AppResult<ApplyOutcome> {
        self.resolve_thread_because(
            id,
            "Owner resolved the discussion; remaining Actions keep their lifecycle",
        )
    }
    /// Same checked resolution with a specific recorded reason, such as an
    /// answered quick question. Linked Actions keep their own lifecycle.
    pub fn resolve_thread_because(&mut self, id: &str, reason: &str) -> AppResult<ApplyOutcome> {
        let record = self
            .store
            .record(id)?
            .ok_or_else(|| AppError::Invalid("Thread not found".into()))?;
        let RecordData::Thread(mut thread) = record.data else {
            return Err(AppError::Invalid("Not a thread".into()));
        };
        thread.state = ThreadState::Resolved;
        thread.attention.clear();
        let mut writes = vec![Put {
            id: id.into(),
            expected_version: Some(record.version),
            archived: false,
            data: RecordData::Thread(thread),
        }];
        for record in self.records()? {
            if let RecordData::Run(mut run) = record.data
                && run.thread == id
                && run.state == RunState::Working
            {
                run.state = RunState::Cancelled;
                run.fence += 1;
                writes.push(Put {
                    id: record.id,
                    expected_version: Some(record.version),
                    archived: record.archived,
                    data: RecordData::Run(run),
                });
            }
        }
        self.owner_change(
            &format!("resolve:{}", uuid::Uuid::new_v4()),
            &ChangeRequest {
                reason: reason.into(),
                writes,
                inputs: vec![],
            },
        )
    }
    /// Reopen a resolved thread with the owner's recorded reason. Messages,
    /// links and history are unchanged; no attention is invented.
    pub fn reopen_thread(&mut self, id: &str, reason: &str) -> AppResult<ApplyOutcome> {
        let record = self
            .store
            .record(id)?
            .filter(|record| !record.archived)
            .ok_or_else(|| AppError::Invalid("Thread not found".into()))?;
        let RecordData::Thread(mut thread) = record.data else {
            return Err(AppError::Invalid("Not a thread".into()));
        };
        if thread.state == ThreadState::Open {
            return Err(AppError::Invalid("This thread is already open".into()));
        }
        thread.state = ThreadState::Open;
        self.owner_change(
            &format!("reopen:{}", uuid::Uuid::new_v4()),
            &ChangeRequest {
                reason: reason.into(),
                writes: vec![Put {
                    id: id.into(),
                    expected_version: Some(record.version),
                    archived: false,
                    data: RecordData::Thread(thread),
                }],
                inputs: vec![],
            },
        )
    }
    /// The workspace's search worker, started on first use. Clones share it.
    pub fn search_handle(&mut self) -> AppResult<search::SearchHandle> {
        if let Some(handle) = &self.search {
            return Ok(handle.clone());
        }
        // The worker stops once the last handle is dropped.
        let (handle, _worker) =
            search::SearchHandle::spawn_with(&self.store, self.embedder.take())?;
        self.search = Some(handle.clone());
        Ok(handle)
    }
    /// Gives search an embedding model for the dense lane. Must be called
    /// before the first search starts the worker.
    pub fn use_embedder(
        &mut self,
        embedder: Box<dyn brn_retrieval::embedding::Embedder + Send>,
    ) -> AppResult<()> {
        if self.search.is_some() {
            return Err(AppError::Invalid(
                "choose the embedding model before searching".into(),
            ));
        }
        self.embedder = Some(embedder);
        Ok(())
    }
    /// Loads the pinned local model from `dir` (never downloads) and uses it
    /// for the dense lane. Returns the model identity.
    #[cfg(feature = "native-retrieval")]
    pub fn open_embedding_model(&mut self, dir: &Path) -> AppResult<String> {
        use brn_retrieval::embedding::Embedder;
        let embedder = brn_retrieval::native::LocalEmbedder::open(dir)?;
        let identity = embedder.identity().to_owned();
        self.use_embedder(Box::new(embedder))?;
        Ok(identity)
    }
    /// Uses the model folder the owner named in [`EMBEDDING_MODEL_ENV`], if
    /// any. `None` when unset; an error when it cannot be used, including a
    /// build without native retrieval. Keyword search works either way.
    pub fn embedding_model_from_env(&mut self) -> Option<AppResult<String>> {
        let dir = std::env::var_os(EMBEDDING_MODEL_ENV).filter(|d| !d.is_empty())?;
        let dir = std::path::PathBuf::from(dir);
        if !dir.is_absolute() {
            return Some(Err(AppError::Invalid(format!(
                "{EMBEDDING_MODEL_ENV} must be an absolute folder"
            ))));
        }
        #[cfg(feature = "native-retrieval")]
        {
            Some(self.open_embedding_model(&dir))
        }
        #[cfg(not(feature = "native-retrieval"))]
        {
            Some(Err(AppError::Unavailable(
                "this build has no native retrieval; semantic search is off".into(),
            )))
        }
    }
    /// The owner's general search: saved text of current notes outside
    /// chats, best first.
    pub fn search(&mut self, query: &str) -> AppResult<Vec<SearchHit>> {
        Ok(self.search_page(query, None, SEARCH_LIMIT, None)?.hits)
    }
    /// One page of results visible in `chat` (`None` = general).
    pub fn search_page(
        &mut self,
        query: &str,
        chat: Option<&str>,
        limit: usize,
        cursor: Option<&str>,
    ) -> AppResult<SearchPage> {
        Ok(self
            .search_handle()?
            .request_page(query, chat, limit, cursor)
            .wait()?
            .into())
    }
    pub fn undo(&mut self, id: &str) -> AppResult<ApplyOutcome> {
        Ok(self.undo_with_key(&uuid::Uuid::new_v4().to_string(), id)?.1)
    }
    /// Owner Undo whose compensation identity is `owner-undo:{key}`. Returns the
    /// compensation with its outcome so a Deferred compensation is retried, not duplicated.
    pub fn undo_with_key(&mut self, key: &str, id: &str) -> AppResult<(OperationId, ApplyOutcome)> {
        let original = self.store.operation(id)?;
        let compensation = self
            .store
            .allocate_operation(&format!("owner-undo:{key}"))?;
        let outcome = self.store.undo(
            &compensation,
            &original,
            &HostAuthority::owner("owner Undo"),
        )?;
        Ok((compensation, outcome))
    }
    pub fn export_now(&self, destination: &Path) -> AppResult<export::ExportManifest> {
        export::publish(&self.store, destination)
    }
    pub fn backup(&self, destination: &Path) -> AppResult<()> {
        Ok(self.store.backup(destination)?)
    }
}
fn applied_receipt(outcome: ApplyOutcome) -> AppResult<Receipt> {
    if let ApplyOutcome::Applied(receipt) = outcome {
        Ok(receipt)
    } else {
        Err(AppError::Invalid(format!(
            "Change was not applied: {outcome:?}"
        )))
    }
}
fn applied_record(outcome: ApplyOutcome) -> AppResult<Record> {
    first_record(applied_receipt(outcome)?)
}
fn first_record(receipt: Receipt) -> AppResult<Record> {
    receipt
        .writes
        .into_iter()
        .next()
        .map(|w| w.after)
        .ok_or_else(|| AppError::Invalid("Empty receipt".into()))
}
