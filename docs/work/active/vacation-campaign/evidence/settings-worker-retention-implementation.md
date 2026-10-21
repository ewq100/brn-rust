# Settings worker-batch draft retention

Branch: `codex/settings-draft-isolation`  
Base: lead test update `140439f907edca2dc1d50c1c4d205a5b2ba2f3af`  
Commit: `c01ce1993217063ac63a53ee90750c8e208c737d` (`Restore Settings draft after worker batch refresh`)

Only `crates/brn-desktop/src/threads_ui.rs` changed. `poll_worker` now keeps a local optional `(WorkspaceSettings, ProviderForm)` snapshot when it handles `WorkerEvent::Finished` while Settings is open on Models and the worker kind is known and not Connect. The existing immediate refresh still runs. Restoration happens only after all events, end-of-batch handling and any `dispatch_next` refresh, and only if Settings is still open on Models and the complete in-memory `self.settings` still equals the snapshot. This covers the second queue-dispatch refresh found by the lead while leaving Connect, Converted, disconnected, Save, toolbar and navigation paths alone. No persistent state or runtime/queue behavior was added.

The current Copilot test supplied by the lead includes `CopilotConnected` followed by `Finished` and checks that the provider/form return to canonical Copilot and Medium; Connect is excluded from preservation. The tiled worker-completion test remains unchanged from the negative witness. No test was run here.

Pinned `cargo-fmt --check -p brn-desktop` and `git diff --check` passed. No build or tests were run, per assignment.

Limit: `refresh()` returns `()` and reports failures through UI notice state, so this change cannot distinguish a successful canonical reread from a refresh failure that leaves the previous `self.settings` snapshot in place. The equality guard means “the cached full settings value was unchanged after the refresh path”; it does not independently prove a fresh store read succeeded. This deliberately retains the requested narrow scope and should be considered during final review.
