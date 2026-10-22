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

## Correction after lead baseline

The first lead baseline was compile-blocked (`E0609`): `Store::note` returns `Record`, not the inner `Note`. The test now pattern-matches `RecordData::Note` before asserting the synthetic marker text. This is a type-only assertion correction; it does not change the scenario or expected failure. Corrected candidate: `dd0bbc0082d1b402de6986ef809d21ec3092ff13`; source SHA-256 `a473ac6b95747cec573f10d304df2510684f3e6bf8f204a5180c966cf3012296`. The corrected candidate has not been built or executed here; the intended runtime behavior remains unobserved pending the lead's next bounded baseline.

## Reachability correction after the second baseline

The corrected `dd0bbc00` baseline compiled, but stopped before unlink because ordinary navigation replaced the Projects pane when the test opened the Note and Connections. This was a test-route failure, not a product reproduction. The test now uses the existing `open-beside` control and `choose(..., "Projects")` menu route after confirming the completed graph. New preconditions require Note, Connections and Projects all remain visible, the same center and selected scope remain, graph generation/result are unchanged and complete, and the actual Unlink control is visible and within the window. After clicking Unlink, it additionally requires that the Connections pane remains visible and neither the result model nor the rendered list retains the revoked neighbor. No production change was made.

Revised candidate: `ff49e6a644bc27837194724acbf3ff18fb80ccc5`; changed source SHA-256 `65eec3c72e7c707c7d510064c807b7e2b48422e2dbaf36a23625fc9c53969b12`. Pinned formatting and `git diff --check` passed. This revision has not been built or executed; it is awaiting the lead's single reassessed baseline. Both earlier failed receipts remain preserved and are not evidence of the product defect.

## Selected repair candidate

Lead's corrected baseline at `ed4e1d2e25ade884fce86b662a5d8138581f8359` compiled and reached the intended post-Unlink boundary. It failed because the Connections graph still contained the formerly visible private Note after the checked visibility query became false and the exact Note remained unchanged. The log and result are preserved at `/tmp/brn-connections-project-qualification/baseline-observed/`; the earlier two failed receipts remain distinct and were not treated as reproductions.

Implemented the selected narrow repair in candidate `d7994250fe33f0ffff40d9281bd034fb3b7d0dcf`:

- `threads_projects.rs`: after a successful checked Project command and normal record refresh, call the existing `refresh_connections` owner. Failed Project commands still return through the existing error path without claiming success.
- `threads_ui.rs`: after the existing checked Undo reconciliation and open-Note reload, call the same `refresh_connections` owner. This covers membership restoration without adding a new cache, visibility predicate or field.
- `threads_tiled_tests.rs`: extend the same regression journey to drain and verify the replacement request does not restore the revoked neighbor, then Undo the exact unlink through the public Changes row while Connections remains visible. It verifies checked access returns, retained Note bytes/record remain exact, and the refreshed graph includes the neighbor again.

The test deliberately does not claim to deterministically race an in-flight request; the existing generation/cancellation behavior remains owned by `refresh_connections`. Focused qualification command:

```sh
cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop threads_ui::tiled_tests::connections_result_is_cleared_when_project_unlink_revokes_neighbor_visibility --locked --offline
```

Pinned Rust 1.98.1 `rustfmt --edition 2024` on the three changed Rust files and `git diff --check` passed. No build/test was run for this candidate; lead owns the shared slot and must qualify it. Candidate is clean and changes only the two mutation entry points and this existing test file. File SHA-256: `threads_projects.rs` `46844511fe84fb9227cb5bac6bc8786826f639f5cf84dbccb267e41072a1834e`; `threads_ui.rs` `9aaed2b1ee2aa4a37a75539acd1909b8e7c74c0ab5f9366ab7de484c7329ec4d`; `threads_tiled_tests.rs` `ef070eb46c6601d4003e3c7837ad1d826983f0a633e26d34b5a139d1192e7197`.

## Final static preflight corrections and actual baseline result

Before the selected repair implementation, the third author revision `ff49e6a` had a compile-only failure (`E0616`) because the test accessed private `graph.generation`; that mechanism assertion was removed without widening production visibility or adding a hook. The next lead baseline at `ed4e1d2e25ade884fce86b662a5d8138581f8359` compiled and exercised the real Project Unlink callback. It failed at the intended **graph-model** assertion after proving current visibility was false and the exact Note was retained. The subsequent rendered-row assertion was not reached, so no pre-repair post-Unlink row observation is claimed. Receipt: `/tmp/brn-connections-project-qualification/baseline-observed/`.

Final lead preflight also identified a moved-`String` compile issue and an assertion-order problem in the post-Undo frame. The initial immediate row lookup now clones its ID for later phases. After Undo, the test first establishes checked visibility and exact Note equality, then validates the refreshed graph and rendered row. This supports distinguishing canonical Undo failure from derived-view failure. These are test-only corrections; no new behavior or test case was added.

Frozen candidate after these corrections: `c12966d0bc3fc37ff0f64f618de481c54de8c562`. Current source SHA-256: `threads_projects.rs` `46844511fe84fb9227cb5bac6bc8786826f639f5cf84dbccb267e41072a1834e`; `threads_ui.rs` `9aaed2b1ee2aa4a37a75539acd1909b8e7c74c0ab5f9366ab7de484c7329ec4d`; `threads_tiled_tests.rs` `a5765c9fa783786b2366a0f0f219ad89025d2dc479e0e2e662003f9b9935ada9`. Pinned Rust 1.98.1 formatting and `git diff --check` passed after this final correction. No candidate tests/builds were run by this worker; final qualification remains with the lead.
