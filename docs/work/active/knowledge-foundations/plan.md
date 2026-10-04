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

## Second slice: fresh identity lookup and archived evidence reading

Baseline: `main@0a80ba286f814f626b00b6524c7f12c409bfd6cb`; only the preserved
owner AGENTS.md change remains unrelated. The identity slice is locally integrated;
no published CI run exists for its commit. Continue within the frozen mission.

Add one read-only evidence path/scan seam permitting top-level archive notes.
Current VaultPath validation and default list/read/search remain unchanged, so
proposal/editor/Save authority does not expand. The same exact-byte, bounded UTF-8,
non-symlink read rules apply to explicit evidence reading. No move, migration or
archived mutation is introduced.

A fresh UUID inventory reads the complete visible Markdown evidence universe,
including archives, instead of trusting cached size/mtime or only index candidates.
It reports unmanaged notes, malformed/unreadable entries and all duplicated IDs.
Resolution returns Unique, Absent, Ambiguous or Incomplete with observed matches
and issues; incomplete inspection cannot certify uniqueness/absence. No path/hash
fallback, UUID minting, database table or automatic repair is added. These are
observations of saved evidence, not a transactional vault snapshot; later writes
must still use the existing exact proposal/source version boundary.

Expose inventory/resolution and explicit evidence reading through AppWorker/CLI.
CLI preflight validates syntax and nonnil UUIDs before operational startup. Useful
checks cover archive/default-current separation; same-size/retained-mtime ID edits;
duplicates, moves and removals; malformed/oversized/non-UTF-8 data; fresh restart
and deleted/rebuilt index; unchanged vault bytes; and unresolved application fences.
Get independent complete read-only review, technically verify findings, run fresh
integrated/default and affected native gates, document a reproducible synthetic
CLI scenario and integrate the completed slice. Current/source/history metadata
classification, scoped search and durable provenance remain subsequent slices.


Second-slice evidence, 2026-10-04: independent complete review against `0a80ba2`
found no actionable macOS defect. Its fresh locked/offline checks passed **27**
checks (5 lookup, 8 existing vault, 3 evidence, 5 inventory/evidence CLI, 5 prior
identity CLI, 1 direct-Invocation preflight). Root lookup tests also passed **5**,
including preserved-mtime duplication, exact archived reads, default current
write/read refusal, incomplete inspection, restart/index rebuild and application
fences. An attempted extra non-UTF-8 filename fixture was rejected by APFS itself
(errno 92); it was replaced with an actual supported filename rejected by the
contained-path rules. Existing non-UTF-8 filename coverage is filesystem-limited
on this host, not presented as actual exercised qualification.

