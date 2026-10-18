# Actual controlled results

Focused exact test: **1passed/0failed/0ignored**, four independently fresh contexts,480 other tests filtered. [Controlled safe projection](controlled-facts.json); complete raw output/command/source/executable/.d receipts in `raw/`. These counts are not four independent test cases or four live requests.

| New fixture | Mock model requests | Actual prepare calls/results | Final canonical facts |
|---|---:|---|---|
| Writing absent |3|1success|1Prepared/registered operation, no receipt for that candidate |
| Writing null |3|1success|1Prepared/registered operation, no receipt for that candidate |
| Writing object, then long metadata variant |4|2error15bytes|ToolRejected in direct typed/private calls, native Other/generic feedback; no candidate/registered operation |
| Checked Stop at actual dispatch |2|1error15bytes|ToolRejected; no candidate/registered operation; current Run Interrupted/fence2, finish refuses late acceptance; next model request blocked |

All protected target Notes/version and all receipts touching each target remain byte-equivalent. No apply_changes or external request ran. The stopped loop cannot send its result to a later model request because liveness blocks that request; the trace records its15-byte output and a separate maintained native adapter check asserts exact `the tool failed` feedback/retained typed source. In object mode the actual next mock-model request also contains that generic feedback. Typed source observations use existing private test access; callbacks themselves do not expose them.

Actual existing callback: every short complete prepare event parsed, classified absent/null/object (Stop absent), matched the full three-entry input ID/version array, and contained `[record content omitted]` rather than either Note-body canary. Long event was explicitly truncated/unparseable, exposed no writing field and **none of the three input IDs**. No safe full-field capture guarantee follows from field order or a short example. Keep metadata-loss flags, never reconstruct partial JSON or lost original calls.

## Preserved attempts

Initial formatting check exit1 showed only unformatted appended test; direct pinned rustfmt fixed it. Initial compiled diagnostic **0passed/1failed** was a new fixture expectation failure: it assumed all input IDs survived the long-event cap. Failed source/binary/.d and exact receipt/log are retained before correction. The correction keeps all original assertions and short-event exactness, measures actual per-ID visibility, and requires demonstrated long-event metadata loss. It changes no production behavior and is not a failing-before production witness. No source fix, blanket retry, old review qualification closure or historical root-cause conclusion is claimed.

The original3 live errors remain UNKNOWN. Both controlled branches yield the recorded shape, proving nonuniqueness rather than selecting a historical cause. No storage-failure branch was executed. [Capture-method limits/proposal](method-and-limits.md) explains why existing Activity can help on complete short events but Full trace cannot yield the underlying AiError. Any future synthetic normal-Auth run remains unactivated and requires separate prospective approval/pin/fixtures/DB+WAL+SHM binding/one-start cap. Original2-start window stays closed; no unused-start authority.
