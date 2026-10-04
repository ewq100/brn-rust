# Current development status

2026-10-04. The owner has authorized sequential BRN v1 delivery under the frozen
[product vision](product/BRN_PRODUCT_VISION.md), [architecture](architecture/overview.md#frozen-target),
[invariants](architecture/invariants.md), [roadmap](roadmap.md) and
[development workflow](development/workflow.md). **Stages 1–4 are implemented,
automated verified and locally integrated. Stage 5 knowledge foundations is active; Stages 5–16 remain
unfinished. Complete BRN v1 delivery is not claimed.**

Manual Save/recovery (`6609442`) preserves exact UTF-8, generation-bound recovery
and file/parent/root identities. Copies install exclusively; missing originals
are not recreated. Acknowledgement establishes recoverability. Guarded navigation
and Quit drain admitted work; system termination can lose unacknowledged typing.
[Stage 1 evidence](work/completed/simple-save/plan.md) retains checks and native
acceptance. Stage 2 (`a5ec4ae`) removes legacy production paths, `brn-core` and
`brn-flow`. The six-crate workspace refuses old/mixed markers before SQLite without
migrating original data. [Stage 2 evidence](work/completed/legacy-removal/plan.md)
records the retired baseline and qualification.

Stage 3’s scoped live round is complete (`d40e0a1`): both fresh human connections
succeeded, using **10 logical probes / 14 completion attempts** within the 11/22
cap. ChatGPT `gpt-5.5` Responses passed low/high read tools and the tiny image.
Copilot `gpt-5.3-codex` Responses passed read tools but reversed image colors.
Both observed hosted web search with correct official SQLite URLs; native citation
metadata remains unqualified. Copilot `gpt-5.5` Chat refused with
`unsupported_api_for_model`; production reports `ModelRefused` without fallback.
[Stage 3 evidence](work/completed/provider-capabilities/plan.md) records limits.
That completed round authorizes no further account calls or model downloads.

Stage 4 Proposal Core is implemented through sequential reviewed slices. Complete
typed Create/Replace/Trash drafts preserve exact before/source bindings and full
review text. Temporary comments retain uncertain quotes without guessing anchors;
edits, comments, rejection and owned Rewrite use one exact review version. Rewrite
shares the owned chat lane, captures explicit provider/model/effort and settles
validated output atomically; restart interrupts without retry. Fresh Ask also
requires explicit low/medium/high effort, while older unknown-effort history stays
readable and replayable offline.

Exact individual/captured-group approval, whole-file application and ordinary
proof recovery are shared by AppWorker/CLI and native controls. Fresh pre-effect
refusal preserves review work; unknown/partial effects stay fenced. Historical
replay never repeats installation and preserves later files and typing. Applied
cleanup removes covered temporary annotations. Paged Activity loads only the
identified full snapshot. Full Undo, original-index Trash restoration and explicit
Finish/Restore repair freeze exact previews/attempts; workflow rechecks eligibility
and retains errors/outcomes without automatic retry. Native initial composition
creates full one-note proposals; CLI supports multi-member requests/groups.
Completed acknowledged answers may explicitly prefill session-bound review work;
real seed drafts/comments/owned Rewrite provide AI writing. Unsubmitted form input
is transient, copyable and guards leaving; only acknowledged creation recovers.
Knowledge changes only after exact approval.

[Stage 4 evidence](work/completed/proposal-core/plan.md) retains each baseline,
independent reviews, technically verified fixes and reproducible manual scenarios.
The final creation slice’s independent review found no actionable defects and
passed **6 default / 6 native creation tests, 4 source-worker tests and 1 widget
test**. Fresh root macOS arm64 / Rust 1.98.1 locked/offline gates passed **684
workspace tests, 0 failed, 2 ignored**, **52 end-to-end assertions**, retirement,
format/build/all-target Clippy with warnings denied and **147 native tests**,
shipping native build and native Clippy. Default desktop passed **131**; workflow
**186**. The two ignored private crash entry points are exercised by subprocess
matrices. Earlier slice counts remain in the evidence, not current gate claims.

Stage 5’s [active plan](work/active/knowledge-foundations/plan.md) has integrated
managed Markdown UUID preparation through ordinary proposals, edit/Rewrite
identity protection, fresh complete evidence lookup and duplicate/incomplete
reporting, explicit exact archived reads, saved classification and scoped
current/source/history/all retrieval through workflow/CLI, the three AI read tools
and native browsing/search with read-only evidence views. Durable exact vault
provenance now lives in ordinary Markdown: fresh UUID/hash/range/quote capture
prepares full proposals with archived source bindings, and normal approval checks
new citations after edits/Rewrite. Inspection retains exact quotes through source
moves, changes, absence/ambiguity and incomplete inspection. A fresh independent
vault/store with only copied Markdown proves session/index independence; actual
session Delete remains later qualification. Source CAS, restart, index rebuild
and Undo are verified; current write/read rules and unresolved-work fences remain
intact. Independent reviews found no remaining actionable findings. Fresh final
locked/offline verification passed **809 workspace tests / 0 failed / 2 ignored**,
**52 end-to-end assertions**, format/build/all-target Clippy, **168 native desktop
tests**, native all-target Clippy and the shipping native desktop build.
Fresh full-byte refresh catches retained-size/mtime changes;
unreadable evidence folders report incomplete inspection while readable current
knowledge remains usable. Fresh native synthetic observation confirmed scope
separation, archived/current-history read-only views, typing refusal and an exact
86-byte BOM/CRLF/Unicode Copy→paste→Save Copy result, preserving original files.
The Mac locked before the final GUI restart check; owner acceptance remains pending.
The provenance review's valid malformed-metadata eligibility defect was reproduced,
fixed and independently rechecked; no actionable finding remains. Its corrected
review passed 32 checks and an actual synthetic CLI reproduction. Native saved-
source inspection preserves unsaved typing, binds late replies to document/
inspection generations and invalidates inspections after Save/Reload. Exact
read-only quote widgets retain all source outcomes; a rendered twelve-citation
regression reproduced and corrected missing overflow scrolling. Final independent
review passed 15 checks, with no remaining actionable finding. Session timestamps
now retain known creation/activity/start/finish times, explicit unknown legacy
values and stable replay/restart observations. Native history shows recorded
activity age. A valid concurrent-summary defect was reproduced and corrected with
one SQLite read snapshot; independent correction review sampled 3,040 summaries
during 1,000 attached writes with no inconsistencies. Fresh synthetic native
startup and CLI checks passed for new data and a V7 database upgrade. Native
GUI/owner acceptance remains pending. Relationships, basic findings and multilingual
qualification continue next;
Stage 5 is not complete.

## Qualification still open

Actual GUI/IME/accessibility, chooser, native Save/recovery/review/approval/Undo/
repair/creation usability, live Rewrite/effort usability and owner acceptance are
pending. Synthetic state/crash/widget tests do not establish physical power-loss
durability, other-volume support, actual inference or release readiness. These
items do not block later safe implementation. The completed provider round leaves
Copilot GPT-5.5 Chat unsupported, Codex vision accuracy and native citations
unqualified. Upstream `block v0.1.6` retains a future-compiler warning.

Stages 1–4 are published in reviewed PRs16–21. Stage4C PR21 merged
`6601374996ecba7d461403aa6e892f1273bed28e` after exact18a315b run37190429533
passed macOSCore/UI/Retrieval and UbuntuSharedCore. Windows Unix metadata failure
leaves overallCIred. Merged tree equality,16widgets+52fixtures, shipping build and
two startup/restart runs passed with exact synthetic bytes and zero credentials.

Stage5A PR22 merged `a9f838295ea905bf25d05953fe03d02a2092dff7` after exact
4895916 run37191800152 passed macOSCore/UI/Retrieval and UbuntuSharedCore.
Windows Unix metadata failure leaves overallCIred. Merged tree equality,
16CLIidentity/inventory/scopes+52fixtures and two shipping startup/restart runs
passed with exact synthetic bytes and zero credentials.

Stage5B provenance/timestamps publication baseline `db9aee3` integrates reviewed
slices throughbaf3bee with qualified Stage5A. Independent review verified
36Stage5-only+13incoming-only exact paths and both source overlaps, with no
actionable defect. Fresh macOS arm64/Rust1.98.1 locked/offline checks passed
809workspace/0failed/2ignored+52fixtures, retirement,format/build/all-target Clippy;
168combined-native/0failed/0ignored, both native Clippy variants and shipping
desktop/CLI builds;136focused-native-workflow/0failed/2ignored. Two shipping
startup/restart runs preserved exact BOM/CRLF/Unicode bytes with zero credentials.
PR23 initial CI passed both Mac native lanes but Ubuntu rejected an unused
Mac-only provenance-test helper. The narrow independently reviewed matching
helper guard passed fresh3CLI provenance tests and workspace Clippy; new exact
head applicable CI and post-merge verification remain pending. GUI/owner
acceptance remains separate. Later relationships/findings/language checkpoints
remain pending; Stage5 is not complete.

Release/public distribution, additional live calls/model downloads, purchases and
original/private-data inspection or migration still need applicable owner
permission. No original data was migrated or inspected.

Older records in [completed evidence](work/completed/README.md) and the
[earlier Rig notes evidence](work/active/simple-rig-notes/evidence.md) are history;
their specifications, process assignments and permissions are not execution plans.
