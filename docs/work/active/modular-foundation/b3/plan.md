# B3 persistent reference-assisted writing implementation plan

**Principal disposition, 17 October:** final source93b92d7b is independently APPROVED by Opus3 and [accepted for local dependent development](principal-disposition.md) after cleanfc0ce49b handoff. B4 is next for source-only preparation while D2 owns heavy. Original source/test/qualification identities below remain historical; no full INT-W/hosted/native-human/owner/PDF acceptance is implied.

**Goal:** expose the existing scoped brief/reference configuration and immutable host-recorded Sources used in the reusable writing panel.

**Identity:** provider OpenAI/Codex, model gpt-6.1-sol (Sol6.1), task B3; bound checkout `/Users/evokessler/.t3/worktrees/brn-rust/t3-239c29dc`, branch `work/b3-20261017`, exact clean base `f2b060bb17080d0241143afb6fadd6ec22350f34`. Local origin/main `8f768943964a77711b26d5235df146f8efe1dc6c` inspected without fetch. B1a/B1b refs and all other worktrees preserved. [B1b acceptance](../b1b/principal-disposition.md) permits local dependent progress, not hosted/native-human/owner/program acceptance.

**Spec:** [master §7.1/7.2/§9 B3/F registry](../proposal/final/01-master-plan.md), [lead prompt](../proposal/final/02-foundation-lead-prompt.md), [Stage2](../../build-sequence/README.md), current explicit B3 allocation and canonical architecture/invariants. D1A/D1B are not integrated here.

**Architecture:** one optional brief text supports purpose/audience/tone/requirements; ordered WritingReference links retain canonical Notes with existing WritingPurpose metadata. Library UI borrows the actual Workspace for checked context/setup reads/writes. Desktop keeps the existing scoped search worker, native picker, full-note converter/import, and worker dispatch. Sources used renders only the existing checked WritingCandidateReview, never model bibliography or a second evidence parser.

## Authority and review focus

SOURCE/TEST/DOC PREPARATION ONLY; D1B owns the sole heavy slot. No Cargo/metadata/build/test/Clippy/mutant/list/stress/headless/process polling, delegation, manifests/lock/CI or other checkout edits. Direct pinned rustfmt/static checks allowed. No B4/PC/editor cutover, live/provider/auth/owner data/download/GUI/publication. No B1b mutation budget is reused.

- Same shared Note across scopes: stale drafts/search replies must reveal no brief, reference names/counts/metadata, quotes or claims after context/access change.
- Concurrent setup edits: checked expected versions/replay; failed save must retain the owner's draft without silently replacing newer setup or changing Note version.
- Async import completion: immutable target Note/scope/Note version/setup version; preserve a legitimately imported Note, refuse wrong-target attachment, never use global Chat as fallback.
- Read coverage: attached/available is distinct from host-recorded use; import coverage, exact retained bytes and model claims/verified quote provenance remain separate.
- Existing writing paths: sole B1b shared start, first dirty guard, selected conversation, CW2 cache/disclosure/early review/Compare refusal, v1/Apply/Undo/queue/Stop remain intact.

## Source preparation sequence

- [x] Verify clean accepted base and create only the allocated new branch.
- [x] Inspect CW1 setup/availability/versioned owner API, CW2 checked review/context, scoped paged search and existing full-note importer.
- [x] Prepare `native::setup` for concrete context-bound setup draft/target and library drawer; register under the existing native gate without root feature/dependency changes.
- [x] Prepare `native::references` rendering for host review evidence and current checked attachment status; keep mandatory private disclosure outside Inputs.
- [x] Prepare only concrete desktop setup/save/search/reference/import-intent/completion adapters and panel view binding. Reuse existing search tickets and import path; no conversion/runtime duplicate.
- [x] Append meaningful native/real-Workspace journeys; preserve every pre-B3 test byte. Prepare before proof only for actual missing behavior, not absent-API compile failures.
- [x] Direct pinned rustfmt, static ownership/prefix/manifest checks, source fingerprint and [BUILD NEED](source-preparation.md); stop before heavy execution.

## Acceptance prepared for a later grant

F01/F02/F03: full-note import/attach, replay, cancellation and wrong-target completion with honest retained-result status. F04/F06/F07: scoped brief/reference/purpose persistence through conversation changes, navigation, closed Chat/restart and exact snapshot capture. F08: separate retained-read/import/claim/quote presentation; full-reading vs partial requirements state from host evidence. F09/F10/F14/F21: stale setup/access loss, scope isolation and notify-only invalidation; retain original CW2 privacy and real typing tests. F12/F15/F16/F19: existing exact review/Apply/Undo/editor/drafts/queue retained. F22: unchanged default feature graph; default/native tests/lint/build later. F20/native file picker/IME and hosted/full F21/owner gates remain open.

Later commands, only after explicit allocation: affected desktop native tests; library native tests and strict all-target native/test-support lint; maintained native suite; default graph/build/lint/tests and applicable app writing_setup/writing_context/shared_run integration. Principal supplies exact serial budget. New UI context/import guards may warrant a separately granted original B3 ≤1800s targeted slice after actual before/normal proof and source pin; no automatic campaign.

**Additional exact app allocation:** observation-only re-import receipts omit the canonical Note. Principal allocated only `Workspace::full_import_note_in_scope` in app intake.rs and six appended import_coverage scenarios. The helper validates the stored applied receipt, current scope/visibility and exact host-observed SHA relation, requires exactly one eligible Note, and does not infer old X from Source.external_version after newer Y. No importer/dedup/version policy change. The desktop consumes this service and preserves successful imports on unavailable attachment; it performs no raw-record resolution.

## Granted verification window

The principal explicitly released D1B and granted the sole heavy slot to B3 for serial locked/offline before, focused service/native, strict lint/format/default/native/combined and synthetic scripted acceptance gates. No mutation is granted. Candidate target `/private/tmp/brn-b3-20261017-target` belongs only to the bound checkout; before root `/private/tmp/brn-b3-before-20261017-source` and target `/private/tmp/brn-b3-before-20261017-target` are distinct. Actual command/source/log associations and failures will be retained in the verification receipt.

## Window disposition

Production pin `29c9cf68` and the test-only `c1eeeb19`/compact-window supplements are locally compiled/verified as described in [actual evidence](verification.md). The heavy slot is released; no mutation has run. Exact source ownership stays frozen for independent principal-owned Opus review and explicit handoff. Remaining acceptance is review/principal, hosted/native-human/IME/picker/owner/full F21/program and carried PDF tooling; this is not B3 principal acceptance or a B4 allocation.
