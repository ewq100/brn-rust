//! Fresh, read-only vault tools. Index text is never trusted without validation.
use crate::library::{
    KnowledgeScope, LibraryError, SearchMode, SharedEmbedder, saved_metadata, search_index,
};
use crate::vault::{self, EvidencePath, VaultPath};
use brn_ai::{
    AiError, AiErrorKind, AiResult, NoteEntry, NotePage, Passage, ReadTools, ToolNote, ToolSearch,
};
use brn_retrieval::note_index::{IndexedNote, NoteHit, NoteIndexReader};
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

pub struct AiTools {
    root: PathBuf,
    current_fence: Mutex<(u64, bool)>,
    reader: Mutex<NoteIndexReader>,
    embedder: Option<SharedEmbedder>,
}

fn rejected() -> AiError {
    AiError::new(AiErrorKind::ToolRejected)
}
fn stale() -> AiError {
    AiError::new(AiErrorKind::IndexStale)
}

impl AiTools {
    pub fn open(
        root: &Path,
        index: &Path,
        embedder: Option<SharedEmbedder>,
    ) -> crate::Result<Self> {
        Ok(Self {
            root: root.to_owned(),
            current_fence: Mutex::new((0, false)),
            reader: Mutex::new(NoteIndexReader::open(index)?),
            embedder,
        })
    }

    pub(crate) fn set_current_blocked(&self, blocked: bool) {
        let mut fence = self.current_fence.lock().expect("owned evidence fence");
        fence.0 = fence.0.checked_add(1).expect("evidence epoch exhausted");
        fence.1 = blocked;
    }

    fn check_current_epoch(&self, epoch: u64) -> AiResult<()> {
        let fence = self.current_fence.lock().map_err(|_| stale())?;
        if fence.1 || fence.0 != epoch {
            return Err(stale());
        }
        Ok(())
    }

    fn check_root(&self) -> AiResult<u64> {
        let epoch = {
            let fence = self.current_fence.lock().map_err(|_| stale())?;
            if fence.1 {
                return Err(stale());
            }
            fence.0
        };
        if self
            .root
            .symlink_metadata()
            .is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink())
        {
            Ok(epoch)
        } else {
            Err(stale())
        }
    }

    fn entries(&self) -> AiResult<Vec<IndexedNote>> {
        self.reader
            .lock()
            .map_err(|_| AiError::new(AiErrorKind::Storage))?
            .notes()
            .map_err(|_| stale())
    }
}

pub(crate) fn validate_hits(root: &Path, hits: &[NoteHit]) -> AiResult<()> {
    validate_hits_scoped(root, hits, KnowledgeScope::Current)
}

fn read_path(path: &str, scope: KnowledgeScope) -> AiResult<EvidencePath> {
    if scope == KnowledgeScope::Current {
        VaultPath::parse(path).map_err(|_| rejected())?;
    }
    EvidencePath::parse(path).map_err(|_| rejected())
}

fn check_class(text: &str, path: &str, scope: KnowledgeScope) -> AiResult<()> {
    let metadata = saved_metadata(text, path);
    if metadata.issue.is_some() || !scope.includes(metadata.source, metadata.history) {
        Err(stale())
    } else {
        Ok(())
    }
}

pub(crate) fn validate_hits_scoped(
    root: &Path,
    hits: &[NoteHit],
    scope: KnowledgeScope,
) -> AiResult<()> {
    for hit in hits {
        let path = read_path(&hit.path, scope)?;
        let note = vault::read_evidence(root, &path).map_err(|_| stale())?;
        check_class(&note.text, &hit.path, scope)?;
        if note.sha256 != hit.note_sha256
            || note.text.get(hit.start_byte..hit.end_byte) != Some(hit.quote.as_str())
        {
            return Err(stale());
        }
    }
    Ok(())
}

pub(crate) fn note_page(
    root: &Path,
    entries: Vec<IndexedNote>,
    folder: Option<&str>,
    cursor: Option<&str>,
) -> AiResult<NotePage> {
    note_page_scoped(root, entries, folder, cursor, KnowledgeScope::Current)
}

