# Independent V1 review corrections

## Current checkpoint

Reviewed snapshot: `codex/v1-inbox-recoverable-removal` at
`7c4f668de467721f728f242c8fac8d14606a6e44`, retained unchanged on GitHub.
Initial integrated baseline: PR60, `48941d3ae2c16dd014b6cb0f69a01b8c4ef60fa0`.
The first correction merged normally in PR61 at
`d41de3cba09d69eb1a451335dcfee2d54a83c76f`, exact reviewed tree
`f22821fc98cbc9ce41f5ae2a9247ddd1fdf524da`.
Rewrite protection merged normally in PR62 at
`7a472875f3b7303460e682a78e1f5d110c55a9c1`, exact reviewed tree
`6d93e3ad30b5715d0004099bb776f2b971db86e3`.
Knowledge identities merged normally in PR63 at
`d36375b23dbf27a575ee73b5843bbf7d40f2a8ab`, exact reviewed tree
`80564adbf04869444c4a6c6af1949cead3fe36b5`.
Action correction merged normally in PR64 at
`90c36e5c2035d1eea4fc37e7828f085a0f1bfba0`, exact reviewed tree
`913c3fa3bbc6d3ddfdbc1fafa87c8cae257d7e02`. Source preservation merged in PR65
at531517248300c15ce28bcb53f337984103817010/tree d63c1827aae5da654410520bc9c48856e1fa5e5a.
Knowledge capture recovery merged in PR66 at
`504f6be33e1add8c5819e28eed08628bc8dec8d0`, reviewed tree
`77ec79e33c75a6a0bbed3100c3815648dcb6fd68`. Current branch:
`codex/v1-original-operation-records`, based on that clean merge. Remaining accepted
corrections follow this plan.
The complete V1 goal is confirmed **active** by `get_goal` on 2026-10-06;
its full objective and roadmap dependency order remain unchanged.

The lifecycle snapshot is implemented and automated verified, but unmerged.
Its removal/restore merge, native removal controls and Stage8 originals wait for
record-shape/performance and cleanup corrections. Native/live/owner acceptance
remains pending. Historical live authorization is exhausted; all checks here
use disposable synthetic data, pinned Rust and locked dependencies.

## Accepted outcome and order

1. Publish the existing vault-byte contract and correct authority/steering drift.
   Resolve model-selected quote text within exact saved bodies in Rust, with a
   defined occurrence selector and safe typed refusals. Preserve durable citation
   ranges, original replay, evidence checks, approval and enabled capabilities.
2. Protect managed metadata during AI Rewrite while retaining deliberate owner
   edits. Independently review and integrate this bounded correction separately.
3. Mint remaining AI candidate identities and load replacement baselines in Rust;
   retain original replay and newer review. Keep task-specific instructions at the
   existing small typed `brn-ai` / `brn-workflow` boundary.
4. Correct original-copy admission and record storage/performance before lifecycle
   integration. Owner ratified approved Source exact preservation plus explicit
   confirmation; failed analyses, pending drafts and later derived-note edits do
   not block cleanup. Inbox disposition remains separate.
5. Address verified startup/relationship cost, persisted-byte compatibility and
   truthful retrieval facts in bounded slices before release. Resume Stage7 then
   later roadmap dependencies; this review does not authorize another framework.

## Finding dispositions

