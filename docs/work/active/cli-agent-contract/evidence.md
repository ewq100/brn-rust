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

### R2-3 A1 CLI page mode and new read commands

- CLI: `crates/brn/src/inspect.rs` (new, summaries and cursor syntax over the R2-2 pages and the existing `note_revisions`/`read_note_revision`/`read_note_range`); `spec.rs` gains `Grammar`/`CursorForm`/`VALUES`, per-command `page`, `bounded`, `bounded_form`, `values`, `needs` and the commands `revisions`, `revision`, `runs`; `read` gains `--limit`/`--cursor` (outcomes `Read`, `Stale`); envelope `page` on every page and on `search`; `AGENT_HELP` names the page commands; README "Inspection", retry rows and a "Changes from contract 1" row. Flag-less `history`/`candidates`/`records`/`read` unchanged in both modes. Shared service: none beyond R2-2.
- Tests `crates/brn/tests/agent_inspect.rs` (8): success `history_pages_are_bounded_stable_and_filter_by_record` (25 notes, pages of 10, a write between pages, no repeats, `--record`, the 32-write cap through a 33-chat project delete, default mode one line, flag-less output equal to default mode), `revisions_and_revision_read_page_through_a_large_note` (200 KiB Unicode, 4 versions reassembled byte-exactly; byte limit 5 inside characters), `records_page_by_kind_and_cursor`, `runs_page_validates_options_and_lists_nothing_on_a_fresh_workspace`; invalid input `page_options_are_refused_before_opening` (both modes, dir stays absent); conflict/refusal `read_pages_detect_a_change_between_pages` (exit 3, `conflict[0]` at version 2, Thread → `invalid_request`), `page_refusals_after_opening_name_the_cause`; retry `identical_page_reads_return_identical_pages`.
- Verification: `cargo test -p brn --locked --offline --no-fail-fast` (`BRN_SKIP_PTY_TESTS=1`): unit 22, `agent_inspect` 8, `agent_discovery` 1, `threads` 11 passed; `agent_contract` 19 passed + the 3 sandbox-blocked SIGINT tests (R2-1); strict workspace clippy clean; `cli_journey.py` passed with an empty temp parent.
- Baseline (`AGENT_CONTRACT_BASELINE_BIN`): all 8 fail — `unknown_option` (`cursor`, `limit`, `record`, `kind`) or `unknown_command` (`runs`, `revisions`), `page` absent (`history_pages…`: `left: Null`), and `retry` `refuse` instead of `read` (`identical_page_reads…`). No guards.

### R2-4 `operation`, `cli_key` and the evaluator migration

- CLI: `spec::cli_key` (the round-1 `host_key` `--key` branch moved verbatim) and `spec::stored_host_key` (lookup only; each prefix cites its app function); new read command `operation --id OP | --key K --command NAME` (forms checked before opening; a key lookup is `Store::operation_by_host_key`, a pure SELECT). README retry row, Inspection row and the one-key-per-project-change sentence; `AGENT_HELP` line.
- `scripts/evaluate-wiki-guidance.py`: `read_operation_inputs(cli, ids)` now reads `brn --json operation --id` through a new `Cli.envelope` (skips `not_found` and `allocated`, `applied = state == 'applied'`, `expectation` defaults to `Current`, output shape unchanged, truncated inputs raise `SystemExit`); `import sqlite3` and `closing` removed. A real-binary check returned `[{'operation': …, 'applied': True, 'inputs': []}]` for a fresh note and skipped an unknown id.
- Tests: unit `cli_keys_are_byte_identical_to_round_one`; process (`agent_inspect.rs`) `operation_by_key_finds_every_keyed_command` (note-create, thread-start, queue, chat intake, keep-in-brn, undo, archive/resolve, unarchive, project-create found through project-chat; each first call, keyed replay and lookup name one operation), `candidates_and_operation_report_unapplied_candidates` (deferred keyed undo `prepared` in `candidates --limit 1` and `operation`, then `applied` with the same id), `operation_lookup_refuses_bad_forms_and_never_allocates`; Python `test_applied_inputs_are_available_without_request_bodies` (stub CLI), `test_truncated_operation_inputs_fail_loudly`, `test_evaluator_does_not_import_sqlite3`.
- Verification: `cargo test -p brn --locked --offline --no-fail-fast` unit 23, `agent_inspect` 11, `agent_discovery` 1, `threads` 11 passed, `agent_contract` 19 + 3 sandbox-blocked; `python3 -m unittest discover -s scripts/tests` 149 OK; strict clippy clean.
- Baseline: the three new process tests fail with `unknown_command` (`operation`) and, for the candidates test, `page` absent before `operation` is reached.

