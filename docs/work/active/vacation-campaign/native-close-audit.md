# Unsaved Note close/relaunch qualification gap

Source audit at reviewed14d7c43e found no reproduced production defect. The app's
last-window close hook cancels active work and allows close; last-window closure
quits. Quit similarly cancels then quits. Neither hook explicitly samples the
Note editor. Normal Change handling persists recoverable edits, and reopening a
Note resolves its authoritative current recovery buffer through NoteLifecycle.
Do not infer an OS input-ordering race from setting a test editor before its
queued Change callback. That ordering has not been observed during real close.

`main.rs::fresh_startup_reopens_and_refuses_other_data` covers fresh workspace
reopening/refusal. The toolkit witness
`hidden_note_editor_cannot_gain_edits_from_tab_after_save_or_layout_change`
checks exact durable unsaved text and generation after hiding the Note tile.
Neither establishes actual Mac window close, process relaunch and recovery.

MAC-04 therefore starts with read-only confirmation that the independent draft is
already durable and canonical text is unchanged. After normal close/relaunch of
that same disposable root, opening the Note should load its kept edit; choose
Recover only if the UI offers it. Verify exact text, original base/session and no
implicit canonical Save. This is an existing documented recovery contract, with
native qualification still pending. It does not require a production change or
claim unflushed-input/crash durability. Run it in the consolidated native session,
using the then-current qualified main binary, after the Save and backup scenarios.
