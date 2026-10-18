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
    // Explicit paths avoid PATH selection and shell interpolation; the host owns
    // executable selection. The document helper installs its own restriction.
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

#[cfg(all(
    test,
    any(target_os = "macos", all(target_os = "linux", target_arch = "x86_64"))
))]
mod tests {
    use super::*;
    use std::sync::Arc;

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
