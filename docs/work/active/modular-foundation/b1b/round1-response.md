# B1b Opus round1 precision response

Principal-owned Opus approved production at `601e95d343fcc9bf86edda832b06eb35ac6f4657` with no production defects. The [full report](opus-review-round1.md) is copied verbatim: **3,079 bytes**, SHA-256 `e9869a6c27dda1ded84cb2a7766ac596c30c0cbabdf4429306b70979b9342ab4`. Nonblocking preferences require no production changes or new design/review round.

## Historical source-only LOW disposition

The second fixture is now named `saved_retry_refuses_wrong_displayed_context_and_preserves_archived_state_at_offline_selection`. Its archive leg adds the exact existing offline provider-selection notice, including the retry wrapper. Static source traces `offline-fixture` to `RunSelectionError::Provider`, then the unchanged selection warning and retry notice. The added assertion is **not executed**; runtime confirmation awaits a grant. No archive-specific retry rejection or Unarchive guidance is claimed.

Wrong Note/general-vs-Project and changed-conversation assertions are untouched. All previous state-preservation assertions remain; removing only the new assertion/comment and reversing the rename exactly restores the reviewed test file. The entire **57,827-byte** pre-B1b test prefix is unchanged. Existing separate ordinary-chat `archived_writing_chat_preserves_feedback_and_requires_explicit_unarchive` evidence remains: it asserts Unarchive at line 1096 and uses the real composer control. This is separate from saved retry's offline-selection leg.

Plan, historical preparation, verification and ownership descriptions now distinguish newly proven displayed-context refusal, archived saved-state preservation at offline selection, and separate existing ordinary-chat Unarchive checks. Original source-preparation/verification JSON, all 17 original command records, baseline patch and omission proposal are byte-unchanged. Historical baseline/candidate executions retain the original fixture name `saved_retry_refuses_wrong_displayed_note_scope_and_archived_context_without_writes` and their actual source identities; the rename does not relabel any execution.

## Historical source-only receipt and BUILD NEED

The [static source receipt](round1-source.json) records six source/manifest/lock hashes and fingerprint **`ffcc9df9231bcb4a5d7cefc03ac4ea5eafaac0ae093fae3ebc369e5f543fa8c0`**, exact test reversal and historical artifact identities. Of the 295 historical manifest entries, **294 remain byte-identical**, with only the allocated test changing. All approved production/manifests/lock remain unchanged; the remaining delta is allocated B1b docs. Direct pinned Rust1.98.1 rustfmt and `git diff --check` pass. The receipt records correction of a static checker assumption that the historical manifest excluded tests; no production/test behavior was adjusted for that checker.

No Cargo, compilation, tests, lint, metadata, mutants, headless or process polling occurred. This correction remains unstaged/uncommitted and unqualified; original `601e95d3` checks retain only their original-source meaning.

**BUILD NEED:** after principal grants the exclusive slot, run the single renamed native fixture with `native-ui,native-test-support` on this actual source, locked/offline, pinned Rust, fresh synthetic TMPDIR and the correctly bound candidate target. The exact command is in the static receipt. No broad normal-lane repeat is requested. D1A currently owns the sole heavy slot.

The existing one-site contextual-reader omission proposal remains **ungranted and unexecuted (0 seconds used)**. Later allocation may run the unmutated actual fixture plus ONE omission variant in a disposable fixed source and separate target, ≤180 seconds or a proportional safe compile limit within the original cumulative ≤1800-second B1b cap. Preserve original proposal bytes; select the new fixture name only for future runs. No unchanged CW2/UI campaign.

Principal acceptance, carried PDF/pdftoppm failure, hosted F22, native-human/owner and full F21/program gates remain separate and open. No other slice or production correction is started.

## Granted affected-check window

Principal subsequently granted the sole heavy slot for the affected correction and one omission recheck. On actual precommit HEAD `601e95d3` plus the recorded precision delta, the renamed native fixture **passed: 1 passed / 0 failed / 272 filtered**, including the exact offline-selection notice. Strict desktop all-target Clippy with `native-ui,native-test-support` and `-D warnings` passed; workspace formatting and static link/diff checks passed. [Actual command/source/log/artifact receipts](round1-checks.json) retain the real precommit identity. These results supersede only the unexecuted scheduling statements above, without rewriting the historical source receipt or original before evidence.

Approved production/manifests/lock remain byte-identical to `601e95d3`; only the allocated test and B1b docs change. No full workspace/native script/combined/headless repetition occurred. The ONE omission recheck is separately bounded at **300 cumulative seconds including setup, baseline, variant, restore, reap and verification**; no variant had started when these affected checks completed. Principal acceptance remains pending.

## Completed bounded omission recheck

On fixed disposable source `/private/tmp/brn-b1b-before-20261016-source` and its own `/private/tmp/brn-b1b-before-20261016-target`, all **955** material files were matched to committed correction `9b4f48e768ac561e84a3bffb4bb78c385b4d6147`. The unmutated renamed fixture passed **1 / 0 / 272 filtered**. ONE seed replaced only the contextual reader with `writing_request(message)`, retaining Note/scope references and all other construction. It compiled successfully, then failed `view.notice.contains("unavailable in this displayed context")`: **0 / 1 / 272 filtered**. This is **CAUGHT**, not a compile-only verdict. [Exact patch, commands, source/input/binary hashes and raw assertion](guard-recheck.json).

Final closed cumulative time was **208.817199 seconds of the 300-second ceiling**, including the initial wrong-SHA preflight, setup, baseline, variant, restoration recovery, immutable seeded-binary preservation, restored-cache invalidation and final full hash/process verification. The intermediate 137.305-second update preceded the last artifact/cache verification; it is not the final total. No clock restart or second variant occurred.

Two harness defects are retained: the initial wrong-SHA identity guard refused before any copy/build/variant; tar restoration later attempted to overwrite an identical read-only synthetic Git object. Recovery wrote only differing snapshot files and verified the complete original source/symlink/Git manifest. Original before commit/tree, test name/patch/logs and its original binary remain unchanged. Changed restored material mtimes were advanced to invalidate seeded cache artifacts; no post-restore executable reuse occurred. The baseline binary hash was recorded at completion; its literal artifact path was later overwritten by the seed, whose binary is separately retained.

The heavy slot is explicitly released. All B1b production/test/shared hunk ownership, including unused reserves, is returned to principal in the [final handoff](final-handoff.md). Principal acceptance and all previously open external/program gates remain pending.
