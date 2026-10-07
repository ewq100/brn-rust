//! Owned converter process. Only copied input crosses this boundary; no vault or credentials.
use brn_intake::{Extraction, HelperRequest};
use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

pub(crate) fn extract(
    kind: &str,
    bytes: Vec<u8>,
    limits: Option<brn_intake::IntakeLimits>,
    cancel: &AtomicBool,
) -> std::result::Result<Extraction, String> {
    if cancel.load(Ordering::Acquire) {
        return Err("cancelled".into());
    }
    let executable = match std::env::var_os("BRN_INTAKE_HELPER") {
        Some(path) => std::path::PathBuf::from(path),
        None => {
            let current = std::env::current_exe().map_err(|e| e.to_string())?;
            let parent = current.parent().ok_or("executable directory unavailable")?;
            let directory = if parent.file_name().is_some_and(|name| name == "deps") {
                parent.parent().unwrap_or(parent)
            } else {
                parent
            };
            directory.join("brn-intake-helper")
        }
    };
    let metadata = std::fs::symlink_metadata(&executable).map_err(|_| {
        "maintained intake helper is unavailable; build/install brn-intake-helper".to_string()
    })?;
    if !metadata.is_file() {
        return Err("intake helper must be an ordinary executable".into());
    }
    let limits = limits.unwrap_or_default();
    limits.validate()?;
    if bytes.len() > limits.max_input_bytes {
        return Err("configured input byte budget exceeded".into());
    }
    let request = serde_json::to_vec(&HelperRequest {
        limits: Some(limits.clone()),
        kind: kind.into(),
        bytes,
    })
    .map_err(|e| e.to_string())?;
    let mut command = Command::new(executable);
    command
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let mut stdin = child.stdin.take().ok_or("helper input pipe unavailable")?;
    let stdout = child
        .stdout
        .take()
        .ok_or("helper output pipe unavailable")?;
    let (send, receive) = mpsc::sync_channel(1);
    let writer = std::thread::spawn(move || stdin.write_all(&request));
    let output_limit = limits.max_output_bytes;
    let reader = std::thread::spawn(move || {
        let mut output = Vec::new();
        let result = stdout
            .take(output_limit as u64 + 1)
            .read_to_end(&mut output)
            .map(|_| output);
        let _ = send.send(result);
    });
    let deadline = Instant::now() + Duration::from_millis(limits.wall_time_ms);
    let mut failure = None;
    let output = loop {
        if cancel.load(Ordering::Acquire) {
            failure = Some("cancelled".to_string());
            break None;
        }
        if Instant::now() >= deadline {
            failure = Some("intake helper timed out".to_string());
            break None;
        }
        match receive.recv_timeout(Duration::from_millis(20)) {
            Ok(Ok(output)) if output.len() <= output_limit => break Some(output),
            Ok(_) => {
                failure = Some("intake helper output exceeded budget or failed".to_string());
                break None;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                failure = Some("intake helper disconnected".to_string());
                break None;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    };
    let status = loop {
        if failure.is_some() || cancel.load(Ordering::Acquire) || Instant::now() >= deadline {
            failure.get_or_insert_with(|| "intake helper cancelled or timed out".into());
            #[cfg(target_os = "macos")]
            unsafe {
                libc::kill(-(child.id() as i32), libc::SIGKILL);
            }
            let _ = child.kill();
            break child.wait().map_err(|e| e.to_string())?;
        }
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let written = writer.join().map_err(|_| "helper writer failed")?;
    reader.join().map_err(|_| "helper reader failed")?;
    if let Some(failure) = failure {
        return Err(failure);
    }
    written.map_err(|e| e.to_string())?;
    // Process success is required before trusting untrusted JSON.
    if !status.success() {
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Refusal {
            error: String,
        }
        let reason = output
            .as_deref()
            .and_then(|bytes| serde_json::from_slice::<Refusal>(bytes).ok())
            .filter(|refusal| {
                refusal.error.len() <= 512 && !refusal.error.chars().any(char::is_control)
            })
            .map(|refusal| refusal.error)
            .unwrap_or_else(|| "intake helper process failed".into());
        return Err(reason);
    }
    let extraction: Extraction = serde_json::from_slice(&output.ok_or("missing helper output")?)
        .map_err(|e| e.to_string())?;
    extraction.validate()?;
    if extraction.converter != brn_intake::CONVERTER {
        return Err("unrecognized maintained converter version".into());
    }
    if extraction.limits != limits || extraction.consumed.is_none() {
        return Err("helper omitted or changed resource budget evidence".into());
    }
    Ok(extraction)
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    #[test]
    fn bounded_refusals_preserve_distinct_review_categories() {
        for (reason, expected) in [
            (
                "maintained intake helper is unavailable",
                "intake_unavailable",
            ),
            ("intake helper timed out", "intake_timeout"),
            ("configured package member quota exceeded", "intake_quota"),
            ("invalid helper JSON", "intake_protocol"),
            ("malformed DOCX archive", "intake_invalid"),
        ] {
            assert_eq!(refusal_code(reason), expected);
        }
        let limits = brn_intake::IntakeLimits {
            max_input_bytes: 1,
            ..Default::default()
        };
        let refusal =
            extract("docx", vec![0; 2], Some(limits), &AtomicBool::new(false)).unwrap_err();
        assert_eq!(refusal_code(&refusal), "intake_quota");
    }
    #[test]
    fn owned_helper_extracts_exact_public_mail_and_joins_before_returning() {
        let bytes = include_bytes!(
            "../../../experiments/architecture-reassessment/p1-office-mime/fixtures/single.eml"
        )
        .to_vec();
        let result = extract("eml", bytes.clone(), None, &AtomicBool::new(false));
        let extraction = result.expect("actual constrained helper must return usable evidence");
        assert_eq!(extraction.sources[0].bytes, bytes);
        assert!(extraction.markdown.contains("Pier B"));
    }
}

/// Persist a bounded category, never untrusted helper prose as executable markup.
pub(crate) fn refusal_code(reason: &str) -> &'static str {
    let lower = reason.to_ascii_lowercase();
    if lower.contains("unavailable") || lower.contains("ordinary executable") {
        "intake_unavailable"
    } else if lower.contains("timed out") {
        "intake_timeout"
    } else if lower.contains("quota") || lower.contains("budget") || lower.contains("limit") {
        "intake_quota"
    } else if lower.contains("json")
        || lower.contains("protocol")
        || lower.contains("scope")
        || lower.contains("converter version")
    {
        "intake_protocol"
    } else {
        "intake_invalid"
    }
}
