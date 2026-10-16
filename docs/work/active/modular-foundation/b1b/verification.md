# B1b compiled candidate qualification and review handoff

B1b compiled candidate `601e95d3` received **principal-owned Copilot Opus round1 production approval**; [verbatim report](opus-review-round1.md). The subsequent [precision-only test/docs correction](round1-response.md) now has affected execution recorded in [round1 checks](round1-checks.json); the historical source-only receipt remains unchanged. Local acceptance remains pending. Review from accepted base `5e98b0157a70076f2fe69346f1496cccc869bc78` through the final pin reported after commit; source-only preparation pin `9bd8f469` is historical. Branch is `work/b1b-20261016`, bound checkout `/Users/evokessler/.t3/worktrees/brn-rust/t3-239c29dc`.

[Requirements and plan](plan.md), [exact ownership](ownership.md), [preparation](source-preparation.md) and [machine verification](verification.json) travel with the diff. No D1A source or lock was integrated. No other refs/worktrees, AI/core/CLI operations, canonical architecture or CI/scripts were changed.

## Actual source and scope

The panel constructs the existing shared ThreadRunStart from its displayed Note, canonical scope and exact context-checked saved request. Both writing and ordinary chat use one desktop Workspace.prepare_thread_run → spawn_prepared/channel/worker path. Generic start_run no longer detects RewriteNote or chooses writing continuation/completion text; the writing caller supplies its exact existing presentation. Ordinary queue/EditNote request assembly remains unchanged. CW2 had already removed arbitrary first-linked selection; the valid canonical-context display/draft/cache map is retained.

Actual production/test changes from the accepted base are six paths: library native.rs (new request/error adapter), desktop threads_ui.rs (retry/common start extraction plus one corrected comment), appended desktop writing tests, the one-line app existing-type re-export, the two desktop test-only dependency declarations and the single lock edge. All existing writing test bytes, synchronous dirty-buffer ordering, complete CW2 cache/key/error handling, scope/disclosure/Compare/Apply checks, editor, v1/v2 readers and Undo paths are preserved.

One candidate compile failure is retained: the core glob's one-parameter Result alias shadowed the standard two-parameter type. The new method now explicitly returns std::result::Result. This fixes only type resolution; no guard or test assertion changed. Native and default strict lint then passed without further source corrections.

Six-path source fingerprint: `204254d0291047ec8110e7b6c1cb4d3dd8e35786149290bf9e8747ed627e819d`. Full 295-file production/manifest/glue fingerprint: `9e7bcdde4ff41327ae971a65a3183681a6103303d5ca637dfd42cd5535b861c7`. Every passing candidate gate's contemporaneous source manifest matches those production bytes. The final commit is associated by exact bytes after creation, never substituted into the precommit command HEAD/diff records.

## Genuine failing-before and corrected proof

The baseline used a separate source root `/private/tmp/brn-b1b-before-20261016-source` and separate target `/private/tmp/brn-b1b-before-20261016-target`. Its genuine synthetic Git base tree **exactly equals** accepted tree `c3e5e792827b63f2dcfd69d60d7af8029f220b7c`; only the appended contextual-refusal fixture was applied. Candidate code, manifests and lock were not copied into it. Fixture patch SHA-256 is `97df04a50de5ae3a4d9d8c93188122469962a4001558f7813a0d4016ab7eba26`.

That exact baseline compiled and ran **0 passed / 1 failed / 271 filtered** at the intended assertion requiring “unavailable in this displayed context.” Old retry reached offline provider-selection refusal after reading raw saved evidence. This is a demonstrated **context-refusal precedence gap**, not an unauthorized-write/leak claim. There was no baseline fixture compile defect or weakened assertion. The same two candidate fixtures then passed **2 / 0 / 271 filtered** and both passed again as part of the single complete native suite. No standalone rerun followed the full suite.

The scoped fixture uses actual Rewrite/Retry clicks with a different hidden Chat and the real Copilot catalogue refusal; record/receipt counts, saved request and unsent feedback stay exact. It then sends the same panel request through the real shared preparer and existing scripted runtime, executes prepare_changes, completes a whole-note candidate without applying it, and inspects actual captured model requests for frozen scoped feedback and exclusion of unrelated/later/unsent text. No provider hook, second production driver or real provider is involved. The refusal fixture proves wrong Note, general-vs-Project context and explicit conversation change refusal. Its archive leg proves only no worker and saved-state preservation at offline provider selection; archived-context rejection and retry Unarchive guidance are not demonstrated. Round1 adds an exact expected selection-notice assertion and renames the current fixture, now passing with its exact expected notice (1 passed / 272 filtered). Original baseline/candidate executions retain the original fixture name `saved_retry_refuses_wrong_displayed_note_scope_and_archived_context_without_writes`; none is relabelled by the source rename.

## Actual allocated checks

All commands were serial, Rust **1.98.1**, locked/offline after the one authorized metadata resolution, CARGO_INCREMENTAL=0 and fresh synthetic TMPDIR. Candidate target `/private/tmp/brn-b1a-20261016-target` is reused only for its original bound source root; before artifacts never entered it. Initial and final dep-info inventories show no foreign checkout/before-root binding. Matching cached arm64 ORT 1.28.0 is explicitly selected by ORT_LIB_PATH, SHA-256 `d68759bf6c644dd5c6cb4e735699fba89e431b49ca588f73c11ee1c48ba838b3`; no download/stub/model assets. External model directory was absent and is explicitly removed for subsequent feature commands.