| Finding | Validated disposition |
| --- | --- |
| A1 original-operation shape | Accept typed owned records and bounded cost correction before merge. V15 alone does not remove repeated parsing. Existing equal mirrors are verified rather than overwritten; do not replace full integrity checks with unproved summaries or mirror-only authority. Measure synthetic scaling. |
| A2 recovery proliferation | Accept durable reuse rule: approved vault/assets use proposal apply; private intake uses the original-operation family. Another family requires a concrete blocker and owner decision. Additive WorkStore tables are local implementation details. |
| B1 vault format | Accept one contract for actual bytes and the existing read-only archive alias. Unknown managed metadata remains fail-closed in scoped tools, with explicit evidence reading. Defer hex-hash migration: current Source exact-header regeneration and persisted hashes require compatible readers/fixtures first. Never silently rewrite vaults. |
| B2 startup/storage | Accept measured cost and byte-compatibility work. Preserve checked startup, retained work and failed evidence. A broad degraded mode or altered backup-retention contract needs a separate bounded design. Claims that every domain is duplicated are overstated. |
| B3 relationships | Accept measured latency and local optimization. A cache must account for unreadable/oversized notes and incomplete identity inventory, even when ordinary change counts are zero. Existing note-metadata no-op transactions do not both commit writes. |
| B4 CLI | Accept owner-authority documentation. Current CLI is owner-operated; agent approval requires explicit delegation. No new auth service or MCP implementation. |
| Responsibility R1 | Accept exact quote text and optional body occurrence; Rust computes byte ranges. Existing Knowledge quotes preserve real source bytes, but do not prove intended passage/body selection. Conflict already verifies text/ranges; remove model arithmetic while retaining immutable replay. |
| R2 cleanup | Owner ratified the simplified preservation/confirmation rule on 2026-10-05. No automatic removal or permanent purge. |
| R3 candidate mechanics | Accept Rust-owned IDs, selected Source injection and full Action baselines. Current model IDs are validated and confer no approval authority. Deduplicate exact intent only within its owned turn, never across semantic tasks. |
| R4 refusal feedback | Accept bounded allowlisted reasons where useful; never expose arbitrary workflow/provider diagnostics. Existing stale-index and storage categories already remain distinct. |
| R5 Rewrite | Accept protection of managed metadata only on AI Rewrite. Manual owner edits remain available; ordinary edits must not inherit this restriction. |
| R6 retrieval facts | Accept current scope/classification and unresolved-conflict facts with exact fresh proofs and explicit unknown/refusal. Annotation alone cannot prove live model disclosure. |
| R7 prompt ownership | Retain the owner-approved typed two-crate boundary. No safety defect establishes a need to move every instruction into workflow. Tool descriptions stay with tool implementations; update behavior text only when its protocol is touched. |

Large-note identity completeness, destination retargeting, provider/CLI schema
abstractions and optional cleanup flags do not justify speculative architecture
work. Preserve current fail-closed authority and record any concrete future
blocker rather than introducing a partial framework.

## First slice acceptance and gates

- A unique exact body quotation resolves to the original saved byte range,
  including BOM, CRLF and multibyte text. Metadata is excluded.
- Repeated and overlapping text requires an explicit 1-based occurrence;
  missing/ambiguous/invalid selections return fixed typed safe errors.
- Conflict IDs are minted from owned analysis plus exact candidate intent so
  original replay can retain proofs/closure after source loss without rereading.
- Changed intent/occurrence is not mistaken for original replay. Fresh captures
  still check identity, scope, saved versions, bounds and the mixed consequence cap.
- Focused real callback tests and protocol/capability tests pass, followed by
  relevant shared/native checks, independent read-only review and exact-head CI.
- Documentation accurately separates implemented, verified, accepted and merged;
  no provider calls, model downloads, data migration or release occur.

Verification/results and the next unfinished gate are recorded here as the slice
progresses; the [resumable checkpoint](../../../development/checkpoint.md) remains
the short navigation entry point.

## First slice implementation evidence

Root quote resolution, real owned callbacks and original replay are implemented.
The first compile exposed a missing exhaustive terminal error mapping; a newly
added replay assertion also needed full JSON comparison because WorkTurn has no
PartialEq. Both were corrected. The first Workflow library run passed275 tests,
with7 documented ignores and one expected old Knowledge instruction fingerprint
failure. Only the related Knowledge text changed; the deliberately updated guard
and explicit safety assertions subsequently passed. These intermediate failures
are retained rather than reported as successful runs.

The AI helper's final protocol and safe-feedback checks passed123 tests,
zero failures/ignores, all-target Clippy and format. Synthetic real Rig routes
retain quote text/occurrences and expose only three fixed safe repair messages;
other tool errors keep existing behavior. No live provider inference occurred.
Independent read-only review validated the vault example's exact439 bytes and
429..438 citation, retained conflict replay and preserved authority. Its CLI
contract-introduction inconsistency was reproduced and corrected. Final review
of the complete correction, including bounded Rig feedback, is clean.

Fresh integrated verification on Darwin arm64, Rust1.98.1, locked/offline,
passed format, workspace build, all-target Clippy, 1,351 workspace tests
(0 failures,8 documented ignores) and52 end-to-end fixtures. The atomic gate
ran 2026-10-05 20:42:06–20:46:52 UTC, terminal exit0, with unchanged
dirty-snapshot SHA256
`d94aa660f81d3514a5773520ca32c1fc46540bec27c56a8b1d88cb8580adb08d`
on parent `cad1a1a60c11608e69be2db3fccf6854f31975fe`.

