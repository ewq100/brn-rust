# Project current-document link regression witness

- Branch: `codex/project-current-document-link`
- Base: `79fa3980c5022e475b07888d9e08f396f45b0238`
- Test-only commit: `8c47af45bb0b0ded56cf58632d8d0232de9419c4`
- Owned file: `crates/brn-threads-app/tests/projects.rs`

Added `project_document_links_reject_superseded_notes_but_allow_unlinking_existing_links`. Through public `Workspace.project_command`, it positively links current Notes to separate Projects, supersedes one Note with an expected-version checked `owner_change`, and requires a new link of that retired Note to fail. The test compares every current record row and the full history receipt list immediately before and after refusal, covering project versions and stored provenance/records without claiming operation-reservation state is absent. It then proves the already-existing link to the retired general Note can still be explicitly removed and that the replacement remains linked by its own ID; no automatic replacement routing is accepted.

The regression assertion is expected to fail against the selected old production behavior because `LinkDocument` currently accepts an active Note without checking `superseded_by`; no production code was changed. This is a regression witness only and does not claim a passing test or fix.

Formatting and patch whitespace only: `rustup run 1.98.1 rustfmt --edition 2024 crates/brn-threads-app/tests/projects.rs`; `git diff --check` passed. Not run: `cargo test -p brn-threads-app --test projects`, any other build/test/CLI workload, native/account/network/SQL/dependency work. The worktree was clean after the test-only commit.
