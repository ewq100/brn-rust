//! `--NAME-file ABSOLUTE_PATH|-`: text options read from a file or stdin,
//! bounded, exact, never from a terminal, and interruptible while waiting.
use crate::contract::{CliError, Code};
use crate::Invocation;
use std::io::{IsTerminal, Read};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

const MAX_BYTES: usize = brn_intake::MAX_INPUT_BYTES;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Reason {
    Unreadable,
    IsDirectory,
    IsTerminal,
    TooLarge,
    NotUtf8,
    ContainsNul,
}
impl Reason {
    fn error(self, option: &str, stdin: bool) -> CliError {
        let (reason, message) = match self {
            Reason::Unreadable => ("unreadable", format!("--{option}: cannot read the file")),
            Reason::IsDirectory => ("is_directory", format!("--{option}: is a directory")),
            Reason::IsTerminal if stdin => {
                ("is_terminal", format!("--{option} -: stdin is a terminal"))
            }
            Reason::IsTerminal => ("is_terminal", format!("--{option}: is a terminal")),
            Reason::TooLarge => (
                "too_large",
                format!("--{option}: larger than {MAX_BYTES} bytes"),
            ),
            Reason::NotUtf8 => ("not_utf8", format!("--{option}: not valid UTF-8")),
            Reason::ContainsNul => ("contains_nul", format!("--{option}: contains NUL")),
        };
        let error = CliError::invalid_input(option, reason, message);
        if self == Reason::TooLarge {
            error.with("max_bytes", MAX_BYTES)
        } else {
            error
        }
    }
}

/// The most bytes an edit token file may hold: the token and one newline.
const TOKEN_MAX_BYTES: usize = 128;

/// Reads every pending `--NAME-file` source, in TEXT_OPTIONS order, into
/// `--NAME`, then the `--token-file` token into `inv.token` (never `args`).
pub(crate) fn resolve(inv: &mut Invocation, cancel: &AtomicBool) -> Result<(), CliError> {
    for (name, source) in std::mem::take(&mut inv.pending_inputs) {
        let text = read(&format!("{name}-file"), source, cancel)?;
        inv.args.insert(name.to_owned(), text);
    }
    if let Some(source) = inv.token_source.take() {
        inv.token = Some(read_token(source, cancel)?);
    }
    Ok(())
}

/// `--token-file`: at most 128 bytes, one trailing newline trimmed, then the
/// token pattern. No message or detail repeats any of the bytes read.
fn read_token(source: String, cancel: &AtomicBool) -> Result<brn_threads_app::EditToken, CliError> {
    let option = "token-file";
    let stdin = source == "-";
    let format = || {
        CliError::invalid_input(
            option,
            "token_format",
            "--token-file: not an edit token (brnet1_ and 64 lowercase hex digits)",
        )
    };
    match read_limited(source, TOKEN_MAX_BYTES, cancel)? {
        Ok(text) => brn_threads_app::EditToken::parse(text.strip_suffix('\n').unwrap_or(&text))
            .ok_or_else(format),
        Err(Reason::IsTerminal) => Err(Reason::IsTerminal.error(option, stdin)),
        Err(Reason::Unreadable | Reason::IsDirectory) => {
            Err(Reason::Unreadable.error(option, stdin))
        }
        Err(_) => Err(format()),
    }
}

fn read(option: &str, source: String, cancel: &AtomicBool) -> Result<String, CliError> {
    let stdin = source == "-";
    read_limited(source, MAX_BYTES, cancel)?.map_err(|reason| reason.error(option, stdin))
}

/// Reads on a detached thread so a blocked `read` or FIFO `open` cannot hide
/// SIGINT: the handler restarts interrupted system calls.
fn read_limited(
    source: String,
    max: usize,
    cancel: &AtomicBool,
) -> Result<Result<String, Reason>, CliError> {
    if cancel.load(Ordering::SeqCst) {
        return Err(CliError::interrupted());
    }
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(read_bounded_to(&source, max));
    });
    loop {
        match receiver.recv_timeout(Duration::from_millis(25)) {
            Ok(result) => return Ok(result),
            Err(RecvTimeoutError::Timeout) if cancel.load(Ordering::SeqCst) => {
                return Err(CliError::interrupted());
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                return Err(CliError::new(
                    Code::Internal,
                    "Input reader stopped without a result",
                ));
            }
        }
    }
}

/// The exact text of `-` (stdin) or an absolute path, at most 16 MiB.
#[cfg(test)]
pub(crate) fn read_bounded(source: &str) -> Result<String, Reason> {
    read_bounded_to(source, MAX_BYTES)
}

