# Knowledge foundations — roadmap Stage 5

Baseline: `main@15d4608db7e1b9308a2870b50b8ad0e73f4f23cf`, 2026-10-04.
The owner’s AGENTS.md change is preserved. Stage 4 is implemented, automated
verified and locally integrated; GUI/live/owner acceptance remains separate.
The inspected local commit has no published CI run. Continue under the v1 mission.

Stage 5 requires stable identities, durable provenance, explicit current/source/
history retrieval, evidence-backed relationships, basic findings/timestamps and
qualified English/Estonian retrieval. The existing index is path-based; sources
have exact file proofs but no durable semantic provenance. Archive paths currently
refuse even explicit reading. No original vault/data inspection is authorized.

## First slice: managed note identity and full review protection

Use the ordinary top-level frontmatter scalar `brn_id: <nonnil UUID>`. A narrow
parser supports this documented field, complete LF/CRLF frontmatter, optional
BOM and quoted UUID scalars; unrelated metadata/body bytes remain opaque and
unchanged. Malformed, duplicate or unsupported managed identity syntax is reported,
never replaced by a path/hash identity. Unmanaged notes remain readable.
A pure insertion helper emits complete proposed bytes while preserving all existing
bytes and their line endings; no refresh, apply or startup mints an ID.

Keep pure format/proposal identity validation in brn-store, within the existing
typed review boundary, and expose it through brn-workflow. Store edit/Rewrite
validation preserves an already-recognized proposed ID against accidental removal
or replacement. Ordinary workflow creation preserves an existing known target ID.
Explicit Undo/Trash restoration must still return exact original bytes, including
removing an identity assignment; historical receipts are not reinterpreted under
new metadata validation. Manual Save remains a direct exact-byte command.

Add a read-only AppWorker/headless preparation command that captures the current
complete source and returns a full ordinary Replace DraftRequest with caller-
chosen note/proposal UUIDs. The user reviews/creates/approves those exact bytes
using Proposal Core. Preparation does not create review records or write files;
creation rechecks source/before CAS. Add CLI inspection/preparation and meaningful
parser, Store edit/Rewrite, worker/install/Undo/restart and process checks.

Acceptance: an explicit synthetic assignment leaves vault bytes unchanged until
approval; approved ID survives restart and exact Undo restores the original bytes.
BOM/CRLF/Unicode/unrelated frontmatter remain exact. Malformed/duplicate identities
never get guessed or silently repaired. Later editing/Rewrite cannot drop or swap
an established ID; fresh valid metadata assignment is possible for unmanaged
notes. Full replay, source CAS, native/default gates and independent read-only
review qualify the slice. Manual CLI scenario: prepare a note identity into a JSON
request, inspect/create/approve it, restart/inspect and Undo the recorded operation.
Native convenience controls/whole-stage owner acceptance remain later qualification.

## Following dependencies

After this slice, add fresh derived identity lookup, duplicate reporting and
current/source/history scopes; identity resolution rechecks saved bytes, including
same-size/retained-mtime edits. Then add durable provenance and relationship
extraction, basic findings/session timestamps, and bilingual retrieval. Fix each
interface before bounded delegation. Reuse index.sqlite; no new datastore/framework.
Actual provider checks or model downloads require separately scoped permission.


First-slice review: independent read-only review against `15d4608` exposed two
valid parser defects (opaque indented metadata and missed root flow identities).
Root worker regressions reproduced both, plus an overbroad ordinary-Markdown
restriction. Final review then reproduced a managed scalar-continuation ambiguity
through the actual CLI; a real-worker red and Store atomicity regression verify
its correction. Fresh independent final checks passed **9 parser, 6 Store
lifecycle, 7 workflow and 5 CLI tests**, with no remaining actionable finding.
No generic YAML parser or dependency was added; unsupported root layouts are
reported rather than interpreted or repaired. Fresh ordinary Create keeps raw
Markdown behavior, and Replace protects recognized known IDs. Full historical
replay/receipts remain outside new metadata validation.

Root’s first targeted worker compile needed five mutable-worker fixture
declarations corrected; those were test construction errors. The final seven
worker tests pass, including exact assignment/approval/Undo/restart, fresh saved
observations, source CAS/fences and all reproduced parser cases. CLI preflight
covers 23 malformed invocations before operational/credential startup. Full
qualification remains open: the first pre-correction workspace gate failed an
existing cross-process ownership-release fixture (`same_nested_and_aliased_roots_exclude_other_processes`)
with VaultBusy. Its isolated exact run and 20 paired runs (40 tests) passed, so the original
intermittent failure was not reproduced. A synthetic Darwin probe established
fork-before-exec lock inheritance; inspection found other library subprocess
launches bypassing the local fixture mutex. The narrow test-only correction uses
one shared mutex through child exit and the ownership release/reacquire proof,
without changing production locks or assertions. All private file fixtures now
use canonical temporary roots outside Git. Independent correction review found
no actionable defect and passed 12 file tests, 2 apply/Undo matrices, 1 editor
interruption matrix and 1 repair interruption matrix. Implementer checks also
passed exact ownership (1), focused subprocess tests (9), and five paired runs (10).


First-slice final qualification, 2026-10-04: fresh macOS arm64 / pinned Rust
1.98.1, locked/offline `TMPDIR=<fresh owned outside-Git parent> bash
scripts/verify-end-to-end.sh` passed retirement, formatting, workspace build,
all-target Clippy with warnings denied, **711 tests / 0 failed / 2 ignored** and
**52 end-to-end assertions**. Both ignored crash entry points are exercised by
subprocess matrices. Fresh native desktop tests
(`native-ui,native-retrieval,native-test-support`) passed **147 / 0 failed**;
shipping build (`native-ui,native-retrieval`, no test-support feature) and native
all-target Clippy with warnings denied passed. Upstream `block v0.1.6` still
reports its known future-compiler warning. No live calls, downloads, original
vault inspection, GUI interaction or actual native inference were performed.

The first identity slice is implemented, automated verified, independently
reviewed and locally integrated with this change; Stage 5 as a whole remains
active. Owner acceptance is pending and not a later safe implementation
dependency. [CLI identity scenario](../../../../crates/brn/README.md#managed-note-identity)
covers preparation, exact review/approval, restart and Undo. Native convenience,
whole-stage metadata/provenance/search qualification and actual English/Estonian
inference remain future slices. No published CI run exists for the local baseline;
inspect the integrated commit separately. Outside-Git logs retain the original
failed gate and final passing gate; only exclusively owned fixture entries are
cleaned with type/owner/identity checks. Full-gate success, rather than the
unreproduced intermittent diagnosis alone, establishes final qualification.