| Gate | Actual result |
|---|---|
| Offline metadata and lock inspection | Exactly desktop → already locked rig edge. All **953** package/source/version/checksum identities unchanged; no D1A memory/version change. |
| New focused native fixtures | **2 passed**, 0 failed, 271 filtered, after the retained Result compile correction. The first failed compile had an extra writing-module filter but executed no tests. |
| Maintained `bash scripts/verify-desktop-shell.sh`, exactly once | All seven commands passed: helper; native library build/strict all-target lint/**44 tests**; desktop native/test-support build/strict all-target lint/**272 passed / 1 existing ignored / 0 filtered**. Desktop execution 4.45 seconds. |
| Cargo fmt; both default normal/feature trees | Passed; **zero GPUI** in both trees. |
| Default workspace build / strict all-target Clippy | Passed, no GPUI compilation. |
| Full workspace tests, `--no-fail-fast --locked --offline`, once | **1,109 passed / 1 failed / 6 existing ignored / 0 filtered**; sole carried PDF failure below. No GPUI compilation. |
| Requested app targets within that full run | shared_run **2**, writing_context **2**, writing **5**, writing_setup **12** passed, zero failures. No separate redundant invocation. |
| Requested CLI targets within that full run | agent_runs **12**, agent_retry **7**, agent_edits **8**, agent_contract **23** passed, zero failures. |
| Combined desktop and CLI native-retrieval builds | Passed using the existing cached ORT. |
| Strict all-target feature-union Clippy | Passed with **native-ui,native-retrieval,native-test-support**, an explicit union covering the widget code too. |
| Exact combined state tests | **55 passed / 0 failed / 0 ignored** with native-ui,native-retrieval. |
| Final combined binary fresh/reopen headless | Both invocations passed over one new synthetic data directory; current threads.sqlite3 exists, retired brn.sqlite absent. No GUI. |

The full native run includes the unchanged A/B reduced-motion journeys, CW2 private disclosure/context/cache/access-loss/real typing witness, F15 real-editor Undo-after-Apply/refresh/attempted-Save, stale feedback/retry/reopen, queue/Stop/recovery and work-bound tests. The existing ignored native fixture still requires BRN_DWR_SHARED_DATA/SOURCE/CHAT. Counts across configurations overlap, and zero doctests are not counted as substantive proof.

The [machine receipt](verification.json) retains **17 command receipts**, each actual source/HEAD/diff, log hash and elapsed time, plus **96 artifact identities**, target/fingerprint/source bindings and exact prerequisite identities. Raw evidence: `/private/tmp/brn-b1b-window1-20261016/`. The receipt labelled native-shell is only `cargo --version`; the actual maintained script is native-shell-actual and ran once. No false second-script claim. The metadata receipt intentionally records the pre-resolution lock; the initial compile receipt intentionally retains the uncorrected Result source. Neither is relabelled final-source passing evidence.

## Acceptance status and limits

| Item | Status |
|---|---|
| Panel shared request; correct saved scoped retry; one runtime path | Implemented; local compiled/synthetic proof passed. |
| Wrong displayed context and explicit conversation change | Genuine baseline failure and corrected native proof; fresh app contextual reader consumed, no new predicate copied. |
| Selection/Copilot tone and scoped saved inputs | New fixture and requested app/CLI targets passed at `601e95d3`. |
| Archived saved retry at offline selection | Original archive leg preserved worker/records/receipts/request/archive state; no archive-specific retry rejection or Unarchive guidance proven. Exact notice assertion passed in the single affected native fixture (1 passed / 272 filtered). |
| Ordinary-chat archived Unarchive, stale feedback and normal chat queue/Stop/EditNote | Existing separate exact checks passed, including `archived_writing_chat_preserves_feedback_and_requires_explicit_unarchive`; this is not retry-specific archive evidence. |
| F04/F06/F07/F09/F10/F12/F15/F16/F21 | Relevant local request/display/preservation evidence recorded; broader milestone/program and native-human gates remain separate. No full F21 claim. |
| F22 | Default graphs/build/lint/test traversal exclude GPUI on this Mac. Required Linux/hosted lane and full all-green qualification remain open. |
| Independent fresh Opus / principal acceptance | Round1 **approved production at `601e95d3`**. Principal acceptance remains pending. Precision-only correction passed its affected test, strict native/test-support lint and formatting/static checks. |
| Targeted changed-guard recheck | **CAUGHT**: one seeded contextual-reader omission compiled and failed the exact context assertion, after a passing fixed-source baseline. **208.817199 / 300 seconds** including setup/recovery/restore/reap/verification; [exact receipt](guard-recheck.json). No second variant or unchanged-guard campaign. |

**Carried failure:** brn-threads-intake qualification `prepared_rust_fixtures_keep_source_and_preview_bindings_in_owned_scratch` returns `Pdf preparation failed: Unavailable`; its harness is 10 passed / 1 failed. Native pdftoppm remains absent. Existing native Poppler pdftotext/pdfimages and the maintained helper hashes are recorded. No wrapper, skip, tool substitution, retry-to-green or test change. Existing external block 0.1.6 future-compatibility warning remains unsuppressed.

Unchanged app native/retrieval suites and broad fixtures-only E2E were not restarted merely for this adapter/type re-export; their B1a evidence retains its actual source qualification. Shared external intake fixture, hosted CI, native-human, owner, full program and original widget-B causal limits remain open. No live provider, model download, owner data, GUI, publication, main merge, release, B3/B4/PC or broader framework work.

**Handoff:** after the compiled local pin and empty precise own-process receipt, release the sole heavy slot to principal promptly. Keep only the exact B1b hunks in ownership.md frozen for independent review/fixes; no further heavy work without a new grant. Final SHA/tree/crates/status and release receipt are reported after commit.

**Final LOW handoff:** correction pin `9b4f48e7` retains the three affected check receipts; the final evidence-only commit changes no crate bytes. [Final scope, release and limitations](final-handoff.md). The original 17-command receipt and original failing-before identity remain byte-unchanged.