Optional native checks on that same code/snapshot ran 20:54:06–20:55:41 UTC:
combined Desktop check; all-target Clippy for native UI, combined features and
combined/headless-widget features;284 Workflow/model tests (0 failures,7 ignores);
285 Desktop tests (0 failures/ignores); shipping combined Desktop/native CLI
builds; two real AppWorker startups against one fresh V14 synthetic directory.
All10 commands exited0 with unchanged identity. The upstream `block v0.1.6`
future-compiler warning remains. Empty model configuration means no real assets,
inference, live provider, graphical or owner acceptance was exercised.

The old offset protocol was independently reproduced accepting outer
`brn_kind: source` metadata as a Knowledge quotation: the synthetic regression
failed its refusal assertion (exit101). The new owned-callback body-selection
regression passes in the complete gate. The temporary baseline probe was removed
without changing the separate Rewrite branch. Final documentation-only evidence
updates do not alter the tested Rust tree. Exact-head CI and integration are the
next unfinished gate at that checkpoint. Subsequently PR61 run37373436230
attempt1 passed all four protected Mac/shared checks and Documentation/tooling at
`bb08549393a02606f9e80e80b8fdc88911658ab2`. Its overall red result is retained:
Windows22 complete compiler blocks and both summaries match qualified PR60.
Normal merge preserved the complete reviewed tree.
Fresh merged verification passed123 AI tests,34 owned Inbox callbacks,7 quote
checks and52 fixtures. The first merged attempt exposed stale local package
artifacts after a baseline probe reused a target: it passed old filtered binaries
before a production build failed against the old AI library (exit101). Clearing
only that target's BRN package artifacts and rebuilding reproduced the correct
merged tests and passed. The failed/zero-test attempt is not qualification;
the [verification guide](../../../development/verification.md) now records this
specific target-isolation constraint. Main37375126865 remains in progress.

## Rewrite correction evidence

AI Rewrite now preserves exact managed `brn_kind`, `brn_state`, `brn_provenance`
and `brn_inbox_source` field presence/line bytes, alongside established identity
guards. Values remain opaque; ambiguous metadata refuses. Store direct Rewrite,
transactional owned settlement and the worker's preflight all use the same guard.
Explicit owner edits still repair or change metadata. Refused AI output becomes
durable `Failed/tool_rejected`, preserving review, comments, vault bytes and replay.

Acceptance: valid body-only Rewrite, all four metadata changes/additions/removals,
BOM/CRLF/opaque values, ambiguous fields, direct/transactional/owned paths, late
results, restart and owner repair. Independent full review and integration delta
review are clean; the later test-only change also passed independent review.

A synthetic old-main60 callback probe completed an AI metadata change where the
new refusal assertion expected Failed (exit101). The new callback/restart matrix
passes. The first broad gate then found one obsolete integration-test expectation
that AI Rewrite could repair provenance; that failed exit101 is retained. The test
now asserts AI refusal/unchanged review, explicit owner repair and successful
body-only Rewrite, preserving forged/unbound provenance approval refusal. All10
provenance integration tests passed.

Final shared gate, 2026-10-05 21:12:55–21:18:13 UTC: format/build/all-target Clippy,
1,357 workspace tests/0 failures/8 ignores and52 fixtures passed, terminal exit0.
Native gate 21:12:22–21:14:34 UTC: combined check, three Clippy configurations,
285 Workflow/model tests/0/7,285 Desktop/0/0, shipping builds and two V14 restarts
passed, all10 command exits0. Both retained unchanged dirty snapshot
`de48cb454aaea838449196e57c15c48297271aab5f5921adabf140e457f807a8`
on parent `3964c22c6e35b0baddce0c593d0eb770cff80da8`. Final code is committed;
subsequent documentation and main61 ancestry updates do not change tested Rust.
The upstream block warning and native/live/asset/owner acceptance remain pending.
Exact-head PR37375653308 attempt1 passed all four protected Mac/shared gates and
Docs at `1e3f05dfebaec03b099d0fdd385f439b8731068c`; Windows22 complete diagnostics
and both summaries match PR61. Normal merge preserved the reviewed tree. Fresh
merged18 Store Rewrite,1 owned callback and10 provenance tests plus52 fixtures
passed. Main62 run37376785106 is in progress. Main61's applicable gates passed;
known Windows22/22/14 compiler blocks and three Linux assertions/backtraces match
main60, with only Linux case order different. Raw logs/differences remain retained.

The Knowledge identity helper separately committed
`f9b2026a247b726220b53d4d6c91c9957b0cb712` with clean independent review,
123 AI tests,278 Workflow tests/7 ignores and all-target Clippy/format passed.
Its final qualification follows.

## Knowledge identity correction evidence

