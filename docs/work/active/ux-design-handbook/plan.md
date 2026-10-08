# UX/UI design handbook and current-app restyle

**Status 2026-10-07:** implemented and automated-verified on branch
`ux/design-handbook` (baseline `main@3d59cdd`). Not merged. Owner review and
acceptance pending; native interaction not yet verified by a human.

## Authorization and scope

Owner request (2026-10-07): establish a coherent UX/UI direction for the full
product vision, improve the current app toward it with **reversible UI-only
changes to existing functionality** on an isolated branch, create a visual HTML
handbook with agent-readable sources, and prototype important unbuilt
experiences. Excluded: pending P2 adoption decisions, backend/workflow changes,
presenting future functionality as working, merging, releases and live calls.

## Outcome

- [Design handbook](../../../design/README.md) (Markdown → mdBook): direction,
  product map/coverage, IA, generated tokens, components, interaction and trust
  patterns, journeys, current and future screens, decisions, agent checklist,
  verification record and owner preferences.
- Shared tokens and components in `brn-desktop`; the shell, rails, chat,
  proposal review, Dashboard, Inbox, Needs Review, Activity, notes/evidence,
  drafts and Settings restyled without behaviour or element-ID changes.
- Clickable [prototype](../../../design/src/prototypes/brn-prototype.html) for
  Today, chat provenance, Inbox review groups, Rewrite/approval/completion,
  Needs Review, projects, people, graph and chat lifecycle.
- Tooling: `scripts/design-demo-workspace.py`, `scripts/design-capture.sh`
  (dev-only `native-capture` feature), `scripts/design-handbook.py`,
  `scripts/design-prototype-check.js`, `scripts/tests/test_design_handbook.py`.

## Evidence

See the handbook's [verification record](../../../design/src/verification.md)
for commands, scopes and results, and its native validation handoff.

## Round 3 implementation (2026-10-08 evening)

Owner choices D24 (One queue), D25 (ChatGPT-style Settings) and D26 (margin
comments + optional Changes/Sources) were built after merging main `32c2538`:
`native/queue.rs`, sidebar and History changes in `simple.rs`/`approval.rs`, the
tabbed `shell/settings.rs`, margin and Changes in `review.rs`, and the word diff
in `src/text_diff.rs`. Sources marks are not built (no claim provenance). Changes
compare only versions displayed in the current session because the workflow
stores no earlier proposal text. Evidence: handbook verification, round 3.

## Next action

Owner reviews the handbook, the prototype and the running app (three tasks in
the session handoff), then records decisions in the handbook decision table.
Integration follows the normal workflow after acceptance or explicit deferral.
