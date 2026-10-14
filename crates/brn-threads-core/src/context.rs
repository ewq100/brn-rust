//! The same context predicate governs searches, exact IDs and historical reads.
use crate::*;
use std::collections::BTreeSet;

/// Membership is current host-owned state, including deleted projects.
pub fn project_scope<'a>(rows: &'a [Record], chat: Option<&str>) -> Option<&'a Record> {
    chat.and_then(|chat| {
        crate::project_for_chat(rows, chat).or_else(|| {
            rows.iter()
                .find(|r| r.id == chat && matches!(r.data, RecordData::Project(_)))
        })
    })
}
pub(crate) fn scope_allowed(owner: &str, rows: &[Record], chat: Option<&str>) -> bool {
    if Some(owner) == chat {
        return !project_scope(rows, chat).is_some_and(|p| p.archived);
    }
    project_scope(rows, chat).is_some_and(|r| !r.archived && (r.id == owner || matches!(&r.data, RecordData::Project(p) if p.chats.iter().any(|id| id == owner) && rows.iter().any(|r| r.id == owner && !r.archived && matches!(&r.data, RecordData::Thread(t) if t.state == ThreadState::Open)))))
}
fn document_allowed(id: &str, owner: Option<&str>, rows: &[Record], chat: Option<&str>) -> bool {
    if let Some(project) = project_scope(rows, chat) {
        if project.archived {
            return false;
        }
        if crate::project_document_ids(rows, project).contains(id) {
            return true;
        }
        if let RecordData::Project(p) = &project.data
            && !p.use_brn_knowledge
            && owner.is_none()
        {
            return rows.iter().any(|r| !r.archived && matches!(&r.data, RecordData::Link(l) if (Some(l.from.as_str()) == chat && l.to == id) || (Some(l.to.as_str()) == chat && l.from == id)));
        }
    }
    if owner.is_some_and(|owner| {
        rows.iter()
            .any(|r| r.id == owner && matches!(r.data, RecordData::Project(_)))
    }) {
        return false;
    }
    owner.is_none_or(|owner| scope_allowed(owner, rows, chat))
}
fn source_allowed(id: &str, owner: Option<&str>, rows: &[Record], chat: Option<&str>) -> bool {
    // A shared document grants its own retained source and assets, not the source project's other material.
    let retained = project_scope(rows, chat).is_some_and(|project| {
        if project.archived { return false; }
        let docs = crate::project_document_ids(rows, project);
        let accessible = |id: &str| rows.iter().any(|r| r.id == id && !r.archived && matches!(&r.data, RecordData::Note(n) if docs.contains(id) || (owner.is_none() && document_allowed(id, n.chat.as_deref(), rows, chat))));
        rows.iter().any(|r| !r.archived && match &r.data {
            RecordData::Note(n) => n.import.as_ref().is_some_and(|i| i.source == id) && accessible(&r.id),
            RecordData::Asset(a) => a.source.as_deref() == Some(id) && accessible(&a.note),
            _ => false,
        })
    });
    retained || (owner.is_none_or(|owner| {
        !rows
            .iter()
            .any(|r| r.id == owner && matches!(r.data, RecordData::Project(_)))
            && scope_allowed(owner, rows, chat)
    }) && !project_scope(rows, chat).is_some_and(|r| {
        r.archived
            || matches!(&r.data, RecordData::Project(p) if !p.use_brn_knowledge && owner.is_none())
    }))
}