The AI input now carries only semantic candidate content, destination, selected
quotations, additional evidence and optional predecessor. Rust derives distinct
proposal/note UUIDs from exact ordered typed input and the owned analysis, before
adding managed metadata or observing mutable evidence. Injected identity refuses.
Same original input reuses retained proofs and newer review; changed intent or
another analysis is a separate draft. Ordinary owner draft IDs and persisted
proposal/approval/recovery formats are unchanged.

Clean independent review covered the complete12-file correction at `f9b2026`.
Tests exercise real callbacks and persisted reviews for title/text/path/quote/
occurrence/predecessor/target-order changes, separate analyses, identity injection,
original replay after source loss/newer review and restart. Real synthetic Rig
routes refuse legacy id/note_id fields before callback dispatch. Other behavior
instructions/capabilities are unchanged; touched Knowledge/context byte guards
were deliberately updated with explicit authority assertions.

Intermediate failures retained: missing Cargo PATH, old Knowledge/context
fingerprints, and denied macOS coordination in the sandboxed library run
(153 passed/124 failed/7 ignored). Qualified execution outside that sandbox
passed278 Workflow tests/7 ignores,123 AI tests and all-target Clippy/format.
No unrelated coordination behavior was changed.

Final shared gate at clean `f945a9bbab2b4268b6a2b711d23ec5f482c311ad`, tree
`a09351f5b2585742f9ebad48067d621f1bf38e7d`, 2026-10-05 21:25:16–21:30:01 UTC:
format/build/all-target Clippy,1,359 workspace tests/0 failures/8 ignores and52
fixtures passed. Native gate21:30:58–21:34:15 UTC: combined check, three Clippy
configurations,287 Workflow/model tests/0/7,285 Desktop/0/0, shipping builds and
two V14 restarts passed. Atomic records retained unchanged clean identity and
terminal exit0 for every command. Later docs/ancestry edits leave tested Rust
unchanged. Upstream block warning and native/live/assets/owner acceptance remain
pending. Exact-head CI and normal integration are next; Action candidate mechanics
follow separately before cleanup/record-shape corrections.

The existing `capability-spike` offline gate additionally passed123 AI library
and3 qualification-harness fixture tests plus all-target Clippy at the same Rust
tree. No discovery request, credentials, live provider or model asset was used.

## Action candidate correction acceptance

Baseline: Knowledge PR63 head `385627a720177e731abbb4ee89ee66c7eb7d518c`,
branch `codex/v1-review-action-candidates`. This changes only the AI candidate
transport and owned admission; persisted Actions/proposals, owner APIs, full-record
approval comparisons and recovery remain unchanged.

- Rust mints proposal/Create identities from exact ordered semantic input in the
  owned turn. Original replay retains proofs/full replacement baselines and newer
  review; another turn or changed input is a distinct candidate.
- Replace carries the typed full-record reference returned by `read_action`, never
  a model-reconstructed before-record. Fresh admission verifies every baseline
  field through the reference, including same-version divergence.
- All14 after-fields stay explicit. Existing UUID and 1-based member references
  support ordered mixed Create/Replace drafts; invalid indices, duplicate targets,
  self/cyclic relationships and completed changes refuse.
- Inbox attaches its selected Source proof and UUID automatically. Additional
  evidence remains ordered/bounded, duplicated paths/UUIDs refuse; the shared20
  consequence cap and replay-at-cap remain intact.
- Real owned callback, strict synthetic Rig schema/capability, Stop/restart, exact
  approval and meaningful stale/replay tests precede independent review, shared
  and optional native gates, exact-head CI and normal integration. No live calls.

## Knowledge integration and Action qualification

PR63 exact-head37377504928 attempt1 passed all four protected Mac/shared jobs
and Docs at385627a720177e731abbb4ee89ee66c7eb7d518c. Overall red retains
Windows22 full compiler blocks and both summaries matching PR62. Normal merge
preserved the complete reviewed tree. Fresh merged123 AI/two owned Knowledge
callback tests and52 fixtures passed. An initial misspelled filter selected zero
tests; it is retained but not callback qualification. The corrected filters ran
both intended tests. Merged-main37378949287 attempt1 passed all applicable gates;
Windows22/22/14 full blocks/summaries match main62. Linux retains three identical
assertions/source locations/backtraces apart from thread IDs; order and terminal
duration changed0.04s→1.56s. Complete raw differences remain retained.

Action protocol/admission uses Rust-owned IDs, typed checked full-record
references and local member indices. Inbox selected Source proof/UUID injection
is automatic; extra paths remain ordered/bounded. Original same-turn replay
preserves retained baselines and newer review after Source loss/Action completion.
All existing owner storage/approval/recovery formats remain unchanged.

