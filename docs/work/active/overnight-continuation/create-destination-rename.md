# Rename a Draft new-note destination within its existing folder

Selected next bounded P3 outcome after the current compensation gates release.
The owner can correct a proposed new note's filename without copying its text into
another proposal or approving intermediate work. Retain exact text/managed UUID,
comments/anchors, group/session, sources, intake/citations, Actions and every
unchanged member proof; advance the approval version once. No vault effect occurs
until approval of the revised version. This slice permits only a same-folder
rename of one NoteChange::Create in an exact Draft. Cross-folder moves, Replace
retarget, splitting, regrouping, assets, bound Inbox Source and visual retarget
remain separate. Same-folder scope preserves ordinary relative link/image meaning
without a new link-repair policy or silent text rewriting.

## Reuse and fixed boundary

Reuse the predecessor structural revision's Store CAS/whole-record transition
validator and native submitted-before/generation handling; existing coordinated
file adapter, Current path policy, target/editor guards, snapshot/intake validators,
exact approval/application/Undo and recovery. No dependency, second proposal store,
converter, AI tool or provider work is needed. Read-only inspection established
that InboxKnowledgeBinding binds note UUID/evidence/citations/supersession rather
than its consequence filename; InboxIntakeBinding.source_path remains the exact
prerequisite Source path and must never change.

Public workflow strict request: CreateRenameRequest { expected: ProposalStamp,
change_index: usize, path: String }. AppWorker returns the existing Proposal event;
CLI maps proposals rename-create --file REQUEST.json. Native review uses the same
request and pure complete-record transition validator. A same-path current request
is a no-op preserving every proof, version and timestamp; a stale stamp still
refuses. Host captures the new destination's absence and current parent through
existing coordination, requiring the parent proof equal the retained proof. Do
not refresh authority if the directory changed. Store changes only the selected
Create path, preserving its parent; changed/occupied/aliased/unsafe targets and
retained editor conflicts refuse. Preparation validates retained evidence/intake
with applied=false; approval still requires the exact Applied prerequisite.

## Compact creation-replay evidence

Current creation replay reconstructs initial input from stored current paths and
parents. Merely changing the path would break replay of the original request.
Keep bounded host-owned original Create paths on first rename, retaining the
earliest path through repeated renames. Same-folder scope preserves the original
parent, so no duplicate text, original Draft or fresh historical proof is needed.
Suggested strict OriginalCreatePath { change_index, path } vector, sorted/unique,
indices to current Create members, same original/current parent and existing path
bounds. Add omitted-when-empty/defaulted evidence to private StoredProposal and
ApplyJournal, keeping historical serialized bytes/hashes canonical. Public
ProposalRecord stays unchanged. Existing approval/recovery constructors preserve
this evidence; new inverse proposals have their own initial creation hash and
empty rename lineage. Do not discard newer valid lineage during recovery of older
settled operations. Validate bounds, corruption and compatibility explicitly.

Store exposes checked original-path reads for workflow creation reconstruction.
Restore earliest paths before the existing immutable creation-hash check, and
return the current reviewed record before fresh filesystem checks. Changed
initial requests still conflict. ApplyJournal ordinary mirror/import and old-DB
restore must preserve replay without source-journal, converter or model replay.
The knowledge callback receipt must report the returned proposal's actual current
Create path, rather than its original args.path. Retained Source/intake identities
and proof order remain exact.

## Acceptance

1. Ordinary and retained Inbox Knowledge Creates rename; a Knowledge/History pair
   retains its complete History member. Title/text/UUID/comments/anchors/evidence/
   grouping/session/Actions and other before proofs remain byte-exact. No file is
   created during preparation; version advances once, no-op stays exact.
2. Old individual/group approvals and late Rewrite results refuse; pending Source
   permits preparation, while approval still requires its exact Applied Source.
3. Occupied/unsafe/aliased/new-parent/stale-source/dirty-editor/index/type/cross-folder
   requests refuse without changing stored review or any vault file. Source/asset/
   visual and Replace operations remain unsupported.
4. Original creation replay returns the current renamed review after text edits,
   repeated rename, restart and Source loss; changed initial payload conflicts.
   Knowledge callback returns the current filename and no duplicate proposal.
5. Approval writes only the new destination with the exact UUID/text; the original
   target remains untouched. Existing recoverable application/Undo remains exact.
6. Complete ordinary mirror export/import and old-DB restoration retain original
   creation replay evidence. Historical empty-field serialization, strict malformed
   lineage refusal, encoded limits and full-size recovery witnesses remain enabled.
7. Native exact before/after capture rejects wrong ID/generation/malformed/later
   acknowledgement and preserves owner typing/navigation on failure. CLI exercises
   prepare/old-stamp refusal/approval/restart/original replay. All actual GUI work
   stays pending in the one morning task; no fresh inference for these mechanics.

