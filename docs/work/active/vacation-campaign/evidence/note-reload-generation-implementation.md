# Note reload generation guard candidate

Candidate branch: `codex/note-reload-generation-guard`
Base observed at task start: `8c18845ca5b70a43d36b0bc0d2811da573086a79`
Candidate HEAD: `bd3c1f695b07a249c357448c8992fb6c25c035f4`

Commits:

- `28bb965c113b0896baabfe89f9d1008ebba14a8e test: guard newer note generation transitions`
- `bd3c1f695b07a249c357448c8992fb6c25c035f4 fix: guard note replacement against newer recovery`

The regression witness extends `note_newer_buffer_generation_refuses_save_and_discard_until_explicit_recovery`: it requires ordinary same-note reload and navigation to another valid Note to retain the old visible generation, local session, record, canonical state, history and newer durable buffer; it also preserves explicit same-note Recover and proves later navigation succeeds.

Production correction is at the shared replacement boundary in `NoteLifecycle::open_note`. After sampling current visible input, non-explicit replacement reads the local edit's current checked store session. A closed or generation-advanced session returns an owner-facing `AppError::Invalid` directing Recover/Compare, leaving local lifecycle/editor state unchanged. Explicit same-note Recover still skips the guard and goes through existing recovery-hint validation. The README records the boundary. It adds no polling, schema, persistence write, or public API.

Validation performed: `git diff --check` passed. No tests, builds, native checks, provider/account/network operations, dependency changes or toolchain changes were run. Lead owns the single post-fix execution and integration. Menu inspection found the production desktop Reload saved note control is disabled for dirty sessions; this candidate establishes the shared API guard and valid navigation behavior, not native Reload-menu reachability.

Initial worktree was clean on the assigned branch; it is clean after the two commits. The initial short base ref `main2078c188` supplied in the assignment did not resolve locally; the actual checked-out commit is recorded above.

## Full candidate diff from observed base

The following is the author’s initial bd3c source record, before the separately reviewed mechanical formatter commit d521. It is literal patch data. The first lead documentation check interpreted its unfenced README-relative links as live evidence links and failed; fencing preserves the original diff without that interpretation. Final qualification and review pin d521.