Intermediate failures retained: helper PATH, test-local helper compile and
undersized aggregate fixture errors; root first compile found the Store constant
path and required ActionData validator argument; the first library run then
passed280/failed2/ignored7 (old context fingerprint and new-intent Source-loss
IndexStale expectation). Both were corrected. Ten focused Action callback/fence
checks, one context guard and the20-member real approval scenario passed. One
broad formatting gate exited1 before build; its assertion wrapping was corrected.
Final independent full20-file read-only review through1d9df120 is clean.

Shared atomic gate22:00:46–22:05:37 UTC at clean1d9df120/tree8bfc86d6:
format/build/all-target Clippy,1,367 workspace tests/0 failures/8 ignores and52
fixtures passed, terminal exit0. Native atomic gate22:05:50–22:09:05 UTC at the
same unchanged identity passed combined check, three all-target Clippy
configurations,291 Workflow/model tests/0/7,285 Desktop/0/0, shipping builds and
two V14 restarts; all10 commands exited0. The upstream block warning remains;
empty model configuration exercises no real assets, inference, live provider,
graphical or owner acceptance. Final documentation changes leave tested Rust
unchanged. Exact-head CI and normal integration are the next unfinished gate.

The existing capability-spike offline gate passed127 AI library and3 synthetic
qualification-harness tests plus all-target Clippy. It performed no discovery
request, authentication, model asset access or live provider call.

## Source-preservation correction acceptance

Baseline: PR64 merge90c36e5c2035d1eea4fc37e7828f085a0f1bfba0. Correct the
existing read-only preview first, before adapting the immutable lifecycle snapshot.

- One approved Applied non-Undo Source still proves the exact original through
  the existing deterministic conversion, complete saved body, UUID, semantic
  original provenance and Source classification. Strict new-proposal headers stay
  unchanged. Owner header Save/new inode, relocation, History and visible archives
  qualify when exact preservation survives.
- Failed/pending analysis, pending processing, consequence drafts/running Rewrite,
  NotApplied Undo and later derived edits do not block or change the selected
  preservation digest. Unsaved editor work remains retained, independently of saved
  evidence. Actual uncertain Save/Apply/completion authority still fences reads.
- One valid witness suffices despite stale alternatives. Missing/changed body,
  identity/provenance/class, ambiguous or incomplete identity inventory refuse.
  Recheck the selected journal/original/full saved proof before returning;
  later confirmation must bind the same witness rather than switch alternatives.
- Keep complete retained review separate. No removal, disposition, permanent purge,
  new AI call, database/schema or original-operation record change in this slice.
  Use focused synthetic Store/Workflow/CLI tests, independent full review, shared
  and relevant native checks, exact-head protected CI and normal integration.

PR64 run37380762825 attempt1 at85947595aacde6728b3c50b337ac694533c0d47b
passed all four protected Mac/shared gates and Docs. Overall red retains Windows22
complete compiler blocks and both summaries identical to PR63. Strict protection
and normal merge requirements passed. Fresh merged127 AI,11 Action callback/fence
tests and52 fixtures passed; merged tree exactly preserves the reviewed head.
Merged-main37382000386 attempt1 passed all applicable gates. Overall red retains
Windows22/22/14 complete blocks/summaries identical to main63 and the same three
Linux assertions/backtraces. Raw order/thread IDs changed and terminal duration
changed1.56s→0.04s; full logs/differences are retained.

## Source-preservation qualification evidence

Complete12-file independent read-only review is clean at
`95940fe6c143d529f98b70ef5914b1e05decdc20`, tree
`5c984a20fab1868cc9ef52633959b94289d9e58c`. The shared converter preserves old
bytes/cancellation. Strict Source proposal validation and saved/approval formats
remain unchanged; the preview now retains one complete selected witness.

Store helper9 focused/364 full tests passed. An initial Clippy needless-reference
finding was corrected, final focused tests/all-target Clippy/owned format passed.
Root13 preservation tests passed; an initial CLI `--lib` command failed because
BRN has only a binary target (exit101); corrected `--bin brn cli::inbox` ran all6
checks successfully. No product failure was hidden. Tests cover owner metadata
Save/new inode/History/archive/restart, stale alternatives, exact body/id/provenance/
class and incomplete-inventory refusal, failed/pending work and derived independence,
dirty editor retention, genuine interrupted Save/Apply fences and near-limit
worst-case escaped proof roundtrip.

