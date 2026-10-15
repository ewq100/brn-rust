# BRN modular foundation and reference-assisted writing

**Planning date:** 15 October 2026  
**Repository:** `ewq100/brn-rust`  
**Inspected baseline:** `b139221f295f3e2d8f22045c21ef7d3d08a7b74e` (`main` at inspection)  
**Status:** Proposed implementation program. Source reviewed; no implementation, builds, benchmarks, or editor/intake qualification performed as part of preparing this pack.

## 1. Decision and intended outcome

Strengthen BRN in place through three coordinated workstreams:

- **A — Intake:** independently testable document extraction, qualification of Xberg/Kreuzberg, and integration through BRN's existing checked import path.
- **B — Writing:** a reusable, reference-assisted writing workspace inside BRN, plus an evidence-based Zorite editor decision.
- **C — Foundation and integration:** shared contracts, checked-core hardening, a scoped retrieval/evidence capability, dependency discipline, and final integration.

C is also the integration lead. It is not a fourth competing rewrite. Start C's small contract/baseline slice first; then A and B can build concurrently while C continues foundation work.

**End-state product outcome:** Open or create a document, attach reference materials, set a brief, write directly, collect passage comments and overall feedback, generate a source-grounded proposed revision, inspect sources and changes, Apply or reject, Undo, and recover the workspace after restart. This works inside BRN even with the ordinary chat panel closed.

**End-state engineering outcome:** Intake, retrieval, writing, and editor integration have clear responsibilities and tests. They reuse one authoritative checked state/change service. The writing component works in a minimal development host without BRN's full navigation. No separate production Writer is released in this program.

“Retrieval expert” means a dedicated scoped retrieval/evidence service first. It is not a requirement to put an additional autonomous model in every search. A read-only research helper may use the same service under existing explicit settings; it cannot replace permissions, exact evidence validation, or ordinary deterministic search.

The term “solid editor” in the request is interpreted as the **Zorite editor evaluation** from the preceding discussion.

## 2. Authority and working rules

This pack is an implementation handoff, not an assertion that work has already been authorized, executed, or verified. When the owner hands a role prompt to an agent, it assigns that role's bounded local implementation, synthetic testing, documentation, review, and local commits. External publication, merges, releases, provider spending, model downloads, and real-data use must follow explicit current owner authority. Historical permission attached to another task is not reused.

Routine reversible decisions within the assigned scope do not need repeated owner confirmation. Record consequential unresolved choices, choose the conservative existing behavior, and continue independent work. Do not block the entire program on an unavailable optional provider or locked Mac.

Use existing repository tooling and the owner's selected model. Do not hard-code a new development model, effort, context size, or agent count. One lead per stream is sufficient; bounded helpers and independent reviewers are optional. Do not create an agent-management framework.

Use Rust and the existing native macOS Apple Silicon direction. No NoteGen, Pi, Electron, Flutter, or Zed application fork is part of this program. No unrelated compiler, Rig, GUI-toolkit, embedding-model, or storage-engine upgrade is implicitly authorized.

Local commits are not permission to merge. Keep baseline and integration refs explicit. Never force-push shared history, weaken required checks, modify unrelated work, use real owner files as fixtures, or store credentials in evidence.

## 3. Verified starting points and remaining uncertainty

The observations below are supported by the source references in section 18, not by a fresh build.

| Area | Observed starting point | Planning consequence |
|---|---|---|
| State | Current invariants require SQLite authority, one checked write boundary, versioned preconditions, replay, exact review, coordinated human writing, and compensating Undo. | Preserve these as contracts; prove them rather than redesigning by assumption. |
| Intake | `brn-intake` and `brn-threads-intake` already exist. `ConversionResult` contains Markdown, assets, source identity, coverage and gaps. | Evolve this boundary; do not introduce a rival import model. |
| Dedicated conversion | The 14 October Rust/Luna assignment specifies deterministic formatting/checks with a dedicated model-assisted conversion path, explicit credentials, and no silent model fallback. | Treat it as part of the baseline. Any replacement needs a documented comparison and explicit supersession, not accidental deletion. |
| Writing | Writing responsibilities span `brn-desktop`, `brn-threads-app`, `brn-threads-core`, and `brn-ai`. The existing writing snapshot captures a note, comments, messages, and overall feedback. | Extraction crosses real boundaries. Moving one file will not suffice. |
| Retrieval | An existing accepted plan and harness cover derived indexes, scoped search, keyword/dense behavior, Connections, and 1k/5k/10k synthetic corpora. | Reuse these; do not start another retrieval implementation or assume semantic search is still absent. |
| Integration | Repository workflow already assigns shared interfaces to one lead. Required checks include Linux core as well as macOS core/native lanes. | Product scope remains Mac-first without ignoring required Linux core failures. |
| Archive | The open PR response includes #112, explicitly marked archive/do-not-merge. | Reuse any relevant design ideas deliberately; do not merge the archive as part of this work. |

GitHub cannot establish what unpushed local work or active agent sessions exist. **C0 must inspect the actual local worktrees, active assignments, and fresh remote state before assigning files.** The inspected commit is a reference, not an instruction to reset to an older revision.

## 4. Non-negotiable preserved behavior

