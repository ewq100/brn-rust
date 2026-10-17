**B4 review, round 1: CHANGES_REQUIRED.** One actual defect blocks approval. It has a small, contained fix.

I reviewed `c5b257fb..c95b17a8` (source, tests, manifest, lock) and `deb310ae..50fa7ae3` (CI, script, docs) read-only. HEAD is `50fa7ae3`, the tree is clean, and the `crates` tree is `f2b22572` at both c95 and 50fa. I didn't run any build or test commands, and I didn't use the code-review skill because it hands work to sub-agents, which the brief forbids.

### Actual defects

**1. Medium: keyboard shortcuts and the editor's right-click menu do nothing, and the docs say they work.**
- `host.rs:549-556` binds cmd-s, cmd-/ and cmd-shift-enter to the `"WritingHost"` key context. Nothing in the example or the library ever sets that context (`git grep` at c95 finds no `key_context` or `on_action`), so these bindings never fire.
- The shared editor right-click menu (`writing.rs:530-541`) sends the `CommentSelection` and `AskAboutSelection` actions. The host has no handler for either, so "Comment on selection" and "Ask in chat" fail silently. Desktop handles both at `threads_shell.rs:1733-1741`.
- This breaks the brief's rules: no control that looks like it works but does nothing, and unsupported actions must say so. `verification.md:43` claims "Shared keyboard/context-menu action forwarding was completed in the host", which isn't true. The doc comment at `host.rs:549` is wrong too.
- **Fix:** in `render` (`host.rs:540`), add `.key_context("WritingHost")` and `on_action` handlers:
  - `Save` → `WritingRequest::Save`
  - `CommentSelection` → `WritingRequest::CommentSelection`
  - `Rewrite` → `WritingRequest::Rewrite`
  - `AskAboutSelection` → the existing "unavailable" notice

  Add one journey assertion, for example cmd-/ opening a comment anchor. Then correct `verification.md:43`.

**2. Low: strict lint never covered the test module, contrary to the evidence.**
- `verification.md:25` says strict Clippy over all targets "includes example/test harness". Examples default to `test = false`, so `--all-targets` checks the example only as a normal binary.
- The retained target confirms it. The only Clippy check artifact, `libwriting_host-7e29f6dc….rmeta`, has no `journey.rs` in its dependency list. Only the test binary `writing_host-dce738f6…` includes it.
- The CI lint step has the same gap.
- **Fix:** either add `test = true` to the `[[example]]` entry (principal-owned manifest), or add an example-test lint command and correct the claim.

### Test gaps (not blocking)
- **Apply refresh isn't checked on its own.** The Apply test never checks the editor value after Apply (`journey.rs:150-161`), only after Undo. Because both run the same `reload`, a missed refresh after Apply would still pass. Add `editor.value() == REVISED` after Apply.
- **First-edit ordering isn't proven by the test.** The first-dirty test asserts after `run_until_parked`, so it can't show that the save happens before render. The code order is right (`host.rs:49-62`).
- **Cleanup on a real GUI quit isn't covered.** On macOS a real quit may exit without running destructors, leaving the synthetic temp folder behind. Only the non-GUI cleanup is tested. It contains synthetic data only and the GUI was never launched, so treat this as a qualification.

### Preferences
- `latest_run` (`host.rs:166-177`) copies about 10 lines of desktop's private helper. The panel's callback interface requires it, and B4 couldn't change the library, so this is acceptable.
- Apply runs `writing_candidate_review` twice. It's redundant but harmless.

### Verified as correct
- **Real shared runtime:** Rewrite goes through `rewrite_run_start` → `prepare_thread_run` → `run_prepared_scripted`. The real tools produce the read ledger, the candidate and its evidence; the host never builds a candidate itself.
- **Apply:** each Apply rechecks the candidate freshly, and the per-render display check is never treated as permission.
- **Unsaved edits:** the host's reload drops the edit session, but this is safe. The core store refuses Apply/Undo while an edit is open on the note (`store.rs:1709-1720`), and Save closes the session before reload.
- **Edit tracking:** `note_changed` runs before any derived work.
- **Cleanup order:** fields drop the Workspace before the temp folder in both the fixture and the host.
- **Dependencies:** they are development-only. `scripted-runs` and `test-utils` were already enabled through desktop and app dev-dependencies, so feature merging doesn't change. Defaults and production dependencies are unchanged, and nothing depends on `brn-desktop`. The lock adds exactly the six `brn-writing-ui` dependency entries.
- **Fixture:** the reference goes through the real `import_full`, coverage stays Partial, and an ordinary Rewrite isn't counted as an exhaustive requirements check.
- **CI/script delta at `50fa7ae3`:** the exact command sits under the correct indentation and is macOS-only, like the existing writing-UI steps. Job names and the default Linux lane are untouched. The explicit step is needed because plain library tests don't run example tests.

Not covered by this approval: native human, GUI launch, live provider, the full F21 suite and INT-W.
