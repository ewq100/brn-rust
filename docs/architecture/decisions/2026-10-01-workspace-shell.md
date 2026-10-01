# Workspace shell UI decisions

Date: 1 October 2026. Status: accepted for slice 1 by the user during brainstorming on 1 October 2026 ([spec](../../superpowers/specs/2026-10-01-workspace-shell-design.md)).

## Context

The BRN UI/UX handoff selects a combined workspace: history, document and chat, and vault, in a refined-terminal style. The desktop had a single-column page switcher. GPUI Kit 0.6.6 remains provisional under the [architecture baseline](2026-09-28-architecture-baseline.md).

## Decisions

- Build the shell on GPUI Kit 0.6.6. This does not end GPUI's provisional status or the deferred native editor acceptance.
- Restructure `brn-desktop` in place. `Desktop` remains the single owner of workflow state; regions are render-only `impl Desktop` modules. Layout rules and tokens are GPUI-free and unit-tested.
- Persist layout and appearance in a versioned `layout.json` in the explicit data directory. It is presentation state only, not authoritative application data, and not credentials or content. Corrupt or unknown files fall back to defaults.
- Follow macOS system appearance by default, with Dark and Light overrides using the handoff tokens.
- Use a transparent titlebar that carries the BRN header.
- Hide handoff features without a backend and track them in the [UI feature backlog](../../ui/feature-backlog.md).

## Alternatives rejected

- One GPUI entity per region: rewrites state ownership and risks dirty-state and stale-response guards.
- A parallel new desktop binary: doubles maintenance.
- Fixture-backed previews of unbacked features: the handoff rejects fixtures that pose as connected behaviour.

## Consequences

SQLite authority is unchanged; `layout.json` can be deleted without data loss. VoiceOver qualification remains open. Later UI slices add regions or controls inside this shell.