1. **One canonical store and change mechanism.** UI, CLI, AI, and later external consumers use the same checked application/core service. No raw canonical SQL writes from writing, conversion, search, or helpers.
2. **Existing authority semantics.** Preserve standing delegation and direct-owner behavior where already applicable. Writing revisions remain proposal-only until exact review. Do not add a blanket approval requirement to unrelated previously authorized maintenance.
3. **Exact, current preconditions.** Capture document version and relevant scope/context versions. A protected-state change, access revocation, stale draft, or changed required current input cannot be bypassed by a successful model response.
4. **Human draft protection.** Preserve dirty guards, editor generations, unsaved text, and delayed-save handling. AI completion does not erase later typing.
5. **Exact-candidate review and durable Undo.** The reviewed text is the committed text. Regeneration needs new review. Undo is a checked compensation, not database rewind and not provider-call cancellation.
6. **Reference access is read-only for writing.** A reference can be independently edited through an authorized document workflow; attaching it does not give writing permission to alter it. Shared references retain one canonical record rather than duplicate copies per project.
7. **Original binaries remain external.** Retain complete useful imported knowledge, required assets, provenance, and coverage, not permanent hidden copies of original source files. Scratch/previews expire and are cleaned after interrupted runs. Do not delete the owner's external original.
8. **Full import is not summarization.** Complete readable content must not silently become an AI summary. Unsupported substantive content is marked partial or unchecked. Model success and extractor output are not proof of completeness.
9. **Scope is host-owned.** Models and source documents cannot grant access, choose other accounts, expand project scope, or supply trusted authorization fields. Exact-ID reads, source previews, snippets, counts, pagination, and graph links obey the same policy as search.
10. **Truthful outcomes.** Empty results, service failure, partial conversion, missing embeddings, cancellation, and budget exhaustion are distinct. No silent alternate provider/model. Stop fences local result acceptance; it does not prove the provider stopped processing.
11. **Compatibility and recovery.** Preserve existing supported workspaces, drafts, reviews, projects, private sources, backups, and undo receipts. Do not reset owner databases. Necessary schema changes are owned by C, separately qualified, and recoverable using verified copies.
12. **One selected production route after qualification.** Temporary comparison backends may exist in tests/spikes. Do not leave a shadow converter/editor that silently hides production failures. Retain a tested source/build rollback and necessary old data readers, not obsolete parallel behavior.

## 5. Target responsibilities, not a mandatory crate explosion

Use existing packages where they already provide a good boundary. New crate names below are proposals to validate, not a requirement to create empty packages.

| Component | Owns | Must not own |
|---|---|---|
| Intake boundary (`brn-intake`, `brn-threads-intake`) | Bounded reading/extraction, assets, normalized output, provenance, coverage/errors. | Canonical writes, project membership, selecting a different AI account, authoritative retrieval indexes. |
| Checked core (`brn-threads-core`) | Authoritative records/revisions, grants, version checks, atomic writes, replay, dirty guards, Undo, backup/recovery. | UI widgets, remote model calls, converter implementation details. |
| Shared application service (`brn-threads-app`) | Use-case coordination and host implementations for imports, writing, context, and search. | Duplicated permission engines or framework-specific editor state. |
| Retrieval (`brn-retrieval` plus existing host adapter) | Rebuildable index, ranking, exact results, bounded search, capability/coverage reporting. | Canonical content authority, permission grants, automatic model calls for every query. |
| Writing engine (proposed `brn-writing`) | Writing-session state, briefs/references, comments, immutable revision requests, proposal/review lifecycle. | BRN navigation, concrete database handles, parser internals, provider credentials. |
| Writing UI (proposed `brn-writing-ui`) | Reusable document/comments/references/review panel and editor adapter. | Another persistence, authorization, or Undo system. |
| Existing AI layer (`brn-ai`) | Provider transport and runtime tools with explicit product/transport boundaries. | A new generic agent framework or authority derived from model output. |
| BRN host and small development host | Compose the real components and present their state. | Copied implementations of the writing workflow. |

The headless writing component must not depend on `brn-desktop` or the full application-service crate. Declare the narrow host operations where that avoids cycles; implement them in the application layer. Reuse neutral existing data types where possible. If types need moving, move the minimum coherent set with C's ownership; do not introduce a universal `common` crate or dependency cycle.

Independence means bounded responsibilities, documented contracts, isolated tests, and no accidental knowledge of the full shell. It does **not** mean duplicating core services so every module has its own database.

## 6. C0: establish the common starting point

C performs this slice before A/B integrate production changes. A/B may simultaneously do read-only research, fixture preparation, and isolated spikes that do not alter shared interfaces.

### C0 tasks

- Read `AGENTS.md`, current status, architecture target/behavior/invariants/dependencies, development workflow/verification, relevant active plans, and actual implementation.
- Inspect current branch/HEAD, fetched main, local worktrees, active processes/assignments, and open PRs. Identify current owners and preserve unrelated work.
- Reconcile already-running intake, writing, retrieval, and quality work. Existing owners may continue as A/B/C; do not start duplicate implementations.
- Inventory supported user-facing behaviors through their actual routes, not just enums or a legacy converter function. Record what already exists, what is unverified, and what is missing.
- Run a relevant clean baseline with exact commands/features. Record pre-existing failures rather than hiding them or claiming a clean baseline.
- Produce a compact dependency map and file-ownership map. Record the compiler/lockfile, native GUI dependency graph, converter path, storage schema, model profile, and required CI contexts.
- Agree minimal shared contracts in section 7, naming existing types/methods wherever suitable. Add enough executable contract fixtures to support both A and B without implementing their products.
- Establish a common local integration ref and a committed C0 boundary revision. Record it in the coordination log. No branch name in this pack authorizes resetting an existing branch.
- Establish fixture families and a baseline cost/latency/artifact-size record. No new performance promises based on marketing.

