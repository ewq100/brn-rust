# First end-to-end workflow implementation plan

2026-09-28 · `trial/end-to-end-flow` · Base `a2fa7d5aff7c4c6bfb5875eed11915fa246a872c`.

**Goal:** Selected Markdown/text import → approve for search → index → keyword/semantic/hybrid search → inspect exact source passages → grounded streamed question → close/reopen persisted conversation.

**Architecture:** Follow the [approved architecture](architecture-checkpoint.md), connecting the independently verified storage, provider and retrieval boundaries. A workflow owner serializes authoritative storage and loaded retrieval resources; native handlers enqueue work, poll bounded/coalesced state, and cancel independently. Provider thread association and pending question/evidence commit before external turn submission. No uncertain operation is automatically replayed.

**Authorization:** The user asked to continue until an end-to-end flow exists and previously authorized routine design, implementation, delegation, tests and private trial pushes. This implements the roadmap's first useful personal-trial flow through chunk 10. Publication, comments and graph work remain later milestones. Native acceptance was deferred; a successful build or headless flow does not substitute for native interaction evidence.

## Delegation and interfaces

Astra reviews architecture and final changes. Sol implements three disjoint areas: store migration/workflow transactions, reusable provider transport, and arbitrary-document retrieval. The lead owns root manifests, `brn-workflow`, command-line integration, evidence and delivery; native wiring is delegated after interfaces stabilize. All changes use the existing isolated trial checkout on a new branch. Earlier worktrees and pushed branches are preserved.

- `brn-store::workflow`: atomic origin-keyed imports with UUID identity, immutable versions and explicit retrieval approval; current document snapshots; thread association; atomic prepared/completed chat turns and transcript projections. Migration retains existing records. Duplicate IDs bind exact command data.
- `brn-provider`: initialized authenticated owned App Server client; persistent thread start/resume separated from submission; bounded framing/queues/timeouts; turn-start persistence callback; streaming deltas, usage, terminal status and cancellation. Use existing managed sign-in without credential inspection or account mutation.
- `brn-retrieval`: reusable loaded keyword/native index, UTF-8-safe chunks, exact provenance, verified immutable generations and real FastEmbed/LanceDB semantic/hybrid adapters. Missing resources fail explicitly. The old synthetic experiment remains historical evidence, not production input.
- `brn-workflow`: selected-file import, durable index activation, authoritative evidence revalidation, provider/session coordination and request deduplication. One shared implementation serves headless and native paths.
- `brn-desktop`: real workspace actions, query/profile controls, sources and evidence, session history, streaming answer/cancellation and useful failures. UI changes are limited to this flow.

## Acceptance tasks

- [x] Import preserves originals, deduplicates unchanged content, creates a version for changed content and respects approval/current state.
- [x] Build a generation from approved current sources; cancellation/failure never activates it. Stale generations cannot grant source eligibility.
- [x] All three profiles return exact attributable passages using arbitrary synthetic imported files; missing native resources never fall back silently.
- [x] Store session association and question/evidence before provider work; save turn ID at acknowledgment; complete answer and operation atomically. Duplicate or uncertain IDs do not cause new submissions.
- [x] Deterministic fake-provider tests cover ordering, resume, errors, cancellation, stale context and restart. Actual installed App Server demonstrates the same integrated flow with synthetic fixtures and the existing subscription sign-in.
- [x] Native shell builds and exposes the shared workflow. Attempt native interaction only through supported computer-use; if the Mac remains locked, record the gate without claiming it passed.
- [x] Run formatting, build, Clippy, unit/integration/CLI checks, existing regressions, and native retrieval/UI checks. Review correctness, credential safety and evidence claims; fix material findings.
- Delivery: commit/push this completed trial, verify the remote commit, and report the resulting identifier with the demonstrated flow and remaining limitations. The delivery result is recorded in the task handoff.

## Review focus

Wrong operation reuse must not submit; source changes between search and ask must invalidate context; provider store/thread mismatch must not silently replace history; cancellation/close must remain usable during blocking work; incomplete/corrupt index generations must fail without erasing authority. Tests cover these classes at their owning boundary and through integration. Inputs are selected `.md`/`.txt` files with bounded size, not a recursive private-vault migration. Retrieval approval permits grounding only and is not publication approval.
