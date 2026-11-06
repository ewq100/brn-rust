# Chat simplification (#249): acceptance

Checks CHAT-A1–A15 come from [#249](https://github.com/ewq100/brn-rust/issues/249) and the [UX spec](../../../product/ux-rebuild/BRN_UX_Rebuild_Spec.md#chat-simplification-additions-249). Every case starts **Not run**.

Status words:
- **Tested** means the automated evidence below passes at the stated commit.
- **Native** means a native Light/Dark observation.
- **Pending native** means the automated part passes but the native observation has not been recorded yet.

None of these checks is owner-accepted or merged.

| ID | Check | Status | Evidence |
|---|---|---|---|
| CHAT-A1 | Clean start | Not run | |
| CHAT-A2 | Automatic web | Pending native | `brn-ai` `allowed_web_search_reports_activity_records_sources_and_is_limited`; desktop `an_answer_lists_its_runs_web_sources` |
| CHAT-A3 | Web Off | Tested | `brn-ai` `without_allowed_web_access_the_tool_is_never_offered`, `a_helper_never_searches_the_web_even_when_its_chat_allows_it`; `brn-threads-app` `chat_web_access_is_per_chat_and_unavailable_routes_never_search`; desktop `allow_web_access_is_saved_per_chat_and_for_the_next_new_chat` |
| CHAT-A4 | Ownership and cancellation | Tested | `brn-ai` `a_stopped_run_discards_the_search_result`. Sources are written only to the run that searched and shown by that run's id, so a late result never reaches another chat or a newer run. |
| CHAT-A5 | Availability | Pending native | Desktop `web_access_states_an_unavailable_route_separately`; `brn-threads-app` `chat_web_access_is_per_chat_and_unavailable_routes_never_search` |
| CHAT-A6 | Automatic skills | Tested | `brn-ai` `saved_builtin_disable_or_stale_digest_cannot_withhold_automatic_source_preloads`; `brn-threads-app` `builtin_skills_stay_available_and_legacy_policy_is_reconciled_on_open` |
| CHAT-A7 | Existing workspaces | Tested | `brn-threads-app` `builtin_skills_stay_available_and_legacy_policy_is_reconciled_on_open`; desktop `native_legacy_builtin_binding_cannot_withhold_or_select_a_builtin` |
| CHAT-A8 | Message-specific command | Pending native | Desktop `the_slash_list_completes_a_command_into_only_the_current_message`; CLI `every_packaged_builtin_can_be_requested_for_one_run_and_is_actually_delivered`, `restored_requested_package_without_local_bytes_is_unavailable_before_provider_contact` |
| CHAT-A9 | Skill controls | Not run | |
| CHAT-A10 | Model route | Pending native | Desktop `model_favorites_keep_their_provider_and_survive_a_reopened_workspace`, `the_model_picker_searches_switches_models_and_adjusts_an_unsupported_effort`; core `model_favorites_are_lazy_bounded_ordered_and_survive_reopen` |
| CHAT-A11 | Reasoning | Pending native | `brn-ai` `subscription_catalogue_reports_each_models_supported_reasoning`; desktop `the_model_picker_searches_switches_models_and_adjusts_an_unsupported_effort`, `a_model_switch_keeps_only_a_supported_effort` |
| CHAT-A12 | Existing journeys | Pending native | Existing desktop, threads-app and CLI suites pass unchanged (desktop: 445 tests pass, 1 ignored) |
| CHAT-A13 | Keyboard | Not run | |
| CHAT-A14 | Native layout | Not run | |
| CHAT-A15 | Handoff | Not run | |

## Before

Existing native captures from the UX rebuild show the baseline:

- [New chat, Light, 1280×900](../ux-rebuild/evidence/final/light-new-chat.png): three starting buttons, a permanent attachment hint, and separate model and effort pills.
- [Conversation, Dark, 1280×900](../ux-rebuild/evidence/final/dark-conversation.png): the same composer row in an active chat.
