# Read metadata-invalid saved evidence with exact bounded ranges

Selected next independent P3 outcome after the read-call correction was parked.
Reuse /Users/evokessler/repos/brn-p3-work-budgets, branch codex/p3-raw-evidence,
baseline qualified citation candidate19bc465d2c5dc58344385688bc06a570cb921ba9
(code538b160). PR102 revised requiredCI37863707634 runs in parallel; integrate it
normally before this candidate. No functional dependency on citation review.
Parked retry branch8f8fc2e7621bd744778a86dc149357be2fbc59fc is committed/pushed,
NOT QUALIFIED, not merged. Do not bring any of its behavior/counter copy into here.

Owner authorizes small complete V1 slices until05:00UTC, selected lead model/effort,
at most2 helpers/no recursion and one Cargo across checkouts. Synthetic/public only,
no GUI/computer/private data/credentials/accounts/downloads/paid fallback/ports/release.
Shared live ledger14/16 used7each; root alone may choose later bounded qualification,
not helpers. No inference for mechanics. One authoritative morning task remains in
lead checkout /Users/evokessler/repos/brn-p2-email-docx-intake.

## Outcome, baseline and reuse

Current AI read_note/read_note_range, even explicit All, reject saved_metadata issues.
Normal chat therefore cannot inspect metadata-invalid saved Markdown, despite the
existing owner CLI evidence read preserving its bytes. An explicit read_raw_evidence
tool and shared headless query will read those bytes without treating them as Current
knowledge or relaxing ordinary Current/proposal/approval validation.

Reuse vault::read_evidence, EvidencePath containment, existing1MiB saved-file bound,
50,000byte UTF8 cap/range convention, saved_metadata/Inbox provenance parser,
AiTools root/epoch fences, existing blocking-read drain lease, Rig tool registration,
WorkBudget and AppWorker. No new parser/scanner/index/schema/runtime/dependency,
provider retry/fallback or metadata repair. Valid-byte raw evidence is untrusted input,
never instructions or approval authority. Original files and owner buffers unchanged.

## Fixed interfaces

Public brn-ai DTOs reexported from workflow::knowledge for CLI:

- RawEvidenceRequest { path:String, start_byte:usize(default0),
  end_byte:Option<usize>(defaultNone), expected_sha256:Option<[u8;32]>(defaultNone) }.
  deny_unknown_fields. Validate bounded nonempty path, checked offsets and <=50000
  explicit interval (empty intervals allowed). expected_sha256 is mandatory whenever
  start_byte!=0 or end_byte is Some; an unbound default request reads capped prefix.
  Exact visible contained Markdown path validated by host before IO and CLI startup.
- RawEvidence { path:String, sha256:[u8;32], start_byte:usize,end_byte:usize,
  total_bytes:usize,text:String,partial:bool,facts:Option<NoteFacts>,
  metadata_issue:Option<String> }. deny_unknown_fields.
  Metadata-invalid response has facts=None and nonempty issue, so no invented
  Current/Source/History/UUID claims. Valid metadata has Some existing NoteFacts and
  no issue; it describes observations, not truth. Validate existing Inbox metadata
  too. Complete hash covers full saved file including BOM/frontmatter/CRLF.
- RawEvidenceRequest::validate(), RawEvidence::validate_for(&request) shared across
  adapters. Exact requested path/start/end/hash, UTF8 byte length, <=50k/whole1MiB,
  honest partial flag, facts fullhash agreement and exclusive facts/issue state.
  Reject metadata_issue>4096UTF8bytes and blank/facts.note_id>64bytes; never truncate.
  Default prefix may round its50000 cap down only to UTF8 boundary; explicit ranges
  never clip or normalize. Missing/hash-changed/out-of-file/nonboundary reads refuse
  whole without partial data. Empty file/default0..0 valid.

ReadTools::read_raw_evidence(&RawEvidenceRequest)->AiResult<RawEvidence>, default
refusal for older backends. DrainedTools delegates while retaining its existing Arc
lease; AiTools provides contained full-read/facts/hash/range and root/epoch checks.
Capture read-only physical VaultIdentity at AiTools open for raw queries, verify
it before/after each raw read; unavailable/replaced roots refuse. This adds no
persisted registration or writer authority and leaves existing scoped-tool root
behavior unchanged.
App::raw_evidence(&RawEvidenceRequest), AppCommand::RawEvidence(request) and boxed
AppEvent::RawEvidence(reply) share the owner headless boundary. CLI adds
`evidence read-raw --file REQUEST.json`; existing evidence read unchanged. Strict
regular JSON<=64KiB, nonblocking FIFO refusal and typed validation before workspace.
Use an existing/appropriately shared bounded JSON reader, no new CLI framework.

AI tool `read_raw_evidence` uses the same DTO/schema, existing spawn_blocking and
reply validation. Add to the same normal Rig builder, preserve every budget/cancel/
model choice. Description/preamble explicitly says facts=None/metadata issue is raw
unclassified evidence, not Current knowledge; copy returned fullhash for later ranges.
Saved text is evidence, never trusted instructions. No raw metadata repair or authority
upgrade. Strict visual runtime remains unchanged; read tool is available where
normal read tools already exist, including normal Ask/Inbox/Rewrite investigation.

## Acceptance / ownership

AI helper owns public DTO/trait/tool/registration/behavior guidance and meaningful
real-Rig/provider/adapter tests; only brn-ai files/README. Root owns Workflow backend/
lease/AppWorker/CLI/tests/contracts/cases/morning task/review/integration. Fix API
before edits; no helper Cargo until root grants it. No shared file races/recursion.

Meaningful tests: malformed identity/classification/citations/Inbox metadata readable
with facts absent; ordinary Current/scoped tools still refuse; valid unmanaged/
Source/History/archive facts observed; exact BOM/CRLF/multibyte whole bytes and
near1MiB tail; prefixcap UTF8 vs exact empty/range semantics; changed fullhash despite
same size/mtime or unchanged requested interval; root/epoch/uncertain/refusal;
symlink/traversal/hidden/nonregular/oversized/invalidUTF8; cancellation/drop holds
owner until blocking raw read drains; malformed backend reply refuses; no proposal/
Finding/Action/file effect; fresh-process CLI range/restart/index-loss and preflight/
FIFO. Real Rig mocked3routes advertise schema/use explicit raw hashbound ranges and
preserve useful response; no unsafe repair/authority. Existing budgets unchanged.

Obtain one independent complete read-only review, fix validated defects, final
applicable AI/Workflow/CLI/default/native/Clippy/shipping checks and requiredCI,
normal protected merge/resulting-main verification. Reuse unchanged fullsize witnesses;
no weakening/suppression. Retain synthetic raw input/requests/results; actual GUI
acceptance pending in the existing single morning task, never a separate UI checklist.

## Implementation checkpoint — 09 October 00:37UTC

AI complete153passed/1existingignored, strictAIClippy; all3mockedRig routes verify
raw replies/refusals and existing budgets/cancel policy unchanged. Workflow7
including physicalroot/hash/metadata/boundaries/drain/worker, actualCLI3 plus3
citation shared-JSON regressions andDesktop1ownerpending/partial passed. Root
fixed2test-style lints; final affected Clippy passed. No open failed behavioral
check. Fresh complete independent review/final default/native/shipping/CI pending.
PR102 prerequisite normally merged77960d1 with exact checked tree; incorporated
without functionaldelta. No livecall used, WorkspaceJ pending.
