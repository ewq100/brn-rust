//! The search worker (D2): one thread owns a read-only canonical snapshot
//! reader and the only in-process writer of `derived/search.sqlite`. Callers
//! hold cloneable [`SearchHandle`]s and never share a SQLite connection.
//! Each request syncs the index to the current canonical watermark, queries
//! it, and re-checks every candidate with core visibility.
use crate::{AppError, AppResult};
use brn_retrieval::search_index::{
    Changes, DocInput, LinkInput, SearchIndex, SyncState, Visibility,
};
use brn_threads_core::{Record, RecordData, SearchEligibility, Snapshot, Store, StoreReader};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::JoinHandle,
};

/// Candidate notes fetched per keyword lane.
const PER_LANE: usize = 50;

#[derive(Debug, Clone, PartialEq)]
pub struct NoteMatch {
    pub note: String,
    pub version: u64,
    pub title: String,
    /// Best passage, as a byte range into the note's saved Markdown.
    pub start: usize,
    pub end: usize,
    pub heading_path: String,
    pub quote: String,
    /// Matching passages of this note.
    pub passages: usize,
    pub score: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchResults {
    pub matches: Vec<NoteMatch>,
    /// Canonical watermark the index reflected for this answer.
    pub watermark: i64,
}

struct Job {
    query: String,
    chat: Option<String>,
    limit: usize,
    cancel: Arc<AtomicBool>,
    reply: mpsc::Sender<AppResult<SearchResults>>,
}

/// A cloneable request channel to the workspace's search worker.
#[derive(Clone)]
pub struct SearchHandle {
    jobs: mpsc::Sender<Job>,
}

/// Owns the worker thread; it stops once every handle is dropped.
pub struct SearchWorker {
    thread: JoinHandle<()>,
}

impl SearchWorker {
    pub fn join(self) -> AppResult<()> {
        self.thread
            .join()
            .map_err(|_| AppError::Invalid("search worker panicked".into()))
    }
}

/// A pending answer. Dropping it cancels work not yet started, so a UI can
/// replace a stale query without waiting for it.
pub struct SearchTicket {
    reply: mpsc::Receiver<AppResult<SearchResults>>,
    cancel: Arc<AtomicBool>,
}

impl SearchTicket {
    pub fn wait(self) -> AppResult<SearchResults> {
        self.reply.recv().map_err(|_| stopped())?
    }
    /// `None` while the answer is still being prepared.
    pub fn try_take(&self) -> Option<AppResult<SearchResults>> {
        match self.reply.try_recv() {
            Ok(answer) => Some(answer),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err(stopped())),
        }
    }
}

impl Drop for SearchTicket {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

fn stopped() -> AppError {
    AppError::Invalid("search worker stopped".into())
}

impl SearchHandle {
    /// Starts the worker for the workspace at `data`. Storage is opened
    /// lazily by the worker, so open failures are reported per request.
    pub fn spawn(store: &Store) -> AppResult<(Self, SearchWorker)> {
        let data = store.directory().to_owned();
        let derived = store.derived_directory()?;
        let (jobs, receiver) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("brn-search".into())
            .spawn(move || run(&data, &derived, receiver))?;
        Ok((Self { jobs }, SearchWorker { thread }))
    }

    /// Queues a search in `chat` (`None` = general) and returns at once.
    pub fn request(&self, query: &str, chat: Option<&str>, limit: usize) -> SearchTicket {
        let (reply, answer) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let job = Job {
            query: query.into(),
            chat: chat.map(Into::into),
            limit,
            cancel: cancel.clone(),
            reply,
        };
        if let Err(mpsc::SendError(job)) = self.jobs.send(job) {
            let _ = job.reply.send(Err(stopped()));
        }
        SearchTicket {
            reply: answer,
            cancel,
        }
    }

