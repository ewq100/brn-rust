//! The vault's notes as a searchable library: keeps `index.sqlite` in step
//! with the vault and answers keyword, semantic and hybrid searches.
use crate::vault::{self, ReadError, SkipReason};
use brn_retrieval::note_index::{NoteIndex, NoteSearch, check_query, fuse_hits};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[cfg(feature = "native-retrieval")]
pub use brn_retrieval::native::LocalEmbedder;
pub use brn_retrieval::note_index::{
    Embedder, EmbeddingProgress, IndexedNote, KnowledgeScope, NoteMetadata,
};

/// How many results each side contributes before hybrid fusion.
const FUSION_DEPTH: usize = 50;

#[derive(Debug)]
pub enum LibraryError {
    Io(std::io::Error),
    Index(brn_retrieval::Error),
}

impl std::fmt::Display for LibraryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "could not read the vault: {e}"),
            Self::Index(e) => write!(f, "search index: {e}"),
        }
    }
}

impl std::error::Error for LibraryError {}

impl From<std::io::Error> for LibraryError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<brn_retrieval::Error> for LibraryError {
    fn from(e: brn_retrieval::Error) -> Self {
        Self::Index(e)
    }
}

pub type LibraryResult<T> = Result<T, LibraryError>;

/// A `.md` file in the vault that could not be indexed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unreadable {
    pub path: String,
    pub reason: &'static str,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RefreshReport {
    pub added: usize,
    pub updated: usize,
    pub removed: usize,
    pub unchanged: usize,
    /// Sorted by path.
    pub unreadable: Vec<Unreadable>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    Keyword,
    Semantic,
    Hybrid,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NoteHit {
    pub path: String,
    pub note_sha256: [u8; 32],
    pub start_byte: usize,
    pub end_byte: usize,
    pub quote: String,
    /// Higher is better; comparable only within one result list.
    pub score: f32,
}

