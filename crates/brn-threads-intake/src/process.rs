use crate::{ConversionError, transient::TransientDir};
use std::{
    ffi::OsString,
    io::{Read, Write},
    path::Path,
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

pub(crate) struct ProcessBudget {
    pub output_bytes: usize,
    pub storage_bytes: usize,
    pub deadline: Instant,
}

/// Owned process group, bounded pipes, empty environment. Stderr is never retained.
pub(crate) fn run(
    executable: &Path,
    args: &[OsString],
    input: Option<Vec<u8>>,
    temp: &TransientDir,
    budget: ProcessBudget,
    cancel: &AtomicBool,
    restrict: bool,
) -> Result<Vec<u8>, ConversionError> {
    let ProcessBudget {
        output_bytes: output_limit,
        storage_bytes: storage_limit,
        deadline,
    } = budget;
    if cancel.load(Ordering::Acquire) {
        return Err(ConversionError::Cancelled);
    }
    if Instant::now() >= deadline {
        return Err(ConversionError::Timeout);
    }
    // Explicit paths avoid PATH selection and shell interpolation. Homebrew symlinks
    // are resolved before sandboxing; the caller owns converter configuration.
    if !executable.is_absolute() {
        return Err(ConversionError::Unavailable);
    }
    let executable = std::fs::canonicalize(executable).map_err(|_| ConversionError::Unavailable)?;
    if !std::fs::symlink_metadata(&executable)
        .map_err(|_| ConversionError::Unavailable)?
        .is_file()
    {
        return Err(ConversionError::Unavailable);
    }
    let mut command = Command::new(&executable);
    command
        .args(args)
        .env_clear()
        .current_dir(&temp.path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    if restrict {
        restrict_command(&mut command, &executable, &temp.path, storage_limit)?;
    }
    let mut child = command.spawn().map_err(|_| ConversionError::Unavailable)?;
    let mut stdin = child.stdin.take().ok_or(ConversionError::Io)?;
    let stdout = child.stdout.take().ok_or(ConversionError::Io)?;
    let writer = std::thread::spawn(move || {
        if let Some(input) = input {
            stdin.write_all(&input)
        } else {
            Ok(())
        }
    });
    let (send, receive) = mpsc::sync_channel(1);
    let reader = std::thread::spawn(move || {
        let mut output = Vec::new();
        let result = stdout
            .take(output_limit as u64 + 1)
            .read_to_end(&mut output)
            .map(|_| output);
        let _ = send.send(result);
    });
    let mut output = None;
    let mut failure = None;
    let status = loop {
        if cancel.load(Ordering::Acquire) {
            failure = Some(ConversionError::Cancelled);
        } else if Instant::now() >= deadline {
            failure = Some(ConversionError::Timeout);
        } else if let Err(error) = temp.check_budget(storage_limit) {
            failure = Some(error);
        }
        if failure.is_none() && output.is_none() {
            match receive.try_recv() {
                Ok(Ok(bytes)) if bytes.len() <= output_limit => output = Some(bytes),
                Ok(_) => failure = Some(ConversionError::Budget),
                Err(mpsc::TryRecvError::Disconnected) => failure = Some(ConversionError::Io),
                Err(mpsc::TryRecvError::Empty) => (),
            }
        }
        if failure.is_some() {
            kill_group(&mut child);
            break child.wait().map_err(|_| ConversionError::Io);
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                kill_group(&mut child);
                break Ok(status);
            }
            Ok(None) => (),
            Err(_) => {
                failure = Some(ConversionError::Io);
                kill_group(&mut child);
                break child.wait().map_err(|_| ConversionError::Io);
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let written = writer.join().map_err(|_| ConversionError::Io)?;
    reader.join().map_err(|_| ConversionError::Io)?;
    if let Some(error) = failure {
        return Err(error);
    }
    let output = output
        .or_else(|| receive.try_recv().ok().and_then(Result::ok))
        .ok_or(ConversionError::Io)?;
    if output.len() > output_limit {
        return Err(ConversionError::Budget);
    }
    if !status?.success() {
        // Preserve bounded helper quota refusals as a category. Never return or
        // log the untrusted helper's message/raw response to the host.
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Refusal {
            error: String,
        }
        if output.len() <= 1024
            && let Ok(refusal) = serde_json::from_slice::<Refusal>(&output)
        {
            let reason = refusal.error.to_ascii_lowercase();
            if reason.contains("quota") || reason.contains("budget") || reason.contains("limit") {
                return Err(ConversionError::Budget);
            }
        }
        return Err(ConversionError::Invalid);
    }
    written.map_err(|_| ConversionError::Io)?;
    temp.check_budget(storage_limit)?;
    Ok(output)
}
fn kill_group(child: &mut std::process::Child) {
    #[cfg(unix)]
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    let _ = child.kill();
}

#[cfg(target_os = "macos")]
fn restrict_command(
    command: &mut Command,
    executable: &Path,
    temp: &Path,
    output_limit: usize,
) -> Result<(), ConversionError> {
    use std::os::unix::process::CommandExt;
    fn escaped(path: &Path) -> Result<String, ConversionError> {
        let path = path.to_str().ok_or(ConversionError::Invalid)?;
        if path.chars().any(char::is_control) {
            return Err(ConversionError::Invalid);
        }
        Ok(path.replace('\\', "\\\\").replace('"', "\\\""))
    }
    let profile = std::ffi::CString::new(format!(r#"(version 1)
(deny default)
(allow process-exec (literal "{}"))
(allow sysctl-read)
(allow file-read* (literal "/"))
(allow file-read-metadata (subpath "/opt/homebrew"))
(allow file-read* file-map-executable (subpath "/System/Library") (subpath "/System/Volumes/Preboot") (subpath "/private/preboot") (subpath "/usr/lib") (subpath "/private/var/db/dyld") (subpath "/usr/share") (subpath "/Library/Fonts") (subpath "/opt/homebrew/Cellar") (subpath "/opt/homebrew/lib") (subpath "/opt/homebrew/share") (literal "{}"))
(allow file-read* file-write* (subpath "{}") (literal "/dev/null"))
"#, escaped(executable)?, escaped(executable)?, escaped(temp)?)).map_err(|_| ConversionError::Invalid)?;
    #[link(name = "sandbox")]
    unsafe extern "C" {
        fn sandbox_init(
            profile: *const std::ffi::c_char,
            flags: u64,
            error: *mut *mut std::ffi::c_char,
        ) -> i32;
    }
    // macOS ignition reads root directory/xattrs during exec; Homebrew dylib
    // symlinks need metadata beneath its prefix. Neither grants content reads
    // beneath the filesystem root, user directories or the BRN store.
    // Only native calls in the forked child; no document parsing here.
    unsafe {
        command.pre_exec(move || {
            if sandbox_init(profile.as_ptr(), 0, std::ptr::null_mut()) != 0 {
                return Err(std::io::Error::from_raw_os_error(libc::EPERM));
            }
            for (resource, limit) in [
                (libc::RLIMIT_CPU, 30u64),
                (libc::RLIMIT_NOFILE, 64),
                (libc::RLIMIT_FSIZE, output_limit as u64),
            ] {
                let limits = libc::rlimit {
                    rlim_cur: limit,
                    rlim_max: limit,
                };
                if libc::setrlimit(resource, &limits) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
            }
            Ok(())
        });
    }
    Ok(())
}
#[cfg(not(target_os = "macos"))]
fn restrict_command(_: &mut Command, _: &Path, _: &Path, _: usize) -> Result<(), ConversionError> {
    Err(ConversionError::Unavailable)
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn sandbox_denies_external_source_and_network_before_parsing() {
        let root = tempfile::tempdir().unwrap();
        let temp = TransientDir::create(root.path()).unwrap();
        let forbidden = root.path().join("external-source");
        std::fs::write(&forbidden, b"synthetic-excluded-canary").unwrap();
        let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        std::fs::write(
            temp.path.join("probe"),
            format!("{}\n{port}", forbidden.display()),
        )
        .unwrap();
        let args = [
            "--ignored",
            "--exact",
            "process::tests::sandbox_probe_child",
            "--nocapture",
        ]
        .map(OsString::from);
        run(
            &std::env::current_exe().unwrap(),
            &args,
            None,
            &temp,
            ProcessBudget {
                output_bytes: 4096,
                storage_bytes: 8192,
                deadline: Instant::now() + Duration::from_secs(5),
            },
            &AtomicBool::new(false),
            true,
        )
        .unwrap();
    }
    #[test]
    #[ignore = "subprocess witness only"]
    fn sandbox_probe_child() {
        let probe = std::fs::read_to_string("probe").unwrap();
        let mut lines = probe.lines();
        assert!(std::fs::read(lines.next().unwrap()).is_err());
        let port: u16 = lines.next().unwrap().parse().unwrap();
        assert!(std::net::TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)).is_err());
        assert_eq!(std::env::vars_os().count(), 0);
    }
    #[test]
    fn cancellation_kills_owned_descendant_and_joins_pipes() {
        let root = tempfile::tempdir().unwrap();
        let temp = TransientDir::create(root.path()).unwrap();
        let marker = temp.path.join("descendant");
        let cancel = Arc::new(AtomicBool::new(false));
        let cancelled = cancel.clone();
        let marker_for_thread = marker.clone();
        let signal = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(2);
            while !marker_for_thread.exists() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(10));
            }
            cancelled.store(true, Ordering::Release);
        });
        let args = [
            "--ignored",
            "--exact",
            "process::tests::process_tree_child",
            "--nocapture",
        ]
        .map(OsString::from);
        let result = run(
            &std::env::current_exe().unwrap(),
            &args,
            None,
            &temp,
            ProcessBudget {
                output_bytes: 4096,
                storage_bytes: 8192,
                deadline: Instant::now() + Duration::from_secs(5),
            },
            &cancel,
            false,
        );
        signal.join().unwrap();
        assert_eq!(result, Err(ConversionError::Cancelled));
        let pid: i32 = std::fs::read_to_string(marker).unwrap().parse().unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while unsafe { libc::kill(pid, 0) } == 0 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(
            unsafe { libc::kill(pid, 0) },
            -1,
            "owned descendant remains alive"
        );
    }
    #[test]
    #[ignore = "subprocess witness only"]
    fn process_tree_child() {
        let pid = unsafe { libc::fork() };
        assert!(pid >= 0);
        if pid > 0 {
            std::fs::write("descendant", pid.to_string()).unwrap();
        }
        loop {
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
