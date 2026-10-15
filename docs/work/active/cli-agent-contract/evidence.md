# CLI agent contract — evidence

**Date:** 15 October 2026. **Branch:** `work/cli-agent-contract` in the isolated worktree `.worktrees/cli-agent-contract`, fast-forwarded from `771116f0` to `origin/main` `8bef439b` before any change. **Baseline tree:** `c93ce64a`. Toolchain 1.98.1 from `rust-toolchain.toml`; every build `--locked --offline`; `Cargo.lock` unchanged; no new crates. All data was synthetic, in fresh temporary directories, with `BRN_*` cleared; no provider or model call was made; no owner data was read.

Status, reported separately: **implemented** yes; **verified** yes (below, with two environment limits); **native-observed** not applicable (CLI only; CLI checks do not establish native UI usability); **owner-accepted** not requested; **merged** no (local commits only, nothing pushed).

## Baseline (before any change)

| Command | Result |
|---|---|
| `cargo build --workspace --locked --offline` | ok |
| `cargo test -p brn --locked --offline` | 11 passed (`tests/threads.rs`) |
| `python3 -m unittest discover -s scripts/tests` | 141 passed |

`work/daily-workflow-reliability` was `0efb6bef` at test-merge time, not the plan's `b3fc584a`; `git diff --stat b3fc584a 0efb6bef` touches only its `evidence.md`, so the design's conflict analysis still held.

## Implemented

- **I1, machine contract.** `crates/brn/src/spec.rs` (command table: options, required order, `required_when`, owner marks, retry classes and variants, outcomes, receipt rule, `description()`), `crates/brn/src/contract.rs` (statuses, 26 error codes, typed `CliError` with exhaustive core mapping, classification, conflicts, envelope, panic-free writers), `crates/brn/src/main.rs` (table-driven `parse()` over `OsString`, pre-open checks, envelope output, `AGENT_HELP` appended at print time).
- **I2, file and stdin input.** `crates/brn/src/input.rs`: `--NAME-file ABSOLUTE_PATH|-` for title, text, prompt, instructions and actions; metadata-first directory check, terminal refusal, 16 MiB bound, UTF-8 and NUL checks, exact bytes, detached reader with a 25 ms SIGINT poll.
- **I3, retry identity.** `--key` on note-create, thread-start, undo, queue, archive/resolve, unarchive, keep-in-brn and full/chat/project intake (`cli-key:{command}:{KEY}`; unkeyed keys unchanged; project keys raw). Shared service: `new_note_with_key`, `new_thread_with_key`, `undo_with_key` in `crates/brn-threads-app/src/lib.rs` with the old functions as wrappers, plus the `RetrievalError` re-export. No file under `brn-threads-core` or `brn-desktop` changed (`git diff --stat origin/main -- crates/brn-threads-core crates/brn-desktop` is empty).
- `ask` error mapping by `AiErrorKind`; C1 and C2 wording applied to the `provider_failed`/`invalid_request` meanings and the README.
- Docs: `crates/brn/README.md` "Agents and scripts"; one sentence in `docs/development/verification.md`; a `docs/status.md` entry.
- Tests: `crates/brn-threads-app/tests/keyed_owner_operations.rs` (3), 22 unit tests in `spec.rs`/`contract.rs`/`input.rs`, `crates/brn/tests/agent_contract.rs` (22 process tests), `scripts/cli_journey.py` with `scripts/tests/test_cli_journey.py` (6), wired into `scripts/verify-end-to-end.sh`.

Small choices made within the design: the description's `statuses` entries also carry the 4.2 `action` text (additive); class and status meanings drop the design's section cross-references; `read` of an unknown ID in machine mode reports `not_found` with the message `Record not found` (no ID echoed); `effort()`/`provider()` refusals name `effort`/`provider` also when reached through the helper options.

