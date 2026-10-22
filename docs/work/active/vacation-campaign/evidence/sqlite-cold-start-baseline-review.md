# Independent cold concurrent Store-open witness review

## Verdict and exact identities

**Source approved for the selected diagnostic; adapter dispatch held for one P2 evidence-path correction below. No product fix or crash reproduction is established.** Reviewer authored neither witness nor adapters. This was a complete read-only source/adapter review; no test, build, process campaign, fixture, signal, account/native/network action, source/Git mutation or helper delegation was performed. Only this report was written.

Verified clean `codex/sqlite-cold-start-guard` in `/Users/evokessler/.codex/worktrees/export-durability/brn-rust`, HEAD `4c786a962ed52998648a0ae73232c61a0b41a5e9`, direct parent actual main211 `aded398a0c1960d1feabe48166284ec0824ee4e1`. Complete change: one new file, `crates/brn-threads-core/tests/sqlite_startup.rs`, 46 additions. Source SHA-256 `73d0eae7229c66eaaed50dcd7340f0a75480e1b3c6e246add0c09474a3391023`.

Read complete inert adapters and independently hashed:

- `/tmp/brn-sqlite-cold-start-baseline/run.sh`: `e1c2ef65a3c0dc854ff7795c7a9d164e4e98d60f1d999fe9ee83aa1398ac351c`
- `/tmp/brn-sqlite-cold-start-baseline/cold_processes.py`: `6c730bc6c78884edc20b772e452ec5f1cc5c23b94d77ebfe627a2555ccc9e9ab`
- `/tmp/brn-sqlite-cold-start-baseline/supervise.py`: `10d0ee8ae181c8838a181b5bcc735eecc2b141a6a4e32169d621c43459cf8822`

Read the author handoff, complete source diff, relevant public Store open/read implementation and crate contract, and reused the prior crash assessment and whole-owned-group supervisor review. These identities describe the initial reviewed packet and must be retained after correction.

## Required correction

**[P2] Preserve a returned child's result before rejecting its output size — `cold_processes.py:30–33`.** The child has already terminated and its raw output is written to `cold-NN.log`, but the >64 KiB rejection raises before `rows.append`. The final receipt then omits that actual attempt, including its return code and log hash; if it was the first child, the receipt reports no attempts despite a process having run. This violates the explicitly selected record-every-returncode requirement on a stop-first-invalid path. It also discards the useful distinction between an ordinary assertion failure and a native negative exit in precisely the finite campaign's failure evidence. The script exits nonzero, so this is evidence loss rather than a false qualification.

Small fix: append the completed attempt's number, actual return code, raw-log hash and byte count before enforcing the accepted-size limit; mark its result invalid/`one_test_pass:false` when oversized, then raise/stop without launching another child. Keep the raw file, explicit error, `qualified:false`, and existing native-stack classification limit. No extra test/framework or repeat campaign is needed. Dispatch should await the corrected exact adapter hash and bounded review disposition.

## Witness correctness and responsibility

The one integration-test binary has one selected test. It creates a canonical fresh temporary parent, distinct previously absent `workspace-N` paths and a 17-party barrier (16 workers plus caller). Each worker constructs its Store within its own thread after the barrier, so no connection is shared across threads and no pre-barrier SQLite call is present in the test. It then checks public fresh-workspace behavior: no records and the ordinary `Owner` purpose. These are synthetic isolated directories; no owner data is opened. All join operations are collected before any returned worker error/panic is surfaced, keeping ordinary worker failures from detaching the remaining openers. Stores drop in their worker threads and the temporary parent drops after joins.

Actual `Store::open` performs path/marker/filesystem work before `Connection::open`, so the barrier creates a concurrent opening opportunity, not proof that all 16 threads enter the same SQLite instruction simultaneously. A passing result cannot exclude an initialization race. A fresh process for each attempt is the correct boundary for attempting a cold global initialization state; this is meaningfully distinct from serially rerunning opens inside an already initialized process. The witness adds no production/configuration/dependency changes, global test-thread override, SQLite initialization mutex or recovery policy.

The core path remains the existing public Store path used below Workspace. It is a useful discriminator for the original initialization stack, while intentionally omitting app-level fixture behavior. A fault here would narrow the boundary; it would not alone establish the precise cause or equivalence of the original app crash. The earlier same-binary serial app pass remains diagnostic only.

## Campaign and cleanup assessment

The runner sources the pinned 1.98.1 locked/offline/jobs1 environment and task-specific temporary directory. Before building it verifies exact HEAD, clean tracked/untracked status, exact test source hash and >3 GiB free space. `cargo test --no-run --message-format=json` discovers the executable from the exact `sqlite_startup` source path and requires one matching artifact inside the shared target. This avoids guessing a binary path; the unique exact test name plus one-pass/zero-filtered result requirement rejects an unrelated binary. The binary is hashed, each launch is a new owned subprocess, at most 20 launches are possible, and the first nonzero or invalid result stops the loop. Source and final binary hashes are recorded. This assumes the lead's exclusive shared-target build slot throughout; no competing build/clean may replace that mutable artifact.

The inner 150-second pre-launch limit and free-space checks are supplemented by the outer 180-second deadline trigger and sampled 1 GiB floor. An individual build/test has no inner timeout; the outer whole-group supervision bounds that lifetime. Cleanup correctly probes the entire owned group even after the leader exits, then performs TERM/10 seconds, KILL/10 seconds if necessary, a separate bounded leader reap and final group-absence verification. Operational errors, unresolved ownership or changed checkout prevent accepted completion. These mechanisms reuse the previously reviewed supervisor; no source overlay/restore is involved. Polling and cleanup add bounded time beyond the nominal 180-second trigger.

Output limits are post-completion accepted-size checks, not live memory/disk quotas: compiler JSON is captured in memory before its 8 MiB check, child output before its 64 KiB check, and the outer log before its 8 MiB check. No stronger quota claim is justified. The compile JSON is saved only on successful bounded compilation; failures must still retain the outer log/receipt and be reported as incomplete, not inferred success. Receipt-write/hash/read failures produce nonzero execution rather than silently accepting a campaign.

`qualified` is always false. An actual child `-11` is recorded as a signal observation and the controller exits 101, retaining a negative outcome. The separate supervisor `diagnostic_recorded` requires a coherent compiled campaign receipt and either that signal observation or all 20 actual one-test passes, along with clean source, resolved/reaped ownership and no errors. An unexpected assertion failure or invalid result remains a preserved nonzero diagnostic, not successful qualification. Any native `-11` still requires authorized matching-stack inspection before linking it to the original `sqlite3_initialize` incident. Twenty passes would be one finite non-reproduction, not proof of safety or closure. No automatic follow-up run, dependency change or product repair is authorized by this report.
