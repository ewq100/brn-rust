//! The search worker (D2): one thread owns a read-only canonical snapshot
//! reader and the only in-process writer of `derived/search.sqlite`. Callers
//! hold cloneable [`SearchHandle`]s and never share a SQLite connection.
//! Each request syncs the index to the current canonical watermark, queries
//! it, and re-checks every candidate with core visibility.
use crate::{AppError, AppResult};
use brn_retrieval::search_index::{
    Changes, DocInput, LinkInput, SearchIndex, SyncState, Visibility,
};
use brn_retrieval::snippet::{Snippet, snippet};
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
/// The best fused, re-checked notes pages can reach; `total` still counts
/// every match.
pub const REACHABLE: usize = 2 * PER_LANE;
/// Largest page.
pub const MAX_LIMIT: usize = 50;
/// Bytes of matching context in a result.
pub const SNIPPET_BYTES: usize = 300;
/// Pages of more than this many results reserve evidence slots.
const QUOTA_ABOVE: usize = 3;

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
    /// Up to [`SNIPPET_BYTES`] of `quote` around the first matching word.
    pub snippet: Snippet,
    /// Matching passages of this note.
    pub passages: usize,
    pub score: f64,
    pub protected: bool,
    pub imported: bool,
    /// Links into this note from notes visible in the same context.
    pub inbound_links: usize,
    /// Imported, or the target of a visible `source` link.
    pub evidence: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchResults {
    pub matches: Vec<NoteMatch>,
    /// Every visible matching note, including ones no page reaches.
    pub total: usize,
    /// Position of the first match in this result list.
    pub offset: usize,
    /// Matches pages can reach (at most `total`).
    pub reachable: usize,
    /// Opaque cursor for the next page, if one exists.
    pub next: Option<String>,
    /// Canonical watermark the index reflected for this answer.
    pub watermark: i64,
}

