//! Only public build/environment identifiers enter the user-reviewed issue form.
use brn_threads_app::WorkspaceSettings;
use std::{path::Path, process::Command, sync::OnceLock};

#[derive(Debug, PartialEq)]
pub(crate) struct AppVersion {
    pub version: String,
    pub build: String,
}
impl AppVersion {
    pub fn current() -> &'static Self {
        static VERSION: OnceLock<AppVersion> = OnceLock::new();
        VERSION.get_or_init(|| {
            std::env::current_exe()
                .ok()
                .and_then(|exe| Self::from_executable(&exe))
                .unwrap_or_else(|| Self {
                    version: env!("CARGO_PKG_VERSION").into(),
                    build: "development build".into(),
                })
        })
    }
    fn from_executable(exe: &Path) -> Option<Self> {
        // The packaged executable is Contents/Resources/bin/brn-desktop.
        let bin = exe.parent()?;
        if bin.file_name()? != "bin" || bin.parent()?.file_name()? != "Resources" {
            return None;
        }
        let plist = bin.parent()?.parent()?.join("Info.plist");
        let read = |key| {
            command_value(
                "/usr/bin/plutil",
                &["-extract", key, "raw", "-o", "-", plist.to_str()?],
            )
        };
        Some(Self {
            version: read("BRNPreviewVersion").or_else(|| read("CFBundleShortVersionString"))?,
            build: read("CFBundleVersion")?,
        })
    }
    pub fn label(&self) -> String {
        format!(
            "BRN {} · {}",
            self.version,
            if self.build == "development build" {
                self.build.clone()
            } else {
                format!("build {}", self.build)
            }
        )
    }
}
fn command_value(command: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(command).args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8(output.stdout).ok())
        .flatten()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
}

pub(crate) fn report_url(settings: &WorkspaceSettings) -> String {
    issue_url(
        AppVersion::current(),
        &command_value("/usr/bin/sw_vers", &["-productVersion"])
            .unwrap_or_else(|| "unavailable".into()),
        std::env::consts::ARCH,
        &settings.provider,
        &settings.model,
    )
}

// Per-field bounds also bound the encoded URL to less than 2,000 bytes.
// Reject unexpected identifier text rather than truncating a path or credential.
fn identifier(value: &str) -> &str {
    if value.is_empty()
        || value.len() > 80
        || value.starts_with('/')
        || value.contains("..")
        || value.starts_with("sk-")
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    {
        "unavailable"
    } else {
        value
    }
}
fn encode(value: &str) -> String {
    let mut out = String::new();
    for b in value.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            out.push(b as char);
        } else {
            use std::fmt::Write;
            write!(out, "%{b:02X}").expect("write to string");
        }
    }
    out
}
fn issue_url(version: &AppVersion, os: &str, arch: &str, provider: &str, model: &str) -> String {
    let build = if version.build == "development build" {
        "development build"
    } else {
        identifier(&version.build)
    };
    let body = format!(
        "### Environment\nApp version: {}\nBuild: {build}\nmacOS: {}\nCPU architecture: {}\nProvider: {}\nModel: {}\n\n### What happened\n\n### What you expected\n\n### Steps to reproduce\n",
        identifier(&version.version),
        identifier(os),
        identifier(arch),
        identifier(provider),
        identifier(model)
    );
    format!(
        "https://github.com/ewq100/brn-rust/issues/new?title={}&body={}",
        encode("Report a problem"),
        encode(&body)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn issue_form_encodes_headings_and_stays_bounded() {
        let version = AppVersion {
            version: "v0.2.0-alpha.2".into(),
            build: "42".into(),
        };
        assert!(version.label().contains("build 42"));
        let url = issue_url(&version, "15.2", "aarch64", "copilot", "gpt-6.1");
        assert!(url.contains("title=Report%20a%20problem&body=%23%23%23%20Environment%0A"));
        assert!(url.contains("App%20version%3A%20v0.2.0-alpha.2%0ABuild%3A%2042"));
        assert!(!url.contains(' '));
        assert_eq!(encode("&?#+é"), "%26%3F%23%2B%C3%A9");
        let huge = "é&".repeat(10000);
        assert!(issue_url(&version, &huge, &huge, &huge, &huge).len() < 2000);
        assert!(
            issue_url(
                &version,
                &"a".repeat(80),
                &"a".repeat(80),
                &"a".repeat(80),
                &"a".repeat(80)
            )
            .len()
                < 2000
        );
    }
    #[test]
    fn report_uses_no_private_settings_or_workspace_content() {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let workspace = brn_threads_app::Workspace::open_without_default_credentials(
            root.path().join("synthetic"),
        )
        .unwrap();
        let mut settings = workspace.settings().unwrap();
        settings.auth_file = Some("/Users/private-person/private@example.com/token".into());
        settings.credentials_dir = Some("/secret/account".into());
        settings.backup_directory = Some("/private/notes".into());
        settings.model = "bad&injected=token@example.com".into();
        let url = report_url(&settings);
        for private in [
            "private-person",
            "example.com",
            "secret",
            "account",
            "notes",
            "injected",
            "auth_file",
            "credentials_dir",
            "backup_directory",
        ] {
            assert!(!url.contains(private), "{url}");
        }
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn packaged_plist_overrides_cargo_version() {
        let root = tempfile::tempdir().unwrap();
        let contents = root.path().join("BRN.app/Contents");
        std::fs::create_dir_all(contents.join("Resources/bin")).unwrap();
        std::fs::write(contents.join("Info.plist"), r#"<?xml version="1.0"?><plist version="1.0"><dict><key>CFBundleShortVersionString</key><string>0.2.0</string><key>BRNPreviewVersion</key><string>v0.2.0-alpha.2</string><key>CFBundleVersion</key><string>42</string></dict></plist>"#).unwrap();
        let exe = contents.join("Resources/bin/brn-desktop");
        assert_eq!(
            AppVersion::from_executable(&exe).unwrap().label(),
            "BRN v0.2.0-alpha.2 · build 42"
        );
        assert!(AppVersion::from_executable(&root.path().join("brn-desktop")).is_none());
    }
}
