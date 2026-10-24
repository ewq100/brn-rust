# BRN decision backlog

Canonical product-disposition list for this audit. Execution remains in the
[selected task record](../daily-followups-inbox-restore/record.json); findings and fact ownership are in the
[documentation audit](documentation-audit.md). Repository evidence was reconciled
against main `f30f37e0dc50fdcf91e97a77bc7cda9acb678d78`. The original daily-use batch subsequently merged as `f60c5a2f` (PR226);
its [qualification](qualification.md) remains source-bound evidence. New follow-up
batch changes are not counted as shipped. All 71 curated chat topics and 14 dot-room aliases are reconciled in the [source map](chat-source-map.md); its coverage limits apply to this backlog.

Stable IDs survive reordering. `Verified shipped` means source/evidence present
on that main, not fresh tests, release readiness or owner acceptance. `Partial`
means some useful implementation/evidence exists but the named outcome remains
incomplete. `Not built` is bounded to the specified capability, not a claim that
no related primitive exists. `Unknown` names an evidence gap. The final column is
reserved for the owner's **Keep / Later / Drop** choice; only the five explicit
narrow decisions below are filled. Existing
scope/authority is not overridden by a blank choice, and no new feature is
assigned merely by listing it. Shipped entries prevent duplicate implementation;
Drop would require a separate concrete retirement assessment before removal.

## Latest owner decisions — 24 October 2026

Source `Sentinel_4a4c26b62cf48191a984cea7ea942e63` approved “all 3” of the
immediately preceding bounded proposal. The [selected baseline](../daily-followups-inbox-restore/baseline.md)
records the scope and earlier calculator deferral from
`Sentinel_c40a2bce390c819198a4113ceb9717ae` (03:21 UTC).

- **D034/D039 Keep, narrow:** meeting-note follow-ups with Action owner, waiting-on
  and follow-up date. Dependencies, decomposition, recurrence and external sending
  remain unapproved by this choice.
- **D032 Keep, manual scan only:** one folder, explicit Scan inbox, status and safe
  retries; original files untouched. Watched-folder/daemon behavior remains later.
- **D013 Keep:** safe desktop Restore using the existing engine.
- **D015 Later:** calculator/chart/report remains deferred, not activated.

All other decisions remain as recorded; blank cells are not approvals. These choices
assign only the stated implementation scope, not new merge/release/Mac access.

## Start with these choices

The owner approved the next follow-ups, manual inbox and desktop Restore batch.
No additional choice is required for that narrow scope. These groups make later
review manageable; remaining blank decisions and implementation evidence are in
the expandable register below.
The owner can reply with individual IDs and Keep / Later / Drop; no default or broader choice
has been inferred.

