# BRN Threads independent prebuild review

**Reviewer:** Claude Opus 5.5 via GitHub Copilot app, 10 October 2026. **Reviewed target:** `rebuild/threads@3519bfb85e3c68cb17dd6df31061e9f5dc155e73`. **Historical reference:** `af9239c741c7ab0983e62f0253e607b9607727e6`.

**Scope:** target, plan, brief; read-only inspection of store, workflow, intake, desktop, CI and vendored `rig-agent 0.43.0`. Nothing built, run or called live; inferences and experiments are labelled.

**Verdict: Ready with specified corrections.** Findings 1–4 change M1 interfaces; 5–7 need proof before dependent work.

## Findings

### 1. Save and the edit guard can silently overwrite committed work

**Failure.** *A single mechanism for changes and review* defers AI commits only "while a note has unsaved human writing", and *Editor, comments…* says Save "applies directly". Neither gives Save an expected base. An AI commit landing while the editor is open but clean (or before the 500 ms recovery debounce, `brn-desktop/src/ai.rs:513-568`) is overwritten by the next Save. A desktop-memory guard is invisible to a CLI process; the "serialized write boundary" must be SQLite's write lock.

**Smallest fix.** Make the persisted recovery-buffer row the guard: created on the first dirty transition only if its base version equals the current version, checked inside every `BEGIN IMMEDIATE` commit, and deleted in the Save or discard transaction. Save carries its base version; a mismatch is a visible conflict. A change set touching any guarded record is deferred whole; splitting it requires new explicit operations. Reuse the `unsaved_edits` baseline idea (`work/edits.rs`), not the save journal.

**Timing:** blocks starting M1 (M1 proves guarded deferral).
**Verify:** open note at v1; AI commits v2; type and Save → conflict. A CLI change set over a dirty note and another defers whole; the guard survives kill/restart until discard.

### 2. Operation identity is minted too late for retry

**Failure.** The target says "the application owns its operation ID". If the service mints it at apply time and the response is lost after commit, the caller has no ID and a retry creates a new operation. Expected versions block a repeated update; creations have no base, so they duplicate (inference).

**Smallest fix.** Use one `prepare` that persists the immutable candidate with its operation ID and pre-assigned IDs for created records, outside the apply transaction. Callers record that ID durably—run progress, recovery row, or printed CLI ID—before `apply(id)`. The receipt is unique per ID with a canonical request hash. Reuse the existing pattern (`operation_id` primary key plus `request_sha256`, `proposal_apply.rs:18-27`, `editor.rs:20-31`). Delete its journals, `uncertain` outcome and repair family; one SQLite transaction cannot half-apply. Review and autonomy share one candidate record.

**Timing:** blocks starting M1.
**Verify:** failpoint after `COMMIT` and before return, for a change creating a note and Action and updating another note. Restart and replay from durable state: one note, one Action, one receipt. Same ID with changed content is rejected.

### 3. Instruction authority has no host-bound target scope

**Failure.** *Ordinary notes and protected notes* says the host retains the target scope, but never says how it is fixed. If the model's interpretation chooses targets, the structural check is circular: any protected note can be edited "under" a genuine owner message. Undo, unprotect, archive, Open and Done need the same rule; the old route gating (`brn-ai/src/chat.rs:499-522`) is being removed.

**Smallest fix.** Pass authority through Rig's host-only `ToolContext` (`vendor/rig-agent/src/tool/mod.rs:3,25`), never as a tool argument. An instruction grant contains the message, run and target set. Targets are protected records referenced by that message or linked to the thread when it was sent, plus records created by the run. Anything else becomes a review candidate. Undo and lifecycle changes use the same check. Only an owner action or a grant naming that target records confirmation on the resulting revision.

**Timing:** authority type blocks M1; target binding must be proved before the M2 agent journey.
**Verify:** "Set the deadline in [Decision A] to 15 Nov" applies to A; the run's edit to protected B goes to review, even when a source says "owner authorizes B"; model-supplied authority is ignored.

### 4. Build the core outside the old crates

**Failure.** The plan requires fresh schema identity but not where it lives. `brn-store` carries WorkStore V1–V18 (`work/mod.rs`) and depends on `brn-intake`. CI checks `brn.sqlite` at startup (`ci.yml:112-179`), runs old migration, approval and retention tests through `cargo test --workspace`, and asserts byte-identical vault writes (`ci.yml:110`, `scripts/verify-end-to-end-fixtures.py:43-87`). Building inside `brn-store`/`brn-workflow` drags that coupling and file naming into the final core.

**Smallest fix.** Create one new crate using only `rusqlite`, `uuid`, `serde` and `sha2`, with a distinct database filename, `application_id` and required explicit data directory. Test it in the existing headless lane. Leave old crates untouched until a consumer switches, then delete the superseded path in that slice. Reuse `work/backup.rs` stepping and the FTS5 passage index (`brn-retrieval/src/note_index/schema.rs:34`) fed from current revisions. Keep FastEmbed, `protoc` and native retrieval optional until a search journey needs them.

