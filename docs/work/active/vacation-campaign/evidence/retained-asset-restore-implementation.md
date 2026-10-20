# Retained Asset source-reopen and restore witness

Implemented and committed the bounded test-only change on branch `codex/retained-asset-restore-witness`.

- Candidate commit: `2b3d84b046ffebbd876664c90b9325bb47d62bd5` (parent `6622443ee47d3cc6564b160b6f686f3dfcbbfa36`).
- Changed file: `crates/brn-threads-app/tests/chat.rs`, only the existing `retained_attachment_is_private_after_restart_then_keep_and_undo_change_same_note` witness.
- The existing synthetic conversion contains bytes `[1, 2, 3]`. The witness now requires the initial attach receipt to contain a nonempty Asset, a Source, and the chat-to-note attachment Link. It then compares every `after` record in that original receipt for exact `Record` equality after reopening the source Workspace and after `Store::restore`. This covers IDs, versions, all provenance/metadata and exact Asset bytes while retaining the prior privacy, Keep/replay, and Undo assertions.
- No production code, helper/configuration, fixture, dependency, or harness changed. No build or test was run per assignment; `git diff --check` passed, and the committed worktree is clean.
- This is deterministic app/core persistence coverage over the existing synthetic 3-byte DTO. It does not establish real converter output, a valid PNG, CLI intake, native behavior, or owner acceptance.
