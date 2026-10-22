# Note exit baseline — bounded input reassessment

**The one-character correction is justified for one explicitly selected corrected baseline.** Use the same ordinary `cx.simulate_input(f.window, "õ")` route; do not add atomic paste, a programmatic text setter, new fault hooks or dependencies. This is witness-route approval, not production repair approval or a new observed Close failure.

Reviewed clean HEAD `06bbf374d2bb2414fd0bf3fca438a642b50d60c8`, branch `codex/note-exit-preservation-20261022`, in `/Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust`. Inspected the complete affected U4 journey, extracted production installer, input subscription/readonly path and actual cached toolkit input/close APIs. No code edits, builds, tests or workspace actions performed.

## Failure disposition and source ordering

The first `ee01` attempt was compile-blocked on the missing `WeakEntity` name. The corrected `06bb` attempt compiled and ran one test, but failed at `threads_writing_tests.rs:1825`: actual live text `"P"` differed from the requested multiline string. The intended Close assertion was **not reached**. Its receipt reports exit 101, source unchanged, group absent and leader reaped; raw-log SHA256 is `40a1c8be475e5acdf1b2ab1b2ba4e18cbbeb6a914c2f32833fd58b8eb68b55e9`. Preserve both construction failures as such.

The result is consistent with the ordinary input API, not evidence that a full string was atomically entered and then lost. GPUI Pre 0.3.9 `TestAppContext::simulate_input` iterates the string's individual character boundaries, dispatching separate keystrokes (`app/test_context.rs:633–639`). Each input reaches the editor and Change subscription. `note_changed` encounters the already-open checked session, records `BlockedElsewhere`, refreshes the recovery state and notifies (`threads_ui.rs:556–568,1398–1417`; `note.rs:179–198`). The Note prose editor becomes readonly when the recovery guard is discovered (`writing.rs:404–405,528–531`). Hence later characters are not part of this live attempt.

`õ` is one Unicode scalar and two UTF-8 bytes. The parser preserves its key; `with_simulated_ime` passes printable nonempty keys into character input (`platform/keystroke.rs:238–263`; `window.rs:5551–5574`). The ordinary simulator therefore supports this narrowing without a new API. It is distinct as a **whole body** from both the saved `"tere tere õun"` and the other recovery body `"Edit from elsewhere"`; sharing a substring with saved text cannot establish exact-byte preservation.

## Required preconditions and intended failure

Keep the existing sequence: install the shared production exit handlers; open the Note; create the other checked edit through a second Workspace; drop that connection; focus/select-all in the real editor; simulate only `"õ"`; drain input work. Before Close require exact visible `"õ"`, `BlockedElsewhere` for this Note/base, no owned local session, and the specific unsaved-text notice. The corrected run already reached the blocked-state check before failing at the longer-text expectation, supporting this fixture ordering.

Keep exact saved-Note equality and capture/compare the **full authoritative updated other EditSession** plus canonical history immediately before input and before Close (not only its Markdown/closed flag). Reuse those comparisons after a refused Close. Require no worker/provider transport and preserve the existing Compare/Discard remainder with its expectations bound to the narrowed `attempted` variable.

Then invoke the actual installed handler through `VisualTestContext::from_window(...).simulate_close()` and require false at the distinctive existing Close assertion. The extracted installer currently still returns true unconditionally, so this is the expected intended baseline failure once all preceding state checks pass. The simulator only returns the callback decision; it does not destroy the test window. Do not count its retained window or post-failure code as native teardown evidence.

If another input/precondition/control failure occurs, stop and reassess; do not substitute a setter or broaden the fixture to manufacture the desired failure. This route qualifies a single live-only Unicode attempt and normal Close admission. Multiline/paste fidelity, Cmd-Q termination, native Cocoa interaction and any production admission repair remain unobserved/unselected.
