//! The same context predicate governs searches, exact IDs and historical reads.
use crate::*;
use std::collections::BTreeSet;

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
                n.chat.as_deref().is_none_or(|owner| Some(owner) == chat)
                    && n.import
                        .as_ref()
                        .is_none_or(|i| linked(&i.source, &mut seen.clone()))
                    && n.superseded_by
                        .as_ref()
                        .is_none_or(|id| linked(id, &mut seen.clone()))
            }
            RecordData::Source(s) => s.chat.as_deref().is_none_or(|owner| Some(owner) == chat),
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
            RecordData::Thread(_) => Some(r.id.as_str()) == chat,
            RecordData::Message(m) => Some(m.thread.as_str()) == chat,
            RecordData::Run(r) => Some(r.thread.as_str()) == chat,
            RecordData::Action(_) => true,
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
        Ok(self.records()?.iter().any(|r| {
            matches!(&r.data,
            RecordData::Note(n) if n.chat.as_deref() == Some(chat))
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