    /// Blocking search for the CLI and agent runs.
    pub fn search(
        &self,
        query: &str,
        chat: Option<&str>,
        limit: usize,
    ) -> AppResult<SearchResults> {
        self.request(query, chat, limit).wait()
    }
}

struct Open {
    reader: StoreReader,
    index: SearchIndex,
}

fn run(data: &Path, derived: &Path, jobs: mpsc::Receiver<Job>) {
    let mut open: Option<Open> = None;
    for job in jobs {
        if job.cancel.load(Ordering::Relaxed) {
            continue;
        }
        if let Err(error) = validate(&job) {
            let _ = job.reply.send(Err(error));
            continue;
        }
        let answer = (|| {
            if open.is_none() {
                open = Some(Open {
                    reader: Store::open_reader(data)?,
                    index: SearchIndex::open(derived)?.0,
                });
            }
            let state = open.as_mut().expect("opened above");
            answer(state, &job)
        })();
        // A failure is reported, never answered as empty. Storage failures
        // reopen next time; caller and concurrency errors keep the connections.
        if let Err(error) = &answer
            && storage_failure(error)
        {
            open = None;
        }
        let _ = job.reply.send(answer);
    }
}

/// Another BRN process can move the shared index past this snapshot between
/// steps; such answers are retried on a fresh snapshot.
const ATTEMPTS: usize = 3;

fn validate(job: &Job) -> AppResult<()> {
    if !(1..=PER_LANE).contains(&job.limit) {
        return Err(AppError::Invalid(format!(
            "search limit must be 1..={PER_LANE}"
        )));
    }
    if !job.query.chars().any(char::is_alphanumeric) || job.query.len() > 512 {
        return Err(AppError::Invalid(
            "search needs words and at most 512 bytes".into(),
        ));
    }
    Ok(())
}

fn storage_failure(error: &AppError) -> bool {
    !matches!(
        error,
        AppError::Invalid(_)
            | AppError::Search(brn_retrieval::Error::Invalid(_) | brn_retrieval::Error::Stale)
    )
}

fn answer(open: &mut Open, job: &Job) -> AppResult<SearchResults> {
    for attempt in 1..=ATTEMPTS {
        if let Some(results) = attempt_answer(open, job, attempt == ATTEMPTS)? {
            return Ok(results);
        }
    }
    unreachable!("the final attempt never yields")
}

fn attempt_answer(open: &mut Open, job: &Job, last: bool) -> AppResult<Option<SearchResults>> {
    let snapshot = open.reader.snapshot()?;
    let Some(watermark) = sync(&snapshot, &mut open.index, last)? else {
        return Ok(None);
    };
    if job.cancel.load(Ordering::Relaxed) {
        return Err(AppError::Invalid("search cancelled".into()));
    }
    let chat = job.chat.as_deref();
    let best: Vec<NoteMatch> = open
        .index
        .keyword(&job.query, chat, PER_LANE)?
        .into_iter()
        .map(|hit| NoteMatch {
            note: hit.doc,
            version: hit.version,
            title: String::new(),
            start: hit.start_byte,
            end: hit.end_byte,
            heading_path: hit.heading_path,
            quote: hit.text,
            passages: hit.passages,
            score: hit.score,
        })
        .collect();
    let ids: Vec<String> = best.iter().map(|m| m.note.clone()).collect();
    let allowed = snapshot.searchable_notes(&ids, chat)?;
    let mut matches = Vec::new();
    for mut candidate in best {
        if matches.len() == job.limit {
            break;
        }
        if !allowed.contains(&candidate.note) {
            continue;
        }
        let Some(Record {
            version,
            data: RecordData::Note(note),
            ..
        }) = snapshot.record(&candidate.note)?
        else {
            continue;
        };
        if version != candidate.version {
            if last {
                return Err(AppError::Invalid(
                    "search index kept changing during the query".into(),
                ));
            }
            return Ok(None);
        }
        candidate.title = note.title;
        matches.push(candidate);
    }
    Ok(Some(SearchResults { matches, watermark }))
}

/// Brings the index to the snapshot's watermark: incrementally when it
/// mirrors the same workspace at an older position, otherwise in full.
/// Returns `None` when another process moved the index past this snapshot or
/// while this step was prepared (unless `last`: then an index ahead of a fresh
/// snapshot means the canonical store went back, and is rebuilt).
fn sync(snapshot: &Snapshot<'_>, index: &mut SearchIndex, last: bool) -> AppResult<Option<i64>> {
    let watermark = snapshot.watermark()?;
    let identity = snapshot.workspace_identity()?;
    let base = index.sync_state()?;
    let (full, changed) = match &base {
        Some(SyncState {
            workspace_identity,
            watermark: at,
        }) if *workspace_identity == identity && *at <= watermark => {
            if *at == watermark {
                return Ok(Some(watermark));
            }
            (false, snapshot.changed_since(*at)?)
        }
        Some(SyncState {
            workspace_identity, ..
        }) if *workspace_identity == identity && !last => return Ok(None),
        _ => (true, snapshot.records()?),
    };
    let mut changes = Changes {
        base,
        full,
        workspace_identity: identity,
        watermark,
        ..Changes::default()
    };
    let mut notes = BTreeSet::new();
    let mut sources = Vec::new();
    for record in &changed {
        match &record.data {
            RecordData::Note(_) => {
                notes.insert(record.id.clone());
            }
            RecordData::Source(_) => sources.push(record.id.clone()),
            RecordData::Link(link) if !record.archived => changes.upsert_links.push(LinkInput {
                id: record.id.clone(),
                from: link.from.clone(),
                to: link.to.clone(),
                relation: link.relation.clone(),
            }),
            RecordData::Link(_) => changes.remove_links.push(record.id.clone()),
            _ => {}
        }
    }
    // A source's context change re-derives the visibility of notes importing it.
    if !full {
        notes.extend(index.docs_importing(&sources)?);
    }
    for id in notes {
        let rows = snapshot.with_references(std::slice::from_ref(&id))?;
        let Some(record) = rows.iter().find(|r| r.id == id) else {
            changes.remove_docs.push(id);
            continue;
        };
        let RecordData::Note(note) = &record.data else {
            continue;
        };
        if record.archived || note.superseded_by.is_some() {
            changes.remove_docs.push(id);
            continue;
        }
        changes.upsert_docs.push(DocInput {
            id: record.id.clone(),
            version: record.version,
            title: note.title.clone(),
            markdown: note.markdown.clone(),
            visibility: match brn_threads_core::search_eligibility(record, &rows) {
                SearchEligibility::General => Visibility::General,
                SearchEligibility::Chat(chat) => Visibility::Chat(chat),
                SearchEligibility::Hidden => Visibility::Hidden,
            },
            protected: note.protected,
            imported_from: note.import.as_ref().map(|i| i.source.clone()),
            language: "en".into(),
        });
    }
    match index.apply(&changes) {
        Ok(()) => Ok(Some(watermark)),
        Err(brn_retrieval::Error::Stale) if !last => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// The derived index location for a workspace, for diagnostics and tests.
pub fn index_path(store: &Store) -> AppResult<PathBuf> {
    Ok(store
        .derived_directory()?
        .join(brn_retrieval::search_index::FILE_NAME))
}
