# Indexed workspace Settings read candidate

**Candidate:** `eae61dde04cda19c764246993966b13b997014eb` (`Read workspace settings through indexed store query`)

**Branch/base:** `codex/indexed-settings-read`, created from exact main209 `5fab84d87f2cff3bc03f40fa5987c1fe73be98ab`. The frozen `codex/note-reload-generation-guard` branch was left unchanged. Worktree is clean at the candidate commit.

**Changed paths:**

- `crates/brn-threads-core/src/store.rs`: added the read-only public `Store::workspace_settings() -> Result<Option<WorkspaceSettings>>`, delegating directly to the existing private `workspace_settings_on(&self.conn)` helper. No SQL was copied or changed.
- `crates/brn-threads-app/src/settings.rs`: `Workspace::settings()` now calls that Store method and retains the exact existing missing-setting error, `Workspace configuration is missing`.

This removes the previous full `Workspace::records()` query and deserialization of every record just to find Settings. It reuses the canonical helper already used by core policy/fence paths. The schema's existing unique partial `one_workspace_settings` index ensures there cannot be two Settings records; the helper filters for the non-archived Settings row. Missing/archived Settings still returns the same application error. The read now deserializes only the selected Settings row, so an unrelated malformed record no longer causes this particular Settings lookup to fail; subsequent full-record reads still surface that issue. No schema, write policy, version, receipt, authority, privacy, Run/Undo, Topics, or caching behavior changed.

No tests were added because this is direct reuse of an existing helper rather than new behavior. Lead-owned checks should include the existing Settings/configuration/fence coverage, including `thread_lifecycle::legacy_settings_without_theme_keep_their_serialized_form`, `thread_lifecycle::theme_waits_for_working_runs_and_never_stops_them`, `product::configuration_and_pause_fence_runs_while_thread_resolution_preserves_actions`, `killed_run_candidate::the_settings_change_also_stops_a_run_that_is_still_live`, and `document_conversion_settings::selected_identity_mismatch_and_checked_settings_replay` as applicable. No tests, builds, or workloads were run by this author.

Validation performed: `rustup run 1.98.1 rustfmt --edition 2024 crates/brn-threads-core/src/store.rs crates/brn-threads-app/src/settings.rs` and `git diff --check` passed. Full source patch: `/tmp/brn-indexed-settings-read.patch` (SHA-256 `824c02a76e996350fe78faf12c945d130f5f6f297b33bb13de8137f35e34e53f`). Candidate is local only; it was not pushed or merged.
