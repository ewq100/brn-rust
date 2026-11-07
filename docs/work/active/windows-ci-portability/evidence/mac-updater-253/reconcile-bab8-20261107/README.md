# Reconcile main salvage without moving qualified artifacts

PR268 head29af383b had clean completed current code/security receipts and no review
threads, but no CI run: newer mainBab8 made its shared task record conflict.
Normal reconciliation preserves B's latest task and every other track object from
main, then regenerates the overview. Runtime files merge without conflict.
[Exact identities, inspected delta and limits](reconciliation.json).

The default-build delta changes usage completeness and safe read_message trace
metadata; summary usage no longer presents missing counters as complete. The new
judge/example is capability-spike gated. Grading/evidence tooling, tests/docs and
one CI step are also carried from accepted main. Inspection indicates no change
to provider/model selection, model payloads, permissions/dispatch, checked writes,
Undo, helper/launcher/updater or packaging scripts/version manifests/locks. This is a scoped inference, not
new native evidence. Fresh integrated-head checks/reviews remain required.

Existing82657packages, source-bound native receipts and running idle test app are
unchanged. No desktop rebuild or artifact relabel occurred. Native normal
quit/reopen/closure still needs owner unlock; the screen remains locked. E reports
measurement batches finished06:54UTC and no current local reservation. PR274 is
separate and not adopted here. No publication or owner acceptance is granted.

Local offline validation:94Python tests total,83passed and11explicit fixture/binary
skips; original log retained. Generated overview, Markdown links and whitespace
checks pass. Rust/combined native-state verification remains in fresh hosted CI;
this local Python result is not native or live-provider requalification.