## Verification (final candidate)

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo build --workspace --locked --offline` | ok |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` (as `verify-storage.sh` runs it) | clean |
| `cargo test -p brn --locked --offline` | 22 unit + 22 `agent_contract` + 11 `threads.rs` passed (`BRN_SKIP_PTY_TESTS=1`, see limits) |
| `cargo test -p brn-threads-app --locked --offline` | all passed, including `keyed_owner_operations` (3) |
| `cargo build -p brn --features native-retrieval --locked --offline` | ok (native crates were cached) |
| `cargo check -p brn -p brn-threads-app --features brn-threads-app/native-retrieval --locked --offline` | ok (wildcard `RetrievalError` arm compiles under feature unification) |
| `git diff --exit-code Cargo.lock` | clean |
| `python3 -m unittest discover -s scripts/tests` | 147 passed (141 + 6 new) |
| `python3 scripts/check-markdown-links.py` | 0 failures |
| `for s in scripts/*.sh; do bash -n "$s"; done` | ok |
| `bash scripts/verify-storage.sh` | exit 101: only 12 `brn-threads-intake` tests failed (`--lib` 3, `--test qualification` 9); see limits. `cargo test --workspace --no-fail-fast`: 734 passed, 12 failed, all in that crate. The script's CLI/desktop tail run by hand: `brn --help` 0, `brn` 1, `brn --unknown` 1, desktop `--help` 0, `--unknown` 1, `--headless-check startup` 0 |
| CI macOS fixture step: `p="$(mktemp -d "${TMPDIR:-/tmp}/brn-ci-fixtures.XXXXXX")"; p="$(cd "$p" && pwd -P)"; TMPDIR="$p" bash scripts/verify-end-to-end.sh --fixtures-only && rmdir "$p"` | passed in 6 s, journey included; `rmdir` succeeded (parent empty) |
| `python3 docs/work/active/projects/cli-witness.py target/debug/brn` | passed, 19 checks, 41 calls |
| `python3 scripts/cli_journey.py target/debug/brn --evidence-dir …` with `TMPDIR=/private/tmp/brn-cli-agent-contract/run-1792087185/parent` and `BRN_*` unset | `{"journey":"passed","steps":61,"checks":590}` in about 3 s (A8: well inside CI's 30-minute lane); parent left empty; transcript 62 lines, no note content (paths, IDs and SHA-256 only) |

Environment limits, both from the agent's macOS sandbox, not the change:

- `terminal_sources_are_refused_without_blocking` cannot allocate a pseudo-terminal here: `posix_openpt` returns EPERM, and so does Python's `os.openpty()`. The test fails loudly as designed; the suite was therefore run with `BRN_SKIP_PTY_TESTS=1`, which only this test honours. CI's Linux and macOS runners must run it without the variable.
- The 12 `brn-threads-intake` failures are its sandboxed-helper tests (`Unavailable` from the helper sandbox); `sandbox-exec -p '(version 1)(allow default)' /usr/bin/true` returns `sandbox_apply: Operation not permitted` here. `git diff --stat origin/main -- crates/brn-threads-intake crates/brn-intake` is empty.

Not reachable offline at the process boundary, covered by unit tests only: `NeedsReview`, `Superseded`, an `ask` that ends Interrupted or Failed, `RetrievalError::Sql` and core `Error::Sqlite` construction (the CLI has no `rusqlite` dependency; a real `Sqlite` error is produced through a second `begin_edit`).

## Test-merge with `work/daily-workflow-reliability`

Scratch worktree `.worktrees/cli-agent-contract-merge-check` on `scratch/cli-agent-contract-merge-check` from `7ad43581`; `git merge --no-commit --no-ff work/daily-workflow-reliability` (`0efb6bef`):

- Conflicts, both in `crates/brn/src/main.rs`: (1) the `HELP` constant region, because that branch edits `HELP` and this branch adds `AGENT_HELP` and the `Result` alias right after it; resolved by taking that branch's `HELP` and keeping `AGENT_HELP` and `Result<T, CliError>`. (2) The expected begin-edit `.map_err` line; resolved as design §10 says: that branch's `.map_err(begin_edit_error)?`, `begin_edit_error` returning `CliError` (`edit_session_open` with `{note, session, base_version}`, this contract's message, that branch's JSON line as `default_text`), `Error::EditSessionOpen` mapped in the exhaustive `From<Error>` match (the merge failed to compile until it was), and the `recovery_buffers` enrichment deleted. The intake `useful_project` region merged cleanly.
- One test assertion needs the merge-time edit: `open_edit_session_reports_edit_session_open` checks today's default stderr (`SQLite: UNIQUE…`); after the merge, default stderr is that branch's JSON line by design, so the scratch resolution asserts `error.kind == "edit_session_open"` and the session instead.
- In the scratch tree: `cargo test -p brn --locked --offline` passed (22 unit, 22 `agent_contract`, 13 `threads.rs` including `a_second_begin_edit_reports_the_open_session_as_structured_json`); `cargo clippy -p brn --all-targets -D warnings` clean.
- `git merge --abort`, then `git merge --no-commit --no-ff origin/codex/linux-ui-cloud` (`7075c46a`): clean, `docs/development/verification.md` auto-merged with this branch's sentence kept; aborted.
- Scratch worktree removed and scratch branch deleted; `work/daily-workflow-reliability` still `0efb6bef`, `main` still `771116f0`; nothing committed or pushed from the scratch tree. The resolution diffs are kept outside the repository in the task artifacts folder (`merge-resolution-main.diff`, `merge-resolution-ours.diff`).

## Before and after (re-run against the built binary, synthetic data)

```text
# Stale save. Before (default mode, unchanged): {"outcome":"Stale","session":{…}}  exit=0
$ brn --json --data-dir "$D" save --session "$S2" --generation 1          # exit=3
{"contract":1,"command":"save","ok":false,"status":"stale","exit":3,"outcome":"Stale","operation":null,"key":null,"retry":"replay","changed":[],"conflict":[{"id":"0a4a…:0","kind":"Note","version":2,"archived":false,"open_edit":true}],"result":{…},"error":null}

# Open edit session. Before: "SQLite: UNIQUE constraint failed: edit_sessions.note" exit=1 (default mode unchanged)
$ brn --json --data-dir "$D" begin-edit --note "$N" --base 2              # exit=6
{"…","status":"rejected","exit":6,"retry":"refuse","error":{"code":"edit_session_open","message":"This note has an open unsaved edit session. Write to it only if your own successful begin-edit returned it.","details":{"note":"0a4a…:0","session":"81bc…","base_version":1}}}

# Lost response. Before: a retry made a second note. After, two identical calls:
$ brn --json --data-dir "$D" note-create --key plan-1 --title Plan --text-file /abs/plan.md
ok 03c2… replay 'line 1\n$HOME `x`\n'      # first call
ok 03c2… replay 'line 1\n$HOME `x`\n'      # retry: same operation, text kept byte for byte

# Deferred undo. Before: exit 0, no operation, every retry left another candidate.
$ brn --json --data-dir "$D" undo --operation "$OP" --key undo-1          # exit=4, "operation":"4c2f…","retry":"replay","conflict":[{"kind":"Note",…,"open_edit":true}]
$ brn --json --data-dir "$D" undo --operation "$OP" --key undo-1          # after discarding the own session: exit 0, Applied, "operation":"4c2f…"

# Second export. Before: "Export destination already exists; choose a new snapshot" exit=1 (unchanged by default)
$ brn --json --data-dir "$D" export --destination "$R/exp"                # exit=6, "code":"destination_exists","details":{"option":"destination"},"retry":"refuse"

# Mis-encoded argument. Before: panic, exit 101.
$ brn --json --data-dir "$D" note-create --title T --text $'\xff'         # exit=2, "command":null,"code":"malformed_option","details":{"position":8}

# Chat attachment without its chat. Before: the source was read and converted first.
$ brn --json --data-dir "$R/none" intake --source "$R/plan.md" --intent chat   # exit=2, missing_option {"option":"thread"}; "$R/none" not created

# Owner marks: brn --json --help gives archive, resolve, unarchive owner_decision true; queue false.
```

## Machine contract

The contract is documented in [the CLI README, "Agents and scripts"](../../../../crates/brn/README.md#agents-and-scripts) and served by `brn --json --help` from the same table the parser uses; `machine_description_lists_every_parsed_command_and_matches_readme` compares the README's status, error-code, retry and owner tables and the owner-rule line with the description. Summary: one compact envelope line on stdout (`contract, command, ok, status, exit, outcome, operation, key, retry, changed, conflict, result, error`); exits 0 ok, 1 internal, 2 usage, 3 stale, 4 deferred, 5 needs_review, 6 rejected, 7 failed, 130 interrupted; 26 stable error codes; retry classes read, replay, converge, refuse, duplicate per command and variant; owner-decision marks are documentation only. Without `--json`, output stays as before apart from the README's five compatibility items.

## Review

Two independent reviews of the task range `8bef439b..7983bc4f` (the six task commits): reviewer A **APPROVED**, reviewer B **APPROVED**, aggregate **APPROVED** with seven non-blocking findings. Handled here: the untracked duplicate `evidence.md` symlink at the worktree root is deleted (it also made the link check fail, because its relative links resolve from the root), and the plan no longer names machine-local paths or branch-deletion commands. Left open and listed under the gaps: the CI-only PTY and intake sandbox tests, the merge-time test edit for `work/daily-workflow-reliability`, no process test for reusing an Undo or keep-in-brn key on a different original (the core refuses it), and the unit-only outcome paths.

## Final verification

Code tree verified: `3638a54a` (base `8bef439b` = `origin/main` as last fetched; local `main` `771116f0`, an ancestor). The only change after the reviewed range is `3638a54a`, a test-only fix (below); the commit that adds this section changes records only. `git fetch origin` failed (`Could not resolve host: github.com`; no network in the agent sandbox), so no newer remote `main` could be checked; neither known ref is ahead of the base, so no rebase was needed. `BRN_*` unset unless shown; synthetic data under `/private/tmp/brn-cli-agent-contract`; no provider call.

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo build --workspace --locked --offline` | ok |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | clean |
| `cargo build -p brn-intake --features helper --bin brn-intake-helper --locked --offline` | ok |
| `git diff --exit-code Cargo.lock` | clean |
| `cargo test -p brn --locked --offline` (no skip variable) | 22 unit and 11 `threads.rs` passed; `agent_contract` 21 passed, 1 failed: `terminal_sources_are_refused_without_blocking`, `posix_openpt failed: Operation not permitted` (sandbox) |
| `BRN_SKIP_PTY_TESTS=1 cargo test -p brn --locked --offline --no-fail-fast` | 22 + 22 + 11 passed; the PTY test self-skips, so it is **not** proof |
| `cargo test -p brn-threads-app --locked --offline --no-fail-fast` | 145 passed, 0 failed (`keyed_owner_operations` 3) |
| `BRN_SKIP_PTY_TESTS=1 BRN_INTAKE_HELPER=… cargo test --workspace --locked --offline --no-fail-fast` | 735 passed, 11 failed, 4 ignored; all 11 in `brn-threads-intake` (lib 3, `qualification` 8) |
| Same 11 on the untouched baseline: `git archive 8bef439b`, own target dir, `cargo test -p brn-threads-intake --locked --offline --no-fail-fast` | the same 11 fail (identical list): the helper sandbox cannot start here (`sandbox-exec … /usr/bin/true` → `sandbox_apply: Operation not permitted`) |
| `bash scripts/verify-storage.sh` | **exit 101**, stopping at the first failing intake target (3 failed of 25 in its lib tests); its tail by hand: `brn --help` 0, `brn` 1, `brn --unknown` 1, desktop `--help` 0, `--unknown` 1, `--headless-check startup` 0 |
| CI macOS fixture step as in `ci.yml` (`RUNNER_TEMP` under the task root; `mktemp -d`, `pwd -P`, `trap 'rmdir …' EXIT`, `TMPDIR=…`, `bash scripts/verify-end-to-end.sh --fixtures-only`) | exit 0 in 3 s, journey included (`{"journey":"passed","steps":61,"checks":590}`); `rmdir` succeeded |
| `python3 scripts/cli_journey.py "$PWD/target/debug/brn" --evidence-dir …`, `TMPDIR` a fresh empty parent, `BRN_*` unset | exit 0 in 0.98 s, 61 steps, 590 checks, stderr empty, parent left empty; transcript 62 lines (paths, IDs and SHA-256 only) kept with the task artifacts |
| `python3 docs/work/active/projects/cli-witness.py target/debug/brn` | passed, 19 checks, 41 calls |
| `python3 -m unittest discover -s scripts/tests` | first run: 146 passed, 1 error (`test_a_hung_child_is_killed_with_its_process_group`); after `3638a54a`: 147 passed |
| `python3 scripts/check-markdown-links.py` | 0 failures (after deleting the root symlink) |
| `for s in scripts/*.sh; do bash -n "$s"; done`; `git diff --check 8bef439b..HEAD` | ok; clean |
| `cargo build -p brn --features native-retrieval --locked --offline`; `cargo check -p brn -p brn-threads-app --features brn-threads-app/native-retrieval --locked --offline` | ok; ok |

The test fix: a freshly written Python stub sometimes took more than 1 s to start here, so the 1 s timeout killed it before it spawned the grandchild and the test failed reading the pid file (3 of 6 isolated runs; 8 of 15 in a probe loop). `3638a54a` uses a 2 s timeout and repeats an attempt whose stub never spawned the grandchild; the timeout and group-kill assertions are unchanged. It then passed 8 of 8 runs. The journey runner itself is unchanged.

Coordination, re-checked: the only shared-service file changed is `crates/brn-threads-app/src/lib.rs` (33 added, 10 removed lines: three keyed functions and the `RetrievalError` re-export; `new_note`, `new_thread`, `undo` became wrappers); nothing under `brn-threads-core`, `brn-desktop`, `brn-threads-intake` or `brn-intake`. `git log 771116f0..<branch> -- crates/brn-threads-app/src/lib.rs` is empty for every branch named in design §10 except `codex/rust-luna-intake`, whose one hit (`859b7f46`) is pre-#144 history already squash-merged as `34c9239d`. `work/daily-workflow-reliability` is still `0efb6bef`, so the scratch test-merge was not repeated; `git merge-tree --write-tree` (no worktree) still reports the one known conflict file, `crates/brn/src/main.rs`, at both `7ad43581` and the candidate. `origin/codex/linux-ui-cloud` (`7075c46a`) merges without conflicts.

Safety: the main checkout has no tracked changes (only the owner's untracked `.vscode/` and `experiments/`); `main` is `771116f0`; `work/cli-agent-contract` has no upstream and no remote ref; no tag contains a task commit; no PR, merge, publish or release; no gate or control file is in any task commit.

## Remaining gaps (ranked)

1. No bounded, paginated inspection: `history`, revisions, receipts and candidates stay unpaged; `evaluate-wiki-guidance.py` still reads SQLite read-only.
2. NeedsReview, Superseded, run Interrupted/Failed and model-candidate review are unit-mapped only; the chats-and-projects witness still fails and writes SQLite; the fresh-workspace `auth_file` default is unchanged.
3. No stale-session rebase; sessions are not bound to a caller, so a lost `begin-edit` response means wait or ask the owner.
4. `topics-create`, `knowledge-check --seed-chat`, `ask` and useful intake have no keys; converge commands still append receipts; unkeyed `undo`, `keep-in-brn` and full/chat/project intake can leave unapplied candidates (reported as `duplicate`); keyed non-Markdown intake re-sends the document to the provider before replay.
5. SIGINT is observed only at the listed points; no SIGTERM envelope, no `--timeout`, no SQLite busy code; a killed `ask` can leave a Working run.
6. No per-command help; `--data-dir` must come first; `backup`/`restore` accept relative paths; `intake --intent chat --thread <PROJECT_ID>` attaches without the owner mark.
7. Read-only commands on a mistyped directory still create a workspace; enum values other than `--trace` are validated after opening.
8. `AppError::Invalid(String)` stays one code; the useful-intake sub-block keeps a coarse `invalid_request` mapping and `conflict: []` until the daily-workflow merge.
9. Owner authority is stated, not enforced, at the CLI.
10. The pseudo-terminal test and the intake sandbox tests were not run in this environment (above); CI must run them.
11. No process test reuses an Undo or keep-in-brn key for a different original (the core refuses it, `store.rs` 511–521).
12. Merging `work/daily-workflow-reliability` needs the recorded resolution, including changing `open_edit_session_reports_edit_session_open` to assert the structured `edit_session_open` default output.
