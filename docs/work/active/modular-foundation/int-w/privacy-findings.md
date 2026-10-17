# Confirmed INT-W F14/F21 retained-Source delivery failures

**Confirmed defects in accepted e8e98304; bounded corrections allocated and applied.** Both new fixtures compiled and failed actual behavior before the production corrections; no missing API/compiler error is called red proof. Exact precommit source maps, commands and binaries are in `/private/tmp/brn-int-w-20261017-verification`.

## Canonical state and contract

A Note can have `chat=None` and retain an ImportCoverage Source whose `chat=Some(P1)`. Checked owner operations create this valid synthetic state. The existing core `record_in_chat`/`visible` predicate follows that Source and denies the Note in general/P2, while allowing it in P1. The app/runtime honor that predicate: the parameterized real-service matrix executes P2/general scoped search/list/exact read/Connections and candidate Sources-used checks without delivering the hidden reference. This is the same visibility edge already used by accepted CW2 general-runtime tests, not a new permission model.

F21 explicitly covers scoped general UI/export delivery. Privileged owner backup/raw records remain distinct; they cannot justify exposing canonical-private titles/content in a general Notes list or general Markdown snapshot.

## Executed export failure

- Test: app `tests/cw2_review.rs::int_w_f21_private_context_never_enters_p2_or_general_delivery`.
- Added canonical retained-Source variant to the same fixture, preserving the original pre-INT-W file prefix. The simple Note.chat=P1 variant passes; the retained-Source variant reaches and passes its P2/general model/tool/request/Sources-used negative checks, then fails general export.
- Command: `cargo test -p brn-threads-app --test cw2_review --locked --offline int_w_f21_private_context_never_enters_p2_or_general_delivery -- --exact --nocapture`.
- Actual result: **0 passed /1 failed /2 filtered**. Assertion: `general export leaked INTW_P1_TITLE`, at the actual test source line1213 in this receipt. Later export canary assertions are not claimed reached.
- Raw log SHA256 `2f8dced716764579d761dc4f7eeaf1f292433f70f40d8a0bda7d107e09a5cea8`, label `retained-source-export-canary`.
- Cause: app `src/export.rs::publish` selects Notes only by active/non-superseded/Note.chat=None. Its Source filter removes the scoped Source metadata but still writes the private Note's title/body.

## Executed Notes UI failure

- Test: desktop `threads_ui::writing_tests::int_w_general_notes_list_hides_p1_reference_with_scoped_retained_source`.
- Real Workspace P1 reference/setup, canonical visibility positive in P1 and negative in general/P2; actual shared Note opened in general and actual Notes tile rendered. Visible public Note is a rendered positive control.
- Command: `cargo test -p brn-desktop --features native-ui,native-test-support --locked --offline threads_ui::writing_tests::int_w_general_notes_list_hides_p1_reference_with_scoped_retained_source -- --exact --nocapture`.
- Actual result: **0 passed /1 failed /288 filtered**. Assertion: `general Notes UI must not reveal a P1-private retained-source reference title`, line4002 in this receipt, at actual `notes-recent-{id}` observation. Later kept/count/P2 iteration assertions are not claimed reached.
- Raw log SHA256 `1ecc95f052202ad4cf030e953b6f99166742ad6632595d5aa00cd6f2c419c9b1`, label `retained-source-notes-ui-canary`.
- Cause: desktop `threads_lists.rs::note_rows` filters Note.chat=None in both general-ID/count and NoteRow construction. `ThreadsUi::refresh` supplies the raw canonical records. Canonical retained-Source visibility is not applied before rendering the title/count.

## Exact hunk allocation granted and applied

1. `crates/brn-threads-app/src/export.rs::publish`: retain its single consistent full canonical snapshot for `record_in_chat(record, canonical, None)` when choosing exported Notes; preserve current scoped-Source metadata exclusion, fresh-snapshot publication, assets and manifest behavior. No importer/schema/export format or policy redesign.
2. `crates/brn-desktop/src/threads_lists.rs::note_rows` plus the needed import: reuse the same `record_in_chat(record, records, None)` predicate in both general-ID/count and NoteRow construction. Keep ordering, activity, section limits and controls/IDs unchanged; no private/general fallback or separate membership policy.

Principal inspected both compiled red receipts and granted only these two production hunks. Export now derives eligible Note IDs against its ONE full canonical snapshot before omitting scoped Source metadata; retained assets are not cloned, and only eligible Notes drive the unchanged asset/publication loop. Notes rows and the general-ID set use the same existing predicate against the original records, after cheap Note/archive/supersession checks. No schema, format, permission or dependency change.

Focused corrected receipts `corrected-export-canary` and `corrected-notes-ui-canary` pass1/0 with respectively2 and288 filtered tests. The native fixture was then strengthened with a private inbound Link and an explicit zero count on the rendered visible Note's derived row, covering both allocated predicates. Final-source focused and broad affected qualifications are separately recorded in [verification](verification.md). The original red source maps/logs remain unchanged; neither defect is classified as a tooling baseline. No mutation execution/budget is inferred.

## Prior integration traversal remains separate

Before adding this source-scope variant/native canary: default1116 passed /1 baseline PDF /6 existing ignores; helper64, capability357/0/1; native library44+host5, desktop287/0/1; native retrieval35, app/native247/0/1; combined55. All25 commands completed, with only the carried default PDF failure. They are valid evidence at their recorded source map, **not proof of these newly exposed privacy surfaces and not final acceptance**.
