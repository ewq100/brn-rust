//! Internal recovery checkpoint status; never authority over an approved operation.
use crate::app::App;
use brn_store::work::BackupCheckpointOutcome;
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupStatus {
    pub latest_path: PathBuf,
    pub completed_at_ms: Option<u64>,
    pub retention_warning: Option<String>,
    pub last_error: Option<String>,
}
impl BackupStatus {
    pub(crate) fn startup(report: &brn_store::OpenReport) -> Self {
        Self {
            latest_path: report.backup.clone(),
            completed_at_ms: None,
            retention_warning: report.retention_warning.clone(),
            last_error: None,
        }
    }
}
impl App {
    pub fn backup_status(&self) -> BackupStatus {
        self.backup_status.clone()
    }

    /// A separate result: a failed checkpoint cannot undo or fail committed work.
    pub fn checkpoint_backup(&mut self) -> BackupStatus {
        match self.store.checkpoint_if_changed() {
            Ok(BackupCheckpointOutcome::Unchanged) => self.backup_status.last_error = None,
            Ok(BackupCheckpointOutcome::Created {
                path,
                completed_at_ms,
                retention_warning,
            }) => {
                self.backup_status = BackupStatus {
                    latest_path: path,
                    completed_at_ms: Some(completed_at_ms),
                    retention_warning,
                    last_error: None,
                };
            }
            Err(error) => self.backup_status.last_error = Some(error.to_string()),
        }
        self.backup_status()
    }
}

pub(crate) const AUTOMATIC_INTERVAL: Duration = Duration::from_secs(60);
const IDLE_WAKE: Duration = Duration::from_secs(2);
pub(crate) struct BackupCadence {
    next: Instant,
    interval: Duration,
}
impl BackupCadence {
    pub(crate) fn new(now: Instant, interval: Duration) -> Self {
        Self {
            next: now + interval,
            interval,
        }
    }
    pub(crate) fn due(&mut self, now: Instant) -> bool {
        if now < self.next {
            return false;
        }
        self.next = now + self.interval;
        true
    }
    pub(crate) fn wait(&self, now: Instant) -> Duration {
        self.next.saturating_duration_since(now).min(IDLE_WAKE)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cadence_bounds_idle_wait_and_checks_sustained_traffic_without_catchup_storm() {
        let now = Instant::now();
        let mut cadence = BackupCadence::new(now, AUTOMATIC_INTERVAL);
        assert_eq!(cadence.wait(now), IDLE_WAKE);
        assert!(!cadence.due(now + Duration::from_secs(59)));
        assert!(cadence.due(now + Duration::from_secs(60)));
        assert!(!cadence.due(now + Duration::from_secs(60)));
        assert!(cadence.due(now + Duration::from_secs(600)));
        assert!(!cadence.due(now + Duration::from_secs(600)));
        assert_eq!(
            cadence.wait(now + Duration::from_secs(659)),
            Duration::from_secs(1)
        );
    }
}
