# Projects candidate evidence

**Scope/authority:** Owner requested implementation and confirmed the planned Projects feature on 14 October 2026. Implements stage two of the [accepted brief](../chats-and-projects/spec.md) using the [plan](plan.md). Release, real-data access and merge are unassigned.

**Baseline:** fetched `origin/main` at `85d291cddfa0f5b5a73e3d4f3b2b96c844445839`. Isolated `codex/projects-feature`; original checkout and unrelated experiments preserved. Candidate identity and final check receipts will be recorded after verification.

| Gate | State |
|---|---|
| Defined / build ready | Satisfied: accepted stage-two decisions; owner confirmation; independent read-only plan review approved. |
| Implementation | Candidate: checked core/app commands, context/retrieval/runtime, native project controls and CLI routes implemented. |
| Automated verification | In progress; final commands/results below. |
| Independent code review | Six supported findings fixed; scoped re-review pending. |
| Native observation | Pending; widget/state tests do not claim actual native interaction. |
| Owner acceptance | Pending; concrete qualification below. |
| Hosted CI / merge | Pending and unassigned for this candidate. |
| Release / real-data access | Unassigned; all local fixtures disposable and synthetic, runtime journeys offline. |

## Owner qualification

1. Create a project; set explicit instructions and turn BRN knowledge off. Create/link/import a document and start two distinct project chats. Confirm earlier discussion remains distinguishable from accepted documents/decisions.
2. Move an existing chat between projects and back to standalone. Confirm its draft/history survive, active work stops and queued text stays paused.
3. Share one private document with a second project. Open it there and create a writing conversation: it uses that project's instructions. Edit the document through existing Save/review/Undo and see the same record through both links. Unlink the original project without promoting or copying it.
4. Delete a project with an exclusive document, a shared document and unfinished tasks. Check the removal/retention list and choose Keep outside or Cancel for each task. Restore from Trash: shared edits persist, cancelled/outside tasks remain cancelled/outside and work stays paused.

## Verification and review receipts

Final candidate commands and results are being collected. Schema-4 adoption, exact review after attention, changed-context stale review, private-history provenance, cancelled-Action isolation, private unlink retention and selected-project native writing have focused regression coverage. One initial broad run was interrupted after discovering unindexed project-metadata scans; schema-5 partial indexes replace those scans. Subsequent gates qualify the corrected code.
