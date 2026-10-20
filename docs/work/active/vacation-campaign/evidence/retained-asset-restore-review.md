# Retained attachment restore witness review

Verdict: no actionable findings in the complete bounded candidate.

Reviewed identity: clean `codex/retained-asset-restore-witness` checkout at `57531c3db0112b5b514eba353641d2fb2c836c5d`, against `6622443ee47d3cc6564b160b6f686f3dfcbbfa36`. The entire diff is 32 added lines in `crates/brn-threads-app/tests/chat.rs`; no production code, converter policy, new test, harness, dependency or configuration change. HEAD and clean status were rechecked at review completion.

The requirements and correctness review covered every added line and the existing test/fixture, `Workspace::attach_full`/`import_full_checked`, core record types/read path, backup/restore implementations, and applicable repository architecture/invariants/workflow/verification guidance. Applied the independent-candidate review skill/resource policy and Rust review checklist, including its linked [Rust best-practices handbook](https://github.com/apollographql/rust-best-practices). One read-only review; no nested delegation.

## Assertion effectiveness

- `chat.rs:284–301` obtains an actual Asset from the initial Applied receipt and refuses an absent or empty asset, so the later loop cannot vacuously qualify a byte-free attachment. The pre-existing fixture at lines 240–262 supplies synthetic `[1, 2, 3]` bytes, their SHA-256, and a Markdown figure reference. Source and attachment-Link presence are also explicit.
- `chat.rs:334–347` iterates every original receipt write and queries its exact ID independently from the reopened source and the restored store. `Record` and nested `RecordData` derive structural `PartialEq/Eq` (`brn-threads-core/src/types.rs:425–431,465–471`). Equality therefore covers ID, version, archived state, exact retained bytes, all Note/Source provenance/coverage fields, asset ownership/source references, and complete Link endpoints/relation. Missing records or changed fields fail; this is more than a Note/read-success or record-count assertion.
- The expected values remain the first attachment's immutable receipt in memory. Backup is created after both attachments, the original Workspace is dropped, a new Workspace is opened, and restore uses a fresh explicit temporary directory. All exact comparisons happen before Keep and Undo change versions/state.
- The second chat's import legitimately has a separate source identity: `intake.rs:388` filters sources by exact chat scope, and creation IDs are bound to distinct host operations. It does not intentionally mutate the first receipt records, so exact initial-record equality is a valid state-order expectation.
- Existing private visibility, general search/export exclusion, Keep on the same protected Note, replay of Keep, and Undo assertions remain in their original order. The added assertions reuse that journey without broadening runtime behavior.

## Checks and limits

Reviewer ran only source/Git inspection and `git diff --check` (passed). No tests, builds, lint execution, native interaction, live/provider calls, account actions, cache actions or repository edits were performed by this reviewer. The lead's focused test and strict chat-target lint/format results were unavailable to this review; no pass is inferred.

This qualifies the intended synthetic app/core persistence witness if its focused checks pass. The three-byte fixture is not a decodable PNG qualification. The review establishes no converter extraction/fidelity, production AI-Off DOCX Asset output, CLI/native usability, owner acceptance, merge or release result. No broader campaign archive was needed to assess the change.