### R2-5 A4 SIGTERM, `--timeout`, hard stop and `busy` (CLI only)

- CLI: new `crates/brn/src/stop.rs` (stop reasons with first-wins `STOP` beside the untouched `on_sigint`/`CANCEL`; machine-mode `sigaction(SIGTERM, SA_RESETHAND|SA_RESTART)`; a 25 ms watchdog that raises the deadline and, 3 s after SIGTERM or the deadline without a result, claims the single output, writes one line and `_exit`s; `restamp` maps an `interrupted` error to `terminated` or `timeout {deadline_seconds, hard:false}`). `main.rs`: global `--timeout SECONDS` after `--data-dir` (1..86400, `repeated_option`, before opening), new observation points after `Workspace::open` and after index readiness in `search`, `connections`, `ask`; every writer claims the output first. `contract.rs`: status `terminated` (143), codes `busy` (failed 7; `Error::Sqlite`/`RetrievalError::Sql` with extended code `& 0xff` in {5, 6}) and `terminated`; `timeout` meaning per C2; run terminals classify by stop reason (SIGINT 130, SIGTERM 143, deadline 7, none stale). README statuses/error rows, "Streams, SIGINT and time" rewritten, killed-`ask` recipe, change rows; `AGENT_HELP` names 143 and `--timeout`; journey knows `terminated`. Shared service: none.
- Tests `crates/brn/tests/agent_bounds.rs` (9): success `timeout_with_time_to_spare_changes_nothing`; invalid input `timeout_values_are_refused_before_opening` (both modes, dir absent); refusal `sigterm_during_stdin_wait_is_terminated_and_creates_nothing` (143, `stop:"sigterm"`), `timeout_during_fifo_wait_is_a_soft_deadline` (exit 7, `hard:false`, under 2 s; default text), `held_write_lock_reports_busy_and_creates_nothing` (a `python3` child holds `BEGIN IMMEDIATE` with no DML, then rolls back; `busy` after ≥ 4 s; default mode keeps `SQLite: …`, exit 1; history total unchanged), `held_lock_past_deadline_hard_stops_with_one_line` (`--timeout 1`: one line, `hard:true`, exit 7 between 3.5 and 4.9 s; history unchanged), `sigint_envelopes_report_stop_sigint`; retry `busy_then_keyed_retry_applies_once`; guard `default_mode_sigterm_keeps_default_disposition` (killed by signal 15, empty stdout).
- Verification: `agent_bounds` 8 passed, 1 sandbox-blocked (`sigint_envelopes_report_stop_sigint`: SIGINT is not delivered here, see R2-1; checked by hand from zsh instead: `sleep 5 | brn --json … note-create --text-file -` then `kill -INT` → exit 130, `"stop":"sigint"`, data dir absent); the rest of `cargo test -p brn` as before; strict clippy clean; scripts tests OK; journey passed with an empty parent; `brn --help` 0, `brn` 1, `brn --unknown` 1.
- Baseline: 8 fail on item assertions (`unknown_command` for `--timeout` in the command position; `storage_error`/internal instead of `busy`; SIGTERM kills with no envelope, exit −1; `history --limit` unknown); `default_mode_sigterm_keeps_default_disposition` passes by design (guard: default disposition kept).

### R2-6 ⚑ A3 retry keys and keyed intake replay before conversion

