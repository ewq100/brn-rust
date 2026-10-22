# Precise edit coordinate guidance implementation

Implemented on `codex/precise-edit-guidance`, based on `c4f0e2e663cb04654c2bc49f9a540499fdd139b9`.

## Changes

- Clarified `prepare_note_edit` schema descriptions and tool description in `crates/brn-ai/src/threads_note_edit.rs`: offsets refer to decoded Note Markdown; derive `start` from returned `note.start` plus the UTF-8 byte length of the prefix within returned `note.markdown`; derive `end` by adding `expected_text`'s UTF-8 byte length. On mismatch, reread the exact passage and correct arguments with a fresh `request_key`.
- Added the same concise coordinate and recovery guidance to the existing local-edit passage in `agent/BRN.md`.
- Bumped the packaged guidance identity to `brn-threads-guidance-18` and updated the existing identity assertions in `crates/brn-ai/src/threads_guides.rs`, `crates/brn-ai/src/threads_summary_tests.rs`, and `crates/brn/tests/agent_runs.rs`.
- Left schema shape/types/required fields and all edit handlers, authorization, guards, replay behavior, and apply/review flow unchanged.

## Source and rationale

`read_note` returns a nested `note` object with delivered `markdown`, absolute Markdown `start`/`end`/`next_start`, version, and total byte information. The tool's slice is from Note Markdown after decoding JSON, so JSON escape/wrapper positions do not form edit coordinates. Existing validation requires `end - start == expected_text.len()`, and the handler checks the exact current range without searching or relocation. This guidance makes that relationship explicit and gives a clear mismatch recovery step.

## Limitations

This is a clarity improvement only. It does not guarantee that a model will calculate offsets correctly, and no model run, test, build, account/provider action, native action, or external action was performed. The observed mismatch does not establish a specific cause; this change makes no claim that a particular arithmetic mistake was fixed.

## Regression fixture correction

The lead's focused run of the committed change on frozen `6b719f6e` failed in `guide_bundle_change_recomputes_prior_stamp_summary_then_reuses_unchanged_stamp` at line 1630 (`left: 0`, `right: 1`); the captured output is `/tmp/brn-precise-edit-guidance-qualification/initial.log`. The fixture still transformed `brn-threads-guidance-17:` to `brn-threads-guidance-16:` after the current bundle had moved to version 18, so it no longer constructed the intended prior identity. Corrected that transformation to 18→17 in `crates/brn-ai/src/threads_summary_tests.rs`. A source search found no other live fixture transformations based on version 17; historical evidence references were left unchanged. No checks were run during this correction.