struct Job {
    query: String,
    chat: Option<String>,
    limit: usize,
    cursor: Option<String>,
    /// Only bring the index up to date (warm-up and measurement).
    sync_only: bool,
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

/// Cancels a request when dropped. Lets a caller hand the ticket to another
/// thread to wait on while keeping the ability to abandon the request.
pub struct CancelOnDrop(Arc<AtomicBool>);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

impl SearchTicket {
    pub fn cancel_on_drop(&self) -> CancelOnDrop {
        CancelOnDrop(self.cancel.clone())
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

    /// Queues a first-page search in `chat` (`None` = general) and returns at once.
    pub fn request(&self, query: &str, chat: Option<&str>, limit: usize) -> SearchTicket {
        self.request_page(query, chat, limit, None)
    }

    /// Queues the page at `cursor` (from a previous answer's `next`, with the
    /// same `limit`), or the first page.
    pub fn request_page(
        &self,
        query: &str,
        chat: Option<&str>,
        limit: usize,
        cursor: Option<&str>,
    ) -> SearchTicket {
        self.send(Job {
            query: query.into(),
            chat: chat.map(Into::into),
            limit,
            cursor: cursor.map(Into::into),
            sync_only: false,
            cancel: Arc::default(),
            reply: mpsc::channel().0,
        })
    }

    /// Queues bringing the index up to date without a query, so a later
    /// search answers quickly. The answer has no matches.
    pub fn prepare(&self) -> SearchTicket {
        self.send(Job {
            query: String::new(),
            chat: None,
            limit: 1,
            cursor: None,
            sync_only: true,
            cancel: Arc::default(),
            reply: mpsc::channel().0,
        })
    }

    fn send(&self, mut job: Job) -> SearchTicket {
        let (reply, answer) = mpsc::channel();
        job.reply = reply;
        let cancel = job.cancel.clone();
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
        // A removed index (or `derived/`) is re-created rather than kept
        // alive as an unlinked file that the next launch could not see.
        if open.is_some()
            && !derived
                .join(brn_retrieval::search_index::FILE_NAME)
                .exists()
        {
            open = None;
        }
        let answer = (|| {
            if open.is_none() {
                let reader = Store::open_reader(data)?;
                let derived = reader.derived_directory()?;
                open = Some(Open {
                    index: SearchIndex::open(&derived)?.0,
                    reader,
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
    if job.sync_only {
        return Ok(());
    }
    if !(1..=MAX_LIMIT).contains(&job.limit) {
        return Err(AppError::Invalid(format!(
            "search limit must be 1..={MAX_LIMIT}"
        )));
    }
    offset(job)?;
    if !job.query.chars().any(char::is_alphanumeric) || job.query.len() > 512 {
        return Err(AppError::Invalid(
            "search needs words and at most 512 bytes".into(),
        ));
    }
    Ok(())
}

/// The cursor is the offset of a page boundary for this page size.
fn offset(job: &Job) -> AppResult<usize> {
    let Some(cursor) = &job.cursor else {
        return Ok(0);
    };
    cursor
        .parse::<usize>()
        .ok()
        .filter(|at| at % job.limit == 0 && *at <= REACHABLE)
        .ok_or_else(|| AppError::Invalid("search cursor does not match this page size".into()))
}

/// Result order with the evidence quota: every page of more than
/// [`QUOTA_ABOVE`] results holds at least 30% evidence while unplaced
/// evidence remains, pulling the best-ranked evidence forward in place of the
/// page's lowest-ranked other results. Pages keep rank order. Returns indices
/// into `evidence`, which is in rank order.
pub fn arrange(evidence: &[bool], limit: usize) -> Vec<usize> {
    let mut remaining: Vec<usize> = (0..evidence.len()).collect();
    let mut order = Vec::with_capacity(evidence.len());
    while !remaining.is_empty() {
        let take = limit.min(remaining.len());
        let mut page: Vec<usize> = remaining[..take].to_vec();
        if take > QUOTA_ABOVE {
            let need = (3 * take).div_ceil(10);
            let have = page.iter().filter(|&&i| evidence[i]).count();
            let pulled: Vec<usize> = remaining[take..]
                .iter()
                .copied()
                .filter(|&i| evidence[i])
                .take(need.saturating_sub(have))
                .collect();
            for i in pulled {
                if let Some(at) = page.iter().rposition(|&j| !evidence[j]) {
                    page.remove(at);
                    page.push(i);
                }
            }
            page.sort_unstable();
        }
        remaining.retain(|i| !page.contains(i));
        order.extend(page);
    }
    order
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
    if job.sync_only {
        return Ok(Some(SearchResults {
            matches: vec![],
            total: 0,
            offset: 0,
            reachable: 0,
            next: None,
            watermark,
        }));
    }
    let chat = job.chat.as_deref();
    let ranked = open.index.keyword(&job.query, chat, PER_LANE)?;
    let ids: Vec<String> = ranked.iter().map(|hit| hit.doc.clone()).collect();
    let allowed = snapshot.searchable_notes(&ids, chat)?;
    // The index applies core eligibility, so a rejection means another
    // writer moved on; it is excluded from the list and the count.
    let rejected = ids.iter().filter(|id| !allowed.contains(*id)).count();
    let ranked: Vec<_> = ranked
        .into_iter()
        .filter(|hit| allowed.contains(&hit.doc))
        .take(REACHABLE)
        .collect();
    let ids: Vec<String> = ranked.iter().map(|hit| hit.doc.clone()).collect();
    let facts = open.index.facts(&ids, chat, crate::relations::SOURCE)?;
    let evidence: Vec<bool> = ids
        .iter()
        .map(|id| {
            facts
                .get(id)
                .is_some_and(|f| f.imported || f.evidence_links > 0)
        })
        .collect();
    let total = open
        .index
        .count(&job.query, chat)?
        .saturating_sub(rejected)
        .max(ranked.len());
    let start = offset(job)?;
    let order = arrange(&evidence, job.limit);
    let mut matches = Vec::new();
    for &i in order.iter().skip(start).take(job.limit) {
        let hit = &ranked[i];
        let Some(Record {
            version,
            data: RecordData::Note(note),
            ..
        }) = snapshot.record(&hit.doc)?
        else {
            continue;
        };
        if version != hit.version {
            if last {
                return Err(AppError::Invalid(
                    "search index kept changing during the query".into(),
                ));
            }
            return Ok(None);
        }
        let fact = facts.get(&hit.doc).copied().unwrap_or_default();
        matches.push(NoteMatch {
            note: hit.doc.clone(),
            version,
            title: note.title,
            start: hit.start_byte,
            end: hit.end_byte,
            heading_path: hit.heading_path.clone(),
            snippet: snippet(&hit.text, &job.query, SNIPPET_BYTES),
            quote: hit.text.clone(),
            passages: hit.passages,
            score: hit.score,
            protected: note.protected,
            imported: note.import.is_some(),
            inbound_links: fact.inbound_links,
            evidence: evidence[i],
        });
    }
    let next = (start + job.limit < ranked.len()).then(|| (start + job.limit).to_string());
    Ok(Some(SearchResults {
        matches,
        total,
        offset: start,
        reachable: ranked.len(),
        next,
        watermark,
    }))
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
