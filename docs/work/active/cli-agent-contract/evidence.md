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

### Orchestrator-run checks outside the sandbox (candidate `00eaa639`)

The orchestrator ran these from an unsandboxed shell in the task worktree. They replace the two sandbox-blocked results above and the failed fetch. I confirmed the counts against its two logs before deleting them, and I confirmed that line 70 of `ci.yml` installs Poppler, that `BRN_SKIP_PTY_TESTS` appears nowhere under `.github/` or `scripts/`, and that `pdftoppm` is not on `PATH` here.

| Command | Result |
|---|---|
| `git fetch origin` | exit 0; `origin/main` still `8bef439b` and contained in `HEAD`; local `main` still `771116f0`; no rebase needed |
| `env -u BRN_SKIP_PTY_TESTS cargo test -p brn --locked --test agent_contract` | 22 passed, 0 failed; `terminal_sources_are_refused_without_blocking` ran for real and passed |
| `bash scripts/verify-storage.sh` | **exit 101**. `fmt --check`, the helper build, the workspace build and `clippy -D warnings` passed first. Inside it, `cargo test --workspace --locked --offline` gave 745 passed, 1 failed, 4 ignored |
| `cargo test --workspace --locked --offline --no-fail-fast` | 745 passed, 1 failed, 4 ignored. The 11 sandbox-exec intake failures do not occur |
| The script's tail, run by hand | `brn --help` 0; `brn` 1; `brn --unknown` 1; desktop `--help` 0; desktop `--unknown` 1; desktop `--headless-check startup` on a fresh data dir prints `Threads ready: 2 records`, exit 0 |

The one failure is `brn-threads-intake --test qualification` `prepared_rust_fixtures_keep_source_and_preview_bindings_in_owned_scratch`, which panics with `Pdf preparation failed: Unavailable` because this machine has no Poppler `pdftoppm`. **Status: local-env-blocked, covered by the CI macOS job, not passed.** CI installs Poppler with `brew install poppler`. `brn-threads-intake` and `brn-intake`, its only dependency, have no diff in `8bef439b..HEAD`.

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
10. The orchestrator ran the pseudo-terminal test outside the sandbox, and it passed. `prepared_rust_fixtures_keep_source_and_preview_bindings_in_owned_scratch` still needs Poppler `pdftoppm`, so the CI macOS job must cover it.
11. No process test reuses an Undo or keep-in-brn key for a different original (the core refuses it, `store.rs` 511–521).
12. Merging `work/daily-workflow-reliability` needs the recorded resolution, including changing `open_edit_session_reports_edit_session_open` to assert the structured `edit_session_open` default output.

<!-- ===================== ROUND 2 BEGIN ===================== -->

# Round 2 (planned; nothing implemented yet)