Fresh shared atomic gate22:36:52–22:41:39UTC at unchanged clean95940fe/tree5c984a20:
format/workspace build/all-target Clippy,1,382 workspace tests/0 failures/8
documented ignores and52 fixtures passed, terminalexit0. Optional native atomic
gate22:42:03–22:43:39UTC at the same unchanged identity passed combined check,
three all-target Clippy configurations,297 Workflow/model tests/0/7,285 Desktop/0/0,
shipping combined Desktop/native CLI builds and two V14 synthetic AppWorker
restarts; all10 command exits0. The upstream block future-compiler warning remains.
Empty model configuration means no real assets, inference, live provider, graphical
or owner acceptance was exercised.361 current links/diff checks passed. Later
qualification documentation does not change tested Rust. Exact-head protected CI
and normal integration are the next gate; Source qualification grants no removal.

Owner scenario remains pending: in a disposable text Inbox fixture, approve its
exact Source, retain a failed analysis/pending consequence and change a derived
note; `inbox removal-preview UUID` should return one saved preservation witness,
no blockers and `needs_owner_confirmation: true`, with the original still retained.
Changing that Source body or duplicating its UUID must refuse qualification.
Explicit confirmation/lifecycle and native controls follow after bounded original
record/performance/recovery correction. No live authorization is renewed.


## Captured-analysis recovery acceptance

Source-preservation PR65 merged normally after clean review and exact-head
PR37384547448 attempt1 protected checks/Docs passed; Windows22 compiler blocks
and both summaries match PR64. Fresh merged9 Store/13 Workflow/6 CLI tests+52
fixtures passed at unchanged5315172/tree d63c1827, terminalexit0. Main65
run37385458794 attempt1 passed all applicable gates; Windows22/22/14 complete
blocks/summaries match main64. Linux retrieval retains the same three assertions/
backtraces/order/terminal summary; only thread IDs changed, retained in full logs.

Inspection confirms a prerequisite to shrinking legacy original-operation records:
ordinary Knowledge approval receipts retain the journal, but not its genuine
`InboxActionJob`. A fresh database import can commit an uncheckable Knowledge
proposal before a later contextual read refuses it. Old lifecycle certificates
restore these captures incidentally; removing that graph without a replacement
would lose recovery evidence. Existing capture limits remain50,000 Source bytes,
512KiB question and1MiB encoded job.

- Retain one genuine immutable captured job per Knowledge-bound approval in the
  existing `.brn-apply-<operation>` family. Keep format1 receipt bytes/hashes and
  equal occupied files unchanged; no new recovery family, database/schema, chat
  framework or provider call. Pure payload checks validate shape/binding; equality
  to retained evidence protects original question/time/selection, never invents them.
- Exclusively publish the bounded hash-checked capture companion before effects;
  equal occupied companions are checked/synced, different/corrupt occupants refuse.
  Valid prepublished orphan captures retain no apply/provider authority. Unknown
  family names and corrupt canonical companions remain fail-closed.
- Import capture/proposal/journal in one Store transaction, checking exact Source,
  purpose, analysis UUID and every citation before commit. Missing capture refuses
  without partial work. Existing checked SQL or validated legacy evidence can seed
  an older receipt's companion; unavailable evidence is never reconstructed.
- Preserve newer review, comments cleanup, supersession and repair. No fabricated
  session/turn or repeated provider work under a recovered historical analysis UUID.
- Prove fresh/older-database recovery, legacy receipt byte/inode compatibility,
  mismatched/missing/corrupt evidence rollback, exclusive/equal replay, interruptions
  and historical execution refusal with synthetic tests. Independent full review,
  relevant shared/native checks, exact-head protected CI and normal merge follow.

Lean original-operation records/lifecycle integration follow this prerequisite;
native/live/owner acceptance and trusted-user packaging remain separately pending.


Captured-analysis implementation retains the exact job plus approval request and
immutable Knowledge-binding digest in a canonical hash-checked companion bounded
at1MiB+4096 bytes. Companion publication/import/repair stays in the existing
approval family. Format1 receipts remain unchanged. A valid prepublished orphan
is checked and inert; it cannot create a proposal, chat or filesystem authority.

The lead's baseline regression failed exit101 on fresh Applied Knowledge recovery:
`Inbox knowledge analysis capture is unavailable`. The Store helper independently
reproduced the old fresh import committing uncheckable work before fixing the
transaction. Helper8 new recovery tests and112 related Store tests passed,
all-target Clippy/owned format/link checks passed; supplied capture and restored
journal are fully checked before one commit. Genuine question/time/selection are
retained exactly; fresh structural validation never claims to derive those facts.

