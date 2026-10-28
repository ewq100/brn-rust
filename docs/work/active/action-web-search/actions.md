# Scoped Action discovery

`query_actions` adds paginated discovery over current Actions visible to the active chat. It reuses BRN's canonical record visibility and writing guards rather than creating a second task store or searching an unscoped index. Helpers cannot call it; frozen writing retains its existing record restrictions. Archived records and other private/project content remain excluded before filtering and cursor construction.

Inputs are typed and explicit:

- `state`: one existing Suggested/Open/Waiting/Done/Cancelled state, or null for no state filter.
- `owner`: an exact stored label, or null for no owner filter. This is a reported label, not a user identity or an inferred commitment.
- `query`: a literal case-insensitive description substring, up to 512 UTF-8 bytes, or null.
- `due` and `follow_up`: separate optional date filters, each with inclusive `from`/`through` YYYY-MM-DD bounds and a `missing` flag. Missing means null, not an invalid legacy date string. Calendar parsing matches the existing pinned chrono semantics. No timezone, implicit today or relative-date business rule is invented.
- `limit`: 1–100; `cursor`: null initially or the previous page's opaque cursor with unchanged filters and limit.

Rows use stable ID order and carry exact current record versions. The encoded result bound may shorten a page while retaining an honest continuation. A single Action that cannot fit returns a bounded-result failure instead of silently truncating its facts. Cursor identity binds the chat, filters, page limit and visible matching record IDs/versions; changed matches or cross-chat replay require restarting. Hidden rows do not contribute to cursors or counts. The tool is read-only: it creates no proposals, completion evidence, read coverage for Notes, Apply authority or receipts.

`read_records` remains the exact-record reader; its literal query searches notes/discussion, not Action descriptions. `query_actions` is the distinct discovery route. Prepare, Apply and candidate lifecycle tools remain separate. A broad read/search facade has no demonstrated benefit sufficient to justify changing their permission and coverage contracts in this batch.

## Local qualification — 28 October 2026

Candidate commits `e95ed8aa` and `25d4bba4` contain the implementation and reviewed follow-up. Eight focused Action-query tests passed, covering filters, encoded bounds, stale and cross-chat cursors, private/project/archived visibility and writing/helper restrictions. The existing production catalog mock-model fixture passed with `query_actions` dispatch and activity checks; strict AI library Clippy passed.

Independent review caught the old catalog-size assertion after adding the tool; the expected catalog and actual dispatch witness were updated without removing prior checks. The initial implementation compile also caught a macro handler-path issue, corrected before passing tests. No live provider or native Mac test was run. The dispatch fixture queries an empty Action set; nonempty filtering is qualified directly, not represented as live semantic quality. Publication and final integrated-head CI remain separate gates.
