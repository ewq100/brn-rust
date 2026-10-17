//! Explicit owner management; these methods are never registered as model tools.
#[cfg(any(target_os = "macos", target_os = "linux"))]
use crate::AppError;
use crate::{AppResult, Workspace};
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub use brn_ai::skill_packages::{Package, PackageInfo, RETENTION_NOTICE};
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub use brn_ai::thread_skills::SkillStatus;
use brn_threads_core::RecordData;
#[cfg(any(target_os = "macos", target_os = "linux"))]
use brn_threads_core::{ApplyOutcome, HostAuthority, SkillBinding, SkillPolicyChange};
#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::path::Path;
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn management(error: std::io::Error) -> AppError {
    use brn_ai::skill_packages::PackageRefusal;
    match error.get_ref().and_then(|e| e.downcast_ref::<PackageRefusal>()) {
        Some(PackageRefusal::DigestConflict) => return AppError::Invalid("Skill package digest conflict; inspect the current digest before removing or reinstalling".into()),
        Some(PackageRefusal::NotInstalled) => return AppError::Invalid("Skill package is not installed".into()),
        None => {}
    }
    // OS errno must keep native I/O semantics even when its ErrorKind overlaps
    // intentional validation or unsupported-capability errors (e.g. EINVAL).
    if error.raw_os_error().is_some() {
        return AppError::Io(error);
    }
    match error.kind() {
        std::io::ErrorKind::InvalidInput => AppError::Invalid(format!("Skill package: {error}")),
        std::io::ErrorKind::Unsupported => AppError::Unavailable(error.to_string()),
        _ => AppError::Io(error),
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
impl Workspace {
    pub fn skills(&self) -> AppResult<Vec<PackageInfo>> {
        brn_ai::skill_packages::list(&self.store).map_err(management)
    }
    pub fn inspect_skill(&self, name: &str) -> AppResult<Package> {
        brn_ai::skill_packages::inspect(&self.store, name).map_err(management)
    }
    /// Copies a bounded instruction snapshot. Does not enable it, move originals,
    /// change canonical records, or promise inclusion in backup/export.
    pub fn install_skill(&self, source: &Path) -> AppResult<PackageInfo> {
        brn_ai::skill_packages::install(&self.store, source).map_err(management)
    }
    /// Canonical scoped policy and unavailable identities, not installation inventory.
    pub fn scoped_skills(&self, project: Option<&str>) -> AppResult<Vec<SkillStatus>> {
        if let Some(project) = project
            && !self
                .store
                .record(project)?
                .is_some_and(|r| matches!(r.data, RecordData::Project(_)))
        {
            return Err(AppError::Invalid(
                "Skill scope must name an existing Project".into(),
            ));
        }
        let mut rows =
            brn_ai::thread_skills::scoped_skills(&self.store, &self.settings()?.skills, project);
        if let Some(project) = project
            && self.store.record(project)?.is_some_and(|r| r.archived)
        {
            for row in &mut rows {
                row.available = false;
                row.unavailable=Some("This Project is inactive; retained policy may be inspected or cleared, but cannot be selected or loaded.".into());
            }
        }
        Ok(rows)
    }
    fn commit_skill_policy(&mut self, change: SkillPolicyChange) -> AppResult<ApplyOutcome> {
        let op = self
            .store
            .allocate_operation(&format!("skill-policy:{}", uuid::Uuid::new_v4()))?;
        let outcome = self.store.change_skill_policy(
            &op,
            &change,
            &HostAuthority::owner("owner skill configuration"),
        )?;
        Ok(self.remove_leases_ended_by(outcome))
    }
    pub fn set_skill_enabled(
        &mut self,
        id: &str,
        digest: &str,
        project: Option<&str>,
        enabled: bool,
    ) -> AppResult<ApplyOutcome> {
        if enabled {
            let package = self.inspect_skill(id.strip_prefix("installed:").unwrap_or(id))?;
            if package.info.id != id || package.info.digest != digest {
                return Err(management(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    brn_ai::skill_packages::PackageRefusal::DigestConflict,
                )));
            }
        }
        self.commit_skill_policy(SkillPolicyChange::Set(SkillBinding {
            id: id.into(),
            digest: digest.into(),
            project: project.map(str::to_owned),
            enabled,
            selected: false,
        }))
    }
    pub fn select_skill(
        &mut self,
        id: Option<&str>,
        project: Option<&str>,
    ) -> AppResult<ApplyOutcome> {
        let digest = if let Some(id) = id {
            let status = self
                .scoped_skills(project)?
                .into_iter()
                .find(|s| s.id == id)
                .ok_or_else(|| AppError::Invalid("Select an enabled skill in this scope".into()))?;
            if !status.enabled {
                return Err(AppError::Invalid(
                    "Select an enabled skill in this scope".into(),
                ));
            }
            if !status.available {
                if let Err(AppError::Io(error)) =
                    self.inspect_skill(id.strip_prefix("installed:").unwrap_or(id))
                {
                    return Err(AppError::Io(error));
                }
                return Err(AppError::Unavailable("The selected skill is unavailable. Reinstall its exact approved package or change the scoped selection before continuing.".into()));
            }
            Some(status.digest)
        } else {
            None
        };
        self.commit_skill_policy(SkillPolicyChange::Select {
            id: id.map(str::to_owned),
            digest,
            project: project.map(str::to_owned),
        })
    }
    pub fn clear_skill_policy(
        &mut self,
        id: &str,
        project: Option<&str>,
    ) -> AppResult<ApplyOutcome> {
        self.commit_skill_policy(SkillPolicyChange::Clear {
            id: id.into(),
            project: project.map(str::to_owned),
        })
    }
    pub fn remove_skill(&mut self, name: &str, digest: &str) -> AppResult<bool> {
        // Verify the caller's exact local identity before fencing anything.
        let package = match self.inspect_skill(name) {
            Ok(p) => p,
            Err(AppError::Invalid(message)) if message == "Skill package is not installed" => {
                return brn_ai::skill_packages::remove(&self.store, name, digest)
                    .map_err(management);
            }
            Err(e) => return Err(e),
        };
        if !package.info.id.starts_with("installed:") || package.info.digest != digest {
            return Err(management(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                brn_ai::skill_packages::PackageRefusal::DigestConflict,
            )));
        }
        if self
            .settings()?
            .skills
            .iter()
            .any(|b| b.id == package.info.id && b.digest == digest && (b.enabled || b.selected))
        {
            self.commit_skill_policy(SkillPolicyChange::Revoke {
                id: package.info.id,
                digest: digest.into(),
            })?;
        }
        brn_ai::skill_packages::remove(&self.store, name, digest).map_err(management)
    }
}