Initial integrated Workflow run passed12, failed1, ignored1. The failing test
incorrectly required a fence after an occupied exact companion was rechecked/
synced and the no-effect refusal was durably completed. Its expectation was
corrected to require terminal NotApplied with no vault effects; earlier unresolved
publication failures still require fences. Corrected13/0/1 and Store/Workflow
all-target Clippy passed. A further near-limit escaped genuine capture test passed,
including fresh recovery and no manufactured chat. Full review/shared/native gates
and exact-head integration remain pending; intermediate failures are retained.


Complete16-file independent read-only review is clean at
10e95e2c8880b6f1295fe630249a6ecd5382d870/tree6a6a33d99039458d644a9f01763d0e08183133a3.
Fresh shared atomic gate23:09:28–23:14:18UTC at the same unchanged clean identity
passed format/workspace build/all-target Clippy,1,397 workspace tests/0 failures/8
documented ignores and52 fixtures, terminalexit0. Optional native qualification
also passed combined check, three all-target Clippy configurations,304 Workflow/
model tests/0/7 and285 Desktop/0/0, shipping builds and two V14 synthetic AppWorker
restarts; every10 command exits0 at unchanged clean identity. Atomic start/end
records retain the exact commands/features/environment. Upstream block future-
compiler warning remains; empty model configuration exercises no real assets,
inference, live provider, graphical or owner acceptance. Exact-head CI and normal
integration remain the next gate; subsequent evidence edits leave Rust unchanged.

New companions require the qualified reader: older builds fail closed on their
unknown family suffix. Existing format1 receipt bytes/hashes and equal inode are
unchanged; no artifact is removed to permit downgrade. Owner scenario remains
pending: recover disposable approved Knowledge/supersession from retained ordinary
receipts into fresh operational state after moving/changing its Source, observe
original history/review without resubmission, and verify damaged/missing capture
refusal. No private data, provider call or original removal is authorized by this.


## Captured-analysis integration

PR66 run37387682980 attempt1 at1eeef2b9f6d1ff100aa8e8955d3cba07ca2c20cf
passed all four strict protected Mac/shared checks and documentation/tooling.
Overall red retains Windows22 complete compiler blocks and both summaries
identical to PR65. Mac workspace tests passed in495seconds versus PR65's388;
no rerun or substituted evidence. Fresh protection/head checks and normal merge
produced504f6be33e1add8c5819e28eed08628bc8dec8d0, exactly the reviewed tree.
Fresh merged8 Store/14 Workflow tests/0 failures/1 documented subprocess ignore
and52 fixtures passed, atomic exit0 with unchanged clean identity at
23:32:41–23:32:51UTC. Exact main37389066259 attempt1 passed all applicable gates.
Overall red retains Windows22/22/14 complete compiler blocks/both terminal
summaries identical to main65; Linux native retrieval keeps the same three
assertions/source locations/backtraces. Raw order/thread IDs and duration0.04s→0.81s
differ and are retained. Mac workspace tests478seconds/job595seconds passed;
no rerun or mutation. Native/live/owner
acceptance remains pending; no original was removed.

## Original-operation Store prerequisite acceptance

Baseline504f6be33e1add8c5819e28eed08628bc8dec8d0. Implement Store records first;
filesystem effects, shared worker/client commands and native controls follow.
Reuse useful code/tests from immutable7c4f668. The owner-ratified preservation
rule is fixed; schema/table layout can evolve within existing WorkStore.

- New bounded Remove records bind the exact available original, one complete
  approved Source journal and fresh saved Source proof, the exact preview digest,
  one versioned explicit owner confirmation, namespace and monotonic times.
  Pure preservation checks grant no filesystem freshness or execution authority.
- Use one direct prior Restore UUID/digest; Restore binds one Remove UUID/digest.
  No accumulated history, processing, chat, finding or consequence graph in new
  records. Preserve legacy format1/five-attestation setting bytes, hashes and
  full semantic validation; new behavior does not inherit those historical gates.
- Add an owned V15 table with canonical complete bodies and cross-checked indexed
  summaries. V14 upgrades and validated backup recovery preserve exact user work.
  Readable semantic/schema damage refuses without replacement or new backup.
- A compact checked inventory parses/checks/drops each full body once per pass;
  check catalog/origin/namespace, parent digests/opposite settled kind, times,
  forks/cycles and legacy cumulative history. No unproved summary or mirror-only
  authority. Atomic import rolls back missing/forked/malformed evidence.
