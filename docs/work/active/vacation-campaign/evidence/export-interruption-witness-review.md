# Independent review: export interruption witness

Repository: `/Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust`, branch `codex/export-interruption-witness`.
Reviewed candidate: `7b77eeb42577cce50748487b9bd77a0643c3737a`.
Verified base/merge base: `9e2a81c0e3d5fbc1b617fe54bdb4b103f607b822`.
The checkout was clean. The complete delta adds 115 test-only lines to `crates/brn-threads-app/src/export.rs`; reviewed file SHA-256 is `079d6c504f35b08f79ee95ae9051c8408d903abbe624fbd63e954bd4cd68ae84`.

## Finding

**[P2] Remove unnecessary mutability before strict qualification — `crates/brn-threads-app/src/export.rs:320`.**

`let mut reopened = Workspace::open(...)` is never mutably borrowed or reassigned. `Store::records` takes `&self` (`store.rs:624`), `Workspace::history` takes `&self` (`lib.rs:323`), and the public retry `Workspace::export_now` also takes `&self` (`lib.rs:583`). Rust's `unused_mut` warning therefore makes the required strict `-D warnings` test-target lint fail. Remove `mut` from this binding. Expected reproduction: the lead's normal strict `cargo clippy -p brn-threads-app --all-targets --locked --offline -- -D warnings` reports `unused_mut` at this line. This is statically verified against signatures; no compiler/lint command was launched by this reviewer. Finding was sent to the lead promptly before completing review.

No other actionable finding was identified.

## Behavioral and lifecycle review

Read the full resulting `export.rs`, not just the new hunk, relevant `Workspace` open/bootstrap/new-note/history/export callers, core row/receipt queries, export README/architecture invariant and verification guidance, AGENTS, implementation handoff, and installed independent-review/resource/Rust guidance.

The child executes the existing production export implementation through its already-present directory-flush seam. That seam runs only after all staged note/manifest files have been written, verified and file-flushed, before exclusive rename. The first callback asserts the assets-directory boundary, writes readiness inside the fresh owned temp root and sleeps. The parent waits with a five-second readiness deadline, checks early child exit, kills only the `Child` returned by its own exact-test re-execution and asserts SIGKILL. It then requires the final destination to remain absent and retries at that exact path through public `Workspace::export_now`. No production publication logic or stage policy changes are present.

The fresh workspace and export parent are siblings under a canonical temporary root. The parent drops its original workspace connection before launching the child. The re-execution selector names only this test and its child-mode root is passed solely to that child. The child does not spawn a provider or another worker. Workspace bootstrap derives a credential-location string from environment but does not read credential contents or contact an account; the new-note/export paths are local.

`OwnedChild` is created immediately around the spawned child and is dropped before the earlier-created temp root on panic/timeout. Its Drop kills and reaps the owned child, preventing the sleeping process from keeping fixture state alive when a parent assertion fails. Success also explicitly kills and waits before reopen/retry. It does not enumerate or signal unrelated processes. Readiness has an explicit deadline; the kill/reap uses ordinary blocking OS `wait`, with no separate wall-clock timeout, so this is not evidence of an absolute cleanup bound under an unresponsive kernel. No production cleanup/sweeper guarantee is inferred. An abandoned hidden stage may remain until the test's owned temp root is removed.

The expected export body comes directly from independent literal title/body inputs, including UTF-8 and newlines; the test does not call `portable_markdown` or export to derive its expected bytes. It checks the fixture note's independently captured identity/version, manifest format/count, actual Markdown bytes and SHA-256, plus matching fields in the on-disk manifest. Filename expectation reuses `safe_name`, so it is not an independent filename-sanitization witness; the generated ordinary note ID needs no transformation and naming correctness is not the selected interruption risk. Expected content bytes and hash checks still meaningfully reject truncated, altered or stale output.

All current canonical record payloads and public receipt history are compared before retry and after successful retry, using stable ordered core reads. This detects application-level source changes, not byte-identical SQLite files, forensic page retention, or an exhaustive direct SQL inventory of every internal history table. The test has one newly created note, no retained assets or external Source records; asset-rich/source-provenance recovery is not claimed.

The helper and test are entirely `cfg(unix)`; production code and existing portable flush-failure tests remain unchanged. The local fixture is small, starts one owned process, and polls readiness at 10 ms with a five-second ceiling; there is no repeated campaign/framework or unbounded readiness polling. Heavy-load and other-platform execution remain empirical qualification, not source-review proof.

## Checks and limits

`git diff --check 9e2a81c0 HEAD` passed. The author reports formatting passed, but no build/test/lint pass is established here. Qualification was pending when this frozen candidate was assigned, and this reviewer launched no build or test because the lead owns the shared target. Fix the located lint issue and finish applicable qualification before integration; a minimal mutability-only correction can be verified by the lead against this finding and the final diff/check evidence.

This adds a process-SIGKILL witness, not a production defect fix or evidence of power-loss durability, arbitrary interruption positions, CLI delivery, physical Mac interaction, native usability, owner acceptance, merge or release. The native gate remains deferred independently. No repository edits, network/native/account actions or nested work were performed. Only this requested review artifact was written. Lead owns dispositions, completed checks and integration.
