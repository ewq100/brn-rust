# Runtime/schema maintenance and separate Rig compatibility checklist

Read-only source inventory, 2026-10-23. Branch
`codex/runtime-schema-maintenance-20261023` stacks on local `125f8ef0`.
The maintenance targets are Tokio 1.53.3 and Schemars 1.2.3. Parent owns edits,
builds and qualification; this checklist does not report tests executed or passed.
Existing parent edits to manifests/lockfiles were preserved.

Rig remains pinned to **0.44.0** in `crates/brn-ai/Cargo.toml`, including
`rig`, `rig-core`, `rig-reqwest` and test-utils. **No target Rig version has been
chosen.** No Rig migration, external release research, provider account calls,
credentials, UI work or publication occurred in this inventory. Qualify the
maintenance independently before proposing a separately reviewed Rig change.

## Focused maintenance gates using existing tests

| Boundary | Existing source/test witnesses | Required interpretation |
|---|---|---|
| Tokio cancellation and stale completion | `crates/brn-ai/src/threads_steering_tests.rs`: `prepared_run_id_exists_before_worker_and_precancel_skips_credentials`, `stop_before_worker_skips_credentials_stale_tools_and_completion`, configuration/archive pre-worker cases | Stop/current-state fences must hold before credential or provider work, not merely return an error eventually |
| Async deadlines and conversion | `crates/brn-ai/src/document_refine_tests.rs`: `ai_off_never_opens_auth_for_either_engine_all_eight`, `refinement_deadline_stop_refusal_and_failure_have_no_save_or_fallback`, `oversized_current_prompt_refuses_before_credentials_or_transport` | Use existing synthetic transport fixtures; no live-model fallback or extra account attempt |
| In-flight MCP fencing | `crates/brn-ai/src/threads_mcp_runtime_tests.rs`: `actual_inflight_remote_work_may_finish_but_stop_or_revoke_fences_late_acceptance_and_writes`, repeated-call and revoked-grant tests | Local synthetic server/SDK execution is distinct from external-account testing; cancellation must fence acceptance even if remote work finishes |
| Process cancellation | `crates/brn-threads-intake/src/process.rs`: `cancellation_kills_owned_descendant_and_joins_pipes` | Kill/reap the owned process group and finish pipe joins; do not substitute timeout-only checks |
| Helper restrictions | `tools/xberg-intake-helper/tests/identity.rs`: `shared_runtime_denies_file_and_network`; `crates/brn-intake/src/helper_runtime{,_linux}.rs` | Both helper variants retain file/network denial and fail-closed platform policy; dependency churn must not weaken seccomp/sandbox restrictions |
| Actual helper formats and identities | `tools/xberg-intake-helper/tests/eight_kinds.rs`, `identity.rs`; `crates/brn-threads-intake/tests/document_modes.rs` | Build/qualify both existing stock and adapted lockfiles/helpers; all eight kinds and exact backend identity remain distinct |
| Schemars tool contract | `crates/brn-ai/src/proposal_tools.rs`: `action_tool_schema_is_equivalent_to_the_frozen_manual_contract`, `outer_protocol_is_closed_and_requires_all_three_explicit_fields`, `knowledge_protocol_is_closed_and_requires_every_candidate_and_quote_field` | Generated JSON schema must remain equivalent to frozen manual contract; changed snapshots alone are not approval |
| Nullable and bounded fields | `crates/brn-ai/src/action_candidates.rs` (`RequiredNullable`), `proposal_tools.rs`: `quote_tool_schemas_leave_nullable_occurrence_optional_and_conflict_ids_owned`, UTF-8/whole-JSON bounds tests | Required-nullable differs from omitted; retain closed objects, bounds, exact quote ownership and no truncation |

Useful parent-run focused commands (not executed by this helper):

```sh
cargo test --locked -p brn-ai threads_steering_tests
cargo test --locked -p brn-ai document_refine_tests
cargo test --locked -p brn-ai threads_mcp_runtime_tests
cargo test --locked -p brn-ai proposal_tools
cargo test --locked -p brn-threads-intake cancellation_kills_owned_descendant_and_joins_pipes
cargo test --locked -p brn-threads-intake --test document_modes
```

Helper test execution must follow the repository's existing separate helper
build/variant procedure; do not silently test only the stock lockfile. Parent
should record actual commands, exit status and artifact identities, then run the
relevant required aggregate gates. These are behavior witnesses, not Mac/native
acceptance.

## Future Rig migration: separate decision and compatibility gates

1. Choose a concrete target only after maintenance qualification and review
   official release/API/dependency changes. Record why the migration is needed;
   do not conflate a newer Rig release with this Tokio/Schemars patch update.
2. Reconcile the coordinated `rig`, `rig-core`, `rig-reqwest` and test-utils pins,
   TLS/features and `rmcp` integration. Preserve scripted-runs as test-only;
   production hosts must not enable mock/scripted providers.
3. Inspect `crates/brn-ai/src/visual.rs`, `tools.rs`, `proposal_tools.rs`,
   `threads_mcp_runtime.rs` and `document_refine.rs` for AgentBuilder,
   MultiTurnStreamItem, Tool/ToolContext, error-kind, finish-reason and typed
   generation-option changes. Preserve explicit invalid-tool failure behavior.
4. Run `threads_argument_correction_tests.rs`, especially mixed-batch rejection,
   invocation-total allowance, stop/round limits and Responses/Chat call/result
   pairing. A transport/framework retry must not become a second owner grant,
   duplicate accepted operation or implicit correction-budget reset.
5. Run existing `provider_formats_tests.rs`, `provider_stderr_tests.rs`,
   `provider_fixture_runtime.rs`, `threads_summary_tests.rs` and relevant
   `threads_writing_tests.rs` wire fixtures. Preserve Codex/Copilot routing,
   terminal/error handling and exact tool result relationships. Local scripted
   fixtures do not qualify real accounts.
6. Preserve context accounting with `threads_budget_tests.rs`: mandatory input,
   exact real tools/response reserve, oversized result refusal and growing
   request checks. Do not fix migration failures by truncating exact source,
   increasing budgets or broadening provider routes.
7. Rerun the maintenance schema/cancellation/helper gates above on the chosen
   Rig candidate, plus relevant aggregate tests and independent source review.
   Preserve raw result/trace privacy (`threads_mcp_runtime_tests.rs` includes
   reviewed-call trace/backup exclusions). Keep immutable operation identities,
   protected-note authority, queue identity and late-completion fences intact.
8. Report implementation, offline tests, native acceptance and live-provider
   qualification separately. No new live-model sessions or credential transfer
   are implied by this checklist. Publication/merge authority remains whatever
   the parent explicitly establishes for that future task.

Current limitation: compatibility with any newer Rig version is **unassessed**.
This is a concrete source/test inventory, not a migration recommendation or a
passing qualification receipt.
