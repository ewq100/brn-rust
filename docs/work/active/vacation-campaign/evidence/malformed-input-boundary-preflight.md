# Malformed-input boundary preflight

**Candidate for lead selection:** `brn project-delete --preview-file` has a concrete file-refusal gap. It reads the caller-supplied path synchronously with `std::fs::read`; a POSIX FIFO with no writer can block at open instead of returning the documented invalid-input refusal. This is a source-backed reachable boundary, not a failure reproduced in this preflight: executing file inputs was explicitly out of scope.

## Baseline and scope

- Checkout: `/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust`, branch `codex/agent-first-followthrough`, HEAD `64c09089edb78a8b072a3515c68e975012f768bc`.
- The inspected CLI/intake source and test files have no diff from reviewed main `9e2a81c0`. I left the existing dirty campaign, plan, and overview edits untouched.
- Followed `AGENTS.md` → `docs/work/overview.md` → selected task `malformed-input-boundary-preflight` (`docs/work/active/vacation-campaign/campaign.json:2027-2049`) → affected source, tests, and contracts.
- Read-only only: no builds, tests, file-input execution, fixture/archive generation, source edits, account/provider/model/native/web/network action, or Git write.

## Boundary evidence

- The `project-delete` command requires `--preview-file`; its option is not one of the five `TEXT_OPTIONS` that are routed through `input::resolve` (`crates/brn/src/spec.rs:488,817-822`; `crates/brn/src/main.rs:381-405`). The command arm calls `std::fs::read(preview-file)` directly before JSON decoding and maps ordinary I/O errors to `invalid_input` / `unreadable` (`crates/brn/src/main.rs:1078-1089`). There is no file-kind check, byte limit, or cancellable reader on this path.
- On POSIX, opening a FIFO for reading blocks until a writer appears. Applying that OS behavior to the direct `std::fs::read` call means a FIFO argument can leave the command waiting indefinitely when no `--timeout` is given. This is an inference from the source call and FIFO open semantics, not an observed run. The command starts its interrupt/deadline watcher in machine mode, but SIGINT only flips the shared cancellation flag (`main.rs:24-27,1707-1727`); the synchronous `read` does not poll it. An explicit timeout reaches the watchdog hard stop only after its three-second grace (`crates/brn/src/stop.rs:25-26,162-180`), producing a timeout/unknown-state result rather than the normal invalid-input refusal.
- The published CLI contract already classifies unreadable or malformed `--preview-file` as `invalid_input` (`crates/brn/src/contract.rs:267-269`). Existing cross-process coverage checks missing and malformed regular preview files and invalid `--actions` JSON (`crates/brn/tests/agent_contract.rs:578-650`); it does not exercise non-regular preview paths or a blocking open. The regular `--actions-file` path is separate: `actions` is in `TEXT_OPTIONS` and goes through the bounded, cancellable reader (`spec.rs:488`; `main.rs:381-405`; `crates/brn/src/input.rs:90-119,127-163`).
- The adjacent `--text-file` reader has a 16 MiB inclusive cap and explicitly isolates blocking file opens so cancellation remains observable (`crates/brn/src/input.rs:10,90-119,121-163`); its unit tests cover exact limit, one byte over, invalid UTF-8/NUL, directory and missing file (`:176-213`). Document source reads independently have a file cap and one-byte-over guard (`crates/brn-threads-intake/src/lib.rs:47-75`), with exact-bound, one-byte-over and directory checks (`crates/brn-threads-intake/tests/intake_safety.rs:54-75`). These are good adjacent controls, but neither covers the preview-file path.
- A numeric `--preview-file` cap is not selected here. `ProjectDeletePreview` contains serialized records, including full removed/retained records, and no current contract sets a serialized preview byte ceiling. Reusing the text-file 16 MiB limit could reject a valid large project preview without a documented bound. The FIFO refusal case tests an independently expected file-type boundary without inventing that policy.

## Distinctness from retained work

The old SQLite `NOTADB` restore-source error was addressed in the completed PR190/mainline path (`campaign.json:1591-1643`, `evidence/invalid-backup-error-audit.md`); this finding concerns a different command and I/O boundary. The retained-image experiment is a failed projection/policy qualification and does not justify changing image defaults (`evidence/image-retention-proof-review.md:29-35`). I did not reopen either issue. Backup publication/durability remains with the other worker.

## Smallest safe follow-up qualification

If selected, add one Unix-only cross-process case to the existing `crates/brn/tests/agent_contract.rs` caller-input refusal family. Use its fresh synthetic workspace helper, create one FIFO with no writer under a canonical temporary root, invoke `project-delete --preview-file FIFO --actions '{}'`, and enforce a **2-second child-process deadline** in the test harness that kills and reaps the child on expiry. Do not use `--timeout` as the assertion mechanism: the product watchdog’s deliberate grace makes that a different behavior.

Independent expected result: prompt `usage` / exit `2`, `retry: refuse`, error code `invalid_input`, details `{"option":"preview-file","reason":"unreadable"}`, and no changed records. The test should compare workspace records before and after. Against current source, timeout is the expected failure signal; stop after this one case, preserve its output, and do not retry or broaden to fuzzing.

If that witness is selected, the focused command after adding it is:

```sh
cargo +1.98.1 test -p brn --test agent_contract --locked --offline --jobs 1 preview_file_fifo_is_rejected_promptly_without_mutation
```

The evidence basis is source and existing test definitions only. The report identifies a narrow high-confidence blocking risk, but does not claim a reproduced hang or prescribe a general JSON byte limit; lead selection remains separate.