/// The exact text of `-` (stdin) or an absolute path, at most `max` bytes.
fn read_bounded_to(source: &str, max: usize) -> Result<String, Reason> {
    let mut bytes = Vec::new();
    let limit = max as u64 + 1;
    if source == "-" {
        let stdin = std::io::stdin();
        if stdin.is_terminal() {
            return Err(Reason::IsTerminal);
        }
        stdin
            .lock()
            .take(limit)
            .read_to_end(&mut bytes)
            .map_err(|_| Reason::Unreadable)?;
    } else {
        // Metadata first: it follows symlinks without blocking on a FIFO.
        let metadata = std::fs::metadata(source).map_err(|_| Reason::Unreadable)?;
        if metadata.is_dir() {
            return Err(Reason::IsDirectory);
        }
        let file = std::fs::File::open(source).map_err(|_| Reason::Unreadable)?;
        if file.is_terminal() {
            return Err(Reason::IsTerminal);
        }
        file.take(limit)
            .read_to_end(&mut bytes)
            .map_err(|_| Reason::Unreadable)?;
    }
    if bytes.len() > max {
        return Err(Reason::TooLarge);
    }
    let text = String::from_utf8(bytes).map_err(|_| Reason::NotUtf8)?;
    if text.contains('\0') {
        return Err(Reason::ContainsNul);
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn file(dir: &tempfile::TempDir, name: &str, bytes: &[u8]) -> String {
        let path = dir.path().join(name);
        std::fs::write(&path, bytes).unwrap();
        path.to_str().unwrap().to_owned()
    }

    #[test]
    fn the_limit_is_inclusive_and_one_more_byte_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let at = file(&dir, "at", &vec![b'a'; MAX_BYTES]);
        assert_eq!(read_bounded(&at).unwrap().len(), MAX_BYTES);
        let over = file(&dir, "over", &vec![b'a'; MAX_BYTES + 1]);
        assert_eq!(read_bounded(&over), Err(Reason::TooLarge));
        let error = Reason::TooLarge.error("text-file", false);
        assert_eq!(error.details["max_bytes"], MAX_BYTES);
        assert_eq!(error.message, "--text-file: larger than 16777216 bytes");
    }

    #[test]
    fn content_is_exact_utf8_without_nul() {
        let dir = tempfile::tempdir().unwrap();
        let exact = "\u{feff}line\r\n\ttab $HOME `x`\n";
        assert_eq!(
            read_bounded(&file(&dir, "exact", exact.as_bytes())).unwrap(),
            exact
        );
        assert_eq!(read_bounded(&file(&dir, "empty", b"")).unwrap(), "");
        assert_eq!(
            read_bounded(&file(&dir, "nul", b"a\0b")),
            Err(Reason::ContainsNul)
        );
        assert_eq!(
            read_bounded(&file(&dir, "bad", b"\xff\xfe")),
            Err(Reason::NotUtf8)
        );
        assert_eq!(
            read_bounded(dir.path().to_str().unwrap()),
            Err(Reason::IsDirectory)
        );
        assert_eq!(
            read_bounded(dir.path().join("absent").to_str().unwrap()),
            Err(Reason::Unreadable)
        );
    }

    #[test]
    fn a_set_cancel_flag_interrupts_before_reading() {
        let cancel = AtomicBool::new(true);
        let error = read("text-file", "/nonexistent/brn-input".into(), &cancel).unwrap_err();
        assert_eq!(error.code, Code::Interrupted);
        assert_eq!(error.message, "Interrupted before operation");
    }

    fn parse(args: &[&str]) -> CliError {
        let args = ["--json", "--data-dir", "/nonexistent/brn-input-data"]
            .iter()
            .chain(args)
            .map(OsString::from);
        match crate::parse(args) {
            Err(refused) => refused.error,
            Ok(_) => panic!("the invocation parsed"),
        }
    }

    #[test]
    fn relative_paths_and_a_reused_stdin_are_refused_at_parse() {
        let relative = parse(&["note-create", "--title", "T", "--text-file", "notes.md"]);
        assert_eq!(relative.code, Code::InvalidValue);
        assert_eq!(relative.details["option"], "text-file");
        assert_eq!(relative.message, "--text-file takes an absolute path or -");
        let reused = parse(&["note-create", "--title-file", "-", "--text-file", "-"]);
        assert_eq!(reused.code, Code::InvalidInput);
        assert_eq!(reused.details["option"], "text-file");
        assert_eq!(reused.details["reason"], "stdin_reused");
        assert_eq!(
            reused.message,
            "--text-file -: stdin is already read by --title-file"
        );
    }
}
