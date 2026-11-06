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
| CHAT-A2 | Automatic web | Pass (native observed) | `brn-ai` `allowed_web_search_reports_activity_records_sources_and_is_limited`, `activity_reports_apply_domain_outcomes_without_exposing_results`; desktop `an_answer_lists_its_runs_web_sources`. Native run 2 found no Web sources because the bundled Codex App Server had updated to `0.167.0-alpha.3.2`, which the adapter refused; Activity also mislabelled that refusal `success` (D2, fixed in recheck 5). The owner approved qualifying the new version ([qualification](#codex-app-server-0167-qualification)). Recheck 7 (`a67b4587`) sent one paid message: a cited answer with 24 run-owned Web sources and Activity `web_search · success`, in Light and Dark at both sizes ([answer](evidence/recheck7-light-a2-answer-1280x900.png), [Activity](evidence/recheck7-dark-a2-activity-success-900x600.png)) |
| CHAT-A3 | Web Off | Native | `brn-ai` `without_allowed_web_access_the_tool_is_never_offered`, `a_helper_never_searches_the_web_even_when_its_chat_allows_it`; `brn-threads-app` `chat_web_access_is_per_chat_and_unavailable_routes_never_search`; desktop `allow_web_access_is_saved_per_chat_and_for_the_next_new_chat`. Native run 2: an Off chat made no tool call and showed no Web sources; after quitting and reopening, Off and Allowed stayed with their chats ([capture](evidence/run2-light-1280-web-off-reopened.png)) |
| CHAT-A4 | Ownership and cancellation | Native | `brn-ai` `a_stopped_run_discards_the_search_result`. Native run 2: Stop during a live search, then switching chats, left the run Interrupted with no answer or sources; the other chat kept its single answer ([capture](evidence/run2-light-1280-stopped-search.png)). Upstream provider cancellation is not observable |
| CHAT-A5 | Availability | Partial native | Desktop `web_access_states_an_unavailable_route_separately`; `brn-threads-app` `chat_web_access_is_per_chat_and_unavailable_routes_never_search`. Native run 2: GitHub Copilot was not connected, so no unsupported route could be selected and the Unavailable row was not reached natively |
| CHAT-A6 | Automatic skills | Native | `brn-ai` `saved_builtin_disable_or_stale_digest_cannot_withhold_automatic_source_preloads`; `brn-threads-app` `builtin_skills_stay_available_and_legacy_policy_is_reconciled_on_open`. Native run 2: an ordinary "Draft a short reply…" loaded `prepare-reply` automatically, shown in Activity and saved on the run ([capture](evidence/run2-light-1280-automatic-skill-activity.png)) |
| CHAT-A7 | Existing workspaces | Tested | `brn-threads-app` `builtin_skills_stay_available_and_legacy_policy_is_reconciled_on_open`; desktop `native_legacy_builtin_binding_cannot_withhold_or_select_a_builtin` |
| CHAT-A8 | Message-specific command | Native | Desktop `the_slash_list_completes_a_command_into_only_the_current_message`; CLI `every_packaged_builtin_can_be_requested_for_one_run_and_is_actually_delivered`. Native runs 1–2: `/pre` lists the catalogue; arrows and Enter insert `/prepare-reply ` without sending; that run admitted `prepare-reply` and the next queued message ran with no loaded skill |
| CHAT-A9 | Skill controls | Native | Desktop `native_skill_install_enable_select_disable_and_reopen`, `native_legacy_builtin_binding_cannot_withhold_or_select_a_builtin`. Native run 1: no skill control in the composer or + menu; Settings > Skills offers Inspect only ([Skills](evidence/run1-light-1280-skills-settings.png)); native run 2: the + menu in Light and Dark holds no skill control ([Dark 1280](evidence/run2-dark-1280-plus-menu.png), [Light 900](evidence/run2-light-900-plus-menu.png)) |
| CHAT-A10 | Model route | Native | Desktop `model_favorites_keep_their_provider_and_survive_a_reopened_workspace`, `the_model_picker_searches_switches_models_and_adjusts_an_unsupported_effort`; core `model_favorites_are_lazy_bounded_ordered_and_survive_reopen`. Native run 2: Favorites kept ChatGPT gpt-6.1-sol across a restart; provider filters, search and arrow keys work ([Favorites](evidence/run2-light-1280-favorites-reopened.png), [Dark 900 picker](evidence/run2-dark-900-model-picker.png)) |
| CHAT-A11 | Reasoning | Partial native | `brn-ai` `subscription_catalogue_reports_each_models_supported_reasoning`; desktop `the_model_picker_searches_switches_models_and_adjusts_an_unsupported_effort`, `a_model_switch_keeps_only_a_supported_effort`. Native run 2: every connected model offered Low, Medium and High, so adjusting an unsupported effort was not reachable; earlier run labels kept their effort ([capture](evidence/run2-light-1280-reasoning-earlier-label.png)) |
| CHAT-A12 | Existing journeys | Partial native | Existing desktop, threads-app and CLI suites pass (desktop: 448 pass, 1 ignored). Native run 2: send, queue and resume, Stop and a Markdown attachment work. The first import timed out and the retry passed (not reproduced). Steer, queue editing and Apply/Discard/Undo were not exercised natively |
| CHAT-A13 | Keyboard | Native | Desktop `the_slash_list_completes_a_command_into_only_the_current_message`, `the_model_picker_searches_switches_models_and_adjusts_an_unsupported_effort`, `shift_enter_adds_a_line_enter_queues_once_and_ime_composition_never_submits`. Native run 2: Enter sends, Shift-Enter adds a line, Escape closes +, the slash list and the picker, and arrows with Enter work in both lists. IME is automated only |
| CHAT-A14 | Native layout | Pass (native observed) | Native run 2 found D1 and D3; recheck 5 (`546c9d7a`) passed both in Light and Dark at 1280×900 and 900×600 ([Light 900](evidence/recheck5-light-d1-slash-900x600.png), [Dark 900](evidence/recheck5-dark-d3-activity-scrolled-900x600.png)). D4, the `/` list before the delegation choice at 900×600, had two parts. Recheck 5 found the list sliding under the header; measured placement fixed it, confirmed in recheck 7. Recheck 7 found the delegation card painting over the list ([before](evidence/recheck7-light-d4a-slash-900x600-up.png)); `9870d389` draws the list above the chat (`the_slash_list_stays_in_the_chat_area_of_a_short_window`). Recheck 8 passed D4 in Light and Dark at both sizes: every row is readable over the card, and a click where they overlap chooses the command ([after](evidence/recheck8-light-d4a-slash-900x600-up.png)) |
| CHAT-A15 | Handoff | Pass (owner acceptance pending) | Spec additions in the [UX spec](../../../product/ux-rebuild/BRN_UX_Rebuild_Spec.md#chat-simplification-additions-249); PR [#250](https://github.com/ewq100/brn-rust/pull/250) closes #249 and reports implemented, tested, native-observed, owner-accepted and merged separately. Native Light and Dark captures are in the run and recheck sections below |

## Native run 1 (`adab7c93`)

A Codex tester on macOS 26.5 built the release candidate, used a fresh synthetic data folder and made no paid chat runs. Its UI automation lost the window (`cgWindowNotFound`) after the model picker, so only Light 1280×900 states before the first message were observed. It found one defect, D1 (Minor): `/` descriptions ended mid-word, for example "…the owner's external d". It is fixed as recorded in CHAT-A14.

## Native run 2 (`f44aaec5`)

A Codex tester on macOS 26.5 built the release candidate and used a fresh synthetic data folder, with six paid ChatGPT gpt-6.1-sol messages (one queued, one stopped during search). Light and Dark were captured at exactly 1280×900 and 900×600. It passed A1, A3, A4, A6, A8, A10 and A13 natively and found:

| # | Severity | Check | Finding | Fix |
|---|---|---|---|---|
| D1 | Minor | A14 | At 900×600 the row's single-line truncation cut descriptions mid-word ("the ow…") although the service text ended at a word | Rows wrap descriptions to two lines |
| D2 | Moderate | A2, CHAT-06 | Activity said `web_search · success` for a search the adapter refused | A refused or failed search is labelled `failed` (`activity_reports_apply_domain_outcomes_without_exposing_results`) |
| D3 | Moderate | A14 | With Activity open, its details painted over the attachment strip and, at 900, crowded out Inspect and Keep in BRN | The card scrolls its contents (`activity_details_stay_inside_the_card_above_attachments`) |

A2 was blocked: the bundled Codex App Server had updated to `0.167.0-alpha.3.2`, and the search adapter refused every version except the qualified `0.162.0-alpha.17.2`. On 6 November 2026 the owner approved qualifying the new version. See [App Server qualification](#codex-app-server-0167-qualification).

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

## Recheck 5 (`546c9d7a`)

A Codex tester on macOS 26.5 built the release candidate, used a fresh synthetic data folder and sent exactly one paid ChatGPT gpt-6.1-sol message with Allow web access checked. Light and Dark were captured at exactly 1280×900 and 900×600.

| Item | Result | Evidence |
|---|---|---|
| D1 `/` descriptions | Pass: two lines at most, every shortened description ends at a word | [Light 1280](evidence/recheck5-light-d1-slash-1280x900.png), [Dark 1280](evidence/recheck5-dark-d1-slash-1280x900.png), [Light 900](evidence/recheck5-light-d1-slash-900x600.png), [Dark 900](evidence/recheck5-dark-d1-slash-900x600.png) |
| D2 truthful web failure | Pass: `web_search · failed`, never `success`; no Web sources and an empty stored source list. Live search stays blocked (A2) | [Light 1280](evidence/recheck5-light-d2-web-failed-1280x900.png), [Dark 1280](evidence/recheck5-dark-d2-web-failed-1280x900.png), [Light 900](evidence/recheck5-light-d2-web-failed-900x600.png), [Dark 900](evidence/recheck5-dark-d2-web-failed-900x600.png) |
| D3 Activity bounds | Pass: details scroll inside the card; Inspect and Keep in BRN take real clicks below it | [Light 1280](evidence/recheck5-light-d3-activity-scrolled-1280x900.png), [Dark 1280](evidence/recheck5-dark-d3-activity-scrolled-1280x900.png), [Light 900](evidence/recheck5-light-d3-activity-scrolled-900x600.png), [Dark 900](evidence/recheck5-dark-d3-activity-scrolled-900x600.png) |
| Spot: one composer, + menu | Pass | [Light 900](evidence/recheck5-light-spot-plus-menu-900x600.png), [Dark 900](evidence/recheck5-dark-spot-plus-menu-900x600.png) |
| D4 (new, Minor) `/` list before the delegation choice at 900×600 | Fail: the intake row hid under the header. Fixed after the run; recheck 7 confirmed | [Light before](evidence/recheck5-light-d1-slash-pending-delegation-900x600.png), [Dark before](evidence/recheck5-dark-d1-slash-pending-delegation-900x600.png) |

Full report: local artifacts folder `brn-ux-rebuild-artifacts/chat-recheck-5/`.

## Recheck 7 (`a67b4587`)

A Codex tester on macOS 26.5 built the release candidate, used a fresh synthetic data folder and sent exactly one paid ChatGPT gpt-6.1-sol message with Allow web access checked. Light and Dark were captured at exactly 1280×900 and 900×600.

| Item | Result | Evidence |
|---|---|---|
| A2 live web search | Pass: "Searching the web…" while running; a cited answer with a Web sources section (24 run-owned sources across 8 domains); Activity `web_search · success` | [Searching](evidence/recheck7-light-a2-searching-1280x900.png), [Light 1280](evidence/recheck7-light-a2-answer-1280x900.png), [Dark 900](evidence/recheck7-dark-a2-answer-900x600.png), [Activity Light 1280](evidence/recheck7-light-a2-activity-success-1280x900.png), [Activity Dark 900](evidence/recheck7-dark-a2-activity-success-900x600.png) |
| D4a `/` list before the delegation choice | Fail at 900×600 in both themes: the list stayed inside the chat and clear of the header, but the delegation card painted over it, including the selected row. Pass at 1280×900. Fixed after the run (the list paints above the chat); recheck 8 passed | [Light](evidence/recheck7-light-d4a-slash-900x600-up.png), [Dark](evidence/recheck7-dark-d4a-slash-900x600-up.png) |
| D4b `/` list after Review changes first | Pass: `/` and `/pre` open above and fit; ↑ wraps to the last row and ↓ keeps the selection visible | [Light 900](evidence/recheck7-light-d4b-recheck-slash-900x600-up.png) |
| D4c docked composer | Pass | [Dark 900](evidence/recheck7-dark-d4c-slash-900x600-up.png) |
| Spot: descriptions, web toggle | Pass: two-line descriptions ending at a word; an unchecked Allow web access stays unchecked when + reopens | [Dark 900](evidence/recheck7-dark-regression-web-unchecked-reopened-900x600.png) |

Full report: local artifacts folder `brn-ux-rebuild-artifacts/chat-recheck-7/`.

## Recheck 8 (`9870d389`)

A Codex tester on macOS 26.5 built the release candidate, used a fresh synthetic data folder and made no paid calls or chat messages. Light and Dark were captured at exactly 1280×900 and 900×600.

| Item | Result | Evidence |
|---|---|---|
| D4a `/` list before the delegation choice | Pass in both themes at both sizes. At 900×600 the list opens below the composer, inside the chat and 65 pt above the window bottom, and paints over the delegation card. Every row's command and description is readable. ↑ selects the last row and ↓ keeps the selection visible. A click on `/prepare-reply` over the Review changes first button inserts the command without choosing a delegation option. Escape closes the list, and the card's buttons work again | [Light 900](evidence/recheck8-light-d4a-slash-900x600-up.png), [Dark 900](evidence/recheck8-dark-d4a-slash-900x600-up.png), [Click](evidence/recheck8-light-d4a-slash-900x600-click.png), [Escape](evidence/recheck8-dark-d4a-slash-900x600-escape.png), [Light 1280](evidence/recheck8-light-d4a-slash-1280x900-up.png) |
| D4b `/` list after Review changes first | Pass | [Dark 900](evidence/recheck8-dark-d4b-slash-900x600.png) |

Full report: local artifacts folder `brn-ux-rebuild-artifacts/chat-recheck-8/`.

## Before

Existing native captures from the UX rebuild show the baseline:

- [New chat, Light, 1280×900](../ux-rebuild/evidence/final/light-new-chat.png): three starting buttons, a permanent attachment hint, and separate model and effort pills.
- [Conversation, Dark, 1280×900](../ux-rebuild/evidence/final/dark-conversation.png): the same composer row in an active chat.

## Codex App Server 0.167 qualification

The owner approved this on 6 November 2026. The search adapter now accepts only `codex-cli 0.167.0-alpha.3.2`; the binary's SHA256 is `a636331ed87605efc4cf84028ac2884a75363f9d10ead63cffb57ea0016d486e`.

- **Protocol comparison.** I generated the JSON Schema from both the bundled binary and the published `@openai/codex@0.162.0-alpha.17.2-darwin-arm64`.
  - Every change is additive. No thread-item type or request method was removed, and no response the adapter reads changed at the top level.
  - Two server notifications are new: `thread/readState/changed` and `thread/section/changed`. They are accepted as inert only when they name BRN's own thread.
- **Zero-turn preflight.** It repeats the adapter's checks up to `thread/start` and sends no turn. All capability-boundary checks passed: standalone web search on, every other tool off, no plugins/MCP/skills/hooks, ChatGPT login, allowance available, `gpt-6.1-sol` offers medium, and an ephemeral read-only thread with no instruction sources.
- **Live query 1 failed closed.** It stopped on `hook/started`, because the owner's Codex configuration enables `sessionStart` hooks and the isolation controls never disabled hooks. This was a real isolation gap that predates the version change, so the adapter now:
  - sets `features.hooks=false` and verifies it in `config/read`;
  - requires `hooks/list` to report no enabled hook, errors or warnings before any turn;
  - treats any `hook/*` notification as unsafe tool use.

  Negative control: without the setting, `hooks/list` reported 3 enabled `sessionStart` hooks; with it, none. Test: `owner_and_managed_hooks_must_be_disabled_and_never_run`.
- **Live query 2 passed.** I asked one public question about the latest Rust release through the production adapter, with medium effort and no retries. It returned a cited answer and Web sources from `blog.rust-lang.org`.

This is a headless qualification of the adapter. Native observation of A2 in the desktop chat is recorded in [recheck 7](#recheck-7-a67b4587).

