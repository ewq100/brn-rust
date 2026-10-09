# Discover Current knowledge whose saved citation evidence needs review

Selected next independent P4 slice after reviewed Create rename. An owner can
find Current notes with changed, absent, ambiguous or incompletely inspected
durable citation evidence and open the exact saved consumer and citation detail.
These are evidence observations, not conclusions that a claim is false or stale.
No semantic Finding, proposal or authoritative edit is created automatically.

Baseline: reviewed Rename candidate182cb8e1247f288b045957128070dd88bb5882ab,
stacked on merged/required-post-verified Action maine1d6b0f. Implementation reuses
`/Users/evokessler/repos/brn-p2-email-docx-intake`, branch
`codex/p4-citation-evidence-review`; PR101 qualification/CI stays in the budget
checkout. Integrate PR101 normally first and incorporate resulting main before
this candidate's protected integration. There is no functional Rename dependency.

## Reuse and bounded interfaces

Reuse fresh IdentityInventory, all five NoteProvenance outcomes, saved metadata
Current semantics, existing current-evidence fences, Needs Review navigation and
read-only proof widgets. Extract the existing resolver into a crate-private helper
accepting saved consumer bytes and one shared inventory. Do not call the public
provenance query per row and repeatedly scan the entire vault. No new scanner,
Store/index schema, persisted Finding, scheduler, dependency or model tool.

Fixed strict workflow DTOs, reexported from `knowledge`:

- `CitationReviewRequest { limit: usize, cursor: Option<CitationReviewCursor> }`,
  default25, accepted1–100.
- `CitationReviewCursor { vault: VaultRecord, observation_digest: [u8;32],
  after_path: String }`. Bound root and fresh evidence digest, never an in-process
  epoch passed between CLI processes. Validate contained visible Current path.
- `CitationReviewIssue { index: usize, outcome: CitationOutcome }`.
- `CitationReviewEntry { path: String, title: String, note_id: Option<Uuid>,
  sha256: [u8;32], citations: Vec<CitationReviewIssue> }`. Only non-Matched issues;
  summary rows do not repeat quotations or full source-match lists.
- `CitationReviewCoverage { incomplete: bool, diagnostics: Vec<IdentityIssue>,
  diagnostic_count: usize, diagnostics_truncated: bool }`, at most32 displayed
  diagnostics with honest count/truncation. Do not silently truncate proof.
- `CitationReviewPage { entries: Vec<CitationReviewEntry>,
  next_cursor: Option<CitationReviewCursor>, inspected_count: usize,
  coverage: CitationReviewCoverage }`.
- `CitationReviewDetailRequest { path: String, expected_sha256: [u8;32] }`.
- `CitationReviewDetail { path: String, title: String, note_id: Option<Uuid>,
  sha256: [u8;32], text: String, provenance: NoteProvenance }`.

Pure methods: both request DTOs expose `validate()`, the returned page exposes
`validate_for(&CitationReviewRequest)`, and detail exposes
`validate_for(&CitationReviewDetailRequest)`. Check bounds/order/non-Matched issue
indices, honest coverage/counts, continuation progress/root/digest, and exact detail
path/text hash/provenance path. These are shared by CLI/native preflight and tests.

App methods `citation_review(&mut self, request)` and
`citation_review_detail(&self, request)`; AppWorker commands `CitationReview` and
`CitationReviewDetail`, same-named boxed reply events. Detail returns complete
read-only consumer bytes and existing provenance together, avoiding unbound
parallel reads when opening a row. Pure validation binds returned detail path,
text hash and provenance path to the submitted request. Complete detail must fit
its explicit bounded response budget or refuse whole; never truncate quotations.

## Behavior and freshness

Consumer eligibility uses existing Current semantics: valid managed metadata,
neither Source nor History, outside case-insensitive top-level archive. Missing
classification fields remain Current as today. No/empty citations is not an issue.
All four non-Matched outcomes are shown. Incomplete means unavailable or uncertain
lookup, never missing evidence or incorrect knowledge. Corrupt/unreadable consumer
metadata appears as incomplete coverage, not an eligible healthy note.

Cited evidence can be Source, History or archived. An unchanged moved source
still matches by UUID/complete hash/quote; duplicate UUIDs are ambiguous even with
equal bytes. Retain original citation bytes and report every observation honestly.

Validate before observations. Capture current-evidence fence/root, refresh
disposable metadata, obtain one shared identity inventory, process at most limit
eligible consumers in path order and advance by last inspected candidate even on
sparse pages. Freshly read consumer/full hash/classification. Bind continuation to
fresh deterministic inventory/metadata digest. Reobserve returned consumer/source
proofs and inventory before return; check current fence at both boundaries.
Changed epoch/root/digest/consumer or uncertain durable operation refuses with
ContextStale and Refresh. Full identity scanning remains O(vault); paging bounds
consumer resolution/output, not that inherited scan. This is a fresh observation,
not an atomic filesystem snapshot. Exact detail refuses a changed consumer hash.

## Ownership and acceptance

Workflow helper owns new knowledge/citation_review.rs, resolver extraction,
knowledge reexports, narrow AiTools/App epoch accessors, worker DTO dispatch and
workflow behavioral tests. Native helper owns citation-review presentation state,
AiState/native Needs Review mode and state/widget tests. Lead owns CLI
`needs-review citations`/`needs-review show --file REQUEST.json`, contracts,
synthetic retained cases, morning task, review and integration. Fix interfaces
before edits. No helpers spawn recursively; at most two active; one Cargo across
checkouts. Retain selected lead model/effort. Root owns shared documentation.

