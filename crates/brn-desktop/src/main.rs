#![recursion_limit = "256"]
use std::{path::PathBuf, process::ExitCode};
#[cfg(all(test, not(feature = "native-ui")))]
#[path = "threads_state.rs"]
mod threads_state;
#[cfg(feature = "native-ui")]
mod threads_ui;
const HELP: &str = "BRN Threads\n\nUsage: brn-desktop --data-dir ABSOLUTE_DIRECTORY\n       brn-desktop --data-dir ABSOLUTE_DIRECTORY --headless-check startup\n\nUse a fresh explicit Threads data directory. Other BRN formats refuse before opening.\nThe provider and model are shown in Settings. Originals remain external.";
fn parse(args: impl IntoIterator<Item = String>) -> Result<Option<(PathBuf, bool)>, String> {
    let mut args = args.into_iter();
    let mut data = None;
    let mut check = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" if data.is_none() && !check && args.next().is_none() => {
                println!("{HELP}");
                return Ok(None);
            }
            "--data-dir" if data.is_none() => {
                data = Some(PathBuf::from(
                    args.next()
                        .ok_or("--data-dir needs an absolute directory")?,
                ))
            }
            "--headless-check" if !check => {
                if args.next().as_deref() != Some("startup") {
                    return Err("--headless-check needs startup".into());
                }
                check = true
            }
            _ => return Err("Unknown or repeated argument; run --help".into()),
        }
    }
    let data = data.ok_or("Supply --data-dir with a fresh absolute Threads directory")?;
    if !data.is_absolute() {
        return Err("--data-dir must be absolute".into());
    }
    Ok(Some((data, check)))
}
fn launch(data: PathBuf, check: bool) -> Result<(), String> {
    if check {
        let workspace = brn_threads_app::Workspace::open(&data).map_err(|e| e.to_string())?;
        println!(
            "Threads ready: {} records",
            workspace.records().map_err(|e| e.to_string())?.len()
        );
        return Ok(());
    }
    #[cfg(feature = "native-ui")]
    {
        threads_ui::launch(data)
    }
    #[cfg(not(feature = "native-ui"))]
    {
        let _ = data;
        Err("This build needs native-ui; headless startup is available".into())
    }
}
fn main() -> ExitCode {
    match parse(std::env::args().skip(1)).and_then(|options| match options {
        Some((data, check)) => launch(data, check),
        None => Ok(()),
    }) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn data_is_explicit_and_absolute() {
        assert!(parse(Vec::new()).is_err());
        assert!(parse(["--data-dir".into(), "relative".into()]).is_err());
        assert!(parse(["--vault".into(), "/tmp/vault".into()]).is_err());
    }
    #[test]
    fn fresh_startup_reopens_and_refuses_other_data() {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = root.path().join("fresh");
        launch(data.clone(), true).unwrap();
        launch(data.clone(), true).unwrap();
        assert!(data.join("threads.sqlite3").is_file());
        let old = root.path().join("old");
        std::fs::create_dir(&old).unwrap();
        std::fs::write(old.join("brn.sqlite"), b"synthetic old format").unwrap();
        assert!(launch(old.clone(), true).is_err());
        assert_eq!(
            std::fs::read(old.join("brn.sqlite")).unwrap(),
            b"synthetic old format"
        );
    }
}