fn client_hits(hits: Vec<brn_retrieval::note_index::NoteHit>) -> Vec<NoteHit> {
    hits.into_iter()
        .map(|hit| NoteHit {
            path: hit.path,
            note_sha256: hit.note_sha256,
            start_byte: hit.start_byte,
            end_byte: hit.end_byte,
            quote: hit.quote,
            score: hit.score,
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchResults {
    pub hits: Vec<NoteHit>,
    /// No embedding model is installed, so only keyword search ran. Callers must tell the user.
    pub keyword_only: bool,
}

pub struct Library {
    root: PathBuf,
    index: NoteIndex,
    embedder: Option<SharedEmbedder>,
}

#[derive(Clone)]
pub struct SharedEmbedder {
    inner: Arc<Mutex<Box<dyn Embedder + Send>>>,
    identity: String,
    dimension: usize,
}

impl SharedEmbedder {
    pub fn new(embedder: Box<dyn Embedder + Send>) -> Self {
        Self {
            identity: embedder.identity().to_owned(),
            dimension: embedder.dimension(),
            inner: Arc::new(Mutex::new(embedder)),
        }
    }
}

impl Embedder for SharedEmbedder {
    fn identity(&self) -> &str {
        &self.identity
    }
    fn dimension(&self) -> usize {
        self.dimension
    }
    fn embed(&mut self, texts: &[&str]) -> brn_retrieval::Result<Vec<Vec<f32>>> {
        self.inner
            .lock()
            .map_err(|_| brn_retrieval::Error::EmbedderPoisoned)?
            .embed(texts)
    }
}

pub fn search_index(
    index: &dyn NoteSearch,
    embedder: Option<&mut dyn Embedder>,
    query: &str,
    mode: SearchMode,
    limit: usize,
) -> LibraryResult<SearchResults> {
    search_index_scoped(index, embedder, query, mode, limit, KnowledgeScope::Current)
}

pub fn search_index_scoped(
    index: &dyn NoteSearch,
    embedder: Option<&mut dyn Embedder>,
    query: &str,
    mode: SearchMode,
    limit: usize,
    scope: KnowledgeScope,
) -> LibraryResult<SearchResults> {
    check_query(query, limit)?;
    let Some(embedder) = embedder else {
        return Ok(SearchResults {
            hits: client_hits(index.keyword_scoped(query, limit, scope)?),
            keyword_only: true,
        });
    };
    if mode == SearchMode::Keyword {
        return Ok(SearchResults {
            hits: client_hits(index.keyword_scoped(query, limit, scope)?),
            keyword_only: false,
        });
    }
    let mut vectors = embedder.embed(&[query])?;
    if vectors.len() != 1 || vectors[0].len() != embedder.dimension() {
        return Err(brn_retrieval::Error::Invalid(
            "embedder returned the wrong count or dimension",
        )
        .into());
    }
    let vector = vectors.remove(0);
    let hits = if mode == SearchMode::Semantic {
        index.semantic_for_model_scoped(&vector, embedder.identity(), limit, scope)?
    } else {
        let keyword = index.keyword_scoped(query, FUSION_DEPTH, scope)?;
        let semantic =
            index.semantic_for_model_scoped(&vector, embedder.identity(), FUSION_DEPTH, scope)?;
        fuse_hits(&[&keyword, &semantic], limit)
    };
    Ok(SearchResults {
        hits: client_hits(hits),
        keyword_only: false,
    })
}

impl Library {
    /// Opens the index at `index_path` (rebuilding it if damaged) for the
    /// vault at `vault_root`. Without an embedder, searches are keyword-only.
    pub fn open(
        vault_root: &Path,
        index_path: &Path,
        embedder: Option<Box<dyn Embedder + Send>>,
    ) -> LibraryResult<Self> {
        Self::open_shared(vault_root, index_path, embedder.map(SharedEmbedder::new))
    }

    /// Uses the same loaded model as read tools; no second model load.
    pub fn open_shared(
        vault_root: &Path,
        index_path: &Path,
        embedder: Option<SharedEmbedder>,
    ) -> LibraryResult<Self> {
        let (mut index, _) = NoteIndex::open(index_path)?;
        if let Some(embedder) = &embedder {
            index.use_embedding_model(embedder.identity(), embedder.dimension())?;
        }
        Ok(Self {
            root: vault_root.to_path_buf(),
            index,
            embedder,
        })
    }

    /// Brings disposable metadata/passages in step with fresh saved evidence,
    /// including source/history notes. Equal size/mtime never substitutes for bytes.
    /// Hits carry the note hash seen at indexing time; callers that need current text must read the note again.
    pub fn refresh(&mut self) -> LibraryResult<RefreshReport> {
        let scan = vault::scan_evidence(&self.root)?;
        let mut report = RefreshReport::default();
        for skipped in &scan.skipped {
            report.unreadable.push(Unreadable {
                path: skipped.path.clone(),
                reason: match skipped.reason {
                    SkipReason::TooLarge => "larger than 1 MiB",
                    SkipReason::InvalidName => "file name is not valid UTF-8",
                    SkipReason::UnreadableDirectory => "could not inspect folder",
                },
            });
        }
        let indexed: HashMap<String, IndexedNote> = self
            .index
            .all_notes()?
            .into_iter()
            .map(|note| (note.path.clone(), note))
            .collect();
        let mut seen = HashSet::new();
        for file in &scan.notes {
            let path = file.path.as_str();
            seen.insert(path.to_owned());
            let old = indexed.get(path);
            let note = match vault::read_evidence(&self.root, &file.path) {
                Ok(note) => note,
                Err(error) => {
                    if let Some(reason) = unreadable_reason(&error) {
                        report.unreadable.push(Unreadable {
                            path: path.to_owned(),
                            reason,
                        });
                    }
                    if old.is_some() {
                        self.index.remove_note(path)?;
                        report.removed += 1;
                    }
                    continue;
                }
            };
            let size = note.text.len() as u64;
            let metadata = saved_metadata(&note.text, path);
            let title = title(&note.text, path);
            if metadata.issue.is_some() {
                report.unreadable.push(Unreadable {
                    path: path.into(),
                    reason: "invalid managed metadata",
                });
            }
            // Equal bytes keep passages unless an older title policy derived
            // a different title for them.
            if old.is_some_and(|old| old.sha256 == note.sha256 && old.title == title) {
                self.index.update_metadata(path, size, file.modified_ns)?;
                self.index.update_note_metadata(path, &metadata)?;
                report.unchanged += 1;
                continue;
            }
            let record = IndexedNote {
                path: path.to_owned(),
                title,
                size,
                modified_ns: file.modified_ns,
                sha256: note.sha256,
            };
            self.index
                .upsert_note_with_metadata(&record, &note.text, &metadata)?;
            if old.is_some() {
                report.updated += 1;
            } else {
                report.added += 1;
            }
        }
        for path in indexed.keys().filter(|path| !seen.contains(*path)) {
            self.index.remove_note(path)?;
            report.removed += 1;
        }
        report.unreadable.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(report)
    }

    /// Embeds up to `batch` passages that have none yet. Call repeatedly in
    /// the background until `embedded == total`. `None` without an embedder.
    pub fn embed_pending(&mut self, batch: usize) -> LibraryResult<Option<EmbeddingProgress>> {
        match self.embedder.as_mut() {
            Some(embedder) => Ok(Some(self.index.embed_pending(embedder, batch)?)),
            None => Ok(None),
        }
    }

    /// Searches the indexed notes. Semantic and hybrid searches fall back to
    /// keyword search, flagged by `keyword_only`, when no model is installed;
    /// they cover only passages embedded so far.
    pub fn search(
        &mut self,
        query: &str,
        mode: SearchMode,
        limit: usize,
    ) -> LibraryResult<SearchResults> {
        self.search_scoped(query, mode, limit, KnowledgeScope::Current)
    }

    pub fn search_scoped(
        &mut self,
        query: &str,
        mode: SearchMode,
        limit: usize,
        scope: KnowledgeScope,
    ) -> LibraryResult<SearchResults> {
        search_index_scoped(
            &self.index,
            self.embedder.as_mut().map(|e| e as &mut dyn Embedder),
            query,
            mode,
            limit,
            scope,
        )
    }

    pub fn replace_embedder(&mut self, embedder: SharedEmbedder) -> LibraryResult<()> {
        self.index
            .use_embedding_model(embedder.identity(), embedder.dimension())?;
        self.embedder = Some(embedder);
        Ok(())
    }

    /// All indexed notes, ordered by path.
    pub fn notes(&self) -> LibraryResult<Vec<IndexedNote>> {
        Ok(self.index.notes()?)
    }

    pub fn notes_scoped(&self, scope: KnowledgeScope) -> LibraryResult<Vec<IndexedNote>> {
        Ok(self.index.notes_scoped(scope)?)
    }

    pub(crate) fn relationship_page(
        &mut self,
        edges: &[brn_retrieval::note_index::NoteEdge],
        scope: KnowledgeScope,
        offset: usize,
        limit: usize,
    ) -> LibraryResult<brn_retrieval::note_index::EdgePage> {
        self.index.replace_edges(edges)?;
        Ok(self.index.edges(scope, offset, limit)?)
    }
}

pub(crate) fn saved_metadata(text: &str, path: &str) -> NoteMetadata {
    let identity = brn_store::note_identity::read(text);
    let classification = brn_store::note_metadata::classify(text);
    let provenance = brn_store::note_provenance::read(text);
    let issue = identity
        .as_ref()
        .err()
        .or_else(|| classification.as_ref().err())
        .or_else(|| provenance.as_ref().err())
        .map(ToString::to_string);
    let classification = classification.unwrap_or_default();
    NoteMetadata {
        note_id: identity.ok().flatten(),
        source: classification.source,
        history: classification.history
            || path
                .split('/')
                .next()
                .is_some_and(|part| part.eq_ignore_ascii_case("archive")),
        issue,
    }
}

fn unreadable_reason(error: &ReadError) -> Option<&'static str> {
    match error {
        ReadError::NotUtf8 => Some("not valid UTF-8"),
        ReadError::TooLarge => Some("larger than 1 MiB"),
        ReadError::Io(_) => Some("could not read note"),
        ReadError::Missing | ReadError::NotAFile => None,
    }
}

/// Saved physical lines, counted from after an optional BOM and including
/// frontmatter, that may supply a note title.
const TITLE_LINES: usize = 50;

/// The first nonempty level-1 Markdown heading in the first 50 saved lines,
/// outside supported leading frontmatter, or the file name without `.md`.
fn title(text: &str, path: &str) -> String {
    heading_title(text).map_or_else(
        || {
            let name = path.rsplit('/').next().unwrap_or(path);
            name[..name.len() - 3].to_owned()
        },
        str::to_owned,
    )
}

/// Markdown structure decides which lines are ATX headings, so code, HTML and
/// setext text cannot supply a title. Eligible headings keep the literal
/// `# ` line policy and their raw trimmed line suffix, not rendered text.
fn heading_title(text: &str) -> Option<&str> {
    let lines = if text.starts_with('\u{feff}') { 3 } else { 0 };
    let window_end = text[lines..]
        .match_indices('\n')
        .nth(TITLE_LINES - 1)
        .map_or(text.len(), |(newline, _)| lines + newline + 1);
    // Unsupported managed layouts stay indexable. Their exact legacy header
    // framing, or no header at all, preserves the previous title boundary.
    let body = brn_store::note_identity::body_start(text)
        .ok()
        .or_else(|| legacy_body_start(text))
        .unwrap_or(lines);
    let window = text.get(body..window_end)?;
    // Only nonempty literal `# ` lines can supply a title. The prefix through
    // the last one fixes their block structure; later lines cannot change it.
    let mut line_start = 0;
    let mut candidates_end = None;
    for line in window.split_inclusive('\n') {
        line_start += line.len();
        if line
            .strip_prefix("# ")
            .is_some_and(|title| !title.trim().is_empty())
        {
            candidates_end = Some(line_start);
        }
    }
    let source = &window[..candidates_end?];
    // Inline constructs cannot change block structure; skipping them bounds
    // parse cost. The pinned parser can panic on some valid setext/thematic
    // break sequences, which is a parse failure with filename fallback.
    let mut options = markdown::ParseOptions::default();
    let constructs = &mut options.constructs;
    constructs.attention = false;
    constructs.autolink = false;
    constructs.character_escape = false;
    constructs.character_reference = false;
    constructs.code_text = false;
    constructs.hard_break_escape = false;
    constructs.hard_break_trailing = false;
    constructs.html_text = false;
    constructs.label_start_image = false;
    constructs.label_start_link = false;
    constructs.label_end = false;
    let root = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        markdown::to_mdast(source, &options)
    }))
    .ok()?
    .ok()?;
    root.children()?.iter().find_map(|node| {
        let markdown::mdast::Node::Heading(heading) = node else {
            return None;
        };
        let start = heading.position.as_ref()?.start.offset;
        if heading.depth != 1 || start != 0 && !source[..start].ends_with('\n') {
            return None;
        }
        source[start..]
            .split('\n')
            .next()?
            .strip_prefix("# ")
            .map(str::trim)
            .filter(|title| !title.is_empty())
    })
}

