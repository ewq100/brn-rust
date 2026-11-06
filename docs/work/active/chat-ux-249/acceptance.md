# Chat simplification (#249): acceptance

Checks CHAT-A1–A15 come from [#249](https://github.com/ewq100/brn-rust/issues/249) and the [UX spec](../../../product/ux-rebuild/BRN_UX_Rebuild_Spec.md#chat-simplification-additions-249). Every case starts **Not run**.

Status words:
- **Tested** means the automated evidence below passes at the stated commit.
- **Native** means a native Light/Dark observation.
- **Pending native** means the automated part passes but the native observation has not been recorded yet.
- **Partial native** means part of the native observation is recorded and the rest is pending or unreachable.
- **Blocked** means the check cannot pass until the stated owner decision.

None of these checks is owner-accepted or merged.

| ID | Check | Status | Evidence |
|---|---|---|---|
| CHAT-A1 | Clean start | Native | Desktop `the_new_chat_start_has_a_greeting_the_composer_three_actions_and_the_delegation_choice`, `the_composer_is_one_row_and_names_the_effort_in_a_narrow_chat`. Native run 2 (`f44aaec5`): new and active chats in Light and Dark at 1280×900 and 900×600 show one composer, one +, no starting-action row or attachment hint, one model · effort control and an accurate context chip ([Light 1280 new](evidence/run2-light-1280-new-chat.png), [Dark 900 new](evidence/run2-dark-900-new-chat.png), [Light 900 conversation](evidence/run2-light-900-conversation.png), [Dark 1280 conversation](evidence/run2-dark-1280-conversation.png)) |
| CHAT-A2 | Automatic web | Blocked | `brn-ai` `allowed_web_search_reports_activity_records_sources_and_is_limited`, `activity_reports_apply_domain_outcomes_without_exposing_results`; desktop `an_answer_lists_its_runs_web_sources`. Native run 2: Activity showed "Searching the web…" ([capture](evidence/run2-light-1280-web-searching.png)) but no Web sources, and the answer said live lookup was unavailable ([capture](evidence/run2-light-1280-web-answer.png)). Cause: the installed ChatGPT app updated its bundled Codex App Server to `0.167.0-alpha.3.2`; the search adapter runs only the qualified `0.162.0-alpha.17.2` and refused before sending anything. Activity also called that refusal `web_search · success` (D2), now `failed`. Blocked until the owner approves qualifying the new App Server version |
| CHAT-A3 | Web Off | Native | `brn-ai` `without_allowed_web_access_the_tool_is_never_offered`, `a_helper_never_searches_the_web_even_when_its_chat_allows_it`; `brn-threads-app` `chat_web_access_is_per_chat_and_unavailable_routes_never_search`; desktop `allow_web_access_is_saved_per_chat_and_for_the_next_new_chat`. Native run 2: an Off chat made no tool call and showed no Web sources; after quitting and reopening, Off and Allowed stayed with their chats ([capture](evidence/run2-light-1280-web-off-reopened.png)) |
| CHAT-A4 | Ownership and cancellation | Native | `brn-ai` `a_stopped_run_discards_the_search_result`. Native run 2: Stop during a live search, then switching chats, left the run Interrupted with no answer or sources; the other chat kept its single answer ([capture](evidence/run2-light-1280-stopped-search.png)). Upstream provider cancellation is not observable |
| CHAT-A5 | Availability | Partial native | Desktop `web_access_states_an_unavailable_route_separately`; `brn-threads-app` `chat_web_access_is_per_chat_and_unavailable_routes_never_search`. Native run 2: GitHub Copilot was not connected, so no unsupported route could be selected and the Unavailable row was not reached natively |
| CHAT-A6 | Automatic skills | Native | `brn-ai` `saved_builtin_disable_or_stale_digest_cannot_withhold_automatic_source_preloads`; `brn-threads-app` `builtin_skills_stay_available_and_legacy_policy_is_reconciled_on_open`. Native run 2: an ordinary "Draft a short reply…" loaded `prepare-reply` automatically, shown in Activity and saved on the run ([capture](evidence/run2-light-1280-automatic-skill-activity.png)) |
| CHAT-A7 | Existing workspaces | Tested | `brn-threads-app` `builtin_skills_stay_available_and_legacy_policy_is_reconciled_on_open`; desktop `native_legacy_builtin_binding_cannot_withhold_or_select_a_builtin` |
| CHAT-A8 | Message-specific command | Native | Desktop `the_slash_list_completes_a_command_into_only_the_current_message`; CLI `every_packaged_builtin_can_be_requested_for_one_run_and_is_actually_delivered`. Native runs 1–2: `/pre` lists the catalogue; arrows and Enter insert `/prepare-reply ` without sending; that run admitted `prepare-reply` and the next queued message ran with no loaded skill |
| CHAT-A9 | Skill controls | Native | Desktop `native_skill_install_enable_select_disable_and_reopen`, `native_legacy_builtin_binding_cannot_withhold_or_select_a_builtin`. Native run 1: no skill control in the composer or + menu; Settings > Skills offers Inspect only ([Skills](evidence/run1-light-1280-skills-settings.png)); native run 2: the + menu in Light and Dark holds no skill control ([Dark 1280](evidence/run2-dark-1280-plus-menu.png), [Light 900](evidence/run2-light-900-plus-menu.png)) |
| CHAT-A10 | Model route | Native | Desktop `model_favorites_keep_their_provider_and_survive_a_reopened_workspace`, `the_model_picker_searches_switches_models_and_adjusts_an_unsupported_effort`; core `model_favorites_are_lazy_bounded_ordered_and_survive_reopen`. Native run 2: Favorites kept ChatGPT gpt-6.1-sol across a restart; provider filters, search and arrow keys work ([Favorites](evidence/run2-light-1280-favorites-reopened.png), [Dark 900 picker](evidence/run2-dark-900-model-picker.png)) |
| CHAT-A11 | Reasoning | Partial native | `brn-ai` `subscription_catalogue_reports_each_models_supported_reasoning`; desktop `the_model_picker_searches_switches_models_and_adjusts_an_unsupported_effort`, `a_model_switch_keeps_only_a_supported_effort`. Native run 2: every connected model offered Low, Medium and High, so adjusting an unsupported effort was not reachable; earlier run labels kept their effort ([capture](evidence/run2-light-1280-reasoning-earlier-label.png)) |
| CHAT-A12 | Existing journeys | Partial native | Existing desktop, threads-app and CLI suites pass (desktop: 447 pass, 1 ignored). Native run 2: send, queue and resume, Stop and a Markdown attachment work. The first import timed out and the retry passed (not reproduced). Steer, queue editing and Apply/Discard/Undo were not exercised natively |
| CHAT-A13 | Keyboard | Native | Desktop `the_slash_list_completes_a_command_into_only_the_current_message`, `the_model_picker_searches_switches_models_and_adjusts_an_unsupported_effort`, `shift_enter_adds_a_line_enter_queues_once_and_ime_composition_never_submits`. Native run 2: Enter sends, Shift-Enter adds a line, Escape closes +, the slash list and the picker, and arrows with Enter work in both lists. IME is automated only |
| CHAT-A14 | Native layout | Fail, fixed, recheck pending | Native run 2 captured new chat, conversation, +, `/` and the model picker in Light and Dark at 1280×900 and 900×600. Two layout defects, fixed after the run: D1, at 900×600 the row cut `/` descriptions mid-word again ([before](evidence/run2-light-900-slash-recheck.png)); rows now wrap to two lines. D3, Activity's guidance and MCP details painted below the card over the attachment strip ([before](evidence/run2-dark-900-attachment-activity.png)); the card now scrolls (`activity_details_stay_inside_the_card_above_attachments`). D4, found in recheck 5's early captures of `546c9d7a`: the taller two-line `/` list slid under the header on the New chat start surface at 900×600; it now opens below the centred composer and keeps the selected row in view (`the_slash_list_stays_in_the_chat_area_of_a_short_window`) |
| CHAT-A15 | Handoff | Partial | Spec additions in the [UX spec](../../../product/ux-rebuild/BRN_UX_Rebuild_Spec.md#chat-simplification-additions-249); PR [#250](https://github.com/ewq100/brn-rust/pull/250) closes #249 and reports implemented, tested, native-observed, owner-accepted and merged separately. Native Light and Dark captures below; A2 is blocked and D1/D3 need a native recheck |

## Native run 1 (`adab7c93`)

A Codex tester on macOS 26.5 built the release candidate, used a fresh synthetic data folder and made no paid chat runs. Its UI automation lost the window (`cgWindowNotFound`) after the model picker, so only Light 1280×900 states before the first message were observed. It found one defect, D1 (Minor): `/` descriptions ended mid-word, for example "…the owner's external d". It is fixed as recorded in CHAT-A14.

## Native run 2 (`f44aaec5`)

A Codex tester on macOS 26.5 built the release candidate and used a fresh synthetic data folder, with six paid ChatGPT gpt-6.1-sol messages (one queued, one stopped during search). Light and Dark were captured at exactly 1280×900 and 900×600. It passed A1, A3, A4, A6, A8, A10 and A13 natively and found:

| # | Severity | Check | Finding | Fix |
|---|---|---|---|---|
| D1 | Minor | A14 | At 900×600 the row's single-line truncation cut descriptions mid-word ("the ow…") although the service text ended at a word | Rows wrap descriptions to two lines |
| D2 | Moderate | A2, CHAT-06 | Activity said `web_search · success` for a search the adapter refused | A refused or failed search is labelled `failed` (`activity_reports_apply_domain_outcomes_without_exposing_results`) |
| D3 | Moderate | A14 | With Activity open, its details painted over the attachment strip and, at 900, crowded out Inspect and Keep in BRN | The card scrolls its contents (`activity_details_stay_inside_the_card_above_attachments`) |

A2 is blocked, not fixed: the bundled Codex App Server is now `0.167.0-alpha.3.2`, and the search adapter refuses every version except the qualified `0.162.0-alpha.17.2`. Qualifying the new version (a zero-turn preflight and one live query) needs the owner's approval.

| State | Light | Dark |
|---|---|---|
| New chat 1280×900 | [capture](evidence/run2-light-1280-new-chat.png) | [capture](evidence/run2-dark-1280-new-chat.png) |
| New chat 900×600 | [capture](evidence/run2-light-900-new-chat.png) | [capture](evidence/run2-dark-900-new-chat.png) |
| Conversation 1280×900 | [capture](evidence/run2-light-1280-conversation.png) | [capture](evidence/run2-dark-1280-conversation.png) |
| Conversation 900×600 | [capture](evidence/run2-light-900-conversation.png) | [capture](evidence/run2-dark-900-conversation.png) |
| + menu 1280×900 | [capture](evidence/run2-light-1280-plus-menu.png) | [capture](evidence/run2-dark-1280-plus-menu.png) |
| + menu 900×600 | [capture](evidence/run2-light-900-plus-menu.png) | [capture](evidence/run2-dark-900-plus-menu.png) |
| `/` list 1280×900 | [capture](evidence/run2-light-1280-slash-recheck.png) | [capture](evidence/run2-dark-1280-slash-recheck.png) |
| `/` list 900×600 (D1, before fix) | [capture](evidence/run2-light-900-slash-recheck.png) | [capture](evidence/run2-dark-900-slash-recheck.png) |
| Model picker 1280×900 | [capture](evidence/run2-light-1280-model-picker.png) | [capture](evidence/run2-dark-1280-model-picker.png) |
| Model picker 900×600 | [capture](evidence/run2-light-900-model-picker.png) | [capture](evidence/run2-dark-900-model-picker.png) |
| Web answer 1280×900 (A2 blocked) | [capture](evidence/run2-light-1280-web-answer.png) | [capture](evidence/run2-dark-1280-web-answer.png) |
| Web answer 900×600 | [capture](evidence/run2-light-900-web-answer.png) | [capture](evidence/run2-dark-900-web-answer.png) |

Full report: local artifacts folder `brn-ux-rebuild-artifacts/chat-249-native-2/`.

## Before

Existing native captures from the UX rebuild show the baseline:

- [New chat, Light, 1280×900](../ux-rebuild/evidence/final/light-new-chat.png): three starting buttons, a permanent attachment hint, and separate model and effort pills.
- [Conversation, Dark, 1280×900](../ux-rebuild/evidence/final/dark-conversation.png): the same composer row in an active chat.
