//! Explicit owner management; these methods are never registered as model tools.
use crate::{AppError, AppResult, Workspace};
pub use brn_ai::skill_packages::{Package, PackageInfo, RETENTION_NOTICE};
use std::path::Path;
fn management(error: std::io::Error) -> AppError {
    if error.kind() == std::io::ErrorKind::WouldBlock {
        AppError::Unavailable(error.to_string())
    } else {
        AppError::Invalid(format!("Skill package: {error}"))
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
