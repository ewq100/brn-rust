//! Scoped owner configuration over CW1. This draft never grants access or changes a Note.
use super::{UI_FONT, UI_TEXT, WritingRequest, WritingView, metadata, pill, subheading};
use crate::theme::pal;
use brn_threads_app::*;
use gpui_kit::base::{Disableable, Selectable};
use gpui_kit::component::Sizable;
use gpui_kit::component::input::{Input, Textarea};
use gpui_kit::{Context, TestSupportExt, div, prelude::*, px, rgb};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetupTarget {
    pub note: String,
    pub note_version: u64,
    pub scope: Option<String>,
    pub setup_version: Option<u64>,
}
impl SetupTarget {
    pub fn resolve(
        workspace: &Workspace,
        note: &str,
        scope: Option<&str>,
    ) -> AppResult<(Self, WritingSetupView)> {
        let context = workspace.active_writing_context(note, scope)?;
        if context.scope.as_deref() != scope {
            return Err(AppError::Invalid(
                "Writing context is no longer displayed here".into(),
            ));
        }
        // This service checks the working Note and reference availability in that exact scope.
        let view = workspace.writing_setup(note, context.scope.as_deref())?;
        let saved = workspace
            .store
            .note(note)?
            .ok_or_else(|| AppError::Invalid("Working Note is unavailable".into()))?;
        Ok((
            Self {
                note: note.into(),
                note_version: saved.version,
                scope: context.scope,
                setup_version: view.record.as_ref().map(|r| r.version),
            },
            view,
        ))
    }
    pub fn validate(
        &self,
        workspace: &Workspace,
        displayed_note: Option<&str>,
        displayed_scope: Option<&str>,
    ) -> AppResult<WritingSetupView> {
        if displayed_note != Some(self.note.as_str()) || displayed_scope != self.scope.as_deref() {
            return Err(AppError::Invalid(
                "Writing target changed; nothing was attached or saved to another context".into(),
            ));
        }
        let (current, view) = Self::resolve(workspace, &self.note, displayed_scope)?;
        if current != *self {
            return Err(AppError::Invalid(
                "Writing Note or setup changed; Reload brief & references before saving".into(),
            ));
        }
        Ok(view)
    }
}

