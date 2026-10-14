//! The search worker (D2): one thread owns a read-only canonical snapshot
//! reader and the only in-process writer of `derived/search.sqlite`. Callers
//! hold cloneable [`SearchHandle`]s and never share a SQLite connection.
//! Each request syncs the index to the current canonical watermark, queries
//! it, and re-checks every candidate with core visibility.
use crate::{AppError, AppResult};
use brn_retrieval::embedding::Embedder;
use brn_retrieval::search_index::{
    Changes, DocInput, LinkInput, SearchIndex, SyncState, Visibility,
};
use brn_retrieval::snippet::{Snippet, snippet};
use brn_threads_core::{Record, RecordData, SearchEligibility, Snapshot, Store, StoreReader};
use std::{
    collections::{BTreeMap, BTreeSet},
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
/// Neighbours shown per hop in Connections.
pub const NEIGHBOURS: usize = 8;
/// Passages embedded per idle-time batch; a queued request waits at most one batch.
const EMBED_BATCH: usize = 16;
/// Nearest passages (above the floor) the dense lane may walk, in order,
/// until it has enough visible notes; other chats' passages cannot crowd
/// out visible ones before this bound.
const DENSE_PASSAGES: usize = 40 * PER_LANE;

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
    /// Opaque cursor for the next page, if one exists. Ranking can shift
    /// between pages while notes change or `semantic` is `partial`.
    pub next: Option<String>,
    /// Canonical watermark the index reflected for this answer.
    pub watermark: i64,
    /// Dense coverage: `none`, `partial(n/m)`, `complete` or `failed: …`.
    pub semantic: String,
}

/// What a neighbour is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NodeKind {
    Note,
    Source,
}

/// One recorded link to a neighbour: `outgoing` when it goes from the centre
/// (or `via`) to the neighbour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relation {
    pub relation: String,
    pub outgoing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Neighbour {
    pub id: String,
    pub version: u64,
    pub title: String,
    pub kind: NodeKind,
    /// How it links to `via` (or the centre), recommended relations first.
    pub relations: Vec<Relation>,
    /// The first-hop neighbour it was reached through; `None` on hop 1.
    pub via: Option<String>,
    /// Links into it from notes visible in the same context.
    pub inbound_links: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hop {
    /// At most [`NEIGHBOURS`], best first.
    pub shown: Vec<Neighbour>,
    /// Every visible neighbour at this distance.
    pub available: usize,
}

/// A note's bounded recorded neighbourhood, visibility-checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connections {
    pub centre: Neighbour,
    pub hops: Vec<Hop>,
}

enum Work {
    Search {
        query: String,
        chat: Option<String>,
        limit: usize,
        cursor: Option<String>,
        /// Add the dense lane when an embedding model is configured.
        hybrid: bool,
    },
    /// Only bring the index up to date (warm-up and measurement).
    Sync,
    Connections {
        chat: Option<String>,
        note: String,
        hops: u8,
        sources: bool,
    },
}

enum Reply {
    Results(mpsc::Sender<AppResult<SearchResults>>),
    Connections(mpsc::Sender<AppResult<Connections>>),
}

enum Answer {
    Results(SearchResults),
    Connections(Connections),
}

impl Reply {
    fn send(&self, answer: AppResult<Answer>) {
        // A dropped ticket means nobody waits; that is not an error.
        match (self, answer) {
            (Self::Results(to), Ok(Answer::Results(r))) => drop(to.send(Ok(r))),
            (Self::Connections(to), Ok(Answer::Connections(c))) => drop(to.send(Ok(c))),
            (Self::Results(to), Err(e)) => drop(to.send(Err(e))),
            (Self::Connections(to), Err(e)) => drop(to.send(Err(e))),
            (Self::Results(to), Ok(_)) => drop(to.send(Err(stopped()))),
            (Self::Connections(to), Ok(_)) => drop(to.send(Err(stopped()))),
        }
    }
}

struct Job {
    work: Work,
    cancel: Arc<AtomicBool>,
    reply: Reply,
}

/// A cloneable request channel to the workspace's search worker.
#[derive(Clone)]
pub struct SearchHandle {
    jobs: mpsc::Sender<Job>,
    progress: Arc<std::sync::Mutex<Option<(usize, usize)>>>,
}

