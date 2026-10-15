# Consolidated target-Mac qualification

**Status:** Test outline, not executed. C must supply exact app build/ref, disposable data path, launch command, fixtures, available provider authority and known limitations before use.

Run only on an accessible/unlocked target Mac. Never replace source verification with visual impressions. Do not use real owner documents by default. Complete offline parts even if provider authorization is absent, but leave live-generation cases pending.

## Setup record to complete

| Field | Record |
|---|---|
| Candidate commit and build features | |
| Mac/OS and app launch command | |
| Fresh synthetic workspace path | |
| Backup/recovery test paths | |
| Current editor choice and pin (Z1 result recorded separately) | |
| Current extraction route and pin (X1 result recorded separately) | |
| Selected provider/model/effort and current authorization | |
| Installed embedding profile, or explicitly unavailable | |
| Known pre-existing limitations | |

Use the shared case pack: draft; requirements with table; evidence with repeated passages/figures; style example; inaccessible other-project/private note; partial/malformed source. Preserve evidence without secrets or real documents.

## Ordered walkthrough

Each step names the master plan section 13 cases it observes natively; the registry there owns applicability. Steps that need generation remain pending without current provider authority.

1. (F16) Open/create the draft and write text. Switch documents and restart. Confirm draft recovery and no implicit data loss.
2. (F01) Attach an existing reference and import another through the native file picker. Check progress, fidelity/partial states, assets and correct project/private scope.
3. (F04, F21) Add a brief and reference purposes. Open a new chat, switch chat and close Chat.
   - Verify these stay available in the same writing context, and that the tile shows the active scope and conversation.
   - Open the same shared draft from a second project. Confirm that the first project's brief, reference list and private reference do not appear there (previews, lists, search, sources) and are not captured in that project's requests.
4. (F05) Select the second copy of an identical paragraph and add a comment. Repeat with Estonian diacritics, combining characters and emoji. Confirm exact highlighting/reveal.
5. (F05, F20) Resize, wrap and scroll a long document. Reopen the comment after layout changes; test keyboard selection, clipboard and native IME/composition. Record any unperformed IME case explicitly.
6. (F06) Add several comments plus overall feedback and run a document-level rewrite with Chat closed. Confirm actual Sources used and a review candidate, not an automatic saved rewrite.
7. (F07, F08) Ask for a paragraph change, then separately ask for a full requirements check against an oversized or partially imported reference. Confirm that review shows import coverage, retained text read and requirements reported as checked separately, and never claims full original-file coverage or that all requirements were verified.
8. (F11) Inspect before/after review and source passages. Confirm references were not modified, unsupported material is flagged and source content cannot trigger actions.
9. (F09) Generate another revision, then type a new edit before completion. Confirm typing is retained and stale Apply cannot overwrite it.
10. (F10) During a run, perform each of these and check the stated result:
    - Edit the brief: the candidate becomes stale.
    - Edit a reference's content: the candidate remains applicable and shows "changed since".
    - Remove a reference or revoke project access: Apply is refused. No earlier approval or retry applies it, and no inaccessible content leaks through cached previews.
11. (F12, F15) Apply an exact reviewed revision; repeat the operation and use durable Undo. Then test local typing Undo and subsequent Save. Confirm no duplicate operation or stale text resurrection.
12. (F03, F16) Stop import/rewrite at a late stage. Confirm no false success/late save. Restart and inspect retained work, queued requests, candidate identity and cleanup.
13. (F14; F13 only if retrieval behavior changed) Exercise scoped search, source reading and Connections with a private distractor. Check visible counts and source previews. If retrieval behavior changed, also exercise unavailable/partial embeddings and rebuild a disposable derived index.
14. (F17) Backup the synthetic workspace and restore to a separate fresh destination. Confirm references, brief and setup, assets, drafts/revisions and history survive without the original binary source being retained in BRN.
15. (F18) Launch the minimal writing development host (the `brn-writing-ui` binary built without `brn-desktop`) and run a representative real-service comment/rewrite/review/Undo flow. Confirm it does not need BRN's entire desktop shell.
16. (F19) Recheck ordinary chat, project navigation, queue/Stop, tasks and an existing authorized-maintenance fixture. Confirm writing modularization did not change their behavior.

## Results

For each step record pass/fail/blocked/not-run, the exact tree, the observed result and the evidence. Together these steps make up F20. Distinguish source-fidelity errors from UI defects and provider failures. Compilation, screenshots and headless state tests alone do not satisfy interactive steps.

C must fix blockers and rerun affected cases. Owner acceptance, main merge and release are separate recorded decisions, not implied by completing this checklist.
