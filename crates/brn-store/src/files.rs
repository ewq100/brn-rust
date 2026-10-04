//! Identity and exact-content proofs shared by operational save journals and
//! the workflow's filesystem adapter. No filesystem or database I/O occurs here.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileFingerprint {
    pub device: u64,
    pub inode: u64,
    pub len: u64,
    pub sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultIdentity {
    pub device: u64,
    pub inode: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultRecord {
    pub id: Uuid,
    pub root: PathBuf,
    pub identity: VaultIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedFile {
    pub relative: PathBuf,
    pub fingerprint: FileFingerprint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactIdentity {
    pub device: u64,
    pub inode: u64,
    pub len: u64,
    pub kind: ArtifactKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactKind {
    Regular,
    Directory,
    Symlink,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetainedArtifact {
    pub relative: PathBuf,
    pub identity: ArtifactIdentity,
    pub sha256: Option<[u8; 32]>,
}