### C0 deliverables

`plan.md`, `contracts.md`, `ownership.md`, `baseline.md`, and `coordination.md` under `docs/work/active/modular-foundation/`, or equivalent existing canonical paths selected by the lead. Avoid duplicating existing documentation authorities; link to maintained specifications.

### C0 exit

A and B can identify exactly what they consume, what they may edit, which baseline they build against, and what the lead alone can change. Contract fixtures compile where appropriate. This is a small integration prerequisite—not completion of all core refactoring.

## 7. Shared contract requirements

These describe semantics. C must map them to existing interfaces before inventing new public APIs.

### 7.1 Conversion result

Prefer extending the current `ConversionResult`, `Coverage`, and associated asset/provenance types.

Carry source identity/hash, format, import intent, normalized complete readable content, required assets, extraction engine/version/configuration, source locators, gaps, and terminal outcome. Preserve distinctions such as complete-relative-to-an-independent-inventory, partial, and unchecked.

Raw document bytes, temporary page images, credentials, private filesystem paths, and model transcripts do not belong in durable conversion results. A content hash identifies bytes; it does not establish authorization or completeness.

Cancellation/errors cannot produce an apparent successful import. The host performs the checked save only after validating the result and rechecking cancellation and scope.

### 7.2 Writing context and versioned request

Persist a writing workspace's document link, reference links, brief, comments, and conversation association through existing shared records wherever suitable. Workspace context survives switching/new chats and restart. Opening a document does not force a project-setup ceremony.

A request binds its identity, draft/document snapshot, document version, editor generation where relevant, chosen references and revisions, source purpose, comments, brief, host scope/context epoch, model selection, and limits. Do not overload the existing immutable-message payload by embedding every reference document in it. Inspect and respect current byte bounds, and distinguish serialized-byte bounds from model token budgets.

Default to the existing safe Save-and-rewrite/Save-and-comment semantics. Make any required save explicit; do not introduce implicit writes just to start AI. Supporting an unsaved snapshot is allowed only with a proven generation/base-version contract and no loss of unsaved owner text.

A retry that uses the same snapshot must revalidate present access and its declared current prerequisites. A refresh/regeneration creates a new request and candidate; it cannot reuse prior approval.

### 7.3 Scoped evidence/retrieval service

The trusted host supplies scope. Query, selected document IDs, requested task type, and budget are untrusted request inputs, not authority.

Return stable canonical document IDs, immutable revision IDs/versions, exact quoted text, validated byte ranges in retained content, honest source locators, retrieval mode/status, and the scope/index generations needed for validation. Keep provenance mapping from a converted paragraph to a source page/section distinct from the paragraph's own byte range.

Provide search and bounded exact reading. A query may select relevant passages; an exhaustive requirement check needs an explicit coverage plan across the selected full document/sections. Top-k relevance is never labeled “read every requirement.” Where budgets prevent full coverage, report the missing coverage rather than truncating silently.

Preserve current-vs-historical semantics: historical citations can describe an old version without claiming it is current, but cannot bypass present access restrictions. Current reference changes/revocations invalidate affected operations according to existing fencing policy.

The scope predicate is defined once. Defense-in-depth rechecks use that same predicate; adapters do not each invent permission rules. Filter before releasing content to the model, and recheck at sensitive read/apply boundaries. Check snippets, counts, pagination, Connections, asset URLs, logs, and cached source panes as well as result IDs.

### 7.4 Editor adapter

Expose the minimum BRN needs: document text, generation, selection in validated source coordinates, change events, read-only/review state, programmatic candidate display/replacement, focus/reveal, and geometry available after real layout.

Differentiate user edits from programmatic refresh/candidate display. A display-only change must not look like user typing, clear a dirty guard, or trigger autosave. Rendering/pasting content cannot load unapproved remote assets, execute source instructions, or bypass link policy.

Local editor undo concerns uncommitted typing. Durable BRN Undo concerns accepted saved operations. Define and test their interaction so that accepting/undoing an AI revision does not resurrect stale local content on the next save.

### 7.5 Applying a proposal

Existing checked services receive exact candidate identity/content, operation identity, expected document/base version, relevant current inputs/context fences, and real host authority. The core commits or refuses atomically and records a receipt. A model cannot create an approval receipt. No interface returns “applied” before this succeeds.

## 8. Workstream A — intake qualification and modularization

### A1. Establish baseline and maintainable extraction boundary

Trace all current import surfaces and the dedicated prepared/Luna path. Record supported formats, assets, locators, completeness obligations, hard limits, provider usage, and current failure/recovery behavior.

Extract only orchestration/parsing coupling that demonstrably violates the agreed boundary. Keep application persistence outside the extractor. Reuse existing worker restrictions, temporary-directory lifecycle, prepared result validation, and import tests.

Create a candidate-versus-baseline runner using the same public/synthetic inputs and an independently established source inventory. A comparison is development tooling, not an enduring production fallback.

### A2. Qualify Xberg/Kreuzberg as the preferred candidate family

