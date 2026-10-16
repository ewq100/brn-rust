//! Read-only snapshots for derived data such as the search index. They never
//! upgrade or write the canonical database, and check visibility over only
//! the records a candidate references instead of the whole workspace.
use crate::store::{has_symlink_ancestor, validate_connection};
use crate::*;
use rusqlite::{Connection, OpenFlags, OptionalExtension, Transaction, params};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    time::Duration,
};

/// Where a current note may appear in search. `General` notes are visible in
/// every context; `Chat` notes only in that conversation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SearchEligibility {
    General,
    Chat(String),
    Hidden,
}

/// Derived from [`context::visible`] itself, so it cannot drift from core
/// rules. Archived and superseded notes are hidden, as in `records_for_chat`.
/// `rows` must contain the record's references (see [`Snapshot::with_references`]).
pub fn search_eligibility(record: &Record, rows: &[Record]) -> SearchEligibility {
    let RecordData::Note(note) = &record.data else {
        return SearchEligibility::Hidden;
    };
    if record.archived || note.superseded_by.is_some() {
        return SearchEligibility::Hidden;
    }
    if crate::context::visible(record, rows, None) {
        return SearchEligibility::General;
    }
    // Only the note's own chat or its import source's chat can admit it.
    let source_chat = note.import.as_ref().and_then(|import| {
        rows.iter()
            .find(|r| r.id == import.source)
            .and_then(|r| match &r.data {
                RecordData::Source(source) => source.chat.clone(),
                _ => None,
            })
    });
    if let Some(owner) = &note.chat
        && rows
            .iter()
            .any(|r| r.id == *owner && matches!(r.data, RecordData::Project(_)))
    {
        return SearchEligibility::Chat(owner.clone());
    }
    [note.chat.clone(), source_chat]
        .into_iter()
        .flatten()
        .find(|chat| crate::context::visible(record, rows, Some(chat)))
        .map_or(SearchEligibility::Hidden, SearchEligibility::Chat)
}

/// Where a chat's saved Messages may appear in owner search, mirroring notes:
/// a general chat wherever general notes appear, a project chat only in its
/// project, and a chat holding private context only in itself. This is the
/// owner's search scope; agent runs keep [`crate::record_in_chat`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MessageScope {
    General,
    Project(String),
    Chat(String),
    Hidden,
}

/// Message roles owner search indexes: final owner and assistant Messages,
/// never partial progress or writing requests.
pub const SEARCHABLE_MESSAGE_ROLES: [&str; 2] = ["owner", "assistant"];

/// A current Message owner search may index; its chat's scope still applies.
pub fn message_searchable(record: &Record) -> bool {
    !record.archived
        && matches!(&record.data, RecordData::Message(m) if SEARCHABLE_MESSAGE_ROLES.contains(&m.role.as_str()))
}

/// The owner-search scope of `thread`'s Messages. Archived chats (Archive,
/// and a project Delete that moved them to Trash) are hidden. `rows` must
/// hold the chat, every Project and the Notes saved in the chat (see
/// [`Snapshot::message_scope_rows`]).
pub fn message_scope(thread: &Record, rows: &[Record]) -> MessageScope {
    if thread.archived || !matches!(thread.data, RecordData::Thread(_)) {
        return MessageScope::Hidden;
    }
    if let Some(project) = crate::project_for_chat(rows, &thread.id) {
        return if project.archived {
            MessageScope::Hidden
        } else {
            MessageScope::Project(project.id.clone())
        };
    }
    if crate::context::chat_private(rows, &thread.id) {
        MessageScope::Chat(thread.id.clone())
    } else {
        MessageScope::General
    }
}

/// The scopes whose Messages owner search shows in `context` (`None` =
/// general). A project, or a chat in it, shows the project's chats, plus
/// general chats when the project uses BRN knowledge, as for general notes;
/// a project in Trash shows none. A general chat also shows itself. `rows`
/// must hold every Project.
pub fn message_scopes_for(rows: &[Record], context: Option<&str>) -> Vec<MessageScope> {
    let Some(context) = context else {
        return vec![MessageScope::General];
    };
    match crate::project_scope(rows, Some(context)) {
        Some(project) if project.archived => vec![],
        Some(project) => {
            let mut scopes = vec![MessageScope::Project(project.id.clone())];
            if matches!(&project.data, RecordData::Project(p) if p.use_brn_knowledge) {
                scopes.push(MessageScope::General);
            }
            scopes
        }
        None => vec![
            MessageScope::General,
            MessageScope::Chat(context.to_owned()),
        ],
    }
}

pub struct StoreReader {
    conn: Connection,
    directory: PathBuf,
}

