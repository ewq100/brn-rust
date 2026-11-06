# Chat simplification (#249): plan

Assigned by the owner through [#249](https://github.com/ewq100/brn-rust/issues/249) on 6 November 2026. Design authority: [UX spec §6.1.1](../../../product/ux-rebuild/BRN_UX_Rebuild_Spec.md#611-composer-tools-web-access-skills-and-model-choice) (revision 1.3) and the dated amendment in the [behavior contract](../../../architecture/threads-behavior.md). Baseline: the UX rebuild branch `work/ux-rebuild-20261105` (PR #248).

## Ownership

- Integration branch `work/chat-ux-249`, stacked on PR #248. One lead owns the spec, model capabilities, every desktop change, integration, tests and evidence.
- Runtime slices were first given to helpers on their own branches in isolated checkouts; both helpers failed, so the lead finished them on the integration branch to the same contracts:
  - `work/chat-ux-249-skills`: built-ins always enabled; reconciliation of old disabled bindings and persistent selections; per-run `requested_skills`; `skill_commands` catalogue for the `/` menu.
  - `work/chat-ux-249-web`: the `web_search` agent tool behind central per-run web access; per-chat Allow web access state; run-owned sources and cancellation; Copilot and missing account routes report Unavailable.
- Synthetic data and fake searchers only in automated checks. Native checks use a fresh data directory.

## Slices

| Slice | Scope | Owner |
|---|---|---|
| Spec | §4.3, §6.1, §6.1.1 (CHAT-07–09), §6.10, §6.11, AC-37, CHAT-A1–A15; behavior-contract amendment | Lead |
| Models | Catalogue reasoning levels and display names; Codex account-file catalogue; Favorites as presentation state | Lead |
| Composer | Remove the start-action row and permanent hint; + menu without skills or the web dialog; combined picker; `/` menu; web toggle; inline web activity and sources | Lead |
| Skills runtime | As above | Lead (helper attempt failed; its partial commit was adopted and finished) |
| Web runtime | As above | Lead (helper attempt failed) |
| Qualification | Focused tests, repository checks, native Light/Dark captures at 1280×900 and 900×600 | Lead + native tester |

## Records

- [Acceptance results](acceptance.md)

Status words follow the repository rule: implemented, verified, native-observed, owner-accepted and merged are reported separately.
