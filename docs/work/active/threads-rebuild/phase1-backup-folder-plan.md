# Phase 1 remembered backup folder plan

**Goal:** The owner chooses a backup folder once; subsequent full backups create fresh files there, with a clear failure and Change folder action when it is unavailable.

**Authority/baseline:** The owner accepted a user-selected destination covered by a separate backup system, and explicitly authorized continued Phase 1 implementation and CI-gated integration without routine approval. Isolated `brn-phase1-backup-folder` / `work/phase1-backup-folder` starts at PR #119 merge `aca1c28c8fe79030a7fce5360143a4beeb1d0cb1`. GPT-6.1 Sol owns implementation; parent owns docs/integration. Native observation remains explicitly deferred; no provider, real user data or release.

**Evidence:** `threads_ui.rs::choose_output` prompts on every backup; `output` already chooses a fresh UUID destination and runs the atomic core backup on one worker. `settings_view` only shows session output status. `WorkspaceSettings` has no saved backup folder. Core `update_workspace_settings` already implements owner-only patches, stable intent/replay and current-policy preservation.

## Contract and design

- Add optional `WorkspaceSettings.backup_directory: Option<String>`, defaulted and omitted when absent. Preserve legacy JSON bytes/operation hashes and schema3; no dependency or SQL schema change. Use an absolute UTF-8 path with explicit refusal of unrepresentable paths, not lossy rewriting.
- Add a dedicated owner-only core Settings patch and app wrapper for this folder. It reads current Settings in the existing checked transaction, preserves all provider/account/auth/delegation fields, binds immutable intent, and retains existing Settings run fencing. Generic configure must preserve the current saved folder, including replay of a stale provider form. Model/tool writes cannot choose a folder.
- Validate a new selection as an existing directory, normalize it consistently, and reject the canonical workspace directory or its descendants, including symlink aliases. Keep filesystem validation out of ordinary workspace open so an unmounted backup drive does not prevent using BRN. Receipt replay must precede mutable filesystem checks. No directory creation or silent fallback on a missing backup destination.
- Native Settings displays the saved folder (or not chosen), Create full backup, and Change folder. First backup opens a folder picker; a successful first choice saves the folder then starts the backup. Change folder saves a new choice without starting a backup. Cancellation leaves preference/status unchanged; picker errors are visible. Recheck busy state after the async picker returns. Existing export behavior remains unchanged.
- Backup click reloads current saved preference, validates availability/safety, then uses the existing fresh UUID output and single Backup worker. Revalidate destination against the actual workspace before dispatch, not only at selection, so a saved folder restored into another workspace cannot write inside canonical storage. Missing/unwritable folders report a visible failure with Change folder available. Never redirect to the app data folder or reuse an existing backup file. Enforce the same outside-workspace rule in shared `Store::backup`, so explicit CLI/API paths cannot leave an unexpected backup file that prevents the next workspace open. Preserve relative backup paths and existing exclusive publication behavior. Do not probe write permissions by creating unrelated files; actual backup errors are the final authority.
- Explain briefly that the chosen folder should be covered by the owner's separate backup system; BRN does not claim to verify external backup coverage. Local path need not be added to the model's runtime prompt. Keep this a field, focused patch and UI reuse, not a new settings/backup framework.

## Verification and gates

- [x] Independent read-only GPT-6.1 Sol plan review approved. Bind stable caller intent before filesystem checks and include restored-into-saved-folder refusal.
- [x] Focused tests: missing-field legacy serialization/hash/replay compatibility; owner-only patch, immutable request/replay after destination disappears; preserved provider/delegation and stale configure; preference persistence/reopen/full backup+restore; direct core/CLI backup inside workspace refuses without changing storage; unavailable and in-workspace/alias paths refused.
- [x] Native tests exercise first choice, saved choice dispatch, Change without backup, picker cancellation/error and busy refusal; two backups use distinct names; failure remains visible without fallback; rendering shows destination and Change control. Use synthetic folders and empty credential paths, no OS picker interaction or provider calls.
- [x] Complete affected core/app/runtime/CLI compilation/tests as needed for Settings literals, native state tests, strict affected Clippy, formatting/docs/diff checks. Do not claim native observation from these checks.
- [x] Independent candidate review approved; replay intent finding fixed and focused checks rerun.
- [ ] Required PR CI; authorized merge; main verification and durable evidence on the PR.

## Remaining limits

Cold interruption/restart and native acceptance remain recovery qualification work. Existing injected failures and reopen witnesses are not physical power-loss qualification. Actual transactional upgrades/pre-upgrade orchestration accompany a real adopted-schema change; no artificial schema bump. External backup coverage is the owner's environmental choice.
