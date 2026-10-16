# CLI agent contract implementation plan

**Assignment.** The owner asked (15 October 2026) for a BRN CLI that coding agents, automated tests and diagnostic scripts can use without guessing, parsing prose or opening SQLite: audit a representative journey, rank the friction, implement the three highest-value bounded improvements, add process-level tests and a maintained journey runner, and report before/after examples, the machine contract, verification results and remaining gaps. No owner data, no publish, no merge, no release.

**Design authority.** This plan condenses the gate-approved design (iteration 6). The design, its review verdict, the audit (S001–S225) and its addendum (S226–S299) are task artifacts kept outside the repository (`design.md`, `design-review.json`, `audit.md`, `audit-addendum.md`). "Design §X" names the section that supplied exact wording during implementation; the [evidence](evidence.md) records what was built and verified.

**Worktree.** An isolated worktree `.worktrees/cli-agent-contract` on branch `work/cli-agent-contract`. Commit locally in reviewable units. Never push, open a PR, merge, or rebase/modify `main` or any other branch.

**Status.** Steps 1–13 are done and independently reviewed (two reviewers, both approved). The final fail-closed verification on the candidate is recorded in the [evidence](evidence.md#final-verification). Not merged; owner acceptance not requested; native observation not applicable.

## Scope

The three improvements (design §1 ranks them):

- **I1, machine contract mode.** A leading `--json` makes every invocation print exactly one JSON envelope on stdout, with a fixed status/exit table, stable error codes, per-command `outcome`/`operation`/`changed`/`retry`, `conflict` summaries for stale/deferred outcomes, and `brn --json --help` as a machine description that marks owner decisions.
- **I2, file and stdin text input.** `--NAME-file ABSOLUTE_PATH|-` for `title`, `text`, `prompt`, `instructions`, `actions`; bytes used exactly, bounded, read before the workspace opens, SIGINT observed while waiting.
- **I3, discoverable retry identity.** `--key KEY` on every replayable write that mints a random key today, with per-command retry classes (`read`, `replay`, `converge`, `refuse`, `duplicate`) reported in every envelope and in the description.

Plus verification deliverables: `crates/brn/tests/agent_contract.rs`, `scripts/cli_journey.py` with `scripts/tests/test_cli_journey.py`, wired into `scripts/verify-end-to-end.sh`; README "Agents and scripts" section; a scratch test-merge with `work/daily-workflow-reliability`.

## Hard constraints (check every item against these)

- CLI stays thin over the one shared checked service (`brn_threads_app::Workspace` and the `workspace.store` calls `main.rs` already makes). The CLI never builds a `ChangeRequest` or `HostAuthority`. No new protocol, server, daemon, RPC or automation framework.
- No safeguard bypass, no `--force`, no seeding/fixture/raw-SQL command. Version checks, dirty guards, protection, exact-candidate review, replay, project boundaries and Undo are untouched (core is not edited). CLI access grants no owner authority; `owner_decision` marks are documentation only.
- **Only shared-service change:** `crates/brn-threads-app/src/lib.rs` gains three keyed functions (existing `new_note`/`new_thread`/`undo` become one-line wrappers) and one re-export line. No file under `crates/brn-threads-core` or `crates/brn-desktop` changes.
- Default (non-`--json`) stdout, stderr text and exit statuses stay identical for every invocation that works today, except the I1(d) list (design §4(d); summarised in step 5). In particular: `threads.rs::json_command` needs exit 0 on a Stale undo; tests assert stderr substrings; witnesses treat nonzero as failure; `live-journeys.py` expects an interrupted `ask` to exit 0 with terminal `Interrupted`; CI runs `brn --help` (exit 0) and `scripts/verify-storage.sh`, which needs nonzero for no args and `--unknown`; CI's macOS fixture step sets `TMPDIR` to a private parent removed by `rmdir`.
- Anti-conflict rules with `work/daily-workflow-reliability` (design §10): do **not** edit the `HELP` constant (append `AGENT_HELP` at print time); do **not** edit the useful-intake sub-block of the `intake` arm (the `else` branch ending in `return ask(…)`); `ask` keeps its signature and `Result<Value>` return; `execute` keeps returning `Result<Value>`; insert `fill_conflicts` before the final `Ok(result)` of `execute`; leave `#[tokio::main]`, `async fn main() -> ExitCode {` and the SIGINT handler block untouched; keep the text "Choose either --project or --thread"; put new tests in a new file; reuse `edit_session_open` with details `{note, session, base_version}`.
- Do not adopt clap (H5 decision); the hand-written scanner stays, its `allowed` match becomes a static table.
- Toolchain from `rust-toolchain.toml` (1.98.1); build `--locked --offline`; no new crates or dependency edges; `git diff --exit-code Cargo.lock` stays clean. Python runner and tests use the stdlib only.
- Synthetic data in fresh explicit temp directories only; no live provider calls (no `ask` that reaches a provider, no non-Markdown intake in tests or the runner); `HOME`/`CODEX_HOME`/`TMPDIR`/`TMP`/`TEMP` of every child point inside the test root; no `BRN_*` variables in children.

## Implementation plan

- [x] 1. Sync the branch and record the baseline.
      The branch has no task commits and `HEAD` (`771116f0`) is an ancestor of `origin/main` (`8bef439b`); fast-forward with `git merge --ff-only origin/main` in the worktree. Confirm `git status --porcelain` is empty. Record baseline results (tree hash, commands, pass counts) in `docs/work/active/cli-agent-contract/evidence.md` (new). Note that `work/daily-workflow-reliability` is now `b3fc584a` (docs-only after the design's `20f73edb`; `git diff 20f73edb b3fc584a` touches only its `evidence.md`).
      Files: `docs/work/active/cli-agent-contract/evidence.md`
      Verify: `cargo build --workspace --locked --offline` succeeds; `cargo test -p brn --locked --offline` passes (11 tests in `tests/threads.rs`); `python3 -m unittest discover -s scripts/tests -v` passes.

- [x] 2. Add the keyed shared-service functions and the retrieval re-export (design §6(b)2, §4(b)2).
      In `crates/brn-threads-app/src/lib.rs`:
      - `pub fn new_note_with_key(&mut self, key: &str, title: &str, markdown: &str) -> AppResult<Receipt>`: today's `new_note` body with `uuid::Uuid::new_v4()` replaced by `key` (host key `new-note:{key}`, actor `HostAuthority::owner("owner")`), returning `applied_receipt(...)`.
      - `pub fn new_thread_with_key(&mut self, key: &str, title: &str) -> AppResult<Receipt>`: same pattern, `new-thread:{key}`.
      - `pub fn undo_with_key(&mut self, key: &str, id: &str) -> AppResult<(OperationId, ApplyOutcome)>`: `owner-undo:{key}`, actor `"owner Undo"`, returns the compensation id with the outcome.
      - `new_note` becomes `applied_record`-equivalent over `new_note_with_key(&Uuid::new_v4().to_string(), …)` (extract `writes[0].after`, same "Empty receipt" error); `new_thread` likewise; `undo` returns `.1` of `undo_with_key(&uuid, id)`. Host keys and behavior for desktop and every other caller stay byte-identical.
      - Add `pub use brn_retrieval::Error as RetrievalError;` beside the other `pub use` lines.
      Add `crates/brn-threads-app/tests/keyed_owner_operations.rs` (pattern: other files in that `tests/` folder): identical keyed `new_note_with_key`/`new_thread_with_key` replay the same receipt and add no record; same key with different content gives `Error::ImmutableRequest` (through `AppError::Core`); `undo_with_key` on an operation whose note has an open edit session is Deferred, then after `discard` of that session the same key returns Applied with the same `OperationId`, and a third call replays it; unkeyed `new_note`, `new_thread`, `undo` still produce distinct operations per call.
      Files: `crates/brn-threads-app/src/lib.rs`, `crates/brn-threads-app/tests/keyed_owner_operations.rs`
      Verify: `cargo test -p brn-threads-app --locked --offline` passes, including the new file; `cargo build --workspace --locked --offline` succeeds (desktop unchanged).

- [x] 3. Add the static command table `crates/brn/src/spec.rs` (design §7.1 first row, §4.5, §4.6, §6(c), required-order table in §7.1).
      Types: `Spec { name, alias_of, options, required, required_when, keyed, owner: Owner, retry: Retry, outcomes, receipt: ReceiptAt }`, `RequiredWhen { option, equals, required }`, `Owner { No, Yes, WithOptions(&[..]) }`, `Retry { default: Class, keyed: Option<Class>, variants }`, `Variant { option, equals, retry: Class, key: bool, receipt: Option<ReceiptAt> }`, `Class { Read, Replay, Converge, Refuse, Duplicate }`, `ReceiptAt { None, Root, Field, Arm, NoteAndPointer, Terminal }`. One entry per command currently in `parse()`'s `allowed` match (including `resolve` with `alias_of: Some("archive")`), with `options` equal to today's lists plus `key` on keyed commands and `NAME-file` for each text option the command allows. `outcomes` built from label constants `APPLY_OUTCOMES = [Applied, Stale, Deferred, NeedsReview, Superseded]`, `SAVE_OUTCOMES = [Saved, Stale, GenerationMismatch, Closed, Deferred]`, `TERMINAL_OUTCOMES = [Completed, Interrupted, Failed]`, etc. (table 4.2b last column; `intake` = APPLY then TERMINAL).
      Functions: `spec(name)`, `retry_for(&Invocation) -> Class` (first matching variant, else `keyed` if `--key` present, else `default`), `receipt_for(&Invocation)`, `key_refused(&Invocation) -> Option<&Variant>` (only for `keyed: true` commands), `description() -> Value` (shape in design §4.5: `contract`, `usage`, `statuses`, `errors`, `retry_classes`, `owner_decision_rule`, `input`, `key.pattern = "^[A-Za-z0-9._:-]{1,128}$"`, `commands[]` with `name, alias_of, options, required, required_when, key, owner_decision, owner_decision_options, retry{default,keyed,variants}, outcomes`). `statuses` and `errors` are filled from `contract.rs` in step 4, so for this step `description()` may take them as parameters or the step-4 types may be introduced here; keep the data in one place.
      Key data to encode:
      - **Retry table** (design §6(c)): reads and `project-delete-preview`, `recovery` → read; `knowledge-check` read with variant `seed-chat=yes` → duplicate, `key:false`; `init` converge; `topics-create` duplicate; `topics-choose`, `configure`, `delegation`, `pause`, `resume`, `upgrade` converge; `project-create`/`project-chat`/`project-document` duplicate/keyed replay; `project-settings`/`project-delete`/`project-restore` refuse/keyed replay; `chat-move`/`project-link`/`project-unlink` converge/keyed replay; `note-create`/`thread-start` duplicate/keyed replay; `begin-edit` refuse; `update-buffer` replay; `save` replay; `discard` refuse; `apply`/`dismiss` replay; `undo` duplicate/keyed replay; `archive`/`resolve`/`unarchive` converge/keyed replay; `keep-in-brn` duplicate/keyed replay; `queue` duplicate/keyed replay; `intake` duplicate/keyed replay with variant `intent=useful` → duplicate, `key:false`, `receipt: Some(Terminal)`; `ask` duplicate; `export`/`backup`/`restore` refuse; `model-install` converge.
      - **Class meanings** (`retry_classes`, design §6(c) table, Markdown backticks removed). The `refuse` text must contain "If the first attempt took effect, an identical retry changes nothing" and "the retry performs the change once and can return ok".
      - **ReceiptAt** (table 4.2b): reads and non-receipt writes `None`; project commands, `queue`, `archive`, `resolve`, `unarchive` `Root`; `note-create`, `thread-start`, `undo` `Arm`; `apply`, `keep-in-brn`, `delegation`, `configure`, `pause`, `resume`, `topics-choose`, `intake`, `save` `Field`; `update-buffer`, `discard`, `dismiss` `None` with outcomes; `topics-create` `NoteAndPointer`; `ask` `Terminal`.
      - **Owner** (design §4.6): `Yes` for `apply`, `dismiss`, `undo`, `configure`, `delegation`, `pause`, `resume`, all nine project write commands, `keep-in-brn`, `archive`, `resolve`, `unarchive`, `topics-create`, `topics-choose`, `upgrade`, `model-install`; `ask` `WithOptions(["edit-note","record-decision"])`; `intake` `WithOptions(["project"])`; everything else `No` (`queue` is `No`).
      - **required** in the order of the §7.1 table (e.g. `project-settings`: project, base, title, instructions, knowledge; `configure`: provider, model, effort; `intake`: source, intent). `required_when` only on `intake`: `intent=chat → [thread]`, then `intent=project → [project]`.
      - **owner_decision_rule** string (exact): `These commands record an owner decision. Running the CLI does not make a caller the owner. Run them only on the owner's explicit instruction for that exact operation.`
      - `input`: `{"text_options":["title","text","prompt","instructions","actions"],"file_suffix":"-file","stdin":"-","max_bytes":16777216,"encoding":"UTF-8 without NUL; bytes used exactly"}`.
      Unit tests (`#[cfg(test)]` in `spec.rs`, list in design §4(e) "Unit tests (spec.rs)"): `retry_for` agrees with `description()` for every command with/without key and variants; `key_refused` is `None` for every `keyed:false` command; required orders match the §7.1 table; owner fields match §4.6 (`resolve` equals `archive` except `name`/`alias_of`); `required_when` only on intake; outcomes lists; class meaning substrings; `model-install` default `converge`; `description()` serialises to one line and has `owner_decision_rule` but no top-level `owner_decision`.
      Declare `mod spec;` in `main.rs`. Unused-code warnings are acceptable until step 5; do not add `#[allow(dead_code)]` that would survive step 5.
      Files: `crates/brn/src/spec.rs`, `crates/brn/src/main.rs` (one `mod` line)
      Verify: `cargo test -p brn --locked --offline` passes (spec unit tests plus the 11 `threads.rs` tests).

- [x] 4. Add typed errors and the contract module; switch `main.rs` to them with default output unchanged (design §4(b)2, 4, 5, 7; §4.3; §7.1 `contract.rs` and `main.rs` rows).
      `crates/brn/src/contract.rs` (new):
      - `Status { Ok=0, Internal=1, Usage=2, Stale=3, Deferred=4, NeedsReview=5, Rejected=6, Failed=7, Interrupted=130 }` with snake_case names and meanings from table 4.2.
      - `Code` with every row of table 4.3: `missing_data_dir, invalid_data_dir, missing_command, unknown_command, malformed_option, missing_value, repeated_option, unknown_option, conflicting_options, missing_option, invalid_value, invalid_input` (usage); `not_found, invalid_request, key_conflict, unsafe_directory, destination_exists, edit_session_open` (rejected); `unavailable, provider_failed, conversion_failed, timeout` (failed); `interrupted`; `storage_error, io_error, internal` (internal). Each has a status and meaning for `description().errors`.
      - `CliError { code, message, details: serde_json::Map, default_text: Option<String> }` with constructors and `From` impls for core `brn_threads_app::Error` (**exhaustive match, no `_` arm**: `Missing → not_found`, `Invalid → invalid_request`, `ImmutableRequest → key_conflict` with `{key}` filled by the caller when `--key` was given, `UnsafeDirectory → unsafe_directory`, `Io` with `AlreadyExists → destination_exists {option:"destination"}`, other `Io → io_error`, `Sqlite`/`Json → storage_error`; these seven are every variant at the baseline, so the daily-workflow `EditSessionOpen` fails to compile until mapped), `AppError` (`Core` delegates, `Search` → `From<RetrievalError>`, `Io → io_error`, `Json → storage_error`, `Invalid → invalid_request`, `Unavailable → unavailable`), `RetrievalError` (every default-build variant named: `Invalid → invalid_request`, `Unavailable`/`Stale → unavailable`, `Cancelled → interrupted`, `Corrupt`/`Sql`/`Native → storage_error`, `Io → io_error`, then a final `#[allow(unreachable_patterns)] _ =>` arm giving `unavailable {reason:"model_install"}`), `brn_ai::conversion::ConversionFailure` (every variant and every `ConversionError` inside `Intake` mapped per §4.3 rows `unavailable`, `provider_failed`, `conversion_failed {reason}`, `timeout`, `interrupted`), `serde_json::Error → internal`, `std::io::Error → io_error`, `String`/`&str → invalid_request`. `message` is exactly today's `e.to_string()` text.
      - `Annotations { operation, changed: Vec<Changed>, conflict: Option<Vec<Conflict>>, sigint: Option<bool> }`, `Changed { id, kind, version, archived, created }`, `Conflict { id, kind, version, archived, open_edit }`.
      - `emit_line(&str)` (stdout + `\n`) and `emit_err(&str)` (exact string to stderr); both ignore write errors and never panic.
      - Unit tests (design §4(e) "Unit tests (contract.rs)") for every error mapping and Details shape, `ImmutableRequest` with/without key, statuses one-to-one with exits, every code has a status, echo rule (`^[a-z][a-z-]{0,39}$`).
      `crates/brn/src/main.rs`:
      - `type Result<T> = std::result::Result<T, CliError>;`. Replace `.map_err(|e| e.to_string())?` with `?` everywhere **except inside the useful-intake sub-block** (it converts through `From<String>`). Add codes at call sites per the §4.3 "CLI-originated messages" table: `Missing --X` from `Invocation::arg` → `missing_option {option}`; `--X must be an integer`, `Effort must be…`, `Provider must be…`, `Delegation choice must be…`, `--trace takes summary or full`, `--knowledge takes yes|no`, `--seed-chat takes yes`, `--sources takes yes`, `Invalid budget`, `Source path must be absolute`, `Supported sources: …`, `Choose useful, full, or chat with --thread`, `hops`/`limit` conversions → `invalid_value {option, expected}`; the three existing conflicts → `conflicting_options {options}`; `--project must name an active project` / `--thread must name a chat` → `invalid_request {option}`; `read_input_file` Io/Budget/Invalid → `invalid_input {option:"source", reason: unreadable|empty_or_too_large|invalid}`, Cancelled → `interrupted`; `Interrupted before operation` and `Import stopped before saving converted content` → `interrupted`; `Semantic search is unavailable…` and non-native `model-install` → `unavailable`.
      - Caller-input call sites (§4.3 "Caller input parsed by the CLI"): `project-delete` `--preview-file` read → `invalid_input {option:"preview-file", reason:"unreadable"}`, its JSON → `reason:"invalid_json"`, `--actions` JSON → `invalid_value {option:"actions", expected:…}`; `backup`/`export` `NotFound` with a missing parent (`try_exists()==Ok(false)`, relative backup destination joined to the current dir) → `invalid_request {option:"destination", reason:"parent_missing"}`; `export` failing with `AppError::Invalid` while the absolute destination `try_exists()==Ok(true)` → `destination_exists {option:"destination"}`.
      - `apply`: on `AppError::Invalid`, if `workspace.store.receipt(&workspace.store.operation(id)?)` is `Some` → `internal {applied:true}`, else `invalid_request`. `dismiss`: `AppError::Invalid` → `internal {dismissed:true}`.
      - `begin-edit`: on core `Error::Sqlite`, read `recovery_buffers()`; if an open session exists for that note raise `edit_session_open {note, session, base_version}` with `message` "This note has an open unsaved edit session. Write to it only if your own successful begin-edit returned it." and `default_text` = the original error string; otherwise `storage_error`. **Use the existing single `.map_err(...)` line position** so the expected section-10 conflict stays a one-line conflict.
      - `ask`: only error-construction lines change. The line `result.map_err(|e| format!("Provider run failed: {:?}", e.kind))?` maps by `e.kind`: `ToolRejected`/`ModelRefused` → `invalid_request {kind}` (snake_case via a small unit-tested CamelCase→snake_case helper), `Storage` → `storage_error {}`, any other → `provider_failed {kind}`; `message` stays `Provider run failed: <Kind>` in every case. Add a unit test over every `AiErrorKind` (16 variants, `brn-ai/src/lib.rs:110-127`).
      - `execute(inv, notes: &mut Annotations)`. The `ask` arm becomes the expression `ask(…).await?`. The intake chat/project branch becomes three statements: `let value = applied(workspace.attach_full(…)?);`, `contract::fill_conflicts(&workspace, inv, &value, notes);`, `return Ok(value);`. Add `contract::fill_conflicts(&workspace, inv, &result, notes);` just before the final `Ok(result)`. Other intake edits are single-line replacements only (design §7.1 `main.rs` row).
      - All seven write sites use `emit_line`/`emit_err` with unchanged text: help print, result/error prints in `main`, `install_model`'s `eprint!("\rmodel …")` and trailing `eprintln!()`, and the intake progress callback `Arc::new(|progress| contract::emit_err(&format!("{}\n", progress.label())))`.
      - In this step `main` still prints default-mode output only (`default_text.unwrap_or(message)` on stderr, exit 1). `classify`/`fill_conflicts`/`envelope` may land here with their unit tests or in step 5.
      Files: `crates/brn/src/contract.rs`, `crates/brn/src/main.rs`
      Verify: `cargo test -p brn --locked --offline` — contract unit tests pass and all 11 `threads.rs` tests pass unchanged (default stderr text, exit codes and stdout unchanged).

- [x] 5. Rewrite `parse()` from the table and add machine mode, the envelope, classification and self-description (design §4(b)1, 3, 4, 6, 8; §4.1, 4.2, 4.2b, 4.4, 4.5; §4(d)).
      - `main` calls `parse(std::env::args_os().skip(1))`; `parse` takes `impl IntoIterator<Item = OsString>`. Detect a first-argument `--json` and the exact forms `--help` and `--json --help` on `OsStr`, then convert every token with `to_str()`; the first failure gives `malformed_option {position}` (1-based after the program name, counting `--json`), message `Arguments must be valid UTF-8`, `command` null, token never echoed. `--json` anywhere else is not special.
      - Order of checks: data dir (`missing_data_dir`/`invalid_data_dir`), command (`missing_command`/`unknown_command` via `spec()`), option scanning (`malformed_option`/`missing_value`/`repeated_option`), allowed options (`unknown_option {option, allowed}`), the two new parse-level conflicts (`--NAME` with `--NAME-file` → `Choose either --NAME or --NAME-file`; `--key` on a `key:false` variant → `--key is not accepted with --intent useful`), key format (`--key must match [A-Za-z0-9._:-]{1,128}`, `invalid_value {option:"key"}`, applies to project commands too), `--NAME-file` value is `-` or absolute (`--NAME-file takes an absolute path or -`), at most one `-` source (`stdin_reused`, reported on the second in TEXT_OPTIONS order), `required` (a `NAME-file` satisfies `NAME`; message `Missing --X`), `required_when`, then `--trace` value for `ask`/`intake` beside the existing `delegation --choice` check. All of this runs before the workspace opens. The echo rule (§4.1) applies to every argv-derived `details.option`/`details.command`.
      - `Invocation` gains `machine: bool`, `key: Option<String>`, `pending_inputs` (for step 6). `trace_level` stays in `ask` and can no longer fail there.
      - Machine-mode `main`: run `execute`, then `contract::classify(inv, &value, &notes, sigint)` with `sigint = notes.sigint.unwrap_or(CANCEL.load(SeqCst))`, build the envelope (all keys always present: `contract:1, command, ok, status, exit, outcome, operation, key, retry, changed, conflict, result, error`), `emit_line` it, exit with the status code. `retry` is `refuse` whenever status is `usage` or `rejected`, otherwise `retry_for`. `read` returning `null` → `rejected not_found` in machine mode only. `key` is null when absent or failing the pattern. No duplicate error on stderr in machine mode.
      - `classify` per table 4.2 domain mapping and 4.2b: ApplyOutcome Applied→ok, Stale/Superseded→stale, Deferred→deferred, NeedsReview→needs_review; SaveOutcome Saved→ok, Stale/GenerationMismatch/Closed→stale, Deferred→deferred; update-buffer Updated→ok, Ignored→ok only when the session is open, its generation equals the requested one and its markdown equals the supplied text, else stale; discard Discarded→ok, Ignored→stale; dismiss→ok; receipt writes ok `Applied`; `ask`/useful intake terminal Completed→ok, Interrupted with sigint→interrupted (130, `conflict` null), Interrupted without sigint→stale with outcome `Interrupted`, `Failed:*`→failed. `operation`/`changed` from `ReceiptAt` (Root: `result.operation` + receipt writes; Field: `result.receipt` when Applied/Saved; Arm: set by the arm via `notes`; NoteAndPointer: created Topics note with `created:true` plus pointer writes when Applied, operation null; Terminal/None: null and `[]`). `changed[].kind` is the `RecordData` variant name, `created` = `before` was null.
      - `fill_conflicts` (read-only `workspace.store.record` and `recovery_buffers`): records the sigint snapshot into `notes.sigint`, classifies, and only for `stale`/`deferred` fills `conflict` from the §4.2b "Conflict sources" (`Stale.records`, `Deferred.guarded`, `Superseded.run`, the session's `note` for Save/Buffer outcomes, `result.run` with kind `Run` for stopped `ask`, `result.pointer.*` for topics-create). `open_edit` is true when any open session exists for that note. A missing record gives null `kind`/`version`/`archived` and `open_edit:false`; a lookup failure leaves `[]`.
      - `brn --json --help` prints `description()` as one compact line, exit 0, opens nothing. Plain `--help` prints `HELP` (unedited) followed by the `AGENT_HELP` constant, verbatim from design §4(b)8 (starts with an empty line, "Agents and scripts: brn --json --data-dir DIR COMMAND ... prints one JSON envelope on stdout.").
      - Default-mode changes allowed only per §4(d): no panic on write errors or non-UTF-8 argv (`Arguments must be valid UTF-8`, exit 1); missing required options (and intake `required_when`, and `--trace`) reported before the workspace opens, so multi-fault invocations may change message; `--key`/`--NAME-file` newly accepted, new refusal texts from the §4.3 "New CLI messages" table; key pattern now also enforced on project commands; `--help` gains `AGENT_HELP`. Everything else byte-identical.
      - Unit tests: every outcome variant and terminal mapping, both ask Interrupted branches, intake useful vs full classification, topics-create Deferred pointer, `retry` refuse on usage/rejected and never null, cleanup classification for apply/dismiss.
      Files: `crates/brn/src/main.rs`, `crates/brn/src/contract.rs`, `crates/brn/src/spec.rs`
      Verify: `cargo test -p brn --locked --offline` passes (unit tests + 11 `threads.rs`); `cargo clippy -p brn -p brn-threads-app --all-targets --locked --offline -- -D warnings` is clean from this step on; manual: `target/debug/brn --help` exits 0, `target/debug/brn` and `target/debug/brn --unknown` exit 1, `bash scripts/verify-storage.sh` passes.

- [x] 6. Add file and stdin text input `crates/brn/src/input.rs` (design §5(b), §5(c)).
      `resolve(inv: &mut Invocation, cancel: &AtomicBool) -> Result<(), CliError>` reads each pending source in TEXT_OPTIONS order (title, text, prompt, instructions, actions) into `inv.args[NAME]`, so no `execute()` arm changes. Each source is read on a detached `std::thread` sending `Result<String, Reason>` over `std::sync::mpsc`; the caller loops on `recv_timeout(25 ms)`, checks `cancel` before the first wait and after each timeout, and returns `interrupted` ("Interrupted before operation") immediately when set. Do **not** use a main-thread read (SA_RESTART blocks forever) or `tokio::task::spawn_blocking` (runtime drop waits). A disconnected channel → `internal` ("Input reader stopped without a result").
      `read_bounded(source)`: for a path: `fs::metadata` (directory → `is_directory`; error → `unreadable`), then `File::open` (error → `unreadable`), then `file.is_terminal()` (→ `is_terminal`, message `--NAME-file: is a terminal`), then read at most 16 MiB + 1 byte (`brn_intake::MAX_INPUT_BYTES`; over → `too_large` with `max_bytes: 16777216`); for `-`: `std::io::stdin().is_terminal()` first (→ `--NAME-file -: stdin is a terminal`), then the bounded read. Then UTF-8 (`not_utf8`) and NUL (`contains_nul`). No trimming, BOM or newline normalisation; empty file → empty string. Errors are `invalid_input {option: "NAME-file", reason, [max_bytes]}` with the exact default texts from the §4.3 "New CLI messages" table. `main` calls `input::resolve` after `parse()` and before `execute`, so any input error or interruption leaves the data directory untouched.
      Unit tests in `input.rs`: limit + 1 boundary, NUL, invalid UTF-8, empty file, directory (`is_directory`), relative path refused at parse, stdin reuse, pre-set cancel flag returns `interrupted` without reading.
      Files: `crates/brn/src/input.rs`, `crates/brn/src/main.rs`
      Verify: `cargo test -p brn --locked --offline` passes; clippy command from step 5 is clean.

- [x] 7. Wire keyed retries into the CLI arms (design §6(b)1, 3; §6(c)).
      Host key with `--key KEY`: `cli-key:{command}:{KEY}` (`resolve` canonicalises to `archive`). Without `--key`, keep today's random keys exactly (`cli-archive:{uuid}`, `cli-unarchive:{uuid}`, `cli-keep:{uuid}`, `cli-queue:{uuid}`, `cli-import:{uuid}`, `cli-attach:{uuid}`, `cli:{uuid}`). Project commands keep passing the raw `--key` unchanged (`project:{KEY}` in core).
      - `note-create`: always call `new_note_with_key(key_or_fresh_uuid, …)`; set `notes.operation` and `notes.changed` from the receipt; `result` stays `writes[0].after` (the Record), with the same "Empty receipt" error as `applied_record`.
      - `thread-start`: same with `new_thread_with_key`.
      - `undo`: always `undo_with_key`; `notes.operation` = the compensation id in every outcome; `notes.changed` = writes only when Applied; `result` stays `applied(outcome)`.
      - `archive`/`resolve`, `unarchive`, `keep-in-brn`, `queue`, `intake --intent full|chat|project` (`import_full` and `attach_full` key argument lines only): keyed host key when `--key` is given.
      - `ImmutableRequest` under `--key` → `key_conflict {key}`.
      Files: `crates/brn/src/main.rs`
      Verify: `cargo test -p brn --locked --offline` passes; `cargo test -p brn-threads-app --locked --offline` passes; `python3 docs/work/active/projects/cli-witness.py target/debug/brn` still reports 19 checks (keyed project replay unchanged).

- [x] 8. Write the CLI documentation: README "Agents and scripts", carry-forward wording fixes, verification note (design §7.1 README row, §4.6, §6(c) recipes, §8).
      In `crates/brn/README.md` add a section "Agents and scripts" covering: invocation; envelope fields (§4.1); owner rule as one blockquote line `> ` + the exact `owner_decision_rule` string, with "Test harnesses on synthetic data act as the owner." on the next line, and the §4.6 table; tables 4.2, 4.2b (with the `commands[].outcomes` definition) and 4.3; streams and SIGINT rules (§4.4, including that SIGINT is observed only at the listed points); intake `required_when`; input rules and the 16 MiB transport bound versus shared-service field limits (ask prompt 128 KiB, queue text 128 KiB, project title 1024 bytes and instructions 128 KiB, useful-intake converted text 1 MiB); the retry table; the recovery recipes (lost response, stale save, stale under a key, `edit_session_open`, Deferred incl. the topics-create exception, stopped run, committed review cleanup, generations); "BRN does not record which caller opened a session"; the Luna re-conversion sentence for keyed non-Markdown intake; "Attach a document to a project only with `--intent project --project ID`, which is an owner decision; `--thread` must name a chat."; "Edit a protected note (`read` shows it) only on the owner's explicit instruction."; the #145 refusal codes (CLI-side `conflicting_options`/`missing_option`/`invalid_value`, shared-service `invalid_request`); the topics-create partial-application paragraph; compatibility (§4(d)); before/after examples (§8).
      Test-parsed formats (fixed): status rows `| <exit> | \`<status>\` | …`; error-code rows `| \`<code>\` | <status> | …`; retry table one row per command including `resolve`: `| \`cmd\` | yes/no | <default class> … | <keyed class>/— | <owner> |` with owner `yes`, `no`, or `with --opt, --opt` (`ask`: `with --edit-note, --record-decision`; `intake`: `with --project`), plus one row per variant `| \`cmd\` (--option value) | … |`.
      **Carry-forward fixes from `design-review.json`, required:**
      - C1: the `provider_failed` description (README error table, `description().errors` meaning text, any code comment) must not say "opening the provider client failed". Use: "any other `AiErrorKind` (none is known to reach `Err` from `run` at `771116f0`; kept as a fallback)", and also `ConversionFailure::Provider`. Keep the unit test over every `AiErrorKind`.
      - C2: wherever the README, the `invalid_request` meaning text or code comments describe `ToolRejected`/`ModelRefused`, say: "`ToolRejected` and `ModelRefused` are almost always pre-provider refusals from `begin`/`Selection::validate`; `finish` can also return `ToolRejected` after provider contact when the run record is inconsistent; the class stays `invalid_request`/`refuse`." No code change.
      In `docs/development/verification.md`, append one sentence to the existing paragraph under "## Current local gates" that begins "`verify-storage.sh` covers default formatting…", right after its `verify-end-to-end.sh` sentence: `verify-end-to-end.sh` also runs the CLI journey through the documented `--json` contract. Touch nothing else in that file.
      Files: `crates/brn/README.md`, `docs/development/verification.md`, `crates/brn/src/contract.rs` (meaning texts, if C1/C2 wording lives there)
      Verify: `python3 scripts/check-markdown-links.py` passes; `cargo test -p brn --locked --offline` still passes.

- [x] 9. Add process-level tests `crates/brn/tests/agent_contract.rs` (design §4(e), §5(e), §6(e) "Process tests").
      Spawn `env!("CARGO_BIN_EXE_brn")` with explicit args (follow `crates/brn/tests/threads.rs` helpers `command`/`root`, but write a local `call(json: bool, data, args, stdin)` that returns the parsed envelope and status). Root under `std::env::temp_dir().canonicalize()`; set child `HOME`, `CODEX_HOME`, `TMPDIR`, `TMP`, `TEMP` inside the root and remove `BRN_*`. Tests (names from the design; `#[cfg(unix)]` where marked):
      - Success: `json_success_envelope_has_every_field_and_result_matches_default_mode` (read in both modes; note-create Arm, project-create Root, pause Field, begin-edit and init non-receipt, notes read).
      - Invalid input: `usage_errors_are_json_with_stable_codes_and_create_nothing` (full row table including intake `required_when` rows with a nonexistent absolute source, the `--trace verbose` row, `--json` in second position, bad key with `key == null`, `--key` on `read`, `--key` with `--intent useful`, Unix non-UTF-8 row with `details.position == 8` and `command == null`, default mode `Arguments must be valid UTF-8` without `panicked`; data dir absent; `retry == "refuse"`); `caller_input_mistakes_after_open_are_usage_or_rejected_not_internal` (preview-file unreadable/invalid_json, `--actions '{'` and via `--actions-file -`, backup/export to a missing parent, then create the parent: identical export exits 0, third gives `destination_exists`; no `internal`); `text_input_refusals_create_nothing` (every platform; NUL, invalid UTF-8, 16 MiB + 1, directory, relative path, two `-` sources → `{option:"text-file", reason:"stdin_reused"}`, `--text` with `--text-file`; each also once in default mode against the §4.3 new-message texts); `terminal_sources_are_refused_without_blocking` (Unix pty via `libc::posix_openpt(O_RDWR|O_NOCTTY)`, `grantpt`, `unlockpt`, `ptsname`; fail with call name and errno if any fails; only `BRN_SKIP_PTY_TESTS=1` skips).
      - Input exactness: `text_files_and_stdin_preserve_exact_multiline_unicode`; `large_text_beyond_argv_limits_is_saved_through_a_file` (2 MiB).
      - Conflict outcomes: `stale_generation_closed_and_ignored_exit_3_with_current_versions`; `open_edit_session_reports_edit_session_open` (exit 6, details equal `{note, session, base_version}`, message is the contract text and lacks "Save or discard"; default stderr unchanged); `deferred_undo_exits_4_and_keyed_retry_applies_the_same_compensation`; `unkeyed_deferred_undo_reports_duplicate`; `not_found_destination_exists_and_unsafe_directory_are_rejected`; `ask_refused_before_the_provider_is_rejected_not_failed` (nonexistent chat → `invalid_request {kind:"tool_rejected"}`, `retry "refuse"`, default `Provider run failed: ToolRejected` exit 1; 128 KiB + 1 `--prompt-file` on a real chat → same). Project-boundary refusal reachable offline: include `search --project <chat id>` → `invalid_request {option:"project"}` and `--project` with `--thread` → `conflicting_options`. NeedsReview and Superseded are not reachable offline; they are covered by step-5 unit tests (record this in evidence).
      - Retry behavior: `keyed_creates_replay_and_reuse_with_other_inputs_is_refused` (note-create, thread-start, queue, archive, unarchive, project-create; counts unchanged; `key_conflict {key}`; unkeyed note-create `retry == "duplicate"`); `refuse_retry_after_an_interrupted_first_attempt_applies_once` (Unix; project-settings with 96 KiB `--instructions-file -`, SIGINT, then identical retry exits 0 at V+1, third exits 6); `keyed_keep_in_brn_retry_replays` (Unix); `keyed_markdown_full_import_replays_and_title_change_conflicts` (Unix; assert `settings.auth_file` is not an existing file first).
      - SIGINT and streams: `sigint_while_stdin_stays_open_is_interrupted_and_creates_nothing` (Unix; 256 KiB written, stdin kept open, `libc::kill(pid, SIGINT)`, exit 130 within 5 s, no data dir); `sigint_while_a_fifo_has_no_writer_is_interrupted` (Unix; `libc::mkfifo`); `closed_stdout_does_not_panic` (Unix; 200 KiB note, both modes exit 0, no `panicked`).
      - Compatibility: `default_mode_domain_outcomes_still_exit_zero` (Stale undo exits 0 in default mode).
      - Description: `machine_description_lists_every_parsed_command_and_matches_readme` (one line; every command dispatches; each command with required options gives `missing_option` for the first before creating the dir; README status, error, retry and owner tables and the owner blockquote line compared with `description()`).
      Every intake test sets child `TMPDIR`/`TMP`/`TEMP` to its own root and uses Markdown only.
      Files: `crates/brn/tests/agent_contract.rs`
      Verify: `cargo test -p brn --locked --offline` passes, including all new tests and the unchanged 11 in `threads.rs`; record whether `BRN_SKIP_PTY_TESTS` was set (it should not be).

- [x] 10. Add the maintained journey runner and its tests, wired into an existing gate (design §7.3, §7.4).
      `scripts/cli_journey.py BRN_BINARY [--evidence-dir ABSOLUTE_DIR] [--keep-root]`, stdlib only, importable (`Brn` helper, `child_env()`, `FIXTURES`). Setup: `parent = Path(tempfile.gettempdir()).resolve()` (a parameter for tests), snapshot `os.listdir(parent)`, `mkdtemp(prefix="brn-cli-journey-", dir=parent)` resolved; `root/tmp` (0700), `root/home`, data dirs under root. `child_env()` drops `BRN_*`, sets `HOME`/`CODEX_HOME` to `root/home` and `TMPDIR`/`TMP`/`TEMP` to `root/tmp`. Never imports `sqlite3`, never reads under a data dir, only checks whether chosen data-dir paths exist; no `ask`, no non-Markdown intake.
      `Brn.call(command, *, data, stdin=None, expect, **options)`: argv list without a shell, stdin `DEVNULL` unless bytes, `start_new_session=True`, `timeout=60` (kill the process group on timeout and record a timeout failure). Checks: exactly one JSON line; `contract == 1`; `ok == (exit == 0)`; `exit` equals the return code; known `status` and `retry`; `command` equals the invoked command, or null only for `usage` with `missing_data_dir`, `invalid_data_dir`, `missing_command`, `unknown_command` or `malformed_option`; `expect` holds. Each step records command, argv (paths only), stdin SHA-256, exit, seconds, status, outcome, operation, key, retry, error code and stderr tail. `Brn.describe()` runs `--json --help` with the line/timeout checks only.
      Journey (design §7.3 items 1–16, about 45 calls): describe; init + settings (`auth_file` null or under root); keyed project-create twice; project-document via files with `--key`; note-create via stdin multiline Unicode + read exact bytes; edit A with identical update-buffer retry and save retry; project-scoped search vs general search; history has the save; stale save with `conflict[0]` Note v3 `open_edit:true` and `edit_session_open` naming the runner's own session; recover (discard, begin-edit, merged update, save v4, Closed on old session); Deferred keyed undo exit 4 then Applied with the same operation, then replay, `candidates` has no unapplied X; keyed note-create replay, `key_conflict`, unkeyed `duplicate`; keyed Markdown intake replay and title `key_conflict`; usage errors (`missing_data_dir` command null, `conflicting_options`, `unknown_option` `bogus`, directory absent); `read --id missing` → `not_found`; backup/export then `destination_exists`, restore to a fresh dir with equal `notes`/`projects`, project search on the copy, restore retry → `unsafe_directory`.
      Failure evidence: on any failed check print to stderr the failing step index, command, argv, exit, stdout, stderr and the data-dir path, plus the last five steps; write `transcript.jsonl` and `failure.json` to `--evidence-dir` when given; print a JSON summary on stdout and exit 1. Success prints `{"journey":"passed","steps":N,"checks":M}` and exits 0. Cleanup always: write evidence, `shutil.rmtree(root)` unless `--keep-root`, then fail if any new entry in `parent` matches `^\.?brn-` or `^threads-intake-` (root exempt with `--keep-root`), naming the entries. Text content never appears in the transcript (paths and SHA-256 only).
      `scripts/tests/test_cli_journey.py` (stdlib `unittest`, stub executables written into a private temp dir; no real `brn` required): `child_env()` paths and no `BRN_*`; a stub that creates `P/brn-threads-intake` in the test's private parent P fails the parent check naming it, and a stub writing only under its `TMPDIR` passes with P unchanged; `Brn.call` rejects non-JSON, two lines, mismatched `exit`, unknown status and records the step index in `failure.json`; accepts `command: null` for a `missing_data_dir` usage envelope and rejects it on `ok` or a different command; a stub sleeping past a shortened timeout is killed with its process group and recorded as a timeout.
      In `scripts/verify-end-to-end.sh` add one line after `python3 scripts/verify-end-to-end-fixtures.py "$CARGO_TARGET_DIR/debug"`: `python3 scripts/cli_journey.py "$CARGO_TARGET_DIR/debug/brn"` (runs in both default and `--fixtures-only` modes). Do not change `verification_evidence.py` invocations or `ci.yml`.
      Files: `scripts/cli_journey.py`, `scripts/tests/test_cli_journey.py`, `scripts/verify-end-to-end.sh`
      Verify: `python3 -m unittest discover -s scripts/tests -v` passes; `bash -n scripts/verify-end-to-end.sh`; `python3 scripts/cli_journey.py "$PWD/target/debug/brn"` (run from the worktree after `cargo build -p brn --locked --offline`) prints `"journey":"passed"` and exits 0; record its runtime (A8). Then the CI reproduction: `p="$(mktemp -d "${TMPDIR:-/tmp}/brn-ci-fixtures.XXXXXX")"; p="$(cd "$p" && pwd -P)"; TMPDIR="$p" bash scripts/verify-end-to-end.sh --fixtures-only && rmdir "$p"` succeeds (build `brn-desktop` and `cargo build -p brn-intake --features helper --bin brn-intake-helper --locked --offline` first, as the fixture script needs them).

- [x] 11. Run the full verification sweep and record evidence (design §7.6, §7.4).
      Run, from the worktree, and record exact commands, tree hash and results in `docs/work/active/cli-agent-contract/evidence.md`:
      `cargo fmt --all -- --check`; `cargo build --workspace --locked --offline`; `cargo clippy -p brn -p brn-threads-app --all-targets --locked --offline -- -D warnings`; `cargo test -p brn --locked --offline`; `cargo test -p brn-threads-app --locked --offline`; `cargo build -p brn --features native-retrieval --locked --offline` and `cargo check -p brn -p brn-threads-app --features brn-threads-app/native-retrieval --locked --offline` (if the native crates are not in the local cargo cache, record that and that CI's native lane covers the first); `git diff --exit-code Cargo.lock`; `python3 -m unittest discover -s scripts/tests -v`; `python3 scripts/check-markdown-links.py`; `for s in scripts/*.sh; do bash -n "$s"; done`; `bash scripts/verify-storage.sh`; the CI fixture reproduction from step 10; `python3 docs/work/active/projects/cli-witness.py target/debug/brn` (19 checks). Also confirm `git diff --stat origin/main -- crates/brn-threads-core crates/brn-desktop` is empty and that the only `crates/brn-threads-app/src` change is `lib.rs`.
      Files: `docs/work/active/cli-agent-contract/evidence.md`
      Verify: every command above exits 0 (native ones may be recorded as unavailable offline, with the reason).

- [x] 12. Test-merge the coordinating branches on a disposable scratch checkout (design §10).
      Create a separate scratch worktree from the task head, outside the task worktree: a worktree `.worktrees/cli-agent-contract-merge-check` on a new branch `scratch/cli-agent-contract-merge-check` from `work/cli-agent-contract`. In it, `git merge --no-commit --no-ff work/daily-workflow-reliability` (head `b3fc584a`). Expected: one certain conflict on the begin-edit arm's `.map_err` line; resolve by taking that branch's `.map_err(begin_edit_error)?`, changing `begin_edit_error` to return `CliError` (`Error::EditSessionOpen` → `edit_session_open`, details `{note, session, base_version}`, `message` = this contract's text, `default_text` = that branch's JSON line; others via `CliError::from`), mapping `Error::EditSessionOpen` the same way in the exhaustive `From<Error>` match, and deleting this branch's `recovery_buffers` enrichment. A possible conflict around `useful_project` in the intake arm: keep both sides. Run `cargo test -p brn --locked --offline` there (both `threads.rs` suites, including `a_second_begin_edit_reports_the_open_session_as_structured_json`, and `agent_contract.rs`). Then `git merge --abort` (or reset the scratch tree), and separately test-merge `origin/codex/linux-ui-cloud` (`7075c46a`) with `--no-commit --no-ff` and confirm `docs/development/verification.md` merges cleanly. Never commit or push the scratch merge. Afterwards remove the scratch worktree and the scratch branch; this removes only the disposable branch created in this step. Do not modify `work/daily-workflow-reliability` or any other branch.
      Files: `docs/work/active/cli-agent-contract/evidence.md` (record conflicts, resolutions, test results)
      Verify: scratch `cargo test -p brn --locked --offline` result recorded; `git worktree list` and `git branch --list 'scratch/*'` show the scratch removed; `git -C <task worktree> status --porcelain` clean apart from task files.

- [x] 13. Close out the plan, status and report.
      Tick completed items here; add a short `docs/status.md` entry next to #148's "Quality simplification batch" entry: implemented and verified on `work/cli-agent-contract`, not merged, owner acceptance not requested, native observation not applicable. In `evidence.md`, include before/after command examples (design §8, re-run against the built binary), the machine contract summary (pointer to the README section and `brn --json --help`), verification results, and remaining gaps (below). Report implemented, verified, native-observed (n/a), owner-accepted (not requested) and merged (no) separately; state that CLI checks do not establish native UI usability. Commit locally; do not push.
      Files: `docs/work/active/cli-agent-contract/plan.md`, `docs/work/active/cli-agent-contract/evidence.md`, `docs/status.md`
      Verify: `python3 scripts/check-markdown-links.py` passes; `git status --porcelain` is clean after the final local commit; `git log origin/main..HEAD --oneline` lists only task commits.

## Decisions taken in this plan

- **No FEAT decomposition of the workflow tail.** The steps share `main.rs` heavily and depend on each other in order; the existing implement-and-review loop runs this plan unchanged.
- **Step order follows dependency:** shared service first (only shared change), then the table, error types, machine mode, input, keys, docs, tests, runner. The README lands before the process tests because one test compares the README with `description()`.
- **Clippy `-D warnings` starts at step 5.** Steps 3–4 add types consumed by machine mode; warnings there are acceptable, but no `#[allow(dead_code)]` may survive.
- **`AiErrorKind` names in `details.kind`** are produced by a small CamelCase→snake_case helper over the `Debug` name, unit-tested over every variant, rather than relying on serde attributes in `brn-ai`.
- **Scratch test-merge in a separate worktree**, so the task worktree is never left mid-merge; the scratch branch is created and deleted inside step 12.
- **Daily-workflow head drift:** the branch is now `b3fc584a`; the only change since the design's `20f73edb` is its `evidence.md`, so the design's conflict analysis still holds.

## Remaining gaps (design §9, ranked)

1. Bounded, paginated inspection (`history`, revisions, receipts, candidates) is not added; `history` stays multi-megabyte and `evaluate-wiki-guidance.py` still reads SQLite read-only.
2. NeedsReview, Superseded, run Interrupted/Failed and model-candidate review cannot be reached offline at the process boundary (unit-mapped only); the chats-and-projects witness still fails and writes SQLite; the fresh-workspace `auth_file` default is unchanged.
3. No stale-session rebase; sessions are not bound to a caller, so a lost `begin-edit` response means wait or ask the owner.
4. `topics-create`, `knowledge-check --seed-chat`, `ask` and useful intake have no keys; converge commands still append receipts; unkeyed `undo`, `keep-in-brn` and full/chat/project intake can still leave unapplied candidates (reported as `duplicate`); keyed non-Markdown intake re-sends the document to the provider before replay.
5. SIGINT is observed only at listed points; no SIGTERM envelope, no `--timeout`, no SQLite busy code; a killed `ask` can leave a Working run that keeps topics-choose Deferred until orphaned-run recovery lands.
6. No per-command help, `--data-dir` must come first, `backup`/`restore` accept relative paths, intake help omits Luna conversion, `intake --intent chat --thread <PROJECT_ID>` attaches without the owner mark.
7. Read-only commands on a mistyped directory still create a workspace; enum values other than `--trace` are validated after opening.
8. `AppError::Invalid(String)` granularity; the useful-intake sub-block keeps a coarse `invalid_request` mapping and `conflict: []` until the daily-workflow merge.
9. Owner authority is stated, not enforced, at the CLI.

<!-- ===================== ROUND 2 BEGIN (round-1 content above is kept unchanged) ===================== -->

# Round 2: closing the remaining agent-contract gaps

**Assignment.** The owner asked to "fix the remaining gaps" left by round 1 (the ranked gaps above and in the [evidence](evidence.md#remaining-gaps-ranked)), inside the original assignment: thin CLI over the shared checked service, no owner data, no publish, merge or release.

**Design authority.** The design is `design-2.md` (rev 4) plus `design-2-appendix.md`. Review 2, iteration 4 approved it and carries seven NITs forward. All three files are task artifacts outside the repository, in `.worktrees/cli-agent-contract.artifacts/round2/`. "Design §X" and "appendix §X" below name the section with the exact wording. Every `carry_forward` item is a required plan item (C1–C7 below). The owner's final decisions are recorded in [owner-decisions.md](owner-decisions.md): B2, B4 and B3 part 1 are A10–A12 here; B1 and B3 part 2 belong to round 3 and stay options-only.

**Worktree.** `/Users/evokessler/repos/brn-rust/.worktrees/cli-agent-contract`, branch `work/cli-agent-contract`, base `8bef439b`. Round 2 starts at HEAD `2537db54`. Commit locally after each step. Never push, open a PR, merge, rebase, tag, or touch `main` or another branch.

**Status.** Planned. Nothing from round 2 is implemented yet.

## The three highest-value bounded improvements (do these first)

1. **A1, bounded paginated inspection** (top gap, round-1 gap 1). Today `history`, `candidates` and `records` return everything, so a mature workspace returns megabytes. There is no access by record, revision, run or operation, and `evaluate-wiki-guidance.py` still opens SQLite. This adds page mode with stable cursors and four read-only commands (`revisions`, `revision`, `operation`, `runs`), plus paged `read`.
2. **A4, termination and bounded execution.** Today SIGTERM kills the process with no envelope, there is no deadline, and SQLite lock contention looks like an internal error. This adds a SIGTERM envelope (`terminated`, 143), `--timeout SECONDS` with a soft stop that never hides a commit and a one-line hard stop, and a `busy` code.
3. **A3, more retry identities.** Today `topics-create` and `knowledge-check --seed-chat yes` duplicate on retry. Keyed full/chat/project intake re-converts the document, and re-sends non-Markdown documents to the provider, before it replays. This adds keys for both commands, and keyed intake replays before conversion.

**Rank sets order only. Every item A1–A12 is in scope (design §0).**

| Rank | Item | Step | Agent-reliability impact | Test effort |
|---|---|---|---|---|
| 1 | A1 bounded inspection | R2-2, R2-3, R2-4 | highest | med |
| 2 | A4 SIGTERM, `--timeout`, `busy` | R2-5 | high | med |
| 3 | A3 retry keys | R2-6 | high | low |
| 4 | A10 no default credentials (B2) | R2-7 | high | low |
| 5 | A2 offline outcomes, witness | R2-8 | med | high |
| 6 | A12 stale-edit recipe (B3.1) | R2-9 | med | low |
| 7 | A11 `Unchanged` converges (B4) | R2-10 | med | med |
| 8 | A6 mistyped dirs, early values | R2-11 | med | low |
| 9 | A5 discovery | R2-12 | med-low | low |
| 10 | A7 refusal reasons | R2-13 | med-low | low |
| — | A8 review scope, A9 round-1 findings | R2-14 to R2-16 and every step | process | low |

R2-1 (contract 2 foundation) comes first because every later envelope carries contract 2. A6 comes before A5 because A5 renders the value table A6 extends. The `Grammar`/`VALUES` table itself starts in R2-3, because A1's new options need pre-open validation.

## Hard constraints: re-check every item, every iteration (verbatim from design §1)

- [ ] 1. **No new infrastructure.** No new protocol, server, daemon, RPC or automation framework. The only listener is A2's test-only loopback stub.
- [ ] 2. **Thin CLI over the one checked service; SQLite owns state.** No direct canonical-DB writes by CLI, tests or scripts; scripts read only documented CLI output. Exception: A4's busy test holds a lock, writing nothing.
- [ ] 3. **CLI access grants no owner authority.** Protection, exact-candidate review, version checks, dirty guards, replay, project boundaries, Undo and PR #145's owner-only helper selection stay core-enforced. The CLI never builds `ChangeRequest`/`HostAuthority`; keyed and converged functions reuse their twin's authority strings; no replay or `Unchanged` path skips a check.
- [ ] 4. **No production bypass.** No force, skip-review, raw SQL, fake provider, base-URL override or `BRN_*` switch. `--timeout` can only stop early.
- [ ] 5. **Contract changes additive or versioned (2) and documented;** default mode unchanged except §2.3. Keep passing: `threads.rs::json_command` (exit 0 on Stale undo); stderr-substring asserts; witnesses failing on any nonzero exit; `live-journeys.py` (interrupted `ask` exits 0, `"terminal":"Interrupted"`); CI `brn --help`; `verify-storage.sh` (`brn`, `brn --unknown` nonzero); the macOS `TMPDIR` + `rmdir` trap.
- [ ] 6. **Off-limits:** `HELP`, and the useful-intake sub-block in `main.rs` (the `else` after `if intent == ImportIntent::FullNote`, ~:961).
- [ ] 7. **Shared-service changes minimal, additive, flagged ⚑, overlap-checked.** The only core write-path change is A11 (owner-approved); the only app behavior addition is A10's open. `brn-desktop` and `brn-ai` untouched; desktop behavior unchanged.
- [ ] 8. **`rust-toolchain.toml`/`Cargo.lock` are version authority;** `--locked`; no new dependencies.
- [ ] 9. **Synthetic data only:** fresh `--data-dir` under `/tmp/brn-cli-agent-contract/<run>/`, fresh `HOME`/`CODEX_HOME`/`TMPDIR`, `BRN_*` cleared, `BRN_INTAKE_HELPER` where needed; no owner data, live provider calls or credentials in evidence.
- [ ] 10. **Local commits on `work/cli-agent-contract` only.** No push, PR, merge, rebase onto main, touching `main` or the main checkout, tag, publish or release.

Plus these rules, re-checked each iteration:

- [ ] **Coordination rule.** Do **not** edit the `HELP` constant (`main.rs:25`). Do **not** edit the useful-intake sub-block, the `else` branch after `if intent == ImportIntent::FullNote` (`main.rs:961` at `2537db54`) that ends in `return ask(…)`. The A3 and A10 intake hooks go between the `intent` match and `converter(kind)?` (`main.rs:903-906`). At the end, run `merge-tree` against `work/daily-workflow-reliability` and do the scratch test-merge (R2-15).
- [ ] **ui-polish commitments (board).** Add no `ApplyOutcome` variant. `chat_command`/`project_command`/`Workspace::open` keep their signatures and behavior; A11 and A10 add functions beside them. No `brn-desktop`/`brn-ai` edit. A step that would break one of these stops and flags it.
- [ ] **Board and Mac lock.** Read `/Users/evokessler/repos/brn-rust/.worktrees/coordination/BOARD.md` before editing `crates/brn-threads-core`, `crates/brn-threads-app` or `crates/brn-ai`, and again before the scratch merge. Never edit it; put coordination items in the step summary. Never launch or drive a native window, and never check or create `MAC-IN-USE`. `brn-desktop --headless-check` is fine.
- [ ] **LIST B stays options-only.** B1 (workspace-scoped CLI authority) and B3 part 2 (caller-bound sessions) are round 3 or later. Add no `--purpose`, no session secret, no actor field and no authority field. Docs that mention them point to `owner-decisions.md` and describe them as future work, without implementing them.
- [ ] **A9 hygiene.** Run `git status --porcelain` before each commit and stage files by name. Run PTY tests without `BRN_SKIP_PTY_TESTS` where possible. Where the sandbox forbids PTYs, record the test as blocked (CI runs it), not as passed.
- [ ] **Clippy and warnings.** `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` stays clean after every step. `#![allow(dead_code)]` is allowed only at the top of `crates/brn/tests/support/mod.rs`, the shared test-helper module, because not every test file uses every helper.

## Carry-forward items from review 2 (required)

| ID | Required change | Lands in |
|---|---|---|
| C1 | Cite daily commits by subject plus their current hash (at review: tip `b76cc5a7`; canonical locators `344331f2` "count one file as one source regardless of path spelling"). Re-derive the A11 core resolutions (`chat.rs`/`projects.rs`) against the **final** daily tip before the scratch merge, and record that tip. | R2-6 (`same_source` comment), R2-15 |
| C2 | The envelope `retry` stays a class string. `Code::EditSessionOpen.meaning()` drops `details.session` and reads "(details.note, details.base_version)". `Code::Timeout.meaning()` covers the `--timeout` deadline and `details.{deadline_seconds, hard}`. The A3 replay sentence goes in the README, `AGENT_HELP` and `intake` per-command help. The A12 sentence goes in `CliError.message` and `Code::EditSessionOpen.meaning()`; the A12 recipe goes in the README, `AGENT_HELP` and `save` per-command help. | R2-5, R2-6, R2-9, R2-12 |
| C3 | `HELP` stays byte-identical. `brn --help` = `HELP` + `\n` + the updated `AGENT_HELP`, exit 0. | R2-12 (wording, test) |
| C4 | Read `workspace.settings()` **once** at `main.rs:904`, check it, then reuse it for the `ConversionJob`; the check precedes `converter(kind)` and `convert`. Note in code and README that `fn ask` is shared with useful intake, so the intake-arm check fires first and the `fn ask` check also guards useful intake. README desktop wording: "Desktop shows no sign-in file for such a workspace; choose one in Settings before running work." If the A2 spike fails, `configure_then_ask_reaches_the_provider` falls back to the configured `ask_refused_before_the_provider_is_rejected_not_failed`. | R2-7, R2-8 |
| C5 | `existing_workspace_keeps_its_stored_auth_file` lives in its own one-test integration file. Before the in-process `Workspace::open` it sets `CODEX_HOME=<root>/home` (edition 2021 `std::env::set_var`). It asserts that the CLI `settings` call reports `<root>/home/auth.json`, and it runs no `ask` or provider intake. | R2-7 |
| C6 | `Unchanged.record`: `archive`/`resolve`/`unarchive` → the thread; `chat-move` → the chat; `project-link`/`project-unlink` → the project. Add `docs/work/active/projects/cli-witness.py:49-59 (all real changes)` to the §2.3 item 11 checked-callers list. | R2-10 |
| C7 | A12 recipe generations. Step 2: `discard --session <result.session.id> --generation <result.session.generation>`. Step 5: `update-buffer --session NEW --generation 1 --text-file merged.md`. Step 6: `save --session NEW --generation 1`. | R2-9 |

## Shared-service overlap (checked at planning time, 16 Oct 2026)

Command: `git -C /Users/evokessler/repos/brn-rust -c core.fsmonitor=false log --oneline 8bef439b..<branch> -- <path>`. It was run for every path this round touches: core `lib.rs`, `chat.rs`, `projects.rs`, `store.rs`, `inspect.rs`, `converge.rs` and `tests/`; app `lib.rs`, `settings.rs`, `keyed.rs`, `converge.rs`, `topics.rs`, `knowledge.rs`, `intake.rs`, `chat.rs`, `projects.rs` and both app test files; `crates/brn/src`, `crates/brn/tests` and the README; and the touched scripts, witnesses and docs. Refs are local, else `origin/`.

| Branch | Ref used | Result |
|---|---|---|
| `codex/projects-feature`, `work/knowledge-retrieval`, `work/knowledge-dense`, `work/knowledge-agent-tools`, `codex/writing-polish`, `codex/tile-ui-polish`, `codex/writing-ux-research`, `work/subagent-design`, `work/quality-simplify` | local (`origin/` also checked for projects-feature and tile-ui-polish) | 0 commits after `8bef439b`: each tip is already an ancestor of the base. No overlap. |
| `codex/linux-ui-cloud` | `origin/` (`7075c46a`; no local ref) | 1 commit, `7075c46a` "Run desktop widget tests on Linux…". It touches `scripts/tests/` (new `test_desktop_verification.py`, `test_development_tooling.py`), `docs/status.md` and `docs/development/verification.md`, none of which this round edits apart from one `docs/status.md` line. `merge-tree` is clean. |
| `work/daily-workflow-reliability` | local `b76cc5a7` (29 commits) | Overlaps:<br>• core `lib.rs`: `821c9516`, `f3deba1a`, `0f7016e8`, `344331f2`, `558250bb`, `8470fa74`, `3aaa0f73`<br>• core `chat.rs`: `51de25d6` "end lease-free Runs like orphans on every stop path", `8470fa74` "a killed Run's notice and candidate stay reachable"<br>• core `projects.rs`: `8470fa74`<br>• core `store.rs`: four commits (untouched here)<br>• app `lib.rs`: `f3deba1a`, `9537f242`<br>• app `settings.rs`: `51de25d6` (from :98)<br>• app `intake.rs`: 7 commits, including `344331f2` canonical locators (semantic dependency only; not edited)<br>• app `chat.rs`, `projects.rs` (not edited)<br>• `crates/brn/src/main.rs`: 4 commits<br>• `crates/brn/tests`: adds `data_folder.rs`, `search_messages.rs`, `source_locator.rs` and edits `threads.rs` |
| `work/integration-ui-reliability` | — | absent |

`git merge-tree --write-tree --name-only work/cli-agent-contract work/daily-workflow-reliability` at `2537db54`/`b76cc5a7` conflicts only in `crates/brn/src/main.rs`; app `lib.rs` auto-merges.

**Consequences for the coder:**
- Every core/app change stays additive and read-only except A11's owner-approved hook (B4 exception, design §1.1).
- New modules go after `mod backup;` (core) and after `mod knowledge;` (app).
- New test files use names daily doesn't use: `agent_*.rs`, core `tests/inspect.rs`, core `tests/converge.rs`.
- Don't touch core `store.rs`. `inspect.rs` uses the public `Store::prepared`/`Store::receipt`/`Store::record` and the crate-visible `conn`.
- Don't touch app `intake.rs`, `chat.rs` or `projects.rs`. `keyed.rs`/`converge.rs` repeat their three-line prefix, binding and authority patterns.
- If a re-run at implementation time finds another branch editing the same function, stop that item and record it.

## Mechanics common to every step

- **Test support.** `crates/brn/tests/support/mod.rs` is new in R2-1 and every new process-test file uses it with `mod support;`. It copies the round-1 patterns from `agent_contract.rs:13-176`:
  - `Root`: the private root has `home/` and `tmp/`, and children get `HOME`, `CODEX_HOME`, `TMPDIR`, `TMP` and `TEMP` inside it, with every `BRN_*` removed.
  - `run` and `wait_output`, with a hard timeout per child.
  - `call`/`ok`/`assert_error`.
  - `brn()`: the binary is `env!("CARGO_BIN_EXE_brn")`, unless the **test process** has `AGENT_CONTRACT_BASELINE_BIN` set. That test-only variable is never passed to children and never read by `brn`.
  - When it is set, the envelope helper skips the `contract == 2` assertion, so a baseline failure shows the item-specific assertion.
  - `agent_contract.rs` keeps its own helpers (no churn).
- **"Fails against 2537db54" rule.**
  - Build the baseline once in R2-1: `mkdir -p /tmp/brn-cli-agent-contract/r2-baseline/src && git -C /Users/evokessler/repos/brn-rust archive 2537db54 | tar -x -C /tmp/brn-cli-agent-contract/r2-baseline/src && (cd /tmp/brn-cli-agent-contract/r2-baseline/src && cargo build -p brn --locked --offline --target-dir /tmp/brn-cli-agent-contract/r2-baseline/target)`.
  - After each step, run that step's new process-test files with `AGENT_CONTRACT_BASELINE_BIN=/tmp/brn-cli-agent-contract/r2-baseline/target/debug/brn cargo test -p brn --locked --offline --test <file> --no-fail-fast`.
  - Every new test must fail there, on an assertion specific to the item. Record the first failing assertion per test in the evidence matrix.
  - **Guard tests** pass on both binaries: they protect compatibility, not new behavior. The step lists each one explicitly and gives the reason.
  - New core/app tests do not compile against `2537db54` (new API). Record them as "n/a: new API".
- **Four test categories per item:** success, invalid input, conflict/refusal and retry. An item that adds no new input surface marks that category "n/a" with the reason.
- **README coupling.** `machine_description_lists_every_parsed_command_and_matches_readme` (`agent_contract.rs:1644`) compares the README's status, error-code, retry and owner tables with `description()`. Every step that adds a status, code, command or class therefore updates those README tables, in the test-parsed row formats of round-1 step 8, in the same commit.
- **Commit per step**, with a Conventional Commit subject, after `cargo fmt --all`. Tick the step here and add its evidence rows to [evidence.md](evidence.md) in the same commit.

### Default versus machine mode (decided per item)

| Item | Machine mode (`--json`) | Default mode |
|---|---|---|
| A1 | Page mode via options; envelope `page`; `read` page Stale exits 3 | Flag-less `history`/`candidates`/`records`/`read` byte-identical; page options and new commands print `result` as one JSON line, exit 0 (a Stale page also exits 0, like every default-mode outcome) |
| A4 | SIGTERM handler → `terminated` 143; `--timeout` soft → `timeout` 7, hard → one envelope line; `busy` 7 | SIGTERM keeps `SIG_DFL`; `--timeout` soft prints "Stopped at the --timeout deadline (N s); nothing was committed", exit 1; a run prints its JSON, exit 0; hard prints "Stopped at the --timeout deadline (N s); state unknown", exit 1; busy keeps SQLite's text, exit 1 |
| A3 | New keys; replay before conversion | Same keys accepted; output unchanged |
| A10 | `credentials_not_configured` (6) before any commit | CLI-created workspaces get `auth_file: null` (§2.3 item 12); `ask`/intake keep today's path |
| A2 | Tests and witness only | Witness runs in default mode |
| A12 | `details` = `{note, base_version}` | Text unchanged on this branch |
| A11 | `Unchanged`, ok, exit 0 | Prints `{"outcome":"Unchanged","record":ID,"version":V}`, exit 0 (§2.3 item 11) |
| A6 | Reads on a missing or empty dir → `missing_workspace` (6); values checked before opening | Reads still create the dir; values checked before opening with the same messages (§2.3 item 8) |
| A5 | `COMMAND --help` JSON; global options anywhere; relative paths refused | Text help; global options anywhere; relative paths refused (§2.3 items 6, 7, 10) |
| A7 | `details.reason` | Unchanged |

### Exit codes after round 2

0 `ok` (includes `Unchanged`); 1 `internal`; 2 `usage`; 3 `stale`; 4 `deferred`; 5 `needs_review`; 6 `rejected` (adds `missing_workspace`, `credentials_not_configured`, and the reserved `data_folder_unusable`); 7 `failed` (adds `busy`, and `timeout` from `--timeout`); 130 `interrupted`; **143 `terminated`** (new). Default mode keeps 0/1.

## Steps

- [x] R2-1. **Record the baseline, commit the owner decisions, and lay the contract-2 foundation.**
      (a) Pre-flight checks:
      - `git -C /Users/evokessler/repos/brn-rust/.worktrees/cli-agent-contract status --porcelain` shows only `owner-decisions.md` (untracked), this `plan.md` and `evidence.md`.
      - HEAD is `2537db54`.
      - Record the daily tip with its subject (C1), re-run the overlap table above, and read BOARD.md.

      (b) Build the baseline binary (Mechanics). Record its path and `--json --help` contract number (1).
      (c) **Commit 1 (docs):** add `owner-decisions.md` byte-for-byte unchanged, plus this plan and the evidence outline.
      (d) **Commit 2 (contract 2):**
      - `contract.rs`: `Envelope::to_value` emits `"contract": 2` and two new always-present keys:
        - `page` (null, or `{limit, shown, cursor, total}`), from a new `Envelope.page: Option<Value>`;
        - `stop` (null, or `sigint|sigterm|deadline`), from `Envelope.stop: Option<&'static str>`.

        Both are null everywhere for now.
      - `spec::description()` emits `"contract": 2`.
      - README "Agents and scripts":
        - the envelope table gains `page` and `stop` rows;
        - a new "Changes from contract 1" table (rows: contract number, `page`, `stop`; later steps add theirs);
        - the forward rule: "Ignore unknown keys. Treat an unknown `status` or `outcome` as non-success."
      - Update the in-repo consumers in the same commit:
        - `scripts/cli_journey.py:146,197` → `== 2`, and assert that `page` and `stop` keys exist;
        - the stubs in `scripts/tests/test_cli_journey.py:21`;
        - every `"contract"` assertion in `agent_contract.rs` and in the `contract.rs` unit tests.
      - Add `crates/brn/tests/support/mod.rs` and `crates/brn/tests/agent_discovery.rs` with `every_envelope_has_contract_2_page_and_stop`. It checks the `ok`, `usage` (missing data dir) and `rejected` (`read --id missing`) envelopes, plus `--json --help`.

      CLI surface: envelope keys `contract:2`, `page`, `stop`. Shared service: none. Mode: machine only.
      Tests:
      - success/refusal: `every_envelope_has_contract_2_page_and_stop` (fails at baseline: `contract` is 1, no `page`);
      - invalid input, conflict, retry: n/a (no new surface).

      Files: `docs/work/active/cli-agent-contract/{owner-decisions,plan,evidence}.md`, `crates/brn/src/contract.rs`, `crates/brn/src/spec.rs`, `crates/brn/README.md`, `crates/brn/tests/{support/mod.rs,agent_discovery.rs,agent_contract.rs}`, `scripts/cli_journey.py`, `scripts/tests/test_cli_journey.py`
      Verify:
      - `cargo test -p brn --locked --offline` passes (unit, `agent_contract`, `agent_discovery`, `threads.rs`);
      - `python3 -m unittest discover -s scripts/tests` passes;
      - `cargo build -p brn --locked --offline && python3 scripts/cli_journey.py "$PWD/target/debug/brn"` prints `"journey":"passed"`;
      - the baseline run of `agent_discovery` fails.

- [x] R2-2. **⚑ A1 core: SELECT-only paging functions** (design A1 "⚑ Core `inspect.rs`").
      Read BOARD.md first. Create `crates/brn-threads-core/src/inspect.rs`:
      - **Types:** `OperationState {Applied, Prepared, Retired, Allocated}`; `OperationInfo {id, state, undo_of, replacement_of, prepared: Option<Prepared>, receipt: Option<Receipt>}`; `CandidateRow {rowid, prepared, undo_of, replacement_of}`.
      - **`impl Store` with `&self` functions:**
        - `receipt_page(after: i64, limit: usize, record: Option<&str>) -> Result<(Vec<(i64, Receipt)>, bool)>`. SQL: `SELECT rowid, body FROM receipts WHERE rowid>?1 [AND operation IN (SELECT operation FROM revisions WHERE id=?)] ORDER BY rowid LIMIT limit+1`. An unknown `record` → `Error::Missing`.
        - `receipt_count(record: Option<&str>) -> Result<u64>`.
        - `candidate_page(after, limit) -> Result<(Vec<CandidateRow>, bool)>`. Use the same filter as `Store::candidates` (`store.rs:483`: hash not null, `retired=0`, no receipt) plus `rowid>?`. Load each row with the public `self.prepared(...)`.
        - `record_page(kind: Option<&str>, after: &str, limit) -> Result<(Vec<Record>, bool)>`, ordered by `id`.
          - `kind` must be one of the 12 `RecordData` variant names in a `RECORD_KINDS` const, else `Error::Invalid("unknown record kind")`.
          - Bind the JSON path `$.data.<Kind>` as a parameter. Never format it into the SQL.
        - `run_page(thread: Option<&str>, state: Option<RunState>, after: &str, limit) -> Result<(Vec<Record>, bool)>` over `json_type(body,'$.data.Run') IS NOT NULL`, with bound `$.data.Run.thread`/`$.data.Run.state` filters (the serde names of `RunState`).
        - `operation_by_host_key(key) -> Result<Option<OperationId>>`.
        - `operation_info(id) -> Result<OperationInfo>`, reading `hash, retired, undo_of, replacement_of` from `operations`. State is `Applied` if there is a receipt, else `Retired` if retired, else `Prepared` if hash is not null, else `Allocated`. An unknown id → `Error::Missing`.
      - Every limit outside `1..=100` → `Error::Invalid("page limit must be 1–100")`. Queries fetch `limit+1` to set the `more` flag.
      - No `BEGIN`, no DML, no use from desktop.

      In `crates/brn-threads-core/src/lib.rs`, add `mod inspect;` right after `mod backup;` and `pub use inspect::{CandidateRow, OperationInfo, OperationState};` right after it. That is two added lines.
      Tests: `crates/brn-threads-core/tests/inspect.rs`, with data written only through `Store` APIs (follow `tests/core.rs`; for Runs follow `tests/delegation.rs`/`chat_storage.rs`). Cover:
      - pages without repeats while receipts are appended between pages;
      - the `record` filter and `receipt_count`;
      - `candidate_page` excluding applied and retired operations;
      - `record_page` kind filter, order and cursor;
      - `run_page` thread/state filters;
      - `operation_by_host_key` returning `None`/`Some`;
      - `operation_info` in all four states;
      - limits 0 and 101 refused;
      - an unknown kind refused.

      Shared service: ⚑ additive, read-only. Mode: n/a.
      Files: `crates/brn-threads-core/src/{inspect.rs,lib.rs}`, `crates/brn-threads-core/tests/inspect.rs`
      Verify: `cargo test -p brn-threads-core --locked --offline` passes; `cargo build --workspace --locked --offline` succeeds; `git -C … diff --stat 2537db54 -- crates/brn-threads-core/src/store.rs crates/brn-desktop crates/brn-ai` is empty.

- [x] R2-3. **A1 CLI: page mode, new read commands and the value table** (design A1 table; appendix §A1).
      **Value table.** In `spec.rs`:
      - `Grammar { Enum(&'static [&'static str]), Int(u64, u64), AbsolutePath, Key, Cursor(CursorForm), Text }`;
      - `CursorForm { Rowid /*^[0-9]{1,18}$*/, Id /*^[A-Za-z0-9._:-]{1,256}$*/, Version /*decimal ≥1*/, Byte /*decimal*/, VersionStart /*^[1-9][0-9]*\.[0-9]+$*/ }`;
      - `VALUES: &[(command, option, Grammar)]`.

      `parse()` checks every present option that has an entry, after the `required`/`required_when` checks and before anything opens. A bad value → `invalid_value {option, expected}`. This step adds entries only for the new options; R2-11 and R2-12 extend the table.

      **CLI surface** (all class `read`, `owner_decision:false`). The default and `--cursor` behavior in each row comes from the design A1 table.

      | Command | Options | Paging |
      |---|---|---|
      | `history` | `[--record ID] [--limit 1..100 =20] [--cursor ROWID]` | page mode when any of the three is given |
      | `candidates` | `[--limit 1..100 =20] [--cursor ROWID]` | `total: null` (the design names no count function) |
      | `records` | `[--kind Note\|Thread\|…] [--limit 1..100 =20] [--cursor ID]` | `total: null` |
      | `revisions` (new) | `--note ID [--limit 1..32 =32] [--cursor VERSION]` | `cursor` = last version shown when the CLI truncated, else core `next_after` |
      | `revision` (new) | `--note ID --version V [--limit 1..131072 =65536] [--cursor BYTE]` | result = `NoteSlice` + `cursor` (`next_start`) |
      | `runs` (new) | `[--thread ID] [--state working\|interrupted\|completed\|cancelled\|failed] [--limit 1..100 =20] [--cursor RUN]` | `total: null` |
      | `read` | `--id ID [--limit 1..131072] [--cursor V.START]` | page mode only with `--limit`; `--cursor` without `--limit` → `missing_option {option:"limit"}` before opening |

      Command details:
      - **`read` paging:**
        - Every page uses `read_note_range(id, v, start, limit, false)`. The first page takes `v` from `store.record`; there, missing → `not_found`, and not a note → `invalid_request` "record is not a note".
        - The result is `NoteReadOutcome` plus `cursor` `"{v}.{next_start}"`.
        - The envelope `outcome` is `Read` or `Stale`. `Stale` → status `stale` (3), with `conflict` naming the note at its current version (extend `contract::conflict_ids` with the Stale outcome's `id`).
      - **Summaries:**
        - Receipt writes are `{id, kind, version, archived, created}`, at most 32, plus `writes_total`.
        - Candidate writes are summarized from `request.writes` as `{id, kind, version: expected_version, archived, created: expected_version.is_none()}`, with `inputs_total`.
        - `reason`, `authority` and run `progress` are cut at 512 chars; record `title` at 160.
        - A `runs` entry is `{id, version, archived, thread, state (lowercase), provider, model, effort, budget, fence, progress, operations_total, guards_writes}`. `guards_writes` = state Working and not archived.

      Envelope `page` is set on every page and on `search` (`{limit, shown, cursor: page.cursor, total: page.total}`) through a new `Annotations.page`. Pages never repeat. README "Inspection" states which pages can miss items created while paging: restart without a cursor for a full pass.
      Description per command: `page: {limit:{min,max,default}, cursor:<form>}` or null. `bounded`: `true` for page commands; `false` for flag-less `history`/`candidates`/`records`/`notes`/`threads`/`needs-you`/`recovery`/`projects`, with `bounded_form` naming the page form or null. `values` from VALUES.
      Spec entries:
      - add the options to `history`/`candidates`/`records`/`read`;
      - add `revisions`, `revision`, `runs` (Read, `ReceiptAt::None`);
      - `read` outcomes `["Read","Stale"]`, documented as "with --limit".

      AGENT_HELP gains one line naming the page commands. README:
      - adds retry-table rows for the three new commands;
      - adds an "Inspection" subsection: cursors, bounds, the missed-items rule, `guards_writes`;
      - adds "Changes from contract 1" rows.

      Shared service: none beyond R2-2 (read-only calls). Mode: see table above.
      Tests (`crates/brn/tests/agent_inspect.rs`; all fail at baseline as `unknown_option`/`unknown_command`; no guards):
      - **Success:**
        - `history_pages_are_bounded_stable_and_filter_by_record`: 25 notes, pages of 10, both modes, writes interleaved between pages, no repeats, the 32-write cap, `--record`, flag-less output unchanged and equal to the concatenation of all pages;
        - `revisions_and_revision_read_page_through_a_large_note`: a 200 KiB Unicode note via `--text-file`, saved 3 times, reassembles byte-exactly;
        - `records_page_by_kind_and_cursor`;
        - `runs_page_validates_options_and_lists_nothing_on_a_fresh_workspace`: `runs:[]`, and an unknown well-formed cursor gives an empty page.
      - **Invalid input:** `page_options_are_refused_before_opening`, in both modes, on a missing dir that stays absent:
        - `history --cursor abc` and `--limit 0`/`101`;
        - `runs --state bogus`;
        - `records --kind Bogus`;
        - `read --cursor 1.0` without `--limit`;
        - `revision --cursor -1`.
      - **Conflict/refusal:**
        - `read_pages_detect_a_change_between_pages`: exit 3, `outcome:"Stale"`, `conflict[0]` at the current version; a Thread id → `invalid_request`;
        - `history --record <unknown>` → `not_found` (6);
        - `revision --cursor <offset inside a multibyte char>` → `invalid_request`.
      - **Retry:** `identical_page_reads_return_identical_pages` (read class: same argv, same page, nothing created).

      Files: `crates/brn/src/{spec.rs,main.rs,contract.rs}`, `crates/brn/README.md`, `crates/brn/tests/agent_inspect.rs`
      Verify:
      - `cargo test -p brn --locked --offline` passes;
      - clippy is clean;
      - the baseline run of `agent_inspect` fails, and every failure is recorded;
      - `python3 scripts/cli_journey.py "$PWD/target/debug/brn"` still passes.

- [x] R2-4. **A1 CLI: `operation`, `cli_key`, and the evaluator migration** (design A1 "Keys", "Migrate").
      **Key refactor.**
      - Move the `--key` branch of `Invocation::host_key` (`main.rs:302-310`) verbatim into `spec::cli_key(cmd, key) -> String`. It returns `"cli-key:{canonical}:{key}"` and resolves `alias_of`. `host_key` calls it, and write paths keep passing this unprefixed key.
      - Add `spec::stored_host_key(cmd, key) -> Option<String>` (lookup only). Each entry gets a comment citing the app line that adds the prefix:

        | Command | Stored host key | Prefix source |
        |---|---|---|
        | `note-create` | `new-note:` + cli_key | `lib.rs:243` |
        | `thread-start` | `new-thread:` + cli_key | `lib.rs:266` |
        | `undo` | `owner-undo:` + cli_key | `lib.rs:471` |
        | `archive`, `resolve`, `unarchive`, `queue` | `chat-control:` + cli_key | `chat.rs:20` |
        | `keep-in-brn` | `keep-in-brn:` + cli_key | `intake.rs:267` |
        | `intake` | `full-import:` + cli_key | `intake.rs:82` |
        | the nine project commands | `project:` + the raw key | `projects.rs:15` |
        | `topics-create` | `new-note:topics:` + cli_key | added in R2-6 |
        | `knowledge-check` | `knowledge-review-message:` + cli_key | added in R2-6 |

      - Unit test `cli_keys_are_byte_identical_to_round_one` asserts the exact round-1 strings for every keyed command, keyed and unkeyed prefixes included.

      **CLI surface: `operation --id OP | --key K --command NAME`** (read class, `owner_decision:false`). All forms are checked before opening:
      - both forms → `conflicting_options [id,key]`;
      - `--key` without `--command`, or the reverse → `missing_option`;
      - neither → `missing_option {option:"id"}`;
      - `--command` not a keyed command → `invalid_value {option:"command"}` "--command must name a command that takes --key";
      - `--key` must match the key pattern.

      Result: `{operation, state: applied|prepared|retired|allocated, undo_of, replacement_of, request_hash, reason, inputs[{record, version, expectation}]≤64, inputs_total, writes[]≤32, writes_total, receipt:{committed_at, authority}|null, related[]}`.
      - Writes come from the receipt, else from the prepared request.
      - An unknown id, or a key never allocated → `not_found`. The lookup is a pure SELECT and never allocates.
      - `related` lists the other operation ids of a two-operation command (`knowledge-check --seed-chat yes`: the chat operation; R2-6). It is `[]` otherwise.
      - README: one key per project change, because project commands share `project:{KEY}`. The envelope `key` echoes the lookup key, and `retry` stays `read`.

      **Evaluator.** In `scripts/evaluate-wiki-guidance.py`, replace `read_operation_inputs` (`:84-100`) with `cli.call('operation', id=…)`:
      - skip `not_found` and `allocated` (today's null-`request` skip);
      - `applied = state == 'applied'`;
      - `expectation` defaults to `Current`;
      - the output shape is unchanged;
      - `inputs_total > len(inputs)` raises `SystemExit(f"operation {id}: inputs truncated ({n}/{total})")`;
      - remove `import sqlite3`.

      In `scripts/tests/test_wiki_guidance_eval.py`:
      - `test_applied_inputs_are_available_without_request_bodies` uses a stub `cli` (applied, allocated, and `SECRET` in writes never emitted);
      - new `test_truncated_operation_inputs_fail_loudly`;
      - new `test_evaluator_does_not_import_sqlite3` (parses the source with `ast`).

      Shared service: R2-2 functions only (read-only).
      Tests (`agent_inspect.rs`; no guards):
      - **Success/retry:** `operation_by_key_finds_every_keyed_command`. It covers every round-1 keyed command, after a first call and after its keyed replay (same id), plus one project key looked up through `project-create` and `project-chat`, which gives the same operation.
      - **Conflict:** `candidates_and_operation_report_unapplied_candidates`: a deferred keyed undo shows `prepared` in `candidates --limit 1` and `operation`; after discarding the session, the keyed retry makes it `applied` with the same id.
      - **Invalid input:** `operation_lookup_refuses_bad_forms_and_never_allocates`: all four bad forms; `not_found` for an unknown id and an unused key; after looking up unused key K, `note-create --key K` still creates normally.

      Files: `crates/brn/src/{spec.rs,main.rs}`, `crates/brn/README.md`, `crates/brn/tests/agent_inspect.rs`, `scripts/evaluate-wiki-guidance.py`, `scripts/tests/test_wiki_guidance_eval.py`
      Verify: `cargo test -p brn --locked --offline` passes; `python3 -m unittest discover -s scripts/tests` passes; the baseline run of the new tests fails.

- [ ] R2-5. **A4: SIGTERM, `--timeout`, hard stop, `busy`** (design A4; appendix §A4). CLI only.
      **Stop state.** Add `static STOP: AtomicU8` (0 none, 1 sigint, 2 sigterm, 3 deadline) and `fn stop_reason()`:
      - A raised STOP wins.
      - If `CANCEL` is set while STOP is still 0, the reason is `sigint`, so `on_sigint` and the `libc::signal(SIGINT…)` block stay **untouched**.
      - The SIGTERM handler and the deadline watchdog set STOP only if it is 0 and `CANCEL` is unset (first wins), then set `CANCEL`. That makes every round-1 observation point serve every reason.

      **SIGTERM.** In machine mode only, after `parse()`, a new `install_sigterm()` (separate from the SIGINT block) uses `libc::sigaction` with `SA_RESETHAND`. A second SIGTERM therefore kills the process. Default mode keeps `SIG_DFL`.
      **`--timeout SECONDS`** (global). This step accepts it in the pre-command position: `--data-dir V` may be followed by `--timeout V`. R2-12 makes it position-free.
      - Validated before opening: integer `1..86400` → else `invalid_value {option:"timeout"}`; repeated → `repeated_option`.
      - A watchdog `std::thread` polls every 25 ms and sets STOP=deadline at the deadline.

      **New observation points** act only on sigterm/deadline: right after `Workspace::open` (`main.rs:583`), and after index readiness in `search`, `connections` and `ask` (after `search_handle()`). SIGINT points are unchanged. Once `export`, `backup`, `restore`, `upgrade` or an index build has started, it runs to completion.
      **Error and outcome mapping.** In `finish`, an `interrupted`-code error is restamped by `stop_reason()`:
      - sigterm → `terminated` (new status `terminated`, exit 143);
      - deadline → `timeout` with `{deadline_seconds, hard:false}`, status failed (7).

      For an ask/useful-intake terminal `Interrupted`:
      - sigint → `interrupted` 130 (today);
      - sigterm → `terminated` 143;
      - deadline → `failed` 7 with outcome `Interrupted` and `result`;
      - no stop → `stale` (today).

      The envelope `stop` = `stop_reason()`. A call already in flight is never aborted, and its real outcome is reported.
      **Hard stop.**
      - Add `static EMITTED: AtomicBool`. The watchdog also polls for STOP ∈ {sigterm, deadline}; 3 s after it is set, if no envelope has been emitted, it claims `EMITTED`.
      - In machine mode it writes one complete envelope line: status `failed`/`timeout` (exit 7) or `terminated` (143); `details.hard: true` (plus `deadline_seconds` for a deadline); message "Stopped; the command may have committed. Read state as the retry class describes before retrying."; `stop` set.
      - In default mode it writes the stderr text from the mode table, exit 1.
      - It then calls `libc::_exit`.
      - `emit_envelope` and the default-mode writers claim `EMITTED` first and write nothing if the watchdog has already claimed it.
      - README leftovers: a Working run (see `runs`), conversion scratch, the restore stage, backup/export temporaries (swept after the daily merge).
      - Tests never hard-stop `intake`, `backup`, `export`, `restore` or `upgrade`.

      **`busy`.** In `From<brn_threads_app::Error>`, `Error::Sqlite(e) if e.sqlite_error().is_some_and(|f| matches!(f.extended_code & 0xff, 5 | 6))` → `busy` (failed, 7), with `default_text` = today's message. Do the same for `RetrievalError::Sql(e)`. This uses rusqlite methods on the value, so no new dependency. Meaning: "SQLite stayed locked past the 5 s busy timeout. This transaction did not commit, although earlier steps of the command may have."
      **C2.** `Code::Timeout.meaning()` reads: "Document conversion exceeded its time limit, or the --timeout deadline passed before the command committed (details.deadline_seconds; details.hard is true when the process stopped without reaching a safe point and may have committed)." `retry` stays a class string.
      **Docs.**
      - README:
        - a `terminated` row in the statuses table;
        - `busy` and `terminated` rows in the error table;
        - the `timeout` row updated;
        - "Streams, SIGINT and time" rewritten for the three stop reasons, observation points, the hard stop and its leftovers;
        - the killed-`ask` recipe: `runs --state working` shows `guards_writes:true`; a new `ask` marks the run Interrupted; or, on the owner's instruction, `pause`/`archive` stops it. It shows `Cancelled` before the daily merge and `Interrupted` after.
        - "Changes from contract 1" rows.
      - `AGENT_HELP` lists 143 and `--timeout`.

      Mode: per the mode table.
      Tests (`crates/brn/tests/agent_bounds.rs`; Unix where signals are used):
      - **Success:** `timeout_with_time_to_spare_changes_nothing` (`--timeout 60 note-create` → ok, `stop:null`; baseline: `missing_data_dir`/unknown).
      - **Invalid input:** `timeout_values_are_refused_before_opening` (`0`, `86401`, `abc`, repeated; both modes; dir absent).
      - **Conflict/refusal:**
        - `sigterm_during_stdin_wait_is_terminated_and_creates_nothing` (143, `stop:"sigterm"`, data dir absent);
        - `timeout_during_fifo_wait_is_a_soft_deadline` (exit 7, `timeout`, `hard:false`, `stop:"deadline"`, under 2 s with `--timeout 1`);
        - `held_write_lock_reports_busy_and_creates_nothing`: a `python3` child holds `BEGIN IMMEDIATE` on the test's own `threads.sqlite3` with no DML and then rolls back; the test fails if `python3` is missing; `--json note-create` → `busy` after about 5 s; history unchanged;
        - `held_lock_past_deadline_hard_stops_with_one_line` (`--json --timeout 1 note-create --title T --text x`: exactly one stdout line, `timeout`, `hard:true`, exit 7 at about 4 s; after release, `history --limit 1` unchanged);
        - `sigint_envelopes_report_stop_sigint`.
      - **Retry:** `busy_then_keyed_retry_applies_once` (keyed `note-create` refused `busy` under the lock; after release, the identical argv → ok; a third call replays the same `operation`; one note).
      - **Guards:** `default_mode_sigterm_keeps_default_disposition` (killed by signal 15, empty stdout; it passes at baseline by design: compatibility).

      Files: `crates/brn/src/{main.rs,contract.rs,spec.rs}`, `crates/brn/README.md`, `crates/brn/tests/agent_bounds.rs`, `scripts/cli_journey.py` (known statuses gain `terminated`), `scripts/tests/test_cli_journey.py`
      Verify:
      - `cargo test -p brn --locked --offline` passes;
      - clippy is clean;
      - `bash scripts/verify-storage.sh` tail (`brn --help` 0, `brn` 1, `brn --unknown` 1) is unchanged;
      - the baseline run of `agent_bounds` fails on every non-guard test.

- [ ] R2-6. **⚑ A3: retry keys for topics-create and seeded knowledge-check; replay keyed intake before conversion** (design A3; appendix §A3).
      Read BOARD.md first. Re-run the overlap check for app `keyed.rs`, `lib.rs`, `topics.rs`, `knowledge.rs` and `intake.rs`. If any `work/knowledge-*` branch now touches `seed_knowledge_review`, defer the knowledge-check row and record it.
      **⚑ App `crates/brn-threads-app/src/keyed.rs`** (new; `mod keyed;` after `mod knowledge;` in `lib.rs`). Every function takes the unprefixed `cli_key`:
      - `create_topics_note_with_key(k)`:
        - calls `new_note_with_key(&format!("topics:{k}"), "Topics", STARTER)`, then `choose_topics_note`;
        - a retry replays the note and chooses it again;
        - in `topics.rs`, `STARTER` becomes `pub(crate)` (one-word change).
      - `seed_knowledge_review_with_key(k, &check)`, receipt-first:
        - Chat: if `operation_by_host_key("new-thread:knowledge-review-chat:{k}")` has a receipt, use that Thread. Otherwise call `new_thread_with_key(&format!("knowledge-review-chat:{k}"), &title)`.
        - Message: the same receipt check on `knowledge-review-message:{k}`. Otherwise a private `keyed_owner_message` sends exactly `post_message`'s request (`lib.rs:287-307`: role `owner`, reason "Owner message", `HostAuthority::owner("owner")`) under host key `knowledge-review-message:{k}`.
        - The first findings win. Changed findings after a crash past prepare → `ImmutableRequest`.
        - The text comes from `review_seed(check) -> (String, String)` (title, text), a pure extraction from `seed_knowledge_review` (`knowledge.rs:550-608`), which then calls it. Behavior unchanged.
      - `full_import_replay(key, title, source_reference, source_bytes, scope) -> AppResult<Option<ApplyOutcome>>`:
        - With no operation for `full-import:{key}`, or no receipt → `None`.
        - Otherwise classify the receipt writes with the design A3 fact table (Source via `same_source(locator, ref) && chat == scope`; the Link relation `FULL_IMPORT_PREFIX + hex(sha256(bytes))` to that Source; the `import.full_note` Note for that Source with the same title):
          - all three match → `Some(store.apply(&op, &HostAuthority::owner("full-note import request")))`;
          - any differs → `Err(Error::ImmutableRequest)`;
          - Link or Note absent → `None`.
        - Private `same_source` is an exact string match on this branch. Its comment (C1) says that at merge it becomes `same_locator(stored, &canonical_locator(ref))`, citing daily "count one file as one source regardless of path spelling" by subject plus its current hash (`344331f2` at review).

      `intake.rs` is untouched; it is a semantic dependency only.
      **CLI.**
      - `topics-create`: `keyed`, default `duplicate`, keyed `converge`.
      - `knowledge-check`:
        - `keyed:true`;
        - variant `seed-chat=yes` becomes `{retry: duplicate, key: true, keyed: Some(replay)}` (`Variant` gains `keyed: Option<Class>`; `retry_of` honors it under `--key`);
        - new `Spec.key_requires: Option<(&str, &str)> = Some(("seed-chat","yes"))`. `--key` without it → `conflicting_options [key, seed-chat]` "--key needs --seed-chat yes", before opening;
        - envelope `operation` = the message op; `related` (through `operation`) = the chat op.
      - Intake: in the keyed `full|chat|project` arm, right after the `intent` match and before `let settings = workspace.settings()?` (`main.rs:903-904`, outside the sub-block), call `full_import_replay(cli_key, title, source, &bytes, scope)`. On `Some`, return `applied(outcome)` through `fill_conflicts`, without converting or printing progress. Useful intake and `ask` get no key.
      - Description: `key_requires`; variants show `keyed`.
      - `stored_host_key` gains the two R2-4 rows.

      **C2 prose.** The README Retries section and `AGENT_HELP` get the replay sentence verbatim: "A keyed full/chat/project intake replays when bytes, path, scope and title match a committed import, without converting or sending the document again; if that import found the bytes already imported, the retry converts again and records only an observation." R2-12 adds it to `intake` help. Other README additions: use a new `topics-create` key if Topics changed in between; the `ask` recovery path (`runs --thread T --limit 5`, then `read --id RUN`).
      Shared service: ⚑ app additive. `topics.rs` gets one visibility word; `knowledge.rs` gets the extraction only.
      App tests (`crates/brn-threads-app/tests/keyed_owner_operations.rs`, n/a at baseline):
      - `keyed_topics_create_converges`;
      - `keyed_seed_chat_replays_first_findings`;
      - `keyed_full_import_replay_does_not_bypass_an_archived_chat` (prepared and unapplied → `None`, then "Choose an active chat…");
      - `keyed_full_import_replay_detects_a_changed_title`;
      - `keyed_full_import_replay_matches_html_source_bytes` (a synthetic `ConversionResult`, as `import_coverage.rs` builds one);
      - `keyed_full_import_replay_after_reimport_is_none`.

      Process tests (`crates/brn/tests/agent_retry.rs`, Markdown only, `TMPDIR` inside the root):
      - **Success/retry:**
        - `keyed_topics_create_retries_without_a_second_note`;
        - `keyed_seed_chat_replays_and_ignores_later_findings`;
        - `keyed_intake_retry_replays_before_conversion_through_a_symlinked_folder` (`<root>/link/doc.md` with `link → real`: the retry prints no "Reading the document…", keeps the same `operation` and stays replay class).
      - **Invalid input:** `knowledge_check_key_requires_seed_chat` (`conflicting_options`, dir absent).
      - **Conflict/refusal:**
        - the changed-title intake case → `key_conflict`, with no progress line;
        - `keyed_undo_with_a_different_original_is_key_conflict` (exit 6, `details.key`, history unchanged);
        - `keyed_keep_in_brn_with_a_different_note_is_key_conflict`.
      - **Guards:** the last two pass at baseline, because round 1 already refused key reuse. They close round-1 gap 11 and are listed as guards.

      Files: `crates/brn-threads-app/src/{keyed.rs,lib.rs,topics.rs,knowledge.rs}`, `crates/brn-threads-app/tests/keyed_owner_operations.rs`, `crates/brn/src/{main.rs,spec.rs}`, `crates/brn/README.md`, `crates/brn/tests/agent_retry.rs`
      Verify:
      - `cargo test -p brn-threads-app --locked --offline` and `cargo test -p brn --locked --offline` pass;
      - `cargo build --workspace --locked --offline` succeeds;
      - `git diff 2537db54 -- crates/brn-threads-app/src/intake.rs crates/brn-desktop crates/brn-ai` is empty;
      - the baseline run of `agent_retry` fails on every non-guard test.

- [ ] R2-7. **⚑ A10: no default credentials in CLI-created workspaces (owner B2)** (design A10; appendix §A10).
      Read BOARD.md first.
      **⚑ App.** `Workspace::open_without_default_credentials(data)` sits beside `open`. Both call a private `open_with(data, DefaultCredentials::{Derive, None})`, which passes the choice to `bootstrap_settings`. `None` writes `auth_file: None`. `open` behaves exactly as before, and desktop keeps calling it. Daily edits `settings.rs` only from :98, so the change stays inside `bootstrap_settings` (:21-74).
      **CLI.** `main.rs:583` calls the new function. Existing workspaces keep their Settings.
      **Machine-mode refusal.** When `auth_file` and `credentials_dir` are both `None`, return `credentials_not_configured` (rejected 6, `{provider}`, retry `refuse`). Message: "No provider credentials are configured for this workspace; run configure --provider … with --auth-file ABS or --credentials-dir ABS". It is checked:
      - right after `let settings = workspace.settings()?;` at the top of `fn ask` (`main.rs:434`), before `search_handle()` and `brn_ai::threads::run`;
      - in the intake arm (C4), using the single existing `let settings = workspace.settings()?;` at `main.rs:904`: the check sits right after it and before `converter(kind)?` and `convert`; the same `settings` is reused for the `ConversionJob`. It applies when `intent == useful` or `kind ∉ {Markdown, Text}`.

      A code comment and the README say that `fn ask` also runs useful intake, so the intake-arm check fires first and the `fn ask` check guards both. No Run, Message, chat or import has been committed at that point. A keyed retry still replays (the A3 replay comes earlier). Default mode keeps today's path.
      **In-repo impact.**
      - `agent_contract.rs::ask_refused_before_the_provider_is_rejected_not_failed` first runs `configure --provider Copilot --model synthetic --effort low --credentials-dir <root>/creds`. Its `tool_rejected` assertions and default text are unchanged.
      - `cli_journey.py:213` requires `auth is None`.
      - The live Codex arms of `evaluate-wiki-guidance.py` (near `:374-375`) and `evaluate_wiki_v2.py` (near `:1793-1795`) call `parser.error("Codex needs an absolute --auth-file")`, with one unit test each in `scripts/tests/test_wiki_guidance_eval.py` and `scripts/tests/test_evaluate_wiki_v2.py`.
      - `live-journeys.py` already passes `--auth-file`; no change.

      **Docs.** README: the B2 rule; the recipe `configure --provider … --auth-file ABS | --credentials-dir ABS`; and the C4 desktop sentence "Desktop shows no sign-in file for such a workspace; choose one in Settings before running work." Also a `credentials_not_configured` error row, §2.3 item 12 under Compatibility, and a "Changes from contract 1" row.
      Mode: per the mode table.
      Tests:
      - App test in `keyed_owner_operations.rs`: `open_without_default_credentials_stores_no_auth_file` (it never calls `open` on a fresh dir, so the test process's real HOME is never read).
      - Process tests in `crates/brn/tests/agent_credentials.rs`:
        - **Success:** `cli_created_workspace_has_no_credential_source` (`auth_file` null even with a synthetic `<root>/home/.codex/auth.json` and `CODEX_HOME` present; baseline shows the derived path).
        - **Invalid input/refusal:**
          - `ask_without_credentials_is_refused_before_any_commit` (exit 6; history and `runs` unchanged);
          - `provider_intake_without_credentials_is_refused_before_conversion` (`.html` full and `.md` useful are refused with no progress output and no commit; `.md` full still works).
        - **Retry:** `refused_ask_stays_refused_and_records_nothing` (identical argv twice → 6 both times, no Run).
      - **C5 guard** in `crates/brn/tests/agent_credentials_existing.rs`, one `#[test]` only: `existing_workspace_keeps_its_stored_auth_file`. It calls `std::env::set_var("CODEX_HOME", <root>/home)`, then in-process `brn_threads_app::Workspace::open(<root>/data)`, then CLI `--json settings` → `auth_file == <root>/home/auth.json`. No `ask` or provider intake. It passes at baseline (compatibility guard).
      - `configure_then_ask_reaches_the_provider` lands in R2-8.

      Files: `crates/brn-threads-app/src/{lib.rs,settings.rs}`, `crates/brn-threads-app/tests/keyed_owner_operations.rs`, `crates/brn/src/{main.rs,contract.rs}`, `crates/brn/README.md`, `crates/brn/tests/{agent_credentials.rs,agent_credentials_existing.rs,agent_contract.rs}`, `scripts/cli_journey.py`, `scripts/evaluate-wiki-guidance.py`, `scripts/evaluate_wiki_v2.py`, `scripts/tests/{test_wiki_guidance_eval.py,test_evaluate_wiki_v2.py}`
      Verify:
      - `cargo test -p brn -p brn-threads-app --locked --offline` passes;
      - `cargo build --workspace --locked --offline` succeeds;
      - `python3 -m unittest discover -s scripts/tests` passes;
      - `cli_journey.py` passes;
      - the baseline run of `agent_credentials` fails.

- [ ] R2-8. **A2: offline outcomes through a test-only stub, and the witness repair** (design A2; appendix §A2). No production change.
      (a) **Spike, at most 90 min, recorded in evidence.** Confirm four things with a throwaway test against the rig 0.44 Copilot provider:
      - a 0600 `copilot.json` `{"token":"SYNTHETIC","expires_at":4102444800,"endpoints":{"api":"http://127.0.0.1:PORT"}}` in a 0700 dir skips the token refresh;
      - with `HTTPS_PROXY`/`HTTP_PROXY`/`ALL_PROXY=http://127.0.0.1:9` and `NO_PROXY=127.0.0.1,localhost`, `endpoints.api=http://192.0.2.1` is never contacted;
      - no `github-token` file is needed;
      - minimal SSE frames ending in `[DONE]` are accepted.

      If any check fails: make no production change, record the result, keep the unit and app `agent_journeys.rs` coverage, list the gap, and apply the C4 fallback.
      (b) If the spike passes, add `crates/brn/tests/support/stub.rs`: a std-only `TcpListener` loopback stub (~150 lines) that serves scripted SSE, a held stream, or HTTP 500, and records every request. The test fails if an expected request is missing. Every child gets fresh homes and `TMPDIR`, the proxy variables, and `BRN_*` cleared. Credentials come only from the synthetic file.
      Scenarios in `crates/brn/tests/agent_runs.rs` (appendix §A2; scripts mirror app `tests/agent_journeys.rs`):
      - **Success:** `stub_ask_completes`, and `configure_then_ask_reaches_the_provider` (A10). C4 fallback if the spike failed: that row is covered by the configured `ask_refused_before_the_provider_is_rejected_not_failed`, and the gap is recorded.
      - **Invalid input:** `stub_ask_on_an_unknown_chat_never_contacts_the_stub` (`invalid_request` `tool_rejected`, and the stub saw no request).
      - **Conflict/refusal:**
        - `stub_provider_error_ends_failed` (500 → Failed, exit 7);
        - `stub_candidate_goes_stale_after_owner_edit` (`apply` → exit 3);
        - `stub_killed_ask_leaves_a_working_run`: after SIGKILL, `runs --state working` shows `guards_writes:true` and a chat write is `deferred` with `conflict[].kind == "Run"`; after `pause`, the run is not `Working` and `guards_writes:false`. Assert "not Working", never `Cancelled`, per design §1.1(d).
      - **Retry:**
        - `stub_model_candidate_review_apply_and_dismiss` (`delegation --choice review-first`; `candidates --limit 1` lists it; `operation` shows `prepared`; `apply` → Applied; a repeated `apply` replays the same operation; a second candidate is dismissed);
        - `runs_page_filters_by_thread_and_state`;
        - `stub_sigint_interrupts_a_held_stream` (exit 130, `stop:"sigint"`, visible through `runs --state interrupted`; default mode exits 0 with `"terminal":"Interrupted"`, the `live-journeys.py` guard);
        - `stub_sigterm_interrupts_a_held_stream` (143);
        - `stub_deadline_interrupts_a_held_stream` (`--timeout 2` → exit 7, `stop:"deadline"`).

      (c) **NeedsReview/Superseded audit.** Check whether any CLI command reaches the model-side authorities at `store.rs` ~1080, 1150, 1240 and 1640. Add a stub test if one does; otherwise record "not reachable from CLI owner authority" with references. The README says which outcomes are reachable.
      (d) **Witness** `docs/work/active/chats-and-projects/cli-witness.py` (default mode):
      - delete lines 14–29 (the SQLite v3 fake) and the `sqlite3` import, keeping the CLI `read` on line 30;
      - lines 35 and 40 use `['hits']`;
      - add `upgrade --backup-directory <abs>` on the current workspace (exit 0, `history --limit 1` identical before and after);
      - run under a fresh `TMPDIR` whose parent ends up empty;
      - repoint the old-format evidence row in `evidence.md` to `chat_storage.rs::adopted_upgrade_requires_external_backup_and_preserves_the_v3_snapshot` (invariant 17: the CLI can't produce an old format).

      Shared service: none. Mode: tests; the witness runs in default mode.
      Guards: the default-mode half of `stub_sigint_interrupts_a_held_stream`.
      Files: `crates/brn/tests/{support/stub.rs,support/mod.rs,agent_runs.rs}`, `docs/work/active/chats-and-projects/cli-witness.py`, `crates/brn/README.md`, `docs/work/active/cli-agent-contract/evidence.md`
      Verify:
      - `cargo test -p brn --locked --offline --test agent_runs` passes;
      - `python3 docs/work/active/chats-and-projects/cli-witness.py target/debug/brn` exits 0 with an empty temp parent;
      - the baseline run of `agent_runs` fails (no `runs`, `stop` or `--timeout`).

- [ ] R2-9. **A12: stale edits stay conflicts; `edit_session_open` never names the session (owner B3 part 1)** (design A12; appendix §A12).
      In `contract.rs::begin_edit_error` (`:434-456`), drop the `.with("session", …)` line, so `details` = `{note, base_version}`. Keep the `Sqlite` + `recovery_buffers()` inference on this branch; at merge it maps the typed `Error::EditSessionOpen` (recorded in the merge notes).
      **C2.** The `CliError.message` and `Code::EditSessionOpen.meaning()` text: "This note has an open unsaved edit session (details.note, details.base_version). Write to it only if your own successful begin-edit returned it; otherwise report it to the owner." Default-mode text is unchanged on this branch.
      **Recipe (C7)**, verbatim in the README "Recovery recipes" (`:241`) and `AGENT_HELP`. R2-12 puts it in `save` help.
      1. Keep `result.session.markdown`.
      2. `discard --session <result.session.id> --generation <result.session.generation>`.
      3. `begin-edit --note N --base <conflict[0].version>`.
      4. Reapply your edit.
      5. `update-buffer --session NEW --generation 1 --text-file merged.md`.
      6. `save --session NEW --generation 1`.

      Known limit (README): until round 3 (B3 part 2, options-only here), an agent that lost its `begin-edit` response can't recover the session from the error; `recovery` still lists sessions for the owner.
      Updates:
      - `agent_contract.rs::open_edit_session_reports_edit_session_open` expects `{note, base_version}`;
      - the `contract.rs` unit test near `:1182`;
      - README `:132`, `:215`, `:243`, `:276`;
      - `cli_journey.py:267` asserts `'session' not in details`;
      - a "Changes from contract 1" row.

      Mode: machine `details` only.
      Tests (`crates/brn/tests/agent_edits.rs`):
      - **Refusal:** `edit_session_open_never_names_the_session` (both modes, both streams contain no session id; fails at baseline).
      - **Success:** `stale_save_recovery_recipe_from_the_readme_completes` follows the six steps by parsing them from the README recipe at compile time with `include_str!`. The version goes up by one with the merged text, and the discarded session then gives `Closed`. **Guard:** the recipe already worked at baseline; the test protects the documented text.
      - **Retry:** `recipe_save_retry_replays_saved` (an identical `save` after step 6 → `Saved`, same `operation`; guard: round-1 behavior).
      - **Invalid input:** n/a. No new input surface; round-1 `stale_generation_closed_and_ignored_exit_3_with_current_versions` covers generations.

      Files: `crates/brn/src/{contract.rs,main.rs}`, `crates/brn/README.md`, `crates/brn/tests/{agent_edits.rs,agent_contract.rs}`, `scripts/cli_journey.py`
      Verify: `cargo test -p brn --locked --offline` passes; `cli_journey.py` passes; the baseline run of `edit_session_open_never_names_the_session` fails.

- [ ] R2-10. **⚑ A11: no-op converge commands record nothing (owner B4)** (design A11; appendix §A11; B4 coordination exception).
      Read BOARD.md first, and record the current daily tip.
      **⚑ Core** (owner-approved write-path change).
      - New `crates/brn-threads-core/src/converge.rs` (`mod converge;` after `mod inspect;`) defines:
        - `Converged<T> {Changed(T), Unchanged(Unchanged)}` and `Unchanged {record: String, version: u64}`, both re-exported;
        - the private `detect_unchanged(tx, kind, &writes) -> Result<Option<Unchanged>>`.
      - `Store::chat_command_converging(..) -> Result<Converged<ChatCommandOutcome>>` and `Store::project_command_converging(..) -> Result<Converged<Receipt>>`.
      - In `chat.rs` (`:109`) and `projects.rs` (`:310`), the existing bodies become private `*_body(.., converge: bool)`. `chat_command`/`project_command` keep their signatures and pass `false`; the impossible `Unchanged` arm maps to `Err(Error::Invalid("no-op detection is off"))`, without a panic.
      - The hook is a three-line call right before `prepare_on`: after the ChatControl push in `chat.rs` (`:503`), and after the `Vec<Put>` conversion in `projects.rs` (`:772`). Binding, replay and every refusal therefore run first.
      - The hook applies only to `Archive`, `Unarchive`, `MoveChat`, `LinkDocument` and `UnlinkDocument`, not `ArchiveProgress`. It returns `Some` iff:
        - every Put has `expected_version == Some(current.version)`;
        - none creates a record;
        - each `(archived, data)` equals the current record. Project chats/documents compare as sets, and `MoveChat` ignores the `context_epoch` bump.

        An unpaused queue on archive, a fenced Working run or a partial message → `Changed`.
      - `Unchanged` returns without committing (the immediate transaction drops), so the allocated operation stays `allocated`.

      **⚑ App `crates/brn-threads-app/src/converge.rs`** (new; `mod converge;` after `mod keyed;`) adds `archive_chat_converged(key, thread)`, `unarchive_chat_converged(key, thread) -> AppResult<Converged<Receipt>>` and `project_command_converged(key, command) -> AppResult<Converged<Receipt>>`. They repeat the twins' three lines exactly: `chat-control:{key}` + the SHA-256 command binding + "owner chat action" (`chat.rs:16-23`), and `project:{key}` + binding + "owner project action" (`projects.rs:11-20`). Desktop keeps the receipt path; the gap is recorded (`threads_projects.rs:57`, `threads_ui.rs:2183,2203`).
      **CLI.**
      - `archive`/`resolve`/`unarchive`/`chat-move`/`project-link`/`project-unlink` call the converged functions.
      - On `Unchanged`: result `{"outcome":"Unchanged","record":ID,"version":V}`, with `record` per C6 (thread for archive/resolve/unarchive, chat for chat-move, project for project-link/unlink); envelope `ok`, exit 0, `outcome:"Unchanged"`, `operation:null`, `changed:[]`. `classify` `ReceiptAt::Root` recognises `result.outcome == "Unchanged"`.
      - Spec outcomes for the six commands gain `Unchanged`.
      - The `Converge` class meaning changes to say: a no-op archive, resolve, unarchive, chat-move, project-link or project-unlink returns Unchanged and records nothing, and a keyed retry of an earlier change still replays its receipt.
      - Default mode prints `result` (§2.3 item 11). README Compatibility item 11 lists the checked callers: `agent_contract.rs:1313-1318` (keyed, real changes), `chats-and-projects/cli-witness.py:37-39`, and **`docs/work/active/projects/cli-witness.py:49-59` (all real changes)** (C6).

      Write `merge-resolution-ours.diff`/`merge-resolution-main.diff` notes into the artifacts folder for the expected conflicts (design §1.1 B4 exception):
      - core `chat.rs`: the body takes `orphans` and `converge`;
      - core `projects.rs`: keep daily's `Vec<Put>` conversion and `ended_run_attention`, then the hook;
      - app `converge.rs`: mirror daily's wrapper (`lease_free_working_runs()` for Archive, `remove_ended_leases` on `Changed`).

      Tests:
      - Core `crates/brn-threads-core/tests/converge.rs`:
        - all five variants `Unchanged` on a no-op, with receipts and versions unchanged;
        - a real change → `Changed`;
        - the non-converging functions still append on a no-op;
        - a Working run → `Changed` with a fence;
        - a binding mismatch → `ImmutableRequest`.
      - App, in `keyed_owner_operations.rs`: `converged_twins_share_host_keys_and_authority` (keyed `archive_chat(K)` then `archive_chat_converged(K)` → `Changed` with the same receipt).
      - Process tests (`crates/brn/tests/agent_converge.rs`):
        - **Success:** `repeated_archive_and_resolve_are_unchanged_without_a_receipt`; `repeated_unarchive_move_link_unlink_are_unchanged` (`history --limit` count and versions unchanged; `record` per C6).
        - **Conflict:** `archive_after_resume_pauses_and_records_a_change`.
        - **Retry:** `keyed_archive_retry_after_a_change_replays_the_original_receipt`.
        - **Default mode:** `default_mode_unchanged_prints_the_unchanged_object`.
        - **Invalid input:** n/a (no new input surface; round-1 project refusals cover it).

      Files: `crates/brn-threads-core/src/{converge.rs,lib.rs,chat.rs,projects.rs}`, `crates/brn-threads-core/tests/converge.rs`, `crates/brn-threads-app/src/{converge.rs,lib.rs}`, `crates/brn-threads-app/tests/keyed_owner_operations.rs`, `crates/brn/src/{main.rs,contract.rs,spec.rs}`, `crates/brn/README.md`, `crates/brn/tests/agent_converge.rs`, `scripts/cli_journey.py` (an `Unchanged` re-archive step)
      Verify:
      - `cargo test -p brn-threads-core -p brn-threads-app -p brn -p brn-desktop --locked --offline` passes (desktop unchanged);
      - `cargo build --workspace --locked --offline` succeeds;
      - `git diff --stat 8bef439b..HEAD -- crates/brn-desktop crates/brn-ai` is empty;
      - the baseline run of `agent_converge` fails.

- [ ] R2-11. **A6: mistyped data directories and values before opening** (design A6; appendix §A6).
      **Machine mode.** For a read-class invocation (`retry_for == Read`, which covers unseeded `knowledge-check`, `revisions`, `revision`, `operation` and `runs`), before `Workspace::open`: if `--data-dir` doesn't exist, or is an empty directory without `DATA_MARKER` (`brn_threads_app::DATA_MARKER`), return `missing_workspace` (rejected 6, `refuse`), "No BRN workspace at --data-dir; check the path, or run init deliberately". Nothing is created. A foreign non-empty dir stays `unsafe_directory`. `init` and writes still create the workspace.
      **Default mode** is unchanged: reads create the dir. Survey and record every default-mode caller before the change: `threads.rs`, both witnesses, `verify-end-to-end-fixtures.py`, both evaluators, `live-journeys.py`. `agent_contract.rs` gets an `init` wherever it reads a fresh dir in machine mode; `cli_journey.py` already runs `init`.
      **Values before opening**, in both modes, with the same codes and messages as today's after-open checks. Extend `VALUES`:
      - `Int` syntax for `base`, `generation`, `version`, `hops`, search `limit` and `budget` ("--X must be an integer"; budget keeps "Invalid budget");
      - `Enum` for:
        - `knowledge` ("--knowledge takes yes|no"), `seed-chat` ("--seed-chat takes yes"), `sources` ("--sources takes yes");
        - `intent` ("Choose useful, full, or chat with --thread");
        - `provider`/`helper-provider`, case-insensitive codex|chatgpt|copilot ("Provider must be Codex or Copilot");
        - `effort`/`helper-effort` ("Effort must be low, medium or high").

      Service ranges stay after opening. `--trash` validation is a recorded gap. README: Compatibility item 8, the `missing_workspace` error row and recipe, and a "Changes from contract 1" row.
      Tests (`crates/brn/tests/agent_discovery.rs`):
      - **Invalid input:** `enum_values_are_refused_before_the_workspace_opens` (both modes, no dir created, default stderr substrings kept; fails at baseline because the dir gets created).
      - **Refusal:** `machine_mode_reads_on_missing_or_empty_dirs_are_missing_workspace`:
        - `notes`, `revisions`, `operation` and `runs` exit 6 and leave the path absent;
        - an empty dir stays empty;
        - a foreign dir gives `unsafe_directory` (guard part);
        - default-mode `notes` still exits 0 and creates the dir (guard part).
      - **Success/retry:** in the same test, after `init` the identical read returns `ok`, and `note-create` on a missing dir still creates it.

      Files: `crates/brn/src/{main.rs,spec.rs,contract.rs}`, `crates/brn/README.md`, `crates/brn/tests/{agent_discovery.rs,agent_contract.rs}`
      Verify: `cargo test -p brn --locked --offline` passes; `python3 docs/work/active/projects/cli-witness.py target/debug/brn` and the CI fixture step (R2-15) pass; the baseline run fails on the non-guard assertions.

- [ ] R2-12. **A5: per-command help, global options anywhere, absolute paths, owner-decision mark** (design A5; appendix §A5).
      **Per-command help.** `brn [--json] [--data-dir D] COMMAND [...] --help`, with `--help` in any option-name position after a known command, prints help, exits 0 and opens nothing. No `--data-dir` is needed. After an unknown command it gives `unknown_command`.
      - Text help is generated from `spec.rs`: summary, usage, required options with their `-file` forms, values (from `VALUES`), retry (default/keyed/variants), owner decision and `owner_decision_when`, outcomes, example, and a `Notes:` paragraph from a new `Spec.note: Option<&'static str>`.
      - `intake` note = the A3 replay sentence (C2). `save` note = the A12 recipe (C2/C7).
      - `--json` prints `{"contract":2,"command":<entry>}` on one line.
      - `Spec` gains `summary` and `example`. The description adds per-command `summary`, `example`, `values` and `owner_decision_when`, and top-level `global_options` (`data-dir`, `timeout`).

      **C3.** `HELP` stays byte-identical. `brn --help` prints `HELP` + `\n` + the updated `AGENT_HELP`, exit 0. Reword the README/design text to match.
      **Global options anywhere, without clap:**
      1. Strip a leading `--json`.
      2. Before the command, consume `--data-dir V`/`--timeout V`. Any other `--x` before a command, with no `--data-dir`, keeps `missing_data_dir` and the text "Supply --data-dir ABSOLUTE_DIRECTORY; run --help".
      3. After the command, pair `--name value` tokens and lift name-position `--data-dir`/`--timeout` out. A repeat → `repeated_option`.
      4. Value positions are never inspected (`--title --data-dir` stays a title).

      A spec unit test asserts that no command declares `data-dir`, `timeout` or `help`.
      **Absolute paths (§2.3 item 6).** `Grammar::AbsolutePath` for `backup`/`export --destination`, `restore --source`, `upgrade --backup-directory` and `model-install --destination`. A relative path → `invalid_value` "--X must be an absolute path" before opening, in both modes. Re-grep in-repo callers and record that all pass absolute paths before the change.
      **Owner-decision mark.** `intake` `owner_decision_when: [{"option":"thread","names":"Project","note":"attaches the document to that project"}]`, plus a README row and an `AGENT_HELP` line. Behavior is unchanged; the owner question stays open.
      Tests (`crates/brn/tests/agent_discovery.rs`):
      - **Success:** `every_command_has_text_and_json_help_matching_the_description`; `data_dir_and_timeout_are_accepted_after_the_command`.
      - **Invalid input:**
        - `relative_backup_and_restore_paths_are_refused_before_opening` (both modes);
        - `help_after_an_unknown_command_is_unknown_command`;
        - `repeated_global_options_are_repeated_option`.
      - **Retry:** n/a (no state).
      - **Guards:**
        - `brn`, `brn --unknown` and `brn notes` keep exit 1 and "Supply --data-dir";
        - `brn --help` exits 0 and starts with "BRN Threads\n\nUsage: brn --data-dir ABSOLUTE_DIRECTORY COMMAND";
        - `--title --data-dir` stays a title;
        - round-1 `usage_errors_are_json_with_stable_codes_and_create_nothing` stays unchanged.

      Files: `crates/brn/src/{main.rs,spec.rs}`, `crates/brn/README.md`, `crates/brn/tests/agent_discovery.rs`
      Verify:
      - `cargo test -p brn --locked --offline` passes;
      - `git diff -U0 8bef439b..HEAD -- crates/brn/src/main.rs` contains no line starting `-const HELP` or `+const HELP` (constraint check);
      - `bash scripts/verify-storage.sh` tail unchanged;
      - the baseline run fails on the non-guard tests.

- [ ] R2-13. **A7: finer refusal reasons, with no shared-service change** (design A7; appendix §A7).
      In `contract.rs`, add `REFUSAL_REASONS: &[(&str, Match::{Exact, Prefix}, &str)]` with the verbatim appendix §A7 table: `version_mismatch`, `inactive_target`, `project_boundary`, `size_limit`, `wrong_kind`.
      - It applies to `Error::Invalid` (match on the inner text), `AppError::Invalid`, and the CLI's own `invalid_request` texts.
      - It does not apply to `From<String>`, so the sub-block stays coarse.
      - Unmatched messages carry no `reason`.
      - `code` stays `invalid_request`. There is no `protected` reason.

      Drift guard unit test `refusal_reasons_name_live_service_messages`: every literal appears in `include_str!` of the core/app sources it comes from (`brn-threads-core/src/{projects,chat,store,edit,revision_read,read}.rs`, `brn-threads-app/src/{intake,projects,chat,lib}.rs`, `crates/brn/src/main.rs`; the coder confirms each path). README: a `details.reason` column note on `invalid_request`. R2-3's `read` Thread case now also asserts `reason == "wrong_kind"`.
      Tests (`crates/brn/tests/agent_reasons.rs`, no Runs):
      - **Refusal:**
        - `stale_project_settings_reason_is_version_mismatch`;
        - `trashed_project_chat_reason_is_inactive_target`;
        - `incomplete_project_delete_actions_reason_is_project_boundary`.
      - **Invalid input:**
        - `oversized_project_title_reason_is_size_limit`;
        - `search_thread_naming_a_note_reason_is_wrong_kind`.
      - **Retry:** in `stale_project_settings…`, the identical retry stays `invalid_request` with the same reason.
      - **Success:** n/a (refusal-only surface).

      Files: `crates/brn/src/contract.rs`, `crates/brn/README.md`, `crates/brn/tests/{agent_reasons.rs,agent_inspect.rs}`
      Verify: `cargo test -p brn --locked --offline` passes; the baseline run of `agent_reasons` fails (no `reason`).

- [ ] R2-14. **Journey runner, docs, before/after and status.**
      **`scripts/cli_journey.py` gains these steps:** paged `history --record`, `revisions`/`revision` reassembly, `operation --key` for the keyed note, `candidates --limit 1`, an `Unchanged` re-archive, the A12 recipe (replacing today's recover block, `:252-285`), and `COMMAND --help --json` for one command. It must finish with empty stderr and an empty temp parent. Keep `scripts/tests/test_cli_journey.py` in step (statuses, `Unchanged`, contract 2).
      **README.** Finish the "Changes from contract 1" table: every row of design §2 plus each step's additions. Complete Compatibility items 6–12. "Remaining gaps" points to the evidence.
      **Before/after.** In the artifacts `before_after.py` (outside the repo), add every new code, outcome, command and option, run against the R2-1 baseline binary and the new binary, and paste the output into `evidence.md` "Round 2 before and after".
      **Status.** In `docs/status.md`, add one line to the existing CLI agent contract entry (line 7): "Round 2 (paged inspection, SIGTERM/--timeout/busy, more keys, owner B2/B4/B3.1, discovery) implemented on the branch; not merged."
      Files: `scripts/cli_journey.py`, `scripts/tests/test_cli_journey.py`, `crates/brn/README.md`, `docs/status.md`, `docs/work/active/cli-agent-contract/evidence.md`
      Verify:
      - `python3 -m unittest discover -s scripts/tests` passes;
      - `python3 scripts/cli_journey.py "$PWD/target/debug/brn" --evidence-dir /tmp/brn-cli-agent-contract/<run>/journey` passes with `TMPDIR` set to a fresh empty parent and `BRN_*` unset; stderr is empty and the parent ends empty;
      - the same runner against the baseline binary fails at its first contract-2 check;
      - `python3 scripts/check-markdown-links.py` reports 0 failures.

- [ ] R2-15. **Full verification, baseline-failure matrix, overlap and merge checks** (design §4, §1.1 "End of run", C1).
      Run every command in "Verification commands" below from the worktree, with `--locked --offline` and `BRN_*` unset. Record the exact command, tree hash and result in `evidence.md`.
      **Baseline matrix.** For every new process test file, run it against the R2-1 baseline binary and record per test either "fails: <assertion>" or "guard: <reason>".
      **Overlap.** Re-run the overlap check for every ⚑ path against all eleven branches, and record the table.
      **Daily tip (C1).**
      - Record the then-current `work/daily-workflow-reliability` tip with its subject.
      - Re-derive the A11 `chat.rs`/`projects.rs` resolutions against that tip; its Archive path may have moved for board decision 2.
      - Update `merge-resolution-{ours,main}.diff`.

      **Merge checks.**
      1. Run `git -C /Users/evokessler/repos/brn-rust -c core.fsmonitor=false merge-tree --write-tree --name-only work/cli-agent-contract work/daily-workflow-reliability`. Expected conflicts: `crates/brn/src/main.rs` and A11's core hooks, matching the recorded resolutions.
      2. Read BOARD.md.
      3. Scratch test-merge (design §1.1):
         - create `scratch/cli-daily-merge` with `git commit-tree` in a temp worktree under `/tmp/brn-cli-agent-contract/<run>/merge`;
         - apply the recorded resolutions, including the merge-time items in design §1.1(a)–(e): typed `EditSessionOpen` mapping without `session` and the daily `threads.rs` ~:219 assertion edit; the `data_folder_unusable` arm and `symlinked_data_dir_is_data_folder_unusable`; search lanes; `Interrupted` tightening; `same_source` → `same_locator`;
         - run one `cargo build --workspace --locked` for CLI and desktop;
         - run our tests, daily `crates/brn/tests/{threads,source_locator,search_messages}.rs`, `keyed_intake_retry_replays_before_conversion_through_a_symlinked_folder`, and daily `cli-journey.py` with a fresh `/private/tmp/...` ROOT only (that script deletes its ROOT);
         - delete the scratch branch and worktree.
      4. If `work/integration-ui-reliability` exists, also run its `merge-tree` and report the result.

      Files: `docs/work/active/cli-agent-contract/evidence.md`
      Verify:
      - every command exits 0, or is recorded as an expected environment failure that also fails on `8bef439b` (`pdftoppm`) or as sandbox-blocked (PTY);
      - `git worktree list` and `git branch --list 'scratch/*'` show no scratch left;
      - `git status --porcelain` shows only task files.

- [ ] R2-16. **Close-out.** Tick R2-1…R2-15 here. In `evidence.md`, write the round-2 report:
      - before/after command examples;
      - the machine contract summary (pointer to the README and `brn --json --help`, plus the contract-2 change table);
      - verification results;
      - the remaining gaps (appendix §Gaps, plus anything the spike or audit adds);
      - the owner questions (appendix), as questions only;
      - LIST B (B1, B3 part 2) as options and pointers only.

      Report these states separately: implemented, verified, native-observed (n/a; CLI checks do not establish native UI usability), owner-accepted (not requested), merged (no). Commit locally; do not push.
      Files: `docs/work/active/cli-agent-contract/{plan,evidence}.md`
      Verify: `python3 scripts/check-markdown-links.py` passes; `git status --porcelain` is clean after the commit; `git log --oneline 8bef439b..HEAD` lists only task commits.

## Review scope for the round-2 loop (A8; required)

The round-2 reviewers review all of `8bef439b..HEAD`, with emphasis on round 2. They must **explicitly cover `3638a54a`, `00eaa639` and `2537db54`** (left unreviewed after round 1), stating for each what was read and what was found. Each review also:
- re-checks the hard-constraint checklist and the coordination rule;
- confirms that every ⚑ change is additive, except A11's approved hook;
- confirms that `HELP` and the useful-intake sub-block are untouched;
- confirms that LIST B stayed options-only;
- checks the baseline-failure matrix.

The final report keeps implemented, verified, native-observed (n/a), owner-accepted and merged separate.

## Verification commands (complete list)

Run from `/Users/evokessler/repos/brn-rust/.worktrees/cli-agent-contract` with `BRN_*` unset and `--locked --offline`:

```sh
cargo fmt --all -- --check
cargo build --workspace --locked --offline
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
git diff --exit-code Cargo.lock
cargo build -p brn-intake --features helper --bin brn-intake-helper --locked --offline
cargo test -p brn -p brn-threads-core -p brn-threads-app -p brn-desktop -p brn-ai --locked --offline
cargo test --workspace --locked --offline --no-fail-fast   # expected env failures (pdftoppm) must also fail on 8bef439b
env -u BRN_SKIP_PTY_TESTS cargo test -p brn --locked --offline --test agent_contract   # PTY test; record as blocked if the sandbox forbids PTYs
cargo build -p brn --features native-retrieval --locked --offline
cargo check -p brn -p brn-threads-app --features brn-threads-app/native-retrieval --locked --offline
bash scripts/verify-storage.sh
p="$(mktemp -d "${TMPDIR:-/tmp}/brn-ci-fixtures.XXXXXX")"; p="$(cd "$p" && pwd -P)"; TMPDIR="$p" bash scripts/verify-end-to-end.sh --fixtures-only && rmdir "$p"
python3 scripts/cli_journey.py "$PWD/target/debug/brn" --evidence-dir /tmp/brn-cli-agent-contract/<run>/journey   # fresh empty TMPDIR parent; stderr empty; parent empty after
python3 docs/work/active/projects/cli-witness.py target/debug/brn
python3 docs/work/active/chats-and-projects/cli-witness.py target/debug/brn   # fresh TMPDIR; parent empty after
python3 -m unittest discover -s scripts/tests
python3 scripts/check-markdown-links.py
for s in scripts/*.sh; do bash -n "$s"; done
git diff --check 8bef439b..HEAD
git diff --stat 8bef439b..HEAD -- crates/brn-desktop crates/brn-ai   # must be empty
target/debug/brn --help; target/debug/brn; target/debug/brn --unknown   # exits 0, 1, 1
target/debug/brn-desktop --headless-check startup   # on a fresh data dir; no window
AGENT_CONTRACT_BASELINE_BIN=/tmp/brn-cli-agent-contract/r2-baseline/target/debug/brn cargo test -p brn --locked --offline --no-fail-fast --test agent_discovery --test agent_inspect --test agent_bounds --test agent_retry --test agent_credentials --test agent_runs --test agent_edits --test agent_converge --test agent_reasons   # every non-guard test fails
git -C /Users/evokessler/repos/brn-rust -c core.fsmonitor=false merge-tree --write-tree --name-only work/cli-agent-contract work/daily-workflow-reliability
```

If git prints "daemon terminated", retry with `git -c core.fsmonitor=false`.

## Decisions taken in this plan (round 2)

- **No FEAT decomposition. The existing implement-and-review loop runs this plan.** The steps are strictly ordered and share `main.rs`, `spec.rs`, `contract.rs` and the README tables that one test compares with `description()`. The existing loop tail (two parallel reviewers, an aggregator writing `review-2.json`, pause on max iterations, then the fail-closed verify-and-report step) already carries the stop contract and the final checks. Rebuilding it would risk losing that contract. Per-step commits and ticks let a later iteration, or a session resumed after compaction, start at the first unticked step.
- **`Grammar`/`VALUES` start in R2-3, not A5.** A1's new options need pre-open validation. R2-11 extends the table and R2-12 renders it, so there is one table and no churn.
- **The SIGINT handler and `on_sigint` stay untouched.** `stop_reason()` infers `sigint` from `CANCEL` with STOP unset, which keeps the daily merge conflict small.
- **`candidates` page `total` is null.** Design A1 names no candidate-count function, and adding one is not needed for paging.
- **Default-mode hard-stop text** is "Stopped at the --timeout deadline (N s); state unknown". The design's "adds '; state unknown'" would otherwise follow "nothing was committed", which a hard stop can't claim.
- **Baseline comparison uses `AGENT_CONTRACT_BASELINE_BIN`.** Only the test process reads it; children and `brn` never see it, so it is not a production switch (constraint 4). Guard tests are listed per step, because compatibility tests are meant to pass on both binaries.
- **`operation --key` echoes the lookup key** in the envelope `key` field with retry `read`. That is the existing envelope rule (`key` = the caller's valid `--key`); the README states it.
- **A10's `configure_then_ask_reaches_the_provider` moves to R2-8**, because it needs the A2 stub. C4's fallback applies if the spike fails.

## Expected remaining gaps after round 2

Gaps after round 2: appendix §Gaps. Plus: desktop no-ops still write receipts (B4 desktop gap); B1 and B3 part 2 (options-only, round 3); `--trash` unvalidated; the scratch-merge items from design §1.1(a)–(e) land only at merge.

<!-- ===================== ROUND 2 END ===================== -->
