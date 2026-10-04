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
