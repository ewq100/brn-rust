// The complete handoff theme JSON exceeds the default macro recursion limit.
#![recursion_limit = "256"]

use brn_core::{Shell, WorkConfig};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

mod ai;
#[cfg_attr(not(feature = "native-ui"), allow(dead_code))]
mod layout;
#[cfg_attr(not(feature = "native-ui"), allow(dead_code))]
mod tokens;

#[cfg(feature = "native-ui")]
mod comments;
#[cfg(feature = "native-ui")]
mod drafts;
#[cfg(feature = "native-ui")]
mod native;
#[cfg(feature = "native-ui")]
mod notes;

const HELP: &str = "BRN desktop\n\nUsage: brn-desktop [--data-dir ABSOLUTE_DIRECTORY] [--vault ABSOLUTE_DIRECTORY] [--model-dir ABSOLUTE_DIRECTORY]\n       brn-desktop --legacy [--data-dir ABSOLUTE_DIRECTORY]\n       brn-desktop --data-dir ABSOLUTE_DIRECTORY --headless-check completion|cancellation|stale\n       brn-desktop --help\n\nNative default: ~/Library/Application Support/BRN-simple (credentials: BRN-simple.credentials sibling).\nSimple notes are saved-file readers; no Markdown Save is implemented here.\n--legacy explicitly opens the old BRN default for local editing/recovery/history only; legacy AI is retired.\nAccounts and provider/model selection are explicit in Settings; startup never logs in or discovers models.";

