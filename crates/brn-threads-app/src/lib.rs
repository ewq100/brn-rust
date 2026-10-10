//! Shared product use cases. Every canonical write delegates to the checked core.
use brn_retrieval::note_index::{IndexedNote, NoteIndex};
pub use brn_threads_core::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path};

mod assets;
mod export;
mod intake;
mod review;
mod settings;
pub use export::ExportManifest;
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
}
pub type AppResult<T> = std::result::Result<T, AppError>;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchHit {
    pub note: String,
    pub version: u64,
    pub title: String,
    pub start: usize,
    pub end: usize,
    pub quote: String,
}

pub struct Workspace {
    pub store: Store,
    index: NoteIndex,
    indexed: BTreeMap<String, u64>,
}
impl Workspace {
    pub fn open(data: impl AsRef<Path>) -> AppResult<Self> {
        let mut workspace = Self {
            store: Store::open(data)?,
            index: NoteIndex::in_memory()?,
            indexed: BTreeMap::new(),
        };
        workspace.bootstrap_settings()?;
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
                !r.archived && matches!(&r.data, RecordData::Note(n) if n.superseded_by.is_none())
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
        Ok(self.threads()?.into_iter().filter(|r|matches!(&r.data,RecordData::Thread(t) if t.state==ThreadState::Open && !t.attention.is_empty())).collect())
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
        let op = self
            .store
            .allocate_operation(&format!("new-note:{}", uuid::Uuid::new_v4()))?;
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
        applied_record(self.store.apply(&op, &HostAuthority::owner("owner"))?)
    }
    pub fn new_thread(&mut self, title: &str) -> AppResult<Record> {
        let op = self
            .store
            .allocate_operation(&format!("new-thread:{}", uuid::Uuid::new_v4()))?;
        let request = ChangeRequest {
            reason: "Owner started a thread".into(),
            writes: vec![Put {
                id: op.creation_id(0),
                expected_version: None,
                archived: false,
                data: RecordData::Thread(Thread {
                    title: title.into(),
                    state: ThreadState::Open,
                    attention: vec![],
                }),
            }],
            inputs: vec![],
        };
        self.store
            .prepare_owner(&op, &request, &HostAuthority::owner("owner"))?;
        applied_record(self.store.apply(&op, &HostAuthority::owner("owner"))?)
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
                reason: "Owner resolved the discussion; remaining Actions keep their lifecycle"
                    .into(),
                writes,
                inputs: vec![],
            },
        )
    }
    pub fn search(&mut self, query: &str) -> AppResult<Vec<SearchHit>> {
        let notes = self.notes()?;
        let current: BTreeMap<_, _> = notes.iter().map(|r| (r.id.clone(), r.version)).collect();
        for id in self
            .indexed
            .keys()
            .filter(|id| !current.contains_key(*id))
            .cloned()
            .collect::<Vec<_>>()
        {
            self.index.remove_note(&id)?;
        }
        for record in &notes {
            if self.indexed.get(&record.id) == Some(&record.version) {
                continue;
            }
            let RecordData::Note(note) = &record.data else {
                continue;
            };
            self.index.upsert_note(
                &IndexedNote {
                    path: record.id.clone(),
                    title: note.title.clone(),
                    size: note.markdown.len() as u64,
                    modified_ns: record.version as i64,
                    sha256: Sha256::digest(note.markdown.as_bytes()).into(),
                },
                &note.markdown,
            )?;
        }
        self.indexed = current;
        let by_id: BTreeMap<_, _> = notes.iter().map(|r| (r.id.as_str(), r)).collect();
        Ok(self
            .index
            .keyword(query, 30)?
            .into_iter()
            .filter_map(|hit| {
                let record = by_id.get(hit.path.as_str())?;
                let RecordData::Note(note) = &record.data else {
                    return None;
                };
                Some(SearchHit {
                    note: record.id.clone(),
                    version: record.version,
                    title: note.title.clone(),
                    start: hit.start_byte,
                    end: hit.end_byte,
                    quote: hit.quote,
                })
            })
            .collect())
    }
    pub fn undo(&mut self, id: &str) -> AppResult<ApplyOutcome> {
        let original = self.store.operation(id)?;
        let compensation = self
            .store
            .allocate_operation(&format!("owner-undo:{}", uuid::Uuid::new_v4()))?;
        Ok(self.store.undo(
            &compensation,
            &original,
            &HostAuthority::owner("owner Undo"),
        )?)
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
    applied_receipt(outcome)?
        .writes
        .into_iter()
        .next()
        .map(|w| w.after)
        .ok_or_else(|| AppError::Invalid("Empty receipt".into()))
}
