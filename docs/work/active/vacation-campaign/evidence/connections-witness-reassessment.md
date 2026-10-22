# Connections witness reachability reassessment

Disposition: **one source-supported witness correction is available; no product defect is established and no production change is justified yet.** This is a bounded diagnosis, not authorization for an automatic retry.

Reviewed clean worker `/Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust`, branch `codex/connections-project-freshness-20261022`, HEAD `dd0bbc0082d1b402de6986ef809d21ec3092ff13` (product base `10f2ff80a39408ce411e10c77200e6d4a89b12d3`). Read the complete new witness in `crates/brn-desktop/src/threads_tiled_tests.rs:1208–1372`, layout/navigation/Project callbacks and graph lifecycle. No source changes or workloads performed.

## What actually failed

The original `dce95a88df1c22cd6b24b2bdb9fb671465c4912e` attempt exited 101 compiling an invalid `Record.markdown` access (E0609). Corrected `dd0bbc00` compiled and ran one test, then failed in GPUI Kit control lookup before Unlink. Its already-completed assertions establish that the neighbor was rendered and visible in the selected Project, but **revocation and the intended stale-result assertion were not reached**. Recorded cleanup reports group absent, leader reaped and source unchanged for both attempts. Independently recomputed raw-log SHA256 values match receipts: initial `f8b61ce4e30bdff5c2a7db15ebf9850b71f3b9db286d346d2061a185a30ae7da`; corrected `766a41792a55ab903ccb6affb1ba5c8e1952fcc2e3f38dd3cf3dd5b3683f80b1`.

The missing control is explained by supported layout behavior. `Layout::destination` assigns both Projects and Note to Main (`threads_layout.rs:305–320`); ordinary `open` replaces that area's content (`352–382`). Opening `project-doc-{centre}` calls `open_project_note` → `open_note_in_context` → `layout.open(Note)` (`threads_projects.rs:93–109`, `threads_ui.rs:1192–1264`). Projects is therefore gone. `note-connections` requests ordinary Connections navigation (`threads_ui.rs:4520`), which occupies Companion. This is a witness layout error, not evidence that Unlink is unavailable in the product.

## One bounded corrected route

After the existing completed-result assertions and **before** the direct Unlink click, use the actual global public control and existing tested menu helper:

```rust
click(cx, &f, "open-beside");
choose(cx, &f, "Projects");
```

`choose` already locates a `popup-menu` row by its accessible label and clicks it (`threads_tiled_tests.rs:75–89`); no guessed menu ID or new test hook is needed. The global menu includes every non-Settings `TileKind`, including Projects, and invokes `show_beside` (`threads_shell.rs:899–920`). `Layout::open_beside` inserts Projects as an extra split without removing Note or Connections (`threads_layout.rs:386–408`). `show_beside(Projects)` performs ordinary `refresh` and does **not** call `refresh_connections` (`threads_shell.rs:818–830`). `relayout` only preserves input, changes layout and focuses content (`threads_ui.rs:1017–1027`); ordinary `refresh` does not modify the graph (`805–902`).

The prior Project-row click retains `selected_project = shared_project`; document opening, `set_chat`, and refresh do not clear it. `shown_project` resolves that retained active ID (`threads_projects.rs:150–158`). Its explicit `p.documents` membership renders the real `project-unlink-{private_note}` button, whose callback uses checked `apply_project(UnlinkDocument)` (`472–486`). No second document opening or Connections request is needed.

Before opening the menu, capture the completed graph generation. After choosing Projects, render a frame and require: Projects/Note/Connections remain in the visible layout; the same centre and shared scope remain; graph generation is unchanged, pending is false, result retains the private neighbor; both the connection row and Unlink control are actually visible/reachable. This guards against accidentally replacing/requerying the graph or testing an offscreen/absent surface. Use existing scroll/reachability support if needed, rather than bypassing the control.

Then retain the direct `update_window` Unlink click without executor drain, followed by checked SQLite revocation and unchanged retained Note/bytes assertions. On the immediate frame, assert Connections itself remains visible and centre/scope unchanged, and require both rendered-row absence and **no retained graph-result node for the revoked Note** (a cleared result is acceptable). Row absence alone could be a false positive if the pane stopped rendering. If the route/control/baseline preconditions fail again, stop and reassess; do not label that a reproduced product defect.

## Limits

This route is established from source and the existing menu helper, not a newly executed witness. No further run, production repair, native observation or pending-result/Undo coverage is approved by this report. A selected corrected baseline must first reach actual public Unlink, prove current visibility revoked, and fail at the intended retained-result boundary before a repair can be considered.
