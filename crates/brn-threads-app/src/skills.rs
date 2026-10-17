//! Explicit owner management; these methods are never registered as model tools.
use crate::{AppError, AppResult, Workspace};
pub use brn_ai::skill_packages::{Package, PackageInfo, RETENTION_NOTICE};
use std::path::Path;
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
    pub fn remove_skill(&self, name: &str, digest: &str) -> AppResult<bool> {
        brn_ai::skill_packages::remove(&self.store, name, digest).map_err(management)
    }
}

#[cfg(test)]
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
