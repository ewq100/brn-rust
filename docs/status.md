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

Stage 5’s [active plan](work/active/knowledge-foundations/plan.md) records ten
integrated slices: managed Markdown identities; fresh duplicate/incomplete
inspection; current/source/history/all retrieval and native read-only browsing;
durable exact provenance and native source inspection; reliable session/turn
timestamps; saved CommonMark links; disposable relationships; and exact stable-link
preparation. Fresh whole-byte observations detect retained-size/mtime changes. UUID links survive moves without
path/title guesses. Saved citations retain their exact historical quotes and
source uncertainty independently of sessions or disposable indexes.

Relationship pages distinguish explicit links from provenance-based candidates,
retain full endpoint hashes and exact proofs, filter both endpoints before
pagination and report duplicates/incomplete inspection. Healthy index V2 upgrades
additively to V3, preserving passages/vectors. No AI or durable write occurs during
reconstruction. Independent review reproduced a valid reference-proof boundary;
the corrected 8,192-proof cache preserves the accepted 4,096-link extractor. An
introduced absent-metadata update regression was also reproduced and corrected.
That slice’s final review passed **45 tests** with no actionable finding, plus
actual CLI probes for **8,192 exact proofs** and **5,000 notes / 4,999 edges**.
The two scale queries took **6.108 s / 10.026 s** on this Mac; each relationship
request currently rederives the saved vault observation.

Stable-link preparation returns full additive Replace review input with both
consumer/target source bindings and no admission or vault write. It preserves the
entire byte prefix, escapes literal labels and verifies parser placement. Fresh
approval checks new UUID targets after edits/Rewrite against exact saved evidence
or same-draft reviewed target bytes. Historical links, Undo and completed replay
retain their authority. Broader tests caught and fixed an unrelated legacy-layout
regression. Independent public probes also reproduced and verified fixes for
case-sensitive aliases hiding duplicates and opaque metadata supplying false old
link authority. Final review passed **62 tests / 0 failed / 1 ignored**, and the
ignored real App case separately passed on fresh owned case-sensitive APFS.

Fresh final Rust 1.98.1/macOS arm64 locked/offline verification passed **884
workspace tests / 0 failed / 3 ignored**, **52 end-to-end assertions**, retirement,
format/build/all-target Clippy, **168 native desktop tests**, **164 native workflow
tests / 0 failed / 3 ignored**, **7 native CLI process tests**, native Clippy
configurations and the shipping native desktop build. Two ignored private crash
entry points remain exercised by subprocess matrices; the third case-sensitive
regression was explicitly qualified. Shipping headless startup/restart and the
synthetic prepare/Create/Approve/rebuild scenario passed with exact original byte
prefix/source bytes and no credential files, provider calls or downloads. Actual
ONNX inference was not exercised. Earlier native scope/Copy/Save Copy observation
is retained in the plan; final GUI restart and owner acceptance remain pending
because the Mac is locked.

Native relationship inspection/preparation controls, basic review findings and
multilingual implementation/qualification remain Stage 5 work. The bounded
multilingual asset-download permission question is still pending; unrelated safe
implementation continues. Stage 5 is not complete.

## Qualification still open

Actual GUI/IME/accessibility, chooser, native Save/recovery/review/approval/Undo/
repair/creation usability, live Rewrite/effort usability and owner acceptance are
pending. Synthetic state/crash/widget tests do not establish physical power-loss
durability, other-volume support, actual inference or release readiness. These
items do not block later safe implementation. The completed provider round leaves
Copilot GPT-5.5 Chat unsupported, Codex vision accuracy and native citations
unqualified. Upstream `block v0.1.6` retains a future-compiler warning.

No published CI run exists for the inspected local Stage 4 commits. The latest
inspected [published main CI](https://github.com/ewq100/brn-rust/actions/runs/37137393000)
at another commit (`609d859`) failed on Windows and optional Linux paths; it does
not qualify this tree. Release/public distribution, additional live calls/model
downloads, purchases and original/private-data inspection or migration still need
applicable owner permission. No original data was migrated or inspected.

Older records in [completed evidence](work/completed/README.md) and the
[earlier Rig notes evidence](work/active/simple-rig-notes/evidence.md) are history;
their specifications, process assignments and permissions are not execution plans.
