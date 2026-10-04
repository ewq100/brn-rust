use std::path::{Path, PathBuf};

pub struct DataDir {
    _owner: tempfile::TempDir,
    data: PathBuf,
}

pub fn data_dir() -> DataDir {
    let parent = std::env::temp_dir().canonicalize().unwrap();
    let owner = tempfile::Builder::new().tempdir_in(parent).unwrap();
    let data = owner.path().join("data");
    std::fs::create_dir(&data).unwrap();
    DataDir {
        _owner: owner,
        data,
    }
}

impl DataDir {
    pub fn path(&self) -> &Path {
        &self.data
    }
}