Lead retains selected model/effort and owns workflow/CLI/shared docs/integration.
Up to two bounded helpers without recursion: Store lineage/CAS/recovery tests and
native capture/state/tests, with explicit ownership/interfaces before edits. One
Cargo across checkouts. Synthetic/public only; preserve unrelated work/data, no
computer/browser/GUI/private data/credentials/download/reset/port/release. One
complete independent read-only review, applicable final gates, actual required CI,
normal protected merge and resulting-main verification. Stop/reassess only a
concrete integrity blocker or consequential unresolved product choice. Record
branch/baseline when implementation starts; no retarget code has been written yet.


Implementation uses the clean reused backup checkout
`/Users/evokessler/repos/brn-p3-work-budgets`, branch
`codex/p3-create-destination-rename`, baseline
8bf40d42202b4e93e8d41bb584b911baf9577ac9 (Action/backups composed candidate).
Action PR100 qualification and CI remain in the other checkout, so source edits
cannot alter its running gate. At most two helpers, no recursion. Store owns
private original-path lineage/CAS/journal/Store tests; desktop owns pure capture,
AiState/native controls/tests. Lead owns workflow DTO/host preflight/creation replay,
AppWorker/CLI/tests/shared docs and integration. Cargo stays unavailable to helpers
until the lead grants it after Action composition gate55052 releases.

Fixed Store APIs: `rename_proposal_create(expected, change_index, path)` and
`proposal_original_create_paths(id)`, plus pure
`validate_create_rename_transition(before, after, change_index, path)`.
Strict public `OriginalCreatePath` is used by checked private Store/journal evidence;
public `ProposalRecord` stays unchanged. Current same-path no-op returns exact work
before fresh evidence checks; stale stamp always refuses. Actual changed rename
requires all retained evidence/target/editor validation. Recovery compares original
normalized paths and directional lineage compatibility; equal-version lineage
forks refuse and older settled operations never replace newer lineage.


## Implemented candidate and focused qualification

Action PR100 passed every required candidate check/docs in37858092859 and merged
normally at e1d6b0fab69c13ceceb09014d6c034ef2178882f. Its tree exactly matches
qualified8bf40d4. Resulting main was incorporated without production changes at
7eccea2388b724adae9254f6be6caa46dcfc91d9. Backup PR99 required post-main checks/docs
passed in37857753049; Action post-main37859299854 remains active.

Store focused200 tests passed, plus strengthened new six-test binary and actual
late-Rewrite rename race; strict all-target Store Clippy passed. Public full-record
transition and host request, parent/absence/alias/editor/evidence preflight,
original creation reconstruction, worker/CLI and current callback receipt are
implemented. Workflow2 actual callback/private-intake tests,3 integration cases
(including nine refusal modes), and2 actual CLI tests passed. These use synthetic
hooks, not provider inference.

Initial pending-private-Source witness failed with absent saved citation identity:
preparation incorrectly reused ordinary approval's saved-source provenance check.
Private Knowledge now uses its exact protected citation/extraction binding during
rename preparation; ordinary saved evidence checks remain and fresh approval still
requires the exact Applied Source. The witness passed after this narrow fix.
An initial group-refusal test expected an in-loop stop; the existing group version
preflight correctly returns ContextStale before admission, and the assertion now
checks that refusal. No production behavior changed for that test correction.

Desktop capture/native controls/tests are implemented, including exact raw Action
field preservation and a rename-specific failure/navigation fence cleared by
existing explicit Retry/Discard. Its focused default/native/Clippy checks are
active under exclusive Cargo. One fresh complete independent read-only review is
next, followed by final cross-crate/native/shipping gates and required CI.
GUI acceptance remains pending in the single morning task. No rename PR yet.


Complete independent review of38files at patch
9927de423dde5b81b84940062a012921c75b512314de2f6113c5bd92dce66180
found one valid native navigation defect: an ordinary Observe after rename refusal
could adopt the unchanged review and clear its failure fence. The correction
requires !create_rename_failed before observation adoption; changed observations
remain explicit conflicts. A new state regression was observed red before the
fix, then green. Four native observation/navigation cases cover refusal/malformed
acknowledgement with same/changed observations and explicit Retry/Discard.
Independent correction review clean at
f3e8a4f78049a99fc8798cdca09313d3eb55949d12096fe1cbb74270e0b8ee18.
No remaining actionable findings. Lead rechecked both exact hashes.

The unchanged pre-correction tree passed full1795 default/18 existing ignores,
436 native workflow/models/16,577 combined desktop/CLI, doctests, strict Clippy,
shipping,52fixtures and links. Store/workflow/full-size recovery evidence remains
applicable: the correction changes only one desktop observation predicate/tests.
Corrected default11 tests passed; native/Clippy checks active. Lead repeats complete
affected desktop/combined/shipping gates before integration, reusing unchanged
Store/workflow coverage rather than repeating full-size crash witnesses.
Action PR100 required post-main checks/docs passed in37859299854.
