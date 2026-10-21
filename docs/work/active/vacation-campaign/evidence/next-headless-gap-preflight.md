# Next headless reliability gap preflight

## Result

No justified new headless reliability task emerged from this bounded pass. Queue/process recovery, original-data preservation through backup/restore, and receipt-specific Undo already have source-backed witnesses or completed follow-ups; the three current relevant headless candidates (Settings draft retention, preview-file cancellation and export interruption) are active and should finish without overlap. Owner-unavailable Mac interaction remains deferred by current campaign authority; no GUI/native qualification is inferred.

## Exact state inspected

- Lead checkout: `/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust`, branch `codex/agent-first-followthrough`, HEAD `9549e9609201d4a7c1fe3e6fdae7c0daa95c4206`, with pre-existing lead-owned changes only under `docs/work/active/vacation-campaign/` and `docs/work/overview.md`; preserved.
- Source ancestry at this head includes current integrated PR194 main `a1e1d899`; this is read-only source/evidence review, not a gate check.
- Read `docs/work/active/vacation-campaign/workflows.md` and `test-families.md` summaries plus current `docs/work/overview.md` and the selected campaign entries. Current state routes Settings worker draft retention (`140439f9`), CLI preview-file cancellation (`PR195/cacab2e5`) and export interruption (`PR194/a1e1d899`, resulting-main gates pending) to their active owners.

## Covered priority areas

- **Queue and interrupted work:** The source audit `docs/work/active/vacation-campaign/evidence/2026-10-20-queue-recovery-audit.md` found no consequential queue/Run gap and names the distinct witnesses. Current `crates/brn-threads-app/tests/run_recovery.rs:144-239` covers restart recovery with one queued dispatch, a live lease blocking Resume, one-time recovery and non-undoable recovery. `tests/chat.rs:96-194` covers durable dispatch/replay and late or forged completion with stopped partial output. No separate source defect or missing boundary appeared in this pass.
- **Original data and restore:** The existing `crates/brn-threads-core/src/restore.rs:432-473` regression checks that corrupt, incompatible, foreign and empty sources leave the destination parent absent and source bytes unchanged. Current campaign evidence records restore-source `SQLITE_NOTADB` classification and completed PR190 disposition; this closes the older invalid-input error finding. `crates/brn-threads-app/tests/chat.rs:265-364` exercises a nonempty synthetic retained asset and Source/Link records, checks exact receipt records after reopen and restore, then Keep and Undo. PR189 is recorded complete with all four required head, preceding-main and resulting-main checks passed. Its explicit limit remains a synthetic three-byte Asset, not image conversion/PNG fidelity or native behavior.
- **Backup interruption and publication:** `crates/brn-threads-core/src/backup.rs:424-511` covers a killed backup stage, owned cleanup on retry, lock release and preservation of unrelated stage-like data. `restore.rs:339-379` similarly covers killed restore-stage recovery; `restore.rs:408-473` covers publication failure and malformed-source preservation. The prior `2026-10-20-backup-failure-audit.md` found no backup/restore source defect. Export was the concrete publication gap it found and is now addressed by the active PR194 kill/retry witness; its remaining resulting-main gate is already tracked.
- **Undo:** The older `action-history-audit.md` missing witness was completed by PR182. Current `crates/brn-desktop/src/threads_tiled_tests.rs:406-454` activates Confirm done, checks completion evidence and the exact receipt, then routes to that receipt’s Undo. Store conflict and version protections remain covered by core tests per the test-family map.

## Bounded disposition

Do not start another task from this evidence set. Finish the existing active candidates and their normal review/integration gates. Revisit headless work only if a gate or review identifies a concrete regression, or a later source audit finds a distinct untested transition in the data/recovery/queue/Undo path. Keep the retained-asset conversion/native limit separate from a source-level persistence gap; the current supported fixture proves the latter only.

No repository files, source, fixtures, processes, accounts, providers, network or native UI were touched. No tests/builds ran. The dirty campaign/overview files and three untracked Settings evidence files were left unchanged.
