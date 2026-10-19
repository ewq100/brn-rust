use brn_intake::{MAX_OUTPUT_BYTES, MAX_REQUEST_BYTES};
use std::io::{Read, Write};

pub fn main_with(handler: fn(bool, &[u8]) -> Result<Vec<u8>, String>) {
    if let Err(error) = run(handler) {
        let response = serde_json::json!({"error":error});
        let _ = serde_json::to_writer(std::io::stdout().lock(), &response);
        std::process::exit(1);
    }
}
fn run(handler: fn(bool, &[u8]) -> Result<Vec<u8>, String>) -> Result<(), String> {
    // Runtime starts first, as qualified by P1. Restrict before reading any input or
    // parsing a document; no profile/input paths or inherited credentials are needed.
    let args: Vec<_> = std::env::args().collect();
    // The fork witness never parses a document. Production conversion admits
    // worker threads but cannot create additional processes on Linux.
    let process_tree_witness = args.len() == 2 && args[1] == "--self-test-process-tree";
    restrictions(process_tree_witness)?;
    if std::env::vars_os().next().is_some() {
        return Err("helper requires an empty inherited environment".into());
    }
    if args.get(1).map(String::as_str) == Some("--self-test-restrictions") {
        let path = args
            .get(2)
            .ok_or("restriction test requires synthetic forbidden path")?;
        let port: u16 = args
            .get(3)
            .ok_or("restriction test requires live loopback port")?
            .parse()
            .map_err(|_| "invalid port")?;
        let denied_file = std::fs::read(path).is_err();
        let denied_network =
            std::net::TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)).is_err();
        let result = serde_json::json!({"file_denied":denied_file,"network_denied":denied_network});
        serde_json::to_writer(std::io::stdout().lock(), &result).map_err(|e| e.to_string())?;
        return if denied_file && denied_network {
            Ok(())
        } else {
            Err("restriction witness failed".into())
        };
    }
    #[cfg(unix)]
    if process_tree_witness {
        for _ in 0..2 {
            // Only the child continues spawning; the test host owns/kills the group.
            if unsafe { libc::fork() } != 0 {
                break;
            }
        }
        println!("{}", std::process::id());
        std::io::stdout().flush().map_err(|e| e.to_string())?;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
    let prepare = args.len() == 2 && args[1] == "--prepare";
    if args.len() != 1 && !prepare {
        return Err("unexpected helper arguments".into());
    }
    let mut input = Vec::new();
    std::io::stdin()
        .lock()
        .take(MAX_REQUEST_BYTES as u64 + 1)
        .read_to_end(&mut input)
        .map_err(|e| e.to_string())?;
    if input.len() > MAX_REQUEST_BYTES {
        return Err("request protocol budget".into());
    }
    let bytes = handler(prepare, &input)?;
    if bytes.len() > MAX_OUTPUT_BYTES {
        return Err("output protocol budget".into());
    }
    std::io::stdout()
        .lock()
        .write_all(&bytes)
        .map_err(|e| e.to_string())
}

#[cfg(target_os = "macos")]
fn restrictions(_process_tree_witness: bool) -> Result<(), String> {
    #[link(name = "sandbox")]
    unsafe extern "C" {
        fn sandbox_init(
            profile: *const std::ffi::c_char,
            flags: u64,
            error: *mut *mut std::ffi::c_char,
        ) -> i32;
        fn sandbox_free_error(error: *mut std::ffi::c_char);
    }
    // No input/temp/vault/home allowance: data arrives solely over inherited pipes.
    // System runtime reads are the bounded P1 working sequence, with no file writes
    // except already-open protocol descriptors and /dev/null; all network is denied.
    let profile = std::ffi::CString::new(r#"(version 1)
(deny default)
(allow process-fork)
(allow sysctl-read)
(allow file-map-executable (subpath "/System/Library") (subpath "/usr/lib"))
(allow file-read* (subpath "/System/Library") (subpath "/System/Volumes/Preboot") (subpath "/private/preboot") (subpath "/usr/lib") (subpath "/private/var/db/dyld"))
(allow file-read* file-write* (literal "/dev/null"))
"#).map_err(|e| e.to_string())?;
    let mut error = std::ptr::null_mut();
    // SAFETY: the profile lives through the call and the native error is copied and freed.
    if unsafe { sandbox_init(profile.as_ptr(), 0, &mut error) } != 0 {
        let message = if error.is_null() {
            "native restriction activation failed".into()
        } else {
            let message = unsafe { std::ffi::CStr::from_ptr(error) }
                .to_string_lossy()
                .into_owned();
            unsafe { sandbox_free_error(error) };
            message
        };
        return Err(message);
    }
    // Put finite process-level CPU/file-descriptor/output limits in place. The host
    // additionally owns wall time, bounded pipes, process group and environment.
    for (resource, limit) in [
        (libc::RLIMIT_CPU, 10),
        (libc::RLIMIT_NOFILE, 64),
        (libc::RLIMIT_FSIZE, MAX_OUTPUT_BYTES as u64),
    ] {
        let limits = libc::rlimit {
            rlim_cur: limit,
            rlim_max: limit,
        };
        if unsafe { libc::setrlimit(resource, &limits) } != 0 {
            return Err("helper resource restriction activation failed".into());
        }
    }
    Ok(())
}
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "helper_runtime_linux.rs"]
mod linux;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn restrictions(process_tree_witness: bool) -> Result<(), String> {
    linux::restrict(process_tree_witness)
}
#[cfg(not(any(target_os = "macos", all(target_os = "linux", target_arch = "x86_64"))))]
fn restrictions(_process_tree_witness: bool) -> Result<(), String> {
    Err("native intake restrictions require macOS or Linux x86_64; conversion refused".into())
}
