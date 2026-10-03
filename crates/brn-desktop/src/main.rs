// The complete handoff theme JSON exceeds the default macro recursion limit.
#![recursion_limit = "256"]

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

mod ai;
#[cfg_attr(not(feature = "native-ui"), allow(dead_code))]
mod layout;
#[cfg_attr(not(feature = "native-ui"), allow(dead_code))]
mod review;
#[cfg_attr(not(feature = "native-ui"), allow(dead_code))]
mod tokens;

#[cfg(feature = "native-ui")]
mod native;

const HELP: &str = "BRN desktop\n\nUsage: brn-desktop [--data-dir ABSOLUTE_DIRECTORY] [--vault ABSOLUTE_DIRECTORY] [--model-dir ABSOLUTE_DIRECTORY]\n       brn-desktop --data-dir ABSOLUTE_DIRECTORY --headless-check startup\n       brn-desktop --help\n\nNative default: ~/Library/Application Support/BRN-simple (credentials: BRN-simple.credentials sibling).\nNotes support explicit Markdown Save, exclusive Save Copy and recoverable unfinished edits.\nLegacy and mixed workspace markers refuse before authority is opened; old data is never migrated.\nAccounts and provider/model selection are explicit in Settings; startup never logs in or discovers models.";

struct Options {
    data_dir: PathBuf,
    explicit: bool,
    check: Option<String>,
    model_dir: Option<PathBuf>,
    vault: Option<PathBuf>,
}
fn parse_args() -> Result<Option<Options>, String> {
    parse(std::env::args().skip(1))
}
fn parse(arguments: impl IntoIterator<Item = String>) -> Result<Option<Options>, String> {
    let mut args = arguments.into_iter();
    let mut data_dir = None;
    let mut check = None;
    let mut model_dir = None;
    let mut vault = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help"
                if data_dir.is_none()
                    && check.is_none()
                    && vault.is_none()
                    && model_dir.is_none()
                    && args.next().is_none() =>
            {
                println!("{HELP}");
                return Ok(None);
            }
            "--data-dir" if data_dir.is_none() => {
                data_dir = Some(PathBuf::from(
                    args.next()
                        .ok_or("--data-dir needs an absolute directory path")?,
                ))
            }
            "--headless-check" if check.is_none() => {
                check = Some(args.next().ok_or("--headless-check needs startup")?)
            }
            "--model-dir" if model_dir.is_none() => {
                model_dir = Some(PathBuf::from(
                    args.next()
                        .ok_or("--model-dir needs an absolute directory path")?,
                ))
            }
            "--vault" if vault.is_none() => {
                vault = Some(PathBuf::from(
                    args.next()
                        .ok_or("--vault needs an absolute directory path")?,
                ));
            }
            _ => {
                return Err(format!(
                    "unknown or repeated argument: {arg}. Run --help for usage."
                ));
            }
        }
    }
    if vault.as_ref().is_some_and(|path| !path.is_absolute()) {
        return Err("--vault must be absolute".into());
    }
    let explicit = data_dir.is_some();
    let data_dir = match data_dir {
        Some(path) => {
            if !path.is_absolute() {
                return Err("--data-dir must be an absolute path".into());
            }
            path
        }
        None => {
            let home = std::env::var_os("HOME").ok_or("HOME is unset; supply --data-dir")?;
            default_directory(PathBuf::from(home))
        }
    };
    if !data_dir.is_absolute() {
        return Err(
            "data directory must be absolute; supply --data-dir when HOME is relative".into(),
        );
    }
    if let Some(ref check) = check
        && check != "startup"
    {
        return Err(format!("unknown headless check: {check}"));
    }
    if model_dir.as_ref().is_some_and(|path| !path.is_absolute()) {
        return Err("--model-dir must be absolute".into());
    }
    Ok(Some(Options {
        data_dir,
        explicit,
        check,
        model_dir,
        vault,
    }))
}
fn default_directory(home: PathBuf) -> PathBuf {
    home.join("Library/Application Support/BRN-simple")
}
fn validate_workspace_mode(path: &Path) -> Result<(), String> {
    match brn_workflow::workspace_mode(path).map_err(|e| e.message)? {
        brn_workflow::WorkspaceMode::Legacy => {
            Err("legacy workspace markers are unsupported; use a new data directory".into())
        }
        brn_workflow::WorkspaceMode::Empty | brn_workflow::WorkspaceMode::Simple => Ok(()),
    }
}
fn validate_data_dir(path: &Path, create_default: bool) -> Result<(), String> {
    if create_default {
        std::fs::create_dir_all(path)
            .map_err(|e| format!("cannot create data directory {}: {e}", path.display()))?;
    }
    let meta = std::fs::metadata(path)
        .map_err(|e| format!("cannot access data directory {}: {e}", path.display()))?;
    if !meta.is_dir() {
        return Err(format!(
            "data directory {} is not a directory",
            path.display()
        ));
    }
    let probe = path.join(format!(
        ".brn-write-probe-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos()
    ));
    let _file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .map_err(|e| format!("data directory {} is not writable: {e}", path.display()))?;
    std::fs::remove_file(&probe).map_err(|e| {
        format!(
            "cannot remove temporary write probe {}: {e}",
            probe.display()
        )
    })?;
    Ok(())
}
fn headless_check(path: PathBuf, config: brn_workflow::app::AppConfig) -> Result<(), String> {
    use brn_workflow::app_worker::{AppEvent, AppWorker};
    let mut worker = AppWorker::start(path, config).map_err(|e| e.message)?;
    loop {
        let (_, event) = worker
            .recv_event_timeout(Duration::from_secs(10))
            .map_err(|e| format!("application worker startup did not complete: {e}"))?;
        match event {
            AppEvent::Ready { .. } => break,
            AppEvent::Failed(error) => return Err(error.message),
            _ => {}
        }
    }
    worker.shutdown().map_err(|e| e.message)?;
    println!("startup: PASS");
    Ok(())
}
fn run() -> Result<(), String> {
    let Some(options) = parse_args()? else {
        return Ok(());
    };
    validate_workspace_mode(&options.data_dir)?;
    if options.check.is_some() {
        if !options.explicit {
            return Err("--headless-check requires an explicit --data-dir".into());
        }
        validate_data_dir(&options.data_dir, false)?;
        return headless_check(
            options.data_dir,
            brn_workflow::app::AppConfig {
                vault_root: options.vault,
                credentials_dir: None,
                model_dir: options.model_dir,
            },
        );
    }
    #[cfg(feature = "native-ui")]
    {
        validate_data_dir(&options.data_dir, true)?;
        let path = std::fs::canonicalize(&options.data_dir)
            .map_err(|e| format!("cannot canonicalize data directory: {e}"))?;
        let preferences = layout::load(&path);
        native::run(
            path,
            brn_workflow::app::AppConfig {
                vault_root: options.vault,
                credentials_dir: None,
                model_dir: options.model_dir,
            },
            preferences,
        );
        Ok(())
    }
    #[cfg(not(feature = "native-ui"))]
    {
        let _ = (
            options.data_dir,
            options.explicit,
            options.model_dir,
            options.vault,
        );
        Err("native UI is unavailable in this build; rebuild with --features native-ui or use --headless-check".into())
    }
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
#[cfg(test)]
mod mode_tests {
    use super::*;
    fn options(args: &[&str]) -> Options {
        parse(args.iter().map(|arg| (*arg).to_owned()))
            .unwrap()
            .unwrap()
    }
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/desktop-mode-fixtures")
                .join(uuid::Uuid::new_v4().to_string());
            std::fs::create_dir_all(&path).unwrap();
            Self(std::fs::canonicalize(path).unwrap())
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
    #[test]
    fn defaults_are_separate_and_repository_credentials_are_refused() {
        let home = PathBuf::from("/synthetic/home");
        let simple = default_directory(home.clone());
        assert_eq!(simple, home.join("Library/Application Support/BRN-simple"));
        let fixture = Fixture::new();
        let data = fixture.0.join("BRN-simple");
        std::fs::create_dir(&data).unwrap();
        assert_eq!(
            brn_workflow::app::default_credentials_dir(&data)
                .unwrap_err()
                .kind,
            brn_workflow::ErrorKind::UnsafeCredentials
        );
        assert!(parse(["--legacy".to_owned()]).is_err());
    }
    #[test]
    fn explicit_modes_include_sidecars_and_backups_and_refuse_mixed() {
        let fixture = Fixture::new();
        let args = ["--data-dir", fixture.0.to_str().unwrap()];
        let simple = options(&args);
        validate_workspace_mode(&simple.data_dir).unwrap();
        std::fs::write(fixture.0.join("brn.sqlite3-journal"), b"synthetic").unwrap();
        assert!(validate_workspace_mode(&simple.data_dir).is_err());
        let with_vault = options(&[
            "--data-dir",
            fixture.0.to_str().unwrap(),
            "--vault",
            "/synthetic/vault",
        ]);
        assert!(validate_workspace_mode(&with_vault.data_dir).is_err());
        std::fs::create_dir(fixture.0.join("backups")).unwrap();
        std::fs::write(fixture.0.join("backups/brn-123.sqlite-wal"), b"synthetic").unwrap();
        assert!(validate_workspace_mode(&simple.data_dir).is_err());
        std::fs::remove_file(fixture.0.join("brn.sqlite3-journal")).unwrap();
        validate_workspace_mode(&simple.data_dir).unwrap();
        assert!(
            parse(["--legacy", "--data-dir", fixture.0.to_str().unwrap()].map(str::to_owned))
                .is_err()
        );
        assert!(!fixture.0.join("brn.sqlite").exists());
        assert!(!fixture.0.join("brn.sqlite3").exists());
    }
    #[test]
    fn flags_refuse_legacy_vault_before_io_and_do_not_accept_provider_executables() {
        assert!(parse(["--legacy", "--vault", "/synthetic/vault"].map(str::to_owned)).is_err());
        assert!(parse(["--codex", "/synthetic/executable"].map(str::to_owned)).is_err());
        assert!(parse(["--vault", "relative"].map(str::to_owned)).is_err());
    }
    #[test]
    fn explicit_missing_directory_is_created_and_canonicalized_before_gpui() {
        let fixture = Fixture::new();
        let path = fixture.0.join("new");
        validate_data_dir(&path, true).unwrap();
        assert!(std::fs::canonicalize(&path).unwrap().is_absolute());
        assert!(!path.join("brn.sqlite").exists());
        assert!(!path.join("brn.sqlite3").exists());
    }
    #[test]
    fn real_app_worker_returns_asynchronous_safe_startup_failure_without_frontend_authority() {
        let fixture = Fixture::new();
        let mut worker = brn_workflow::app_worker::AppWorker::start(
            fixture.0.join("missing"),
            brn_workflow::app::AppConfig {
                vault_root: None,
                credentials_dir: None,
                model_dir: None,
            },
        )
        .unwrap();
        let (_, event) = worker.recv_event_timeout(Duration::from_secs(5)).unwrap();
        assert!(matches!(
            event,
            brn_workflow::app_worker::AppEvent::Failed(_)
        ));
        assert!(worker.shutdown().is_err());
        assert!(!fixture.0.join("missing/brn.sqlite3").exists());
    }
}
