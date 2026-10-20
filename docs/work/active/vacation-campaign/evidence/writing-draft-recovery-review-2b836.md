# Independent final draft-recovery review

Disposition: PASS within the inspected scope. No remaining actionable finding. The P2 from the complete574 correction review is fixed. This is a source/record review, not merge authorization or native acceptance.

## Exact identities and scope

- Worker checkout: `/Users/evokessler/.codex/worktrees/export-durability/brn-rust`, clean HEAD `2b8361480261eda65818b90cb50508c047a62031`, direct parent `5744d102573134474c38ae1352ea1abfccb53567`.
- Entire final delta changes only `crates/brn-desktop/src/threads_notice.rs`: the dismissal branch and one regression. Actual `git diff --binary 5744d102 2b836148` SHA-256 is `46deb8fa3d5f3ad37e42050d6c579147bd3dafb5c7028ed9426898ffef7af8c2`. The earlier author-message digest suffix was incorrect; the lead wrapper records the actual recomputed digest and preserves the correction. This is a reporting correction, not a source/check failure.
- Reused prior independent complete690 implementation and574 correction reviews, retained byte-identically as `writing-draft-recovery-review-690.md` and `writing-draft-recovery-review-574.md`. Core Store source remains identical690 through final; no redundant Store re-audit or build was performed.
- Complete lead documentation delta inspected: `63345a8ea55021009a636106b4249efb7fa966f4..1d75088f212aef9028e33383406db779fbf65c21` in `/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust`, 31 documentation/evidence/archive paths; no application, dependency, configuration or CI source changes. Separately inspected the subsequently added untracked final qualification wrapper and archive, not falsely included in that commit pin.
- Final wrapper SHA-256: `038cf52f65867cc6431a40492ca9f7dc4b62e5ba5890b109da7d6fc0d94a5689`; final archive SHA-256: `a14400104bfbada82f8d667ab7aabf9d3632a0718e9a45e4c9874fab8099f199`.

## Prior P2 disposition and correctness

`threads_notice.rs:89–101` now removes only the displayed scoped recovery entry and preserves an underlying `Presentation::Ordinary` for its own acknowledgement. A subsequent ordinary-only Dismiss still clears it. The existing shell callback uses this method, so the production control reaches the corrected branch.

The whole NoticeState and actual callers were checked: background input/start clearing, worker completion, same-Note successful handoff, shell provenance/restoration and explicit Dismiss. Per-Note recovery still takes display priority, blocks work-status restoration while visible and survives background updates; acknowledging/saving one Note does not remove another Note's attention. Preserving an ordinary notice does not turn it into remembered work. Remembered-work dismissal/restoration and normal ordinary behavior retain their previous semantics. No additional general notice redesign is introduced.

The added test checks both visible messages, error severity, lack of work provenance and separate acknowledgements. Preserved attempt02 runs exactly one compiled test against574 plus its test-only patch, exits101 and fails at the intended assertion: the hidden ordinary error becomes empty after recovery Dismiss. Attempt01 selected zero tests and exited0; it is retained and explicitly invalidated, not counted as behavioral red or qualification. The prior690 actual worker-completion red proof and original draft-loss red proofs remain historical evidence, not rewritten as passes.

## Evidence and documentation assessment

Both prior candidate archives passed compressed-hash and every member hash/length binding (44 members for690 and36 for574). All18 final archive bindings also passed, as did the six embedded original command-record and raw-log bindings. Final original source-delta hash was independently recomputed. The final immutable-artifact manifest hash matches its wrapper; no app was launched.

Current campaign/overview distinguish the pending final correction/review from the dated690/574 local results and review findings. Older publication/progress snapshots retain their observation times and original pending states. The final wrapper explicitly says independent review was pending at receipt creation; this report supplies its subsequent disposition. Current source/gate metadata should be reconciled when composing, as already planned.

PR183 receipts identify publication head `1e6abb60fa4a94c6367fa1e983db379c4ef5bcd4`, preceding main `bc8a08e64b0dc1e2b33ff181dfdb54b62f59f5e4`, strict protected contexts and all four required head/preceding-main successes before normal merge `63345a8ea55021009a636106b4249efb7fa966f4`. Resulting merge tree was independently compared and equals publication-head tree (`df90458a0ca79201b3a879aaea86eeaadafddac1`). Receipt run39421007719 is explicitly pending at that resulting main; pre-merge success is not substituted for it. Optional Windows failures remain recorded. These CI receipts do not qualify the new draft source. All requested edits in the three saved remote-update receipts occur in their stored readbacks; no remote action was performed by this reviewer.

MAC-10 fixture bytes/hash/terminal-LF match the manifest. Queue and read-only checker specify a fresh synthetic Note, actual first-comment handoff, source tombstone, exact target draft and generations, unchanged canonical Note/sentinel, and visible recovery after actual OS close/reopen. They explicitly describe prepared expectations, not observations. Historical690/574 binaries are not represented as final accepted apps; no native usability, owner acceptance, crash/power-loss recovery or live account test is claimed. The prepared publication description matches the implemented behavior and preserves hosted/native/owner gates.

## Checks and limits

No reviewer build, test execution, target mutation, Git mutation, network/API/account/native action or delegation. Read-only Git/source/record inspection, hash/length checks and whitespace check only.

Saved author receipts now exist for exact clean final2b836 (empty dirty-patch SHA): notice18 passed; full desktop340 passed/0 failed/1 existing ignored; strict all-target Clippy exit0; ordinary native-ui build exit0. Original logs corroborate those outcomes and are hash-bound in the final archive. This reports saved evidence, not independent reruns. Format/diff PASS is retained in qualification; the final source delta whitespace check also passed here. Unchanged core storage11 is reused from690, not claimed rerun on2b836. Existing ignored external intake fixture and block0.1.6 advisory remain disclosed.

This bounded review does not establish hosted checks, publication/integration of the draft repair, actual Mac interaction, owner acceptance or release. Any later source change requires a fresh scope assessment; a later composed documentation/source commit still needs its exact identity and normal gates recorded.