Acceptance includes all four outcomes; Matched/no-citation exclusion; Current vs
Source/History/archive consumer scope; moved/archived source success; malformed
consumer and unrelated unreadable-identity coverage; sparse pagination/digest
mismatch; same-size retained-mtime edits; source/consumer/epoch changes; restart
and index-loss reconstruction; exact detail/quotes; stale native generations and
row clicks; and owner editor/composer/review preservation. Existing Finding
Resolve/Dismiss controls remain confined to persisted Findings. Derived citation
rows are read-only, with clear reasons and complete saved evidence.

Use only public/synthetic fixtures and deterministic tests. No inference is needed
for discovery, inspection, replay or approval mechanics. No GUI/computer use,
private data/credential inspection, optional downloads, resets, ports or release.
One complete independent read-only review, final applicable tests/Clippy/native/
shipping, actual required CI, normal protected merge and post-main verification.
All interactive expectations go into the single morning task, never a separate
UI checklist. Reassess only a concrete integrity blocker or consequential owner
choice; preserve recoverable work and continue elsewhere when needed.

## Implementation checkpoint — 08 October 23:58 UTC

Implemented shared backend, strict DTO/whole-reply validation, shared provenance
resolution, read-only native Needs Review mode and owner CLI. Backend10 new tests
plus10 existing provenance tests and strict workflow Clippy passed. Desktop5 state
tests,8 native state/headless widgets and4 existing Findings widget regressions
passed; strict default/native desktop Clippy passed. Confirmed vault rebind/root
change invalidates derived proofs; ordinary same-root status retains the view.
Real-process CLI3 tests passed: typed preflight, nonblocking FIFO refusal, sparse
paging across restart, exact detail/index loss, stale source/consumer and no effects.
Full final gates, complete independent review and candidate CI remain pending.

Zero-inference WorkspaceI citation-review-case is preserved with four explicitly
approved synthetic citation consumers, changed/absent/ambiguous/moved evidence,
31 healthy prefix notes for sparse native pagination and malformed coverage.
Preparation used qualified runtime182; new-candidate CLI qualification pending.
All interactive acceptance remains in the single morning task. PR101 normally
merged6a63173 with identical qualified tree; post-main37861908706 pending.

## Independent review and disposition — 09 October 00:02 UTC

Fresh complete read-only reviewer citation_evidence_review inspected all28 changed
production/test/doc files atb433d9897b880975417d9d22fff14ec0356d1e7f against
main6a63173. Complete binary patch SHA256
`c56cfb40bd0985fc53fce008ee9a7fe31066d4fb4c42d48e34e2842b37c85c6c`.
One P2 finding, **valid / correction pending**: malformed managed brn_inbox_source
is not checked by saved_metadata eligibility, so a no-citation Current file can
abort the entire page during provenance resolution. Lead reproduced actual
CLIexit1 WORKFLOW_ERROR in citation-malformed-intake-repro/before-fix.json.
Expected behavior is incomplete coverage, excluded malformed consumer and
continued sparse-page discovery of later affected consumers. No other findings.

Root prepared a narrow observation validation patch and regression outside the
checkout under /private/tmp/brn-overnight-20261008/citation-malformed-intake-*.
Apply only after frozen baseline full gate67708 releases Cargo, demonstrate
regression red then green, obtain independent correction review and rerun final
affected Workflow/CLI/Clippy/shipping gates. Existing unchanged full-size Store
crash/recovery/Undo witnesses may be reused. PR102 candidate b433 remains
unmerged; required run37862406544 is active, and corrected candidate needs its
own actual required checks. No integrity bypass.

New-candidate defaultCLI qualification of WorkspaceI passed zero-inference at
23:59:25UTC: first page25 inspected/zero issues/continuation; second page exact
Absent/Ambiguous/Changed rows; moved source Matched; all four exact details and
malformed coverage retained. CLI SHA
cc835950863df76f35bf0ab8d1dfac98666abc208890fca744a6da875f63911f.
Receipts are in citation-review-case/receipts/candidate-citation-review. This does
not qualify the newly identified malformed Inbox field or establish GUI acceptance.

## Correction qualified — 09 October 00:10 UTC

The new malformed Inbox consumer regression was observed red on the original
production code (WORKFLOW_ERROR strict single-line JSON), then green after adding
existing Inbox provenance validation to observation eligibility. Invalid consumers
now give sorted incomplete coverage and are excluded; sparse pages continue to
later affected notes. No writer/authority or public DTO change. All11 citation
backend tests and3 actual CLI process tests passed after correction.

Required Ubuntu CI onb433 also found the new read-only CLI test used macOS-only
provenance.capture during fixture setup. Replace only setup with the existing pure
Store citation metadata writer and inventory hash; keep every read-only assertion
on Unix and add malformed Inbox coverage assertion. No test skip, suppression,
weaker assertion or product platform expansion. Revised required Ubuntu CI must
confirm this fixture correction. Raw receipt pr102-ubuntu-first-failure.log retained.

Frozen baseline full qualification completed:1815 default/18 existing ignores,
446 native workflow/models/16,597 combined desktopCLI, all doctests, strict default/
combined workspace Clippy, shipping/helper,52fixtures and608 links. Only the new
observation validation/regression and portable CLI setup changed afterward; reuse
unchanged Store/crash/Undo and native state/widget evidence. Final affected
backend/native knowledge/CLI/Clippy/shipping gates and independent correction
review remain pending. PR101 all four required post-main checks/docs passed in
37861908706; resulting main is verified.