pub(crate) fn note_page_scoped(
    root: &Path,
    entries: Vec<IndexedNote>,
    folder: Option<&str>,
    cursor: Option<&str>,
    scope: KnowledgeScope,
) -> AiResult<NotePage> {
    if let Some(folder) = folder {
        if scope == KnowledgeScope::Current {
            VaultPath::validate_folder(folder).map_err(|_| rejected())?;
        } else {
            EvidencePath::validate_folder(folder).map_err(|_| rejected())?;
        }
    }
    if let Some(cursor) = cursor {
        read_path(cursor, scope)?;
    }
    let mut notes = Vec::new();
    for entry in entries {
        let path = read_path(&entry.path, scope)?;
        if folder.is_some_and(|folder| {
            !entry
                .path
                .strip_prefix(folder)
                .is_some_and(|suffix| suffix.starts_with('/'))
        }) || cursor.is_some_and(|cursor| entry.path.as_str() <= cursor)
        {
            continue;
        }
        let current = vault::read_evidence(root, &path).map_err(|_| stale())?;
        check_class(&current.text, &entry.path, scope)?;
        if current.sha256 != entry.sha256 {
            return Err(stale());
        }
        notes.push(NoteEntry {
            path: entry.path,
            title: entry.title,
        });
        if notes.len() == 201 {
            break;
        }
    }
    let next_cursor = if notes.len() > 200 {
        notes.pop();
        notes.last().map(|note| note.path.clone())
    } else {
        None
    };
    Ok(NotePage { notes, next_cursor })
}

impl ReadTools for AiTools {
    fn search_notes(&self, query: &str, limit: usize) -> AiResult<ToolSearch> {
        if !(1..=10).contains(&limit) {
            return Err(rejected());
        }
        brn_retrieval::note_index::check_query(query, limit).map_err(|_| rejected())?;
        let epoch = self.check_root()?;
        let mut embedder = self.embedder.clone();
        let results = {
            let reader = self
                .reader
                .lock()
                .map_err(|_| AiError::new(AiErrorKind::Storage))?;
            search_index(
                &*reader,
                embedder
                    .as_mut()
                    .map(|e| e as &mut dyn crate::library::Embedder),
                query,
                SearchMode::Hybrid,
                limit,
            )
            .map_err(|error| match error {
                LibraryError::Index(brn_retrieval::Error::Invalid(_)) => rejected(),
                LibraryError::Index(brn_retrieval::Error::ModelMismatch) => stale(),
                LibraryError::Index(brn_retrieval::Error::Corrupt(_)) => stale(),
                _ => AiError::new(AiErrorKind::Storage),
            })?
        };
        validate_hits(&self.root, &results.hits)?;
        self.check_current_epoch(epoch)?;
        Ok(ToolSearch {
            hits: results
                .hits
                .into_iter()
                .map(|hit| Passage {
                    path: hit.path,
                    start_byte: hit.start_byte,
                    end_byte: hit.end_byte,
                    quote: hit.quote,
                })
                .collect(),
            keyword_only: results.keyword_only,
        })
    }

    fn read_note(&self, path: &str) -> AiResult<ToolNote> {
        let parsed = VaultPath::parse(path).map_err(|_| rejected())?;
        let epoch = self.check_root()?;
        let note = vault::read_note(&self.root, &parsed).map_err(|_| rejected())?;
        check_class(&note.text, path, KnowledgeScope::Current).map_err(|_| rejected())?;
        let (text, truncated) = brn_ai::capped_text(&note.text);
        self.check_current_epoch(epoch)?;
        Ok(ToolNote {
            path: path.to_owned(),
            text: text.to_owned(),
            truncated,
        })
    }

    fn list_notes(&self, folder: Option<&str>, cursor: Option<&str>) -> AiResult<NotePage> {
        let epoch = self.check_root()?;
        let page = note_page(&self.root, self.entries()?, folder, cursor)?;
        self.check_current_epoch(epoch)?;
        Ok(page)
    }
}
