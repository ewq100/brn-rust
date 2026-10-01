//! Subprocess tests for SIGINT arriving while the command is blocked in the
//! workspace-acquisition wait: the exclusive owner lock is held in-process,
//! the child is proven (via `lsof`) to be inside `Store::open`'s bounded
//! retry loop, then SIGINT is delivered and the lock is released within the
//! remaining retry window. The requested mutation must not occur.
//! Workspaces are disposable temp dirs; no provider is involved.
use brn_workflow::{Config, Workspace};
use serde_json::Value;
use std::{
    fs,
    io::Read,
    os::unix::process::ExitStatusExt,
    path::Path,
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};
use tempfile::tempdir;

const FIXTURE: &str =
    "The synthetic Aurora mission launches on Tuesday. Its crew includes Mira and Niko.\r\n";

fn run_json(root: &Path, args: &[&str]) -> Value {
    let mut all = args.to_vec();
    all.extend_from_slice(&["--data-dir", root.to_str().unwrap(), "--json"]);
    let output = Command::new(env!("CARGO_BIN_EXE_brn"))
        .args(all)
        .stdin(Stdio::null())
        .output()
        .expect("brn binary runs");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    serde_json::from_slice(&output.stdout).expect("exactly one JSON envelope")
}

/// The two sequence tests run one at a time: both spawn children and poll
/// `lsof`, and serialization keeps observation latency inside the child's
/// bounded acquisition window.
static SEQUENCE: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Marks that the bounded wait gave up before the child exited; the guard
/// has already killed and reaped the child by the time the caller sees it.
#[derive(Debug)]
struct TimedOut;

/// Kills and reaps the child on drop unless it was already waited, so test
/// failures never leak a brn process holding or waiting on the lock.
struct ChildGuard(Option<std::process::Child>);

impl ChildGuard {
    /// True once the child has exited (and been reaped): this attempt missed
    /// the acquisition window and can be retried with a fresh child.
    fn has_exited(&mut self) -> bool {
        self.0
            .as_mut()
            .expect("child alive")
            .try_wait()
            .expect("brn child polls")
            .is_some()
    }

    fn spawn(root: &Path, args: &[&str]) -> Self {
        ChildGuard(Some(
            Command::new(env!("CARGO_BIN_EXE_brn"))
                .args(args)
                .arg("--data-dir")
                .arg(root)
                .arg("--json")
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("brn binary spawns"),
        ))
    }

    fn id(&self) -> u32 {
        self.0.as_ref().expect("child alive").id()
    }