- Overlap re-check (app `keyed.rs`, `lib.rs`, `topics.rs`, `knowledge.rs`, `intake.rs`, `settings.rs`): 0 commits after `8bef439b` on every listed branch except `work/daily-workflow-reliability` (9, none in `knowledge.rs` or `topics.rs`), so the knowledge-check row was not deferred. BOARD.md unchanged since it was read (modified 08:48).
- ⚑ Shared service (app, additive): new `crates/brn-threads-app/src/keyed.rs` (`create_topics_note_with_key`, `seed_knowledge_review_with_key` with a private `keyed_owner_message` that sends exactly `post_message`'s request, `full_import_replay` with the private exact-match `same_source` whose comment records the merge-time `same_locator` change, citing daily "count one file as one source regardless of path spelling" `344331f2`); `mod keyed;` after `mod knowledge;`; `topics.rs` `STARTER` becomes `pub(crate)`; `knowledge.rs` extracts the pure `review_seed` and `seed_knowledge_review` calls it (behavior unchanged). `intake.rs`, desktop and `brn-ai` untouched (`git diff --stat 2537db54 -- crates/brn-threads-app/src/intake.rs crates/brn-desktop crates/brn-ai` empty).
- CLI: `topics-create --key` (converge), `knowledge-check --seed-chat yes --key` (variant `keyed: replay`; `Spec.key_requires`; `--key` without `--seed-chat yes` is `conflicting_options`; envelope `operation` is the message, `operation --key … --command knowledge-check` lists the chat in `related`); keyed full/chat/project intake calls `full_import_replay` right after the intent match, before `settings()`/`converter()` and outside the useful-intake sub-block. README retry rows and edge-case paragraph with the C2 replay sentence, `AGENT_HELP`. Round-1 usage case `knowledge-check --seed-chat yes --key k1` (`unknown_option`) is now accepted by design; that row now checks `knowledge-check --key k1` → `conflicting_options`.
- Tests: app (`keyed_owner_operations.rs`) `keyed_topics_create_converges`, `keyed_seed_chat_replays_first_findings`, `keyed_full_import_replay_does_not_bypass_an_archived_chat` (the key is allocated by the refused first attempt rather than prepared: an attachment is refused before prepare, so this is the reachable unapplied state; replay is `None` and the normal path refuses again), `keyed_full_import_replay_detects_a_changed_title` (title, path, scope), `keyed_full_import_replay_matches_html_source_bytes`, `keyed_full_import_replay_after_reimport_is_none`. Process (`agent_retry.rs`): `keyed_topics_create_retries_without_a_second_note`, `keyed_seed_chat_replays_and_ignores_later_findings`, `knowledge_check_key_requires_seed_chat`, `keyed_intake_retry_replays_before_conversion_through_a_symlinked_folder` (no "Reading the document…" on the retry or on the retitled `key_conflict`; also a keyed chat attachment), guards `keyed_undo_with_a_different_original_is_key_conflict`, `keyed_keep_in_brn_with_a_different_note_is_key_conflict` (round 1 already refused key reuse; they close round-1 gap 11).
- Also in this step: the SIGTERM/SIGINT tests in `agent_bounds.rs` wait for readiness by writing 256 KiB to the child's stdin (the round-1 pattern) instead of sleeping, after a 500 ms sleep proved too short under a parallel suite; the hard-stop upper bound is 6 s.
- Verification: `cargo test -p brn-threads-app --locked --offline --no-fail-fast` 151 passed; `cargo test -p brn` twice in full: only the 4 sandbox-blocked SIGINT tests fail; workspace build and strict clippy clean; journey passed with an empty parent.
- Baseline: the 4 non-guard process tests fail (`unknown_option` for `--key` on `topics-create`/`knowledge-check`; the retry printed `Reading the document…`); the 2 guards pass.

### R2-7 ⚑ A10 no default credentials in CLI-created workspaces (owner B2)

- ⚑ Shared service (app, additive): `Workspace::open_without_default_credentials` beside an unchanged `open`; both call a private `open_with(data, DefaultCredentials::{Derive, None})` that passes the choice to `bootstrap_settings` (`None` writes `auth_file: None`). The change stays inside `bootstrap_settings` (`settings.rs` :21–74 region; daily edits only from :98). Desktop still calls `open`.
- CLI: `execute` opens with `open_without_default_credentials`. `contract::require_credentials` refuses in machine mode with `credentials_not_configured` (rejected 6, `{provider}`, retry `refuse`) at the top of `fn ask` (shared with useful intake) and in the intake arm right after the single `workspace.settings()` read (C4), before `converter(kind)`/`convert`, when `intent == useful` or the kind is not Markdown/Text. Default mode keeps today's path. README "Credentials" (B2 rule, recipe, the C4 desktop sentence, the shared `fn ask` note), error row, Compatibility item 12, change row.
- In-repo callers: `agent_contract.rs::ask_refused_before_the_provider_is_rejected_not_failed` first runs `configure --provider Copilot --model synthetic --effort low --credentials-dir <root>/creds` (assertions unchanged); `cli_journey.py` now requires `auth_file` and `credentials_dir` to be null; the live Codex arms of `evaluate-wiki-guidance.py` and `evaluate_wiki_v2.py` call `parser.error("…Codex needs an absolute --auth-file")` (tests `test_live_codex_needs_an_absolute_auth_file`, `test_live_codex_arm_needs_an_absolute_auth_file`); `scripts/qualification/threads-runtime/live-journeys.py` already passes `--auth-file` (no change).
- Tests: app `open_without_default_credentials_stores_no_auth_file`; process `agent_credentials.rs` (`cli_created_workspace_has_no_credential_source` with synthetic `home/auth.json` and `home/.codex/auth.json` present, `ask_without_credentials_is_refused_before_any_commit`, `provider_intake_without_credentials_is_refused_before_conversion` (`.html` full and `.md` useful refused with empty stderr, no commit, no chat; `.md` full still applies), `refused_ask_stays_refused_and_records_nothing`); C5 guard `agent_credentials_existing.rs::existing_workspace_keeps_its_stored_auth_file` (one test; `CODEX_HOME=<root>/home` set in-process before `Workspace::open`; CLI `settings` reports `<root>/home/auth.json`; no provider call).
- Verification: app 152 passed; `cargo test -p brn` passes except the 4 sandbox-blocked SIGINT tests; workspace build, strict clippy clean; scripts tests 151 OK; journey passed with an empty parent.
- Baseline: the 4 `agent_credentials` tests fail (derived `auth_file`; `history --limit` unknown; the baseline `ask` ran and ended `Failed`, exit 7, instead of a pre-commit refusal); the C5 guard passes on both binaries (compatibility).

### R2-8 A2 offline outcomes through a test-only stub, and the witness repair

- **Spike (about 20 min, passed all four checks), synthetic data under `/private/tmp/brn-cli-agent-contract/spike`, fresh `HOME`/`CODEX_HOME`/`TMPDIR`, proxies `http://127.0.0.1:9` with `NO_PROXY=127.0.0.1,localhost`:** a 0600 `copilot.json` `{"token":"SYNTHETIC","expires_at":4102444800,"endpoints":{"api":"http://127.0.0.1:PORT"}}` in a 0700 dir, `configure --provider Copilot --model synthetic-model --effort low --credentials-dir DIR`, then `ask`: (1) the cached unexpired record is reused, no token refresh; (2) the only request was `POST /chat/completions` with `Bearer SYNTHETIC` at the loopback stub; with `endpoints.api=http://192.0.2.1` the run ended `Failed:Network` in about 1 s through the closed proxy port instead of contacting the TEST-NET address; (3) no `github-token` file existed; (4) minimal SSE chunks ending in `data: [DONE]` gave `Completed` with the scripted answer, and an HTTP 500 gave `Failed`, exit 7. No production change. (A spike shell left `HOME`/`TMPDIR`/proxy variables exported in the agent's shell for one later command, which failed to reach rustup; the variables were restored before anything else ran.)
- Tests: `crates/brn/tests/support/stub.rs` (std-only `TcpListener` stub: scripted SSE answers and tool calls, a held stream, HTTP errors; records every request; synthetic credentials; proxy environment) and `crates/brn/tests/agent_runs.rs` (11): success `stub_ask_completes`, `configure_then_ask_reaches_the_provider` (refused before `configure`, stub untouched; reaches it after); invalid input `stub_ask_on_an_unknown_chat_never_contacts_the_stub`; refusal/conflict `stub_provider_error_ends_failed` (exit 7), `stub_candidate_goes_stale_after_owner_edit` (model candidate, owner save, `apply` → exit 3 with `conflict[0]` at version 2), `stub_killed_ask_leaves_a_working_run` (SIGKILL during a held stream: `runs --state working` shows `guards_writes:true`, `topics-choose` is `deferred` with `conflict[0].kind == "Run"`; after `pause` the run is not Working and `guards_writes` is false; never asserts `Cancelled`); retry `stub_model_candidate_review_apply_and_dismiss` (`delegation --choice review-first`; the scripted model reads two notes and prepares two candidates; `candidates --limit 1` pages; `operation` shows `prepared` with the read input `Current` and the chat input `ChatContext`; `apply` applies, a repeated `apply` replays the same operation; the second is dismissed), `runs_page_filters_by_thread_and_state`, `stub_sigint_interrupts_a_held_stream` (130 and default-mode exit 0 with `"terminal":"Interrupted"`), `stub_sigterm_interrupts_a_held_stream` (143, `runs --state interrupted`), `stub_deadline_interrupts_a_held_stream` (`--timeout 2` → exit 7, `stop:"deadline"`, outcome `Interrupted`). Signals are sent only after the stub has received the request.
- **NeedsReview/Superseded audit:** not reachable from CLI owner authority. Every CLI write uses `HostAuthority::owner(…)` with no grant, run, chat or project binding (`brn-threads-app` has no `for_run`/`for_chat`/`for_project_context` call; `types.rs` `allows` returns true for `Scope::Owner`). In `store.rs` `apply_on`, `NeedsReview` comes only from a Grant bound to another operation (~1076), Runtime replay (~1086), chat-bound inputs (~1184–1211), non-owner writing markers (~1217), Maintenance without standing delegation (~1236) and denied capabilities (~1642, never for Owner); `Superseded` only from a project binding (~1101) or run binding (~1123–1150). README "Outcomes" now says which outcomes a CLI command can see; the model-side ones stay covered by unit and app tests.
- Witness `docs/work/active/chats-and-projects/cli-witness.py` (default mode): the SQLite v3 fake (former lines 14–29) and `import sqlite3` are gone; it now checks `upgrade --backup-directory <abs>` on the current workspace (exit 0, `history --limit 1` identical before and after); `search` results use `['hits']` (the witness had been failing since `search` returned a page); children get a `TMPDIR` inside the witness root, and it accepts the binary as an optional argument. The old-format evidence row in `docs/work/active/chats-and-projects/evidence.md` now points to `chat_storage.rs::adopted_upgrade_requires_external_backup_and_preserves_the_v3_snapshot` (invariant 17: the CLI cannot produce an old format). Run: `TMPDIR=<fresh parent> python3 docs/work/active/chats-and-projects/cli-witness.py "$PWD/target/debug/brn"` passed; the parent was empty (`rmdir` succeeded).
- Verification: `agent_runs` 10 passed, 1 sandbox-blocked (`stub_sigint_interrupts_a_held_stream`, SIGINT not delivered here); the rest of `cargo test -p brn` as before; strict clippy clean.
- Baseline: 10 of 11 fail (no `runs`, page options, `stop` or `--timeout`; the baseline `ask` on an unconfigured workspace ran and failed instead of refusing; SIGTERM killed it, exit −1). Guard: `stub_ask_on_an_unknown_chat_never_contacts_the_stub` passes at baseline (round-1 pre-provider refusal, kept).

### R2-9 A12 stale edits stay conflicts; `edit_session_open` never names the session (owner B3 part 1)

- CLI: `contract::begin_edit_error` drops `session` (details `{note, base_version}`; still inferred from `Sqlite` + `recovery_buffers()` on this branch, typed at merge). C2: `CliError.message` and `Code::EditSessionOpen.meaning()` read "This note has an open unsaved edit session (details.note, details.base_version). Write to it only if your own successful begin-edit returned it; otherwise report it to the owner." Default-mode text unchanged (`SQLite: UNIQUE constraint failed: edit_sessions.note`). No auto-rebase.
- Docs: README error row, retry row, the six-step "Stale save" recipe (C7 generations), the `edit_session_open` recipe with the round-3 known limit, the before/after example and a change row; `AGENT_HELP` carries the recipe in one line.
- Updates: `agent_contract.rs::open_edit_session_reports_edit_session_open` expects `{note, base_version}` and that the envelope contains no session ID; the `contract.rs` unit test expects the same; `cli_journey.py` asserts `'session' not in details`, that the envelope never contains A2's ID, and `details == {note, base_version: 2}`.
- Tests `crates/brn/tests/agent_edits.rs` (3): refusal `edit_session_open_never_names_the_session` (both modes, all four streams); success `stale_save_recovery_recipe_from_the_readme_completes` (parses the README recipe's steps with `include_str!` and follows them: the version goes from 2 to 3 with the merged text; the discarded session then saves `Closed`) — guard: the recipe already worked at baseline, the test protects the documented text; retry `recipe_save_retry_replays_saved` — guard (round-1 replay). Invalid input: n/a (no new input surface).
- Verification: `agent_edits` 3 passed; `agent_contract` 19 + 3 sandbox-blocked; unit 23; journey passed (651 checks) with an empty parent; strict clippy clean.
- Baseline: `edit_session_open_never_names_the_session` fails ("the session ID leaked"); the two guards pass.

### R2-10 ⚑ A11 no-op converge commands record nothing (owner B4)

- BOARD.md re-read (modified 09:58: daily posted READY FOR INTEGRATION @ `192cd765`, agreed with this branch's `main.rs` resolution including dropping `session`, and noted `archived` on every search `hits[]`/`messages[]` entry; main is now `a4ad9677`, docs-only after `8bef439b`). Daily tip at this step: `192cd765` "docs: record the round 6 native check".
- ⚑ Core write path (owner-approved exception): new `crates/brn-threads-core/src/converge.rs` (`Converged<T>`, `Unchanged {record, version}`, private `detect_unchanged`: `Some` iff every write targets an existing record at its current version with unchanged `archived` and data, project chats/documents compared as sets, `MoveChat` ignoring the `context_epoch` bump). `chat.rs` and `projects.rs`: the bodies became private `*_body(.., converge)`; `chat_command`/`project_command` keep their signatures and pass `false` (the impossible `Unchanged` arm is `Err(Invalid("no-op detection is off"))`, no panic); new `chat_command_converging`/`project_command_converging`. The hook sits right before `prepare_on` (after the ChatControl push; after the `Vec<Put>` conversion), so binding, receipt replay and every refusal run first; it applies only to `Archive`, `Unarchive`, `MoveChat`, `LinkDocument`, `UnlinkDocument`. `Unchanged` returns without committing (the operation stays `allocated`). No `ApplyOutcome` variant was added.
- ⚑ App (additive): `crates/brn-threads-app/src/converge.rs` with `archive_chat_converged`, `unarchive_chat_converged`, `project_command_converged`, repeating the twins' `chat-control:`/`project:` prefixes, SHA-256 command binding and "owner chat action"/"owner project action" authority. Desktop keeps the receipt path (gap recorded: `threads_projects.rs:57`, `threads_ui.rs:2183,2203`).
- CLI: the six commands call the converged functions; `Unchanged` prints `{"outcome":"Unchanged","record":ID,"version":V}` (C6: the chat for archive/resolve/unarchive/chat-move, the project for link/unlink); envelope `ok`, exit 0, `outcome:"Unchanged"`, `operation:null`, `changed:[]`; spec outcomes `["Applied","Unchanged"]`; the `converge` class meaning gained the B4 sentence. README outcome row, class row, Compatibility item 11 with the checked callers (C6, including `docs/work/active/projects/cli-witness.py`), change row.
- Merge notes for the expected `chat.rs`/`projects.rs`/app conflicts: `.worktrees/cli-agent-contract.artifacts/round2/merge-resolution-notes.md` (re-derived at R2-15).
- Tests: core `crates/brn-threads-core/tests/converge.rs` (4: all five variants `Unchanged` with receipts and versions unchanged and the operation left `allocated`; the existing functions still append; a Working run makes archive `Changed` with a fence; a binding mismatch is `ImmutableRequest`); app `converged_twins_share_host_keys_and_authority`; process `crates/brn/tests/agent_converge.rs` (5): `repeated_archive_and_resolve_are_unchanged_without_a_receipt`, `repeated_unarchive_move_link_unlink_are_unchanged`, `archive_with_a_working_run_records_a_change` (the plan's `archive_after_resume_pauses_and_records_a_change` is not reachable through the CLI, which has no command that resumes a chat queue; this test covers the same refusal-to-collapse rule with a run left Working by a killed `ask` through the stub: the archive is `Applied` with a Run write, the repeat is `Unchanged`), `keyed_archive_retry_after_a_change_replays_the_original_receipt`, `default_mode_unchanged_prints_the_unchanged_object`.
- Verification: `cargo test -p brn-threads-core -p brn-threads-app -p brn-desktop --locked --offline --no-fail-fast` 296 passed, 0 failed (desktop unchanged); `cargo test -p brn` passes except the 4 sandbox-blocked SIGINT tests; both witnesses passed with an empty temp parent; workspace build and strict clippy clean; `git diff --stat 8bef439b..HEAD -- crates/brn-desktop crates/brn-ai` empty.
- Baseline: all 5 process tests fail (`history --limit` unknown; default mode printed a new receipt instead of `Unchanged`). `keyed_archive_retry…` fails at baseline only on `history --limit`; its replay assertion is round-1 behavior, so it is effectively a guard.

### R2-11 A6 mistyped data directories and values before opening

- Caller survey (default mode, unchanged): `crates/brn/tests/threads.rs`, both witnesses, `scripts/verify-end-to-end-fixtures.py`, both wiki evaluators' `Cli.call` and `scripts/qualification/threads-runtime/live-journeys.py` run without `--json`. Machine-mode callers: `cli_journey.py` (runs `init` first), `evaluate-wiki-guidance.py` `Cli.envelope` (reads `operation` on an initialised workspace), `agent_contract.rs` (two tests read a fresh dir: `keyed_markdown_full_import_replays_and_title_change_conflicts` and `agent_credentials.rs::cli_created_workspace_has_no_credential_source` now run `init` first; `agent_inspect.rs::runs_page…` too).
- CLI: before `Workspace::open`, a machine-mode read (`retry_for == read`, which covers unseeded `knowledge-check`, `revisions`, `revision`, `operation` and `runs`) on a missing path or an empty folder returns `missing_workspace` (rejected 6, refuse) and creates nothing; a foreign non-empty folder still reaches the core (`unsafe_directory`); `init` and writes still create. Reserved `data_folder_unusable` (rejected, `{problem}`) is listed with `"reserved": true` (every description error entry now carries `reserved`). `VALUES` gains `Grammar::Number`/`Grammar::Choice` entries for today's after-open checks with the same messages: integer syntax for `--base`, `--generation`, `--hops`, `search --limit`, `--budget` (keeps "Invalid budget"); `--knowledge`, `--seed-chat`, `--sources`, `--intent`, `--provider`/`--helper-provider` (case-insensitive), `--effort`/`--helper-effort`. Service ranges and `--trash` stay after opening (recorded gap). README rows, Compatibility item 8, "Streams" text and change rows.
- Tests (`agent_discovery.rs`): invalid input `enum_values_are_refused_before_the_workspace_opens` (11 cases, both modes, stderr text kept, dir never created); refusal `machine_mode_reads_on_missing_or_empty_dirs_are_missing_workspace` (`notes`, `revisions`, `operation`, `runs` exit 6 with the path left absent; an empty dir stays empty; guard parts: a foreign dir is `unsafe_directory`, default-mode `notes` still exits 0 and creates; success/retry: after `init` the identical read is `ok`, and `note-create` on a missing dir creates it).
- Also: `timeout_during_fifo_wait_is_a_soft_deadline` now allows 3 s (about 1 s alone; it exceeded 2 s once under a parallel suite).
- Verification: `cargo test -p brn` passes except the 4 sandbox-blocked SIGINT tests; strict clippy clean; scripts tests OK; journey and the projects witness passed with an empty parent.
- Baseline: both new tests fail (the baseline created the dir for `notes` and returned `ok`; `--knowledge maybe` was checked after opening, so the dir was created).

### R2-12 A5 per-command help, global options anywhere, absolute paths, owner-decision mark

- CLI: new `crates/brn/src/help.rs` (one summary and example per command, notes for `intake` (the A3 replay sentence and the owner mark) and `save` (the A12 recipe), text help rendered from the command's description entry); `spec::describe` is the per-command entry shared by `brn --json --help` and `brn --json COMMAND --help` (`{"contract":2,"command":ENTRY}`); the description gains per-command `summary`, `example`, `note`, `owner_decision_when` and top-level `global_options` and `command_help`. `parse()` (no clap): a leading `--json`; before the command `--data-dir`/`--timeout` in any order (another `--x` without `--data-dir` keeps `missing_data_dir` with the round-1 text); after the command `--name value` pairs with name-position `--data-dir`/`--timeout` lifted out and `--help` (no value) recognised; repeats are `repeated_option`; values are never inspected. An unknown command stays `unknown_command` (also with `--help`), or `missing_data_dir` when no `--data-dir` appears at all, as before. `Grammar::AbsolutePath` for `backup`/`export --destination`, `restore --source`, `upgrade --backup-directory`, `model-install --destination` ("--NAME must be an absolute path", before opening, both modes). Owner mark: `owner_decision_when` on `intake` (`--thread` naming a Project), a README owner-table row and an `AGENT_HELP` line; behavior unchanged.
- In-repo callers of the absolute-path options (re-grepped): `crates/brn/tests/threads.rs`, `agent_contract.rs`, `cli_journey.py`, both witnesses and the round-2 tests pass absolute paths; the one relative case was the round-1 test `…export --destination relative-export` (expected `invalid_request` after opening), now `invalid_value` before opening by design (Compatibility item 6).
- C3: `HELP` is byte-identical (`git diff -U0 8bef439b -- crates/brn/src/main.rs | grep -c '^[-+]const HELP'` is 0; the `const HELP` line hashes the same on `8bef439b` and HEAD); `brn --help` prints `HELP` + `\n` + the updated `AGENT_HELP`, exit 0. The useful-intake sub-block is untouched.
- Tests (`agent_discovery.rs`): success `every_command_has_text_and_json_help_matching_the_description` (all 56 commands, both modes, also with a `--data-dir` that stays absent), `data_dir_and_timeout_are_accepted_after_the_command` (also `--title --data-dir` stays a title); invalid input `relative_backup_and_restore_paths_are_refused_before_opening` (both modes), `help_after_an_unknown_command_is_unknown_command`, `repeated_global_options_are_repeated_option`; guard `bare_and_unknown_invocations_keep_their_exits_and_help_text` (`brn`, `brn --unknown`, `brn notes` exit 1 with "Supply --data-dir…"; `brn --help` exit 0 starting "BRN Threads\n\nUsage: brn --data-dir ABSOLUTE_DIRECTORY COMMAND"). Unit `global_options_are_not_command_options_and_every_command_has_help`. Round-1 `usage_errors_are_json_with_stable_codes_and_create_nothing` is unchanged and passes.
- Verification: `cargo test -p brn` passes except the 4 sandbox-blocked SIGINT tests; strict clippy clean; journey and both witnesses passed with an empty parent; `brn --help` 0, `brn` 1, `brn --unknown` 1.
- Baseline: the 5 new non-guard tests fail (`missing_data_dir` for `COMMAND --help` and for options after the command; relative paths reached the service); the guard passes.

<!-- ===================== ROUND 2 END ===================== -->