/// Owns the worker thread; it stops once every handle is dropped.
pub struct SearchWorker {
    thread: JoinHandle<()>,
}

impl SearchWorker {
    pub fn join(self) -> AppResult<()> {
        self.thread
            .join()
            .map_err(|_| AppError::Unavailable("search worker panicked".into()))
    }
}

/// A pending answer. Dropping it cancels work not yet started, so a UI can
/// replace a stale request without waiting for it.
pub struct Ticket<T> {
    reply: mpsc::Receiver<AppResult<T>>,
    cancel: Arc<AtomicBool>,
}

pub type SearchTicket = Ticket<SearchResults>;
pub type ConnectionsTicket = Ticket<Connections>;

impl<T> Ticket<T> {
    pub fn wait(self) -> AppResult<T> {
        self.reply.recv().map_err(|_| stopped())?
    }
    /// `None` while the answer is still being prepared.
    pub fn try_take(&self) -> Option<AppResult<T>> {
        match self.reply.try_recv() {
            Ok(answer) => Some(answer),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err(stopped())),
        }
    }
    pub fn cancel_on_drop(&self) -> CancelOnDrop {
        CancelOnDrop(self.cancel.clone())
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

impl<T> Drop for Ticket<T> {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

fn stopped() -> AppError {
    AppError::Unavailable("search worker stopped".into())
}

impl SearchHandle {
    /// Starts the worker for the workspace at `data`. Storage is opened
    /// lazily by the worker, so open failures are reported per request.
    pub fn spawn(store: &Store) -> AppResult<(Self, SearchWorker)> {
        Self::spawn_with(store, None)
    }

    /// Like [`Self::spawn`], with an embedding model for the dense lane. The
    /// worker embeds passages in bounded batches while no request waits.
    pub fn spawn_with(
        store: &Store,
        embedder: Option<Box<dyn Embedder + Send>>,
    ) -> AppResult<(Self, SearchWorker)> {
        let data = store.directory().to_owned();
        let derived = store.derived_directory()?;
        let (jobs, receiver) = mpsc::channel();
        let progress = Arc::new(std::sync::Mutex::new(None));
        let dense = embedder.map(|embedder| Dense {
            model: embedder.identity().to_owned(),
            embedder,
            matrix: None,
            counts: None,
            idle: false,
            failed: None,
            retry: true,
            progress: progress.clone(),
        });
        let thread = std::thread::Builder::new()
            .name("brn-search".into())
            .spawn(move || run(&data, &derived, receiver, dense))?;
        Ok((Self { jobs, progress }, SearchWorker { thread }))
    }

    /// `(embedded passages, all passages)` once known, when an embedding
    /// model is configured.
    pub fn embedding_progress(&self) -> Option<(usize, usize)> {
        *self.progress.lock().ok()?
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
        self.results(Work::Search {
            query: query.into(),
            chat: chat.map(Into::into),
            limit,
            cursor: cursor.map(Into::into),
            hybrid: true,
        })
    }

    /// [`Self::request_page`] with only the keyword lanes.
    pub fn request_keyword_page(
        &self,
        query: &str,
        chat: Option<&str>,
        limit: usize,
        cursor: Option<&str>,
    ) -> SearchTicket {
        self.results(Work::Search {
            query: query.into(),
            chat: chat.map(Into::into),
            limit,
            cursor: cursor.map(Into::into),
            hybrid: false,
        })
    }

    /// Queues bringing the index up to date without a query, so a later
    /// search answers quickly. The answer has no matches.
    pub fn prepare(&self) -> SearchTicket {
        self.results(Work::Sync)
    }

    /// Queues `note`'s recorded neighbourhood as seen from `chat`: `hops` is
    /// 1 or 2; `sources` adds linked source records.
    pub fn request_connections(
        &self,
        note: &str,
        chat: Option<&str>,
        hops: u8,
        sources: bool,
    ) -> ConnectionsTicket {
        let (reply, answer) = mpsc::channel();
        self.send(
            Work::Connections {
                chat: chat.map(Into::into),
                note: note.into(),
                hops,
                sources,
            },
            Reply::Connections(reply),
            answer,
        )
    }

    fn results(&self, work: Work) -> SearchTicket {
        let (reply, answer) = mpsc::channel();
        self.send(work, Reply::Results(reply), answer)
    }

    fn send<T>(&self, work: Work, reply: Reply, answer: mpsc::Receiver<AppResult<T>>) -> Ticket<T> {
        let cancel = Arc::new(AtomicBool::new(false));
        let job = Job {
            work,
            cancel: cancel.clone(),
            reply,
        };
        if let Err(mpsc::SendError(job)) = self.jobs.send(job) {
            job.reply.send(Err(stopped()));
        }
        Ticket {
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

    /// Blocking Connections for the CLI and agent runs.
    pub fn connections(
        &self,
        note: &str,
        chat: Option<&str>,
        hops: u8,
        sources: bool,
    ) -> AppResult<Connections> {
        self.request_connections(note, chat, hops, sources).wait()
    }
}

struct Open {
    reader: StoreReader,
    index: SearchIndex,
}

/// The dense lane's model state, kept across index reopens.
struct Dense {
    embedder: Box<dyn Embedder + Send>,
    model: String,
    /// Stored vectors in one contiguous matrix, loaded on first use.
    matrix: Option<Matrix>,
    /// `(embedded, all)` passages, refreshed after each request.
    counts: Option<(usize, usize)>,
    /// Nothing left to embed until the next request.
    idle: bool,
    /// The last embedding failure, reported until a batch succeeds.
    failed: Option<String>,
    /// A failed batch is retried once per request, never in a busy loop.
    retry: bool,
    progress: Arc<std::sync::Mutex<Option<(usize, usize)>>>,
}

struct Matrix {
    passages: Vec<i64>,
    data: Vec<f32>,
    dimension: usize,
}

impl Matrix {
    fn push(&mut self, passage: i64, vector: &[f32]) {
        if vector.len() == self.dimension {
            self.passages.push(passage);
            self.data.extend_from_slice(vector);
        }
    }
    /// Passage IDs at or above `floor` by descending cosine similarity
    /// (vectors are unit length). Empty unless the best passage stands at
    /// least `margin` above the background (median of ranks 10–50), so a
    /// query nothing really matches finds nothing.
    fn nearest(&self, query: &[f32], k: usize, floor: f32, margin: f32) -> Vec<i64> {
        let mut scored: Vec<(f32, i64)> = self
            .data
            .chunks_exact(self.dimension)
            .zip(&self.passages)
            .map(|(row, id)| (row.iter().zip(query).map(|(a, b)| a * b).sum::<f32>(), *id))
            .collect();
        if margin > 0.0 && scored.len() >= 50 {
            let mut top: Vec<f32> = scored.iter().map(|(s, _)| *s).collect();
            top.select_nth_unstable_by(49, |a, b| b.total_cmp(a));
            top.truncate(50);
            top.sort_by(|a, b| b.total_cmp(a));
            if top[0] - top[30] < margin {
                return vec![];
            }
        }
        scored.retain(|(score, _)| *score >= floor);
        let k = k.min(scored.len());
        if k == 0 {
            return vec![];
        }
        scored.select_nth_unstable_by(k - 1, |a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        scored.truncate(k);
        scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        scored.into_iter().map(|(_, id)| id).collect()
    }
}

impl Dense {
    fn semantic(&self) -> String {
        if let Some(error) = &self.failed {
            return format!("failed: {error}");
        }
        match self.counts {
            Some((with, all)) if with >= all && all > 0 => "complete".into(),
            Some((0, _)) | None => "none".into(),
            Some((with, all)) => format!("partial({with}/{all})"),
        }
    }

    fn refresh_counts(&mut self, index: &SearchIndex) -> AppResult<()> {
        let counts = index.vector_counts(&self.model)?;
        self.counts = Some(counts);
        self.idle = counts.0 >= counts.1;
        if let Ok(mut progress) = self.progress.lock() {
            *progress = Some(counts);
        }
        Ok(())
    }

    /// Embeds one bounded batch of passages that have no vector yet.
    /// Forgets cached rows and counts after the index was created or rebuilt.
    fn reset(&mut self) {
        self.matrix = None;
        self.counts = None;
        self.idle = false;
    }

    /// One idle-time batch; a failure is kept for reporting and retried only
    /// after the next request.
    fn embed_idle(&mut self, index: &mut SearchIndex) {
        match self.embed_batch(index) {
            Ok(()) => self.failed = None,
            Err(error) => {
                self.failed = Some(error.to_string());
                self.retry = false;
            }
        }
    }

    fn embed_batch(&mut self, index: &mut SearchIndex) -> AppResult<()> {
        let pending = index.pending_passages(&self.model, EMBED_BATCH)?;
        if pending.is_empty() {
            return self.refresh_counts(index);
        }
        let prefix = self.embedder.prefixes().1;
        let texts: Vec<String> = pending
            .iter()
            .map(|(_, t)| format!("{prefix}{t}"))
            .collect();
        let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
        let vectors = self.embedder.embed(&refs)?;
        if vectors.len() != pending.len() {
            return Err(AppError::Unavailable(
                "embedding model returned a wrong count".into(),
            ));
        }
        let rows: Vec<(i64, Vec<f32>)> = pending
            .iter()
            .map(|(id, _)| *id)
            .zip(vectors)
            .map(|(id, v)| Ok((id, brn_retrieval::search_index::unit(&v)?)))
            .collect::<AppResult<_>>()?;
        index.store_vectors(&self.model, &rows)?;
        if let Some(matrix) = &mut self.matrix {
            for (id, vector) in &rows {
                matrix.push(*id, vector);
            }
        }
        self.refresh_counts(index)
    }

    /// Passage IDs nearest to `query`, or `None` without stored vectors.
    fn nearest(&mut self, index: &SearchIndex, query: &str) -> AppResult<Option<Vec<i64>>> {
        let with = self.counts.map_or(0, |c| c.0);
        if with == 0 {
            return Ok(None);
        }
        // Removed passages leave surplus rows; another process may have
        // stored vectors this one lacks. Either way, reload.
        let stale = self.matrix.as_ref().is_none_or(|m| {
            m.passages.len() < with || m.passages.len() > with + with / 2 + EMBED_BATCH
        });
        if stale {
            let rows = index.load_vectors(&self.model)?;
            let dimension = self.embedder.dimension();
            let mut matrix = Matrix {
                passages: Vec::with_capacity(rows.len()),
                data: Vec::with_capacity(rows.len() * dimension),
                dimension,
            };
            for (id, vector) in &rows {
                matrix.push(*id, vector);
            }
            self.matrix = Some(matrix);
        }
        let prefix = self.embedder.prefixes().0;
        let vector = self
            .embedder
            .embed(&[&format!("{prefix}{query}")])?
            .pop()
            .ok_or_else(|| AppError::Unavailable("embedding model returned nothing".into()))?;
        let vector = brn_retrieval::search_index::unit(&vector)?;
        Ok(Some(self.matrix.as_ref().expect("loaded above").nearest(
            &vector,
            DENSE_PASSAGES,
            self.embedder.similarity_floor(),
            self.embedder.min_margin(),
        )))
    }
}

fn run(data: &Path, derived: &Path, jobs: mpsc::Receiver<Job>, mut dense: Option<Dense>) {
    let mut open: Option<Open> = None;
    loop {
        let job = match jobs.try_recv() {
            Ok(job) => job,
            Err(mpsc::TryRecvError::Disconnected) => break,
            Err(mpsc::TryRecvError::Empty) => {
                // Idle: embed one batch, then look for requests again.
                if let (Some(d), Some(state)) = (dense.as_mut(), open.as_mut())
                    && !d.idle
                    && d.retry
                {
                    d.embed_idle(&mut state.index);
                    continue;
                }
                match jobs.recv() {
                    Ok(job) => job,
                    Err(_) => break,
                }
            }
        };
        if job.cancel.load(Ordering::Relaxed) {
            continue;
        }
        if let Err(error) = validate(&job.work) {
            job.reply.send(Err(error));
            continue;
        }
        if let Some(d) = dense.as_mut() {
            d.retry = true;
        }
        // A removed index (or `derived/`) is re-created rather than kept
        // alive as an unlinked file that the next launch could not see.
        if open.is_some()
            && !derived
                .join(brn_retrieval::search_index::FILE_NAME)
                .exists()
        {
            open = None;
            if let Some(d) = dense.as_mut() {
                d.reset();
            }
        }
        let answer = (|| {
            if open.is_none() {
                let reader = Store::open_reader(data)?;
                let derived = reader.derived_directory()?;
                let (mut index, opened) = SearchIndex::open(&derived)?;
                if let Some(d) = dense.as_mut() {
                    // A created or rebuilt index starts passage IDs afresh.
                    if opened != brn_retrieval::search_index::Opened::Existing {
                        d.reset();
                    }
                    index.retain_vectors(&d.model)?;
                }
                open = Some(Open { index, reader });
            }
            let state = open.as_mut().expect("opened above");
            answer(state, dense.as_mut(), &job)
        })();
        // A failure is reported, never answered as empty. Storage failures
        // reopen next time; caller and concurrency errors keep the connections.
        if let Err(error) = &answer
            && storage_failure(error)
        {
            open = None;
        }
        job.reply.send(answer);
    }
}

/// Another BRN process can move the shared index past this snapshot between
/// steps; such answers are retried on a fresh snapshot.
const ATTEMPTS: usize = 3;

fn validate(work: &Work) -> AppResult<()> {
    match work {
        Work::Sync => Ok(()),
        Work::Search {
            query,
            limit,
            cursor,
            ..
        } => {
            if !(1..=MAX_LIMIT).contains(limit) {
                return Err(AppError::Invalid(format!(
                    "search limit must be 1..={MAX_LIMIT}"
                )));
            }
            offset(cursor.as_deref(), *limit)?;
            if !query.chars().any(char::is_alphanumeric) || query.len() > 512 {
                return Err(AppError::Invalid(
                    "search needs words and at most 512 bytes".into(),
                ));
            }
            Ok(())
        }
        Work::Connections { note, hops, .. } => {
            if !(1..=2).contains(hops) || note.is_empty() || note.len() > 256 {
                return Err(AppError::Invalid(
                    "connections need a note ID and 1 or 2 hops".into(),
                ));
            }
            Ok(())
        }
    }
}

/// The cursor is the offset of a page boundary for this page size.
fn offset(cursor: Option<&str>, limit: usize) -> AppResult<usize> {
    let Some(cursor) = cursor else {
        return Ok(0);
    };
    cursor
        .parse::<usize>()
        .ok()
        .filter(|at| at % limit == 0 && *at <= REACHABLE)
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
            | AppError::Unavailable(_)
            | AppError::Search(brn_retrieval::Error::Invalid(_) | brn_retrieval::Error::Stale)
    )
}

fn answer(open: &mut Open, mut dense: Option<&mut Dense>, job: &Job) -> AppResult<Answer> {
    for attempt in 1..=ATTEMPTS {
        if let Some(answer) = attempt_answer(open, dense.as_deref_mut(), job, attempt == ATTEMPTS)?
        {
            return Ok(answer);
        }
    }
    unreachable!("the final attempt never yields")
}

fn attempt_answer(
    open: &mut Open,
    dense: Option<&mut Dense>,
    job: &Job,
    last: bool,
) -> AppResult<Option<Answer>> {
    let snapshot = open.reader.snapshot()?;
    let Some(watermark) = sync(&snapshot, &mut open.index, last)? else {
        return Ok(None);
    };
    if job.cancel.load(Ordering::Relaxed) {
        return Err(AppError::Unavailable("search cancelled".into()));
    }
    // This sync may have added passages without vectors: count before
    // reporting coverage, so progress never lags behind an answer.
    let mut dense = dense;
    if let Some(d) = dense.as_deref_mut() {
        d.refresh_counts(&open.index)?;
    }
    match &job.work {
        Work::Sync => Ok(Some(Answer::Results(SearchResults {
            matches: vec![],
            total: 0,
            offset: 0,
            reachable: 0,
            next: None,
            watermark,
            semantic: dense.map_or_else(|| "none".into(), |d| d.semantic()),
        }))),
        Work::Search {
            query,
            chat,
            limit,
            cursor,
            hybrid,
        } => {
            let (nearest, semantic) = match dense {
                Some(d) if *hybrid => {
                    match d.nearest(&open.index, query) {
                        Ok(nearest) => (nearest, d.semantic()),
                        // Reported, never hidden: keyword lanes still answer.
                        Err(error) => (None, format!("failed: {error}")),
                    }
                }
                Some(d) => (None, format!("off ({})", d.semantic())),
                None => (None, "none".into()),
            };
            Ok(search(
                &snapshot,
                &open.index,
                query,
                chat.as_deref(),
                *limit,
                cursor.as_deref(),
                watermark,
                last,
                nearest.as_deref().unwrap_or_default(),
            )?
            .map(|mut results| {
                results.semantic = semantic;
                Answer::Results(results)
            }))
        }
        Work::Connections {
            chat,
            note,
            hops,
            sources,
        } => Ok(Some(Answer::Connections(connections(
            &snapshot,
            &open.index,
            note,
            chat.as_deref(),
            *hops,
            *sources,
        )?))),
    }
}

#[allow(clippy::too_many_arguments)]
fn search(
    snapshot: &Snapshot<'_>,
    index: &SearchIndex,
    query: &str,
    chat: Option<&str>,
    limit: usize,
    cursor: Option<&str>,
    watermark: i64,
    last: bool,
    dense: &[i64],
) -> AppResult<Option<SearchResults>> {
    let ranked = index.hybrid(query, chat, PER_LANE, dense)?;
    let ids: Vec<String> = ranked.iter().map(|hit| hit.doc.clone()).collect();
    let allowed = snapshot.searchable_notes(&ids, chat)?;
    // Notes only the dense lane ranked may still match the keywords further
    // down; only true non-matches add to the exact keyword count.
    let unranked: Vec<String> = ranked
        .iter()
        .filter(|hit| !hit.keyword)
        .map(|hit| hit.doc.clone())
        .collect();
    let lexical = index.matching_keywords(query, &unranked)?;
    let matches_keywords =
        |hit: &brn_retrieval::search_index::DocHit| hit.keyword || lexical.contains(&hit.doc);
    // The index applies core eligibility, so a rejection means another
    // writer moved on; it is excluded from the list and the count.
    let rejected = ranked
        .iter()
        .filter(|hit| !allowed.contains(&hit.doc) && matches_keywords(hit))
        .count();
    let semantic_only = ranked
        .iter()
        .filter(|hit| allowed.contains(&hit.doc) && !matches_keywords(hit))
        .count();
    let ranked: Vec<_> = ranked
        .into_iter()
        .filter(|hit| allowed.contains(&hit.doc))
        .take(REACHABLE)
        .collect();
    let ids: Vec<String> = ranked.iter().map(|hit| hit.doc.clone()).collect();
    let facts = index.facts(&ids, chat, crate::relations::SOURCE)?;
    let evidence: Vec<bool> = ids
        .iter()
        .map(|id| {
            facts
                .get(id)
                .is_some_and(|f| f.imported || f.evidence_links > 0)
        })
        .collect();
    // Exact keyword matches plus the notes only the dense lane found.
    let total =
        (index.count(query, chat)?.saturating_sub(rejected) + semantic_only).max(ranked.len());
    let start = offset(cursor, limit)?;
    let order = arrange(&evidence, limit);
    let mut matches = Vec::new();
    for &i in order.iter().skip(start).take(limit) {
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
                return Err(AppError::Unavailable(
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
            snippet: snippet(&hit.text, query, SNIPPET_BYTES),
            quote: hit.text.clone(),
            passages: hit.passages,
            score: hit.score,
            protected: note.protected,
            imported: note.import.is_some(),
            inbound_links: fact.inbound_links,
            evidence: evidence[i],
        });
    }
    let next = (start + limit < ranked.len()).then(|| (start + limit).to_string());
    Ok(Some(SearchResults {
        matches,
        total,
        offset: start,
        reachable: ranked.len(),
        next,
        watermark,
        semantic: String::new(),
    }))
}

/// Recommended relations first, in their listed order; other names after.
fn relation_rank(relations: &[Relation]) -> usize {
    relations
        .iter()
        .map(|r| {
            crate::relations::RECOMMENDED
                .iter()
                .position(|known| *known == r.relation)
                .unwrap_or(crate::relations::RECOMMENDED.len())
        })
        .min()
        .unwrap_or(usize::MAX)
}

/// The visible notes (and, with `sources`, source records) linked to each
/// of `from`, keyed by neighbour, excluding `exclude`. Each neighbour keeps
/// the first `from` node reaching it.
fn neighbours(
    snapshot: &Snapshot<'_>,
    index: &SearchIndex,
    from: &[String],
    exclude: &BTreeSet<String>,
    chat: Option<&str>,
    sources: bool,
) -> AppResult<Vec<(String, String, Vec<Relation>)>> {
    let mut found: BTreeMap<String, (String, Vec<Relation>)> = BTreeMap::new();
    let mut order = Vec::new();
    for node in from {
        for link in index.links_of(node)? {
            let (other, outgoing) = if link.from == *node {
                (link.to.clone(), true)
            } else {
                (link.from.clone(), false)
            };
            if exclude.contains(&other) {
                continue;
            }
            let entry = found.entry(other.clone()).or_insert_with(|| {
                order.push(other.clone());
                (node.clone(), Vec::new())
            });
            if entry.0 == *node {
                let relation = Relation {
                    relation: link.relation,
                    outgoing,
                };
                if !entry.1.contains(&relation) {
                    entry.1.push(relation);
                }
            }
        }
    }
    let notes = snapshot.searchable_notes(&order, chat)?;
    let visible = if sources {
        snapshot.visible_records(&order, chat)?
    } else {
        BTreeSet::new()
    };
    let mut out = Vec::new();
    for id in order {
        let keep = notes.contains(&id)
            || (visible.contains(&id)
                && matches!(
                    snapshot.record(&id)?,
                    Some(Record {
                        data: RecordData::Source(_),
                        ..
                    })
                ));
        if keep {
            let (via, mut relations) = found.remove(&id).expect("recorded above");
            relations.sort_by_key(|r| relation_rank(std::slice::from_ref(r)));
            out.push((id, via, relations));
        }
    }
    Ok(out)
}

fn node(
    snapshot: &Snapshot<'_>,
    id: &str,
    relations: Vec<Relation>,
    via: Option<String>,
    inbound: &BTreeMap<String, brn_retrieval::search_index::DocFacts>,
) -> AppResult<Option<Neighbour>> {
    let Some(record) = snapshot.record(id)? else {
        return Ok(None);
    };
    let (kind, title) = match &record.data {
        RecordData::Note(note) => (NodeKind::Note, note.title.clone()),
        RecordData::Source(source) => (NodeKind::Source, source.label.clone()),
        _ => return Ok(None),
    };
    Ok(Some(Neighbour {
        id: record.id,
        version: record.version,
        title,
        kind,
        relations,
        via,
        inbound_links: inbound.get(id).map_or(0, |f| f.inbound_links),
    }))
}

/// The bounded neighbourhood of a note visible in `chat`. A note the context
/// cannot see is refused, exactly like a missing one.
fn connections(
    snapshot: &Snapshot<'_>,
    index: &SearchIndex,
    note: &str,
    chat: Option<&str>,
    hops: u8,
    sources: bool,
) -> AppResult<Connections> {
    let centre_id = note.to_owned();
    if !snapshot
        .searchable_notes(std::slice::from_ref(&centre_id), chat)?
        .contains(&centre_id)
    {
        return Err(AppError::Invalid("note not found".into()));
    }
    let mut seen = BTreeSet::from([centre_id.clone()]);
    let mut frontier = vec![centre_id.clone()];
    let mut layers = Vec::new();
    for _ in 0..hops {
        let found = neighbours(snapshot, index, &frontier, &seen, chat, sources)?;
        let ids: Vec<String> = found.iter().map(|(id, ..)| id.clone()).collect();
        let facts = index.facts(&ids, chat, crate::relations::SOURCE)?;
        let first_hop = layers.is_empty();
        let mut shown = Vec::new();
        for (id, via, relations) in found {
            seen.insert(id.clone());
            shown.extend(node(
                snapshot,
                &id,
                relations,
                (!first_hop).then_some(via),
                &facts,
            )?);
        }
        shown.sort_by(|a, b| {
            relation_rank(&a.relations)
                .cmp(&relation_rank(&b.relations))
                .then_with(|| b.inbound_links.cmp(&a.inbound_links))
                .then_with(|| a.title.cmp(&b.title))
                .then_with(|| a.id.cmp(&b.id))
        });
        let available = shown.len();
        shown.truncate(NEIGHBOURS);
        frontier = shown.iter().map(|n| n.id.clone()).collect();
        layers.push(Hop { shown, available });
    }
    let facts = index.facts(
        std::slice::from_ref(&centre_id),
        chat,
        crate::relations::SOURCE,
    )?;
    let centre = node(snapshot, &centre_id, vec![], None, &facts)?
        .ok_or_else(|| AppError::Invalid("note not found".into()))?;
    Ok(Connections {
        centre,
        hops: layers,
    })
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

/// The host side of the agent tools (D4): runs search in the run's chat.
impl brn_ai::threads::RunNoteSearch for SearchHandle {
    fn search(
        &self,
        chat: &str,
        query: &str,
        limit: usize,
        cursor: Option<&str>,
        hybrid: bool,
    ) -> Result<brn_ai::threads::NoteSearchPage, brn_ai::threads::NoteSearchError> {
        let ticket = if hybrid {
            self.request_page(query, Some(chat), limit, cursor)
        } else {
            self.request_keyword_page(query, Some(chat), limit, cursor)
        };
        let results = ticket.wait().map_err(agent_error)?;
        Ok(brn_ai::threads::NoteSearchPage {
            total: results.total,
            shown: results.matches.len(),
            cursor: results.next,
            semantic: results.semantic,
            results: results
                .matches
                .into_iter()
                .map(|m| brn_ai::threads::NoteSearchResult {
                    id: m.note,
                    version: m.version,
                    title: m.title,
                    heading_path: m.heading_path,
                    snippet: m.snippet.text,
                    start: m.start,
                    matches: m.passages,
                    protected: m.protected,
                    imported: m.imported,
                    evidence: m.evidence,
                    inbound_links: m.inbound_links,
                })
                .collect(),
        })
    }

    fn connections(
        &self,
        chat: &str,
        note: &str,
        hops: u8,
        sources: bool,
    ) -> Result<brn_ai::threads::NoteNeighbourhood, brn_ai::threads::NoteSearchError> {
        let found = self
            .request_connections(note, Some(chat), hops, sources)
            .wait()
            .map_err(agent_error)?;
        Ok(brn_ai::threads::NoteNeighbourhood {
            centre: agent_node(found.centre),
            hops: found
                .hops
                .into_iter()
                .zip(1..)
                .map(|(hop, n)| brn_ai::threads::ConnectionHop {
                    hop: n,
                    available: hop.available,
                    neighbours: hop.shown.into_iter().map(agent_node).collect(),
                })
                .collect(),
        })
    }
}

fn agent_error(error: AppError) -> brn_ai::threads::NoteSearchError {
    match error {
        AppError::Invalid(_) | AppError::Search(brn_retrieval::Error::Invalid(_)) => {
            brn_ai::threads::NoteSearchError::Rejected
        }
        other => brn_ai::threads::NoteSearchError::Unavailable(other.to_string()),
    }
}

fn agent_node(node: Neighbour) -> brn_ai::threads::ConnectionNode {
    brn_ai::threads::ConnectionNode {
        id: node.id,
        version: node.version,
        title: node.title,
        kind: match node.kind {
            NodeKind::Note => "note",
            NodeKind::Source => "source",
        }
        .into(),
        relations: node
            .relations
            .into_iter()
            .map(|r| format!("{}:{}", if r.outgoing { "out" } else { "in" }, r.relation))
            .collect(),
        via: node.via,
        inbound_links: node.inbound_links,
    }
}