Verify current official project identity, published Rust package, release/tag, license and dependency provenance. Older Kreuzberg docs describe v4 LTS and redirect to Xberg; current Xberg presents a fresh v1 line. Do not infer the package version from the old “v5+” wording.

Prefer a maintained native Rust interface in an isolated bounded worker. Select the minimum features required for the agreed existing formats. Explicitly inspect native dependencies, PDF backend, optional OCR engines, ONNX use, model assets/downloads, transitive licenses, binary footprint, feature unification, and interaction with BRN's existing embedding runtime.

A separate process is not automatically a security sandbox. Verify the restrictions actually available on the target host: permitted inputs/scratch, no canonical-store/credential access, network denied for local extraction, output/input/memory/page/time bounds, child-process cancellation, and cleanup. Document residual limitations.

Do not inherit optional crawling, embeddings, transcription, remote LLM extraction, or entity features merely because the package supports them. Reference chunking/ranking and the authoritative derived search index remain owned by C's retrieval boundary.

Native text extraction comes before OCR. OCR is a selective, explicitly enabled route only for pages without usable extracted text or a demonstrated need. Do not OCR all documents. Do not expand this program into mathematical layout fidelity or universal Office rendering.

### A3. Resolve model-assisted conversion explicitly

Compare the existing prepared/Luna route and candidate extraction at equivalent user-visible tasks. Preserve deterministic source/value bindings and independent coverage checks.

Decide, with evidence, which of these applies per supported conversion profile:

- The new extractor supplies equivalent/better prepared evidence; existing qualified model-assisted interpretation remains explicitly selected.
- The new extractor can replace a currently model-assisted step without weakening substantive fidelity or changing a current owner-required behavior; document that supersession before cutover.
- The candidate fails a mandatory case; keep the existing production route and record a bounded upstream/adaptation issue.

Do not silently remove the dedicated conversion model, repurpose general chat settings, or route a failed run to a different model. An optional visual interpretation step must remain visibly separate from extraction and from deterministic validation. Do not introduce a second generic AI client when existing BRN/Rig infrastructure suffices.

### A4. Fixtures and acceptance

Preserve every supported user-facing format at the revalidated baseline. Prioritize Markdown/text, DOCX, PDF and email; include current HTML/spreadsheet/presentation cases where the actual active path supports them. A parser's enum entry is not proof of support. No obligation to enable every Xberg format.

Test headings, nested lists, footnotes, tables and cell association, leading zeros, exact numeric values/formulas/caches where currently supported, repeated figures, captions, image relationships, source locators, Unicode, malformed/truncated/password-protected inputs, missing native tools, cancellation, process death, limits, and cleanup after restart.

For full-import fixtures, establish expected substantive content independently of candidate output. Unknown coverage stays unknown. Converted table/figure relationships need evidence; matching individual words is not proof of correct association.

In BRN, test import to appropriate project/private/general contexts, preview, checked save, operation replay, cancel immediately before save, Undo, backup/restore, and retained-content use offline after the original is unavailable. Never use private real documents without separate current authority.

### A5. Cutover and retirement

Adopt only after mandatory fidelity, scope, failure-handling, platform/build, and integration gates pass. Measure latency, memory, native requirements, build impact, and maintenance/glue cost against baseline. No full-success claim from library marketing or a handful of easy files.

Switch one selected production route. Remove replaced parser/orchestration paths, redundant helpers, stale defaults, unused dependencies, and tests asserting retired behavior. Preserve needed historical result readers and source evidence, not a hidden old converter. Record a clean rollback ref and data compatibility.

If the candidate fails, ship the useful boundary/tests without pretending the dependency was adopted. A sound negative qualification is an acceptable A outcome.

## 9. Workstream B — reusable writing workspace and Zorite decision

### B1. Characterize and extract existing behavior

Using the current editor, preserve Read/Write, formatting, passage comments, overall feedback, saved request context, proposal-only rewrite, exact review, Apply/reject, Stop/retry, durable Undo, and per-document draft recovery.

Separate headless writing behavior from `ThreadsUi` and global chat selection. Introduce a reusable writing panel only where needed. Do not copy the writing implementation into another directory and leave both production routes alive.

Keep canonical changes behind the agreed host service. Move shared writing/core types only through C's ownership and compatibility review. Continue to use the current tested editor while extraction is verified; do not mix every extraction diff with an editor replacement.

### B2. Evaluate Zorite in an isolated development host

Compare the current editor with a precisely pinned Zorite package/commit using the same document corpus and host contract. Inspect the published package as well as upstream checkout: upstream workspace patches or demo dependencies may not be inherited by consumers.

Verify one compatible GPUI graph with BRN's existing GPUI Kit pin. No broad toolkit/compiler upgrade to make the demo run. Inspect any upstream patches, license boundaries and unpublished dependencies. Reuse editor/library code, not the whole GPL application by accident. Record notices and transitive obligations; do not treat this plan as legal advice.

Mandatory qualification cases:

