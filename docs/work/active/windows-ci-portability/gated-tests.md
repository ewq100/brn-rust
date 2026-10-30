# Exact newly platform-gated tests

Audit of main `89e6221a` versus this local candidate. Test bodies and assertions remain intact on supported platforms. These are capability boundaries, not Windows success evidence. No workflow job was removed.

## crates/brn-ai/src/auth.rs

33 tests exercise Unix ownership/mode/link/no-follow credential operations or protected-file authentication. The two pure route/selection tests now remain all-platform. Unix metadata-policy checks test Unix primitives, not portable API behavior.

Newly gated tests: 33.

- `unsafe_folder_modes_and_symlinks_are_rejected`
- `cache_symlinks_hardlinks_owner_and_modes_are_rejected`
- `status_is_offline_and_identity_is_display_only`
- `missing_expired_and_malformed_caches_never_start_device_login`
- `disconnect_deletes_only_target_provider_and_owned_client_releases_lock`
- `stale_copilot_cache_does_not_block_chatgpt_or_explicit_disconnect`
- `disconnect_unlinks_owned_symlinks_and_hardlinks_without_touching_targets`
- `disconnect_refuses_directories_and_unsafe_folders_before_deleting`
- `disconnect_metadata_policy_refuses_foreign_entries_and_non_files`
- `real_copilot_connect_caches_name_and_tightens_all_files`
- `login_error_tightens_partial_cache_and_does_not_retain_http_body`
- `code_expiry_has_retry_copy_without_automatic_retry`
- `copilot_discovery_failure_is_not_fallback`
- `cancellation_tightens_partial_login_and_disconnect_cannot_recreate_it`
- `other_provider_login_and_status_progress_while_chatgpt_login_waits`
- `models_network_wait_does_not_hold_auth_lock_and_cancels`
- `subscription_models_require_auth_and_handle_network_and_precancellation_safely`
- `refresh_is_serialized_and_disconnect_waits_for_its_finalization`
- `unsafe_replacement_during_login_never_reports_success`
- `empty_token_and_invalid_expiry_are_reconnect_not_success_or_panic`
- `empty_oauth_reply_does_not_create_a_success_shaped_client`
- `real_chatgpt_connect_and_refresh_tighten_caches_without_default_printing`
- `dropping_login_future_also_tightens_partial_cache`
- `cancelled_lock_waiter_never_touches_cache_or_transport`
- `missing_identity_is_explicit_and_permission_drift_is_refused`
- `copilot_discovery_returns_only_the_actual_authenticated_list`
- `copilot_catalog_ids_and_duplicates_are_validated_atomically`
- `in_repository_and_symlinked_ancestor_paths_are_rejected`
- `malformed_display_metadata_is_storage_not_expired_login`
- `malformed_github_token_is_reconnect_not_connected`
- `token_store_roundtrip_preserves_legacy_files_without_combined_records`
- `token_store_saves_partial_github_before_session_and_deletes_invalidated_bootstrap`
- `unsafe_copilot_destination_prevents_all_bootstrap_writes`

## crates/brn-ai/src/codex_auth.rs

Three tests read protected selected-account files or exercise Unix symlink/file-mode checks. A new portable invalid-provider/relative-path witness keeps the common pre-authentication contract enabled; non-Unix explicit-refusal coverage is separate.

Newly gated tests: 3.

- `malformed_and_api_key_files_refuse_without_fallback_or_secret_errors`
- `unsafe_path_or_provider_is_refused_before_authentication`
- `selected_access_token_stays_read_only_and_uses_no_transport`

## crates/brn-ai/src/provider_formats_tests.rs

Four subscription-catalog tests authenticate through an actual protected credential cache before mocked HTTP. All other provider-format tests retain the in-memory HTTP fixture on Windows.

Newly gated tests: 4.

- `subscription_catalog_uses_protocol_compatibility_instead_of_brn_package_version`
- `authenticated_subscription_catalog_preserves_wire_identity_and_server_order`
- `malformed_subscription_catalog_is_an_atomic_safe_failure`
- `subscription_catalog_statuses_are_safe_without_fallback_or_retry`

## crates/brn-ai/src/threads_budget_tests.rs

Two end-to-end provider-error tests resolve a selected account from a protected file. Pure request-budget, local-estimate, typed-error and mock-runner tests remain portable.

Newly gated tests: 2.

- `provider_context_exceeded_and_local_limit_share_display_attention_and_terminal_path`
- `sanitized_provider_cannot_spoof_a_local_brn_estimate_notice`

## crates/brn-ai/src/threads_cache_tests.rs

Seven production transport/session/retry tests open protected account files (directly or through production_run/scripted_run). Pure cache-key/body/session policy tests remain portable.

