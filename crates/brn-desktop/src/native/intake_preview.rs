//! Explicit local inspection of exact captured originals using the system facility.
use std::{
    io::Write,
    process::{Child, Command, Stdio},
};

pub(super) struct OriginalPreview {
    child: Child,
    _directory: tempfile::TempDir,
}
impl OriginalPreview {
    pub(super) fn open(bytes: &[u8], media_type: &str) -> Result<Self, String> {
        let extension = match media_type {
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => "docx",
            "message/rfc822" => "eml",
            "application/vnd.openxmlformats-officedocument.presentationml.presentation" => "pptx",
            _ => return Err("Original preview supports retained DOCX/EML/PPTX. Export other attachments for inspection.".into()),
        };
        if bytes.is_empty() || bytes.len() > brn_intake::MAX_INPUT_BYTES {
            return Err("Original preview byte budget.".into());
        }
        let directory = tempfile::Builder::new()
            .prefix("brn-intake-preview-")
            .tempdir()
            .map_err(|e| format!("Cannot create private original preview: {e}"))?;
        let path = directory
            .path()
            .join(format!("retained-original.{extension}"));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options
            .open(&path)
            .and_then(|mut file| {
                file.write_all(bytes)?;
                file.sync_all()
            })
            .map_err(|e| format!("Cannot retain exact original preview bytes: {e}"))?;
        #[cfg(target_os = "macos")]
        {
            use std::os::unix::process::CommandExt;
            let child = Command::new("/usr/bin/qlmanage")
                .arg("-p")
                .arg(&path)
                .env_clear()
                .current_dir(directory.path())
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .process_group(0)
                .spawn()
                .map_err(|e| format!("System Quick Look could not start: {e}"))?;
            Ok(Self {
                child,
                _directory: directory,
            })
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = path;
            Err("System original preview is qualified only on macOS; export exact bytes for inspection.".into())
        }
    }
}
impl Drop for OriginalPreview {
    fn drop(&mut self) {
        #[cfg(unix)]
        // This child was created as its own process group leader, never the app's group.
        unsafe {
            libc::kill(-(self.child.id() as i32), libc::SIGKILL);
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