Fresh final macOS arm64 / Rust 1.98.1 locked/offline gate passed retirement,
workspace format/build/all-target Clippy, **725 tests / 0 failed / 2 ignored**
and **52 end-to-end assertions**. Fresh native desktop tests passed **147 / 0
failed** (`native-ui,native-retrieval,native-test-support`); shipping native build
without test support and native all-target Clippy with warnings denied passed.
The known upstream `block v0.1.6` future-compiler warning remains. No original
vault/private data, live provider calls, downloads or actual GUI/native inference
were inspected/exercised. Local Markdown links and diff checks are validated
before integration. Only own synthetic fixture entries are cleaned; outside-Git
logs remain. This slice is implemented, automated verified, independently reviewed
and locally integrated with this change. [CLI manual lookup/evidence scenario](../../../../crates/brn/README.md#managed-note-identity)
is reproducible; owner/native acceptance remains pending. Stage 5 is still active.

## Third slice: metadata classification and scoped retrieval

Baseline: `main@7694e789c4b83d46d4ac949cfee83c089a8aeadb`, with only the
preserved owner AGENTS.md change unrelated. The second slice is locally integrated
and has no published CI run. Continue within the frozen mission.

Use two ordinary optional scalar fields: `brn_kind: knowledge|source` and
`brn_state: current|history`. Absent fields preserve unmanaged current-note
behavior; top-level archive paths remain historical evidence regardless of state.
Source scope returns original source notes, including historical originals;
History returns historical knowledge and sources; All is explicit combined access.
Current returns current knowledge only. Unsupported/malformed managed class fields
are inspection issues, not guessed defaults. All metadata changes still require
ordinary complete proposals (or explicit manual Save); no automatic stamping.

Keep the narrow metadata parser/protection in Store and caller-supplied disposable
metadata in index.sqlite. Extend the existing BRNI index schema/rebuild mechanism,
not another datastore. Refresh reads saved full bytes even when size/mtime are
unchanged, preserving unchanged passages/vectors by hash. Derived rows carry UUID,
source/history flags and metadata issues. UUID resolution keeps its fresh full-
universe observation rather than trusting index uniqueness.

Filter scope before keyword/semantic candidate ranking and limits, including
hybrid fusion. Preserve existing default Current APIs and model mismatch/keyword-
only rules; add explicit scoped APIs through AppWorker and CLI. Current reads and
retained AI tools validate saved class/hash as well as visibility, and all scopes
keep unresolved-work fences. Explicit evidence reading still preserves malformed
original wording. AI tool scope selection/native scope controls follow immediately
as separate reviewable integration slices; no live provider call is needed here.

Acceptance: source/history content never crowds out eligible current hits or
silently enters default queries; explicit source/history/all return attributable
exact saved passages. Same-size/retained-mtime content and classification edits are
noticed; malformed classes are visible issues. Derived index deletion/outdated
schema rebuild restores classifications without modifying notes; no embeddings
are needlessly discarded for unchanged content. Meaningful parser, index/reader,
keyword/semantic/hybrid, workflow fence/freshness and CLI preflight/process tests,
independent review, fresh relevant gates and a synthetic manual scenario qualify
this slice. Durable provenance/relationships/findings/bilingual qualification
remain later Stage 5 work.

Third-slice evidence, 2026-10-04: the Store parser/lifecycle checks passed 21,
retrieval checks passed 37, and CLI checks passed 22 process tests plus one
direct preflight test. Root regression coverage exercises actual AppWorker scope
pages/reads/search, same-size/retained-mtime class/content changes, source/history
ranking before limits, unchanged-vector retention, malformed metadata, index
rebuild, exact original bytes and all-scope unresolved-work fences. Initial
integration assertions were updated to count the new complete evidence cache,
while default results still exclude sources/history.

Independent complete review against `7694e78` passed 61 scoped checks and found
one valid P2: an unreadable archive directory aborted startup and blocked readable
current notes. Root's actual permission regression failed before correction.
Nested evidence-directory failures now become explicit inspection issues; refresh
drops uninspected cached descendants and identity lookup remains Incomplete.
Root's fresh corrected scope/lookup/library/AI/vault checks passed 36. Independent
correction review passed 15 checks and an offline CLI build, then repeated an
actual owned CLI permission fixture: startup/current list/search/read succeed,
stale archive rows disappear, and zero/one observed UUID matches both remain
Incomplete. No remaining actionable finding. The earlier unreadable-file regression
was also reproduced and corrected. Root's first cleanup inspection used the wrong
layout prefix and refused before deletion; the corrected inventory verified exact
owned names/types/UID/device/inode before cleanup.

Fresh final macOS arm64 / Rust 1.98.1 locked/offline qualification after the last
production correction passed `TMPDIR=<fresh private outside-Git parent> bash
scripts/verify-end-to-end.sh`: retirement, workspace format/build/all-target
Clippy with warnings denied, **751 passed / 0 failed / 2 ignored**, and **52
end-to-end assertions**. Fresh optional commands all exited 0:

```sh
cargo +1.98.1 test -p brn-retrieval --features native --lib --test model_download --locked --offline
cargo +1.98.1 test -p brn-workflow --features native-retrieval --lib --test models --locked --offline
cargo +1.98.1 test -p brn-desktop --features native-ui,native-retrieval,native-test-support --locked --offline
cargo +1.98.1 clippy -p brn-desktop -p brn --all-targets --features brn-desktop/native-ui,brn-desktop/native-retrieval,brn/native-retrieval --locked --offline -- -D warnings
cargo +1.98.1 build -p brn-desktop --features native-ui,native-retrieval --locked --offline
cargo +1.98.1 build -p brn --features native-retrieval --locked --offline
```

Earlier pre-correction full/native gates also passed but do not qualify the final
correction. The known upstream `block v0.1.6` future-compiler warning remains.
No live calls, downloads, original/private vault inspection, actual GUI interaction
or English/Estonian native inference occurred. Final diff checks and 119 local
Markdown links across 13 affected/current contract files passed before integration.
Only the two exclusively owned gate parents' 20 layout fixture directories and
16 regular files were cleaned; outside-Git logs/ownership metadata remain.

The third slice is implemented, automated verified, independently reviewed and
locally integrated with this change. [Scoped CLI manual scenario](../../../../crates/brn/README.md#commands)
is reproducible with synthetic data; owner/native acceptance remains pending.
AI tool scope arguments/native controls follow next, then durable provenance,
relationships, findings/timestamps and bilingual qualification. Stage 5 remains
active. Published CI availability is checked separately; no push or release is
authorized merely to create a run.

## Fourth slice: AI and native access to explicit evidence scopes

Baseline: `main@4a97dcb0e952b4d78f6ed57d25cecfec33bca0a3`; only the preserved
owner AGENTS.md change is unrelated. Third slice is locally integrated; hosted
CI lookup returned no published run. Existing workflow scoped APIs cover this
slice without changing authority or architecture.

Add a narrow serializable `brn_ai::ReadScope` enum (Current/Source/History/All)
and scoped read-tool methods, mapping explicitly to workflow KnowledgeScope.
Existing trait methods/default calls remain Current; old implementers reject
unsupported explicit scopes rather than silently reading current. The same three
Rig tools accept optional strict scope arguments and label scope in serialized
results. Workflow validates scope/class/hash/quotes and all evidence fences;
the owned drain lease forwards scoped calls, preserving cancellation/shutdown.
Actual provider routes are checked only with synthetic transports, not live calls.

Native browsing uses a transient selected scope, scoped list/search commands and
correlated request generations/cursors. Default Current note openings retain the
existing guarded editor. Source/History/All openings use ScopedNote and a separate
read-only exact-text view with scope/path and Copy, without editor registration
or write controls. Existing unsaved editor/review/comment/form navigation guards
also protect evidence navigation. Scope switching changes browsing only, not
the open document or Ask's default scope. Late list/search/note/error replies
cannot replace newer scope/document state; requested scope labels do not invent
per-note classification.

Acceptance: actual offline Rig routes accept all explicit scopes and reject
unknown/extra arguments; omitted scope remains Current and result labels agree.
Fresh source/history reads/search/list preserve exact evidence and current
exclusions, fences and owned read leases. Native selection/pagination/search/open
freeze scope and reject stale replies; source/history views preserve BOM/CRLF/
Unicode exact bytes without Save/proposal admission. Meaningful adapter, worker,
presentation and native widget checks, independent complete review, fresh relevant
gates and a disposable manual scenario qualify this slice. Durable provenance,
relationships, findings/timestamps and real bilingual inference remain later.

Fourth-slice evidence, 2026-10-04: initial scoped workflow regressions failed
against the unextended adapter; after implementation, 2 new scoped tool, 5
existing tool and 7 knowledge-scope tests passed, including retained all-scope
application fences. One real owned-worker test proves scoped list/search/read
forwarding and Completed waiting for its retained reader despite Stop. Obsolete
private Current-only wrappers were removed after Clippy exposed their disuse.
AI library checks passed 84; five new synthetic production-route tests exercise
all three provider dialects, four scopes, omitted Current compatibility, strict
refusal before reads and existing caps. Initial schema/error-copy expectations
were overconstrained: root verified pinned Rig's Copilot Responses strict-schema
normalization and same-model transient argument parse diagnostics, then rejected
those unsupported expectations without changing provider policy or local safe
error/progress handling.

Desktop default checks passed 137 (130 unit + 7 CLI); fresh native/test-support
checks passed 155 (148 + 7), including real-worker restart, stale correlation,
actual exact-text readonly widget/Copy and editor/review/comment/form guards.
Independent complete read-only review against `4a97dcb0`, including untracked
tests, found no actionable defects and passed 28 focused checks. Its factual
README carryover was corrected; no product scope was expanded.

Fresh final locked/offline macOS arm64 / Rust 1.98.1 workspace gate passed
retirement, formatting/build/all-target Clippy with warnings denied, **765 passed /
0 failed / 2 ignored**, and **52 end-to-end assertions**. Fresh native all-target
desktop/CLI Clippy and shipping desktop/CLI native builds passed; the final frozen
desktop native tests above remain applicable. Commands match the third-slice
native Clippy/build commands and `scripts/verify-end-to-end.sh` with a new owned
outside-Git TMPDIR. The two ignored crash entry points remain exercised by their
subprocess matrices. Known upstream `block v0.1.6` warning remains.

Root additionally observed the shipping native binary in a fresh explicit
synthetic data/vault/credential pair, using a unique local fixture bundle identity
and fixture-only launcher logs. Current excluded sources/history; Source keyword
search returned exact original/archived paths; Source and History openings showed
requested path/scope and Read only with no Save. Confirmed readonly typing changed
nothing. Browsing scope changes retained the open document; navigation from an
unsaved current editor to evidence retained registered recovery. Actual Copy→OS
paste→guarded Save Copy produced **86 identical bytes**, including BOM/CRLF and
Estonian characters, while current/original/archive/history files stayed exact.
AX text did not expose the leading BOM; the actual saved-byte comparison qualified
the copy path. Initial capture failed once and recovered by exact bundle selection;
stale AX indices refused two operations before refresh, with no unrelated action.
The Mac then locked before guarded GUI Quit/restart could complete. That native
restart/owner/IME/accessibility qualification remains pending; synthetic worker
restart is verified. No provider call, model download or original/private data
inspection occurred. The synthetic GUI fixture remains owned for that pending
check; other gate fixtures are cleaned by exact ownership/type/identity guards.
Final diff checks and 136 local Markdown links across 15 contract/status files
passed. Root's final gate cleaned five owned layout directories/four regular
JSON files and its empty parent; logs and ownership metadata remain outside Git.

This slice is implemented, independently reviewed, automated verified and locally
integrated with this change. [Native manual scenario](../../../../crates/brn-desktop/README.md#current-workspace-native-default)
is reproducible; partial native observation is distinct from owner acceptance.
Stage 5 remains active for durable provenance, relationships, findings/timestamps
and actual English/Estonian retrieval qualification.

## Fifth slice: durable exact vault provenance

Baseline: `main@4fb2763e66d82d4241a6b92b8dcb6549c2baee23`; preserve the owner's
unrelated AGENTS.md change. Use one optional ordinary frontmatter field,
`brn_provenance: <single-line JSON array>`, containing bounded typed vault
citations: nonnil note UUID, full saved SHA-256, UTF-8 byte range and exact quote.
The quote is durable Markdown content, independent of operational sessions and
the disposable index. No new database, automatic metadata stamping or migration.
Web provenance extends this typed representation in its later roadmap stage.

Store supplies pure strict read/write/validation, retaining unrelated bytes, BOM
and line endings. Workflow resolves UUIDs from the fresh complete evidence
universe, captures exact saved citations and prepares additive ordinary Replace
drafts with full target/source fingerprints. Explicit archive source capture uses
EvidencePath; writable destinations remain VaultPath. Capture/preparation creates
no editor, review or file changes. Self-capture preparation is refused; existing
citations are preserved, including later unavailable/stale references.

Fresh approval validates newly added/changed citations against bound source
versions and fresh unique IDs, exact ranges and quotes; unchanged historical
citations and exact Undo remain readable/restorable. Replay must stay history-only.
Read-only inspection reports matched, changed, absent, ambiguous or incomplete
source resolution without substituting paths/quotes. AppWorker and strict CLI
expose inspection/capture/preparation; native convenience follows separately.

Acceptance covers exact BOM/CRLF/Unicode preservation, archived sources, duplicate/
incomplete resolution, stale preparation/admission/approval, unsupported metadata,
full-review edits and approval, restart/index rebuild and exact Undo. Verify that
ordinary files alone retain and resolve provenance in a fresh operational store;
actual session Delete remains a later lifecycle qualification. Run meaningful
Store/workflow/CLI tests, independent complete review, fresh integrated and affected
native checks, and document a reproducible synthetic scenario before integration.

Fifth-slice evidence, 2026-10-04: Store pure/parser/lifecycle checks passed **190**
and CLI checks **119**, including strict JSON/FIFO/direct-Invocation preflight and
actual capture/preparation/review/approval/restart. Root's corrected focused
workflow/AI/index checks passed **48**, including ten provenance tests. Quotes
remain exact through BOM/CRLF/Unicode, archived capture, moves/duplicates/stale or
missing originals, restart/index deletion, ordinary full edits/Rewrite, exact
Undo and copying only ordinary Markdown to an independent vault/store. Pending
applications fence all new reads/preparation; no operational session is required.
Actual session Delete remains later Stage 13 qualification.

Independent complete read-only review against `4fb2763` passed **75** initial
checks and found one valid defect: malformed saved provenance bypassed derived
metadata eligibility. Root reproduced the missing inspection issue before fixing
the existing `saved_metadata` issue chain. Independent corrected checks passed
**32**, an offline CLI build and actual synthetic CLI reproduction: invalid notes
are excluded/refused while explicit raw EvidenceRead retains exact original bytes.
No remaining actionable finding; no index schema or destination authority changed.

The first integrated attempt failed because the desktop exhaustive event match
omitted the three new query replies; its existing ignored unsolicited-query branch
now includes them. Two test-only Clippy clone warnings were corrected. A subsequent
pre-metadata-correction gate passed 787 tests/52 assertions; it does not qualify
the final correction. Fresh final macOS arm64 / pinned Rust 1.98.1 locked/offline
`TMPDIR=<fresh owned outside-Git parent> bash scripts/verify-end-to-end.sh` passed
retirement, formatting/build/all-target Clippy with warnings denied, **788 passed /
0 failed / 2 ignored**, and **52 end-to-end assertions**. The ignored private
crash entry points remain exercised by subprocess matrices.

Fresh native desktop checks passed **155 / 0 failed**, native desktop/CLI
all-target Clippy and both shipping builds passed, using the fourth-slice feature
commands. Only the known upstream `block v0.1.6` future-compiler warning remains.
Native controls did not change; this does not establish GUI provenance usability,
IME/accessibility, real English/Estonian inference or physical power-loss behavior.
No live provider call, download or original/private data access occurred.
[CLI provenance scenario](../../../../crates/brn/README.md#durable-source-provenance)
is reproducible; owner acceptance and native provenance convenience remain pending.
After native builds finished, a serialized default workspace build, **119 CLI
tests** and fixture-only gate (**52 assertions**) also passed, so executable
feature outputs cannot interfere with the default fixture qualification. Final
Markdown link/diff checks passed. Only the three exclusively owned gate parents'
ten layout directories/eight regular JSON files and empty parents were cleaned
with type/UID/device/inode guards; logs/metadata remain outside Git. No published
CI run exists for the local baseline; integration is local, without push/release.
This slice is implemented, independently reviewed, automated verified and locally
integrated with this change. Stage 5 remains active.

## Sixth slice: native saved provenance inspection

Baseline: `main@122d7632cda65a383fa96c70e189dfbb186c5024`, preserving the owner
AGENTS.md change; no published CI run exists. Existing read-only workflow DTOs
cover this native slice. Add a Sources control to the current editor and exact
evidence view, explicitly inspecting saved provenance without changing unsaved
typing, scope, review work or files. Expand each stored quote with its source
UUID, observed path(s), resolution status and exact Copy; unknown/changed sources
remain visibly distinct and no guessed source navigation is introduced.

Bind requests/results/errors to the open document path, document generation and
inspection generation. Clear transient inspection on navigation; late replies
must not replace a newer document or newer inspection. Quote widgets remain
read-only, bounded and copy exact BOM/CRLF/Unicode text. Loading/empty/error states
remain useful without introducing a new mutation or approval route. No provider
call, credential or model operation is needed.

Acceptance: real AppWorker source status/quote inspection, saved vs unsaved work,
stale success/error correlation, all resolution outcomes, exact widget text/Copy
and unchanged navigation guards. Get independent complete review, technically
validate findings, run fresh desktop/default/native and integrated checks, retain
a synthetic reproducible manual scenario and integrate locally. GUI observation
and owner acceptance may remain pending while the Mac is locked; continue later
safe Stage 5 foundations.

Sixth-slice review found one valid bounded-panel defect: a scrollbar layer did
not enable overflow scrolling, so later source quotes were unreachable. An
actual Root/Desktop regression with twelve real-worker citations failed on the
unchanged wheel offset before the fix. Adding overflow scrolling made the same
wheel input reveal citation twelve; its actual Copy button preserves the full
BOM/CRLF/Unicode source. The independent correction review passed **15 checks**
(11 provenance/state/widget, two review guards, two scope/navigation guards),
with no remaining actionable finding. Save and confirmed Reload invalidate old
inspections; Close and accepted navigation cannot be undone by late replies.

Fresh macOS arm64 / Rust 1.98.1 locked/offline helper checks passed **143 default
desktop tests**, **166 native desktop tests** and both shipping and headless-test
native all-target Clippy with warnings denied. The native feature set was
`native-ui,native-retrieval,native-test-support`; the shipping Clippy omits the
test feature. The known upstream `block v0.1.6` warning remains. Reproduce manual
acceptance through the [desktop scenario](../../../../crates/brn-desktop/README.md):
inspect saved quotes while typing remains recoverable, Refresh after changed or
duplicated fixture sources, copy exact text and exercise read-only All scope.
Native GUI/owner acceptance remains pending; these are headless observations.

Root final integrated gate on the corrected tree passed **794 workspace tests /
zero failures / two ignored private crash entry points**, all **52 end-to-end
assertions**, retirement, workspace format/build and all-target Clippy with
warnings denied. Command: `TMPDIR=<fresh owned canonical parent> bash
scripts/verify-end-to-end.sh`, pinned 1.98.1, locked/offline. After that serialized
default gate, the shipping `native-ui,native-retrieval` desktop build and real
AppWorker `--headless-check startup` passed. No provider/model/network, original
data, GUI observation or owner acceptance is claimed. Final documentation checks
cover local links, formatting and diff whitespace. This slice is implemented,
automated verified, independently reviewed and locally integrated with this
change; relationships, findings/timestamps and bilingual qualification remain.

## Seventh slice: reliable session timestamps

Baseline: `main@63c618d685d40780d280c606b3b9d38e09c0b06e`, with only the owner
AGENTS.md change. Expose known conversation creation time and add nullable
conversation last-activity and turn start/finish times through a narrow additive
WorkStore V8 migration. Historical unknown times stay unknown. Capture fresh
admission and explicit/provider finalization atomically with existing chat writes;
exact replay, read-only inspection and restart interruption must not refresh
activity. Keep times monotonic under wall-clock rollback and validate stored
nonnegative values, pair agreement and temporal ordering before reads/recovery.

Existing workflow Conversations/Turns events and CLI JSON carry these projections;
native session labels show readable last-activity age or explicit unknown time.
Unpersisted partials retain unknown timestamps. No new proposal/worker framework,
clock service, account/model operation or session lifecycle behavior. Archive,
Restore/Delete and the 30-day policy stay Stage 13 work.

Acceptance: new admission/finalization timing, stable running/terminal replay,
rollback on failed admission/finalization, owner/attached-writer serialization,
old-schema upgrade and backup restoration, restart without fabricated activity,
malformed pair/conversation refusal and exact old content preservation. Verify
one coherent conversation summary during attached writes, real worker/CLI
projection and native readable unknown/current/future-clock
labels. Obtain independent read-only review, validate findings, run fresh Store,
integrated/default/native checks and update evidence before local integration.
Manual scenario: inspect sessions/turns from a fresh synthetic completed and
interrupted turn, restart without new chat and confirm the recorded times do not
change; inspect a restored older fixture and observe unknown historical activity.

Seventh-slice evidence, 2026-10-04: fresh Store checks passed **201 / 0 failed**,
including 11 timestamp tests covering atomic rollback, attached writers, legacy
upgrade/backup restoration, malformed values and unchanged restart/replay timing.
Real owned worker and actual CLI process regressions qualify the projections;
native labels keep unknown, future-clock and current activity distinct. The CLI
regression first reproduced omitted fields before the explicit null/time mapping
was corrected. Unpersisted partials never acquire invented timing.

Initial independent review passed 33 checks. A subsequent public-API probe
reproduced five incoherent summaries during 1,000 valid attached writes: a newer
count could accompany an older activity time. Lead verified the supported-input
defect; a regression failed before the correction. The summary now uses one
deferred read transaction across title/count/times. Independent correction review
passed the new regression and rebuilt the original probe against current code:
**1,000 admissions/finalizations, 3,040 summaries, 0 inconsistencies**. No remaining
actionable finding. Whole-history snapshot behavior was not added.

Fresh final pinned Rust 1.98.1/macOS arm64 locked/offline gate:
`TMPDIR=/private/tmp/brn-timestamps-root-9uvlb86h bash scripts/verify-end-to-end.sh`
passed retirement, workspace formatting/build/all-target Clippy with warnings
denied, **809 tests / 0 failed / 2 ignored** and **52 end-to-end assertions**.
Fresh native desktop tests (`native-ui,native-retrieval,native-test-support`)
passed **168 / 0 failed**. Both native all-target Clippy configurations and the
shipping native desktop build without test support passed. Shipping headless
AppWorker startup/restart and actual CLI history checks passed in separate fresh
synthetic data and V7-schema upgrade fixtures; unknown legacy values stayed null
and vault bytes stayed exact. The ignored crash entry points remain exercised by
subprocess matrices. The known upstream `block v0.1.6` warning remains.

The reproducible owner fixture is
`/private/tmp/brn-timestamps-root-9uvlb86h/manual`: `data` has completed/interrupted
synthetic sessions; `legacy-data` has the same shapes with unknown historical
activity/start/finish times. Run `target/debug/brn conversations list --json
--data-dir <fixture>/data`, then `conversations show <id>` with the same flags;
repeat without new chat and compare times. Launch shipping `brn-desktop` with
`--data-dir <fixture>/data --vault <fixture>/vault`, inspect History and restart.
Repeat with `legacy-data`; unknown history must not look recently active.
This fixture was seeded through public Store APIs with no provider request; its
owned legacy copy was downgraded before testing the supported additive upgrade.
Logs and ownership metadata are retained outside Git. The Mac is locked, so GUI
and owner acceptance remain pending. No account calls, model downloads, private
data inspection or session Archive/Restore/Delete occurred. This slice is
implemented, automated verified, independently reviewed and locally integrated
with this change. Stage 5 remains active.

## Eighth slice: exact saved Markdown link inspection

Baseline: `main@baf3beeb8fbca5a586419a61b416e64cf704609f`; only the owner
AGENTS.md change is unrelated. The timestamp slice is locally integrated and
hosted CI lookup returned no published run. Continue Stage 5 relationships.

Extract ordinary CommonMark inline/reference links from saved body bytes, excluding
frontmatter, code, images and raw HTML. Reuse the managed frontmatter walk for an
exact body offset, including BOM, CRLF and both supported closing delimiters.
Use pinned markdown 1.0.0, already in the lockfile, through brn-workflow. Preserve
exact occurrence/used-definition byte ranges and quotes. Bounded inspection
refuses excessive output explicitly rather than silently truncating evidence.

Resolve contained relative Markdown paths and `brn://note/<nonnil UUID>` links
against one fresh identity inventory; UUID links survive renames. No filename,
title or cached-index guess is allowed. Preserve external/non-note/unsupported
destinations as explicit outcomes, and distinguish absent, unmanaged, ambiguous,
incomplete and changed saved observations. Source identity uncertainty remains
visible. These are derived saved-link observations, not newly approved/inferred
relationships. No vault writes, network, graph datastore or new operational table.

Expose read-only inspection through AppWorker and CLI, with preflight before
authority startup. Acceptance covers exact Unicode/BOM/CRLF proofs, reference
definitions before/after uses, frontmatter/code/image exclusion, contained relative
and percent-encoded paths, UUID moves/duplicates/incomplete inspection, fresh
retained-mtime edits, restart/index loss, unresolved-work fences and unchanged
vault bytes. Obtain independent read-only review, validate findings, run fresh
affected/integrated gates, record a reproducible CLI scenario and integrate.
Native controls, derived edge indexing, durable link preparation and basic
findings remain following Stage 5 slices; the graph canvas stays Stage 14.

Eighth-slice evidence, 2026-10-04: exact saved extraction passed seven pure tests;
nine Store body-offset tests retain both closing fences, BOM/CRLF, opaque values
and bounded refusal. Seven real workflow tests prove UUID moves/duplicates,
retained-mtime edits, incomplete inspection, source uncertainty, URI containment,
restart/index deletion/new-store reconstruction and unresolved-work fences.
CLI checks passed **42 unit / 3 process tests**, including direct preflight before
authority/credential startup. Fixtures use separate owned data/vault roots.

Lead's actual App regression and independent review reproduced an inherited
valid-input defect: an ordinary leading thematic break followed by a link was
mistaken for unsupported root-flow frontmatter. Non-strict reads now recognize
the existing no-managed/no-closer ordinary-body case before interpreting layout.
Strict assignment, complete unsupported flow, malformed closers and incomplete
managed syntax stay refused. Independent review also reproduced the missing
native `NoteLinks` event match; the existing read-only evidence branch now handles
it. Final corrected-tree review passed **62 tests**, shipping native check and
six actual synthetic CLI cases, with no remaining actionable finding.

Fresh root Rust 1.98.1/macOS arm64 locked/offline gate:
`TMPDIR=/private/tmp/brn-links-root-ufgzz6bk bash scripts/verify-end-to-end.sh`
passed retirement, workspace format/build/all-target Clippy with warnings denied,
**836 tests / 0 failed / 2 ignored** and **52 end-to-end assertions**. The first
gate stopped at a test-only non-octal permissions literal; the corrected complete
gate qualifies the final tree. Both private ignored crash entry points remain
exercised by subprocess matrices. Native desktop tests passed **168 / 0 failed**;
native workflow library/model/link checks passed **135 / 0 failed / 2 ignored**.
Both native desktop Clippy configurations and shipping build without test support
passed. Shipping headless AppWorker startup/restart passed in a new owned link
fixture. Actual ONNX/model inference was not exercised. The known upstream
`block v0.1.6` future-compiler warning remains.

[CLI manual acceptance](../../../../crates/brn/README.md#saved-note-links) covers
exact proof inspection, a renamed source, duplicates and index deletion. The
reproducible fixture at `/private/tmp/brn-links-root-ufgzz6bk/manual` retains
`data`, `vault/current.md` (BOM/CRLF), an unmanaged thematic-break note and an
archived source. Root's actual `links show` returned three exact resolved proofs;
all vault bytes remained unchanged. Logs and ownership metadata stay outside Git.
The parser dependency is the existing pinned markdown 1.0.0; only the workflow
dependency line was added to Cargo.lock after correcting an unintended offline
lock refresh. No dependency versions changed. No live calls, downloads or
original-data inspection occurred. Owner/GUI acceptance remains pending. This slice is implemented,
automated verified, independently reviewed and locally integrated with this
change. Relationship indexing/preparation/views, basic findings and multilingual
qualification continue; Stage 5 remains active.

## Ninth slice: disposable saved-note relationships

Baseline: `main@14499fec2c105787f4adf49defdc9263152d0946`; preserve the unrelated
owner AGENTS.md change. Derive directed explicit Markdown links and separately
labelled provenance-based candidates from fresh saved evidence. Only unique,
eligible managed endpoints with exact full hashes and nonempty UTF-8 byte/quote
proofs enter the existing disposable index. No inference call, durable relationship
write, graph datastore or operational table is introduced.

Use one fresh identity inventory per relationship observation. Reuse the exact
CommonMark resolver; combine repeated proofs by endpoint pair and origin. A saved
provenance quote suggests a source relationship only while its UUID/hash/range/
quote still matches. Self-links do not create note-to-note edges. Ambiguous or
incompletely inspected identities never become guessed edges. Recheck endpoint
bytes before recording the observation; report incomplete/extraction observations.
The cache is not a transactional vault snapshot or authority.

Store typed endpoint/proof records atomically in BRNI schema V3, with foreign-key
removal and reader validation. Upgrade healthy V2 additively, retaining passages
and embeddings; branded damage still rebuilds and foreign files stay refused.
The cache accepts up to 8,192 distinct proofs per coalesced edge: the accepted
4,096-link extractor can produce an occurrence and definition for every link.
The existing 4 MiB summed-quote bound remains intact.
Queries filter both endpoints by existing Current/Source/History/All eligibility
before deterministic pagination. Current is the default; All explicitly includes
cross-scope connections. Fresh workflow/AppWorker/CLI queries rebuild derived
edges offline, so source-unchanged target edits, moves or new duplicates cannot
reuse a stale resolution. Limit pages to 1–200 records and expose the matching
total, exact evidence, inspection issues and duplicate identities.

Acceptance: exact inline/reference/provenance proofs, distinct origins, repeated
proof coalescing, self/external/unmanaged exclusion, scope-before-limit, target-only
edits/moves/duplicates/incomplete inspection, unchanged vault/no proposals, atomic
invalid replacement refusal, read-only parity/refusal, V2 upgrade retaining
vectors, restart/index deletion reconstruction and existing uncertain-work fences.
Obtain independent read-only review, validate findings, run affected/default/native
checks and record a CLI manual scenario before local integration. Durable link
preparation/native views and basic findings follow; graph canvas remains Stage 14.

Ninth-slice evidence, 2026-10-04: retrieval tests passed **47 / 0 failed / 0
ignored**, including ten edge tests for full cross-passage proofs, exact limits,
atomic replacement/invalidation rollback, aliases, corruption/reader refusal,
snapshot pagination and healthy V2 passage/vector retention. CLI passed **128**
tests, including strict direct preflight and three actual process scenarios.
Root workflow link/provenance/scope/relationship checks passed **32**, including
eight relationship regressions. Vault bytes and proposals stay unchanged.

Independent review found a valid boundary mismatch: 4,096 accepted references
with a shared definition need 4,097 distinct coalesced proofs. Lead's real App
regression passed link inspection but failed relationship caching before the
8,192-proof correction; the existing 4 MiB quote bound remains. A helper self-check
also reproduced an introduced absent-path metadata update invalidating another
UUID's edges; the corrected no-op precedes invalidation. Both corrections were
independently rechecked. Final review passed **45 tests / 0 failed / 0 ignored**
and found no remaining actionable defect. Actual CLI probes passed 8,192 exact
BOM/CRLF/Unicode proofs, and a 5,000-note chain reported 4,999 matching edges
with correct first/final pages, hashes and quotes. All 5,002 probe notes remained
byte-identical; no proposals or credential files. Scale requests took 6.108 s /
10.026 s on this machine; each request currently rebuilds a fresh observation.

Fresh root pinned Rust 1.98.1/macOS arm64 locked/offline gate:
`TMPDIR=/private/tmp/brn-relationships-root-q6u4n8p2 bash scripts/verify-end-to-end.sh`
passed retirement, formatting/build/all-target Clippy with warnings denied,
**858 tests / 0 failed / 2 ignored** and **52 end-to-end assertions**. Native
desktop tests passed **168 / 0 failed**; native workflow library/model/link/
relationship checks passed **143 / 0 failed / 2 ignored**. Both native Clippy
configurations and shipping desktop build without test support passed. The two
private ignored crash entry points remain exercised by subprocess matrices.
Actual ONNX inference was not run; the known upstream block 0.1.6 warning remains.

[CLI manual acceptance](../../../../crates/brn/README.md#derived-relationship-pages)
covers scope, exact origins/proofs, restart/index loss, UUID moves and duplicates.
The retained owned fixture at `/private/tmp/brn-relationships-root-q6u4n8p2/manual`
has data, current/related Markdown and an archived source: Current returns one
edge, All two coalesced edges. Shipping headless AppWorker startup/restart passed.
Logs and ownership metadata remain outside Git. No package versions changed;
UUID's existing serde feature was enabled explicitly. No live calls, model
downloads or original/private data inspection occurred. Owner/native acceptance
remains pending. This slice is implemented, automated verified, independently
reviewed and locally integrated with this change. Durable relationship preparation/
native views, basic findings and multilingual qualification continue in Stage 5.

## Tenth slice: durable stable-link proposal preparation

Baseline: `main@61b7dd0f6d205ae2f29c1a537bffd27d8d5d7a74`; preserve the owner
AGENTS.md change. No published CI run exists for that local commit. Continue
Stage 5 using the existing typed proposal lifecycle.

Prepare one additive stable `brn://note/UUID` link from an eligible current note
to an already identified eligible saved target, including source/history evidence.
Both identities must be freshly unique; the request binds the selected target's
full saved hash. Capture the complete consumer and target source versions. Return
ordinary full Replace review input without admitting a proposal or changing files.
Preserve every original byte, escape the single-line label, retain the existing
line-ending convention, and verify the appended link is actually parsed outside
code/HTML. Refuse self-links, existing resolved relationships, malformed metadata,
incomplete identity inspection, stale target selection or excessive note/output.

Fresh ordinary approval checks newly introduced stable-UUID targets after review
edits/Rewrite: existing targets require unique identities and exact captured source
bindings. A target created/replaced within the same complete approved draft uses
those exact reviewed bytes and normal destination/before proofs; duplicate IDs or
targets removed by that draft are refused. Unchanged old links, exact Undo and
completed replay retain their existing authority. Preparation targets saved notes;
cross-proposal creation dependencies remain outside this convenience command.

Expose preparation through AppWorker and `links prepare --file` with strict
pre-startup validation. Acceptance: exact BOM/CRLF/mixed-EOF byte prefix, literal
Unicode/punctuation labels, parser-confirmed placement/refusal, archived target
bindings, no writes/admission before explicit Create/approval, fresh alias/content
refusal at preparation/approval, edit/Rewrite source protection, valid same-draft
targets, restart/rebuild, existing historical links/Undo and uncertain-work fences.
Obtain independent read-only review, validate findings, run relevant integrated/
native gates and record a reproducible CLI scenario before local integration.
Native relationship browsing/preparation controls remain the following slice.


Tenth evidence, 2026-10-04, macOS arm64 / pinned Rust 1.98.1. Private owned root
`/private/tmp/brn-links-root-sn2zitcv` retains logs, ownership metadata and a
synthetic manual fixture. CLI stub RED passed two preflight cases and failed the
two runtime cases; pure helper refusal RED failed seven new tests while seven
existing extraction tests passed. Root controller/approval RED passed three and
failed seven behavioral cases. The first root test build error was corrected
before the behavioral run; it is not counted as RED evidence.

Preparation now captures both complete source versions without admission, keeps
the exact BOM/CRLF/mixed-EOF prefix and verifies one escaped literal AST link.
Approval compares newly introduced stable IDs without the public evidence-output
caps and validates the complete reviewed after-state. Same-draft targets use exact
reviewed bytes; saved targets need their full captured binding. Create removes no
existing inventory occupant; Replace/Trash overlay removal needs the actual
coordinated full before-file proof, including device/inode, rather than folded
names or a matching content hash.

A broader workflow run found a real compatibility regression: unconditional
strict body-offset inspection refused an unrelated legacy root-flow header body
edit. Root minimized RED reproduced the exact error. Independent review then
reproduced two valid approval defects through public CLI: the initial overlay
mistook conservative name folding for positive alias proof on case-sensitive
APFS, and the initial compatibility fallback counted an opaque metadata UUID as
an old body link. Root separately reproduced both before fixing them. Exact
complete leading-header framing now excludes all opaque metadata without
interpreting its fields; unsupported/incomplete framing cannot supply a guessed
boundary. Existing body links retain their authority.

Corrected focused root verification passed **14 preparation + 10 provenance
tests**, with one explicit case-sensitive fixture test ignored by default. That
real App regression separately passed against a fresh owned case-sensitive APFS
image, for both Create and Replace aliases. Root and reviewer confirmed shutdown
and the reviewer detached/removed only its owned images. Independent final review
passed **62 tests / 0 failed / 1 ignored**, plus that explicitly qualified
case-sensitive test **1 / 0 / 0**. Both real CLI defect probes now refuse before
Applying, with all vault bytes unchanged. Public bounds accepted 4,096 resulting
links and exactly 1 MiB; one extra link/byte refused without proposals or credential
files. Final review found no remaining actionable defect.

Fresh root full gate:
`TMPDIR=/private/tmp/brn-links-root-sn2zitcv bash scripts/verify-end-to-end.sh`
passed retirement, formatting/build/all-target Clippy with warnings denied,
**884 workspace tests / 0 failed / 3 ignored** and **52 end-to-end assertions**.
The two pre-existing ignored private crash entry points remain exercised by
subprocess matrices; the third case-sensitive test was explicitly qualified as
above. Actual model assets/inference were not exercised and no live provider calls
or downloads occurred. The first broader workflow attempt failed on the legacy
regression; the final corrected whole gate is the current passing evidence.

[Reproducible CLI acceptance](../../../../crates/brn/README.md#saved-note-links)
uses a fresh synthetic managed current note and archived source. The retained
`/private/tmp/brn-links-root-sn2zitcv/manual` scenario passed prepare without
proposal admission or byte changes, explicit Create and exact review-version
Approve, resolved inspection, restart/index loss and exact original prefix/source
bytes. It retained both full source bindings and zero credential files. Native
relationship controls and owner usability acceptance remain subsequent work;
cross-proposal target creation is outside this convenience command.


Fresh optional native offline verification passed **168 desktop tests**, **164
workflow library/model/link/relationship/preparation tests / 0 failed / 3 ignored**,
and **7 CLI process tests**. Desktop (test-support and shipping), workflow and CLI
native all-target Clippy with warnings denied passed; the shipping desktop build
without test support passed. Actual model assets/ONNX were not used; upstream
block 0.1.6's known future-compiler warning remains. Shipping real AppWorker
headless startup/restart passed twice against the retained manual fixture with all
vault bytes unchanged and zero credential files. Local Markdown checks passed
**172 links across 16 primary/active/crate documents**, and `git diff --check`
passed. This slice is implemented, automated verified, independently reviewed and
locally integrated with this change; native controls and owner acceptance remain
pending. Continue to bounded native relationship inspection before native
preparation, basic findings and multilingual work. No package versions changed.


## Eleventh slice: native saved-link and relationship inspection

Baseline: `main@21f4fb777b18b985fb42765525b46a5aea89d6de`; only the owner
AGENTS.md change remains. Exact-commit hosted CI query returned no published runs.
Use existing NoteLinks and Relationships commands; no new workflow/storage API.

Expose a bounded relationship page in the Vault browser using the existing
current/source/history/all selection. Show explicit links separately from inferred
provenance candidates, endpoint UUID/path/hash proofs, exact evidence and inspection
issues/duplicates. Previous/Next/Refresh replace each page: each request is a fresh
observation, not a cross-page snapshot. Correlate operation UUID, scope, requested
offset/limit and generation. Closing or changing scope invalidates late success
and error replies; show pending/refused/empty outcomes without blocking UI.

Add per-document Saved links inspection for current/source/history evidence.
Retain source identity/hash and uncertainty, target outcomes/observed matches, and
exact occurrence/used-definition quotes with read-only selectable text and exact
Copy. Only one selected link/edge proof needs a persistent editor widget; bound
browser rendering instead of building thousands of proof editors. Saved inspection
leaves unsaved typing intact and is invalidated on original Save, Reload, navigation
or panel close. All operations go through the existing application lane and respect
uncertain-work fences. Inspection neither admits proposals nor writes Markdown.

The native preparation adapter is the following slice: it must preserve complete
DraftRequest source bindings rather than reconstruct only the consumer source.
Graph canvas/navigation remains its later roadmap outcome. This inspection slice
uses existing guarded note browsing; it does not add an unqualified endpoint-open
shortcut.

Acceptance: state correlation/stale replies/closed panes, scope and checked page
bounds, exact multiline/Unicode quotes and Copy, source/history read-only behavior,
retained unsaved buffers, Save/Reload invalidation, pending/empty/refused/ambiguous
outcomes, and real AppWorker observations with unchanged vault/proposals/credentials.
Add meaningful state/native widget tests, obtain independent read-only review,
validate findings, run fresh integrated/native checks and document a reproducible
synthetic manual scenario. Actual unlocked GUI and owner acceptance remain separate.

Independent review identified two concrete defects. A real AppWorker Save Copy
probe preserved original/copy bytes while changing source identity Unique to
Ambiguous; the prior DTO remained displayed. Two behavioral RED failures were
fixed by invalidating Links as well as Relationships after any Applied Save.
Fresh focused state verification passed **10 tests / 0 failed / 0 ignored**, with
unchanged live typing. Full Desktop/Root rendering at the supported 480×480
minimum also reproduced offscreen Links controls with independent Sources open;
both saved-note and read-only evidence regressions failed before the bounded
layout correction. Both valid findings were corrected and re-reviewed below.

The owner now authorizes task-owned milestone branches, PRs and automatic merges
after independent review/local verification and applicable exact-head macOS/shared
CI, with GitHub requirements satisfied. Publish completed stages in order while
Stage 5 continues. [Stage 1 checkpoint PR](https://github.com/ewq100/brn-rust/pull/16)
preserves `6609442`, incorporates hosted CI and now uses head
`8b17a3f4af9a7bfd56d0d62ac2dd4631fb950bf8`.
Fresh isolated local qualification passed **728 tests / 0 failed / 1 ignored**,
**47 end-to-end assertions**, retirement/format/build/all-target Clippy. Its exact
initial head passed three macOS lanes but Ubuntu Clippy caught a macOS-only test
helper without its matching guard. Corrected head `8b17a3f` passed macOS
Core/UI/Retrieval and Ubuntu shared Core in run `37181606332`; Windows failed on
existing Unix-only APIs, so the overall run is red. PR #16 merged as
`3665651a7968d82f5f2724e47ecced462394fc31`. The merged tree equals its qualified
head, and fresh **47 fixtures + 5 editor process tests** passed. Stage 5 has no
checkpoint CI qualification yet. The Stage 1
plan/PR retain acceptance, next-stage and Mac mini environment requirements.

Final independent review of the corrected inspection diff found no remaining
actionable defects. Fresh locked/offline **10 state + 7 new native widget + 5
existing Sources widget tests passed / 0 failed / 0 ignored**. Actual full
480×480 views now reach independent Sources/Links controls and preserve both
exact quotes; inner-pane wheel events do not move the outer document scroller.
Fresh root final `TMPDIR=<owned canonical synthetic parent>
BRN_NATIVE_MODEL_DIR='' bash scripts/verify-end-to-end.sh` passed retirement,
format/build/all-target Clippy, **894 workspace tests / 0 failed / 3 ignored** and
**52 end-to-end assertions**. Two pre-existing ignored crash entry points remain
exercised by subprocess matrices; the third case-sensitive regression was
explicitly qualified in the prior unchanged adapter slice. Native desktop
`native-ui,native-retrieval,native-test-support` passed **185 tests / 0 failed /
0 ignored**. Test-support/shipping all-target Clippy with warnings denied and
shipping `native-ui,native-retrieval` build passed. Upstream block 0.1.6 retains its
known future-compiler notice. Fresh shipping AppWorker startup/restart passed
twice on synthetic managed current/archive source notes, with exact vault bytes
and zero credential files. No provider calls, assets or actual ONNX inference.

[Native acceptance scenario](../../../../crates/brn-desktop/README.md) exercises
Links, exact Copy, unsaved typing, Current/All, archived read-only evidence and
minimum-height independent panes. Actual unlocked GUI/owner acceptance remains
pending. This slice is implemented, automated verified, independently reviewed
and locally integrated by the commit containing this record; checkpoint
publication follows the completed stages. Next: native stable-link preparation
preserving the complete DraftRequest source bindings, then basic findings and
multilingual implementation/qualification. Mac mini requirements remain pinned
Rust 1.98.1, macOS Apple Silicon/Command Line Tools, cached locked dependencies,
protobuf, Bash/Python 3, fresh canonical synthetic directories and an unlocked
session for native acceptance. Existing credentials/data are not transfer inputs.


## Twelfth slice: native exact stable-link preparation

Baseline: `main@9298ad0b7f837c863edfb4a81901292239c210e8`; preserve the owner's
AGENTS.md edit. Stage 2 checkpoint PR #17 is independently reviewed and pending
corrected shared CI; its three macOS lanes passed at the initial head.

Use the existing guarded initial proposal form, captured ProposalSource,
NoteLinks target inspection and PrepareNoteLink command. A Replace form with
an exact captured consumer accepts a scalar target path and literal label.
Preparation is available only before submission, with an empty body or the exact
captured source body, so it cannot discard authored replacement text. A separate
target request requires Unique managed identity and retains full UUID/hash;
archive source/history targets are allowed. Operation, form/input generations,
document generation and full captured consumer proof correlate late replies.
Changed input and stale success/errors cannot replace current typing. A target
may move by UUID during preparation; preserve the returned current binding.

Retain the complete validated prepared DraftRequest with both sources and full
Replace fingerprint. Only title/body remain editable; path/kind/bindings stay
fixed. Keep actual captured before text only when its full proof matches the
prepared consumer. Separate-proposal cloning keeps all bindings with a fresh UUID.
Explicit Create admits ordinary review work; exact Approve applies Markdown.
No new storage/API/framework, automatic admission, provider call or navigation
shortcut. Native controls show the complete immutable source proofs and reuse
existing full body widgets, dirty-form/recovery guards and exact Copy.

Acceptance: meaningful form/state tests for binding retention, retry/separate,
wrong/stale operations/input/consumer proof, archived/moved targets, Unicode and
BOM/CRLF, refusal without input loss, and real AppWorker preparation with unchanged
vault/proposals/credentials until explicit creation/approval. Native widget tests
must exercise the actual form and complete proof rendering. Independent read-only
review, technical validation/fixes and fresh default/native checks precede local
integration. Record a reproducible synthetic manual scenario; actual unlocked GUI
and owner acceptance remain pending. Root owns state/native integration; bounded
helper owns DraftForm's fixed complete-request retention seam and its tests.


The complete request adapter's behavioral RED became **7 focused form tests**
passed; its independent reviewer found no actionable defect and separately passed
all seven. Root's real-worker preparation RED became **6 state tests** passed,
covering archived UUID moves, same-byte consumer inode replacement, stale form/
input/document replies, invalid labels/authored body refusal, and explicit
prepare/Create/Approve with no premature Markdown effects. A second reviewer
found no state defect and independently passed those six plus **6 existing
creation regressions**. Native missing-controls RED became **3 widget tests**
passed at 480×480, including full immutable proof/body Copy and pending Create
refusal. Independent native review passed **3 new + 1 existing initial widget**
tests with no actionable defect. The pinned Editor consumes wheel events when
its inner offset changes; no speculative propagation change was needed.

Fresh final root macOS arm64 / Rust 1.98.1 locked/offline integrated checks passed
retirement, format/build/all-target Clippy, **907 workspace tests / 0 failed /
3 ignored** and **52 end-to-end assertions**. Ignored private crash entries remain
exercised by subprocess matrices; the unchanged case-sensitive adapter regression
was qualified in the tenth slice. Native desktop `native-ui,native-retrieval,
native-test-support` passed **201 tests / 0 failed / 0 ignored**. Test-support and
shipping native all-target Clippy with warnings denied, and shipping native build
passed. Only upstream block 0.1.6's known future-compiler notice remains. Fresh
shipping AppWorker startup/restart passed twice against exact synthetic managed
current/archive notes with zero credential files. No provider/model/private-data
calls or actual ONNX inference. Changed-doc local links/fragments and diff checks
passed. See the native README for the reproducible complete preparation/review/
approval and stale-input scenario; actual unlocked GUI/IME/accessibility and owner
acceptance remain pending.

Stage 2 PR #17 merged as `f615398` and Stage 3 PR #18 as `a7c15d7`, each after all
three exact-head macOS lanes and Ubuntu shared Core passed. Windows retains the
existing Unix API failure, so both overall runs are red. Qualified merged trees
match; fresh post-merge fixtures and targeted checks passed. Stage 3's completed
live scope was not repeated. Their PRs retain exact heads, results and resumable
environment requirements. Proposal Core publication proceeds as coherent A/B/C
checkpoints rather than a giant PR; Stage 5 checkpoint CI remains pending.

This twelfth slice is implemented, automated verified, independently reviewed and
locally integrated by the commit containing this record. Next: basic persisted
review findings with exact saved evidence, then multilingual implementation and
qualification. Download permission remains pending. Transfer requirements remain
pinned Rust 1.98.1, Apple Silicon/Command Line Tools, cached locked dependencies,
protobuf, Bash/Python 3, fresh canonical synthetic data/TMPDIR and an unlocked
session for owner GUI acceptance. Original data and credentials are not inputs.


## Thirteenth slice: basic retained review findings

Baseline `main@b9b2338d877be900592fece56eb47988974c122b`; preserve the owner's
AGENTS.md edit. Add a narrow operational finding record in existing brn.sqlite,
with additive V9 migration, immutable creation binding, exact saved evidence,
Open/Resolved/Dismissed lifecycle and bounded pages. Findings are tentative work,
not knowledge authority or another proposal workflow. Closing changes only queue
state; correcting Markdown remains an exact approved proposal. Stage 10 owns
semantic detection, scheduling and web maintenance; this slice supplies basic
records and two existing deterministic producers, not an AI service/framework.

Freeze FindingOrigin to IdentityAmbiguity(note UUID) or UnresolvedLink(saved path,
source hash, destination, occurrence start). CaptureFindingRequest binds a nonnil
finding UUID plus that origin. FindingDraft retains request, VaultRecord, bounded
title/summary and 1–64 exact FindingEvidence entries (SourceVersion, optional
observed UUID, optional UTF-8 quote/range). Identity ambiguity needs at least two
distinct saved paths with that UUID; unresolved-link occurrence/definition quotes
share the exact source proof. Quotes are at most 16 KiB each; all retained work is
at most 1 MiB before JSON expansion. Strict validation/hash/row binding precedes
use. Record version 1 is Open; one exact-stamp Resolve/Dismiss advances to terminal
version 2. Identical creation/closure replay returns the retained current record;
changed payload or stale competing closure refuses atomically. No source is
re-anchored or substituted later. Evidence inspection retains old proof and reports
fresh drift/unavailability separately.

App captures fresh evidence through existing coordinated saved-source APIs and
unresolved-change fences. Duplicate identity and unresolved saved links produce
real bounded findings without changing vault/proposals/credentials. Exact request
replay precedes fresh vault requirements. Listing/detail/closure remain available
without current vault evidence. AppWorker owns admitted create/closure and drains
shutdown. CLI uses only shared workflow DTOs for capture/list/show/close; native
queue controls follow as the next bounded adapter slice. Preserve source/history
access and empty-note proof without fabricating quotes or stable identity.

Acceptance: meaningful strict shape/byte/range tests, creation/closure replay and
conflicts, transactional rollback, V8 upgrade/reopen/backup restoration preserving
existing work, bounded fresh pages and open counts, real issue capture with exact
Unicode/BOM/CRLF evidence and unchanged vault, changed/missing source retention,
worker drain and CLI JSON/human parity. Independent review with verified fixes,
fresh default gates and concise manual scenario precede local integration. Root
owns workflow/admission/coherence; a bounded helper implements only the frozen
store DTO/record seam and its tests. No original-data operations or network work.

The Store refusal-stub lifecycle RED became 13 focused tests. Independent review
found one valid defect: case-folding distinct evidence paths refused legitimate
case-sensitive saved-file proofs. Root reproduced the public WorkStore RED,
changed only exact-path uniqueness and the old duplicate-path fixture, and added
a full retained-proof/reopen regression. Store findings now pass **14** tests.
The workflow refusal stubs produced **0 passed / 5 failed**, then the actual
App/worker suite passed; final **8** cases include original reference/definition
quotes, Unicode excerpt boundaries, whole fingerprints, same-byte inode drift,
missing/foreign-vault evidence, current fences, closed replay and shutdown drain.
Independent workflow review passed **7** then-existing cases and found no defect.

CLI process tests exposed a real scanner conflict between global --version and
command-local closure --version N. Declared local options now take precedence;
standalone/global banners retain no-storage behavior. Helper qualification passed
**71 selected tests** and CLI Clippy; independent Store/CLI review passed **14
Store + 2 parser/preflight + 4 process tests**, no remaining actionable findings.
A malformed synthetic reference fixture was corrected; it was not a product
defect. No review expanded detection or authoritative-write scope.

Fresh final root macOS arm64 / Rust 1.98.1 locked/offline gate passed retirement,
format/build/all-target Clippy, **935 workspace tests / 0 failed / 3 ignored**,
and **52 end-to-end assertions**. The two ignored private crash entries remain
exercised by subprocess matrices; the unchanged case-sensitive link adapter
retains its prior explicit APFS qualification. Native desktop with
native-ui,native-retrieval,native-test-support passed **201 / 0 / 0**; both native
all-target Clippy variants and shipping native build passed. Only the known
upstream block 0.1.6 future-compiler notice remains. Fresh shipping AppWorker
startup/restart passed twice with exact synthetic bytes and zero credential files.
No model download/inference, provider call or original-data inspection occurred.

This thirteenth slice is implemented, independently reviewed, automated verified
and locally integrated by the commit containing this record. The Store/workflow/
CLI READMEs retain the active contract and reproducible capture/restart/drift/
closure scenario; owner/manual acceptance remains pending. Native queue controls
are the next bounded adapter slice, followed by multilingual implementation/
qualification. Stage 5 remains unfinished; its checkpoint CI is pending. Stage 4A
PR #19 merged as `73410a3` after all exact-head macOS/shared checks passed; Windows
retains the known Unix API failure and overall run is red. Post-merge tree equality,
8 application tests, 52 fixtures and startup/restart passed. Stage 4B publication
is proceeding separately. Transfer requirements remain pinned Rust 1.98.1/Apple Silicon/
Command Line Tools/cached dependencies/protobuf/Bash/Python 3, canonical synthetic
data/TMPDIR and an unlocked session for GUI acceptance. Original data and
credentials are not transfer inputs.

Changed-document validation passed **72 local file/fragment links** (literal
code examples excluded), final format and diff checks. No owner changes were
staged; AGENTS.md remains the owner’s independent edit.

## Fourteenth slice: native basic findings queue

Baseline `main@46d60289e9ea6cf78003feb51fe08a843ed440f5`, incorporating
Thirteenth `263fe67` and the published Stage 4A portability fixes; preserve owner
AGENTS.md. Add Needs Review access through existing guarded document navigation,
bounded filtered 25-record pages, selected complete retained evidence, separate
fresh proof inspection and exact direct Resolve/Dismiss. Explicit Keep finding
controls use only the already-inspected saved-link/ambiguous-source observation;
the shared workflow recaptures and rechecks it. No Markdown correction, proposal
admission, semantic detector, provider, scheduling or new persistence/framework.

Freeze native state over existing AppWorker commands/DTOs: correlated view/page/
selection generations; stale closed/filter/selection replies cannot overwrite
current state/errors. Captured requests and acknowledged mutation outcomes remain
retained across navigation, with exact explicit retries and no automatic mutation
retry. A current successful mutation refreshes only the visible queue observation;
closing never discards a pending outcome. Selected full proof stays in one
persistent read-only/copyable editor, with controls reachable at 480×480. Historical
queue/detail/closure work without a vault; fresh source unavailability remains
explicit. Navigation preserves unacknowledged note/review/draft input.

Acceptance: real worker saved-issue capture/restart/inspect/closure through native
state, exact request/record correlation and stale replies, complete evidence/Copy,
filter/paging/error retention, same-byte inode drift, absent vault history, guarded
navigation and actual headless widget tests at 480×480. Meaningful RED/verification,
independent read-only review with validated fixes, fresh relevant default/native
gates and concise manual scenario precede integration. Root owns behavior/UI/
integration; a bounded helper implements only the fixed state seam and its tests.

Fourteenth's real-worker refusal stubs produced **0/5**, then eight state tests
passed, including explicit lost-outcome/missing-vault retry and full immutable
receipt binding outside the active filter (each separately reproduced RED).
Independent state review found no actionable defect and passed **8/0/0**.
Native review reproduced a valid refused-New-proposal navigation defect: early
queue invalidation left the visible pane unusable. The actual TestAppContext RED
was corrected by invalidating only after successful Draft admission. Independent
final joint review and **4 widgets / 0 failed / 0 ignored** passed at 480×480,
covering full persistent read-only proof/Copy, original quote Copy, exact failed
requests without selection/vault, explicit retry and retained navigation input.
Historical capture and terminal closure receipts remain separate observations;
a proposed read-regression issue was rejected after tracing generation guards.

Fresh final root macOS arm64 / Rust 1.98.1 locked/offline checks passed retirement,
format/build/all-target Clippy, **943 workspace tests / 0 failed / 3 ignored**
and **52 end-to-end assertions**. A clone-on-Copy test lint was minimally fixed
before the fresh gate. Native desktop with native-ui,native-retrieval,
native-test-support passed **213 / 0 / 0**; both native Clippy variants and
shipping native build passed. Fresh shipping startup/restart passed twice with
exact synthetic BOM/CRLF/Unicode bytes and zero credential files. Only the known
upstream block 0.1.6 future-compiler notice remains. No asset download/inference,
provider call or original-data inspection occurred.

This slice is implemented, independently reviewed, automated verified and locally
integrated by the commit carrying this record. Desktop README provides the
reproducible synthetic capture/inspection/drift/restart/Resolve/Dismiss/input-guard
scenario; GUI/IME/accessibility and owner acceptance remain pending. Stage 5 is
unfinished: the next bounded slice is multilingual model implementation, with
actual asset-download permission still pending. Stages 1–3 and 4A/B are published;
4B PR #20 merged as d7a1a72 after exact-head macOS/shared checks passed. Windows
retains known Unix metadata failures and the overall run is red. Post-merge tree
equality, **12 CLI Activity/Undo/Repair tests + 52 fixtures** and startup/restart
passed. Stage 4C publication follows separately. Transfer requires Apple Silicon/
Command Line Tools, pinned Rust 1.98.1, cached locked dependencies, protobuf/
Bash/Python 3, canonical synthetic data/TMPDIR and an unlocked GUI for acceptance;
original data/credentials are not transfer inputs. Owner AGENTS.md is preserved.

Changed-document verification passed **38 local file/fragment links**, final
format and diff checks. No owner changes were staged.

## Fifteenth slice: pinned multilingual retrieval profile and explicit installation

Baseline main@01e7c2f858b71103c98e75dda4a81b3d142e03df; preserve owner
AGENTS.md. Existing LocalEmbedder/Embedder/NoteIndex/shared AppWorker seams remain
unchanged. Update only the explicit installer to the five pinned multilingual
MiniLM-L12 quantized assets (135392488 bytes; revision
2c4055b12046f11709e9df2c122e59ffbdc2f900). The larger tokenizer is LFS-backed
and needs its content SHA256, not pointer/git-object hash. No model assets or
inference are authorized by implementation; the scoped download question remains
pending. Public primary metadata investigation made no asset requests.

Freeze a private new profile: mean pooling,384dimensions,max128tokens,FastEmbed
Static quantization, distinct full-bundle-derived identity including profile.
Recognized new ONNX bytes require every pinned companion; reject mixed new bundles
before ONNX initialization. Other explicit/saved directories retain the exact
legacy loader behavior and identity; no automatic model/provider/account fallback.
Fresh default loader/download target is models/multilingual-minilm-l12-v2; saved
or explicit paths keep precedence and old asset directories remain untouched.
Scope new consent to this pinned revision; old approved/declined settings remain
retained and do not authorize or suppress the new asset offer. Startup/search never
download, and each install/retry still needs a fresh explicit action. Existing
vector identity+dimension guards and tool-drain activation remain authoritative.

Acceptance: synthetic pinned-manifest/size/hash refusal and installer interruption/
reuse/cancel/occupied tests; private profile/bundle checks without real ONNX;
current/legacy consent separation, exact prompt cost/source/path, saved/explicit
precedence and no-network restart; unchanged384/differentidentity vector discard
and stale-reader refusal; fresh relevant native/default gates, independent
read-only review and concise manual scenario. Lead owns loader/workflow/coherence;
a bounded helper may implement only frozen asset pins/byte-verification tests.
Actual EN↔ET paraphrase/inflection/distractor/scope/restart/rebuild/tool+CLI parity,
exact quotes and long-tail passage qualification remain pending authorized assets.
128-token truncation is explicitly a quality risk to measure, not an architecture
redesign trigger. Conversational response-language qualification stays separate.

Fifteenth implementation and independent read-only review are complete. Public
pinned metadata independently matches all five lengths/digests, including tokenizer
LFS SHA256. Meaningful RED→GREEN checks cover byte pins, profile identity, old
consent separation and actual CLI destination submission. Review reproduced the
CLI legacy-folder defect; the corrected shared default preserves explicit targets
and legacy bytes. No remaining actionable review finding. The existing explicit
local-model test now accepts both retained profiles; a new pinned-only ignored
six-query EN/ET smoke test checks ranking and finite normalized384 vectors. It
compiled without executing any asset load/inference; wider qualification remains
pending, including128-token long-tail quality and response language.

Fresh final macOS arm64/Rust1.98.1 locked/offline checks: retirement, format,
workspace build/all-target Clippy,945workspace/0failed/3ignored and52fixtures;
28native retrieval/0failed/0ignored;137native workflow/0failed/2ignored;
213combined-native desktop/0failed/0ignored, native retrieval/CLI and both desktop
Clippy variants, shipping native desktop/CLI builds. The two workflow private
crash entry points are exercised by subprocess matrices. Shipping AppWorker
startup/restart passed twice with exact synthetic BOM/CRLF/Unicode and zero
credential files. Known upstream block0.1.6 future-compiler notice remains.
No downloads, provider calls or original/private data inspection occurred.

Manual acceptance: launch native retrieval with fresh synthetic data and no
model; verify the exact pinned source,135392488byte/~129MiB offer and separate
multilingual folder. Decline, restart, and search keyword-only without network;
Settings still permits a fresh explicit Download. Only after scoped acquisition
permission, install in a fresh owned destination, run the ignored bilingual smoke
and qualify full mixed-language retrieval, exact quotes, scopes, restart/rebuild,
tool/CLI parity and long passages. GUI/owner/model qualification is pending and
does not block Actions foundations. Preserve prior legacy data/assets.

Publication checkpoint: Stage4C PR21 merged6601374996ecba7d461403aa6e892f1273bed28e;
exact18a315b run37190429533 passed MacCore/UI/Retrieval+UbuntuSharedCore. Windows
Unix-metadata failure leaves overallCIred. Merged tree equality,16widgets+52fixtures,
shipping build and2startup/restart runs passed. Stage5A identities/scopes is next;
Stage5 remains unfinished. Mac mini requires pinnedRust1.98.1/locked caches,
AppleSilicon/CLT, protobuf/Bash/Python3 and canonical synthetic TMPDIR outsideGit;
actual GUI needs unlockedMac, assets require pending permission. No owner changes
will be staged.

## Sixteenth slice: current-question response language

Baseline d583b051ef1d4b72cc4e3d6cb6d5ac6d811006b1. Vision§38 requires mixed
English/Estonian conversation and normally answering in the current user's
language unless asked otherwise. Add this instruction and preservation of original
source-quote language only to existing Answer/AnswerWithEffort preambles. Rewrite's
strict captured JSON protocol, tool/transport selection and all durable boundaries
remain unchanged. No language detector, setting or extra AI call.

Acceptance: meaningful synthetic actual-Rig transport RED→GREEN checks on the
selected ChatGPT/Copilot routes retain exact EN/ET questions, explicit output-language
requests and original source quotes, and deliver the same instruction. Targeted
adapter verification, independent read-only review, fresh integrated gates and
concise manual scenario. A bounded helper may implement only the fixed preamble
and request tests; lead owns integration. Real answer-language/provider quality
needs separately authorized calls and remains pending; no live calls are implied.

Sixteenth implementation passed meaningful2-test RED→GREEN; independent read-only
review reran2tests/0failed and found no actionable defect. Actual Rig serialization
covers both Ask APIs,three explicit routes,four EN/ET/default-or-explicit questions:
24mocked sessions/48scripted completion requests. Model/effort/history/question,
exact BOM/CRLF source quotes and continuation input stay intact; Rewrite's strict
JSON contract excludes the new instruction. Mocked replies establish transport/
byte correctness, not provider compliance. No network/account/assets/private work.

Fresh final macOS arm64/Rust1.98.1 locked/offline verification passed retirement,
format/workspace build/all-target Clippy,947workspace/0failed/3ignored+52fixtures;
86capability-library+1example/0failed and feature Clippy;213combined-native tests/
0failed/0ignored, both native Clippy variants and shipping native build. Earlier
unchanged retrieval/model-profile qualification remains Fifteenth evidence;
actual assets/inference and answer-language/owner qualification remain pending.
Manual acceptance after separately authorized connection: ask an Estonian
question about English/Estonian synthetic notes, then request English explicitly;
verify answer language and retain source quotes verbatim. No extra call is implied
by these instructions. Next safe implementation is Actions/dashboard in existing
WorkStore and narrow typed lifecycle, with mixed recovery qualified explicitly.
Stage5A PR22 merged a9f838295ea905bf25d05953fe03d02a2092dff7 after exact4895916
run37191800152 passed MacCore/UI/Retrieval+UbuntuSharedCore; Windows Unix metadata
failure leaves overallCIred. Merged tree equality,16CLIidentity/inventory/scope
tests+52fixtures and2shipping startup/restart runs passed with exact bytes and
zero credentials. Sixteenth shipping startup/restart also passed twice with exact
synthetic bytes and zero credentials. Later Stage5
publication remains sequential; Stage5 whole remains unqualified for real models.

## Stage5A publication checkpoint — identities and scoped evidence

Baseline e9179eb merges reviewed first-four slices through4fb2763 with qualified
Stage4C main6601374. Identity preparation, fresh UUID resolution, current/source/
history/all filtering, scoped read tools and native read-only evidence browsing
are this checkpoint; later Stage5 slices remain separate. Independent read-only
review verified60Stage5-only+11incoming-only exact paths and all four source
merges, preserving platform/subprocess guards, CI and complete prior evidence.
No actionable defect. Both YAML files parse, with no conflicts/unrelated changes.

Fresh macOS arm64/Rust1.98.1 locked/offline checks passed765workspace/0failed/
2ignored,52fixtures,retirement,format/build/all-target Clippy;155combined-native/
0failed/0ignored, both native Clippy variants and shipping desktop/CLI builds;
84capability-library+1example;132focused native workflow/0failed/2ignored.
Shipping startup/restart passed twice with exact BOM/CRLF/Unicode bytes and zero
credentials. Private ignored crash entries remain exercised by subprocess matrices.
Exact latest-head applicable Mac/shared CI and post-merge verification are pending;
Windows platform results remain visible. Only upstream block0.1.6 future warning.

Manual acceptance: with fresh synthetic current/source/history/archived notes,
verify default current queries exclude sources/history; switch scopes and inspect
full exact archived evidence read-only, Copy exact bytes, preserve unsaved typing
across scope changes and refuse overwriting archived notes. Prepare/approve a UUID,
move its note and resolve by UUID; duplicates/incomplete inspection must not guess.
Owner/GUI/IME/accessibility and actual bilingual inference remain pending. Next
publish durable provenance/timestamps, then links/relationships and findings; do
not claim all Stage5 complete. Mac mini: AppleSilicon/CLT,pinnedRust1.98.1,cached
lockfile dependencies,protobuf/Bash/Python3,canonical synthetic TMPDIR outsideGit,
unlockedGUI for acceptance. No assets/live/original-data/release actions occurred.

Stage5A PR22 publication completed at exact4895916/run37191800152 with
MacCore/UI/Retrieval+UbuntuSharedCore passed; Windows Unix metadata failure keeps
overallCIred. Mergea9f838295ea905bf25d05953fe03d02a2092dff7 has the qualified tree.
Fresh16CLIidentity/inventory/scopes+52fixtures and2shipping startup/restart runs
passed with exact synthetic bytes and zero credentials. Lead-tree integration
changes only status/evidence; all sixteen knowledge slices stay byte-identical.

## Stage5B publication checkpoint — provenance and session timestamps

Baseline db9aee379b34527564ef48e9e0815fb30202eb94 integrates reviewed provenance,
native exact source inspection and session/turn timestamps throughbaf3bee with
qualified Stage5A maina9f8382. No later Stage5 behavior is included. Independent
read-only integration review verified36B-only+13incoming-only paths, both source
overlaps and all retained evidence; no actionable defect. CI YAML parses.

Fresh macOS arm64/Rust1.98.1 locked/offline retirement,format/build/all-target
Clippy passed;809workspace/0failed/2ignored+52fixtures;168combined-native/0failed/
0ignored, both native Clippy configurations and shipping desktop/CLI builds;
136focused native workflow/0failed/2ignored (`--lib --test models --test ai_tools
--test knowledge_provenance`). Shipping startup/restart passed twice with exact
BOM/CRLF/Unicode bytes and zero credential files. Crash-entry ignores remain
exercised by subprocess matrices. Only upstream block0.1.6 future warning remains.
Exact latest PR applicable Mac/shared CI and post-merge checks remain pending.
Logs/ownership metadata: /private/tmp/brn-v1-stage5b-checkpoint-46w97ttl.

Manual acceptance: in disposable managed notes, approve exact source quote and
citation metadata, inspect the full original through Source, change the source
and verify a fresh stale outcome; Copy must retain exact original quote bytes.
Inspect recorded session/turn times, restart without new chat and verify unchanged
activity; upgraded legacy unknown times stay explicit unknown. GUI/owner acceptance
is pending. This publishes implemented/automated verified behavior, not live
provider or actual bilingual model qualification. Next publish relationships and
findings, then language; safe Stage6 implementation continues independently.
Mac mini requirements: AppleSilicon/CLT, pinnedRust1.98.1, cached locked dependencies,
protobuf/Bash/Python3, canonical owned synthetic TMPDIR and unlockedGUI acceptance.
No assets/live/original-data/release actions occurred.

Stage5B initial exact57c71f5 run37193439338 passed both macOS native lanes but
Ubuntu Clippy rejected the unconditional `provenance_cli::ok` test helper. Its
actual log reports unused `ok`; all callers are inside one cfg(macos) test.
Independent read-only correction review confirmed that a sole matching helper
guard preserves both shared preflight tests and found no other B-test dead helper.
Fresh corrected3CLI provenance tests,workspace all-target Clippy,format and diff
passed; applicable CI must qualify the new exact head. Windows retains the known
Unix API failure. Initial macOSCore completion is not relied on for integration.

Stage5B PR23 merged d48654098f79c8b4a6b13982650c258245b2d800 after exact
2d0993f935e3af309d2e97c6f097f28a0b0ec39a run37193690701 passed all three
Mac lanes and UbuntuSharedCore. Windows Unix APIs fail,leaving overallCIred.
Merged tree equality,14CLIprovenance/Storetimestamp tests+52fixtures and two
shipping startup/restart checks passed with exact bytes and zero credentials.
Stage5C now integrates this qualified baseline; final incoming-delta review and
current checkpoint CI remain required.

## Stage5C publication checkpoint — relationships and exact link preparation

Baseline71a27a2aa5a4280c87dcda6eb58d295c7ede6a72 integrates reviewed slices8–12
throughb9b2338 with qualified Stage5B d486540. It retains approved Markdown links,
disposable evidence-backed explicit/inferred edges, native relationship inspection
and exact native/headless link-to-proposal preparation; basic findings/language
remain later checkpoints. Initial independent merge review verified72C-only+
13incoming-only paths and both source overlaps with no defect. Final B-delta review
verified sole exactincomingMac helper guard,205othercrate paths unchanged and
full12-slice/A/B evidence preserved, with no actionable defect.

Fresh macOS arm64/Rust1.98.1 locked/offline retirement,format/build/all-target
Clippy passed;907workspace/0failed/3ignored+52fixtures;201combined-native/0failed/
0ignored, both native Clippy configurations and shipping desktop/CLI builds;
21native-retrieval/0failed/0ignored (`--lib --test model_download --test note_edges`)
and164focused-native-workflow/0failed/3ignored (`--lib --test models --test
knowledge_links --test knowledge_relationships --test knowledge_link_preparation`).
After sole incomingtest guard, fresh3CLIprovenance tests,workspace Clippy and
format passed. Two shipping startup/restart checks preserve exact BOM/CRLF/Unicode
current/archive bytes with zero credential files. Ignored private crash entries
remain exercised by subprocess matrices; actualcase-sensitive volume qualification
was separately recorded in slice10. Only upstream block0.1.6 future warning remains.
Exact latest PR applicable Mac/shared CI and post-merge verification are pending.
Logs/ownership metadata: /private/tmp/brn-v1-stage5c-checkpoint-ajcwhli7.

Manual acceptance: in fresh managed current/archive fixtures,inspect explicit and
inferred relationship origins/proofs,move a target and resolve by UUID,change its
bytes and observe fresh evidence,duplicate an ID and observe ambiguity. Prepare a
link in the native Replace form;full immutable target/source proofs and exact text
must reach review unchanged,with no Markdown effect until Approve. Edits/separate/
retry preserve full bindings;stale replies retain newer typing. Rebuildindex and
restart. GUI/IME/accessibility and owner acceptance remain pending. Next publish
NeedsReview findings and language; safe Stage6 implementation continues.
Mac mini: AppleSilicon/CLT,pinnedRust1.98.1,cached locked dependencies,protobuf,
Bash/Python3,canonical owned synthetic TMPDIR,unlockedGUI for acceptance.
No assets,live/original-data/release actions occurred.

Stage5C PR24 merged2483b31f38a2b941ab71b7fba5449519da600e82 after exact
716aede/run37194454005 passed MacCore/UI/Retrieval+UbuntuSharedCore. Windows
Unix APIs fail,overallCIred. Merged tree equality,10CLIrelationships/links+
52fixtures and two startup/restart checks passed with exact bytes,zero credentials.
Stage5D integrates this qualified baseline with reviewed basic Findings/NeedsReview.

## Stage5D Findings/Needs Review checkpoint — 2026-10-04

Baseline9cb0fdea213e2d803ef55dcda8251b047489f966 combines reviewed14-slice
01e7c2f with qualified Stage5C2483b31. Independent merge review found no defect;
all exclusive paths,source combinations and full reviewed evidence were retained.
A separate concrete P1 recovery finding was accepted: unknown indexed state or
31-byte creation/record digest made quick_check restore older Open/v1 work over a
legitimate Resolved/v2 receipt. All3 public probes reproduced it. A meaningful
startup regression was RED; the physical-main/invalid-newest-backup regression
passed before correction and protects the recovery boundary.

Keep V9 DDL compatible. Existing supported brandedV9+ Findings validation now
precedes quick_check. Semantic-invalid main data refuses before any move; the
internal checked result lets backup restoration skip invalid candidates. Only
physical SQLite corruption codes take recovery. Foreign/newer handling and
post-migration validation remain. Independent correction review found no defect,
passed20focused startup/Findings tests and4 public probes (3semantic plus real
Findings B-tree corruption), retaining terminalv2 and exact backup bytes.

Fresh macOSarm64/Rust1.98.1 locked/offline retirement/format/build/all-target
Clippy passed;945workspace/0failed/3ignored+52fixtures;213combined native-desktop/
0failed/0ignored;143focused native-workflow/0failed/2ignored (`--lib --test models
--test findings`); both native Clippy configurations and shipping desktop/CLI
builds. Two shipping startup/restart checks retained exact BOM/CRLF/Unicode vault
bytes,V9 and zero credential files. Only upstream block0.1.6 future warning.
Logs/ownership metadata: /private/tmp/brn-v1-stage5d-checkpoint-yry8r21v.
FocusedRED/Green logs: /private/tmp/brn-findings-semantic-regression-{red,green}.log.
Exact latest-head Mac/shared CI,PR/merge/post checks remain pending.

Manual scenario: with fresh managed duplicate-UUID and unresolved-link fixtures,
capture findings,inspect complete retained proof and separate fresh source status,
move/change evidence and confirm original quotes survive. Resolve/Dismiss the
identified exact version,inspect filtered pages/full closed work,restart and
rebuild the disposable index. No Markdown changes occur. Explicit retries and
stale replies preserve current selection and typing. GUI/owner/IME/accessibility
acceptance remains pending; language/model/provider qualification follows.
Macmini requires AppleSilicon/CLT,pinnedRust1.98.1,cached locked libraries,protobuf,
Bash/Python3,canonical owned syntheticTMPDIR and unlockedGUI for acceptance.
No providers,assets,original/private-data or release actions occurred.


Stage5D combined baseline844dc92e6e569bbfbd7de7e44ac7d6099e8bf64f merges qualified
PR25/8295326; independent preservation review found no code/authority defect.
Findings code/tests match31ca2e59; client boundary code/authorities matchPR25.
Fresh locked/offline shared gate passed946workspace/0failed/3ignored+52fixtures;
213combined-native desktop/0failed/0ignored;154focused native-workflow/0failed/
2ignored (`--lib --test models --test findings --test library --test ai_tools_scopes`);
both native Clippy configurations and shipping desktop/CLI builds passed. Two
shipping startup/restart runs retained exact synthetic bytes,V9,zero credentials.
The earlier945/143 evidence describes pre-amendment31ca2e59. Client amendment
PR25 passed exact-head Mac3+UbuntuShared,merged8295326 and passed tree equality,
3focused tests+52fixtures/startup2; Windows Unix APIs failed,overallCIred.
Current D exact-head PR/CI/merge/post remains pending; GUI/owner/IME/accessibility,
actual inference and provider language qualification remain separate.


Stage5D PR26 mergedb0e93fb81cf702da5d4957ed984305ed8852683f after exactfc98b0d/
run37199082097 passed MacCore/UI/Retrieval+UbuntuSharedCore. WindowsCore failed
on existing Unix-only auth/Store APIs;overallCIred. Merged tree equality,16Store+
8workflow+4CLI Findings tests,52fixtures and two shipping startup/restart checks
passed,V9,exact bytes,zero credentials. Main37199659960 passed Mac3+UbuntuCore/UI;
Ubuntu native retrieval failed6pass/3fail on unchanged unsupported exclusive-install
expectations;Windows3failed. Independent job analysis found no new sharedMac
defect. Non-Mac-specific failures remain informational under owner policy.

## Stage5 language implementation checkpoint — 2026-10-04

Baselinebc8a724eda647b42f227ed9bbc3886e048125859 retains reviewed16slices including
pinned multilingual installation/profile and current-question Ask language. It
merges qualified Findings/client-boundary candidatefc98b0d (now PR26 mergedb0e93fb).
Independent full preservation review verified all exclusive/overlapping source
blobs/modes,retained full plans/publication evidence and92local doclinks,no defect.
Three documentation conflicts were composed; source merged without conflict.

Fresh macOSarm64/Rust1.98.1 locked/offline retirement/format/build/all-target
Clippy passed;950workspace/0failed/3ignored+52fixtures. Combined nativeDesktop
passed213/0/0,both native Clippy configurations and shipping desktop/CLI builds.
Native retrieval `--lib --test model_download --test note_edges` passed25/0/0;
`--test note_embeddings` passed10/0/0;local_embedder missing-model check1/0/0
compiled both real-model tests without running them. Focused native workflow
`--lib --test models --test library --test ai_tools_scopes --test knowledge_scopes`
passed155/0failed/2ignored. Capability-spike all-target tests passed86library+
1example;feature Clippy passed. Two shipping startup/restart runs retained exact
BOM/CRLF/Unicode vault bytes,V9 and zero credential files. Only upstream block0.1.6
future warning. Logs/ownership:/private/tmp/brn-v1-stage5-language-9yd45cpj.

Actual asset acquisition/ONNX inference,EN↔ET quality/long-tail/scoped parity and
live response-language compliance remain pending. Synthetic vectors/transport
checks do not establish them. GUI/owner/IME/accessibility remains separate. No
assets,live provider,original/private-data or release operations occurred. This
publishes implemented/offline-qualified behavior; wholeStage5 is not complete.
Manual scenarios and Macmini requirements remain in slices15/16 above. Exact
latest-head applicable CI/PR/merge/post qualification remains pending. Next
publish the checked Stage6 Action foundation/review and continue joined application.


Language PR27 merged42722526009b6c986271afc75680828bc5287c02 at2026-10-04T12:17:32Z
after exactcee0144a05de4e52e8b4d7ce536eb9ce41e0a102/run37201040507 passed MacCore/
UI/Retrieval+UbuntuSharedCore. Windows Core failed before tests on known Unix-only
APIs; overallCIred. Merged source tree equals qualified candidate. Post-merge
15native retrieval+7native workflow model+2Rig Ask language tests passed24/0/0,
52fixtures and two shipping startup/restart checks passed,V9,exact bytes and zero
credentials. Logs:merged-post-*.log,qualified-startup-fptu5yc4 under the language
owned parent above. Main37201599486 completed/failure: Mac3+UbuntuCore/UI passed,
Ubuntu native retrieval failed10pass/3fail/0ignored on the same three unsupported
exclusive-install expectations as PR26;Windows3failed on Unix-only APIs before
tests. Independent comparison confirmed identical installer/platform branches
and failing test bodies; no sharedMac/language defect. Logs/hashes:
/private/tmp/brn-pr27-main-ci-review-lfx9dx_6. Model/live/native owner gaps remain
pending; Stage5 is not complete. Safe Stage6 Action implementation continues.

Native Luna qualification,2026-10-04,baseline0243c5d,used the owned local bundle
and synthetic data/vault under/private/tmp/brn-v1-native-luna-3xaxva10. Reproduce:
place distinct `orchard-827` text in managed Current/Source/History notes; search
each scope and open its result. Current exposes editor/Save; Source/History expose
exact saved evidence/copy without Save. Explicitly Save `café 🌱` in the BOM/CRLF
current fixture: disk retained BOM,all5CRLF,zero bare LF and exact Unicode.
Type two unsaved markers,acknowledge Flush/retry recovery,complete any displayed
quit guard,verify full process exit,then relaunch/open current.md. Native UI
restored both unsaved markers while disk remained the exact126-byte saved version;
fresh Current search returned it. Final Quit left no owned process. An initial
apparent restart had not completed the quit guard; the corrected full restart
supersedes that observation. AX worked; ScreenCaptureKit-3811/-3812 prevented
screenshots. Native scope/Save/acknowledged-recovery qualification passed; visual
layout,IME,broader review/Undo and owner acceptance remain pending. No provider,
model asset or original/private data access occurred. Exact temporary evidence:
logs/native-evidence.md under the owned parent; durable steps/results are above.


## Current subscription catalog and Luna qualification

Owner amendment2026-10-04: BRN's picker uses the current subscription catalog;
exact GPT-6Luna (`gpt-6-luna`) is used for live app/provider tests. Development/
review may use Sol/Luna,neverAstra. Baseline actual mergedb5f7ce8; catalog source
reviewed at0243c5d and joined unchanged with current Actions throughd344dc6.
Use pinned Rig0.43 authentication/HTTP and model request encoding; decode the
Codex models/slug/visibility/priority envelope privately. Supply real BRN package
client_version and resolved Rig account/caller headers,without SDK/auth redesign.
Return validated visible options in stable provider order; failures supply no
substitute. Preserve exact saved selections/history; catalog refresh never chooses
a model. Current Copilot new-use membership checks remain. UI hides obsolete
picker options when explicit Connect/Disconnect/Models begins. No startup discovery.

Acceptance: real synthetic transport proves authenticated exact route/headers,
malformed/duplicate/status/network/cancel behavior,visibility/order and API-flag
independence; exact Luna Responses/effort/history/tool continuation/refusal; saved
choice readability despite rediscovery; process/native presentation parity without
credential effects. Independent full review found no defect. Getter regression
first failed0/1,then all13App tests passed; native picker regression first failed
0/1,then passed. AI91default+91capability tests and all-targetClippy passed.
Joined shared verification caught two obsolete CLI expectations; corrected8real
process tests preserve old choices,refuse malformed identifiers before authority,
and refuse new stale Copilot use with explicit effort. Independent correction
review found no defect. A formatting invocation used edition2024 for the2021CLI;
Cargo formatting corrected it. Fresh full shared/native qualification is pending.

Manual/live scenario: use fresh owned explicit synthetic data/vault/credentials;
human Connect ChatGPT,explicit Models refresh,inspect actual options,choose exact
Luna and Low. Ask for one saved synthetic Current fact; inspect exact saved/source
citations and recorded model. Create a tiny synthetic review draft and use High
Rewrite; it remains review work until exact approval. Reopen and refresh without
changing selection or choosing a fallback. If Luna is unavailable or refused,
record that exact result and stop that route. Current owner authorization covers
this bounded fresh qualification: up to two short Ask/Rewrite probes,max18completion
requests under the existing eight-tool-round limits,plus up to two explicit catalog
requests. Stop on quota/refusal. No old credentials,private vault,model assets,
purchase or release action. Native access is currently pending: latest tool says
Mac locked despite owner's unlock message; an unlock/awake question is pending.
Actual subscription/inference/effort/response-language/Rewrite quality and owner
acceptance remain unqualified. Continue safe Actions implementation independently.


Fresh final joined macOSarm64/Rust1.98.1 locked/offline qualification passed
1019workspace/0failed/3ignored+52fixtures,retirement/format/build/all-targetClippy;
165focused-native-workflow/0failed/2ignored;214combined-native-desktop/0failed/
0ignored;both native desktop Clippy configurations,featureCLIClippy and shipping
native desktop/CLI builds. Shipping startup/restart2 passed,V10,exact synthetic
BOM/CRLF/Unicode bytes,zero credential files (qualified-startup-s7r9ukpw). The
reviewed AI91default+91capability gates remain valid for unchanged source. Final
43local Markdown links and diff checks passed. Logs:catalog-*-final.log under
/private/tmp/brn-v1-stage1-checkpoint-s3nawyyb; early catalog-shared-final.log records
obsolete CLI expectations,while catalog-shared-corrected-final.log is finalpassed
evidence. Initial CLIformat mismatch corrected before this fresh full pass.
Provider/live/native owner acceptance and exact-head CI/publication remain pending.


Subscription checkpoint PR31 merged13895d905bd18d0e7615d7623325304f9126cd9d
after exactfcc5f633/run37210863902 passed Mac3+UbuntuShared. Windows failed
22Unix errors before tests; independent source/log review found no shared catalog
defect. Merged treeafef4427 equals qualifiedsource. Post91AI+8CLI+13App+1UI=113
tests/0failed,52fixtures,startup2V10/exactbytes/zero credentials passed
(qualified-startup-r02clweg under/private/tmp/brn-v1-subscription-catalog-61kz1mnj).
Main37211590271 completed5success/4failure:Mac3+UbuntuCore/UI passed;Ubuntu
installer10pass/3fail,WindowsCore/UI22each and nativeRetrieval14Unix errors remain.
Actual source/log comparison retains overallCIred/platform qualification gaps.

## Native dialog qualification correction

Baseline13895d9. Unlocked CUA captured the older synthetic workspace visually,then
normal Quit and focused process check proved full exit; vault hashes/zero
credentials unchanged. Fresh fixtureCurrent search/read and savedDraft were
reachable,but Settings button/menu/Cmd-comma never appeared. PinnedGPUI0.6.6
Root manages dialogs but requires its client to render the dialog layer. Add
one outer DesktopWindow view rendering Desktop plus Root::render_dialog_layer,
avoiding dialog builders reading Desktop during its mutable render borrow.
Observe Desktop notifications so open Settings/login reflects current state.
No application/domain/provider behavior is moved into UI.

Corrected real shipping-root tests with original composition failed0pass/2fail:
Settings was admitted without painted content,and a synthetic modal was absent.
Outer view passed2tests; Settings updates/reopens and modal dismissal restore
background interaction. Initial harness used a nonexistentheader button and
confused debugselectors with observedIDs; correctedfixtures supersede those
failures. Independent Sol review found no production defect. Its valid test
weakness was technically verified: Settings already has a modal guard,so that
background click cannot prove blocking. Correctedtest uses Needs Review: None
under modal,then Some(Findings) after dismissal; fresh2tests passed.

Fresh nativeDesktop216/0failed/0ignored,both nativeall-targetClippy,shippingbuild,
52fixtures and startup2V10/exactBOMCRLFUnicode/zero credentials passed. Evidence:
dialog-*.log and qualified-startup-vj1o3h95 under the publicationparent above.
Source hashes match independent review. Exact-head CI/integration remains next.
Manual scenario: open Settings using footer/menu/shortcut,scroll to Connection,
explicit Connect and inspect visible human sign-in,then cancel; reopen and verify
current account/model/effort controls. No provider call occurred in this correction.
NativefixedSettings/sign-in remains pending because Mac locked again before
relaunch; keep Macunlocked/awake for later bounded authorized Luna qualification.
Owneracceptance remains separate. [Screenshots](../../../ui/screenshots/2026-10-04/INDEX.md)
retain inspected synthetic UI captures for ownerimprovementwork; exclude auth
secrets and private/original data. Earlier unretained imagebytes cannot be restored.
Safe Stage6 reference/execution work continues independently.