Newly gated tests: 7.

- `codex_runs_pin_one_session_and_an_append_only_prefix_on_either_credential_route`
- `different_chats_get_different_session_ids`
- `trace_records_each_turns_cached_input`
- `a_transient_status_mid_run_is_resent_identically_once_and_traced`
- `a_repeated_transient_failure_ends_the_run_with_its_safe_detail_in_the_trace`
- `a_failed_turn_after_the_reply_started_is_not_resent`
- `re_sends_are_reported_without_a_trace`

## crates/brn-ai/src/threads_research_tests.rs

Six tests construct protected Codex/Copilot account files or call codex_helper_scope before mocked helper HTTP. Mock helper budgets, dispatch, scope and cancellation remain portable; a new non-Unix test asserts unavailable outcomes with no HTTP and unchanged files.

Newly gated tests: 6.

- `helper_provider_context_limit_is_budget_with_safe_diagnostics_and_no_fallback`
- `codex_helpers_use_their_own_cache_key_and_only_the_owner_model`
- `copilot_helpers_route_through_the_copilot_client`
- `helper_resends_are_recorded_in_the_helper_trace`
- `a_helper_provider_outage_is_unavailable_with_status_but_no_provider_text`
- `copilot_helper_provider_overflow_is_budget_without_unavailability_or_fallback`

## crates/brn-ai/src/threads_summary_tests.rs

Sixteen additional tests resolve protected-file identity/authentication before mocked compaction/chat (including four newly excluded tests in review1). Three other review1 tests already had Unix gates before this patch. Four in-memory/pre-auth summary tests remain portable, including the guide-bundle publication test.

Newly gated tests: 16.

- `model_backed_summary_precedes_chat_without_chat_cache_authority`
- `semantic_coverage_is_derived_without_canonical_summary_message`
- `cold_reopen_reuses_valid_coverage_then_corruption_and_loss_recompute`
- `malformed_or_invented_summary_has_visible_gap_and_keeps_mandatory_request`
- `stop_and_concurrent_configuration_change_never_publish_inflight_summary`
- `timeout_and_busy_cache_keep_valid_prior_with_gap_without_replacing_it`
- `incremental_summary_carries_prior_and_freezes_append_only_chat_prefix`
- `project_move_knowledge_off_delete_restore_and_source_versions_invalidate_cache`
- `derived_instructions_cannot_become_system_authority_or_note_read_evidence`
- `applied_receipts_get_host_stamps_but_proposals_cannot_be_summarized_as_applied`
- `canonical_generation_change_rejects_publication_even_when_run_and_sources_stay_live`
- `resolved_account_change_invalidates_without_a_settings_or_history_change`
- `access_token_rotation_keeps_same_chatgpt_account_coverage`
- `complete_validated_cache_ignores_busy_writer_lease`
- `well_formed_cache_digest_corruption_recomputes_with_other_stamps_unchanged`
- `unrelated_project_members_and_current_sources_do_not_invalidate`

## crates/brn-retrieval/src/native/download_tests.rs

Eight unit tests require actual filesystem installation, exclusive atomic publication, private staging or Unix inode/device identity checks. These are existing unsupported operations outside Linux/macOS; digest/pin/manifest tests stay portable. New unsupported tests verify no fetch/progress/writes and cancellation precedence.

Newly gated tests: 8.

- `synthetic_five_assets_install_exclusively_and_verified_reuse_has_no_fetch`
- `synthetic_corruption_truncation_oversize_http_error_and_cancel_clean_only_owned_stage`
- `concurrent_destination_creation_cannot_be_overwritten_by_install`
- `concurrently_created_empty_destination_is_preserved_and_stage_is_removed`
- `competing_installers_publish_one_complete_model_and_remove_losing_stage`
- `private_http_seam_uses_synthetic_manifest_and_local_server_only`
- `http_failure_has_safe_typed_error_and_cleans_the_exclusive_stage`
- `cancellation_between_chunks_cleans_a_partial_file_without_installing`

## crates/brn-retrieval/tests/model_download.rs

One occupied-target test expects the supported installer to inspect and classify assets. Unsupported platforms now fail earlier with ExclusiveInstallUnavailable; a separate new test preserves existing and absent destinations.

Newly gated tests: 1.

- `occupied_target_is_refused_without_network_or_overwrite`

## crates/brn-threads-intake/src/transient.rs

Two scratch-success/budget tests require a private Unix scratch directory. The existing symlink test was already Unix-only. A new non-Unix test verifies refusal before root creation or cleanup.

Newly gated tests: 2.

- `missing_scratch_root_and_excess_entries_still_fail`
- `concurrent_cache_renames_do_not_fail_scratch_budget_monitoring`

