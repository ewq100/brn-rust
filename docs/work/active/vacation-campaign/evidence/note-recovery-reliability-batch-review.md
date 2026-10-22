# Note recovery reliability batch: independent integration review

**Approved exact candidate `dfe4cf91e3ed19800f68aaa55f2a32a9eea2a8b0`, with no actionable findings.** Reviewed against actual main208 `11c48f9b91cb0d4e9deba9e0f45b43ef284b63e1` in `/Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust`, branch `codex/note-reload-generation-guard`. Actual HEAD and branch match the assignment; staged, working and untracked status were empty at inspection and final confirmation. This reviewer authored neither component nor integration and performed only source/evidence reads. The sole write is this report; no tests, builds, CLI workloads, network/CI/account/native actions, source edits or nested helpers were performed.

## Complete candidate and identity checks

Inspected the complete base-to-candidate diff: exactly four files, 159 insertions and 19 deletions. Verified entire-file Git blob equality, not merely patch similarity:

| Batch file | Entire-file approved component | Identical blob |
| --- | --- | --- |
| `crates/brn-ai/src/provider_stderr_tests.rs` | `a6b6d9902c8797a696cb96e1d9e848c564425b9e` | `59ab6a947c05945c24abe0c2531b167c1342709c` |
| `crates/brn-writing-ui/README.md` | `d521722ab984eb2fd125a990b9d4e13956138717` | `70458cd247a4226c9f07bd2c97fdfaaedfa7c460` |
| `crates/brn-writing-ui/examples/writing_host/journey.rs` | `d521722ab984eb2fd125a990b9d4e13956138717` | `56f07cfbc201f07efc1189b93a174939bac6cf20` |
| `crates/brn-writing-ui/src/note.rs` | `d521722ab984eb2fd125a990b9d4e13956138717` | `efbd6c029814e2e4ac4674817983120dc804179d` |

Verified ancestry with merge bases: actual main208, Note `d521`, and provider `a6b6` are each ancestors of exact candidate `dfe4`. The first local merge `9c2707bf0569d123c84dc29f9de40016773ff0ed` has parents `d521` and main208; final `dfe4` has parents `9c2707bf` and `a6b6`. The complete main207-to-main208 scope is the eight previously reviewed maintenance files in core process witnesses and task-reading tooling/docs; it does not modify the four current component files. Main208 and qualified PR208 candidate `7dde6a41a4fbc66468e0400d2cc3c859f04c8ef5` have equal complete trees (also confirmed by empty whole-tree diff). No additional merge resolution or source edits were found.

Reused the complete independent component reviews explicitly after the foregoing actual diff and entire-file checks:

- `/tmp/brn-note-reload-generation-review.md`, approved exact `d521` after complete initial `bd3c` inspection and separate mechanical-format delta review.
- `/tmp/brn-provider-test-child-bounds-final-review.md`, approved exact `a6b6` after complete initial `5d4` inspection and strict UTF-8/constant-diagnostic correction disposition.

## Correctness, ownership and interaction assessment

The Note change remains eleven production lines at the shared replacement boundary: sample current visible input first, then re-read the local editing session and refuse non-explicit replacement when closed or generation-mismatched, before `load_note` mutates panel/editor/comparison state. Checked session identity and monotonic generations remain the authority. Explicit same-note Recover still reaches the existing current-buffer/hint validation; it does not trust stale hint contents. Save, Discard, SQLite/history authority and ordinary valid navigation retain their existing responsibilities. There is no new polling, persistence write, public API/schema, ownership protocol or generation framework. This is transition-time validation, not a claim of a new cross-process atomic presentation snapshot.

The existing host journey still targets the distinct stale local-equality shortcut after the previously checked Save/Discard refusals. Ordinary same-note reopen and another valid Note navigation must refuse while preserving exact visible text, local session/record, complete durable buffer vector, canonical Note and history. Explicit Recover and subsequent successful navigation demonstrate recovery remains available and the guard does not become permanent. The unchanged adjacent journey covers invalid-hint refusal and validated recovery of a blocked attempt. The README accurately documents this narrow contract.

The provider component remains private test-only lifetime management in the existing isolated synthetic stderr witness. It owns only its spawned child, reaps normal success, retains kill/wait cleanup through timeout/panic, replaces pipe capture with private separate files, checks each accepted capture size before reading, and preserves exact child selection, success/three-route markers and redaction assertions. Strict UTF-8 rejection uses constant diagnostics without raw capture bytes. Its 64 KiB check is a post-exit accepted/read bound, not a hard live disk quota; finite watchdog/RAII cleanup does not create a guarantee against an OS refusing termination/reap.

The two components share no newly introduced state, environment contract, helper, public interface or production control flow. The private `brn-ai` test guard is distinct from the existing test-only core process helper introduced by main208; no name/export/lifetime responsibility is crossed. Main208 task-reading changes remain unchanged, including default generation and record preservation. The current batch adds neither tooling edits nor new process witness retirement. No interaction or invariant-placement regression was found.

## Evidence attribution and remaining gates

Read the lead-owned source-bound qualification receipts under `/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust/docs/work/active/vacation-campaign/evidence/`. These are component execution evidence, not reviewer execution or combined-head results:

- `note-reload-generation-qualification.json`: exact `d521`; writing-UI library 44 passed, host 12 passed, selected desktop module 344 passed/0 failed/1 ignored/3 filtered; strict all-target lint, production build (not launched), formatting, links, whitespace and clean-source checks recorded passing. The prior actual test-only baseline failed at the intended ordinary shared reopen refusal.
- `provider-test-child-bounds-qualification.json`: exact `a6b6`; library 500 passed/0 failed/1 intentionally ignored child; strict lint/format/clean checks passing. The separate negative receipt records the expected one-test watchdog failure after 1.01 seconds, actual owned PID 61304 reaped with signal 9, and restoration of exact source hash `764ac8d1ccb09684c1f2b7996897d7834415798425cc2b78ff28ee99379c4c77` before final checks. The deliberate diagnostic failure is separate from final successful qualification; no overlay is accepted as candidate source.

The lead reported combined-head gates starting; no combined final passing result was supplied or inferred at this report. Required hosted checks, resulting-main checks (including the separately pending main208 run), publication, merge and release remain separate gates. Local tree equality and ancestry do not establish their status. This report makes no live-provider/account usefulness claim and no native interaction or owner-acceptance claim. In particular, desktop dirty Reload is disabled; the Note witness qualifies shared API reopen/navigation, not a native dirty-Reload control interaction.
