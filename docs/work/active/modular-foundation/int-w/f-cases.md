# INT-W F-case source/evidence map

Requirements mapped from accepted base e8e98304 and current source. [Actual verification](verification.md) and its command/source ledger distinguish earlier-source, corrected-source and unavailable evidence. The [master registry](../proposal/final/01-master-plan.md#13-shared-acceptance-suite) owns requirements/applicability. Existing cases are reused, not counted as new tests.

| Case | Existing meaningful witnesses / qualification lane | Execution lane and remaining qualification |
|---|---|---|
| F01 | app `import_coverage`, intake qualification/fixtures; desktop B3 import attach/reimport; B4 real importer Partial reference | Default/app/native/desktop/host regressions executed; sole carried PDF Unavailable failure, native picker/live conversion pending |
| F02 | app `product::full_import_retains_assets_source_identity_and_rejects_changed_retry_input`, `import_coverage` replay/dedup and B3 reimport; core operation replay | Passed applicable deterministic checks; no importer edits |
| F03 | AI conversion/intake/steering, core cancel/queue/recovery, desktop B3 cancelled conversion | Passed applicable deterministic checks; late Stop native observation pending |
| F04 | app/core `writing_setup`, app `writing_context`; desktop B3 persistence/closed Chat; B4 cold reopen | Passed applicable deterministic checks |
| F05 | app/core writing UTF8/exact anchors; desktop keyboard selection/Read mapping; helper formatting tests | Passed applicable deterministic checks; current editor native IME/geometry pending |
| F06 | app writing multiple comments/overall feedback; AI writing snapshot; desktop workbench/request controls; B4 real journey | Passed applicable deterministic checks |
| F07 | AI scoped search/read/ranges and writing whole-note proposal constraints; app real shared-start integration | Passed deterministic targeted-input safety; not a new paragraph-only product command |
| F08 | AI `threads_cw2_tests` partial ranges/exhaustive/current/quote checks; app `cw2_review`; desktop B3 Sources used | Passed applicable deterministic checks; original import versus retained read versus model claim remains distinct |
| F09 | core/app dirty/stale candidate/version tests; desktop typing/review/recovery | Passed applicable deterministic checks |
| F10 | core `cw2_visibility`; app `cw2_review`, `writing_setup`; AI current-input/evidence tests; native CW2/B3 access cycles | Passed applicable deterministic checks, no earlier review retry authority |
| F11 | core owner/maintenance/writing gates; AI project/delegation/source/read strict tools | Passed deterministic hostile-source/forged authority cases; no live claim |
| F12 | core changes/Undo/writing; app review/cw2_review and existing exact replay/compensation; desktop Apply/Undo | Passed applicable deterministic checks |
| F13 | Existing default/native retrieval capability/error/rebuild tests run in integration | Ranking/model/index eval inapplicable: no ranking/index/model implementation change; not waived F14/F21 |
| F14 | app search worker/search/Connections/current visibility; AI real read/search/count/trace/asset scope tests; desktop checked cache/privacy | Passed, including F21 matrix |
| F15 | native real editor Undo-after-Apply/refresh + attempted Save fixture from B1a; B4 immediate editor refresh before durable Undo | Passed applicable deterministic checks; local editor Undo distinct from receipt Undo |
| F16 | core/app replay/recovery/drafts/queue/lease; CLI offline recovery and desktop fixtures | Passed applicable deterministic checks; genuine crash/native owner observation still separate |
| F17 | core backup/restore/schema7 compatibility; app imports/setup references + backup; offline export/restore fixtures | Passed applicable deterministic checks; privileged backup is not scoped model/UI delivery |
| F18 | ordinary native writing tests automatically include writing_host test=true, real Workspace + actual shared scripted tools/candidate/comment/review/Apply/Undo | Passed current cmd-shift-r host fixture; actual GUI launch remains pending |
| F19 | full default + desktop regression; CLI offline journeys; AI maintenance/projects/chat queue/Stop | Passed applicable deterministic checks |
| F20 | consolidated [native outline](../proposal/final/06-native-qualification.md) | NOT RUN: GUI/native observation outside authority; headless is deterministic only |
| F21 | CW1 separate lists/search/export/setup/request tests; CW2 real P1 evidence/Apply and general model search; desktop actual P1/P2/general review/Compare/cache/setup labels | Joined parameterized real Workspace P1/P2/general matrix executes actual P2/general shared runs, tool counts/cursors/snippets/Connections/exact reads and Sources used with visible positive control, plus exact P1 owner Apply/replay/Undo. Retained-Source export and native title/count canaries exposed two genuine defects; allocated fixes pass. Existing native privacy/Compare/cache and setup/request witnesses remain complementary; review/native-human qualification still open |
| F22 | default normal/features graph/build/strict all-target lint/tests; native library/host/mac desktop gates; source boundary review | Local Mac default/native build/lint/test executed; hosted required Ubuntu/macOS candidate lanes pending without publication |

Deterministic results and exact per-command source identities are in [verification](verification.md); rows refer to applicable existing tests, not one fabricated new dedicated test per F case. F01 retains the PDF qualification failure; deterministic passes do not make that gate green. F20/hosted/live gates remain unperformed. No new skipped/ignored test or unchanged mutation rerun is authorized. Section17 cannot be recorded sufficient across a demonstrated privacy/version/replay/Undo defect.