    /// Waits the child under a bound and collects piped output. The child
    /// stays inside the guard until `try_wait` confirms it has exited, so a
    /// timeout or panic still kills and reaps the owned child on drop; it is
    /// only taken out after exit, when the kill-on-drop guard is not needed.
    fn wait_output_with_timeout(mut self, bound: Duration) -> Result<Output, TimedOut> {
        let deadline = Instant::now() + bound;
        let status = loop {
            match self
                .0
                .as_mut()
                .expect("child alive")
                .try_wait()
                .expect("brn child polls")
            {
                Some(status) => break status,
                None => {
                    if Instant::now() >= deadline {
                        // Returning drops `self`: the guard kills and reaps
                        // the still-owned child before the caller sees the
                        // error.
                        return Err(TimedOut);
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
        };
        let mut child = self.0.take().expect("child exited but not yet taken");
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        if let Some(mut s) = child.stdout.take() {
            let _ = s.read_to_end(&mut stdout);
        }
        if let Some(mut s) = child.stderr.take() {
            let _ = s.read_to_end(&mut stderr);
        }
        // `try_wait` above already reaped the child; this returns the cached
        // status and proves the child is fully collected on this path.
        let _ = child.wait();
        Ok(Output {
            status,
            stdout,
            stderr,
        })
    }

    fn wait_output(self) -> Output {
        self.wait_output_with_timeout(Duration::from_secs(15))
            .expect("brn child exits before the bound")
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            // SAFETY: kill with SIGKILL on an owned child pid only.
            unsafe { libc::kill(child.id() as libc::c_int, libc::SIGKILL) };
            let _ = child.wait();
        }
    }
}

/// Runs the full lock-wait/SIGINT/release sequence and returns the child's
/// output. Readiness is proven with `lsof`: the child provably cannot pass
/// `Store::open` while the exclusive lock is held elsewhere, so its open
/// `brn.owner.lock` descriptor proves it is inside the bounded retry loop
/// (matched by file name: lsof reports the resolved symlink path
/// /private/var/... while tempdir hands out /var/...). Under load a single
/// `lsof` sweep can be slower than the child's ~1s acquisition window; a
/// missed observation ends that attempt (the child exits WORKSPACE_BUSY on
/// its own) and a fresh child is tried — the retry is sound because the
/// child can never reach the mutation while the holder lives.
fn cancel_during_workspace_wait(
    root: &Path,
    args: &[&str],
    holder: &mut Option<Workspace>,
) -> Output {
    for _ in 0..5 {
        let mut child = ChildGuard::spawn(root, args);
        let deadline = Instant::now() + Duration::from_secs(6);
        loop {
            if let Ok(out) = Command::new("lsof")
                .arg("-p")
                .arg(child.id().to_string())
                .output()
            {
                if String::from_utf8_lossy(&out.stdout).contains("brn.owner.lock") {
                    assert_eq!(
                        // SAFETY: kill with SIGINT on an owned child pid only.
                        unsafe { libc::kill(child.id() as libc::c_int, libc::SIGINT) },
                        0,
                        "SIGINT is delivered to the waiting child"
                    );
                    // Release well inside the child's remaining retry window.
                    drop(holder.take().expect("lock holder alive"));
                    return child.wait_output();
                }
            }
            if child.has_exited() {
                break; // missed the window this attempt; retry with fresh child
            }
            assert!(
                Instant::now() < deadline,
                "acquisition wait not observed in time"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    panic!("could not observe the child inside the workspace-acquisition wait");
}

/// SIGINT during the acquisition wait, with the lock released inside the
/// retry window, must end the command without performing the import.
#[test]
fn import_cancelled_during_workspace_wait_does_not_mutate() {
    let _sequential = SEQUENCE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = tempdir().unwrap();
    let root = dir.path();
    // Hold the exclusive owner lock in this process.
    let mut holder = Some(Workspace::open(root, Config::default()).unwrap());
    let file = root.join("note.md");
    fs::write(&file, FIXTURE).unwrap();

    let out = cancel_during_workspace_wait(root, &["import", file.to_str().unwrap()], &mut holder);
    assert_eq!(
        out.status.code(),
        Some(130),
        "interrupted exit code, stdout: {} stderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.status.signal().is_none(),
        "brn must exit, not die by signal: {:?}",
        out.status.signal()
    );
    let envelope: Value =
        serde_json::from_slice(&out.stdout).expect("exactly one JSON envelope on stdout");
    assert_eq!(envelope["ok"], false, "{envelope}");
    assert_eq!(envelope["error"]["code"], "INTERRUPTED", "{envelope}");

    // `Store::import_text` creates the source and its operation in one
    // transaction, so an empty sources() is the honest observable that the
    // requested import never committed.
    let mut workspace = Workspace::open(root, Config::default()).unwrap();
    let sources = workspace.sources().unwrap();
    assert!(
        sources.is_empty(),
        "the cancelled import must not persist: {sources:?}"
    );
}

/// The same lock-wait/SIGINT/release sequence against the approval mutation:
/// the originally seeded state must survive untouched.
#[test]
fn approval_cancelled_during_workspace_wait_is_not_applied() {
    let _sequential = SEQUENCE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = tempdir().unwrap();
    let root = dir.path();
    // Seed the known state while the lock is free, then drop the workspace.
    let file = root.join("note.md");
    fs::write(&file, FIXTURE).unwrap();
    let source_id;
    {
        let mut seeder = Workspace::open(root, Config::default()).unwrap();
        let seeded = seeder
            .import_file(
                &std::sync::atomic::AtomicBool::new(false),
                uuid::Uuid::new_v4(),
                &file,
                brn_workflow::SearchApproval::Draft,
            )
            .unwrap();
        source_id = seeded.source_id;
        let version_id = seeded.version_id;
        drop(seeder);

        // Hold the exclusive owner lock in this process.
        let mut holder = Some(Workspace::open(root, Config::default()).unwrap());
        let out = cancel_during_workspace_wait(
            root,
            &[
                "documents",
                "set-search-approval",
                &source_id.to_string(),
                "--version-id",
                &version_id.to_string(),
                "--state",
                "approved",
            ],
            &mut holder,
        );
        assert_eq!(
            out.status.code(),
            Some(130),
            "interrupted exit code, stdout: {} stderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            out.status.signal().is_none(),
            "brn must exit, not die by signal: {:?}",
            out.status.signal()
        );
        let envelope: Value =
            serde_json::from_slice(&out.stdout).expect("exactly one JSON envelope on stdout");
        assert_eq!(envelope["ok"], false, "{envelope}");
        assert_eq!(envelope["error"]["code"], "INTERRUPTED", "{envelope}");
    }

    let mut workspace = Workspace::open(root, Config::default()).unwrap();
    let sources = workspace.sources().unwrap();
    assert_eq!(sources.len(), 1, "the seeded source stands: {sources:?}");
    assert_eq!(sources[0].source_id, source_id);
    assert_eq!(
        sources[0].approval,
        brn_workflow::SearchApproval::Draft,
        "the cancelled approval must not be applied"
    );
}

/// The same lock-wait/SIGINT/release sequence against the draft-creation
/// mutation: the command must be refused after acquiring the workspace and
/// before the create, leaving no draft behind.
#[test]
fn draft_create_cancelled_during_workspace_wait_does_not_mutate() {
    let _sequential = SEQUENCE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = tempdir().unwrap();
    let root = dir.path();
    let file = root.join("note.txt");
    fs::write(&file, FIXTURE).unwrap();

    // Hold the exclusive owner lock in this process.
    let mut holder = Some(Workspace::open(root, Config::default()).unwrap());
    let out = cancel_during_workspace_wait(
        root,
        &[
            "drafts",
            "create",
            "--title",
            "cancelled draft",
            "--text-file",
            file.to_str().unwrap(),
        ],
        &mut holder,
    );
    assert_eq!(
        out.status.code(),
        Some(130),
        "interrupted exit code, stdout: {} stderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.status.signal().is_none(),
        "brn must exit, not die by signal: {:?}",
        out.status.signal()
    );
    let envelope: Value =
        serde_json::from_slice(&out.stdout).expect("exactly one JSON envelope on stdout");
    assert_eq!(envelope["ok"], false, "{envelope}");
    assert_eq!(envelope["error"]["code"], "INTERRUPTED", "{envelope}");

    let workspace = Workspace::open(root, Config::default()).unwrap();
    let drafts = workspace.drafts().unwrap();
    assert!(drafts.is_empty(), "the cancelled create must not persist");
}

/// The same lock-wait/SIGINT sequence against a working-draft save must stop
/// after workspace ownership is released and before the shared save path, so
/// the original text, base and generation remain intact.
#[test]
fn draft_save_cancelled_during_workspace_wait_does_not_mutate() {
    let _sequential = SEQUENCE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = tempdir().unwrap();
    let root = dir.path();
    let mut seeder = Workspace::open(root, Config::default()).unwrap();
    let draft = seeder
        .create_draft(uuid::Uuid::new_v4(), "cancelled save", "original")
        .unwrap();
    drop(seeder);

    let file = root.join("save.txt");
    fs::write(&file, "must not save").unwrap();
    let draft_id = draft.id.to_string();
    let base = draft.stamp.base_revision.to_string();
    let generation = draft.stamp.generation.to_string();

    // Hold the exclusive owner lock in this process.
    let mut holder = Some(Workspace::open(root, Config::default()).unwrap());
    let out = cancel_during_workspace_wait(
        root,
        &[
            "drafts",
            "save",
            &draft_id,
            "--base-revision",
            &base,
            "--expected-generation",
            &generation,
            "--generation",
            "1",
            "--text-file",
            file.to_str().unwrap(),
        ],
        &mut holder,
    );
    assert_eq!(
        out.status.code(),
        Some(130),
        "interrupted exit code, stdout: {} stderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.status.signal().is_none(),
        "brn must exit, not die by signal: {:?}",
        out.status.signal()
    );
    let envelope: Value =
        serde_json::from_slice(&out.stdout).expect("exactly one JSON envelope on stdout");
    assert_eq!(envelope["ok"], false, "{envelope}");
    assert_eq!(envelope["error"]["code"], "INTERRUPTED", "{envelope}");

    let workspace = Workspace::open(root, Config::default()).unwrap();
    assert_eq!(workspace.draft(draft.id).unwrap().unwrap(), draft);
}

/// The comment capture mutation must perform the same post-acquisition
/// cancellation check as draft creation and save, so releasing the owner
/// lock after SIGINT cannot let a new comment commit.
#[test]
fn comment_add_cancelled_during_workspace_wait_does_not_mutate() {
    let _sequential = SEQUENCE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = tempdir().unwrap();
    let root = dir.path();
    let mut seeder = Workspace::open(root, Config::default()).unwrap();
    let draft = seeder
        .create_draft(
            uuid::Uuid::new_v4(),
            "cancelled comment",
            "Eesti jõgi voolab.\n",
        )
        .unwrap();
    drop(seeder);

    let text_file = root.join("comment-text.txt");
    let quote_file = root.join("comment-quote.txt");
    let body_file = root.join("comment-body.txt");
    fs::write(&text_file, "Eesti jõgi voolab.\n").unwrap();
    fs::write(&quote_file, "jõgi").unwrap();
    fs::write(&body_file, "Märkus.").unwrap();
    let draft_id = draft.id.to_string();
    let base = draft.stamp.base_revision.to_string();
    let mut holder = Some(Workspace::open(root, Config::default()).unwrap());
    let out = cancel_during_workspace_wait(
        root,
        &[
            "comments",
            "add",
            "--draft",
            &draft_id,
            "--base-revision",
            &base,
            "--expected-generation",
            "0",
            "--generation",
            "0",
            "--text-file",
            text_file.to_str().unwrap(),
            "--start-byte",
            "6",
            "--end-byte",
            "11",
            "--quote-file",
            quote_file.to_str().unwrap(),
            "--body-file",
            body_file.to_str().unwrap(),
        ],
        &mut holder,
    );
    assert_eq!(out.status.code(), Some(130), "{out:?}");
    assert!(
        out.status.signal().is_none(),
        "brn must exit, not die by signal"
    );
    let envelope: Value =
        serde_json::from_slice(&out.stdout).expect("exactly one JSON envelope on stdout");
    assert_eq!(envelope["ok"], false, "{envelope}");
    assert_eq!(envelope["error"]["code"], "INTERRUPTED", "{envelope}");

    let workspace = Workspace::open(root, Config::default()).unwrap();
    assert_eq!(workspace.draft(draft.id).unwrap().unwrap(), draft);
    assert!(workspace
        .draft_comments(draft.id)
        .unwrap()
        .comments
        .is_empty());
}

/// Resolving a comment must perform the same post-acquisition cancellation
/// check as the other comment mutation: releasing the owner lock after SIGINT
/// cannot let the status change commit.
#[test]
fn comment_resolve_cancelled_during_workspace_wait_does_not_mutate() {
    let _sequential = SEQUENCE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = tempdir().unwrap();
    let root = dir.path();
    let text_file = root.join("resolve-draft.txt");
    let quote_file = root.join("resolve-quote.txt");
    let body_file = root.join("resolve-body.txt");
    fs::write(&text_file, "Eesti jõgi voolab.\n").unwrap();
    fs::write(&quote_file, "jõgi").unwrap();
    fs::write(&body_file, "Märkus.").unwrap();

    let created = run_json(
        root,
        &[
            "drafts",
            "create",
            "--title",
            "cancelled resolve",
            "--text-file",
            text_file.to_str().unwrap(),
        ],
    );
    let draft_id = created["data"]["id"].as_str().unwrap().to_string();
    let base_revision = created["data"]["base_revision"]
        .as_str()
        .unwrap()
        .to_string();
    let added = run_json(
        root,
        &[
            "comments",
            "add",
            "--draft",
            &draft_id,
            "--base-revision",
            &base_revision,
            "--expected-generation",
            "0",
            "--generation",
            "0",
            "--text-file",
            text_file.to_str().unwrap(),
            "--start-byte",
            "6",
            "--end-byte",
            "11",
            "--quote-file",
            quote_file.to_str().unwrap(),
            "--body-file",
            body_file.to_str().unwrap(),
        ],
    );
    let comment_id = added["data"]["comment_id"].as_str().unwrap().to_string();

    let mut holder = Some(Workspace::open(root, Config::default()).unwrap());
    let out = cancel_during_workspace_wait(
        root,
        &[
            "comments",
            "resolve",
            &comment_id,
            "--draft",
            &draft_id,
            "--expected-status-version",
            "0",
        ],
        &mut holder,
    );
    assert_eq!(out.status.code(), Some(130), "{out:?}");
    assert!(
        out.status.signal().is_none(),
        "brn must exit, not die by signal"
    );
    let envelope: Value =
        serde_json::from_slice(&out.stdout).expect("exactly one JSON envelope on stdout");
    assert_eq!(envelope["ok"], false, "{envelope}");
    assert_eq!(envelope["error"]["code"], "INTERRUPTED", "{envelope}");

    let workspace = Workspace::open(root, Config::default()).unwrap();
    let comment = &workspace
        .draft_comments(draft_id.parse().unwrap())
        .unwrap()
        .comments[0]
        .comment;
    assert_eq!(comment.id, comment_id.parse::<uuid::Uuid>().unwrap());
    assert_eq!(comment.status, brn_workflow::CommentStatus::Open);
    assert_eq!(comment.status_version, 0);
}

/// Reopening a comment must perform the same post-acquisition cancellation
/// check as resolving it: releasing the owner lock after SIGINT cannot let
/// the status change commit.
#[test]
fn comment_reopen_cancelled_during_workspace_wait_does_not_mutate() {
    let _sequential = SEQUENCE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = tempdir().unwrap();
    let root = dir.path();
    let text_file = root.join("reopen-draft.txt");
    let quote_file = root.join("reopen-quote.txt");
    let body_file = root.join("reopen-body.txt");
    fs::write(&text_file, "Eesti jõgi voolab.\n").unwrap();
    fs::write(&quote_file, "jõgi").unwrap();
    fs::write(&body_file, "Märkus.").unwrap();

    let created = run_json(
        root,
        &[
            "drafts",
            "create",
            "--title",
            "cancelled reopen",
            "--text-file",
            text_file.to_str().unwrap(),
        ],
    );
    let draft_id = created["data"]["id"].as_str().unwrap().to_string();
    let base_revision = created["data"]["base_revision"]
        .as_str()
        .unwrap()
        .to_string();
    let added = run_json(
        root,
        &[
            "comments",
            "add",
            "--draft",
            &draft_id,
            "--base-revision",
            &base_revision,
            "--expected-generation",
            "0",
            "--generation",
            "0",
            "--text-file",
            text_file.to_str().unwrap(),
            "--start-byte",
            "6",
            "--end-byte",
            "11",
            "--quote-file",
            quote_file.to_str().unwrap(),
            "--body-file",
            body_file.to_str().unwrap(),
        ],
    );
    let comment_id = added["data"]["comment_id"].as_str().unwrap().to_string();
    let resolved = run_json(
        root,
        &[
            "comments",
            "resolve",
            &comment_id,
            "--draft",
            &draft_id,
            "--expected-status-version",
            "0",
        ],
    );
    assert_eq!(resolved["ok"], true, "{resolved}");

    let mut holder = Some(Workspace::open(root, Config::default()).unwrap());
    let out = cancel_during_workspace_wait(
        root,
        &[
            "comments",
            "reopen",
            &comment_id,
            "--draft",
            &draft_id,
            "--expected-status-version",
            "1",
        ],
        &mut holder,
    );
    assert_eq!(out.status.code(), Some(130), "{out:?}");
    assert!(
        out.status.signal().is_none(),
        "brn must exit, not die by signal"
    );
    let envelope: Value =
        serde_json::from_slice(&out.stdout).expect("exactly one JSON envelope on stdout");
    assert_eq!(envelope["ok"], false, "{envelope}");
    assert_eq!(envelope["error"]["code"], "INTERRUPTED", "{envelope}");

    let workspace = Workspace::open(root, Config::default()).unwrap();
    let comment = &workspace
        .draft_comments(draft_id.parse().unwrap())
        .unwrap()
        .comments[0]
        .comment;
    assert_eq!(comment.id, comment_id.parse::<uuid::Uuid>().unwrap());
    assert_eq!(comment.status, brn_workflow::CommentStatus::Resolved);
    assert_eq!(comment.status_version, 1);
}

/// The timeout path must kill and reap the owned child: after the bounded
/// wait gives up, the exact owned pid is gone (signal-0 probe on that pid
/// only; reaped, not a zombie). The test cannot leak its child: cleanup
/// happens inside `wait_output_with_timeout` before the error is returned.
#[test]
fn wait_timeout_kills_and_reaps_the_owned_child() {
    let child = Command::new("sleep")
        .arg("30")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("sleep spawns");
    let guard = ChildGuard(Some(child));
    let pid = guard.id() as libc::c_int;
    let outcome = guard.wait_output_with_timeout(Duration::from_millis(200));
    assert!(
        outcome.is_err(),
        "the non-exiting child hits the injected bound"
    );
    // SAFETY: signal-0 probe on an owned child pid only.
    assert_eq!(
        unsafe { libc::kill(pid, 0) },
        -1,
        "owned child killed, not left running"
    );
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ESRCH),
        "owned child reaped: the pid is gone, not a zombie"
    );
}
