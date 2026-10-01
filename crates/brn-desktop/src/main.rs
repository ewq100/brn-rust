// The complete handoff theme JSON exceeds the default macro recursion limit.
#![recursion_limit = "256"]

use brn_core::{Shell, WorkConfig};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

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

const HELP: &str = "BRN desktop\n\nUsage: brn-desktop [--data-dir ABSOLUTE_DIRECTORY] [--codex ABSOLUTE_EXECUTABLE] [--model-dir ABSOLUTE_DIRECTORY]\n       brn-desktop --data-dir ABSOLUTE_DIRECTORY --headless-check completion|cancellation|stale\n       brn-desktop --help\n\nThe native Notes page opens local Markdown notes in one chosen vault. Explicit Save/Cmd-S writes Markdown; automatic buffer recovery only protects edits in BRN. The workspace also imports Markdown/text, builds search, and answers from approved sources. The headless checks preserve the original deterministic shell fixture.";

struct Options {
    data_dir: PathBuf,
    explicit: bool,
    check: Option<String>,
    codex: Option<PathBuf>,
    model_dir: Option<PathBuf>,
}
fn parse_args() -> Result<Option<Options>, String> {
    let mut args = std::env::args().skip(1);
    let mut data_dir = None;
    let mut check = None;
    let mut codex = None;
    let mut model_dir = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" if data_dir.is_none() && check.is_none() && args.next().is_none() => {
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
            "--codex" if codex.is_none() => {
                codex = Some(PathBuf::from(
                    args.next()
                        .ok_or("--codex needs an absolute executable path")?,
                ))
            }
            "--model-dir" if model_dir.is_none() => {
                model_dir = Some(PathBuf::from(
                    args.next()
                        .ok_or("--model-dir needs an absolute directory path")?,
                ))
            }
            _ => {
                return Err(format!(
                    "unknown or repeated argument: {arg}. Run --help for usage."
                ));
            }
        }
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
            PathBuf::from(home).join("Library/Application Support/BRN")
        }
    };
    if let Some(ref check) = check
        && !["completion", "cancellation", "stale"].contains(&check.as_str())
    {
        return Err(format!("unknown headless check: {check}"));
    }
    if codex.as_ref().is_some_and(|path| !path.is_absolute()) {
        return Err("--codex must be absolute".into());
    }
    if model_dir.as_ref().is_some_and(|path| !path.is_absolute()) {
        return Err("--model-dir must be absolute".into());
    }
    Ok(Some(Options {
        data_dir,
        explicit,
        check,
        codex,
        model_dir,
    }))
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
    if let Some(kind) = options.check {
        if !options.explicit {
            return Err("--headless-check requires an explicit --data-dir".into());
        }
        validate_data_dir(&options.data_dir, false)?;
        return headless_check(&kind);
    }
    #[cfg(feature = "native-ui")]
    {
        validate_data_dir(&options.data_dir, !options.explicit)?;
        native::run(
            options.data_dir,
            brn_workflow::Config {
                codex: options.codex,
                model_dir: options.model_dir,
                codex_home: None,
            },
        );
        Ok(())
    }
    #[cfg(not(feature = "native-ui"))]
    {
        let _ = (
            options.data_dir,
            options.explicit,
            options.codex,
            options.model_dir,
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
