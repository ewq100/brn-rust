# Consolidated target-Mac qualification

**Status (INT-W, 17 October 2026): native observation NOT RUN.** This task authorizes deterministic headless checks only, no GUI/owner data/provider calls. The [current INT-W record](../../int-w/plan.md), [F-case mapping](../../int-w/f-cases.md) and [compiled deterministic receipts](../../int-w/verification.md) identify the exact local writing-lane candidate. Headless GPUI, including real simulated keyboard/input/click events, is not native-human/IME/menu observation. Independent combined review and owner acceptance remain separate gates.

Run only on an accessible/unlocked target Mac. Never replace source verification with visual impressions. Do not use real owner documents by default. Complete offline parts even if provider authorization is absent, but leave live-generation cases pending.

## Setup record to complete

| Field | Record |
|---|---|
| Candidate commit and build features | Accepted base e8e98304; INT-W immutable pin and actual binary/dep-info/source hashes recorded in `int-w/` handoff. Current serial lanes: `native-ui,native-test-support` tests; combined shipped binary `native-ui,native-retrieval`; native-only writing_host build. No D1A/D1B/D2 integration. |
| Mac/OS and app launch command | Actual local environment in INT-W receipts. Future authorized owner launch: `/private/tmp/brn-int-w-20261017-target/debug/brn-desktop --data-dir ABSOLUTE_FRESH_SYNTHETIC_DIRECTORY`. No GUI launched by this worker. |
| Fresh synthetic workspace path | Owner must create a new explicit disposable directory outside Git; suggested prefix `/private/tmp/brn-int-w-owner-`. Never use the machine's default BRN data or existing owner workspace. Test-generated directories are ephemeral and must not be assumed reusable. |
| Backup/recovery test paths | Fresh sibling `backup` and `restored` destinations under the owner-created synthetic root; original data remains distinct. Existing automated fixtures create their own directories. |
| Current editor choice and pin (Z1 result recorded separately) | Current GPUI Kit0.7.3 editor, shared `crates/brn-writing-ui/src/{native,writing,document}.rs`; Zorite not adopted. Read comment geometry/PC remains deferred; do not merge draft149. |
| Current extraction route and pin (X1 result recorded separately) | Existing restricted `brn-intake-helper` and `brn-threads-intake`; Poppler retained, no Xberg cutover. App import receipt resolver adds no converter. |
| Selected provider/model/effort and current authorization | No live/auth/model-list authority. B4 example uses a visibly synthetic fixed scripted Codex/gpt-6.1-sol/medium selection with no transport. BRN generation remains pending unless separately authorized; deterministic provider fixtures are not live evidence. |
| Installed embedding profile, or explicitly unavailable | No model assets installed or loaded for INT-W. Native-retrieval feature builds/error-state fixtures use existing cached arm64 ORT only; no semantic relevance claim. |
| Known pre-existing limitations | Native pdftoppm absent; carried PDF Unavailable. GUI/IME/picker/native menu click, first-input pre-render native ordering and GUI quit scratch cleanup unobserved. Hosted candidate lanes, full program/owner acceptance pending. |

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
15. (F18) Launch `/private/tmp/brn-int-w-20261017-target/debug/examples/writing_host` (the native-only example built without a `brn-desktop` dependency) and run a representative real-service comment/rewrite/review/Undo flow. Confirm it does not need BRN's entire desktop shell. Use ⌘/ on a saved passage, ⌘S to Save and ⌘⇧R to Rewrite. Its script uses real shared tools/service and exact candidate controls; it is not a live provider. Check actual editor right-click Comment and explicit unavailable Ask, and actual GUI quit cleanup of its own synthetic TempDir. Headless Window action dispatch proves none of those OS menu/quit observations.
16. (F19) Recheck ordinary chat, project navigation, queue/Stop, tasks and an existing authorized-maintenance fixture. Confirm writing modularization did not change their behavior.

## Results

For each step record pass/fail/blocked/not-run, the exact tree, the observed result and the evidence. Together these steps make up F20. Distinguish source-fidelity errors from UI defects and provider failures. Compilation, screenshots and headless state tests alone do not satisfy interactive steps.

C must fix blockers and rerun affected cases. Owner acceptance, main merge and release are separate recorded decisions, not implied by completing this checklist.


## Current deterministic companion paths and native result ledger

| Native steps | Current executable deterministic companion (not native observation) |
|---|---|
| 1, 9, 11, 12 | `crates/brn-desktop/src/threads_document_tests.rs`, `threads_writing_tests.rs` and workbench/recovery/queue tests. Existing real-editor local Undo-after-Apply/refresh/Save witness is distinct from durable receipt Undo. |
| 2–3, 7–10, 13 | `crates/brn-desktop/src/threads_writing_tests.rs` B3 setup/import/dirty-completion/Sources-used and CW2 private review/Compare/cache/real typing cases; `crates/brn-threads-app/tests/{writing_setup,cw2_review,writing_context,import_coverage}.rs`; AI `threads_{cw2,writing,search,reads,chat}_tests.rs`. |
| 4–5 | App/core writing exact UTF8/range tests, library formatting/Read mapping and desktop keyboard selection. No substitute for IME, native clipboard or visual layout observations. |
| 14 | App/core backup/restore/schema7, imported reference and export tests; maintained CLI offline fixture journey. Owner raw inspection/full backup intentionally retains private state; general Markdown export must omit private Notes/scoped setup. |
| 15 | `crates/brn-writing-ui/examples/writing_host/journey.rs`; ordinary `cargo test -p brn-writing-ui --features native-ui,native-test-support --locked --offline` automatically executes the example with `test=true`. |
| 16 | Existing full default/native desktop regression and maintained offline `scripts/verify-end-to-end.sh --fixtures-only`; no live/provider substitution. |

**All native steps 1–16: NOT RUN by INT-W.** No GUI launch, human observation, native picker or screenshots claimed. At the later owner gate record exact source/binary, fresh paths, each action and observed result separately; provider-dependent steps remain blocked without new authority. A native pass must preserve P1/P2/general privacy, exact review/Apply and local/durable Undo; failures return to the principal before acceptance. Do not claim the milestone accepted from deterministic results alone.

## INT-W deterministic outcome (separate from native results)

Corrected-source native desktop288 passed /0 failed /1 existing ignored; writing library44+real host5 passed, app/native247/0/1, combined desktop55 passed; fresh/reopen combined startup and maintained108-step/1266-check offline CLI journey passed. Default workspace1116 passed /1 carried PDF Unavailable /6 existing ignored. These counts overlap suites and are not native observations. Two compiled retained-Source export/list leaks were corrected with existing canonical visibility; the final new real-service and rendered list/count canaries pass. [Before/after](../../int-w/privacy-findings.md), [exact source/artifact ledger](../../int-w/commands.json) and [program limits](../../int-w/program-status.md). No hosted/native-human/live/owner result or milestone acceptance follows from these deterministic checks.
