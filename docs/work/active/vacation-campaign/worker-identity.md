# Worker operation identity — 19 October 2026

Source checkpoint `823a154497ff1ff3f4abb05b75eebf6473d45c70` continues from PR162 head `aebfa12b`, already merged by the owner at `bd42c1d9` with the same source tree. The original campaign expiry remains **2026-11-01T21:41:51Z**. This is another partial improvement in inventory15; it does not complete Run/worker presentation ownership or qualify a release.

## Owner and interface

Previously every constructor coordinated `WorkerKind`, optional chat and optional Run ID. Run-only callers checked these values separately, and some synthetic fixtures constructed a Run worker without a persisted Run. No new production bug was reproduced in this slice.

`threads_worker.rs::Worker` now owns a private operation enum. A Run carries one `RunIdentity` with required chat and Run ID; import, export, backup and Copilot connection operations carry no Run identity. Explicit constructors hide cancellation defaults and operation construction. `run_identity()` supplies one coherent borrowed identity; `kind()` derives existing display/cancellation policy. No second durable authority or new generic adapter is introduced. IDs are still checked by Workspace/SQLite; the Rust type requires their presence, not proof that arbitrary strings name an authorized persisted pair.

Run start, Stop and Send, stream routing, archive, project/settings interruption, terminal notices and writing presentation consume the same identity. The old mutable kind/chat/Run fields and constructor coordination are retired. The existing first-terminal admission and conversion-to-successor handoff remain intact. Imports keep their captured conversion settings/scope; output workers retain non-cancellable completion.

The alternative of adding three getters over the old fields would preserve invalid combinations. A whole Run/presentation reducer would pull scattered preview/context state into this bounded transport change. The chosen enum removes those combinations while leaving checked Run/queue/lease decisions in the existing service and GPUI mechanics in the toolkit.

## Behavior evidence

The existing production-poller stream/navigation/archive journey now starts a real synthetic Working Run and holds its lease. It views and resolves chat B while A streams, then resolves A; assertions check cancellation, chat ownership and durable partial assistant output in A with no output in B. Run controls likewise use a real persisted Run and lease. The MCP immediate-busy fixture uses a persisted Run without a lease; it does not exercise lease-sensitive transitions. The Omakase terminal fixture refers to its existing persisted Run. A conversion fixture no longer mislabels its non-Run worker with a chat. Distinct tests and safety assertions are retained.

Independent Sol [source review](evidence/worker-identity-review.json) found no actionable findings, including the MCP fixture correction described below. Final desktop source diff SHA256 is `e346235c98c26e9ce1922bfbf92ba6f32d4bd42dcc2c674567cab992ea9e2f88` against `aebfa12b`. It checked responsibility placement, caller complexity, terminal/disconnect and conversion successor behavior, fixture semantics and retirement; it performed no builds or native interaction. Focused navigation/archive verification passed1/0 on the first build. Final desktop315/0/1, explicit offline CLI/desktop1/0, strict all-target lint, production native-ui build and formatting passed. [Exact commands, exits and log hashes](evidence/worker-identity-qualification.json) distinguish earlier fixture failures and final source. No provider access, native OS interaction, release acceptance or hosted verification of this new slice is claimed at this stage.

The first full run passed314/failed1/ignored1: the migrated MCP busy fixture created its persisted Run after input review, correctly invalidating the exact review stamp. The corrected fixture establishes the Run and refreshes before capturing the review; the focused case passed1/0. Independent review confirmed all busy, pending, unchanged-record/history and final review-consumption assertions remain intact. The original failed run is retained. Production code did not change for this correction.

The explicit CLI/desktop case used a fresh purpose=test fixture and the identified unchanged CLI binary SHA256 `492a6ef0547225938eac78190744002aae8d968771e0a80be651ade17978400b`; the CLI source was unchanged by this slice. Initial setup assertions rejected normal progress stderr, then a `/tmp` auth path correctly produced UnsafeCredentials. The final explicitly absent `/var/tmp` auth path produced the expected ReconnectNeeded; desktop reused the same chat/instruction and added one Run. [CLI receipt](evidence/worker-identity-cli-fixture.json) and [fixture script](evidence/worker-identity-cli-fixture.py) retain the successful setup separately from both original setup failures. This is offline service/toolkit evidence, not a live provider or OS-input witness.

An additional default-feature policy run was stopped during dependency compilation, exit130 with no tests executed. Its partial log is retained and is not counted as passing evidence; the requested native checks above were complete. No build/test jobs remain.

## Remaining ownership

Stream/progress, activity presentation, context-edit state and reference-import state remain in their current UI owners. Revisit them when reconciling remaining Run/worker transitions and OS input routes. Batch admission and typed identity are implemented increments, not full inventory15 qualification. The [workflow denominator](workflows.md) remains25 provisional domains and0 fully qualified against the complete campaign standard.
