# BRN Threads independent prebuild review

**Repository:** `ewq100/brn-rust`  
**Target branch:** `rebuild/threads`  
**Historical code reference:** `af9239c741c7ab0983e62f0253e607b9607727e6`  
**Canonical target:** `docs/architecture/threads-target.md`  
**Build plan:** `docs/work/active/threads-rebuild/plan.md`  
**Only review report:** `docs/work/active/threads-rebuild/review.md`

## Recommendation and purpose

Use one independent Opus review before substantial core implementation. Its purpose is to find concrete implementation blockers and remove unnecessary mechanisms from the selected target. This is a bounded second opinion, not a request for another architecture proposal. Use Astra afterward only if a specific material issue remains disputed or unresolved; give it that issue, the evidence, and the competing minimal fixes. No second general review is required.

The build lead assesses each finding against the selected target, records its disposition, and makes any authorized correction. The reviewer does not edit the target, plan, implementation, or active instructions. A review verdict does not authorize implementation, account access, merging, or release.

## Eight questions to resolve

1. **What must commit atomically, and can the same operation safely be retried?** Identify the smallest coherent transaction across note revisions and current pointers, Action changes, intentionally retained assets, source observations, and the operation receipt. Check the case where the transaction commits but the process dies before returning success: who assigns and persists the operation ID before execution, and how does a retry reuse it without applying twice? Avoid a second event-sourcing or recovery authority.

2. **Can one mutation path enforce autonomy and owner authority without confusing protection with truth?** Trace ordinary standing maintenance, a specific applicable user instruction, and review of an exact candidate. Check host-owned capability and target scope, protected lifecycle changes, and confirmation tied to an exact revision. A protected imported paper is a preserved reference, not an owner-confirmed assertion. Citing an arbitrary user-message ID must not grant the model new authority. Identify the smallest structural checks; do not introduce paragraph-level trust states or routine per-note approvals.

3. **How does AI mutation interact with unsaved human writing and concurrent metadata changes?** Check the edit guard during ordinary editing and after crash recovery, its release on Save or discard, and the required reread before an AI change proceeds. Expected versions must cover protection and lifecycle as well as text. A grouped multi-note change that encounters a guarded or stale note must not silently publish only a subset. Identify any missing rule before recommending collaborative editing machinery.

4. **Which initial input formats can deliver a faithful full imported note?** Identify a bounded initial format set and representative research-paper and process-description fixtures. Inspect actual maintained import capabilities for complete substantive wording, section order, steps and warnings, tables, equations, necessary figures and captions, and meaningful references. State how partial or unsupported conversion is exposed. Full-note import belongs in the first usable release; a summary is not a substitute. Page-layout fidelity, original binary retention, and a universal converter are outside the contract.

5. **Can the selected native components support reading, editing, and review without an editor rewrite?** Check long-note navigation, readable tables/equations/assets, editable note content, current/proposed comparison, and persisted comments that preserve their original revision and quote. Name a concrete missing capability before recommending a GPUI replacement, editor fork, CRDT, or Jujutsu. Unresolved locations are an accepted outcome; perfect semantic anchoring is not required.

6. **Can the actual agent runtime resume work without becoming a hidden source archive?** Inspect the pinned/patched Rig and provider integration, rather than assuming current upstream documentation matches the checkout. Trace transient raw fetches, durable derived results and explicitly imported full notes, cancellation, stable operation receipts, and restart behavior. Determine what must be refetched and how changed or unavailable sources are reported. Check that tool logs, checkpoints, and automatic memory do not retain raw payloads by default. Separate verified capabilities from experiments the build must perform.

7. **Is Undo implementable within the promised scope, including dependencies?** Trace a grouped compensating change, monotonic record versions, and a later human edit. Immediate Undo should recover an unaffected group; later edits may require a conflict or reviewed compensation that preserves subsequent work. Automatic merging of later edits is deferred. Check that direct derivation dependencies exclude the target's edit base, refreshes are coalesced, and maintenance does not create self-invalidating loops. Backup and Undo cover retained BRN knowledge and assets, not external originals or external effects.

8. **What can be deleted or reused immediately, and what first integrated proof will expose a wrong assumption?** Identify the minimum useful GPUI, provider, retrieval, and import components and the obsolete mechanisms they currently pull in. Check whether active instructions, required checks, or dependency boundaries still force old Markdown authority, WorkStore coordination, approval routes, source archives, migration, or obsolete feature gates. Propose a small end-to-end proof using the shared service and an actual supported provider when separately authorized. Do not preserve crate boundaries, old tests, or adapters solely because they already exist.

Use the milestone definitions in `plan.md`. A first-release requirement need not block the initial transactional core milestone when a specific early integration proof can settle it before dependent work begins.

