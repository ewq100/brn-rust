//! Bounded read-only saved relationship presentation over existing worker DTOs.
use super::{AiState, Pending};
use brn_workflow::{
    app_worker::AppCommand,
    knowledge::{NoteLinks, RelationshipPage, RelationshipRequest},
    library::KnowledgeScope,
};
use uuid::Uuid;

impl AiState {
    pub fn clear_links(&mut self) {
        self.links_generation = self.links_generation.wrapping_add(1);
        self.links = None;
        self.links_error = None;
    }
    pub fn links_loading(&self) -> bool {
        self.pending.values().any(|pending| {
            matches!(pending,
            Pending::Links { path, document_generation, inspection_generation }
                if self.links_request_matches(path, *document_generation, *inspection_generation))
        })
    }
    pub fn inspect_links(&mut self) -> Option<(Uuid, AppCommand)> {
        if !self.ready || !self.vault_bound || self.application_busy() || self.links_loading() {
            return None;
        }
        let path = self.saved_document_path()?.to_owned();
        self.clear_links();
        Some(self.command(
            Pending::Links {
                path: path.clone(),
                document_generation: self.note_generation,
                inspection_generation: self.links_generation,
            },
            AppCommand::NoteLinks(path),
        ))
    }
    pub(super) fn links_request_matches(&self, path: &str, document: u64, inspection: u64) -> bool {
        self.saved_document_path() == Some(path)
            && self.note_generation == document
            && self.links_generation == inspection
    }
    pub(super) fn received_links(&mut self, pending: Option<&Pending>, links: NoteLinks) {
        if let Some(Pending::Links {
            path,
            document_generation,
            inspection_generation,
        }) = pending
            && links.source.path == *path
            && self.links_request_matches(path, *document_generation, *inspection_generation)
        {
            self.links = Some(links);
            self.links_error = None;
        }
    }

    pub fn clear_relationships(&mut self) {
        self.relationships_generation = self.relationships_generation.wrapping_add(1);
        self.relationships = None;
        self.relationships_error = None;
    }
    pub fn relationships_loading(&self) -> bool {
        self.pending.values().any(|pending| {
            matches!(pending,
            Pending::Relationships { scope, generation, .. }
                if self.relationship_request_matches(*scope, *generation))
        })
    }
    pub fn refresh_relationships(&mut self, offset: usize) -> Option<(Uuid, AppCommand)> {
        if !self.ready
            || !self.vault_bound
            || self.application_busy()
            || self.relationships_loading()
        {
            return None;
        }
        self.clear_relationships();
        let request = RelationshipRequest {
            scope: self.knowledge_scope,
            offset,
            limit: 25,
        };
        Some(self.command(
            Pending::Relationships {
                scope: request.scope,
                offset,
                limit: request.limit,
                generation: self.relationships_generation,
            },
            AppCommand::Relationships(request),
        ))
    }
    pub fn relationship_next_offset(&self) -> Option<usize> {
        let page = self.relationships.as_ref()?;
        page.offset
            .checked_add(25)
            .filter(|offset| *offset < page.total)
    }
    pub fn relationship_previous_offset(&self) -> Option<usize> {
        let page = self.relationships.as_ref()?;
        (page.offset > 0).then(|| page.offset.saturating_sub(25))
    }
    pub(super) fn relationship_request_matches(
        &self,
        scope: KnowledgeScope,
        generation: u64,
    ) -> bool {
        scope == self.knowledge_scope && generation == self.relationships_generation
    }
    pub(super) fn received_relationships(
        &mut self,
        pending: Option<&Pending>,
        page: RelationshipPage,
    ) {
        if let Some(Pending::Relationships {
            scope,
            offset,
            limit,
            generation,
        }) = pending
            && self.relationship_request_matches(*scope, *generation)
            && page.scope == *scope
            && page.offset == *offset
            && page.edges.len() == (*limit).min(page.total.saturating_sub(*offset))
        {
            self.relationships = Some(page);
            self.relationships_error = None;
        }
    }
}
