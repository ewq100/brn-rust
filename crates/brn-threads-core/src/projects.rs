//! Owner project commands. Preview, fences, Action choices and Trash are atomic.
use crate::store::{apply_on, prepare_on, receipt_on};
use crate::*;
use rusqlite::TransactionBehavior;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub title: String,
    pub instructions: String,
    pub use_brn_knowledge: bool,
    pub chats: Vec<String>,
    pub documents: Vec<String>,
    pub trash: Option<ProjectTrash>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectTrash {
    pub records: Vec<TrashedRecord>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrashedRecord {
    pub id: String,
    pub deleted_version: u64,
    pub was_archived: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectDeletePreview {
    pub project: String,
    pub snapshot: String,
    pub removed: Vec<Record>,
    pub retained: Vec<Record>,
    pub unfinished_actions: Vec<Record>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectActionChoice {
    Keep,
    Cancel,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectCommand {
    /// The host may preserve displayed public answer progress in the same group.
    WithProgress {
        command: Box<ProjectCommand>,
        run: String,
        partial: String,
    },
    Create {
        title: String,
    },
    CreateChat {
        project: String,
        title: String,
    },
    Configure {
        project: String,
        version: u64,
        title: String,
        instructions: String,
        use_brn_knowledge: bool,
    },
    MoveChat {
        chat: String,
        project: Option<String>,
    },
    LinkDocument {
        project: String,
        note: String,
    },
    UnlinkDocument {
        project: String,
        note: String,
    },
    CreateDocument {
        project: String,
        title: String,
        markdown: String,
    },
    Delete {
        preview: ProjectDeletePreview,
        actions: BTreeMap<String, ProjectActionChoice>,
    },
    Restore {
        project: String,
        version: u64,
    },
}
pub(crate) fn project_rows_on(conn: &rusqlite::Connection) -> Result<Vec<Record>> {
    let mut q = conn.prepare(
        "SELECT body FROM records WHERE json_type(body,'$.data.Project') IS NOT NULL ORDER BY id",
    )?;
    q.query_map([], |r| r.get::<_, String>(0))?
        .map(|s| Ok(serde_json::from_str(&s?)?))
        .collect()
}
impl Store {
    pub fn project_context_record(&self, chat: &str) -> Result<Option<Record>> {
        Ok(project_for_chat(&project_rows_on(&self.conn)?, chat).cloned())
    }
}
/// Host builders use these versioned writes together with their Project write.
pub fn project_context_fences(rows: &[Record], chats: &[String]) -> Result<Vec<Put>> {
    let mut writes = vec![];
    for r in rows {
        if let RecordData::Run(run) = &r.data
            && run.state == RunState::Working
            && chats.contains(&run.thread)
        {
            let mut w = put(r);
            let RecordData::Run(run) = &mut w.data else {
                unreachable!()
            };
            run.state = RunState::Interrupted;
            run.fence = run
                .fence
                .checked_add(1)
                .ok_or_else(|| Error::Invalid("run fence overflow".into()))?;

            writes.push(w);
        }
        if let RecordData::ChatControl(c) = &r.data
            && (c
                .pending
                .as_ref()
                .is_some_and(|q| chats.contains(&q.thread))
                || c.queue.iter().any(|q| chats.contains(&q.thread)) || rows.iter().any(|r| !r.archived && matches!(&r.data, RecordData::Run(run) if run.state == RunState::Working && chats.contains(&run.thread))))
        {
            let mut w = put(r);
            let RecordData::ChatControl(c) = &mut w.data else {
                unreachable!()
            };
            c.paused = true;
            if let Some(q) = &mut c.pending {
                q.run = None;
                q.dispatch = None;
            }
            writes.push(w);
        }
    }
    Ok(writes)
}

pub(crate) fn rows_on(conn: &rusqlite::Connection) -> Result<Vec<Record>> {
    let mut q = conn.prepare("SELECT body FROM records ORDER BY id")?;
    q.query_map([], |r| r.get::<_, String>(0))?
        .map(|s| Ok(serde_json::from_str(&s?)?))
        .collect()
}
pub fn project_for_chat<'a>(rows: &'a [Record], chat: &str) -> Option<&'a Record> {
    rows.iter()
        .find(|r| matches!(&r.data, RecordData::Project(p) if p.chats.iter().any(|id| id == chat)))
}
pub fn project_document_ids(rows: &[Record], record: &Record) -> BTreeSet<String> {
    let RecordData::Project(p) = &record.data else {
        return BTreeSet::new();
    };
    let mut ids: BTreeSet<_> = p.documents.iter().cloned().collect();
    for r in rows.iter().filter(|r| !r.archived) {
        if let RecordData::Link(l) = &r.data {
            let id = if l.from == record.id {
                Some(&l.to)
            } else if l.to == record.id {
                Some(&l.from)
            } else {
                None
            };
            if let Some(id) = id
                && rows
                    .iter()
                    .any(|r| r.id == *id && matches!(r.data, RecordData::Note(_)))
            {
                ids.insert(id.clone());
            }
        }
        if let RecordData::Note(n) = &r.data
            && n.chat.as_ref().is_some_and(|c| p.chats.contains(c))
        {
            ids.insert(r.id.clone());
        }
    }
    ids
}
fn project(rows: &[Record], id: &str) -> Result<Record> {
    rows.iter()
        .find(|r| r.id == id && !r.archived && matches!(r.data, RecordData::Project(_)))
        .cloned()
        .ok_or_else(|| Error::Invalid("Choose an active project".into()))
}
fn put(r: &Record) -> Put {
    Put {
        id: r.id.clone(),
        expected_version: Some(r.version),
        archived: r.archived,
        data: r.data.clone(),
    }
}
fn text(title: &str, instructions: &str) -> Result<()> {
    if title.trim().is_empty() || title.len() > 1024 || instructions.len() > 128 * 1024 {
        return Err(Error::Invalid(
            "Project needs a title up to 1024 bytes and instructions up to 128 KiB".into(),
        ));
    }
    Ok(())
}
fn preview(rows: &[Record], id: &str) -> Result<ProjectDeletePreview> {
    let r = project(rows, id)?;
    let RecordData::Project(p) = &r.data else {
        unreachable!()
    };
    let docs = project_document_ids(rows, &r);
    let mut exclusive = BTreeSet::new();
    let mut retained = vec![];
    for note in rows.iter().filter(|r| docs.contains(&r.id)) {
        let RecordData::Note(n) = &note.data else {
            continue;
        };
        let elsewhere = rows.iter().any(|other| {
            other.id != id
                && !other.archived
                && matches!(other.data, RecordData::Project(_))
                && project_document_ids(rows, other).contains(&note.id)
        });
        if n.chat.is_none() || elsewhere {
            retained.push(note.clone());
        } else {
            exclusive.insert(note.id.clone());
        }
    }
    let mut removed_ids: BTreeSet<_> = p.chats.iter().cloned().collect();
    removed_ids.extend(exclusive.iter().cloned());
    removed_ids.insert(id.into());
    // Dependent content stays recoverable, never eligible through an exact ID.
    for r in rows {
        match &r.data {
            RecordData::Message(m) if p.chats.contains(&m.thread) => {
                removed_ids.insert(r.id.clone());
            }
            RecordData::Run(run) if p.chats.contains(&run.thread) => {
                removed_ids.insert(r.id.clone());
            }
            RecordData::Asset(a) if exclusive.contains(&a.note) => {
                removed_ids.insert(r.id.clone());
            }
            RecordData::Comment(c) if exclusive.contains(&c.note) => {
                removed_ids.insert(r.id.clone());
            }
            _ => {}
        }
    }
    let mut action_ids: BTreeSet<_> = rows.iter().filter(|r| !r.archived && matches!(&r.data, RecordData::Action(a) if a.chat.as_ref().is_some_and(|owner| owner == id || p.chats.contains(owner)))).map(|r| r.id.clone()).collect();
    for r in rows.iter().filter(|r| !r.archived) {
        if let RecordData::Link(l) = &r.data
            && (removed_ids.contains(&l.from)
                || removed_ids.contains(&l.to)
                || docs.contains(&l.from)
                || docs.contains(&l.to))
        {
            for target in [&l.from, &l.to] {
                if rows
                    .iter()
                    .any(|r| r.id == *target && matches!(&r.data, RecordData::Action(a) if !a.kept_outside_project && a.chat.as_ref().is_none_or(|owner| owner == id || p.chats.contains(owner))))
                {
                    action_ids.insert(target.clone());
                }
            }
            if removed_ids.contains(&l.from) || removed_ids.contains(&l.to) {
                removed_ids.insert(r.id.clone());
            }
        }
    }
    for action in rows.iter().filter(|r| !r.archived && action_ids.contains(&r.id) && matches!(&r.data, RecordData::Action(a) if matches!(a.state, ActionState::Done | ActionState::Cancelled))) {
        let outside = rows.iter().any(|r| !r.archived && matches!(&r.data, RecordData::Link(l) if (l.from == action.id && !removed_ids.contains(&l.to)) || (l.to == action.id && !removed_ids.contains(&l.from))));
        let owned = matches!(&action.data, RecordData::Action(a) if a.chat.as_ref().is_some_and(|owner| owner == id || p.chats.contains(owner)));
        if owned || !outside { removed_ids.insert(action.id.clone()); }
    }
    for r in rows.iter().filter(|r| !r.archived) {
        if let RecordData::Link(l) = &r.data
            && (removed_ids.contains(&l.from) || removed_ids.contains(&l.to))
        {
            removed_ids.insert(r.id.clone());
        }
    }
    let unfinished_actions = rows.iter().filter(|r| !r.archived && action_ids.contains(&r.id) && matches!(&r.data, RecordData::Action(a) if !matches!(a.state, ActionState::Done | ActionState::Cancelled))).cloned().collect();
    let removed: Vec<_> = rows
        .iter()
        .filter(|r| removed_ids.contains(&r.id))
        .cloned()
        .collect();
    let snapshot = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(
            &removed,
            &retained,
            &unfinished_actions
        ))?)
    );
    Ok(ProjectDeletePreview {
        project: id.into(),
        snapshot,
        removed,
        retained,
        unfinished_actions,
    })
}
impl Store {
    pub fn project_delete_preview(&self, id: &str) -> Result<ProjectDeletePreview> {
        preview(&rows_on(&self.conn)?, id)
    }
    pub fn project_command(
        &mut self,
        op: &OperationId,
        command: &ProjectCommand,
        owner: &HostAuthority,
    ) -> Result<Receipt> {
        if !matches!(owner.scope, crate::types::Scope::Owner) {
            return Err(Error::Invalid(
                "Projects require owner configuration authority".into(),
            ));
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let binding = format!("{:x}", Sha256::digest(serde_json::to_vec(command)?));
        let prior: Option<String> = tx.query_row(
            "SELECT input_hash FROM operations WHERE id=?1",
            [op.as_str()],
            |r| r.get(0),
        )?;
        if prior.as_deref() != Some(&binding) {
            return Err(Error::ImmutableRequest);
        }
        if let Some(receipt) = receipt_on(&tx, op)? {
            tx.commit()?;
            return Ok(receipt);
        }
        let rows = rows_on(&tx)?;
        let (command, partial) = match command {
            ProjectCommand::WithProgress {
                command,
                run,
                partial,
            } => {
                if matches!(command.as_ref(), ProjectCommand::WithProgress { .. }) {
                    return Err(Error::Invalid(
                        "Project progress wrapper cannot be nested".into(),
                    ));
                }
                crate::chat::partial_text(partial)?;
                (command.as_ref(), Some((run, partial)))
            }
            command => (command, None),
        };
        let mut writes = BTreeMap::<String, Put>::new();
        let mut affected = BTreeSet::<String>::new();
        match command {
            ProjectCommand::WithProgress { .. } => {
                return Err(Error::Invalid("Nested project command".into()));
            }
            ProjectCommand::Create { title } => {
                text(title, "")?;
                let id = op.creation_id(0);
                writes.insert(
                    id.clone(),
                    Put {
                        id,
                        expected_version: None,
                        archived: false,
                        data: RecordData::Project(Project {
                            title: title.trim().into(),
                            instructions: String::new(),
                            use_brn_knowledge: true,
                            chats: vec![],
                            documents: vec![],
                            trash: None,
                        }),
                    },
                );
            }
            ProjectCommand::CreateChat { project: id, title } => {
                text(title, "")?;
                let mut r = project(&rows, id)?;
                let RecordData::Project(p) = &mut r.data else {
                    unreachable!()
                };
                affected.extend(p.chats.iter().cloned());
                let chat_id = op.creation_id(0);
                p.chats.push(chat_id.clone());
                writes.insert(
                    chat_id.clone(),
                    Put {
                        id: chat_id,
                        expected_version: None,
                        archived: false,
                        data: RecordData::Thread(Thread {
                            private_context: true,
                            context_epoch: 1,
                            title: title.trim().into(),
                            state: ThreadState::Open,
                            attention: vec![],
                        }),
                    },
                );
                writes.insert(r.id.clone(), put(&r));
            }
            ProjectCommand::Configure {
                project: id,
                version,
                title,
                instructions,
                use_brn_knowledge,
            } => {
                text(title, instructions)?;
                let mut r = project(&rows, id)?;
                if r.version != *version {
                    return Err(Error::Invalid(
                        "Project changed; refresh its settings".into(),
                    ));
                }
                let RecordData::Project(p) = &mut r.data else {
                    unreachable!()
                };
                affected.extend(p.chats.iter().cloned());
                p.title = title.trim().into();
                p.instructions = instructions.clone();
                p.use_brn_knowledge = *use_brn_knowledge;
                writes.insert(r.id.clone(), put(&r));
            }
            ProjectCommand::MoveChat {
                chat,
                project: destination,
            } => {
                let c = rows
                    .iter()
                    .find(|r| r.id == *chat && matches!(r.data, RecordData::Thread(_)))
                    .ok_or_else(|| Error::Invalid("Choose a saved active chat".into()))?;
                if project_for_chat(&rows, chat).is_some_and(|r| r.archived) {
                    return Err(Error::Invalid(
                        "Restore its project before moving this chat".into(),
                    ));
                }
                if let Some(id) = destination {
                    project(&rows, id)?;
                }
                for mut r in rows.iter().filter(|r| matches!(&r.data, RecordData::Project(p) if p.chats.contains(chat) || destination.as_ref() == Some(&r.id))).cloned() {
                    let RecordData::Project(p) = &mut r.data else { unreachable!() };
                    affected.extend(p.chats.iter().cloned());
                    p.chats.retain(|id| id != chat);
                    if destination.as_ref() == Some(&r.id) { p.chats.push(chat.clone()); }
                    writes.insert(r.id.clone(), put(&r));
                }
                // Membership changes version the chat, invalidating prepared context.
                let mut chat_write = put(c);
                if let RecordData::Thread(t) = &mut chat_write.data {
                    t.private_context |= destination.is_some();
                    t.context_epoch = t
                        .context_epoch
                        .checked_add(1)
                        .ok_or_else(|| Error::Invalid("context epoch overflow".into()))?;
                }
                writes.insert(c.id.clone(), chat_write);
                affected.insert(chat.clone());
            }
            ProjectCommand::LinkDocument { project: id, note }
            | ProjectCommand::UnlinkDocument { project: id, note } => {
                let mut r = project(&rows, id)?;
                let n = rows
                    .iter()
                    .find(|r| r.id == *note && !r.archived && matches!(r.data, RecordData::Note(_)))
                    .ok_or_else(|| Error::Invalid("Choose an active saved document".into()))?;
                let RecordData::Project(p) = &mut r.data else {
                    unreachable!()
                };
                affected.extend(p.chats.iter().cloned());
                p.documents.retain(|id| id != note);
                if matches!(command, ProjectCommand::LinkDocument { .. }) {
                    p.documents.push(note.clone());
                }
                if matches!(command, ProjectCommand::UnlinkDocument { .. }) {
                    if matches!(&n.data, RecordData::Note(n) if n.chat.as_ref().is_some_and(|owner| rows.iter().any(|r| r.id == *owner && matches!(r.data, RecordData::Project(_)))))
                        && !rows.iter().any(|other| {
                            other.id != *id
                                && !other.archived
                                && matches!(other.data, RecordData::Project(_))
                                && project_document_ids(&rows, other).contains(note)
                        })
                    {
                        return Err(Error::Invalid("An exclusive document needs Keep in BRN or another project link before removal".into()));
                    }
                    for link in rows.iter().filter(|r| !r.archived && matches!(&r.data, RecordData::Link(l) if (l.from == *id && l.to == *note) || (l.to == *id && l.from == *note))) {
                        let mut w = put(link); w.archived = true; writes.insert(w.id.clone(), w);
                    }
                }
                writes.insert(r.id.clone(), put(&r));
            }
            ProjectCommand::CreateDocument {
                project: id,
                title,
                markdown,
            } => {
                text(title, "")?;
                if markdown.len() > 50 * 1024 * 1024 {
                    return Err(Error::Invalid("Document exceeds 50 MiB".into()));
                }
                let mut r = project(&rows, id)?;
                let RecordData::Project(p) = &mut r.data else {
                    unreachable!()
                };
                affected.extend(p.chats.iter().cloned());
                let note_id = op.creation_id(0);
                p.documents.push(note_id.clone());
                let mut n = Note::working(title, markdown);
                n.chat = Some(id.clone());
                writes.insert(
                    note_id.clone(),
                    Put {
                        id: note_id,
                        expected_version: None,
                        archived: false,
                        data: RecordData::Note(n),
                    },
                );
                writes.insert(r.id.clone(), put(&r));
            }
            ProjectCommand::Delete {
                preview: approved,
                actions,
            } => {
                let current = preview(&rows, &approved.project)?;
                if current != *approved {
                    return Err(Error::Invalid(
                        "Project changed since deletion preview; refresh the removal set".into(),
                    ));
                }
                let expected: BTreeSet<_> = current
                    .unfinished_actions
                    .iter()
                    .map(|r| r.id.clone())
                    .collect();
                if expected != actions.keys().cloned().collect() {
                    return Err(Error::Invalid(
                        "Choose Keep outside project or Cancel for each unfinished Action".into(),
                    ));
                }
                let mut r = project(&rows, &approved.project)?;
                let RecordData::Project(p) = &mut r.data else {
                    unreachable!()
                };
                affected.extend(p.chats.iter().cloned());
                let mut trash = vec![];
                for removed in current.removed.iter().filter(|r| r.id != approved.project) {
                    // Action relationship choices are deliberate and never restored.
                    let action_link = matches!(&removed.data, RecordData::Link(l) if actions.contains_key(&l.from) || actions.contains_key(&l.to));
                    let mut w = put(removed);
                    w.archived = true;
                    if !action_link {
                        trash.push(TrashedRecord {
                            id: removed.id.clone(),
                            deleted_version: removed
                                .version
                                .checked_add(1)
                                .ok_or_else(|| Error::Invalid("version overflow".into()))?,
                            was_archived: removed.archived,
                        });
                    }
                    writes.insert(w.id.clone(), w);
                }
                for action in &current.unfinished_actions {
                    let mut w = put(action);
                    let RecordData::Action(a) = &mut w.data else {
                        unreachable!()
                    };
                    let owned = a
                        .chat
                        .as_ref()
                        .is_some_and(|owner| owner == &approved.project || p.chats.contains(owner));
                    if actions[&action.id] == ProjectActionChoice::Cancel {
                        a.state = ActionState::Cancelled;
                        let removed_ids: BTreeSet<_> =
                            current.removed.iter().map(|r| r.id.as_str()).collect();
                        let outside = rows.iter().any(|r| !r.archived && matches!(&r.data, RecordData::Link(l) if (l.from == w.id && !removed_ids.contains(l.to.as_str())) || (l.to == w.id && !removed_ids.contains(l.from.as_str()))));
                        w.archived = owned || !outside;
                    } else {
                        a.chat = None;
                        a.kept_outside_project = true;
                    }
                    if w.archived {
                        for link in rows.iter().filter(|r| !r.archived && matches!(&r.data, RecordData::Link(l) if l.from == w.id || l.to == w.id)) {
                            let mut link_write = put(link);
                            link_write.archived = true;
                            writes.insert(link_write.id.clone(), link_write);
                            trash.retain(|t| t.id != link.id);
                        }
                    }
                    writes.insert(w.id.clone(), w);
                }
                p.trash = Some(ProjectTrash { records: trash });
                r.archived = true;
                writes.insert(r.id.clone(), put(&r));
            }
            ProjectCommand::Restore {
                project: id,
                version,
            } => {
                let mut r = rows
                    .iter()
                    .find(|r| r.id == *id && r.archived && matches!(r.data, RecordData::Project(_)))
                    .cloned()
                    .ok_or_else(|| Error::Invalid("Choose a project in Trash".into()))?;
                if r.version != *version {
                    return Err(Error::Invalid(
                        "Trash changed; refresh before restoring".into(),
                    ));
                }
                let RecordData::Project(p) = &mut r.data else {
                    unreachable!()
                };
                let trash = p
                    .trash
                    .take()
                    .ok_or_else(|| Error::Invalid("Project has no recoverable deletion".into()))?;
                affected.extend(p.chats.iter().cloned());
                for t in trash.records {
                    let old = rows
                        .iter()
                        .find(|r| r.id == t.id)
                        .ok_or_else(|| Error::Missing(t.id.clone()))?;
                    if old.version != t.deleted_version || !old.archived {
                        return Err(Error::Invalid(format!(
                            "{} changed after deletion; preserved in Trash",
                            t.id
                        )));
                    }
                    let mut w = put(old);
                    w.archived = t.was_archived;
                    if old.archived != w.archived
                        && let RecordData::Run(run) = &mut w.data
                    {
                        run.fence = run
                            .fence
                            .checked_add(1)
                            .ok_or_else(|| Error::Invalid("run fence overflow".into()))?;
                    }
                    writes.insert(w.id.clone(), w);
                }
                let restored_notes: BTreeMap<_, _> = writes
                    .values()
                    .filter_map(|w| match &w.data {
                        RecordData::Note(n) if !w.archived => Some((
                            w.id.clone(),
                            (
                                w.expected_version.unwrap_or(0).saturating_sub(1),
                                w.expected_version.unwrap_or(0) + 1,
                                n.markdown.clone(),
                            ),
                        )),
                        _ => None,
                    })
                    .collect();
                for w in writes.values_mut() {
                    if !w.archived
                        && let RecordData::Comment(c) = &mut w.data
                        && let Some((before_archive, restored_version, text)) =
                            restored_notes.get(&c.note)
                    {
                        let range = c.mapped_range.or(c.range);
                        if !c.unresolved
                            && c.mapped_version.unwrap_or(c.base_version) == *before_archive
                            && range.is_some_and(|(start, end)| {
                                text.get(start..end) == Some(c.quote.as_str())
                            })
                        {
                            c.mapped_version = Some(*restored_version);
                            c.mapped_range = range;
                        } else if range.is_some() || !c.quote.is_empty() {
                            c.mapped_version = Some(*restored_version);
                            c.mapped_range = None;
                            c.unresolved = true;
                        }
                    }
                }
                r.archived = false;
                writes.insert(r.id.clone(), put(&r));
            }
        }
        // Fence every affected live invocation and pause saved work, preserving messages.
        let mut ended = vec![];
        for r in &rows {
            if let RecordData::Run(run) = &r.data
                && run.state == RunState::Working
                && affected.contains(&run.thread)
            {
                let mut w = writes.get(&r.id).cloned().unwrap_or_else(|| put(r));
                let RecordData::Run(run) = &mut w.data else {
                    unreachable!()
                };
                run.state = RunState::Interrupted;
                run.fence = run
                    .fence
                    .checked_add(1)
                    .ok_or_else(|| Error::Invalid("run fence overflow".into()))?;
                ended.push((w.id.clone(), run.clone()));
                writes.insert(w.id.clone(), w);
            }
        }
        if !affected.is_empty()
            && let Some(r) = rows
                .iter()
                .find(|r| matches!(r.data, RecordData::ChatControl(_)))
        {
            let mut w = put(r);
            let RecordData::ChatControl(c) = &mut w.data else {
                unreachable!()
            };
            if c.pending
                .as_ref()
                .is_some_and(|q| affected.contains(&q.thread))
                || c.queue.iter().any(|q| affected.contains(&q.thread))
                || rows.iter().any(|r| !r.archived && matches!(&r.data, RecordData::Run(run) if run.state == RunState::Working && affected.contains(&run.thread)))
            {
                c.paused = true;
                if let Some(q) = &mut c.pending {
                    q.run = None;
                    q.dispatch = None;
                }
                writes.insert(w.id.clone(), w);
            }
        }
        if let Some((run_id, text)) = partial
            && !text.trim().is_empty()
            && let Some(Record {
                data: RecordData::Run(run),
                ..
            }) = rows.iter().find(|r| r.id == *run_id)
            && run.state == RunState::Working
            && affected.contains(&run.thread)
        {
            let id = op.creation_id(1);
            let archived = writes.get(&run.thread).is_some_and(|w| w.archived);
            writes.insert(
                id.clone(),
                Put {
                    id: id.clone(),
                    expected_version: None,
                    archived,
                    data: RecordData::Message(Message {
                        thread: run.thread.clone(),
                        role: "assistant-partial".into(),
                        text: text.clone(),
                    }),
                },
            );
            if archived {
                for w in writes.values_mut() {
                    if let RecordData::Project(p) = &mut w.data
                        && p.chats.contains(&run.thread)
                        && let Some(trash) = &mut p.trash
                    {
                        trash.records.push(TrashedRecord {
                            id: id.clone(),
                            deleted_version: 1,
                            was_archived: false,
                        });
                    }
                }
            }
        }
        // The owner acted on the project, not in these chats: each stopped Run
        // leaves its notice and reviews, inside a trashed chat's own write.
        let mut writes: Vec<Put> = writes.into_values().collect();
        crate::run_attention::ended_run_attention(
            &mut writes,
            &ended,
            true,
            |id| crate::store::record_on(&tx, id),
            |operations| crate::run_attention::active_candidates_on(&tx, operations),
        )?;
        let request = ChangeRequest {
            reason: "Owner project operation".into(),
            writes,
            inputs: vec![],
        };
        prepare_on(&tx, op, &request, true)?;
        let outcome = apply_on(&tx, op, owner, None)?;
        let ApplyOutcome::Applied(receipt) = outcome else {
            return Err(Error::Invalid(format!(
                "Project operation preserved unchanged: {outcome:?}"
            )));
        };
        tx.commit()?;
        Ok(receipt)
    }
}
/// Checked writer validates membership even for raw owner proposals and Undo.
pub(crate) fn validate_projects(rows: &[Record]) -> Result<()> {
    let mut members = BTreeSet::new();
    for r in rows {
        if let RecordData::Project(p) = &r.data {
            text(&p.title, &p.instructions)?;
            let mut docs = BTreeSet::new();
            for id in &p.chats {
                if !members.insert(id)
                    || !rows
                        .iter()
                        .any(|r| r.id == *id && matches!(r.data, RecordData::Thread(_)))
                {
                    return Err(Error::Invalid(
                        "Chat must exist and belong to at most one project".into(),
                    ));
                }
            }
            for id in &p.documents {
                if !docs.insert(id)
                    || !rows
                        .iter()
                        .any(|r| r.id == *id && matches!(r.data, RecordData::Note(_)))
                {
                    return Err(Error::Invalid(
                        "Project documents must be unique existing notes".into(),
                    ));
                }
            }
        }
    }
    Ok(())
}