#[derive(Clone)]
pub struct ReferenceLabel {
    pub title: String,
    pub version: u64,
    pub import_coverage: Option<ImportCoverage>,
}
/// One fallible, explicitly loaded form. A canonical token is display invalidation only.
/// Local buffer writes do not invalidate it; any canonical change hides it until Reload.
pub struct SetupDraft {
    pub target: SetupTarget,
    pub observed: ViewChangeToken,
    pub saved: WritingSetup,
    pub references: Vec<WritingReference>,
    pub labels: BTreeMap<String, ReferenceLabel>,
    pub matches: Vec<brn_threads_app::search::NoteMatch>,
    pub next: Option<String>,
    pub match_total: usize,
    pub search_pending: bool,
    pub must_be_current: BTreeSet<String>,
}
impl SetupDraft {
    pub fn load(workspace: &Workspace, note: &str, scope: Option<&str>) -> AppResult<Self> {
        let observed = workspace.store.change_token()?;
        let (target, view) = SetupTarget::resolve(workspace, note, scope)?;
        let saved = view
            .record
            .as_ref()
            .and_then(|r| match &r.data {
                RecordData::WritingSetup(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or(WritingSetup {
                note: note.into(),
                chat: target.scope.clone(),
                brief: None,
                references: vec![],
            });
        let mut labels = BTreeMap::new();
        for state in &view.references {
            // Unavailable IDs never cause an unscoped title/content lookup.
            if let Some(version) = state.version {
                let label = checked_label(workspace, &state.reference.note, version)?;
                labels.insert(state.reference.note.clone(), label);
            }
        }
        if workspace.store.change_token()? != observed {
            return Err(AppError::Unavailable(
                "Writing setup changed during inspection; Reload".into(),
            ));
        }
        Ok(Self {
            target,
            observed,
            references: saved.references.clone(),
            saved,
            labels,
            matches: vec![],
            next: None,
            match_total: 0,
            search_pending: false,
            must_be_current: BTreeSet::new(),
        })
    }
    pub fn displayed(
        &self,
        workspace: &Workspace,
        note: Option<&str>,
        scope: Option<&str>,
    ) -> bool {
        note == Some(self.target.note.as_str())
            && scope == self.target.scope.as_deref()
            && workspace
                .store
                .change_token()
                .is_ok_and(|token| token == self.observed)
    }
    pub fn value(&self, brief: &str) -> WritingSetup {
        WritingSetup {
            note: self.target.note.clone(),
            chat: self.target.scope.clone(),
            brief: (!brief.trim().is_empty()).then(|| brief.to_owned()),
            references: self.references.clone(),
        }
    }
    pub fn save(
        &self,
        workspace: &mut Workspace,
        key: &str,
        note: Option<&str>,
        scope: Option<&str>,
        brief: &str,
    ) -> AppResult<Option<Receipt>> {
        self.target.validate(workspace, note, scope)?;
        let value = self.value(brief);
        if value == self.saved {
            return Ok(None);
        }
        workspace
            .set_writing_setup(key, self.target.setup_version, &value)
            .map(Some)
    }
    /// Only a still-current scoped search result can enter the pending reference list.
    pub fn attach_match(
        &mut self,
        workspace: &Workspace,
        id: &str,
        purpose: WritingPurpose,
    ) -> AppResult<()> {
        if !self.displayed(
            workspace,
            Some(&self.target.note),
            self.target.scope.as_deref(),
        ) {
            return Err(AppError::Invalid(
                "Reference search changed; Reload before attaching".into(),
            ));
        }
        let hit = self
            .matches
            .iter()
            .find(|hit| hit.note == id && !hit.archived && hit.note != self.target.note)
            .ok_or_else(|| {
                AppError::Invalid("Choose an available Note from this scoped search".into())
            })?;
        let label = checked_label(workspace, id, hit.version)?;
        if workspace.store.change_token()? != self.observed {
            return Err(AppError::Unavailable(
                "Reference changed during inspection; Reload".into(),
            ));
        }
        if !self.references.iter().any(|r| r.note == id) {
            self.references.push(WritingReference {
                note: id.into(),
                purpose,
            });
        }
        self.labels.insert(id.into(), label);
        Ok(())
    }
}
fn checked_label(workspace: &Workspace, id: &str, version: u64) -> AppResult<ReferenceLabel> {
    // Only reached with an existing scoped service's exact usable version; the surrounding token fences the lookup.
    let record = workspace
        .store
        .note(id)?
        .filter(|r| !r.archived && r.version == version)
        .ok_or_else(|| {
            AppError::Unavailable("Reference changed; Reload before inspecting".into())
        })?;
    let RecordData::Note(note) = record.data else {
        return Err(AppError::Invalid("Reference is not a Note".into()));
    };
    Ok(ReferenceLabel {
        title: note.title,
        version,
        import_coverage: note.import,
    })
}
pub(crate) fn purpose_label(purpose: WritingPurpose) -> &'static str {
    match purpose {
        WritingPurpose::Requirements => "Requirements",
        WritingPurpose::Evidence => "Evidence",
        WritingPurpose::Style => "Style",
    }
}
pub(crate) fn import_label(coverage: Option<&ImportCoverage>) -> String {
    match coverage {
        None => "BRN Note; no original-import coverage asserted".into(),
        Some(c) => format!(
            "Original import: {}{}",
            c.report
                .as_ref()
                .map_or("Coverage unrecorded / unchecked", ImportReport::summary),
            if c.gaps.is_empty() {
                ""
            } else {
                " · retained gaps; partial original coverage"
            }
        ),
    }
}
impl<T: 'static> WritingView<'_, T> {
    pub fn setup_drawer(&self, cx: &mut Context<T>) -> gpui_kit::AnyElement {
        if !self.setup_open {
            return div().into_any_element();
        }
        let p = pal();
        let mut panel = div()
            .id("writing-setup")
            .flex()
            .flex_col()
            .gap_2()
            .px_3()
            .py_2()
            .flex_shrink(1.)
            .min_h(px(44.))
            .max_h(px(300.))
            .overflow_y_scroll()
            .border_b_1()
            .border_color(rgb(p.hairline))
            .font_family(UI_FONT)
            .text_size(UI_TEXT)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(subheading("Brief & references"))
                    .child(
                        pill("reload-writing-setup", "Reload")
                            .on_click(self.on(cx, WritingRequest::OpenSetup)),
                    )
                    .child(
                        pill("close-writing-setup", "Close")
                            .on_click(self.on(cx, WritingRequest::CloseSetup)),
                    ),
            );
        let Some(draft) = self.setup.as_ref().filter(|draft| {
            draft.displayed(self.workspace, self.open_note_id(), self.displayed_scope)
        }) else {
            return panel.child(metadata("Context or saved setup changed. Reload to inspect this context; unsaved form content is not shown in another scope."))
                .test_support().into_any_element();
        };
        if self.reference_importing {
            panel = panel.child(metadata(self.progress.to_owned())).child(
                pill("stop-writing-reference-import", "Stop import")
                    .on_click(self.on(cx, WritingRequest::Cancel)),
            );
        }
        panel = panel.child(metadata("Optional brief: purpose, audience, tone, requirements. References are read-only; their purpose never grants permission."))
            .child(Textarea::new(&self.setup_brief).small().aria_label("Optional writing brief: purpose, audience, tone, requirements"))
            .child(metadata("Save brief & references before rewriting or importing. Attached does not mean read."));
        for (index, reference) in draft.references.iter().enumerate() {
            let saved = draft.saved.references.contains(reference);
            let label = draft.labels.get(&reference.note);
            let status = label.map_or_else(
                || "No longer available here".to_string(),
                |r| {
                    format!(
                        "{} · v{} · Available · {}",
                        r.title,
                        r.version,
                        import_label(r.import_coverage.as_ref())
                    )
                },
            );
            let mut row = div()
                .id(format!("writing-reference-{index}"))
                .flex()
                .flex_col()
                .gap_1()
                .child(metadata(if saved { "Attached" } else { "Pending Save" }))
                .child(
                    div()
                        .id(format!("writing-reference-availability-{index}"))
                        .aria_label(status.clone())
                        .child(status)
                        .test_support(),
                );
            let mut purposes = div().flex().flex_wrap().gap_1();
            for purpose in [
                WritingPurpose::Requirements,
                WritingPurpose::Evidence,
                WritingPurpose::Style,
            ] {
                purposes = purposes.child(
                    pill(
                        format!("reference-purpose-{index}-{}", purpose_label(purpose)),
                        purpose_label(purpose),
                    )
                    .selected(reference.purpose == purpose)
                    .on_click(self.on(
                        cx,
                        WritingRequest::ReferencePurpose {
                            note: reference.note.clone(),
                            purpose,
                        },
                    )),
                );
            }
            row = row
                .child(purposes)
                .child(
                    pill(
                        format!("reference-current-{index}"),
                        "Must be current for next configured rewrite",
                    )
                    .selected(draft.must_be_current.contains(&reference.note))
                    .disabled(label.is_none())
                    .on_click(
                        self.on(cx, WritingRequest::ReferenceCurrent(reference.note.clone())),
                    ),
                )
                .child(
                    pill(format!("remove-reference-{index}"), "Remove reference").on_click(
                        self.on(cx, WritingRequest::RemoveReference(reference.note.clone())),
                    ),
                );
            if label.is_some() {
                row = row.child(
                    pill(
                        format!("open-writing-reference-{index}"),
                        "Open reference as Note",
                    )
                    .on_click(self.on(cx, WritingRequest::OpenReference(reference.note.clone()))),
                );
            }
            panel = panel.child(row);
        }
        for removed in draft
            .saved
            .references
            .iter()
            .filter(|old| !draft.references.iter().any(|r| r.note == old.note))
        {
            // Only already-checked labels are used; unavailable sources remain unnamed.
            panel = panel.child(metadata(format!(
                "Removed, pending Save: {}",
                draft
                    .labels
                    .get(&removed.note)
                    .map_or("Unavailable reference", |r| r.title.as_str())
            )));
        }
        panel = panel
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .child(
                        pill("save-writing-setup", "Save brief & references")
                            .on_click(self.on(cx, WritingRequest::SaveSetup)),
                    )
                    .child(
                        pill("import-writing-reference", "Import file as reference…").on_click(
                            self.on(
                                cx,
                                WritingRequest::ImportReference(WritingPurpose::Evidence),
                            ),
                        ),
                    )
                    .child(
                        pill("rewrite-with-references", "Rewrite with selected freshness")
                            .on_click(self.on(cx, WritingRequest::RewriteConfigured(false))),
                    )
                    .child(
                        pill("check-writing-requirements", "Check all requirements")
                            .on_click(self.on(cx, WritingRequest::RewriteConfigured(true))),
                    ),
            )
            .child(
                Input::new(&self.reference_query)
                    .small()
                    .aria_label("Find an existing reference Note in this writing scope"),
            )
            .child(
                pill("find-writing-reference", "Find existing Note")
                    .on_click(self.on(cx, WritingRequest::SearchReferences(None))),
            );
        if draft.search_pending {
            panel = panel.child(metadata("Searching this writing scope…"));
        }
        for (index, hit) in draft
            .matches
            .iter()
            .enumerate()
            .filter(|(_, hit)| !hit.archived && hit.note != draft.target.note)
        {
            let id = hit.note.clone();
            panel = panel.child(
                pill(
                    format!("attach-writing-reference-{index}"),
                    format!("Attach {} · v{}", hit.title, hit.version),
                )
                .on_click(self.on(
                    cx,
                    WritingRequest::AttachReference {
                        note: id,
                        purpose: WritingPurpose::Evidence,
                    },
                )),
            );
        }
        if let Some(next) = &draft.next {
            panel = panel.child(
                pill("next-writing-references", "Next reference results")
                    .on_click(self.on(cx, WritingRequest::SearchReferences(Some(next.clone())))),
            );
        }
        panel.child(metadata(format!("{} available search matches; each page is bounded. Search matches are not recorded reading.", draft.match_total)))
            .test_support().into_any_element()
    }
}
