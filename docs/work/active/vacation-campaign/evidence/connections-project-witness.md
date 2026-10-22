# Connections project-unlink freshness witness

- Checkout: `/Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust`
- Branch: `codex/connections-project-freshness-20261022`
- Base: `10f2ff80a39408ce411e10c77200e6d4a89b12d3`
- Candidate: `dce95a88df1c22cd6b24b2bdb9fb671465c4912e`
- Changed file: `crates/brn-desktop/src/threads_tiled_tests.rs` (SHA-256 `139d73a5f4668e21b9a179235b948c1c2e5635e763d2fb7e0bb02bfa1e61de6c`)

Added `connections_result_is_cleared_when_project_unlink_revokes_neighbor_visibility` alongside the existing Connections journey. It creates two Projects through checked `Workspace::project_command`: a private project owns a synthetic Note; a second project also links that Note and the fixture's general center. A checked owner write links the two Notes. This avoids the exclusive-last-link refusal: unlinking the second Project leaves the Note retained in its owner Project while removing its visibility from the selected second Project.

The test opens the center from the selected Project's actual Documents row and waits until Connections visibly contains the private marker Note. It verifies `Store::record_visible_in_chat(note, Some(project))` is true at that point. Then it clicks the rendered `project-unlink-{note}` control, which dispatches the real `apply_project` callback. Before checking the Connections frame, it independently verifies the same checked visibility query is now false and the exact retained Note record and marker text are unchanged. The final immediate rendered-frame assertion requires the now-invisible neighbor to be absent. The current source's `apply_project` calls `refresh()` but does not invalidate `graph`; this assertion is expected to expose the stale result on the lead's single baseline run.

Expected focused command for the lead's baseline:

```sh
cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop threads_ui::tiled_tests::connections_result_is_cleared_when_project_unlink_revokes_neighbor_visibility --locked --offline
```

Checks performed here: pinned Rust 1.98.1 `rustfmt --edition 2024` on the changed file and `git diff --check` both succeeded. No build or test was run; the lead owns the shared build slot. The test covers a completed visible result and synchronous UI invalidation after checked revocation. It does not claim to cover an in-flight response race, native rendering, or owner acceptance.
