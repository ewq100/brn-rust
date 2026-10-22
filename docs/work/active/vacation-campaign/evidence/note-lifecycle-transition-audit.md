# BRN note lifecycle transition audit

## Scope and state

Read-only assessment of frozen source checkout `/Users/evokessler/.codex/worktrees/export-durability/brn-rust` at `7dde6a41a4fbc66468e0400d2cc3c859f04c8ef5` (`codex/backup-process-witness-bounds`). `git status --short` was empty before and after. No source/configuration files changed; no tests/builds were run. Only this report was written, at `/tmp/brn-note-lifecycle-transition-audit.md`.

Reviewed the checkout's `AGENTS.md`, architecture overview/invariants, `crates/brn-writing-ui/README.md`, shared lifecycle, desktop host, example host and existing relevant witnesses.

## Finding: non-explicit reload/switch can replace the visible stale generation

**Trigger:** Note A has an open edit session at generation *g*. A valid owner-host writer advances that same durable session to a newer generation *g+1* (for example another window/input path sharing the owner session), while this window's editor still displays the exact generation-*g* text. The owner then invokes ordinary Reload on A, or navigates to another note.

**Path and cause:** `NoteLifecycle::note_changed` returns `Ok(None)` at `crates/brn-writing-ui/src/note.rs:192-193` when `session.markdown == text`, without checking the store's current generation. `open_note` therefore sees no warning and proceeds to `load_note` at lines 87-94. For ordinary same-note Reload, `load_note` resolves the current authoritative recovery buffer by note at lines 111-130, then replaces the editor at lines 137-145. The current generation-*g+1* text replaces the locally visible generation-*g* text even though this was not explicit same-note Recover. For a switch to B, the same `open_note` guard also passes and overwrites the one panel editor with B; A's generation-*g* text is no longer available in that view. The newer A buffer remains durable, but the older visible text had been superseded in SQLite and is lost.

The desktop caller reaches this through `WritingRequest::Reload` at `crates/brn-desktop/src/threads_ui.rs:4499-4502`; note navigation goes through `open_note_in_context` at lines 1192-1224, which calls `preserve_visible_note_input` before the same lifecycle transition. That preservation helper calls `note_changed` at lines 920-925, but the same equality shortcut does not detect the generation conflict. The example host has the same ordinary reload shape in `crates/brn-writing-ui/examples/writing_host/host.rs:399-402`.

**Authority/consequence:** This is a UI transition hazard, not a SQLite authority violation: the newer durable buffer/session identity remains intact, and Save at the old generation is correctly refused. The loss is of the distinct older visible attempt. Explicit same-note Recover is deliberately allowed to select the fresh authoritative buffer; that explicit path should remain unchanged.

**Existing witnesses:** `crates/brn-writing-ui/examples/writing_host/journey.rs:924-1005` proves newer durable generation refuses Save and Discard, retains local text, and explicit same-note Recover adopts the newer generation. `journey.rs:1011-1101` covers blocked local attempts and validates the explicit recovery hint. Desktop `crates/brn-desktop/src/threads_document_tests.rs:422-518` covers a late checked refresh and stale Save; `threads_writing_tests.rs:630-655` covers ordinary dirty-note navigation and returning to the per-note recovery. None exercises ordinary reload or switching after the store generation has advanced while visible text remains equal to the stale local session buffer.

**Smallest justified follow-up:** Before any non-explicit replacement transition (ordinary reload or a different-note open), compare the local `EditSession` identity/generation with the current checked recovery buffer for that same note. If the durable session is newer, refuse the transition and keep the visible text/editor state, presenting the existing recover/compare route. Preserve explicit same-note Recover's existing validated-hint behavior. Add one host-level witness that advances the same session generation, invokes ordinary Reload or note switch, and asserts the old visible text remains while the newer durable buffer is untouched; an explicit same-note Recover should then select the newer buffer. Do not migrate note state or redesign the enum.

## No second finding

The read-only pass did not establish another independent path where a refused Save or automatic refresh assigns an edit to the wrong Note/session owner. `refresh_note` retains an `Editing` or `BlockedElsewhere` state (`note.rs:272-289`); ignored writes retain the editor and old base (`note.rs:195-210`). Existing witnesses cover stale Save/recovery/discard, another caller's edit and confirmed discard (`journey.rs:690-923`), and desktop first-dirty/access-loss behavior (`journey.rs:186-260`). The identified replacement hazard is the untested equality-plus-newer-generation transition described above.