impl Store {
    /// Opens a validated, read-only view of an existing current-schema
    /// workspace. Older schemas are refused rather than upgraded.
    pub fn open_reader(directory: impl AsRef<Path>) -> Result<StoreReader> {
        let directory = directory.as_ref();
        if !directory.is_absolute()
            || has_symlink_ancestor(directory)?
            || std::fs::read_to_string(directory.join(DATA_MARKER))? != "BRN Threads SQLite v1\n"
        {
            return Err(Error::UnsafeDirectory(
                "validated explicit Threads directory required".into(),
            ));
        }
        let conn = Connection::open_with_flags(
            directory.join(DATABASE_FILENAME),
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        conn.busy_timeout(Duration::from_secs(5))?;
        validate_connection(&conn)?;
        Ok(StoreReader {
            conn,
            directory: directory.to_owned(),
        })
    }
}

impl StoreReader {
    /// The workspace's `derived/` directory, re-created with the same checks
    /// as [`Store::derived_directory`] if it was removed.
    pub fn derived_directory(&self) -> Result<PathBuf> {
        crate::store::ensure_derived(&self.directory)
    }
    /// One consistent read snapshot; it ends when dropped.
    pub fn snapshot(&mut self) -> Result<Snapshot<'_>> {
        Ok(Snapshot {
            tx: self.conn.transaction()?,
        })
    }
}

pub struct Snapshot<'a> {
    tx: Transaction<'a>,
}

fn record_from(body: String) -> Result<Record> {
    Ok(serde_json::from_str(&body)?)
}