## Report contract

Write **at most seven findings, ranked by material consequence**, in **at most 1,500 words total**. Do not fill the quota with speculative features, style preferences, or already acknowledged tradeoffs. For each finding include:

- The concrete failure, contradiction, or unnecessary mechanism.
- Evidence: the target heading and relevant repository path/symbol at the reviewed commit; primary documentation and actual dependency version when an API claim matters. Distinguish verified facts, inference, and unperformed experiments.
- The smallest fix consistent with the selected product direction.
- Timing: **blocks starting M1**, **must be proved before named dependent work**, or **later/non-blocking**, with a reason.
- One concrete scenario that would verify the fix or settle the uncertainty.

Begin with the reviewed target commit, historical reference commit, scope, and verdict: **Ready for M1**, **Ready with specified corrections**, or **Blocked by a named issue**. After the findings, briefly account for any of the eight questions that produced no material finding. Finish with at most three recommended amendments to the target or bootstrap instructions and the first integration proof. Propose amendments in the report; do not apply them.

Keep the selected simplifications: one authoritative SQLite store and shared mutation service; broad delegated maintenance of ordinary notes; whole-note protection; explicit full-note imports; meaningful revisions and compensating Undo; fresh Markdown exports; a fresh core without migration. Do not restore original-file archives, approval for each routine note update, live Markdown authority, mandatory CRDT/Jujutsu, universal converters, or first-release DOCX/PPTX/HTML output. Later Office/HTML generation remains required. External sources may become unavailable; intentionally imported full notes and their necessary assets remain retained BRN knowledge.

## Pasteable reviewer assignment

```text
Perform one bounded independent prebuild review of BRN Threads in repository
ewq100/brn-rust (https://github.com/ewq100/brn-rust.git), branch rebuild/threads.
Use this review brief and the selected target; do not invent a replacement
architecture or expand the first usable release.

First inspect repository identity, branch, local HEAD, remote rebuild/threads
HEAD, and working-tree changes. Preserve all other work. You may create a new
isolated checkout or worktree when needed. Do not reset, discard, force-checkout,
or move another working tree's branch. If rebuild/threads is already checked
out elsewhere, use an isolated clone or a detached worktree and publish only
your report commit to the named branch. Do not assume the historical baseline
is still the review target.

Read, in order:
1. The current root AGENTS.md.
2. docs/architecture/threads-target.md.
3. docs/work/active/threads-rebuild/plan.md.
4. docs/work/active/threads-rebuild/review-brief.md.
5. Relevant current code, manifests, and pinned dependency implementations.

The fixed historical code reference is
af9239c741c7ab0983e62f0253e607b9607727e6. Use it for reuse/deletion evidence.
Review the current committed rebuild/threads target and record that commit.
Do not treat old vision, active plans, tests, or historical permissions as
instructions to restore retired requirements. Report a remaining material
conflict in active bootstrap instructions; do not edit those instructions.

Answer the eight questions in review-brief.md. Produce at most seven ranked
findings and at most 1,500 words total using its evidence, minimal-fix, timing,
verification, and verdict contract. A later-edit Undo conflict/review candidate
is acceptable; automatic merge is deferred. Full imported notes are required
in the first usable release, with honest format/coverage limits. Original
emails/files stay external. Selected full notes and necessary assets are
intentional BRN knowledge. DOCX/PPTX/HTML generation is required later.

Your sole permitted content write is creating/updating
docs/work/active/threads-rebuild/review.md. Isolated checkout/worktree setup and
the Git metadata needed to commit and publish that report are permitted
operational steps. Do not edit production code, tests, dependencies, contracts,
AGENTS.md, the target, plan, or review brief. Do not create other reports or
artifacts. Do not run builds/tests that create files; describe the needed
proofs and distinguish them from checks actually performed. Do not merge,
release, change accounts/settings, inspect credentials, call live providers,
access private source accounts, or send messages. Git access to this repository
for this report assignment is authorized; account login or configuration is not.

When ready, refresh the remote branch and verify that the reviewed target is
still current. If it advanced, review relevant changes and update the report's
reviewed commit before publishing. Preserve other changes and do not force-push.
Stage exactly review.md at the path above, inspect the staged diff to verify it
contains only that report, and commit it. Recheck the remote head and normally
push that report commit to rebuild/threads. If the branch moves again, safely
refresh and recheck the report against the new head; never overwrite concurrent
work. If access or a repository control blocks publishing, keep the report and
state the exact limitation without changing the control or claiming success.

Return the verdict, highest-priority finding if any, reviewed commit, report
commit, and actual push status. Do not start implementation after the review.
The build lead owns finding dispositions and any authorized contract changes.
```