- **Useful outputs:** calculator/chart worker; later DOCX, PPTX and HTML exports ([D015](#brn-d015), [D016](#brn-d016), [D017](#brn-d017), [D018](#brn-d018)).
- **Everyday intake:** manual inbox first, later watched behavior, URLs, meeting-note extraction and figure fidelity ([D007](#brn-d007), [D032](#brn-d032), [D033](#brn-d033), [D034](#brn-d034)).
- **Writing and knowledge:** style/templates, profiles/preferences, maintenance/organization and graph ([D019](#brn-d019), [D020](#brn-d020), [D021](#brn-d021), [D030](#brn-d030), [D031](#brn-d031), [D035](#brn-d035), [D036](#brn-d036), [D037](#brn-d037), [D038](#brn-d038), [D051](#brn-d051)).
- **Daily controls:** desktop Restore, richer Actions, chat lifecycle and UI concepts ([D013](#brn-d013), [D039](#brn-d039), [D040](#brn-d040), [D041](#brn-d041)).
- **External/platform scope:** read access, providers, connectors/sending, sync, mobile, encryption and Windows ([D022](#brn-d022), [D023](#brn-d023), [D025](#brn-d025), [D026](#brn-d026), [D027](#brn-d027), [D042](#brn-d042), [D043](#brn-d043), [D046](#brn-d046), [D047](#brn-d047)).
- **Evidence and delivery:** native acceptance, model comparisons, package performance and release readiness ([D008](#brn-d008), [D010](#brn-d010), [D011](#brn-d011), [D024](#brn-d024), [D048](#brn-d048), [D049](#brn-d049), [D050](#brn-d050)).
- **Alternative directions:** Pi/runtime exploration and development-tool alternatives ([D044](#brn-d044), [D045](#brn-d045)).

Existing shipped controls and engineering policies are retained as reference,
not presented as undone work. Historical architecture conflicts are resolved by
the later SQLite/Threads direction in the source map; this audit does not reopen
them. “Keep” selects a direction for later concrete scoping, not blanket execution,
merge, spending, external access or release permission.

<details>
<summary>Full decision register, including shipped baselines and separate future choices</summary>

| ID | User-visible outcome | Implementation status | Source evidence / limits | Prerequisite or next evidence | Owner choice: Keep / Later / Drop |
| --- | --- | --- | --- | --- | --- |
| <a id="brn-d001"></a>BRN-D001 | Dependable notes, checked changes, history and Undo | Verified shipped | [Core](../../../../crates/brn-threads-core/src/store.rs), [invariants](../../../architecture/invariants.md); later edits/protection/version guards remain required. | Retain existing service; native acceptance is D011. | |
| <a id="brn-d002"></a>BRN-D002 | Durable chats, attachments, queue and interruption recovery | Verified shipped | [Chat service](../../../../crates/brn-threads-app/src/chat.rs), [chat plan/evidence](../chats-and-projects/plan.md), [daily workflow tests](../../../../crates/brn-threads-app/tests/daily_workflow_eval.rs). | Broader daily outcome evidence D008; no new chat engine needed. | |
| <a id="brn-d003"></a>BRN-D003 | Private Projects and safe deletion/restoration | Verified shipped | [Project service](../../../../crates/brn-threads-app/src/projects.rs), [tests](../../../../crates/brn-threads-app/tests/projects.rs), [invariants](../../../architecture/invariants.md). | Preserve scope/dirty/unfinished-Action choices; actual owner acceptance separate. | |
| <a id="brn-d004"></a>BRN-D004 | Find knowledge through search, Topics and Connections | Verified shipped | [Retrieval](../../../../crates/brn-retrieval/README.md), [topics tests](../../../../crates/brn-threads-app/tests/topics_knowledge.rs), [Connections UI](../../../../crates/brn-desktop/src/threads_connections.rs). English-first; full graph drawing is not implied. | Corpus/language quality D019; native breadth D010/D011. | |
| <a id="brn-d005"></a>BRN-D005 | Optional read-only research helpers with checked citations | Verified shipped | [Runtime](../../../../crates/brn-ai/src/threads_research.rs), [accepted decisions](../subagents/plan.md). Experimental/off by default; no helper writes, automatic fallback or UI usage display. | Later specialized roles D020 need distinct decision. | |
| <a id="brn-d006"></a>BRN-D006 | Owner-installed instruction skills and one approved local MCP tool connection | Verified shipped | [Setup contracts](../../../development/setup.md), [skills tests](../../../../crates/brn-threads-app/tests/skills.rs), [MCP tests](../../../../crates/brn-threads-app/tests/mcp.rs). Instructions do not grant execution; MCP is anonymous literal loopback with exact invocation review. | Broad connectors remain D022; no general plugin platform implied. | |
| <a id="brn-d007"></a>BRN-D007 | Useful/full document intake that retains supported figures honestly | Partial | [Intake contract](../../../../crates/brn-intake/README.md), [current helper](../../../../tools/xberg-intake-helper/src/project.rs). Eight-kind routes plus bounded PNG/JPEG and reference-image retention shipped in PR226; broad format/fidelity remains incomplete. | Preserve PR226 qualification, exact hashes/provenance and hostile input bounds; later native/fidelity evidence remains separate. | |
| <a id="brn-d008"></a>BRN-D008 | Documents → grounded knowledge → useful answer or writing, with recovery | Partial | [Existing journey tests](../../../../crates/brn-threads-app/tests/agent_eval_journeys.rs), [wiki evaluator](../../../../scripts/evaluate_wiki_v2.py), [retrieval evidence](../knowledge-retrieval/evidence.md). Deterministic success is not broad live usefulness. | PR226 deterministic extension is merged; separately bounded live evidence remains necessary where authorized. | |
| <a id="brn-d009"></a>BRN-D009 | Precise edits with exact preview, Apply, persisted reopen and Undo | Verified shipped | [Precise edit qualification](../precise-edit-ui-qualification/README.md), [later real-model Mac headless result](../precise-edit-ui-qualification/mac-live.md). One successful narrow live case; broader semantic/native acceptance not implied. | D011 for actual Mac UI; don't repeat narrow live run merely due stale plans. | |
| <a id="brn-d010"></a>BRN-D010 | Dependable Linux native menus, Connections and accessibility | Partial | [Linux records](../linux-native-gaps/README.md), [later disabled-state qualification](../precise-edit-ui-qualification/README.md). Bounded actual observations exist; picker blocked and full screen-reader behavior unproved. | Legitimate portal access and broader accessibility acceptance. | |
| <a id="brn-d011"></a>BRN-D011 | Actual Mac daily-use UI, focus/accessibility and owner acceptance | Partial | [Baseline native checklist](baseline.md), [historical native queue](../vacation-campaign/native-queue.md). Some dated native evidence exists; CLI/live success is not this gate. | Separate authorized Mac session; no Mac access in this cloud batch. | |
| <a id="brn-d012"></a>BRN-D012 | Complete backup/restore and recovery without losing later work | Verified shipped | [Application contract](../../../../crates/brn-threads-app/README.md), [file workflow evidence](../linux-native-gaps/file-workflows.md). CLI and storage behavior implemented; desktop Restore control excluded from this status. | D013 for desktop path; actual owner restore acceptance remains distinct. | |
| <a id="brn-d013"></a>BRN-D013 | Restore a full backup through an understandable desktop flow | Not built | [File workflow record](../linux-native-gaps/file-workflows.md) explicitly reports no desktop full-backup Restore control. Existing CLI restore is D012. | Approved narrow UI scope; reuse existing restore validation/service; later native qualification. | Keep — safe existing-engine desktop flow |
| <a id="brn-d014"></a>BRN-D014 | Portable Markdown/assets snapshot export | Verified shipped | [Export service](../../../../crates/brn-threads-app/src/export.rs), [application contract](../../../../crates/brn-threads-app/README.md). New snapshot destination, retained source provenance; not live external editing. | Keep atomic publication/asset integrity; Additional formats D016–D018. | |
| <a id="brn-d015"></a>BRN-D015 | Exact calculations, managed charts and reviewable reports | Not built | [Design proposal](artifact-proposal.md), [M5 proof](../agent-first-simplification/artifact-capability.md). Disposable calculation/SVG proof exists; no production tool. | Separate activation decision, dependency/containment/raster/cancellation qualification. | Later — calculator remains deferred |
| <a id="brn-d016"></a>BRN-D016 | Export a useful DOCX document from selected knowledge | Not built | [Target output formats](../../../architecture/threads-target.md#later-document-presentation-and-html-output), CLI export currently Markdown only. DOCX intake does not implement DOCX generation. | Separate output design and maintained writer/preview qualification; no round-trip requirement. | |
| <a id="brn-d017"></a>BRN-D017 | Export presentations as PPTX | Not built | [Product scope](../../../product/BRN_PRODUCT_VISION.md), [target](../../../architecture/threads-target.md). Input PPTX conversion is distinct. | Later output decision; explicitly outside current batch. | |
| <a id="brn-d018"></a>BRN-D018 | Export shareable HTML | Not built | [Product scope](../../../product/BRN_PRODUCT_VISION.md), current [export](../../../../crates/brn-threads-app/src/export.rs) writes Markdown/assets. | Later bounded output design; hosting remains separate/out of scope. | |
| <a id="brn-d019"></a>BRN-D019 | Reliable retrieval beyond English, including Estonian | Partial | [Retrieval language scope](../../../../crates/brn-retrieval/README.md): multilingual model exists, English qualified; Estonian held by earlier owner decision. | Owner language choice plus analyzer/corpus qualification; no canonical-store migration needed. | |
| <a id="brn-d020"></a>BRN-D020 | Specialized digest, triage or draft-verification helper roles | Not built | [Helper design/decisions](../subagents/design.md); current generic research can digest long notes, but separately packaged roles remain later choices. | Demonstrated advantage over existing research and distinct scope/budget/evidence contract. | |
| <a id="brn-d021"></a>BRN-D021 | A replacement editor or richer editing engine | Not built | [Zorite experiment](../modular-foundation/experiments/z1-zorite.md): gate 2 failed, not adopted; current editor retained. | Concrete user-visible limitation and independent qualification; rewrite excluded from current batch. | |
| <a id="brn-d022"></a>BRN-D022 | Broad authenticated/remote connectors | Not built | [Canonical scope](../../../architecture/threads-target.md), [local MCP contract](../../../development/setup.md). Narrow reviewed MCP D006 does not implement broad connectors. | Separate connector-specific purpose, access and authority decision; outside batch. | |
| <a id="brn-d023"></a>BRN-D023 | A supported Windows implementation | Partial | [Verification guide](../../../development/verification.md), latest baseline records optional Windows failures; existing portable code is not supported/native qualification. | Explicit platform scope and diagnosed failures; outside batch. | |
| <a id="brn-d024"></a>BRN-D024 | Ship another accepted release | Unknown | [Roadmap](../../../roadmap.md) records earlier previews; [preview process](../../../development/preview-release.md) is procedure, not current readiness evidence. | Fresh source/package/signing/clean-machine/native/owner gates and separate release authority. No current release investigation performed. | |
| <a id="brn-d025"></a>BRN-D025 | Send email/messages or perform external delivery from BRN | Not built | [Product scope](../../../product/BRN_PRODUCT_VISION.md) defers direct external sending; preparing a reply is not sending it. | Destination/recipient/action authority and evidence of actual delivery; outside batch. | |
| <a id="brn-d026"></a>BRN-D026 | Sync or collaborate across devices/users | Not built | [Canonical scope](../../../architecture/threads-target.md) defers sync/multiplayer; SQLite backup is not sync. | Separate consistency, privacy and conflict design; outside batch. | |
| <a id="brn-d027"></a>BRN-D027 | Recurring automation or background execution | Not built | [Canonical scope](../../../architecture/threads-target.md) selects manual intake and defers daemon/full recurring machinery. The historical weekly-on-next-launch choice (D036) and watched folder (D032) are distinct from an always-running daemon. | Define the exact trigger/outcome from owner evidence; no watcher or schedule assigned here. | |
| <a id="brn-d028"></a>BRN-D028 | Clear, compact development instructions and documentation | Partial | [AGENTS](../../../../AGENTS.md), [strategic programming](../../../development/strategic-programming.md), [this audit](documentation-audit.md). Existing guidance and PR226 factual corrections are shipped; the new selected-task routing is being updated. | Preserve dated evidence and distinguish runtime from development prompts; validate each selected-task routing change. | |
| <a id="brn-d029"></a>BRN-D029 | A visible development cycle with truthful progress and gates | Partial | [Renderer](../../../../scripts/render-work-overview.py), [workflow](../../../development/workflow.md), [selected record](record.json). Generated visibility exists; old remote campaign projection is not current live execution. | Keep one source record and current projection; any new remote refresh/automation needs its own authorized scope. | |
| <a id="brn-d030"></a>BRN-D030 | A separate writing app after proving the internal module | Partial | [Writing module](../../../../crates/brn-writing-ui/README.md) and writing_host exist; separate product not established. | Keep internal qualification distinct; separate-app product decision remains open. | |
| <a id="brn-d031"></a>BRN-D031 | Learn approved writing style and suggest reusable document templates | Unknown | [Installed skills](../../../development/setup.md) provide instructions; an automatic approved-output learning/template workflow is not established by this audit. | Define owner-controlled learning and template adoption; inspect skills before adding a subsystem. | |
| <a id="brn-d032"></a>BRN-D032 | A document inbox: manual scan first, watched behavior later | Not built | [Target intake](../../../architecture/threads-target.md) and [current adapter](../../../../crates/brn-threads-intake/src/lib.rs) cover explicit intake; a watched-folder service is not established. CHAT-012 repeats the idea. | Implement explicit manual scan/status/retry only; watched behavior still requires a later decision. | Keep — manual Scan inbox only; no watcher |
| <a id="brn-d033"></a>BRN-D033 | Import web URLs with useful content and origin/date | Unknown | [Source tools](../../../../crates/brn-ai/src/threads_sources.rs) retain supplied-source provenance; this audit does not establish a production arbitrary-URL fetch workflow. | Verify actual URL admission/host path and scoped network policy before design. | |
| <a id="brn-d034"></a>BRN-D034 | Turn meeting notes into decisions, actions and unresolved questions | Partial | [Intake skill](../../../../agent/skills/intake/SKILL.md), [Action type](../../../../crates/brn-threads-core/src/types.rs) and existing agent tools can express outcomes; semantic quality is not proved. | Approved current tools/skills plus narrow Action follow-up fields; no dedicated meeting subsystem implied. | Keep — narrow owner/waiting-on/follow-up scope |
| <a id="brn-d035"></a>BRN-D035 | Maintain person/project profiles, aliases and relationship context | Partial | [Topics](../../../../crates/brn-threads-app/src/topics.rs) and generic Notes/Links exist; complete profile/alias behavior is not qualified. | Separate useful note conventions from any justified specialized model. | |
| <a id="brn-d036"></a>BRN-D036 | Quiet stale/conflict checks, including weekly checks on next launch | Unknown | [Maintenance skill](../../../../agent/skills/maintain-notes/SKILL.md) supports agent maintenance; a weekly launch trigger is not established. | Preserve historical next-launch choice; define bounded trigger/notice policy separately from D027 daemon. | |
| <a id="brn-d037"></a>BRN-D037 | Propose organization for an empty or messy knowledge base | Partial | [Maintenance skill](../../../../agent/skills/maintain-notes/SKILL.md), Topics and checked changes provide primitives; end-to-end organization quality unqualified. | Evaluate current agent on representative messy input without user-authored metadata. | |
| <a id="brn-d038"></a>BRN-D038 | Maintain owner working preferences without learning accidental defaults | Unknown | [Settings](../../../../crates/brn-threads-app/src/settings.rs) exist; a distinct learned working-preference profile is not established. | Current instructions must override saved defaults; define explicit adoption/revocation. | |
| <a id="brn-d039"></a>BRN-D039 | Richer Action ownership, follow-up dates, dependencies and decomposition | Partial | [Current Action](../../../../crates/brn-threads-core/src/types.rs) has state/description/evidence/due; generic Links exist. Separate follow-up/blocked/owner/dependency UX is not established by old proposal types. | Implement only owner/waiting-on/follow-up fields; preserve real completion; dependencies/decomposition and D027 recurrence remain outside scope. | Keep — owner/waiting-on/follow-up only; no dependencies/decomposition/recurrence |
| <a id="brn-d040"></a>BRN-D040 | Choose roles for Living Brief, Maintained Brain and Thinking Desk | Unknown | [UI records](../tiled-writing-workbench/plan.md) and current workspace do not prove adoption of these three later concepts. | Exploration only: decide useful role(s) before production modules; avoid three parallel workspace rewrites. | |
| <a id="brn-d041"></a>BRN-D041 | Archive inactive chats and warn before losing uncaptured outcomes | Partial | [Chat service](../../../../crates/brn-threads-app/src/chat.rs) supports durable chats/archive; automatic 30-day archive and uncaptured-outcome warning are not established. | Reconcile historical policy with current Project/conversation model and explicit deletion behavior. | |
| <a id="brn-d042"></a>BRN-D042 | Expose bounded read access to external Codex/ChatGPT agents | Partial | [CLI contract](../../../../crates/brn/README.md) enables external callers; BRN as a remotely exposed knowledge server is not established. | Clarify local CLI versus server/API need; existing outgoing local MCP is not an incoming server. | |
| <a id="brn-d043"></a>BRN-D043 | Subscription/model/reasoning choice with understandable privacy and usage | Partial | [Provider/setup](../../../development/setup.md) and [runtime](../../../../crates/brn-ai/README.md) exist. [Recorded cloud admission failure](../precise-edit-ui-qualification/evidence/live-admission.json) (2026-10-23) and missing universal cost/privacy evidence remain separate. | Qualify chosen existing account route; no credentials/security changes or assumed provider parity. | |
| <a id="brn-d044"></a>BRN-D044 | Evaluate Pi or another runtime/UI stack as an alternative direction | Not built | [Current architecture](../../../architecture/overview.md) is Rust/Rig/GPUI; chat requested research, not an approved Pi/Bun/Electron/TUI rewrite. | Require comparison and an explicit replacement decision; preserve current working paths. | |
| <a id="brn-d045"></a>BRN-D045 | Evaluate development orchestration and low-cost triage alternatives | Unknown | [Development workflow](../../../development/workflow.md) is current; Kiro ACP/T3/Pi orchestration, Jev trials and external Product-to-Production skill adoption are not verified repo features. | Development tooling only; measure benefit, keep quota/security constraints, and avoid runtime scope creep. | |
| <a id="brn-d046"></a>BRN-D046 | A mobile client | Not built | [Product scope](../../../product/BRN_PRODUCT_VISION.md) defers mobile; desktop portability is not a mobile implementation. | Separate client/platform decision; sync D026 is another choice. | |
| <a id="brn-d047"></a>BRN-D047 | App-level lock or encryption | Not built | [Product scope](../../../product/BRN_PRODUCT_VISION.md) excludes this from v1; local storage is not an app-lock/encryption guarantee. | Separate threat model, recovery and platform decision; no security settings changed here. | |
| <a id="brn-d048"></a>BRN-D048 | Broader model/reasoning and helper-delegation benchmarks | Partial | [Wiki evaluator](../../../../scripts/evaluate_wiki_v2.py) and [journeys](../../../../crates/brn-threads-app/tests/agent_eval_journeys.rs) exist; current extension does not prove a broad live model comparison. | Reuse existing framework; require supported admission, explicit bounds and actual usage/cost evidence. | |
| <a id="brn-d049"></a>BRN-D049 | Smaller packages and faster useful builds | Unknown | [Packaging workflow](../../../development/preview-release.md) exists; historical 700 MB observation is not a current measurement. | Measure current package/build before optimizing; no release implied. | |
| <a id="brn-d050"></a>BRN-D050 | Evaluate EmbeddingGemma against the current local embedding model | Not built | [Retrieval model](../../../../crates/brn-retrieval/README.md) pins multilingual-e5-small; EmbeddingGemma was a research question. | Controlled corpus/latency/size comparison before adoption; indexes stay rebuildable. | |
| <a id="brn-d051"></a>BRN-D051 | A simple visual graph of note relationships | Partial | [Connections](../../../../crates/brn-desktop/src/threads_connections.rs) and Links are implemented; a rendered spatial graph is not established. | Decide whether visual graph adds value over current Connections; do not duplicate relationship storage. | |

</details>

D022 originally grouped several deferred directions in the first uncommitted draft;
D025–D027 now separate those owner decisions. No choice or status history was lost.

Do not treat old `build-sequence` decisions D-S/D-M or old not-started tables as
new approvals needed for already-shipped D006. Conversely, a shipped primitive
never approves a broader future capability. The source map retains origin, latest direction and implementation status separately.
Future reconciliation keeps these IDs and aliases instead of copying another backlog
into chat notes or the execution record.
