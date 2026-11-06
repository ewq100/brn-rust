# Chat simplification (#249): acceptance

Checks CHAT-A1–A15 come from [#249](https://github.com/ewq100/brn-rust/issues/249) and the [UX spec](../../../product/ux-rebuild/BRN_UX_Rebuild_Spec.md#chat-simplification-additions-249). Every case starts **Not run**.

Status words:
- **Tested** means the automated evidence below passes at the stated commit.
- **Native** means a native Light/Dark observation.
- **Pending native** means the automated part passes but the native observation has not been recorded yet.
- **Partial native** means part of the native observation is recorded and the rest is pending.

None of these checks is owner-accepted or merged.

| ID | Check | Status | Evidence |
|---|---|---|---|
| CHAT-A1 | Clean start | Partial native | Desktop `the_new_chat_start_has_a_greeting_the_composer_three_actions_and_the_delegation_choice` (no start-action row; the + menu holds attach, new document and web), `the_composer_is_one_row_and_names_the_effort_in_a_narrow_chat` (one combined model control); native run 1: new chat at Light 1280×900 shows one composer, one +, the context chip and one model control ([capture](evidence/run1-light-1280-new-chat.png)); existing chat and Dark pending |
| CHAT-A2 | Automatic web | Pending native | `brn-ai` `allowed_web_search_reports_activity_records_sources_and_is_limited`; desktop `an_answer_lists_its_runs_web_sources` |
| CHAT-A3 | Web Off | Tested | `brn-ai` `without_allowed_web_access_the_tool_is_never_offered`, `a_helper_never_searches_the_web_even_when_its_chat_allows_it`; `brn-threads-app` `chat_web_access_is_per_chat_and_unavailable_routes_never_search`; desktop `allow_web_access_is_saved_per_chat_and_for_the_next_new_chat` |
| CHAT-A4 | Ownership and cancellation | Tested | `brn-ai` `a_stopped_run_discards_the_search_result`. Sources are written only to the run that searched and shown by that run's id, so a late result never reaches another chat or a newer run. |
| CHAT-A5 | Availability | Pending native | Desktop `web_access_states_an_unavailable_route_separately`; `brn-threads-app` `chat_web_access_is_per_chat_and_unavailable_routes_never_search` |
| CHAT-A6 | Automatic skills | Tested | `brn-ai` `saved_builtin_disable_or_stale_digest_cannot_withhold_automatic_source_preloads`; `brn-threads-app` `builtin_skills_stay_available_and_legacy_policy_is_reconciled_on_open` |
| CHAT-A7 | Existing workspaces | Tested | `brn-threads-app` `builtin_skills_stay_available_and_legacy_policy_is_reconciled_on_open`; desktop `native_legacy_builtin_binding_cannot_withhold_or_select_a_builtin` |
| CHAT-A8 | Message-specific command | Partial native | Desktop `the_slash_list_completes_a_command_into_only_the_current_message`; CLI `every_packaged_builtin_can_be_requested_for_one_run_and_is_actually_delivered`, `restored_requested_package_without_local_bytes_is_unavailable_before_provider_contact`; native run 1: `/pre` lists the catalogue; Down, Up and Enter insert `/prepare-reply ` without sending ([capture](evidence/run1-light-1280-slash-menu.png)); two-message run pending |
| CHAT-A9 | Skill controls | Native (Light) | Desktop `native_skill_install_enable_select_disable_and_reopen` (built-ins offer Inspect only, no Disable), `native_legacy_builtin_binding_cannot_withhold_or_select_a_builtin`; the + menu rows in `the_new_chat_start_has_a_greeting_the_composer_three_actions_and_the_delegation_choice` contain no skill control; native run 1: no skill control in the composer or + menu; Settings > Skills offers Inspect only ([+ menu](evidence/run1-light-1280-plus-menu.png), [Skills](evidence/run1-light-1280-skills-settings.png)); Dark pending |
| CHAT-A10 | Model route | Partial native | Desktop `model_favorites_keep_their_provider_and_survive_a_reopened_workspace`, `the_model_picker_searches_switches_models_and_adjusts_an_unsupported_effort`; core `model_favorites_are_lazy_bounded_ordered_and_survive_reopen`; native run 1: search, star and Favorites keep `ChatGPT gpt-6.1-sol` ([picker](evidence/run1-light-1280-model-picker.png), [search](evidence/run1-light-1280-model-search.png), [Favorites](evidence/run1-light-1280-model-favorites.png)); restart and provider filter pending |
| CHAT-A11 | Reasoning | Pending native | `brn-ai` `subscription_catalogue_reports_each_models_supported_reasoning`; desktop `the_model_picker_searches_switches_models_and_adjusts_an_unsupported_effort`, `a_model_switch_keeps_only_a_supported_effort` |
| CHAT-A12 | Existing journeys | Pending native | Existing desktop, threads-app and CLI suites pass unchanged (desktop: 446 tests pass, 1 ignored, at `136c36b6`) |
| CHAT-A13 | Keyboard | Partial native | Desktop `the_slash_list_completes_a_command_into_only_the_current_message` (arrows, Enter, Escape, IME composition never completes), `the_model_picker_searches_switches_models_and_adjusts_an_unsupported_effort` (arrows, Enter, Escape), `shift_enter_adds_a_line_enter_queues_once_and_ime_composition_never_submits`; native run 1: slash arrows and Enter, + menu Escape, picker Down/Enter/Escape work; chat Enter and Shift-Enter pending |
| CHAT-A14 | Native layout | Fail, fixed, recheck pending |native run 1 (`adab7c93`) found D1: `/` descriptions ended mid-word without an ellipsis ([capture](evidence/run1-light-1280-slash-menu.png)). Fixed: the service now shortens a description at a whole word and adds an ellipsis (`brn-threads-app` `command_descriptions_end_at_a_word_with_an_ellipsis`). Other Light 1280×900 captures showed no clipping; Dark, 900×600, active conversation and web activity pending |
| CHAT-A15 | Handoff | Partial | Spec additions in the [UX spec](../../../product/ux-rebuild/BRN_UX_Rebuild_Spec.md#chat-simplification-additions-249); PR [#250](https://github.com/ewq100/brn-rust/pull/250) closes #249 and reports implemented, tested, native-observed, owner-accepted and merged separately. Native evidence pending |

## Native run 1 (`adab7c93`)

A Codex tester on macOS 26.5 built the release candidate, used a fresh synthetic data folder and made no paid chat runs. Its UI automation lost the window (`cgWindowNotFound`) after the model picker, so only Light 1280×900 states before the first message were observed. It found one defect, D1 (Minor): `/` descriptions ended mid-word, for example "…the owner's external d". It is fixed as recorded in CHAT-A14.

## Before

Existing native captures from the UX rebuild show the baseline:

- [New chat, Light, 1280×900](../ux-rebuild/evidence/final/light-new-chat.png): three starting buttons, a permanent attachment hint, and separate model and effort pills.
- [Conversation, Dark, 1280×900](../ux-rebuild/evidence/final/dark-conversation.png): the same composer row in an active chat.
