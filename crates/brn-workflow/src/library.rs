//! The vault's notes as a searchable library: keeps `index.sqlite` in step
//! with the vault and answers keyword, semantic and hybrid searches.
use crate::vault::{self, ReadError, SkipReason, VaultPath};
use brn_retrieval::note_index::{NoteIndex, check_query, fuse_hits};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

#[cfg(feature = "native-retrieval")]
pub use brn_retrieval::native::LocalEmbedder;
pub use brn_retrieval::note_index::{Embedder, EmbeddingProgress, IndexedNote, NoteHit};

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
pub struct SearchResults {
    pub hits: Vec<NoteHit>,
    /// Semantic search was requested but no embedding model is installed, so
    /// only keyword search ran. Callers must tell the user.
    pub keyword_only: bool,
}

pub struct Library {
    root: PathBuf,
    index: NoteIndex,
    embedder: Option<Box<dyn Embedder + Send>>,
}

impl Library {
    /// Opens the index at `index_path` (rebuilding it if damaged) for the
    /// vault at `vault_root`. Without an embedder, searches are keyword-only.
    pub fn open(
        vault_root: &Path,
        index_path: &Path,
        embedder: Option<Box<dyn Embedder + Send>>,
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

    /// Brings the index in step with the vault. Files whose size and
    /// modification time are unchanged are not read again.
    pub fn refresh(&mut self) -> LibraryResult<RefreshReport> {
        let scan = vault::scan(&self.root)?;
        let mut report = RefreshReport::default();
        for skipped in &scan.skipped {
            report.unreadable.push(Unreadable {
                path: skipped.path.clone(),
                reason: match skipped.reason {
                    SkipReason::TooLarge => "larger than 1 MiB",
                    SkipReason::InvalidName => "file name is not valid UTF-8",
                },
            });
        }
        let indexed: HashMap<String, IndexedNote> = self
            .index
            .notes()?
            .into_iter()
            .map(|note| (note.path.clone(), note))
            .collect();
        let mut seen = HashSet::new();
        for file in &scan.notes {
            let path = file.path.as_str();
            seen.insert(path.to_owned());
            let old = indexed.get(path);
            if old.is_some_and(|old| old.size == file.size && old.modified_ns == file.modified_ns) {
                report.unchanged += 1;
                continue;
            }
            let note = match vault::read_note(&self.root, &file.path) {
                Ok(note) => note,
                Err(ReadError::Io(e)) => return Err(e.into()),
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
            if old.is_some_and(|old| old.sha256 == note.sha256) {
                self.index.update_metadata(path, size, file.modified_ns)?;
                report.unchanged += 1;
                continue;
            }
            let record = IndexedNote {
                path: path.to_owned(),
                title: title(&note.text, &file.path),
                size,
                modified_ns: file.modified_ns,
                sha256: note.sha256,
            };
            self.index.upsert_note(&record, &note.text)?;
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
        match self.embedder.as_deref_mut() {
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
        check_query(query, limit)?;
        let embedder = match (mode, self.embedder.as_deref_mut()) {
            (SearchMode::Keyword, _) | (_, None) => {
                return Ok(SearchResults {
                    hits: self.index.keyword(query, limit)?,
                    keyword_only: mode != SearchMode::Keyword,
                });
            }
            (_, Some(embedder)) => embedder,
        };
        let vector = embedder
            .embed(&[query])?
            .pop()
            .ok_or(brn_retrieval::Error::Invalid("embedder returned no vector"))?;
        let hits = if mode == SearchMode::Semantic {
            self.index.semantic(&vector, limit)?
        } else {
            let keyword = self.index.keyword(query, FUSION_DEPTH)?;
            let semantic = self.index.semantic(&vector, FUSION_DEPTH)?;
            fuse_hits(&[&keyword, &semantic], limit)
        };
        Ok(SearchResults {
            hits,
            keyword_only: false,
        })
    }

    /// All indexed notes, ordered by path.
    pub fn notes(&self) -> LibraryResult<Vec<IndexedNote>> {
        Ok(self.index.notes()?)
    }
}

fn unreadable_reason(error: &ReadError) -> Option<&'static str> {
    match error {
        ReadError::NotUtf8 => Some("not valid UTF-8"),
        ReadError::TooLarge => Some("larger than 1 MiB"),
        ReadError::Missing | ReadError::NotAFile | ReadError::Io(_) => None,
    }
}

/// The first level-1 Markdown heading in the first 50 lines, or the file name without `.md`.
fn title(text: &str, path: &VaultPath) -> String {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    text.lines()
        .take(50)
        .find_map(|line| line.strip_prefix("# "))
        .map(str::trim)
        .filter(|heading| !heading.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| {
            let name = path.as_str().rsplit('/').next().unwrap_or(path.as_str());
            name[..name.len() - 3].to_owned()
        })
}
