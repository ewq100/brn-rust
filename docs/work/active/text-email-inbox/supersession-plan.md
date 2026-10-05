# Inbox supersession consequence — Mac mini

## Baseline and authorization

2026-10-05: owner resumed sequential V1 development on Mac mini, permitting
reviewed milestone commits/push/PR/normal merge after relevant local checks,
exact latest-head macOS/shared CI and GitHub requirements. Production baseline
`ceb3d0f9e1e958915bfcb5e6b5e169a5571d13bc`; documentation-only handoff
`74725adf74fca2f3abf231c0712b19d358740b6c`. New isolated checkout at that handoff
on `codex/v1-stage7-history`. Primary checkout remains main/18f3891 with unrelated
untracked Finder files; two older feature worktrees and separate trial checkout
remain untouched. Native worktree creation was unavailable in this projectless
chat, so Git created a task-owned checkout under this chat's work directory.

Frozen vision §§7–9/23, architecture/invariants and development workflow govern.
No architecture reset, new datastore/lifecycle, direct AI write, service or MCP.
No private data, live calls, model assets, purchases or release. Development
helpers use Sol/Luna only; actual provider qualification needs fresh permission.

## Coherent outcome

Extend the opt-in Inbox knowledge tool with a named current predecessor.
One exact proposal creates a new Current note with a distinct UUID and marks the
predecessor History at its existing path. Preserve the predecessor's identity,
body, provenance and unrelated metadata; change only managed brn_state.
The original selected Inbox Source remains immutable saved evidence. Other
knowledge and Actions remain independent proposals in the same existing group.
Unresolved semantic conflict capture follows as a separate deliverable before
qualified original-copy deletion; this slice never chooses a conflict winner.

## Contracts and implementation

- Optional supersedes path on KnowledgeProposalArgs; omitted legacy input
  preserves Create-only behavior. Explicit full predecessor proof follows the
  selected Source, then existing ordered additional paths. No caller History text.
- Optional immutable predecessor binding in InboxKnowledgeBinding; omitted fields
  remain canonically omitted to preserve historical hashes. Store checks exact
  Create/Replace shape and byte-preserving historical output against before_text.
  Editing/Rewrite may alter the new Current content while preserving identity,
  Source citations and predecessor relationship; cannot alter the History member.
- Workflow qualifies Current knowledge and unique predecessor UUID from complete
  inventory, full saved bytes and proof. Creation replay uses retained before_text
  and original proofs before fresh files, preserving later review.
- Approval rechecks all authority; applying/recovery/Finish permit only the exact
  predecessor baseline or this operation's exact prepared History object.
  Existing whole-proposal staging, receipts, Restore, Undo and fences remain.
- Reuse existing native proposal review and shared CLI/AppWorker boundary.

## Acceptance and checks

1. Actual synthetic callback creates one two-member draft without vault effects;
   exact approval yields new Current and previous History. Selected Source/original
   stay exact. Explicit relationship and history survive index loss/restart.
2. Identical replay preserves proofs/newer review after source/predecessor loss;
   changed UUID/path/order/input refuses. Legacy single-Create replay still works.
3. Edit/Rewrite cannot change History wording, identity, state or immutable proof.
4. Changed/aliased/ambiguous predecessor, dirty editor, stale Source or new identity
   collision refuses before success; uncertain partial application stays fenced.
5. Genuine process interruption, restart, Finish/Restore and Undo preserve whole
   consequence and exact originals; settled replay preserves newer user edits.
6. Independent read-only complete review, validated dispositions, focused Store,
   workflow and AI synthetic-route tests; shared/native checks selected from the
   verification guide; doc links/diff; exact-head hosted gates and merged witnesses.

## Machine qualification and checkpoint

Darwin arm64/macOS 27.0.1 (26A434), Command Line Tools, Rust 1.98.1,
protobuf 36.2, Python 3.14.6. Synthetic TMPDIR is existing canonical owner-only
`/private/tmp/brn-mini-synthetic-20261005`, outside Git. Initial offline build
refused uncached Rig; cargo +1.98.1 fetch --locked populated ordinary dependencies
without lockfile changes or model requests. Before implementation, fresh baseline
fmt/locked offline build/all-target Clippy,1269 shared tests/0failures/8ignores,
shipping native build/all3 native Clippy variants,282Desktop tests/0/0 and startup
passed. All52fixtures and two shipping V14 AppWorker restarts passed, with zero
credential files and no legacy database. GUI/live/real-assets qualification stays
pending. The active implementation has focused passing paired-approval/history,
rebuild/Undo, stale/ambiguous/dirty-editor/hidden-link refusal and genuine process
interruption Finish/Restore tests. Final local candidate verification and independent review passed as below;
exact-head hosted gates and integration remain pending.

## Independent review disposition

A fresh read-only Sol review compared the entire candidate against the
74725ad handoff. One valid defect was reproduced through the real worker:
an earlier visible predecessor link allowed an exact footer hidden by an
unclosed code fence to pass identity-set qualification. The correction checks
the inline Markdown node's exact footer byte span at preparation, approval and
apply/Finish. Preparation and edited-proposal regressions pass. Full re-review
is clean against all19Rust files, manifest SHA256
`264fdf391b9eadda206c3bfe9fc5da9d76bb980361646bfd58ada55ccb6642be`.
The reviewer verified the manifest, complete initial diff and correction,
with no other concrete safety/correctness blockers. No code was changed by review.

## Final local candidate evidence

2026-10-05, handoff HEAD plus the reviewed candidate, Rust1.98.1,
locked/offline Cargo, the synthetic TMPDIR above and BRN_NATIVE_MODEL_DIR empty:

- `bash scripts/verify-desktop-shell.sh --native`: format, workspace build and
  all-target Clippy;1282shared tests/0failures/8documented ignores; shipping combined
  native build, all3native Clippy variants;282Desktop tests/0/0; real startup passed.
- `cargo +1.98.1 test -p brn-workflow --features native-retrieval --lib --test models --locked --offline`:
  259/0/7. Real-assets/GUI tests remain unqualified; ignored process-child harnesses
  are exercised by their parent recovery tests.
- `cargo +1.98.1 test -p brn-ai --features capability-spike --all-targets --locked --offline`:
  106/0/0, using synthetic transports, plus the same feature's all-target Clippy.
- `bash scripts/verify-end-to-end.sh --fixtures-only`:52assertions.
- Shipping combined-native binary started/shut down twice against one fresh
  disposable synthetic data directory: V14, current database present, no legacy
  database or credentials. No GUI or live account/model operation occurred.
- `git diff --check` and all73changed local Markdown links/fragments passed.

The pre-correction full gate also passed, but final evidence above is the fresh
post-correction gate. Review regressions and complete tests qualify the final Rust
manifest recorded above. Hosted and merged witnesses remain required.

Reproducible synthetic acceptance: run
`cargo +1.98.1 test -p brn-workflow --lib supersession --locked --offline`
with the pinned environment above. The real owned-worker tests admit one paired
draft, edit only Current content, approve exact Current/History effects, inspect
scoped relationships after index loss, recover each interrupted member and Undo
exact retained predecessor bytes. Native owner review/live provider compliance
remains pending and requires separately authorized conditions.