| Area | Required evidence |
|---|---|
| Source fidelity | Round-trip supported Markdown without silent destructive normalization; rendered selection maps to exact source text. |
| Passage identity | Repeated identical paragraphs select the intended occurrence; Unicode, combining marks, emoji and CRLF are handled correctly. |
| Geometry | Comment reveal/placement survives scroll, wrap, resize, font changes and first layout; unavailable geometry is explicit. |
| Input | Native keyboard navigation, drag selection, clipboard, macOS IME/composition, focus and long-document behavior. Automated state tests are not a substitute for native observation. |
| Host ownership | No database writes; no accidental autosave on programmatic updates; real read-only/review controls, including programmatic paths. |
| Review | Display proposed content without changing candidate identity or committed content; Apply goes through the checked host. |
| Undo | Local typing history and durable saved-operation Undo cannot overwrite each other or recreate stale drafts. |
| Safety | Unsafe HTML/links/assets/source instructions remain subject to BRN's existing renderer policy. |
| Performance | Use existing typical/large fixtures; measure edit/scroll/selection latency and memory on the target Mac under comparable conditions. |
| Integration cost | Record dependency footprint, patches, glue and tests needed. A pretty demo alone is not a win. |

Choose **adopt**, **keep current**, or **bounded adaptation then re-evaluate**. The last option needs a specific change, owner, test and upstream/maintenance plan. It is not permission for an unbounded fork.

The writing module must complete even if Zorite fails. An editor cutover is its own reviewable slice after extraction and mandatory checks pass. Remove abandoned production routes once the decision is made; preserve comparison fixtures/reports.

### B3. Implement persistent reference-assisted writing

Workspace contains a working document, read-only reference links, optional brief, passage feedback, overall feedback, and associated conversations. A brief supports purpose/audience/tone/requirements without a new taxonomy system. Reference purpose such as requirements/evidence/style may be lightweight workspace metadata, not globally imposed document roles.

Support attaching an existing BRN note/project document and importing a file through A's interface. Use C's current project/visibility services rather than a second membership table or access policy. A shared document is one canonical record with appropriate links.

References/brief survive new chat, chat switch, note navigation, closed chat panel and application restart. Adding/removing references is an owner configuration action. Multiple independent conversations about the same document remain possible; the document is not itself a single chat.

Every writing action obtains a versioned writing context from the host. It can use relevant passages for a paragraph edit, or an explicit full-reference coverage plan for requirements checking. A source role affects interpretation, not authority. Source documents may contain malicious instructions; those remain untrusted content.

Provide visible states for attached, available, actually used, incomplete/unchecked, changed, removed or inaccessible materials. “Attached” is not “read in full.” A Sources used panel is derived from the recorded evidence, not a model-invented bibliography. Exact citations prove where a quotation came from, not automatically whether every generated claim is supported.

Use byte/token/output limits and cancellation without silently clipping required content. When a request is too large, fail usefully or use an explicit bounded staged reading plan with coverage; do not simply raise an envelope limit and send every file every time.

### B4. Prove independence without shipping another app

Build a minimal development host with the reusable writing panel, synthetic documents/references, and the same checked service or explicit contract-test adapters. It must not initialize the full BRN desktop/navigation/task system. At least one host journey must use the real shared service, not just mocks.

Changing a passage-comment or review behavior should change both BRN and the development host through one implementation. No second document library, production settings suite, sync engine, installer, or release is required.

### B5. Writing exit

The entire user workflow runs in BRN using the extracted implementation. Reference context persists. Scopes and exact review hold. Editor choice is evidenced. Restart and native interaction are verified where the environment permits; unobserved native cases remain an explicit gate, not a pass.

## 10. Workstream C — checked foundation, retrieval and integration

### C1. Make invariants executable before changing implementation

Map each relevant core invariant to current code and a meaningful test. Add missing regression/property/fault tests first, then fix actual defects or remove evidenced duplication. An audit may legitimately find a component needs no production change.

Prioritize atomic grouped writes, replay after lost responses, version/protection changes, draft generation/dirty guards across processes, exact-candidate approval, current versus historical inputs, private/project fencing, interrupted import/rewrite, Undo after subsequent edits, backups/restores and upgrades.

Use targeted mutation testing where it meaningfully tests guards: version comparisons, scope checks, input binding, cancellation-before-commit, replay identity, candidate binding and Undo conflict checks. Reuse the repository's recent mutation pilot. Classify survivors; do not pursue a vanity whole-repository score or mutate expensive live/provider tests indiscriminately.

Keep model/network/conversion work outside canonical transactions. Do not introduce CRDTs, event sourcing, a new datastore, new universal command/event buses, or generic dependency-injection infrastructure without a concrete current requirement.

### C2. Consolidate reference/context assembly

Reuse the current project and private-context predicates, source reads, snapshot logic, and application service. Expose the scoped evidence capabilities agreed in section 7 to writing and existing chat/tools.

Unify policy and evidence representation, not every product prompt. Writing and knowledge maintenance have different intents and authority. Preserve private-origin information even when a document is linked elsewhere.

Document which context is stored, retrieved, sent to a model, and refreshed. Record request/candidate IDs, used source revisions, omitted material and coverage without retaining original binaries, secrets, hidden reasoning, or raw provider transcripts.

Cache keys/invalidation must include the relevant scope epoch and document/index generation. Reference access revoked during a run cannot leak through an old Sources used pane or result cache. Revalidation uses the same core predicate.

### C3. Strengthen existing retrieval as a dedicated capability

Preserve the existing retrieval/index/worker and harness first. Improve boundaries or deficiencies demonstrated by tests, rather than replacing working retrieval to fit a new name.

Requirements:

