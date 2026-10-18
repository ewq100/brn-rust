//! One transient owner for exact-candidate comparison and editable final text.
//! Workspace remains the authority for preparation, Apply, versions and dirty guards.
use super::*;

#[derive(Default)]
pub struct ReviewLifecycle {
    state: State,
}
#[derive(Default)]
enum State {
    #[default]
    Closed,
    Open {
        review: Review,
        mode: Mode,
        inputs: bool,
    },
}
#[derive(Clone, Copy)]
enum Mode {
    Changes,
    Read,
    Editing { dirty: bool, return_to: Comparison },
}
#[derive(Clone, Copy)]
enum Comparison {
    Changes,
    Read,
}
impl ReviewLifecycle {
    pub fn as_ref(&self) -> Option<&Review> {
        match &self.state {
            State::Closed => None,
            State::Open { review, .. } => Some(review),
        }
    }
    pub fn is_some(&self) -> bool {
        self.as_ref().is_some()
    }
    pub fn is_none(&self) -> bool {
        self.as_ref().is_none()
    }
    pub fn is_editing(&self) -> bool {
        matches!(
            self.state,
            State::Open {
                mode: Mode::Editing { .. },
                ..
            }
        )
    }
    pub fn is_edited(&self) -> bool {
        matches!(
            self.state,
            State::Open {
                mode: Mode::Editing { dirty: true, .. },
                ..
            }
        )
    }
    pub fn is_reading(&self) -> bool {
        matches!(
            self.state,
            State::Open {
                mode: Mode::Read,
                ..
            }
        )
    }
    pub fn inputs_shown(&self) -> bool {
        matches!(self.state, State::Open { inputs: true, .. })
    }
    pub fn toggle_inputs(&mut self) {
        if let State::Open { inputs, .. } = &mut self.state {
            *inputs = !*inputs;
        }
    }
    pub fn begin_edit(&mut self) -> bool {
        match &mut self.state {
            State::Open { review, mode, .. } if review.note.is_some() => {
                if !matches!(mode, Mode::Editing { .. }) {
                    let return_to = if matches!(mode, Mode::Read) {
                        Comparison::Read
                    } else {
                        Comparison::Changes
                    };
                    *mode = Mode::Editing {
                        dirty: false,
                        return_to,
                    };
                }
                true
            }
            _ => false,
        }
    }
    /// Called by the real GPUI input subscription. Text and IME stay in GPUI.
    pub fn buffer_changed(&mut self, text: &str) {
        if let State::Open {
            review,
            mode: Mode::Editing { dirty, .. },
            ..
        } = &mut self.state
        {
            *dirty = text != review.original;
        }
    }
    pub fn set_reading(&mut self, reading: bool) -> bool {
        match &mut self.state {
            State::Open { review, mode, .. }
                if review.note.is_some() && !matches!(mode, Mode::Editing { .. }) =>
            {
                *mode = if reading { Mode::Read } else { Mode::Changes };
                true
            }
            _ => false,
        }
    }
    pub fn toggle_group(&mut self, id: &str) {
        if let State::Open { review, mode, .. } = &mut self.state
            && !matches!(mode, Mode::Editing { .. })
            && review.groups.groups.iter().any(|group| group.id == id)
            && !review.selected.remove(id)
        {
            review.selected.insert(id.into());
        }
    }
    /// No dirty draft or partial selection can authorize the old candidate.
    pub fn exact_candidate(&self) -> AppResult<Prepared> {
        let review = self
            .as_ref()
            .ok_or_else(|| AppError::Invalid("Open the exact proposal first".into()))?;
        if self.is_edited() || review.selected.len() != review.groups.groups.len() {
            return Err(AppError::Invalid(
                "Prepare your edited text or selected groups before applying it.".into(),
            ));
        }
        Ok(review.candidate.clone())
    }
    /// A card for the candidate under review shares its draft/selection guard.
    /// Other candidate cards retain their own exact displayed result.
    pub fn candidate_to_apply(&self, candidate: Prepared) -> AppResult<Prepared> {
        if self
            .as_ref()
            .is_some_and(|review| review.candidate.operation == candidate.operation)
        {
            self.exact_candidate()
        } else {
            Ok(candidate)
        }
    }
    /// Keeping a dirty draft requires preparing it first. Failure retains it.
    pub fn keep(&mut self) -> AppResult<()> {
        if self.is_edited() {
            return Err(AppError::Invalid(
                "Prepare your edited text before leaving this proposal.".into(),
            ));
        }
        self.state = State::Closed;
        Ok(())
    }
    /// An asynchronous/durable completion only closes the candidate it names.
    pub fn completed(&mut self, operation: &str) {
        if self
            .as_ref()
            .is_some_and(|review| review.candidate.operation.as_str() == operation)
        {
            self.state = State::Closed;
        }
    }
}
impl WritingPanel {
    /// Resolve saved inputs before replacing transient state. Reopening the same
    /// immutable candidate preserves its draft, mode and selection.
    pub fn open_review<T: 'static>(
        &mut self,
        workspace: &Workspace,
        candidate: Prepared,
        note: Option<String>,
        window: &mut Window,
        cx: &mut Context<T>,
    ) -> AppResult<()> {
        if self
            .review
            .as_ref()
            .is_some_and(|review| review.candidate.operation == candidate.operation)
        {
            return Ok(());
        }
        if self.review.is_edited() {
            return Err(AppError::Invalid(
                "Prepare your edited text before opening another proposal.".into(),
            ));
        }
        let groups = workspace.review_groups(candidate.operation.as_str())?;
        let note = note
            .filter(|id| {
                candidate
                    .request
                    .writes
                    .iter()
                    .any(|write| &write.id == id && matches!(write.data, RecordData::Note(_)))
            })
            .or_else(|| {
                candidate
                    .request
                    .writes
                    .iter()
                    .find(|write| matches!(write.data, RecordData::Note(_)))
                    .map(|write| write.id.clone())
            });
        let original = note
            .as_ref()
            .and_then(|id| {
                candidate
                    .request
                    .writes
                    .iter()
                    .find_map(|write| match &write.data {
                        RecordData::Note(note) if &write.id == id => Some(note.markdown.clone()),
                        _ => None,
                    })
            })
            .unwrap_or_default();
        let selected = groups.groups.iter().map(|group| group.id.clone()).collect();
        self.review.state = State::Open {
            review: Review {
                candidate,
                note,
                original,
                groups,
                selected,
            },
            mode: Mode::Changes,
            inputs: false,
        };
        self.sync_review_document(workspace, window, cx);
        Ok(())
    }
    fn sync_review_document<T: 'static>(
        &mut self,
        workspace: &Workspace,
        window: &mut Window,
        cx: &mut Context<T>,
    ) {
        let Some(review) = self.review.as_ref() else {
            return;
        };
        let text = review.original.clone();
        let current = review
            .note
            .as_ref()
            .and_then(|id| workspace.store.note(id).ok().flatten())
            .and_then(|record| match record.data {
                RecordData::Note(note) => Some(note.markdown),
                _ => None,
            })
            .unwrap_or_default();
        self.review_editor
            .update(cx, |editor, cx| editor.set_value(text.clone(), window, cx));
        self.current_view
            .update(cx, |view, cx| view.set_text(&current, cx));
        self.candidate_view
            .update(cx, |view, cx| view.set_text(&text, cx));
        self.sync_diff(workspace, window, cx);
    }
    pub fn cancel_review_edit<T: 'static>(&mut self, window: &mut Window, cx: &mut Context<T>) {
        if let State::Open { review, mode, .. } = &mut self.review.state {
            // Change state before set_value can emit an InputEvent.
            *mode = match *mode {
                Mode::Editing {
                    return_to: Comparison::Read,
                    ..
                } => Mode::Read,
                Mode::Editing {
                    return_to: Comparison::Changes,
                    ..
                } => Mode::Changes,
                other => other,
            };
            let text = review.original.clone();
            self.review_editor
                .update(cx, |editor, cx| editor.set_value(text, window, cx));
        }
    }
    pub fn select_review_note<T: 'static>(
        &mut self,
        workspace: &Workspace,
        id: &str,
        window: &mut Window,
        cx: &mut Context<T>,
    ) -> AppResult<()> {
        if self.review.is_edited() {
            return Err(AppError::Invalid(
                "Prepare your edited text before switching notes.".into(),
            ));
        }
        if let State::Open { review, mode, .. } = &mut self.review.state {
            let Some(text) = review
                .candidate
                .request
                .writes
                .iter()
                .find_map(|write| match &write.data {
                    RecordData::Note(note) if write.id == id => Some(note.markdown.clone()),
                    _ => None,
                })
            else {
                return Ok(());
            };
            review.note = Some(id.into());
            review.original = text;
            *mode = Mode::Changes;
            self.sync_review_document(workspace, window, cx);
        }
        Ok(())
    }
    /// Preparing creates another immutable candidate; only its newly displayed
    /// result becomes eligible for Apply. On failure the original draft survives.
    pub fn prepare_review_edit<T: 'static>(
        &mut self,
        workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<T>,
    ) -> AppResult<()> {
        let Some(review) = self.review.as_ref() else {
            return Ok(());
        };
        let Some(note) = &review.note else {
            return Ok(());
        };
        let text = self.review_editor.read(cx).value().to_string();
        let candidate =
            workspace.review_replace(review.candidate.operation.as_str(), note, &text)?;
        let note = Some(note.clone());
        self.replace_review(workspace, candidate, note, window, cx)
    }
    pub fn prepare_review_selection<T: 'static>(
        &mut self,
        workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<T>,
    ) -> AppResult<()> {
        if self.review.is_edited() {
            return Err(AppError::Invalid(
                "Prepare your edited text before choosing proposal groups.".into(),
            ));
        }
        let Some(review) = self.review.as_ref() else {
            return Ok(());
        };
        let operation = review.candidate.operation.as_str().to_owned();
        let note = review.note.clone();
        let selected = review.selected.iter().cloned().collect::<Vec<_>>();
        let candidate = workspace.review_select(&operation, &selected)?;
        self.replace_review(workspace, candidate, note, window, cx)
    }
    fn replace_review<T: 'static>(
        &mut self,
        workspace: &Workspace,
        candidate: Prepared,
        note: Option<String>,
        window: &mut Window,
        cx: &mut Context<T>,
    ) -> AppResult<()> {
        // Preserve the draft if loading the newly prepared candidate fails.
        let previous = std::mem::take(&mut self.review);
        if let Err(error) = self.open_review(workspace, candidate, note, window, cx) {
            self.review = previous;
            return Err(error);
        }
        Ok(())
    }
}