impl Snapshot<'_> {
    /// Rises with every applied record write (all writes add a revision in
    /// the same transaction); 0 for a workspace without writes.
    pub fn watermark(&self) -> Result<i64> {
        Ok(self
            .tx
            .query_row("SELECT coalesce(max(rowid), 0) FROM revisions", [], |r| {
                r.get(0)
            })?)
    }
    /// The workspace's first operation, distinguishing workspaces whose
    /// watermarks happen to match.
    pub fn workspace_identity(&self) -> Result<Option<String>> {
        Ok(self
            .tx
            .query_row(
                "SELECT id FROM operations ORDER BY rowid LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()?)
    }
    pub fn records(&self) -> Result<Vec<Record>> {
        let mut stmt = self.tx.prepare("SELECT body FROM records ORDER BY id")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.map(|row| record_from(row?)).collect()
    }
    /// Current records written after `watermark`, in id order.
    pub fn changed_since(&self, watermark: i64) -> Result<Vec<Record>> {
        let mut stmt = self.tx.prepare(
            "SELECT body FROM records WHERE id IN \
             (SELECT id FROM revisions WHERE rowid > ?1) ORDER BY id",
        )?;
        let rows = stmt.query_map([watermark], |r| r.get::<_, String>(0))?;
        rows.map(|row| record_from(row?)).collect()
    }
    pub fn record(&self, id: &str) -> Result<Option<Record>> {
        self.tx
            .query_row("SELECT body FROM records WHERE id=?1", params![id], |r| {
                r.get::<_, String>(0)
            })
            .optional()?
            .map(record_from)
            .transpose()
    }
    /// The records named by `ids` plus everything core visibility follows
    /// from them: import sources, supersession targets, asset and comment
    /// notes, and link ends. Missing ids are skipped.
    pub fn with_references(&self, ids: &[String]) -> Result<Vec<Record>> {
        let mut found = BTreeMap::new();
        let mut pending: Vec<String> = ids.to_vec();
        while let Some(id) = pending.pop() {
            if found.contains_key(&id) {
                continue;
            }
            let Some(record) = self.record(&id)? else {
                continue;
            };
            match &record.data {
                RecordData::Note(note) => {
                    pending.extend(note.import.as_ref().map(|i| i.source.clone()));
                    pending.extend(note.superseded_by.clone());
                    pending.extend(note.chat.clone());
                }
                RecordData::Action(a) => pending.extend(a.chat.clone()),
                RecordData::Source(_) => {
                    let mut importing = self.tx.prepare("SELECT id FROM records WHERE json_type(body,'$.data.Note') IS NOT NULL AND json_extract(body,'$.data.Note.import.source')=?1")?;
                    for id in importing.query_map([&record.id], |r| r.get::<_, String>(0))? {
                        pending.push(id?);
                    }
                    let mut assets = self.tx.prepare("SELECT id FROM records WHERE json_type(body,'$.data.Asset') IS NOT NULL AND json_extract(body,'$.data.Asset.source')=?1")?;
                    for id in assets.query_map([&record.id], |r| r.get::<_, String>(0))? {
                        pending.push(id?);
                    }
                }
                RecordData::Asset(asset) => {
                    pending.push(asset.note.clone());
                    pending.extend(asset.source.clone());
                }
                RecordData::Comment(comment) => pending.push(comment.note.clone()),
                RecordData::Link(link) => {
                    pending.push(link.from.clone());
                    pending.push(link.to.clone());
                }
                _ => {}
            }
            found.insert(id, record);
        }
        let mut projects = self.tx.prepare(
            "SELECT body FROM records WHERE json_type(body,'$.data.Project') IS NOT NULL",
        )?;
        for body in projects.query_map([], |r| r.get::<_, String>(0))? {
            let r = record_from(body?)?;
            found.insert(r.id.clone(), r);
        }
        if found
            .values()
            .any(|r| matches!(r.data, RecordData::Project(_)))
        {
            let ids: Vec<_> = found.keys().cloned().collect();
            let mut links = self.tx.prepare("SELECT body FROM records WHERE json_type(body,'$.data.Link') IS NOT NULL AND (json_extract(body,'$.data.Link.from')=?1 OR json_extract(body,'$.data.Link.to')=?1) AND json_extract(body,'$.archived')=0")?;
            for id in ids {
                for body in links.query_map([id], |r| r.get::<_, String>(0))? {
                    let r = record_from(body?)?;
                    found.insert(r.id.clone(), r);
                }
            }
        }
        Ok(found.into_values().collect())
    }
    /// Of `ids`, the non-archived records core visibility admits in `chat`
    /// (`None` = general). Superseded notes are excluded, as in
    /// `records_for_chat`.
    pub fn visible_records(&self, ids: &[String], chat: Option<&str>) -> Result<BTreeSet<String>> {
        let rows = self.with_references(ids)?;
        Ok(ids
            .iter()
            .filter_map(|id| rows.iter().find(|r| &r.id == id))
            .filter(|r| {
                !r.archived
                    && !matches!(&r.data, RecordData::Note(n) if n.superseded_by.is_some())
                    && crate::context::visible(r, &rows, chat)
            })
            .map(|r| r.id.clone())
            .collect())
    }
    /// The records [`message_scope`] and [`message_scopes_for`] read for
    /// `chats`: those chats, every Project, and the Notes saved in them.
    pub fn message_scope_rows(&self, chats: &[String]) -> Result<Vec<Record>> {
        let mut found = BTreeMap::new();
        let mut projects = self.tx.prepare(
            "SELECT body FROM records WHERE json_type(body,'$.data.Project') IS NOT NULL",
        )?;
        for body in projects.query_map([], |r| r.get::<_, String>(0))? {
            let r = record_from(body?)?;
            found.insert(r.id.clone(), r);
        }
        let mut notes = self.tx.prepare("SELECT body FROM records WHERE json_type(body,'$.data.Note') IS NOT NULL AND json_extract(body,'$.data.Note.chat')=?1")?;
        for chat in chats {
            if let Some(r) = self.record(chat)? {
                found.insert(r.id.clone(), r);
            }
            for body in notes.query_map([chat], |r| r.get::<_, String>(0))? {
                let r = record_from(body?)?;
                found.insert(r.id.clone(), r);
            }
        }
        Ok(found.into_values().collect())
    }
    /// The Message scopes owner search shows in `context`; see
    /// [`message_scopes_for`].
    pub fn message_scopes(&self, context: Option<&str>) -> Result<Vec<MessageScope>> {
        Ok(message_scopes_for(&self.message_scope_rows(&[])?, context))
    }
    /// Of `ids`, the Messages owner search may show in `context` (`None` =
    /// general): current final owner and assistant Messages whose chat's
    /// [`message_scope`] is among the context's [`message_scopes_for`].
    pub fn searchable_messages(
        &self,
        ids: &[String],
        context: Option<&str>,
    ) -> Result<BTreeSet<String>> {
        let mut messages = Vec::new();
        for id in ids {
            if let Some(record) = self.record(id)?
                && message_searchable(&record)
            {
                messages.push(record);
            }
        }
        let chats: Vec<String> = messages
            .iter()
            .filter_map(|r| match &r.data {
                RecordData::Message(m) => Some(m.thread.clone()),
                _ => None,
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let rows = self.message_scope_rows(&chats)?;
        let allowed = message_scopes_for(&rows, context);
        let shown: BTreeSet<&str> = chats
            .iter()
            .filter(|chat| {
                rows.iter()
                    .find(|r| r.id == **chat)
                    .is_some_and(|thread| allowed.contains(&message_scope(thread, &rows)))
            })
            .map(String::as_str)
            .collect();
        Ok(messages
            .iter()
            .filter(
                |r| matches!(&r.data, RecordData::Message(m) if shown.contains(m.thread.as_str())),
            )
            .map(|r| r.id.clone())
            .collect())
    }
    /// Of `ids`, the current, non-archived, non-superseded notes visible in
    /// `chat` (`None` = general), matching `records_for_chat` for notes.
    pub fn searchable_notes(&self, ids: &[String], chat: Option<&str>) -> Result<BTreeSet<String>> {
        let rows = self.with_references(ids)?;
        Ok(ids
            .iter()
            .filter_map(|id| rows.iter().find(|r| &r.id == id))
            .filter(|r| {
                !r.archived
                    && matches!(&r.data, RecordData::Note(n) if n.superseded_by.is_none())
                    && crate::context::visible(r, &rows, chat)
            })
            .map(|r| r.id.clone())
            .collect())
    }
}
