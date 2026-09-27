use brn_core::{Shell, WorkConfig};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

#[cfg(feature = "native-ui")]
mod native;

const HELP: &str = "BRN desktop shell (sample work only)\n\nUsage: brn-desktop [--data-dir ABSOLUTE_EXISTING_DIRECTORY] [--headless-check completion|cancellation|stale]\n       brn-desktop --help\n\nThe native shell opens Workspace, Activity and Settings. Explicit data directories must exist and be writable. If omitted, native launch creates ~/Library/Application Support/BRN. The sample task keeps its input and result in memory; no documents or database are written.";

struct Options {
    data_dir: PathBuf,
    explicit: bool,
    check: Option<String>,
}
fn parse_args() -> Result<Option<Options>, String> {
    let mut args = std::env::args().skip(1);
    let mut data_dir = None;
    let mut check = None;
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
    Ok(Some(Options {
        data_dir,
        explicit,
        check,
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
        native::run(options.data_dir);
        Ok(())
    }
    #[cfg(not(feature = "native-ui"))]
    {
        let _ = (options.data_dir, options.explicit);
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