```diff
diff --git a/crates/brn-writing-ui/README.md b/crates/brn-writing-ui/README.md
index edce3586..70458cd2 100644
--- a/crates/brn-writing-ui/README.md
+++ b/crates/brn-writing-ui/README.md
@@ -8,4 +8,6 @@ The panel composes three different kinds of state. [`NoteLifecycle`](src/note.rs
 
 A host owns and admits the `Workspace` context, supplies the current records, history, candidates and recovery buffers to [`WritingView`](src/native.rs), and dispatches panel requests through the shared app/service operations. Connect editor Change events to `note_changed`, and use the panel's Save, refresh and navigation operations so current visible input is sampled before publishing or replacing it. The host also handles recovery/discard choices, access changes and visible errors. Rejected or stale writes must leave the owner’s visible text available for comparison; hosts should use checked `Workspace` operations rather than write canonical SQLite state directly. The disposable [`writing_host` example](examples/writing_host.rs) shows this wiring with synthetic data; its [host](examples/writing_host/host.rs) owns the workspace and callbacks.
 
+Before a non-explicit note replacement, the lifecycle rechecks its local recovery session against the checked store and refuses replacement if that buffer changed or closed; explicit same-note Recover still adopts the validated current buffer.
+
 For component changes, consult the [verification guide](../../docs/development/verification.md) and choose evidence for the affected boundary. Widget or state checks do not establish native interaction quality; the guide describes the separate native qualification limits.
diff --git a/crates/brn-writing-ui/examples/writing_host/journey.rs b/crates/brn-writing-ui/examples/writing_host/journey.rs
index f8f3ea51..faf975d1 100644
--- a/crates/brn-writing-ui/examples/writing_host/journey.rs
+++ b/crates/brn-writing-ui/examples/writing_host/journey.rs
@@ -925,6 +925,15 @@ fn note_newer_buffer_generation_refuses_save_and_discard_until_explicit_recovery
     cx: &mut TestAppContext,
 ) {
     let (window, view) = host(cx);
+    let other_note = cx
+        .update_window(window, |_, _, cx| {
+            view.update(cx, |host, _| {
+                host.workspace
+                    .new_note("Another synthetic document", "Other saved text")
+                    .unwrap()
+            })
+        })
+        .unwrap();
     cx.update_window(window, |_, window, cx| {
         view.update(cx, |host, cx| {
             let length = host.panel.note_editor.read(cx).value().len();
@@ -979,6 +988,77 @@ fn note_newer_buffer_generation_refuses_save_and_discard_until_explicit_recovery
                 host.workspace.store.recovery_buffers().unwrap()[0].markdown,
                 "A newer durable generation"
             );
+            let local_record = host.panel.note.as_ref().unwrap().record().clone();
+            let newer_buffer = host.workspace.store.recovery_buffers().unwrap();
+
+            // A reload or navigation must not silently replace the locally
+            // visible generation just because it still equals the stale local
+            // session. Only explicit same-note Recover may adopt the newer
+            // durable generation.
+            assert!(host
+                .panel
+                .open_note(
+                    &mut host.workspace,
+                    &session.note,
+                    None,
+                    window,
+                    cx,
+                )
+                .is_err());
+            assert_eq!(
+                host.panel.note_editor.read(cx).value().as_ref(),
+                "Local first generation"
+            );
+            assert_eq!(
+                host.panel.note.as_ref().unwrap().session().unwrap(),
+                &session
+            );
+            assert_eq!(
+                host.panel.note.as_ref().unwrap().record(),
+                &local_record
+            );
+            assert_eq!(
+                host.workspace.store.recovery_buffers().unwrap(),
+                newer_buffer
+            );
+            assert_eq!(
+                host.workspace.store.note(&session.note).unwrap().unwrap(),
+                saved_before
+            );
+            assert_eq!(host.workspace.history().unwrap(), history_before);
+
+            assert!(host
+                .panel
+                .open_note(
+                    &mut host.workspace,
+                    &other_note.id,
+                    None,
+                    window,
+                    cx,
+                )
+                .is_err());
+            assert_eq!(
+                host.panel.note_editor.read(cx).value().as_ref(),
+                "Local first generation"
+            );
+            assert_eq!(
+                host.panel.note.as_ref().unwrap().session().unwrap(),
+                &session
+            );
+            assert_eq!(
+                host.panel.note.as_ref().unwrap().record(),
+                &local_record
+            );
+            assert_eq!(
+                host.workspace.store.recovery_buffers().unwrap(),
+                newer_buffer
+            );
+            assert_eq!(
+                host.workspace.store.note(&session.note).unwrap().unwrap(),
+                saved_before
+            );
+            assert_eq!(host.workspace.history().unwrap(), history_before);
+
             host.panel
                 .open_note(
                     &mut host.workspace,
@@ -1002,6 +1082,20 @@ fn note_newer_buffer_generation_refuses_save_and_discard_until_explicit_recovery
                     .generation,
                 session.generation + 2
             );
+            // Once the owner explicitly adopts the fresh recovery, ordinary
+            // navigation can proceed and leaves that durable session intact.
+            host.panel
+                .open_note(&mut host.workspace, &other_note.id, None, window, cx)
+                .unwrap();
+            assert_eq!(host.panel.open_note_id(), Some(other_note.id.as_str()));
+            assert_eq!(
+                host.panel.note_editor.read(cx).value().as_ref(),
+                "Other saved text"
+            );
+            assert_eq!(
+                host.workspace.store.recovery_buffers().unwrap(),
+                newer_buffer
+            );
         });
     })
     .unwrap();
diff --git a/crates/brn-writing-ui/src/note.rs b/crates/brn-writing-ui/src/note.rs
index a137498f..efbd6c02 100644
--- a/crates/brn-writing-ui/src/note.rs
+++ b/crates/brn-writing-ui/src/note.rs
@@ -91,6 +91,17 @@ impl WritingPanel {
         {
             return Err(AppError::Invalid(message.into()));
         }
+        if !recovering_current
+            && let Some(session) = self.note.as_ref().and_then(|open| open.session())
+        {
+            let current = workspace.store.edit_session(&session.id)?;
+            if current.closed || current.generation != session.generation {
+                return Err(AppError::Invalid(
+                    "The recovery buffer changed or closed. Recover or compare it before replacing this visible edit."
+                        .into(),
+                ));
+            }
+        }
         self.load_note(workspace, id, Load::Recover(recovery), window, cx)
     }
     fn load_note<T: 'static>(

```