- The index is derived and rebuildable from canonical retained content. Missing/corrupt/stale indexes are handled explicitly without losing canonical data.
- Current-only defaults exclude archived/superseded documents, pending candidates, old revisions and scratch as specified by current product rules.
- Keyword and optional semantic paths report actual availability, failure and coverage. Never fabricate semantic availability or silently download another model.
- Search, exact reads, source expansion and Connections share the same host-controlled scope. Result counts/cursors/neighbor links must not leak private material.
- Long references support exact section/passages and full-reading coverage separately from search ranking.
- Cancellation, index generation, restart, document removal, changed source links, and concurrent indexing remain tested.
- Preserve the selected model and current ranking behavior during boundary refactoring. Any later ranking/model change is a separate measured slice.

Reuse existing 1k/5k/10k fixtures and relevance queries; add cases for the new reference workspace and cross-project/private access. Compare recall@10/50 and ranking per query stratum, abstention/no-result behavior, source-span correctness, p50/p95 latency, index/update costs and memory. Reuse existing strict release-scale thresholds. Do not relax thresholds to manufacture success; explain noise and rerun comparable measurements. Refactoring needs non-regression, not an invented claim of a relevance improvement.

Any optional “expert” model uses these read-only tools, bounded rounds and explicit budget, returns source-backed evidence, and has no persistence/permission authority. Preserve any existing experimental/off-by-default helper setting. Qualification of such a helper is optional and cannot block deterministic search or become a mandatory hidden tax on every writing request.

### C4. Clarify provider/runtime ownership only where needed

Separate reusable provider communication from BRN-specific tools, request context and policy at the module/interface level. Preserve the existing qualified Rig integration and credentials behavior. Only create a new provider crate when a concrete consumer and dependency graph justify it.

Centralize writing/task guidance at the established behavior boundary; do not scatter new prompts through GUI callbacks or build a dynamic skill platform. Source materials are not trusted runtime instructions.

### C5. Integrate and simplify

C lands shared schema/types/manifests/lockfile/runtime wiring changes and integrates A/B slices in dependency order. Preserve single supported production routes after qualified cutover. Update affected authoritative docs, status and evidence so historical instructions do not accidentally reassign retired work.

Remove proven dead code, obsolete adapters and redundant dependencies once callers and saved-data needs have been checked. Fewer lines or crates is useful evidence, not the goal. Do not delete useful safety code because it makes the module appear smaller.

## 11. Ownership and parallel execution

Proposed local branch names, only if unused:

- C: `work/modular-foundation`
- A: `work/modular-intake`
- B: `work/modular-writing`

Use separate worktrees, scratch/output directories and synthetic databases. Do not let agents test against the same running BRN workspace. Use a predictable build-cache policy; distinct target directories avoid concurrent artifact/feature confusion. Shared compiler caches are optional. Schedule heavy native builds and benchmarks to avoid resource contention contaminating results.

| Surface | Primary owner | Rule |
|---|---|---|
| Intake implementations, extraction worker, prepared readers/checks, intake fixtures | A | Preserve declared contracts. Changes to dedicated AI conversion files are explicitly carved out in C0. |
| New writing packages, writing-specific app operations/tests, writing-related desktop files and editor adapter | B | No ownership of generic core or shared AI runtime. |
| Core storage/authorization/visibility/recovery; retrieval and context adapters/tests | C | Do not concurrently edit B's allocated writing files. |
| Shared type declarations, core writing-snapshot representation, schema/migrations, root manifests/lockfile, generic runtime dispatch, CI, canonical architecture docs | C/integration lead | A/B submit a small explicit interface/change request with rationale and compatibility notes. |
| Stream-specific plan/evidence/handoff files | Respective stream | Use separate files; no shared append race. |

C0 must turn these conceptual boundaries into exact current paths and owners. Shared ownership of an entire crate is too vague.

A/B may make clearly recorded minimal local manifest/module-declaration patches needed to compile their isolated candidates. These do not transfer integration ownership. C reconciles shared patches and regenerates the final lockfile from agreed manifests; do not merge generated lock conflicts blindly. Cross-cutting schema/authority changes are never smuggled in as build fixes.

A uses conversion fixtures; B consumes the same contract fixtures and current adapter until A qualifies. C supplies an initial evidence interface backed by the existing retrieval behavior. Thus A's engine decision does not block B's UI extraction, and B's Zorite decision does not block C's scope tests.

Do not casually cherry-pick another agent's unreviewed working commits. Coordinate through explicit contract revisions and reviewable slices. A contract change records affected consumers and migration steps; update both consumers and tests before declaring integration complete.

## 12. Delivery slices and order

These are logical review units, not a mandatory number of PRs.

| Slice | Contents | Dependencies |
|---|---|---|
| C0 | Baseline, ownership, minimum contracts, shared fixtures | None |
| A1 | Existing intake characterization and candidate harness | C0 |
| B1 | Writing extraction using current editor and minimal host | C0 |
| C1 | Scoped evidence adapter over existing retrieval; missing core regression tests | C0 |
| A2 | Pinned extractor qualification and dependency/fidelity decision | A1 |
| B2 | Pinned Zorite qualification using editor adapter | B1 interface; isolated spike may begin earlier |
| C2 | Targeted core fixes and retrieval boundary/evaluation improvements | C1 |
| B3 | Persistent brief/references and source-grounded rewriting | B1 + C1; can use current intake |
| A3 | Qualified intake cutover or retained-backend modular boundary | A2 + current shared contracts |
| B4 | Qualified editor cutover or documented current-editor decision | B2 + B3, with refreshed checks |
| C3 | Combined intake → references → retrieval → rewrite → review → Undo/restart | A3 + B3/B4 + C2 |
| C4 | Dead-route cleanup, final documentation, independent integrated review | C3 |

