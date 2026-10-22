# MAC-13 preparation runner correction handoff

The candidate remains task-only and inert unless explicitly invoked with `--run`. This correction packet addresses the five P2 findings from `runner-review.md` and its P3 projection/history precision advisory. It changes only `prepare.py`; no runner, CLI, workspace, build, fixture, test, account, native, provider, network, source, or shared-record action was performed.

**Corrected candidate SHA-256:** `ae66bc3b9f289862c4168b7818ede962ee6cf23840922dfc764f359ab97f1dc7`

The original candidate identity `eb837f368f755e9afcf23c8a8194c2aa8afeda3d88ab875ffc45c5004297bb69` and review remain preserved. The earlier `runner-implementation.md` is unchanged; this report supersedes it for the corrected candidate.

Corrections:

- The fixed child environment is initialized before its variable names are recorded in the manifest.
- `init` now requires `ready == true`, the returned data path to equal the intended fresh path, and a typed nested `settings` object with explicitly present null credential sources and explicit Xberg/AI-off conversion settings before any intake.
- Owned process-group cleanup runs for every created CLI child, even when the leader has already exited. Cleanup records TERM/KILL/reap/probe errors, treats unknown group state as possibly alive, makes an independent final KILL/reap/absence attempt, and fails closed if absence cannot be established.
- Receipt finalization keeps failure status on exceptions or secret-cleanup failures. Exit zero is possible only when preparation completed, no failure was recorded, and the final receipt was persisted. Atomic receipt temporaries are unique; a failed final write changes the outcome to failed and attempts a bounded-status fallback receipt.
- Read-only Git checks use a task-local owned-group runner with five-second/overall-deadline bounds, a sanitized noninteractive environment, no pager/optional locks, bounded output detection, cleanup metadata, and no success on unresolved cleanup. No `subprocess.run` calls remain.
- Fresh fixture checks require the exact expected general Notes ID set. Rehearsal history must retain every prior receipt and contain exactly the one new checked Save receipt.

The existing four-root plan, synthetic inputs, source/CLI/helper identity checks, no-account/no-provider boundary, token handling, and native-pending status are otherwise retained. No automatic retry or prepared claim follows a receipt-write failure.

Validation actually performed: Python AST parse of `prepare.py`; targeted source inspection and search confirming no `subprocess.run` remains; SHA-256 calculation. No runner invocation, CLI command, workspace creation, test/build, process signal, or native observation was performed. Independent review is still required before any execution. The reviewed build receipt/artifact identities were not rebuilt or changed by this correction.
