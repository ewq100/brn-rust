//! Explicit saved-profile query intent and acknowledgement correlation only.
use super::{AiState, Pending};
use brn_workflow::{
    app_worker::{AppCommand, AppEvent},
    knowledge::{ProfileContext, ProfileContextRequest, ProfileLens, note_identity, note_metadata},
    library::KnowledgeScope,
    proposals::{ProposalSource, SourceVersion},
};
use uuid::Uuid;

#[derive(Clone)]
pub struct ContextCapture {
    pub request: ProfileContextRequest,
    text: String,
    document: u64,
    editor_generation: u64,
    generation: u64,
}
#[derive(Default)]
pub struct ProfileContextView {
    pub context: Option<ProfileContext>,
    pub error: Option<String>,
    pub intent: Option<Uuid>,
    generation: u64,
}
impl AiState {
    pub fn clear_profile_context(&mut self) {
        self.profile_context.generation = self.profile_context.generation.wrapping_add(1);
        self.profile_context.context = None;
        self.profile_context.error = None;
        self.profile_context.intent = None;
    }
    fn profile_source(&self) -> Option<(SourceVersion, Uuid, &str)> {
        if !self.ready
            || !self.vault_bound
            || self.application_busy()
            || self.knowledge_scope != KnowledgeScope::Current
            || self.evidence.is_some()
            || self.pending.values().any(|pending| {
                matches!(
                    pending,
                    Pending::Bind
                        | Pending::Refresh
                        | Pending::Editor { .. }
                        | Pending::EditorReload
                        | Pending::EditorSave
                        | Pending::EditorReconcile
                )
            })
        {
            return None;
        }
        let editor = self.editor.as_ref()?;
        if !editor.can_leave() || editor.dirty() || !editor.can_save() {
            return None;
        }
        let text = editor.view.saved.as_ref()?;
        let metadata = note_metadata::classify(text).ok()?;
        let path = &editor.view.record.path;
        if metadata.source
            || metadata.history
            || path
                .split('/')
                .next()
                .is_some_and(|part| part.eq_ignore_ascii_case("archive"))
        {
            return None;
        }
        let note_id = note_identity::read(text).ok()??;
        let fingerprint = editor.view.observed.clone()?;
        if fingerprint.len != text.len() as u64 {
            return None;
        }
        let source = SourceVersion {
            path: path.clone(),
            fingerprint,
        };
        Some((source, note_id, text.as_str()))
    }
    pub fn profile_context_available(&self) -> bool {
        self.profile_source().is_some()
    }
    pub fn profile_context_loading(&self) -> bool {
        self.profile_context.intent.is_some_and(|id| {
            matches!(self.pending.get(&id), Some(Pending::ProfileContext(capture)) if self.context_capture_matches(id, capture))
        })
    }
    /// A new explicit lens/page supersedes the old intent, retaining its last valid view.
    pub fn inspect_profile_context(
        &mut self,
        lens: ProfileLens,
        action_offset: usize,
        relationship_offset: usize,
    ) -> Option<(Uuid, AppCommand)> {
        let (source, note_id, text) = self.profile_source()?;
        let profile = ProposalSource {
            source,
            text: text.to_owned(),
        };
        profile.validate().ok()?;
        let request = ProfileContextRequest {
            profile: profile.source,
            note_id,
            lens,
            action_offset,
            relationship_offset,
            limit: 25,
        };
        request.validate().ok()?;
        self.profile_context.generation = self.profile_context.generation.wrapping_add(1);
        self.profile_context.error = None;
        let capture = ContextCapture {
            request: request.clone(),
            text: profile.text,
            document: self.note_generation,
            editor_generation: self.editor.as_ref()?.generation,
            generation: self.profile_context.generation,
        };
        let command = self.command(
            Pending::ProfileContext(capture),
            AppCommand::ProfileContext(request),
        );
        self.profile_context.intent = Some(command.0);
        Some(command)
    }
    fn context_capture_matches(&self, id: Uuid, capture: &ContextCapture) -> bool {
        self.profile_context.intent == Some(id)
            && self.profile_context.generation == capture.generation
            && self.note_generation == capture.document
            && self
                .editor
                .as_ref()
                .is_some_and(|editor| editor.generation == capture.editor_generation)
            && self
                .profile_source()
                .is_some_and(|(source, note_id, text)| {
                    source == capture.request.profile
                        && note_id == capture.request.note_id
                        && text == capture.text
                })
    }
    /// Handle this query before generic event settlement: malformed or unrelated
    /// events cannot settle this intent (or any other pending operation).
    pub(super) fn apply_profile_context_event(&mut self, id: Uuid, event: &AppEvent) -> bool {
        let Some(Pending::ProfileContext(capture)) = self.pending.get(&id) else {
            return matches!(event, AppEvent::ProfileContext(_));
        };
        if !self.context_capture_matches(id, capture) {
            self.pending.remove(&id);
            return true;
        }
        match event {
            AppEvent::ProfileContext(context) => {
                if context.validate_for(&capture.request).is_err()
                    || context.profile.text != capture.text
                {
                    return true;
                }
                self.profile_context.context = Some((**context).clone());
                self.profile_context.error = None;
            }
            AppEvent::Failed(error) => self.profile_context.error = Some(error.message.clone()),
            _ => return true,
        }
        self.profile_context.intent = None;
        self.pending.remove(&id);
        true
    }
    pub fn profile_action_offset(&self, next: bool) -> Option<usize> {
        let context = self.profile_context.context.as_ref()?;
        context_page_offset(
            context.request.action_offset,
            context.request.limit,
            context.action_total,
            next,
        )
    }
    pub fn profile_relationship_offset(&self, next: bool) -> Option<usize> {
        let context = self.profile_context.context.as_ref()?;
        context_page_offset(
            context.request.relationship_offset,
            context.request.limit,
            context.relationship_total,
            next,
        )
    }
}
fn context_page_offset(offset: usize, limit: usize, total: usize, next: bool) -> Option<usize> {
    if next {
        offset.checked_add(limit).filter(|offset| *offset < total)
    } else {
        (offset > 0).then(|| offset.saturating_sub(limit))
    }
}
