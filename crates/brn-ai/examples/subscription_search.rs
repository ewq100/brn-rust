//! One explicitly authorized public-query qualification of the production adapter.
//! No default query, retries, workspace content, or account override.
use brn_ai::{Provider, ReasoningEffort, Selection, subscription_search};
use serde_json::json;
use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    path::Path,
    process::ExitCode,
};
use tokio_util::sync::CancellationToken;

struct PrivateDiagnostic(File);
impl PrivateDiagnostic {
    #[cfg(unix)]
    fn create(path: &Path) -> io::Result<Self> {
        use std::os::unix::fs::OpenOptionsExt;
        let parent = path.parent().ok_or(io::ErrorKind::InvalidInput)?;
        if !path.is_absolute() || parent.canonicalize()? != parent {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        for ancestor in parent.ancestors() {
            if ancestor.join(".git").try_exists()? {
                return Err(io::ErrorKind::PermissionDenied.into());
            }
        }
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)
            .map(Self)
    }
    #[cfg(not(unix))]
    fn create(_: &Path) -> io::Result<Self> {
        Err(io::ErrorKind::Unsupported.into())
    }
    fn retain_warning(&mut self, message: &str) -> io::Result<()> {
        serde_json::to_writer(
            &mut self.0,
            &json!({"kind":"provider_warning","message":message}),
        )?;
        self.0.write_all(b"\n")?;
        self.0.sync_all()
    }
}

fn retain_warning(
    error: &subscription_search::SearchError,
    file: Option<&mut PrivateDiagnostic>,
) -> bool {
    match (error.private_warning_message(), file) {
        (Some(message), Some(file)) => file.retain_warning(message).is_ok(),
        _ => false,
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if !matches!(args.len(), 3 | 5)
        || args[0] != "--live-public-query"
        || (args.len() == 5 && args[3] != "--private-warning-file")
    {
        eprintln!(
            "usage: subscription-search --live-public-query EXACT_MODEL PUBLIC_QUERY [--private-warning-file ABSOLUTE_NEW_FILE]\nRequires owner authorization and a durable live-turn reservation; uses normal managed login, medium effort, one request, 60-second deadline. Private diagnostics require a fresh file outside repositories in a canonical existing directory. Never publish that file."
        );
        return ExitCode::from(2);
    }
    // Refuse an unusable diagnostic destination before any provider process or
    // model turn. Holding this new file prevents replacement/overwrite races.
    let mut diagnostic = if args.len() == 5 {
        match PrivateDiagnostic::create(Path::new(&args[4])) {
            Ok(file) => Some(file),
            Err(_) => {
                eprintln!("Private diagnostic destination refused; no search was sent.");
                return ExitCode::from(2);
            }
        }
    } else {
        None
    };
    let request = subscription_search::Request {
        selection: Selection {
            provider: Provider::Chatgpt,
            model: args[1].clone(),
        },
        effort: ReasoningEffort::Medium,
        query: args[2].clone(),
        auth_file: None,
    };
    match subscription_search::search(request, CancellationToken::new()).await {
        Ok(result) => {
            let sources: Vec<_> = result
                .sources
                .into_iter()
                .map(|s| json!({"url":s.url,"title":s.title}))
                .collect();
            println!(
                "{}",
                json!({"status":"passed","answer":result.answer,"sources":sources,"provider":"chatgpt","model":result.provenance.model,"effort":"medium","cost":null})
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            let private_warning_saved = retain_warning(&error, diagnostic.as_mut());
            println!(
                "{}",
                json!({"status":"failed","error":format!("{error:?}"),"message":error.to_string(),"private_warning_saved":private_warning_saved,"cost":null})
            );
            ExitCode::FAILURE
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::{PermissionsExt, symlink};

    #[test]
    fn private_diagnostic_is_fresh_owner_only_and_never_overwrites() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        let path = root.join("private.json");
        let mut diagnostic = PrivateDiagnostic::create(&path).unwrap();
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let message = "synthetic private warning — Ω\nnot an instruction";
        diagnostic.retain_warning(message).unwrap();
        let original = std::fs::read(&path).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&original).unwrap();
        assert_eq!(value["message"], message);
        assert!(PrivateDiagnostic::create(&path).is_err());
        let link = root.join("link.json");
        symlink(&path, &link).unwrap();
        assert!(PrivateDiagnostic::create(&link).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
    }

    #[test]
    fn private_diagnostic_refuses_repository_and_ambiguous_destinations() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        assert!(PrivateDiagnostic::create(Path::new("relative.json")).is_err());
        let alias = root.join("alias");
        symlink(&root, &alias).unwrap();
        assert!(PrivateDiagnostic::create(&alias.join("private.json")).is_err());
        std::fs::write(root.join(".git"), "synthetic worktree marker").unwrap();
        assert!(PrivateDiagnostic::create(&root.join("private.json")).is_err());
        assert!(!root.join("private.json").exists());
    }

    #[test]
    fn failed_private_write_preserves_existing_bytes() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        let path = root.join("readonly.json");
        std::fs::write(&path, "unchanged").unwrap();
        let mut readonly = PrivateDiagnostic(File::open(&path).unwrap());
        assert!(readonly.retain_warning("synthetic warning").is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "unchanged");
    }
}
