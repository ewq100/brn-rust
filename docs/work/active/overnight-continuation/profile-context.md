# Explicit person/project context over saved records

Selected next ready P8 outcome, 9 October 2026 01:36UTC. Owner authorizes continued
small V1 outcomes through05UTC, independent review/qualification/normal protected
integration. P7 actual-sent completion is qualifying separately; incorporate its
eligible merged main before this candidate integrates. No GUI or provider calls;
campaign16/16used. No private data, account/release/config changes or new dependency.

## Outcome and bounded semantics

From one saved Current managed note, explicitly choose Person context or Project
context. Show its full saved version, approved retained Actions whose existing
related_person/related_project equals that exact UUID, direct incoming/outgoing
saved-note relationships, and the displayed Actions' explicit Source/thread UUIDs.
Open supporting saved Markdown through existing guarded navigation and inspect
exact relationship proofs. Completed Actions remain visible as Completed.

The lens is an explicit query choice, never a persisted/inferred profile type,
alias, membership or authority. Current metadata has no accepted Person/Project
registry. No synthesis, graph canvas, multi-hop exploration, AI tool, profile
creation, lifecycle, maintenance or semantic truth rule. Session Delete is parked:
its uncaptured-outcome/retention warning contract needs a separate exact spec.

Reuse/adapt existing fresh identity/relationship derivation, scoped saved Markdown,
checked Action pages and native read-only evidence/unsaved-work navigation. Extract
one private fresh relationship collection so context need not repeatedly rebuild
for internal pages. Do not add a projection store, graph/index engine or schema.
Pinned Rig has no role. Existing relationships use both-endpoints scope filtering;
context must derive ALL edges internally, then select direct UUID connections and
label each endpoint Current, Source or History. Never apply Current filtering before
collecting links to Source emails/History. These are fresh observations, not an
atomic vault snapshot; incomplete scans/issues are visible, never certified empty.

## Fixed public Workflow interfaces

In knowledge/context.rs, reexport through knowledge.rs:

- ProfileLens enum Person|Project, serde snake_case, Copy/Eq.
- ProfileContextRequest { profile: SourceVersion, note_id: Uuid, lens: ProfileLens,
  action_offset: usize, relationship_offset: usize, limit: usize }, strict serde.
  validate pure: existing contained visible Markdown path/fingerprint bounds,
  nonnil note_id, limit1..200; offsets checked arithmetic, no effects/startup.
- ProfileRelationship { edge: NoteEdge, source_scope: KnowledgeScope,
  target_scope: KnowledgeScope }. Only Current/Source/History, never All.
- ProfileNote { note: NoteIdentityInfo, scope: Option<KnowledgeScope> }.
  Unknown classification remains None with issue; never guessed Current.
- ProfileReference { resolution: IdentityResolution, matches: Vec<ProfileNote> }.
  Include exact union of displayed Action.sources and thread, once per UUID,
  sorted; matches correspond exactly to resolution.matches. Missing/duplicate/
  incomplete identity remains explicit; only Unique valid classified matches
  permit native navigation. A path cannot name different UUIDs across profile,
  edges or references, and a reference to the captured profile cannot claim absence
  or uncertain identity.
- ProfileContext { request: ProfileContextRequest, profile: ProposalSource,
  action_total: usize, actions: Vec<ActionRecord>, relationship_total: usize,
  relationships: Vec<ProfileRelationship>, references: Vec<ProfileReference>,
  issues: Vec<IdentityIssue>, duplicates: Vec<DuplicateIdentity>, complete: bool }.
  validate_for(&request) pure strict correlation/wholeprofile/fingerprint/id/Current
  metadata, Actionvalid+matchingrole, exact pagelen/totals, connected edgeUUID/proof
  shapes/endpointscope, reference union/matches/known scopes and complete nevertrue
  with issues/duplicates. Reuse validators; no authority granted by pure validation.