Land non-behavioral extraction separately from dependency or behavior cutovers where practical. After each integrated slice, consumers build against the recorded integration ref. Do not leave integration until all streams finish independently.

Publishing/merging uses current owner authority and required checks. Local integration tests can proceed on the lead's branch without permission to merge protected main. A release or a production standalone Writer remains a separate assignment.

## 13. Shared acceptance suite

Use one synthetic case pack across all streams: a working document; a requirements reference with headings, nested lists and a table; an evidence reference with figures and duplicate passages; a style reference; a private reference in another scope; and deliberately incomplete/malformed sources.

| ID | Scenario | Required result |
|---|---|---|
| F01 | Import supported reference as full note | Substantive content/assets preserved or explicit partial/unchecked status; provenance retained. |
| F02 | Repeated import operation after lost response | One canonical outcome; no duplicate document or receipt. |
| F03 | Worker/model failure or Stop near commit | No falsely successful or unauthorized partial write; scratch cleaned/recovered. |
| F04 | Close/reopen writing workspace; new/closed chat | Document, brief, reference links and comments persist independently of one selected chat. |
| F05 | Select second occurrence of repeated paragraph, Unicode/emoji text | Exact intended source range; no guessed reattachment. |
| F06 | Several passage comments plus overall feedback | Immutable request captures selected inputs; proposal available in document panel. |
| F07 | Targeted paragraph rewrite | Only permitted relevant material supplied; actual sources visible. |
| F08 | Exhaustive requirements check with oversized reference | Full-reading coverage is explicit; incomplete/budget-limited work is not presented as exhaustive. |
| F09 | Edit working draft while generation runs | New typing preserved; stale candidate refused or explicitly refreshed into a newly reviewed candidate. |
| F10 | Change/remove/revoke reference or project membership during run | Fences and current-input policy enforced; no leaked source content or continued unauthorized access. |
| F11 | Malicious instructions embedded in a reference | No new authority, provider change, hidden writes or broader retrieval. |
| F12 | Accept candidate, repeat acceptance, Undo, then conflicting newer edit | Exact reviewed result, idempotency, safe compensation and visible conflict. |
| F13 | Missing/corrupt index, partial/unavailable embeddings, restart | Honest capability/error state; rebuild from canonical content; no fake empty result or model substitution. |
| F14 | General/private/project searches and source panes | No leaks through IDs, quotes, counts, cursors, links, assets, caches or traces. |
| F15 | Local editor Undo after accepted AI revision/refresh | No stale content resurrection or mistaken implicit Save. |
| F16 | Crash with drafts, request/candidate and queued work | Meaningful work recoverable; authority/identity preserved; no duplicate apply. |
| F17 | Backup/restore/imported references without original available | Retained notes/assets/revisions/context remain usable; originals were not secretly retained. |
| F18 | Real shared-service development host | Same writing implementation works without full BRN navigation. |
| F19 | Existing chats/tasks/projects and delegated maintenance | No unrelated behavior or authority regression. |
| F20 | Native complete workflow on target Mac | Readable comments/review, functional keyboard/IME, source inspection, Apply and Undo; observable evidence, not screenshots alone. |

A successful test asserts meaningful outcomes; zero tests, self-skipped tests and mocked-only acceptance do not count. Use mocks/scripted providers for deterministic CI. Live selected-provider tests need current authority and fresh public/synthetic sources, recorded model/account-selection mechanism, effort and outcome; secrets stay out of receipts.

## 14. Verification, resource use and reporting

Reuse actual current scripts and inspect them before invoking. At the inspected baseline, useful gates include `verify-storage.sh`, `verify-end-to-end.sh`, `verify-desktop-shell.sh`, the existing retrieval evaluation runner, strict release-scale tests, and documentation/link checks. Commands/locations may change; revalidate them in C0 rather than pasting stale commands.

Required hosted contexts observed include:

- `Core and CLI (ubuntu-24.04)`
- `Core and CLI (macos-15)`
- `Native UI build and state (macos-15)`
- `Native retrieval and combined build (macos-15)`

Read current protection/CI before integration. Mac-first does not exempt a newly broken required Linux core check. Do not expand into unrelated optional Windows/Linux GUI repair.

After each meaningful slice, run focused tests and independent review. Run broader/default/native/end-to-end gates after relevant integration changes. Run benchmarks without competing heavy jobs and preserve the same fixture/model/environment parameters. Do not repeat whole-suite or live-provider runs without a relevant change or unresolved risk.

When the Mac is locked or inaccessible: no computer use, no GUI automation, no claim of native acceptance. Continue headless work and collect one consolidated owner/native qualification task with setup, ordered actions, expected results and known risks. Unlocking/login is a reported external prerequisite, not something an agent fabricates.

Report these statuses separately: **implemented; deterministic-verified; live-provider-verified; native-observed; independently reviewed; owner-accepted; integrated locally; published; merged; released.** Each carries exact commit/tree/features and limitations.