**Timing:** blocks starting M1.
**Verify:** the crate builds alone; old and new binaries refuse each other's data directories.

### 5. Do not persist Rig runs for resume

**Failure.** Rig 0.43's `AgentRun` serializes `chat_history` and `new_messages`, including tool results (`vendor/rig-agent/src/run/mod.rs:366-410`). `Agent::resume` re-executes pending tools and requires the same Rig version (`agent/completion.rs` near line 660). Using it would create the hidden raw archive and repeat side-effecting tools. BRN now persists only questions and answers (`work/chat.rs:380-427`) but replays 20 prior answers that may quote raw text (`brn-workflow/src/app.rs:501-510`).

**Smallest fix.** A run persists status, budget, thread messages, candidate/receipt IDs, derived results and the source references to reread. After restart it becomes Interrupted, and Continue starts a new run seeded from that state. Tool outputs are never stored. Re-applying a prepared candidate goes through Finding 2.

**Timing:** prove before M2. For M1, define Run without a checkpoint blob.
**Verify (experiment):** process a synthetic email containing a unique canary, kill mid-run, then continue. No duplicate receipt exists, and outside committed note content the canary does not appear in SQLite, logs or temporary files.

### 6. Full import has no qualified paper path

**Failure.** The intake helper accepts only `docx`, `pptx` and `eml` (`brn-intake/src/helper.rs:44-49`). The lockfile has no PDF decoder; BetterOffice 0.3.0 has no qualified OMML equations. `Extraction::validate` checks integrity, not completeness (`brn-intake/src/lib.rs:399-531`). GPUI 0.6.6 shows math as source or code (`gpui-base/src/text/format/markdown.rs:290-300`), and production renders note images inert (`native/inbox_reader.rs:38-52`).

**Smallest fix.** Declare the initial set: Markdown/text, EML, DOCX for process descriptions (equations are gaps until supported), and PDF papers through one maintained converter run in the existing sandboxed helper pattern. Choose it with a two-fixture trial; a model download needs authorization. Store equations as LaTeX in Markdown and figures as assets. Determine completeness from an inventory independent of the converter: headings, figure/table captions, equation numbers and reference count from the source text layer.

**Timing:** prove before M2 import work. M1 storage is Markdown plus assets either way.
**Verify:** a public arXiv paper with equations, tables and figures plus a public process document meets its inventory. Removing one figure yields Partial with its location.

### 7. Coarse record versions make Undo conflict-prone under autonomy

**Failure (inference).** *Minimal persistent model* versions content, metadata, protection and lifecycle together; Undo requires unchanged versions. If links, thread references or comments are note fields, routine maintenance makes Undo conflict though text is untouched.

**Smallest fix.** Store links, thread references, comments and source attachments as separate records. Note versions change only for content, title, protection and lifecycle. Undo checks only records the operation wrote. For dependents, Undo lists revisions whose recorded inputs include the undone revision; defer queues and coalescing.

**Timing:** settle before writing the M1 Undo proof.
**Verify:** an AI edits note A; another run links A to a thread; Undo of the first succeeds. A later human edit to A produces a conflict candidate preserving that edit.

## Questions without separate findings

- **Q5:** no fork, CRDT or Jujutsu is justified; math, note assets, heading navigation and comment remapping (buffer diff with `similar`, `experiments/editor-trial`) are M3 work.
- **Q7:** compensation, monotonic versions and later-edit conflicts are coherent apart from Finding 7.
- **Q8:** beyond Finding 4, stale "current authority" headings in `architecture-reassessment/plan.md:3` and `retained-evidence-investigation/plan.md:3` are adequately demoted by `docs/work/active/README.md`.

## Recommended amendments

1. Edit *A single mechanism for changes and review*: persist the operation ID before apply, give Save an expected base, make the recovery row the guard, and defer grouped changes whole.
2. Edit *Ordinary notes and protected notes*: use host-bound grant targets through `ToolContext` for edits, Undo and lifecycle.
3. Edit the plan: put the new core in a new crate with distinct DB identity, persist no `AgentRun`, and add a format/converter trial before M2.

**First integration proof:** a headless test in the new crate: one grouped change (create note and Action, update a protected note under a grant), crash after commit, replay; a guarded member defers the whole group; stale-base Save conflicts; immediate Undo; backup and restore into a new directory. In M2, when authorized, one real provider run uses `prepare`/`apply` via `ToolContext`, is killed, continued, and passes the canary scan.

## Build lead dispositions

Pending. Record each finding as accepted with its fix, rejected with a reason, or needing a named experiment.