App::profile_context(&mutself,&request)->Result<ProfileContext>. Fresh current
fence/vault, exact full physical profile proof, Current classification and managed
UUID. Fresh identity must be Unique/exactpath; absent/ambiguous/incomplete refuses
with descriptive typed error, no false empty context. Obtain one fresh All
relationship observation/inventory; preserve issues/duplicates. Read checked Action
pages to exhaustion before deriving matching totals and paging; don't filter only
firstpage. Stable immutable Action creation ordering; keep full exact records.
Direct edges collected before paging; stable existing edge ordering. Both pagination
axes independent and no semantic conclusions from page counts. Resolve displayed
Action references using same inventory, read/compare exact saved matches for honest
classifications and append diagnostics on changed/unreadable evidence. Final
profile proof/fence checked again. No note or operational writes; disposable index
refresh is existing behavior. Do not claim continuous filesystem atomicity.

AppCommand::ProfileContext(ProfileContextRequest) ->
AppEvent::ProfileContext(Box<ProfileContext>), ordinary read-only query, not critical
mutation. Update exhaustive real-worker test mappings without weakening assertions.

CLI `context inspect --file REQUEST.json`, existing regular/nonblocking bounded
proposal/Action JSON reader, typed validate before workspace/credentials. Full exact
JSON response; human output quotes terminal controls through existing conventions.
Output must call validate_for original captured request; worker read dispatch included.
No new auth/model/network path.

## Fixed Desktop ownership / behavior

Native entry from selected saved Current note: two explicit Person context/Project
context controls. Capture saved document path, existing saved bytes/fingerprint,
managed UUID and selected lens, require clean acknowledged saved editor and no
busy/fenced work. No registry/type inference; button label is query lens only.
AiState ProfileContext generation/intent binds document generation, path/hash/id,
lens and both offsets. Reject wrong/late/malformed acknowledgements without
replacing retained view or settling another intent. Clear/invalidate on note,
scope, Refresh, Save/application/rebind and existing navigation lifecycle.

Show saved profile read-only, matching Actions with exact state/details and explicit
roles/references, direct incoming/outgoing relationship endpoints with scope and
origin labels, full exact proof reader/copy, counts/pagecontrols and coverage/issues.
No summaries or silently truncated exact evidence. Existing read-only text widgets
can display complete JSON for exact Action/context details as needed; user-facing
navigation labels explain content. Supporting links route through existing guarded
simple_scoped_note/EditorTransition; never bypass unsaved draft/comment guards.
Keep native surface functional and bounded, no broad visual redesign. Actual GUI
acceptance stays pending in ONE authoritative morning task after final runtime.

## Acceptance and verification

1. Same UUID Person/Project lenses match different explicit Action fields; unrelated
   rows excluded; Completed visible. Whole saved profile and exact records retained.
2. Direct Current-profile→Source email, History, incoming link and inferred provenance
   remain classified/directional and exact quote/hash proofs inspectable.
3. >200 underlying Actions/edges before matches proves totals/filtering/paging; no
   firstpage false empty. Invalid/stale/duplicate/missing/incomplete identity and
   changed source hashes refuse or show honest uncertainty without guessed results.
4. Restart/index deletion reconstructs equivalent domain view; Markdown/operational
   records unchanged, zero inference. Backend/worker/CLI/native equivalent DTOs.
5. Pure malformed request/output refusal before workspace or replacing owner review;
   native stale reply after selection/lens/page/Refresh/Save/rebind ignored, guarded
   supporting navigation preserved; fullproof/copy/paging realheadless widgets.
6. One independent complete read-only review, final applicable workspace/native/
   combinedClippy/shipping/fixtures/links, actual4requiredCI+docs, normal protected
   merge/resultmain verification. Reuse unchanged relevant gates honestly.

Lead keeps selected model/effort and owns plan/shareddocs/morningcases/integration.
At most2active bounded helpers, no recursion/GUI/provider. Backend helper owns ONLY
brn-workflow + brn CLI code/tests/READMEs; Desktop helper owns ONLY brn-desktop.
Root P7 Cargo currently owns target/budgets; P8 helpers initially noCargo. Later
explicit grant soleCargo target/intake-ui, canonical TMPDIR/private synthetic only.
Do not edit other checkout or shared status/docs outside owned contract. Stop/reassess
on unresolved product semantics or unsafe scan/lifecycle requirement; ordinary
technical implementation choices proceed. P8 independently ready from c1a97ef but
must incorporate P7 eligible main before integration; no second engine or rewrite.