pub fn visible(record: &Record, records: &[Record], chat: Option<&str>) -> bool {
    fn visit(r: &Record, rows: &[Record], chat: Option<&str>, seen: &mut BTreeSet<String>) -> bool {
        if !seen.insert(r.id.clone()) {
            return false;
        }
        let linked = |id: &str, seen: &mut BTreeSet<String>| {
            rows.iter()
                .find(|r| r.id == id)
                .is_some_and(|r| visit(r, rows, chat, seen))
        };
        match &r.data {
            RecordData::Note(n) => {
                document_allowed(&r.id, n.chat.as_deref(), rows, chat)
                    && n.import
                        .as_ref()
                        .is_none_or(|i| linked(&i.source, &mut seen.clone()))
                    && n.superseded_by
                        .as_ref()
                        .is_none_or(|id| linked(id, &mut seen.clone()))
            }
            RecordData::Source(s) => source_allowed(&r.id, s.chat.as_deref(), rows, chat),
            RecordData::Asset(a) => {
                linked(&a.note, &mut seen.clone())
                    && a.source
                        .as_ref()
                        .is_none_or(|id| linked(id, &mut seen.clone()))
            }
            RecordData::Comment(c) => linked(&c.note, seen),
            RecordData::Link(l) => {
                // Branches need separate cycle guards when they share an ancestor.
                linked(&l.from, &mut seen.clone()) && linked(&l.to, &mut seen.clone())
            }
            RecordData::Thread(_) => scope_allowed(&r.id, rows, chat),
            RecordData::Message(m) => scope_allowed(&m.thread, rows, chat),
            RecordData::Run(r) => {
                Some(r.thread.as_str()) == chat
                    && !project_scope(rows, chat).is_some_and(|p| p.archived)
            }
            RecordData::Action(_) if r.archived => false,
            RecordData::Action(a) if a.kept_outside_project => !project_scope(rows, chat)
                .is_some_and(|p| {
                    p.archived || matches!(&p.data, RecordData::Project(p) if !p.use_brn_knowledge)
                }),
            RecordData::Action(a) if a.chat.is_some() => {
                let owner = a.chat.as_deref().unwrap();
                scope_allowed(owner, rows, chat) || project_scope(rows, chat).is_some_and(|r| !r.archived && matches!(&r.data, RecordData::Project(p) if p.chats.iter().any(|id| id == owner)))
            }
            RecordData::Action(_) => {
                let endpoints: Vec<_> = rows
                    .iter()
                    .filter(|row| !row.archived)
                    .filter_map(|row| match &row.data {
                        RecordData::Link(l) if l.from == r.id => Some(l.to.as_str()),
                        RecordData::Link(l) if l.to == r.id => Some(l.from.as_str()),
                        _ => None,
                    })
                    .collect();
                if endpoints.is_empty() {
                    !project_scope(rows, chat).is_some_and(|p| {
                        p.archived
                            || matches!(&p.data, RecordData::Project(p) if !p.use_brn_knowledge)
                    })
                } else {
                    endpoints.iter().any(|id| scope_allowed(id, rows, chat) || project_scope(rows, chat).is_some_and(|p| !p.archived && p.id == *id) || rows.iter().any(|row| row.id == *id && matches!(&row.data, RecordData::Note(n) if document_allowed(id, n.chat.as_deref(), rows, chat))))
                }
            }
            RecordData::Project(_) => {
                project_scope(rows, chat).is_some_and(|p| !p.archived && p.id == r.id)
            }
            RecordData::Settings(_) | RecordData::ChatControl(_) => false,
        }
    }
    visit(record, records, chat, &mut BTreeSet::new())
}

impl Store {
    pub fn records_for_chat(&self, chat: Option<&str>) -> Result<Vec<Record>> {
        let rows = self.records()?;
        Ok(rows
            .iter()
            .filter(|r| {
                !r.archived
                    && !matches!(&r.data, RecordData::Note(n) if n.superseded_by.is_some())
                    && visible(r, &rows, chat)
            })
            .cloned()
            .collect())
    }
    pub fn record_visible_in_chat(&self, id: &str, chat: Option<&str>) -> Result<bool> {
        let rows = self.records()?;
        Ok(rows
            .iter()
            .find(|r| r.id == id)
            .is_some_and(|r| visible(r, &rows, chat)))
    }
    pub fn revision_visible_in_chat(
        &self,
        id: &str,
        version: u64,
        chat: Option<&str>,
    ) -> Result<bool> {
        let rows = self.records()?;
        let current = rows.iter().find(|r| r.id == id);
        Ok(current.is_some_and(|r| visible(r, &rows, chat))
            && self
                .revision(id, version)?
                .as_ref()
                .is_some_and(|r| visible(r, &rows, chat)))
    }
    /// Archived unpromoted inputs still restrict information retained in chat history.
    pub fn chat_has_private_context(&self, chat: &str) -> Result<bool> {
        let rows = self.records()?;
        Ok(crate::project_for_chat(&rows, chat).is_some()
            || rows.iter().any(|r| {
                matches!(&r.data, RecordData::Note(n) if n.chat.as_deref() == Some(chat))
                    || matches!(&r.data, RecordData::Thread(t) if r.id == chat && t.private_context)
            }))
    }
}

/// Managed URL tokens use the same complete-identity delimiters as export.
pub fn managed_asset_ids(text: &str) -> impl Iterator<Item = &str> {
    text.split("brn-asset://").skip(1).map(|rest| {
        let end = rest
            .find(|c: char| c.is_whitespace() || matches!(c, ')' | '>' | '<' | '"' | '\'' | '`'))
            .unwrap_or(rest.len());
        &rest[..end]
    })
}
