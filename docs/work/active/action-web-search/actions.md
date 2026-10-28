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

## Integrated qualification

Normal main refresh onto `d2f03975` preserved the reviewed query implementation.
The shipped guide now distinguishes Action discovery from exact record reads;
bundle identity advances from 24 to 25. Independent review cleared scope and
cache invalidation. At this integrated source, eight query tests, the production
catalog dispatch test, three packaged-guide tests and the prior-stamp summary
recomputation/reuse regression passed. Strict AI library Clippy and formatting
passed. The first query selector matched zero tests; that is not qualification.
The corrected `action_query::tests` selection ran all eight tests. Logs are in
`/workspace/brn-next-evidence/action-query-integrated/`. No live calls were made.

## Hosted correction checkpoint

At `a6cb06eb`, Linux default-workspace CI failed with exit 101; the permitted
annotation did not identify individual tests. Source inspection found one missed
CLI persistence assertion still expecting bundle 24 despite the intentional
bundle-25 guidance change. It now expects 25, retaining every other assertion.
Independent review found no other stale executable expectation (the summary
regression intentionally retains prior stamp 24). The focused CLI check is
pending at this checkpoint. This fixes a demonstrated assertion mismatch, not
a claim that every original hosted failure has been diagnosed. No denied log
download or blind rerun was attempted; the corrected head requires fresh CI.

## Mac failure diagnostic — 28 October 2026

At `b2db293385b4b7192970d4f077929ba3277bfa66`, CI 40658350078 passed
five supported jobs but failed Mac default-workspace tests with exit 101.
The permitted annotation contained no test identity. Linux core passed. The
focused local CLI fixture independently failed at `UnsafeCredentials` before
its guide assertion; that is not evidence of the hosted Mac failure's cause.
Restricted historical logs were not retrieved or accessed through another route.

The next qualification adds bounded failed-test-name annotations to the existing
workspace test step. It executes the identical Cargo command, streams its normal
output, preserves its exit status (including signal failure), and publishes at
most 50 ASCII Rust test identities. It does not publish assertion payloads as
annotations, upload raw logs, skip tests, retry failures or alter credentials.
Six deterministic subprocess tests cover status preservation, success, unknown
failures, ANSI/duplicate/hostile names, the bound and exact command. This is a
purposeful diagnostic run, not a demonstrated product repair. If it passes,
the original Mac failure remains unexplained. PR240 stays draft and unmerged.

The existing Mac relay reports monitoring paused. No independent Mac execution
has been claimed or started; consumed live job001 is not reopened.

Independent review cleared the diagnostic's command/status fidelity and bounded
identity-only annotation contract. Local documentation/overview/whitespace checks
passed. The broad local tooling invocation ran 226 tests: one process-group
timeout assertion failed, six Xberg generator tests errored because that invocation
had not loaded the configured Cargo archive location, and one test was skipped.
These failures are retained in the local receipt; they are not identified as the
unknown hosted Mac failure. No timeout assertion or credential guard was changed.
