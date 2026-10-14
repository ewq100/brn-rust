//! Shared Projects use cases for native, CLI and host integrations.
use crate::*;
impl Workspace {
    pub fn projects(&self, include_trash: bool) -> AppResult<Vec<Record>> {
        Ok(self
            .records()?
            .into_iter()
            .filter(|r| (include_trash || !r.archived) && matches!(r.data, RecordData::Project(_)))
            .collect())
    }
    pub fn project_command(&mut self, key: &str, command: ProjectCommand) -> AppResult<Receipt> {
        let binding = format!("{:x}", Sha256::digest(serde_json::to_vec(&command)?));
        let op = self
            .store
            .allocate_bound_operation(&format!("project:{key}"), &binding)?;
        Ok(self.store.project_command(
            &op,
            &command,
            &HostAuthority::owner("owner project action"),
        )?)
    }
    /// New writing conversations inherit an explicitly selected linked project.
    pub fn new_document_chat(
        &mut self,
        title: &str,
        note: &str,
        preferred: Option<&str>,
    ) -> AppResult<Record> {
        let rows = self.records()?;
        let selected = preferred
            .and_then(|id| {
                rows.iter().find(|r| {
                    r.id == id
                        && !r.archived
                        && matches!(r.data, RecordData::Project(_))
                        && project_document_ids(&rows, r).contains(note)
                })
            })
            .or_else(|| {
                if !rows.iter().any(|r| {
                    r.id == note && matches!(&r.data, RecordData::Note(n) if n.chat.is_some())
                }) {
                    return None;
                }
                rows.iter().find(|r| {
                    !r.archived
                        && matches!(r.data, RecordData::Project(_))
                        && project_document_ids(&rows, r).contains(note)
                })
            });
        if let Some(p) = selected {
            let receipt = self.project_command(
                &uuid::Uuid::new_v4().to_string(),
                ProjectCommand::CreateChat {
                    project: p.id.clone(),
                    title: title.into(),
                },
            )?;
            return receipt
                .writes
                .into_iter()
                .find(|w| matches!(w.after.data, RecordData::Thread(_)))
                .map(|w| w.after)
                .ok_or_else(|| AppError::Invalid("Project chat was not created".into()));
        }
        self.new_thread(title)
    }
    pub fn project_delete_preview(&self, id: &str) -> AppResult<ProjectDeletePreview> {
        Ok(self.store.project_delete_preview(id)?)
    }
}
