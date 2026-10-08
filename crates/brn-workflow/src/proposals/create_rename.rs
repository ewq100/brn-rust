//! Owner-selected destination correction over the existing exact review path.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateRenameRequest {
    pub expected: ProposalStamp,
    pub change_index: usize,
    pub path: String,
}

impl CreateRenameRequest {
    pub fn validate(&self) -> Result<()> {
        if self.expected.id.is_nil()
            || self.expected.version == 0
            || self.change_index >= MAX_PROPOSAL_CHANGES
            || self.path.len() > 4096
        {
            return Err(invalid(
                "new-note rename needs an exact review stamp, member and bounded path",
            ));
        }
        path_check(&self.path)
    }
}

/// Complete acknowledgement validation, including the exact no-op response.
pub fn validate_create_rename_transition(
    before: &ProposalRecord,
    after: &ProposalRecord,
    change_index: usize,
    path: &str,
) -> Result<()> {
    Ok(
        brn_store::work::proposals::validate_create_rename_transition(
            before,
            after,
            change_index,
            path,
        )?,
    )
}

fn stale(message: &str) -> WorkflowError {
    WorkflowError::typed(ErrorKind::ContextStale, message)
}

impl App {
    /// Revises one absent new-note destination; it never writes a vault file.
    pub fn rename_proposal_create(
        &mut self,
        request: &CreateRenameRequest,
    ) -> Result<ProposalRecord> {
        request.validate()?;
        let record = self.proposal(request.expected.id)?;
        if record.stamp() != request.expected || record.state != ProposalState::Draft {
            return Err(stale("new-note rename review version or state changed"));
        }
        if record.draft.intake.is_some()
            || record.draft.inbox_source.is_some()
            || record.draft.inbox_visual.is_some()
        {
            return Err(invalid(
                "bound Source and visual destinations cannot be renamed",
            ));
        }
        let Some(NoteChange::Create { path, parent, .. }) =
            record.draft.changes.get(request.change_index)
        else {
            return Err(invalid("new-note rename needs a Markdown Create member"));
        };
        if Path::new(path).parent() != Path::new(&request.path).parent() {
            return Err(invalid("new-note rename must stay in its captured folder"));
        }
        if path == &request.path {
            return Ok(self.store.rename_proposal_create(
                request.expected,
                request.change_index,
                &request.path,
            )?);
        }
        self.require_current_evidence()?;
        self.editor_files()?;
        self.validate_intake_dependency(
            crate::intake_dependencies::dependency(&record.draft),
            false,
        )?;
        self.validate_inbox_knowledge(record.draft.inbox_knowledge.as_deref())?;
        let files = self.editor_files()?;
        let destination = Path::new(&request.path);
        for (index, change) in record.draft.changes.iter().enumerate() {
            if index != request.change_index
                && files
                    .reserved_copy_path_matches(destination, Path::new(change.path()))
                    .map_err(file_error)?
            {
                return Err(stale("proposal destinations alias the same namespace"));
            }
        }
        files
            .coordinate(destination, || {
                Ok((|| -> Result<()> {
                    if files.parent_identity(destination).map_err(file_error)? != *parent {
                        return Err(stale("reviewed proposal parent changed"));
                    }
                    files
                        .validate_copy_destination(destination)
                        .map_err(file_error)?;
                    if files.artifact(destination).map_err(file_error)?.is_some() {
                        return Err(stale("proposed creation destination is occupied"));
                    }
                    Ok(())
                })())
            })
            .map_err(file_error)??;
        let mut candidate = record.draft.clone();
        let NoteChange::Create { path, .. } = &mut candidate.changes[request.change_index] else {
            unreachable!("checked Create");
        };
        *path = request.path.clone();
        self.preflight_proposal_targets(&candidate, None)?;
        // A private Knowledge binding already validates its exact protected
        // citations against retained extraction bytes. Preparation may precede
        // Source approval; the ordinary saved-citation validator belongs to
        // fresh approval once that prerequisite is Applied. No text or citation
        // changes are introduced by this same-folder destination revision.
        if candidate
            .inbox_knowledge
            .as_ref()
            .is_none_or(|binding| binding.intake.is_none())
        {
            self.validate_proposal_provenance(&candidate)?;
        }
        self.validate_proposal_links(&candidate)?;
        self.validate_action_references(&candidate)?;
        let files = self.editor_files()?;
        for source in &candidate.sources {
            if files
                .observe(Path::new(&source.path))
                .map_err(file_error)?
                .fingerprint
                != source.fingerprint
            {
                return Err(stale("reviewed proposal source changed"));
            }
        }
        Ok(self.store.rename_proposal_create(
            request.expected,
            request.change_index,
            &request.path,
        )?)
    }
}
