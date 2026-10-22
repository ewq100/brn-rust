//! Note writing coordination and GPUI effects over the checked durable editor service.
use super::*;

#[derive(Default)]
pub struct NoteLifecycle {
    state: State,
}
#[derive(Default)]
enum State {
    #[default]
    Closed,
    Open(Box<OpenNote>),
}
#[derive(Clone, Copy)]
enum Presentation {
    Write,
    Read,
}
enum EditState {
    Clean,
    Editing(EditSession),
    BlockedElsewhere { note: String, base_version: u64 },
}
pub struct OpenNote {
    record: Record,
    editing: EditState,
    presentation: Presentation,
}
impl OpenNote {
    pub fn record(&self) -> &Record {
        &self.record
    }
    pub fn session(&self) -> Option<&EditSession> {
        match &self.editing {
            EditState::Editing(session) => Some(session),
            _ => None,
        }
    }
}
impl NoteLifecycle {
    pub fn as_ref(&self) -> Option<&OpenNote> {
        match &self.state {
            State::Closed => None,
            State::Open(open) => Some(open),
        }
    }
    pub fn is_some(&self) -> bool {
        self.as_ref().is_some()
    }
    pub fn is_none(&self) -> bool {
        self.as_ref().is_none()
    }
    pub fn is_reading(&self) -> bool {
        self.as_ref()
            .is_some_and(|open| matches!(open.presentation, Presentation::Read))
    }
    pub fn open_edit_elsewhere(&self) -> Option<(&str, u64)> {
        match &self.as_ref()?.editing {
            EditState::BlockedElsewhere { note, base_version } => Some((note, *base_version)),
            _ => None,
        }
    }
}
enum Load {
    Recover(Option<EditSession>),
    Current,
}
/// The confirmed discard's observable result. An ignored newer generation is retained.
pub enum NoteDiscard {
    Own,
    Other { adopted: bool },
    Newer,
}
impl WritingPanel {
    /// Load only after the host admits this Note's context. Recovery is resolved
    /// afresh from the checked store; a caller's old session is only a selection hint.
    pub fn open_note<T: 'static>(
        &mut self,
        workspace: &mut Workspace,
        id: &str,
        recovery: Option<EditSession>,
        window: &mut Window,
        cx: &mut Context<T>,
    ) -> AppResult<()> {
        // Explicit same-note Recover deliberately selects the authoritative
        // kept buffer over the local attempt. A rejected hint changes nothing.
        let recovering_current = recovery.is_some() && self.open_note_id() == Some(id);
        if !recovering_current
            && let Some((Tone::Warning, message)) =
                self.note_changed(workspace, self.note_editor.read(cx).value().to_string())?
        {
            return Err(AppError::Invalid(message.into()));
        }
        if !recovering_current
            && let Some(session) = self.note.as_ref().and_then(|open| open.session())
        {
            let current = workspace.store.edit_session(&session.id)?;
            if current.closed || current.generation != session.generation {
                return Err(AppError::Invalid(
                    "The recovery buffer changed or closed. Recover or compare it before replacing this visible edit."
                        .into(),
                ));
            }
        }
        self.load_note(workspace, id, Load::Recover(recovery), window, cx)
    }
    fn load_note<T: 'static>(
        &mut self,
        workspace: &Workspace,
        id: &str,
        load: Load,
        window: &mut Window,
        cx: &mut Context<T>,
    ) -> AppResult<()> {
        let record = workspace
            .store
            .note(id)?
            .ok_or_else(|| AppError::Invalid("That note is no longer current.".into()))?;
        let RecordData::Note(note) = &record.data else {
            return Err(AppError::Invalid("Not a Note".into()));
        };
        let session = match load {
            Load::Current => None,
            Load::Recover(recovery) => {
                let buffers = workspace.store.recovery_buffers()?;
                if recovery.as_ref().is_some_and(|hint| {
                    hint.note != id
                        || !buffers
                            .iter()
                            .any(|session| session.id == hint.id && session.note == id)
                }) {
                    return Err(AppError::Invalid(
                        "The kept edit changed. Refresh recovery before reopening it.".into(),
                    ));
                }
                buffers.into_iter().find(|session| session.note == id)
            }
        };
        let text = session
            .as_ref()
            .map_or_else(|| note.markdown.clone(), |session| session.markdown.clone());
        let current = note.markdown.clone();
        let presentation = if self.note.is_reading() {
            Presentation::Read
        } else {
            Presentation::Write
        };
        self.note.state = State::Open(Box::new(OpenNote {
            record,
            editing: session.map_or(EditState::Clean, EditState::Editing),
            presentation,
        }));
        self.selection = None;
        self.note_editor
            .update(cx, |editor, cx| editor.set_value(text, window, cx));
        self.current_view
            .update(cx, |view, cx| view.set_text(&current, cx));
        self.sync_read_text(cx);
        self.sync_note_marks(workspace, cx);
        Ok(())
    }
    /// Establish the durable first-dirty guard before any delayed/full-buffer
    /// work. Refused generations and errors preserve the editor and old base.
    pub fn note_changed(
        &mut self,
        workspace: &mut Workspace,
        text: String,
    ) -> AppResult<Option<(Tone, &'static str)>> {
        let State::Open(open) = &mut self.note.state else {
            return Ok(None);
        };
        let RecordData::Note(note) = &open.record.data else {
            return Ok(None);
        };
        if matches!(open.editing, EditState::Clean) && text == note.markdown {
            return Ok(None);
        }
        if !matches!(open.editing, EditState::Editing(_)) {
            let session = match workspace
                .store
                .begin_edit(&open.record.id, open.record.version)
            {
                Ok(session) => session,
                Err(error @ brn_threads_app::Error::EditSessionOpen { .. }) => {
                    if let brn_threads_app::Error::EditSessionOpen {
                        note, base_version, ..
                    } = &error
                    {
                        open.editing = EditState::BlockedElsewhere {
                            note: note.clone(),
                            base_version: *base_version,
                        };
                    }
                    return Err(AppError::Core(error));
                }
                Err(error) => return Err(AppError::Core(error)),
            };
            open.editing = EditState::Editing(session);
        }
        let EditState::Editing(session) = &open.editing else {
            unreachable!()
        };
        if session.markdown == text {
            return Ok(None);
        }
        Ok(Some(
            match workspace
                .store
                .update_buffer(&session.id, session.generation + 1, &text)?
            {
                BufferOutcome::Updated(updated) => {
                    open.editing = EditState::Editing(updated);
                    (
                        Tone::Info,
                        "Your edit is recoverable. Save to update the note.",
                    )
                }
                BufferOutcome::Ignored(_) => (
                    Tone::Warning,
                    "This recovery buffer changed elsewhere. Your visible text is retained; compare before saving.",
                ),
            },
        ))
    }
    pub fn set_note_reading<T: 'static>(
        &mut self,
        workspace: &mut Workspace,
        reading: bool,
        cx: &mut Context<T>,
    ) -> AppResult<bool> {
        if self.review.is_some() || self.note.is_none() {
            return Ok(false);
        }
        if self.note.open_edit_elsewhere().is_none() {
            self.note_changed(workspace, self.note_editor.read(cx).value().to_string())?;
        }
        if let State::Open(open) = &mut self.note.state {
            open.presentation = if reading {
                Presentation::Read
            } else {
                Presentation::Write
            };
        }
        self.sync_read_text(cx);
        self.selection = None;
        Ok(true)
    }
    /// Save the exact visible generation. Rejected Save keeps its session/base
    /// and text; only the saved-current comparison is refreshed.
    pub fn save_note<T: 'static>(
        &mut self,
        workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<T>,
    ) -> AppResult<Option<SaveOutcome>> {
        self.note_changed(workspace, self.note_editor.read(cx).value().to_string())?;
        let Some(session) = self.note.as_ref().and_then(|open| open.session()).cloned() else {
            return Ok(None);
        };
        let outcome = workspace.store.save(&session.id, session.generation)?;
        if matches!(outcome, SaveOutcome::Saved(_)) {
            self.load_note(workspace, &session.note, Load::Current, window, cx)?;
        } else {
            self.sync_saved_comparison(workspace, &session.note, cx);
        }
        Ok(Some(outcome))
    }
    fn sync_saved_comparison<T: 'static>(
        &mut self,
        workspace: &mut Workspace,
        id: &str,
        cx: &mut Context<T>,
    ) {
        if let Ok(Some(record)) = workspace.store.note(id)
            && let RecordData::Note(note) = record.data
        {
            self.current_view
                .update(cx, |view, cx| view.set_text(&note.markdown, cx));
        }
    }
    /// After an external checked change, preserve every owned/blocked edit.
    /// A refresh cannot silently adopt the current version as a dirty base.
    pub fn refresh_note<T: 'static>(
        &mut self,
        workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<T>,
    ) -> AppResult<()> {
        if self.note.open_edit_elsewhere().is_some() {
            return Ok(());
        }
        self.note_changed(workspace, self.note_editor.read(cx).value().to_string())?;
        let State::Open(open) = &self.note.state else {
            return Ok(());
        };
        if !matches!(open.editing, EditState::Clean) {
            return Ok(());
        }
        let id = open.record.id.clone();
        self.load_note(workspace, &id, Load::Current, window, cx)
    }
    /// Host confirmation must precede this operation. Store generation checks
    /// decide whether the selected edit actually closed.
    pub fn discard_note_session<T: 'static>(
        &mut self,
        workspace: &mut Workspace,
        session: &EditSession,
        window: &mut Window,
        cx: &mut Context<T>,
    ) -> AppResult<NoteDiscard> {
        let own = self
            .note
            .as_ref()
            .and_then(|open| open.session())
            .is_some_and(|open| open.id == session.id);
        match workspace.store.discard(&session.id, session.generation)? {
            BufferOutcome::Updated(closed) if closed.closed && own => {
                self.load_note(workspace, &session.note, Load::Current, window, cx)?;
                Ok(NoteDiscard::Own)
            }
            BufferOutcome::Updated(closed) if closed.closed => {
                if let State::Open(open) = &mut self.note.state
                    && matches!(open.editing, EditState::BlockedElsewhere { .. })
                {
                    open.editing = EditState::Clean;
                }
                self.note_changed(workspace, self.note_editor.read(cx).value().to_string())?;
                let adopted = self
                    .note
                    .as_ref()
                    .is_some_and(|open| open.session().is_some());
                Ok(NoteDiscard::Other { adopted })
            }
            _ => Ok(NoteDiscard::Newer),
        }
    }
}