struct Options {
    data_dir: PathBuf,
    explicit: bool,
    check: Option<String>,
    model_dir: Option<PathBuf>,
    legacy: bool,
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
    let mut legacy = false;
    let mut vault = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help"
                if data_dir.is_none()
                    && check.is_none()
                    && !legacy
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
                check = Some(
                    args.next()
                        .ok_or("--headless-check needs completion, cancellation, or stale")?,
                )
            }
            "--model-dir" if model_dir.is_none() => {
                model_dir = Some(PathBuf::from(
                    args.next()
                        .ok_or("--model-dir needs an absolute directory path")?,
                ))
            }
            "--legacy" if !legacy => legacy = true,
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
    if legacy && vault.is_some() {
        return Err("--legacy and --vault are incompatible".into());
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
            default_directory(PathBuf::from(home), legacy)
        }
    };
    if !data_dir.is_absolute() {
        return Err(
            "data directory must be absolute; supply --data-dir when HOME is relative".into(),
        );
    }
    if let Some(ref check) = check
        && !["completion", "cancellation", "stale"].contains(&check.as_str())
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
        legacy,
        vault,
    }))
}
fn default_directory(home: PathBuf, legacy: bool) -> PathBuf {
    home.join("Library/Application Support")
        .join(if legacy { "BRN" } else { "BRN-simple" })
}
#[cfg_attr(not(feature = "native-ui"), allow(dead_code))]
fn launch_mode(options: &Options) -> Result<brn_workflow::WorkspaceMode, String> {
    use brn_workflow::WorkspaceMode;
    let mode = brn_workflow::workspace_mode(&options.data_dir).map_err(|e| e.message)?;
    match mode {
        WorkspaceMode::Legacy if options.vault.is_some() => Err(
            "--vault is incompatible with legacy workspace markers; use a new simple directory"
                .into(),
        ),
        WorkspaceMode::Simple if options.legacy => {
            Err("--legacy is incompatible with simple workspace markers".into())
        }
        WorkspaceMode::Empty => Ok(if options.legacy {
            WorkspaceMode::Legacy
        } else {
            WorkspaceMode::Simple
        }),
        mode => Ok(mode),
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
fn wait(shell: &mut Shell) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while shell.active().is_some() && Instant::now() < deadline {
        shell.poll();
        std::thread::sleep(Duration::from_millis(2));
    }
    if shell.active().is_some() {
        Err("sample worker timed out".into())
    } else {
        Ok(())
    }
}
fn headless_check(kind: &str) -> Result<(), String> {
    let mut shell = Shell::new("Sample workspace text");
    let config = WorkConfig {
        steps: 50,
        step_delay: Duration::from_millis(3),
    };
    shell
        .start(config)
        .map_err(|e| format!("start failed: {e:?}"))?;
    match kind {
        "completion" => {
            wait(&mut shell)?;
            if shell.result().is_none() {
                return Err("sample did not complete".into());
            }
        }
        "cancellation" => {
            if !shell.cancel() {
                return Err("cancellation was not accepted".into());
            }
            wait(&mut shell)?;
            if !shell.was_cancelled() {
                return Err("sample was not cancelled".into());
            }
        }
        "stale" => {
            shell
                .edit("New text")
                .map_err(|e| format!("edit failed: {e:?}"))?;
            wait(&mut shell)?;
            if shell.result().is_some() {
                return Err("stale result became current".into());
            }
        }
        _ => return Err("invalid check".into()),
    }
    println!("{kind}: PASS");
    Ok(())
}
fn run() -> Result<(), String> {
    let Some(options) = parse_args()? else {
        return Ok(());
    };
    if let Some(ref kind) = options.check {
        if !options.explicit {
            return Err("--headless-check requires an explicit --data-dir".into());
        }
        validate_data_dir(&options.data_dir, false)?;
        return headless_check(kind);
    }
    #[cfg(feature = "native-ui")]
    {
        let mode = launch_mode(&options)?;
        validate_data_dir(&options.data_dir, true)?;
        let path = std::fs::canonicalize(&options.data_dir)
            .map_err(|e| format!("cannot canonicalize data directory: {e}"))?;
        let preferences = layout::load(&path);
        native::run(
            path,
            brn_workflow::Config {
                model_dir: options.model_dir,
            },
            mode,
            options.vault,
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
            options.legacy,
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
    use brn_workflow::WorkspaceMode;
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
        let simple = default_directory(home.clone(), false);
        assert_eq!(simple, home.join("Library/Application Support/BRN-simple"));
        assert_eq!(
            default_directory(home.clone(), true),
            home.join("Library/Application Support/BRN")
        );
        let fixture = Fixture::new();
        let data = fixture.0.join("BRN-simple");
        std::fs::create_dir(&data).unwrap();
        assert_eq!(
            brn_workflow::app::default_credentials_dir(&data)
                .unwrap_err()
                .kind,
            brn_workflow::ErrorKind::UnsafeCredentials
        );
        assert!(!options(&[]).legacy);
        assert!(options(&["--legacy"]).legacy);
    }
    #[test]
    fn explicit_modes_include_sidecars_and_backups_and_refuse_mixed() {
        let fixture = Fixture::new();
        let args = ["--data-dir", fixture.0.to_str().unwrap()];
        let simple = options(&args);
        assert_eq!(launch_mode(&simple).unwrap(), WorkspaceMode::Simple);
        std::fs::write(fixture.0.join("brn.sqlite3-journal"), b"synthetic").unwrap();
        assert_eq!(launch_mode(&simple).unwrap(), WorkspaceMode::Legacy);
        let with_vault = options(&[
            "--data-dir",
            fixture.0.to_str().unwrap(),
            "--vault",
            "/synthetic/vault",
        ]);
        assert!(launch_mode(&with_vault).is_err());
        std::fs::create_dir(fixture.0.join("backups")).unwrap();
        std::fs::write(fixture.0.join("backups/brn-123.sqlite-wal"), b"synthetic").unwrap();
        assert!(launch_mode(&simple).is_err());
        std::fs::remove_file(fixture.0.join("brn.sqlite3-journal")).unwrap();
        assert_eq!(launch_mode(&simple).unwrap(), WorkspaceMode::Simple);
        let legacy = options(&["--legacy", "--data-dir", fixture.0.to_str().unwrap()]);
        assert!(launch_mode(&legacy).is_err());
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
