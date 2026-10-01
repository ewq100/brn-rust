# Workspace shell evidence

Date: 1 October 2026

Implementation: pending. Verification: not run. Native and user acceptance: pending. Integration: documentation only; no application code changed, and no merge or release.

## Design and planning

- Inspected local `main` at `4f059881f5a34d22ffdf1bb53e79954c94c8608f` during brainstorming. The specification was committed at `9af18f5`.
- Reference: the BRN UI/UX handoff at `/Users/evokessler/repos/brn/brn-ui-handoff` (read-only). It assumes a Pi SDK provider; this repository uses the Codex App Server.
- User decisions during brainstorming:
  - slice 1 is the shell;
  - stay on GPUI Kit 0.6.6;
  - hide unbacked features but document them;
  - JSON layout persistence, without persisting unsent text;
  - System appearance;
  - reflow with tabs;
  - hybrid titlebar;
  - approach A.
- The user approved four design sections, reviewed a visual review page, then approved the written specification.
- The [implementation plan](plan.md) was prepared from the existing `native.rs` structure and the pinned GPUI sources:
  - `TitleBar::window_options`;
  - `Context::observe_window_appearance`;
  - `Theme::change` / `Theme::global_mut`;
  - `Styled::cursor_col_resize`;
  - `FocusHandle::tab_stop`;
  - `MouseMoveEvent::pressed_button`.

No Rust build, test or native run is claimed.

## Next action

Choose an execution mode. At execution time, create an isolated worktree, implement Tasks 1–6 with red-green verification, and record actual commands, results and limitations here.