The owner asked to "fix the remaining gaps". The approved round-2 design (`design-2.md` rev 4 and its appendix, review 2 iteration 4: APPROVED, seven NITs carried forward) is a task artifact outside the repository. The ordered steps are in the [plan, Round 2](plan.md#round-2-closing-the-remaining-agent-contract-gaps). The owner's decisions are in [owner-decisions.md](owner-decisions.md).

## Scope outline

| Rank | Item | What changes for an agent | Shared service |
|---|---|---|---|
| 1 | A1 bounded inspection | Paged `history [--record]`, `candidates` and `records [--kind]`; new `revisions`, `revision`, `operation --id \| --key --command`, `runs [--thread] [--state]`; paged `read --limit [--cursor]`; envelope `page`. The wiki evaluator stops opening SQLite | ⚑ core `inspect.rs`, SELECT-only |
| 2 | A4 bounded execution | SIGTERM → `terminated` (143); `--timeout` soft stop (`timeout`, 7) and one-line hard stop; `busy` (7); envelope `stop` | none |
| 3 | A3 retry keys | `--key` on `topics-create` and `knowledge-check --seed-chat yes`; keyed full/chat/project intake replays before conversion | ⚑ app `keyed.rs` (additive) |
| 4 | A10 (owner B2) | CLI-created workspaces have no credential source; `credentials_not_configured` (6) before any commit | ⚑ app `open_without_default_credentials` beside an unchanged `open` |
| 5 | A2 offline outcomes | Test-only loopback stub for `ask` outcomes; witness without SQLite; NeedsReview/Superseded reachability audit | none |
| 6 | A12 (owner B3.1) | `edit_session_open` details `{note, base_version}`; documented stale-edit recipe | none |
| 7 | A11 (owner B4) | No-op archive/resolve/unarchive/chat-move/project-link/unlink → `Unchanged`, no receipt | ⚑ core `*_converging` + private hook (owner-approved exception); app `converge.rs` |
| 8 | A6 | Machine-mode reads on a missing or empty dir → `missing_workspace` (6); enum and integer values checked before opening | none |
| 9 | A5 | `COMMAND --help` (text and JSON); `--data-dir`/`--timeout` after the command; absolute paths required; owner mark on `intake --thread <project>` | none |
| 10 | A7 | `invalid_request` `details.reason` | none |
| — | A8/A9 | Review covers `8bef439b..HEAD`, explicitly including `3638a54a`, `00eaa639` and `2537db54`; PTY tests run unsandboxed or are recorded as blocked | — |

Everything ships as contract 2, with a "Changes from contract 1" table and a forward rule. Default mode changes only per design §2.3 items 6–12. `HELP` and the useful-intake sub-block stay untouched. LIST B items B1 and B3 part 2 stay options-only (round 3).

## Planning-time checks (16 Oct 2026)

- Worktree at `2537db54`; the only untracked file is `owner-decisions.md`.
- Overlap check (`git log --oneline 8bef439b..<branch> -- <path>` for every path round 2 touches):
  - Nine branches are already ancestors of the base: `codex/projects-feature`, the three `work/knowledge-*`, `codex/writing-polish`, `codex/tile-ui-polish`, `codex/writing-ux-research`, `work/subagent-design` and `work/quality-simplify`.
  - `origin/codex/linux-ui-cloud` (`7075c46a`) touches only scripts tests and docs that round 2 doesn't edit, apart from one `docs/status.md` line; `merge-tree` is clean.
  - `work/daily-workflow-reliability` (`b76cc5a7` "docs: record the second rebase onto 8bef439b"; 29 commits) touches:
    - core `lib.rs`, `chat.rs`, `projects.rs` and `store.rs`;
    - app `lib.rs`, `settings.rs` (from :98), `intake.rs`, `chat.rs` and `projects.rs`;
    - `crates/brn/src/main.rs` and `crates/brn/tests`.

    Its canonical-locator commit is `344331f2` "count one file as one source regardless of path spelling".
  - `work/integration-ui-reliability` is absent.
- `merge-tree --write-tree --name-only` against daily: conflict only in `crates/brn/src/main.rs`.

## Evidence to be filled in by the round-2 steps

- Baseline binary built from `2537db54` (path, contract 1).
- Per-step verification rows.
- The baseline-failure matrix: each new process test either fails at `2537db54` (with the failing assertion) or is a listed guard.
- The A2 spike result and the NeedsReview/Superseded audit.
- Caller surveys for §2.3 items 6 and 11 and for A6.
- The final verification table.
- The re-run overlap table and the final daily tip.
- `merge-tree` and scratch-merge results.
- Round-2 before/after examples, machine-contract changes and remaining gaps.

## Round 2 step records

### R2-1 contract 2 foundation

- Pre-flight: worktree at `2537db54`; `git status --porcelain` showed only `owner-decisions.md` (untracked), `plan.md` and `evidence.md`. Daily tip at implementation time: `b5212618` "fix: the owner's Stop of a lease-free Run adds no notice (round 6, decision 2 applied to Stop)". `work/integration-ui-reliability` absent. BOARD.md read (no new ASK for this branch beyond the ones already answered).
- Baseline binary: `git archive 2537db54` into `/tmp/brn-cli-agent-contract/r2-baseline/src`, `cargo build -p brn --locked --offline --target-dir /tmp/brn-cli-agent-contract/r2-baseline/target`; `/tmp/brn-cli-agent-contract/r2-baseline/target/debug/brn --json --help` reports `"contract":1`.
- **Sandbox limit found (environment, not the change):** in this agent session a SIGINT sent to any process spawned by a non-shell parent (cargo's test harness, Python) never reaches its handler, while SIGTERM does (a Python child with its own SIGINT handler sleeps through `os.kill(pid, SIGINT)`; the same `brn` spawned by zsh gets it). The three round-1 SIGINT tests (`refuse_retry_after_an_interrupted_first_attempt_applies_once`, `sigint_while_stdin_stays_open_is_interrupted_and_creates_nothing`, `sigint_while_a_fifo_has_no_writer_is_interrupted`) therefore time out here **on the unchanged `2537db54` tree too** (checked with the change stashed). They are recorded as sandbox-blocked, not passed; CI runs them.
- Verification: `cargo test -p brn --locked --offline --test agent_discovery --test threads` 1 + 11 passed; unit tests 22 passed; `agent_contract` 19 passed, 3 sandbox-blocked (above), PTY test skipped with `BRN_SKIP_PTY_TESTS=1`; `python3 -m unittest discover -s scripts/tests` 147 OK; `cli_journey.py target/debug/brn` with a fresh `TMPDIR` parent: passed (61 steps, 650 checks), parent empty; `cargo clippy -p brn --all-targets --locked --offline -- -D warnings` clean.
- Baseline run (`AGENT_CONTRACT_BASELINE_BIN=… --test agent_discovery`): `every_envelope_has_contract_2_page_and_stop` fails: `contract` is 1.

### R2-2 ⚑ core inspection pages (additive, SELECT-only)

- ⚑ Shared service: new `crates/brn-threads-core/src/inspect.rs` (`OperationState`, `OperationInfo`, `CandidateRow`, `RECORD_KINDS`; `Store::{receipt_page, receipt_count, candidate_page, record_page, run_page, operation_by_host_key, operation_info}`), plus two lines in core `lib.rs` (`mod inspect;` and its `pub use`, after `mod backup;`). No transaction, no DML, not used by the desktop; the kind filter is a bound JSON path. BOARD.md read first.
- Tests: `crates/brn-threads-core/tests/inspect.rs` (7, data written only through `Store`; n/a at baseline: new API).
- Verification: `cargo test -p brn-threads-core --locked --offline --no-fail-fast` 119 passed, 0 failed; `cargo build --workspace --locked --offline` ok; `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` clean; `git diff --stat 2537db54 -- crates/brn-threads-core/src/store.rs crates/brn-desktop crates/brn-ai` empty.

<!-- ===================== ROUND 2 END ===================== -->