- Test meaningful new/mixed legacy chains, rehashed metadata and Source damage,
  historical byte/digest compatibility, migrations/backups and long/multi-item
  scaling. Measure actual full-body parsing/encoded size/time; V15 alone is not
  evidence of bounded cost. Independent full review, shared/native qualification,
  exact-head required CI, normal merge and fresh merged checks remain gates.

The following lifecycle slice must reuse the existing private Remove/Restore
family, preserve equal mirror bytes/inodes, import exact Item/legacy genuine
captures before approval companions, surface bootstrap failures and prevent
intake resurrection. No new recovery family, datastore or semantic gate is added.


## Original-operation Store qualification

Implemented code48faf3f6a50eae152c80509a3316b7a6e6b94045, tree
c0872f36ea59fc5378d8f10b9ed9ba40b7e29575, clean independent read-only review of
all30 changed files against504f6be. Helper05fc6b3 was integrated as1d81b34;
the lead added exact current Workflow preview JSON/digest and escaped-limit
compatibility checks. New records carry no cumulative consequence/chat graph.
V15 owns canonical bodies and checked indexed metadata; compact inventory drops
bodies per pass. Fallible streaming import is atomic, without an aggregate input
list/cap; selected bodies stay bounded128/64MiB. Snapshot-consistent visitation
checks all authority before its first callback, then reads at most2N bodies.
Legacy v1 envelopes/five attestations/digests remain unchanged and fully checked.

A literal19,563-byte legacy fixture pins envelope hash
380a30650a5a8e605283ccc83e6a23c0eb8ca9c1cf40ceae5790753dd90e303b and old
record digest acc1203cb8773872226b7d0ecfc4856999aa97322a5f55f6c84776bda95f4630.
Its layout derives from immutable7c4f668; no separate old-binary execution is
claimed. Tests cover exact V14 migration/backup bytes, malformed pre-migration
settings, mixed histories, missing/forked parents, rehashed schema/metadata/body
corruption, iterator rollback, visit snapshot/error ordering and exact replay.

The legacy-only historical analysis regression failed exit101: recovering the
certificate without a real WorkTurn let its UUID start new execution. The checked
legacy reservation fence fixes this without creating Sessions or turns; actual
failed/completed turn replay still precedes the fence and unissued reservations
still start. Preserve the red log. Helper's initial Clippy found one test-only
cloned-ref-to-slice lint, fixed with from_ref. Lead workspace rustfmt found one
missed long migration-fixture SQL line, formatted before the final gate; neither
failed attempt is qualification. Helper final Store386/0/0 and all-target Clippy
passed; lead Workflow14/0/0 passed, including complete escaped preview equality.

Fresh shared atomic gate00:06:22–00:12:44UTC on2026-10-06 passed format, workspace
build/all-target Clippy,1,412 tests/0 failures/8 documented ignores and52 fixtures,
terminalexit0 with unchanged clean48faf3f identity. Fresh offline native gate
00:12:58–00:14:37UTC at the same identity passed all11 command exits0: combined
check, three Desktop Clippy feature configurations,305 Workflow/model tests/0/7,
285 Desktop/0/0, shipping combined Desktop/native CLI builds, default AppWorker
startup and two combined-native restarts. Actual SQLite queries confirmed V15 and
the owned table in both fresh directories. Exact commands/features and start/end
identity remain in atomic records. The existing storage gate's shared checks are
covered once here; its default startup is included rather than repeating the
workspace suite. Upstream block future-compiler warning remains; empty model
configuration uses no real assets/inference/live provider/GUI or owner acceptance.

Debug synthetic observations:256-row streaming import351ms and inventory172ms;
seven-operation family with75,368,088 bytes of four near-limit escaped Remove
bodies imported in30.91s, visitor18.54s/14 reads, inventory9.28s/7 reads. Selected
four bodies correctly refuse the64MiB output bound. These are elapsed debug
observations, not release benchmarks; broader B2 startup/backup cost remains
pending. New table alone is not a performance claim.

Exact-head CI and normal integration are the next gate; later evidence-only
Markdown updates do not change tested Rust. No original filesystem effect is
introduced. Following lifecycle adaptation reuses immutable7c4f668's endpoint/
exclusive-rename protocol, distinct version2 envelopes in the same private family,
streamed mirror import/repair and current Source+confirmation admission. Preserve
equal legacy bytes/inodes, surface bootstrap errors before approval companions,
never start a move at startup or install capture stages for any operation head.
Native/live/owner acceptance, later roadmap work and trusted-user packaging remain
pending; full V1 goal remains active.