/// Exact complete `---` header framing for unsupported legacy metadata. It
/// locates the body without interpreting opaque YAML; an unsupported or
/// incomplete delimiter never supplies a guessed boundary.
pub(crate) fn legacy_body_start(text: &str) -> Option<usize> {
    let (mut offset, text) = text
        .strip_prefix('\u{feff}')
        .map_or((0, text), |body| (3, body));
    let mut lines = text.split_inclusive('\n');
    let first = lines.next()?;
    if !matches!(first, "---\n" | "---\r\n") {
        return None;
    }
    offset += first.len();
    for line in lines {
        offset += line.len();
        let content = line
            .strip_suffix("\r\n")
            .or_else(|| line.strip_suffix('\n'))
            .unwrap_or(line);
        if matches!(content, "---" | "...") {
            return Some(offset);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::title;

    #[test]
    fn unsupported_managed_layouts_use_exact_legacy_header_framing() {
        for (text, expected) in [
            (
                "---\n# yaml comment\nbrn_id:bad\n---\n```\n# Fake\n```\n# Malformed Title\n",
                "Malformed Title",
            ),
            (
                "\u{feff}---\r\n# yaml comment\r\nbrn_id: a\r\nbrn_id: b\r\n...\r\n# Duplicate Title\r\n",
                "Duplicate Title",
            ),
            ("--- \n# Opening Title\n---\n", "Opening Title"),
            (
                "---\nbrn_id:bad\n--- x\n# Unclosed Managed\n",
                "Unclosed Managed",
            ),
        ] {
            assert!(
                brn_store::note_identity::body_start(text).is_err(),
                "{text:?}"
            );
            assert_eq!(title(text, "dir/fallback.md"), expected, "{text:?}");
        }
        assert_eq!(title("", "dir/fallback.md"), "fallback");
    }
}