## 15. Evidence and handoffs

Each stream maintains a short current status plus detailed evidence where needed. Minimum handoff fields:

- Baseline, final commit and integration dependency/contract revision.
- Exact owned paths changed and any shared-file patches awaiting the lead.
- User-visible behavior changed, unchanged, deferred or refused.
- Dependency choices, exact pins, features, native requirements and license/source records.
- Tests/commands/environments, actual results, failed/skipped/blocked checks and limitations.
- Migration/data compatibility, legacy-reader requirements, rollback reference and reversal procedure.
- Independent review findings and disposition.
- Native/live tests still required and the precise next action.

Update authoritative product behavior, architecture/dependency boundaries, setup, verification, runtime guidance and status when affected. Avoid a forest of overlapping specs. Historical evidence stays linked as history, not as a new task mandate.

## 16. Scope limits and stop conditions

Not in this program: production standalone app, second repo, automatic cross-app synchronization, full NoteGen/Zed/AppFlowy adoption, a new AI framework, generic plugin infrastructure, global knowledge taxonomy, universal Office/WYSIWYG renderer, collaboration/CRDT, new database, broad platform port, arbitrary model upgrades, or “make every file its own crate.”

A dependency candidate fails adoption when mandatory correctness/platform/data-safety cases fail, it needs an unbounded maintained fork, its license/provenance is unresolved, or it forces a broad unrelated replatform. Deliver the useful modular boundary and a documented negative decision instead.

Never remove baseline coverage merely because a new component has a different feature list. Never declare the foundation complete while privacy/version/replay/Undo failures remain. An inaccessible Mac may permit implementation delivery with explicit native gates still open, not full acceptance.

A rollback to an older executable must account for schema compatibility. Preserve real data and verified backups. A code revert alone is not always a safe data rollback.

## 17. Definition of complete and later standalone trigger

This program is complete when:

- A/B/C responsibilities and dependencies are documented and enforced by useful tests.
- Existing core contracts remain intact and identified weaknesses are fixed or explicitly gated.
- Intake and editor evaluations have exact-source, reproducible decisions rather than assumed winners.
- The reference-assisted writing workflow works inside BRN and in a small real-service development host with one implementation.
- Retrieval/evidence is scoped, current, independently testable, and honest about reading/search coverage and errors.
- Combined failure/recovery and existing-product regressions pass relevant gates.
- Replaced production routes and stale dependencies are retired coherently.
- Required native observation, independent review and integration evidence are recorded; any pending owner/merge/release gates remain honestly separate.

Only then consider the standalone Writer: a small document navigation/settings/packaging host consuming the proven writing components and existing shared services. Do not move the library out of BRN or duplicate its logic. A separate repository needs a real maintenance/release reason and is not automatic.

## 18. Source register and qualification limits

### BRN, exact inspected revision

Base: `https://github.com/ewq100/brn-rust/tree/b139221f295f3e2d8f22045c21ef7d3d08a7b74e`

Read or available from the source inspection in this conversation:

- `AGENTS.md`; `Cargo.toml`; `docs/status.md`.
- `docs/architecture/overview.md`; `dependencies.md`; `invariants.md`.
- `docs/development/workflow.md`; `verification.md`.
- `docs/work/active/writing-polish/plan.md`.
- `docs/work/active/knowledge-retrieval/plan.md` (relevant tasks inspected; long tool output was truncated).
- `docs/work/active/rust-luna-intake/plan.md`.
- `crates/brn-threads-intake/src/lib.rs` (lines 1–260).
- `crates/brn-threads-core/src/writing.rs`; `crates/brn-threads-app/src/writing.rs` (relevant portions).
- `crates/brn-ai/src/threads.rs` and `Cargo.toml` (relevant portions).
- `crates/brn-desktop/src/threads_writing.rs` (relevant portion).

Remote main and open-PR collection were read on 15 October 2026. Local worktrees, unpushed changes and actual current build performance were not observable here. C0 must refresh and complete the source trace; this is not an exhaustive audit.

### External primary sources, checked 15 October 2026

- Zorite editor documentation: `https://packetthrower.github.io/zorite/reference/crates/zorite-editor/`
- Zorite editor manifest: `https://github.com/packetThrower/zorite/blob/main/crates/zorite-editor/Cargo.toml` — inspected content declares `0.11.2`, MIT; this is a main-branch manifest, not verification of a compatible published release. Blob observed: `d7cb4ecf61d257bec53e917314516c3e820778ba`.
- Zorite workspace manifest: `https://github.com/packetThrower/zorite/blob/main/Cargo.toml` — relevant first 180 lines inspected. Blob observed: `58f0d3a5f2d1fa0fad081f98133b154bf7d6095e`. Full patches/features still require qualification.
- Kreuzberg legacy documentation: `https://docs.kreuzberg.dev/` — describes v4 LTS and points to Xberg.
- Xberg documentation: `https://docs.xberg.io/`
- Xberg repository: `https://github.com/xberg-io/xberg` — describes a rebranded fresh v1 line. Exact release, Rust dependency and feature pin must be established by A; no advertised speed/coverage claim is treated as a BRN benchmark.

External projects and docs can change. Pin exact qualified artifacts and preserve decision evidence. This plan does not claim that Zorite or Xberg builds, passes BRN's fixtures, or is ready for adoption.