#[cfg(all(test, any(target_os = "macos", target_os = "linux")))]
mod tests {
    use super::*;
    #[test]
    fn skill_management_preserves_os_faults_and_domain_refusals() {
        for errno in [
            libc::EINVAL,
            libc::EOPNOTSUPP,
            libc::EIO,
            libc::ENOSPC,
            libc::EACCES,
            libc::EWOULDBLOCK,
            libc::EEXIST,
            libc::ENOENT,
        ] {
            let AppError::Io(error) = management(std::io::Error::from_raw_os_error(errno)) else {
                panic!("OS fault {errno} was reclassified as caller validation");
            };
            assert_eq!(error.raw_os_error(), Some(errno));
        }
        assert!(matches!(
            management(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid source"
            )),
            AppError::Invalid(_)
        ));
        let conflict = brn_ai::skill_packages::PackageRefusal::DigestConflict;
        assert!(matches!(
            management(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                conflict
            )),
            AppError::Invalid(_)
        ));
    }
}

/// Historical delivered identities, projected through current host scope before
/// counting or bounding. No instruction files are reopened or reconstructed.
#[derive(Default, serde::Serialize)]
pub struct LoadedSkillDetails {
    pub details: Vec<brn_threads_core::SkillLoad>,
    pub total: usize,
    pub omitted: usize,
}
impl Workspace {
    pub fn loaded_skill_details(&self, run_id: &str) -> AppResult<LoadedSkillDetails> {
        let Some(record) = self.store.record(run_id)? else {
            return Ok(LoadedSkillDetails::default());
        };
        let RecordData::Run(run) = record.data else {
            return Ok(LoadedSkillDetails::default());
        };
        let Some(chat) = self.store.record(&run.thread)? else {
            return Ok(LoadedSkillDetails::default());
        };
        if chat.archived
            || !matches!(chat.data,RecordData::Thread(ref t) if t.state == brn_threads_core::ThreadState::Open)
        {
            return Ok(LoadedSkillDetails::default());
        }
        let project = self.store.project_context_record(&run.thread)?;
        if project.as_ref().is_some_and(|p| p.archived) {
            return Ok(LoadedSkillDetails::default());
        }
        let scope = project.as_ref().map(|p| p.id.as_str());
        let visible = run
            .loaded_skill_details
            .into_iter()
            .filter(|d| d.project.as_deref() == scope)
            .collect::<Vec<_>>();
        let total = visible.len();
        Ok(LoadedSkillDetails {
            details: visible.into_iter().take(16).collect(),
            total,
            omitted: total.saturating_sub(16),
        })
    }
}
