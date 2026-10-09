//! Derived, read-only citation observations, isolated from owner editing buffers.
use super::{AiState, Pending};
use brn_workflow::{
    app_worker::{AppCommand, AppEvent},
    knowledge::{
        CitationReviewDetail, CitationReviewDetailRequest, CitationReviewEntry, CitationReviewPage,
        CitationReviewRequest,
    },
};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NeedsReviewMode {
    #[default]
    Findings,
    CitationEvidence,
}
#[derive(Clone)]
pub struct PageCapture {
    view: u64,
    page: u64,
}
#[derive(Clone)]
pub struct RowCapture {
    page: PageCapture,
    entry: CitationReviewEntry,
}
#[derive(Clone)]
pub struct SelectionCapture {
    row: RowCapture,
    selection: u64,
}
#[derive(Default)]
pub struct CitationReviewView {
    pub visible: bool,
    pub page: Option<CitationReviewPage>,
    pub selected: Option<CitationReviewEntry>,
    pub detail: Option<CitationReviewDetail>,
    pub error: Option<String>,
    view: u64,
    page_generation: u64,
    selection: u64,
    request: Option<CitationReviewRequest>,
}
impl AiState {
    pub fn open_citation_review(&mut self) -> Option<(Uuid, AppCommand)> {
        if !self.ready || !self.vault_bound {
            return None;
        }
        self.close_findings();
        self.needs_review_mode = NeedsReviewMode::CitationEvidence;
        self.citation_review.visible = true;
        self.refresh_citation_review()
    }
    pub fn close_citation_review(&mut self) {
        let view = &mut self.citation_review;
        view.visible = false;
        view.view = view.view.wrapping_add(1);
    }
    pub(super) fn invalidate_citation_review(&mut self) {
        let view = &mut self.citation_review;
        view.view = view.view.wrapping_add(1);
        view.page_generation = view.page_generation.wrapping_add(1);
        view.selection = view.selection.wrapping_add(1);
        view.page = None;
        view.selected = None;
        view.detail = None;
        view.request = None;
        view.error = view
            .visible
            .then(|| "Vault context changed. Refresh citation evidence to inspect again.".into());
    }
    pub fn refresh_citation_review(&mut self) -> Option<(Uuid, AppCommand)> {
        self.citation_review_page_command(CitationReviewRequest::default())
    }
    pub fn load_more_citation_review(&mut self) -> Option<(Uuid, AppCommand)> {
        if self.citation_review_loading() {
            return None;
        }
        let cursor = self.citation_review.page.as_ref()?.next_cursor.clone()?;
        self.citation_review_page_command(CitationReviewRequest {
            limit: 25,
            cursor: Some(cursor),
        })
    }
    fn citation_review_page_command(
        &mut self,
        request: CitationReviewRequest,
    ) -> Option<(Uuid, AppCommand)> {
        if !self.ready
            || !self.vault_bound
            || !self.citation_review.visible
            || self.needs_review_mode != NeedsReviewMode::CitationEvidence
            || request.validate().is_err()
        {
            return None;
        }
        let view = &mut self.citation_review;
        view.page_generation = view.page_generation.wrapping_add(1);
        view.selection = view.selection.wrapping_add(1);
        view.selected = None;
        view.detail = None;
        view.page = None;
        view.error = None;
        view.request = Some(request.clone());
        let capture = PageCapture {
            view: view.view,
            page: view.page_generation,
        };
        Some(self.command(
            Pending::CitationReview {
                capture,
                request: request.clone(),
            },
            AppCommand::CitationReview(request),
        ))
    }
    pub fn citation_review_row_capture(&self, entry: &CitationReviewEntry) -> Option<RowCapture> {
        let view = &self.citation_review;
        if !view.visible
            || self.needs_review_mode != NeedsReviewMode::CitationEvidence
            || !view.page.as_ref()?.entries.contains(entry)
        {
            return None;
        }
        Some(RowCapture {
            page: PageCapture {
                view: view.view,
                page: view.page_generation,
            },
            entry: entry.clone(),
        })
    }
    pub fn select_citation_review(&mut self, row: RowCapture) -> Option<(Uuid, AppCommand)> {
        if !self.ready || !self.vault_bound || !self.citation_row_matches(&row) {
            return None;
        }
        let request = CitationReviewDetailRequest {
            path: row.entry.path.clone(),
            expected_sha256: row.entry.sha256,
        };
        request.validate().ok()?;
        let view = &mut self.citation_review;
        view.selection = view.selection.wrapping_add(1);
        view.selected = Some(row.entry.clone());
        view.detail = None;
        view.error = None;
        let capture = SelectionCapture {
            row,
            selection: view.selection,
        };
        Some(self.command(
            Pending::CitationReviewDetail {
                capture,
                request: request.clone(),
            },
            AppCommand::CitationReviewDetail(request),
        ))
    }
    fn citation_page_matches(&self, capture: &PageCapture) -> bool {
        let view = &self.citation_review;
        view.visible
            && self.needs_review_mode == NeedsReviewMode::CitationEvidence
            && view.view == capture.view
            && view.page_generation == capture.page
    }
    fn citation_row_matches(&self, row: &RowCapture) -> bool {
        self.citation_page_matches(&row.page)
            && self
                .citation_review
                .page
                .as_ref()
                .is_some_and(|page| page.entries.contains(&row.entry))
    }
    fn citation_selection_matches(&self, capture: &SelectionCapture) -> bool {
        self.citation_row_matches(&capture.row)
            && self.citation_review.selection == capture.selection
            && self.citation_review.selected.as_ref() == Some(&capture.row.entry)
    }
    pub fn citation_review_loading(&self) -> bool {
        self.pending.values().any(|pending| matches!(pending, Pending::CitationReview { capture, request } if self.citation_page_matches(capture) && self.citation_review.request.as_ref() == Some(request)))
    }
    pub fn citation_review_detail_loading(&self) -> bool {
        self.pending.values().any(|pending| matches!(pending, Pending::CitationReviewDetail { capture, .. } if self.citation_selection_matches(capture)))
    }
    pub(super) fn received_citation_review(
        &mut self,
        id: Uuid,
        event: &AppEvent,
    ) -> Option<Vec<(Uuid, AppCommand)>> {
        let pending = self.pending.get(&id)?.clone();
        let current = match &pending {
            Pending::CitationReview { capture, request } => {
                self.citation_page_matches(capture)
                    && self.citation_review.request.as_ref() == Some(request)
            }
            Pending::CitationReviewDetail { capture, .. } => {
                self.citation_selection_matches(capture)
            }
            _ => return None,
        };
        if !current {
            self.pending.remove(&id);
            return Some(Vec::new());
        }
        match (&pending, event) {
            (Pending::CitationReview { request, .. }, AppEvent::CitationReview(page)) => match page
                .validate_for(request)
            {
                Ok(()) => {
                    self.citation_review.page = Some((**page).clone());
                    self.citation_review.error = None;
                }
                Err(_) => {
                    self.citation_review.error =
                        Some("Invalid citation evidence page. Refresh to inspect again.".into());
                }
            },
            (
                Pending::CitationReviewDetail { request, capture },
                AppEvent::CitationReviewDetail(detail),
            ) => {
                if detail.validate_for(request).is_ok()
                    && detail.note_id == capture.row.entry.note_id
                    && detail.title == capture.row.entry.title
                {
                    self.citation_review.detail = Some((**detail).clone());
                    self.citation_review.error = None;
                } else {
                    self.citation_review.error = Some(
                        "Invalid or changed citation evidence detail. Refresh to inspect again."
                            .into(),
                    );
                }
            }
            (_, AppEvent::Failed(error)) => {
                self.citation_review.error =
                    Some(format!("{}. Refresh to inspect again.", error.message));
            }
            // A mismatched event does not settle this exact operation.
            _ => return Some(Vec::new()),
        }
        self.pending.remove(&id);
        Some(Vec::new())
    }
}
