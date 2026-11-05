# BRN UX Rebuild Handoff

**Version 1.1 · 5 November 2026 · Owner Evo**

The selected direction is simple and clean: a ChatGPT-inspired native shell, one main workspace, a spacious document canvas and an article-based Library. Keep GPUI Kit and upgrade it to **0.9.0** as an early part of the rebuild. Light and Dark are equal parts of the design, with a System preference.

## Give this package to the build agent

Read [BRN_UX_Rebuild_Spec.md](BRN_UX_Rebuild_Spec.md) first. It is the main UX specification and includes the current capability map, behavior contracts, theme rules, code boundaries, delivery sequence and 40 acceptance cases. The package is already in `docs/product/ux-rebuild/`. When the owner assigns implementation, connect that task's work record to this location.

| File | Purpose |
|---|---|
| [BRN_UX_Rebuild_Spec.md](BRN_UX_Rebuild_Spec.md) | Complete implementation and acceptance specification |
| [BRN_Design_Reference.pdf](BRN_Design_Reference.pdf) | Drawn light and dark compositions and design-system reference |
| [BRN_Interactive_Reference.html](BRN_Interactive_Reference.html) | Offline interactive reference with System, Light and Dark preview controls |
| [BRN_Design_Tokens.json](BRN_Design_Tokens.json) | Semantic color pairs, spacing, type and layout targets |
| [BRN_Wiki_Library_Spec.md](BRN_Wiki_Library_Spec.md) | Related article consolidation and evidence requirements, included for repository access |
| [BUILD_AGENT_PROMPT.txt](BUILD_AGENT_PROMPT.txt) | Starting prompt for the receiving build agent |
| [THIRD_PARTY_NOTICES.txt](THIRD_PARTY_NOTICES.txt) | License notice for the reference's bundled icons |

## Explore the design

Open the HTML reference in a modern browser. Use the appearance choice above the app to inspect Dark. Open “Your knowledge library” from the conversation, inspect the document review, try Apply and Undo, then visit Library and New chat. The setting above the app is a preview control; production appearance belongs in Settings.

The HTML uses sample data and temporary in-memory actions. It does not call an AI provider, read BRN data, import files or save changes to BRN. The PDF contains manually drawn vector design plates, not native screenshots. The specification is the authority for real behavior and any edge case the reference does not implement.

## Important implementation decisions

- Rebuild the complete presentation and navigation, while retaining supported user capabilities.
- Keep the existing shared services, editor, scope rules and recovery mechanisms unless a demonstrated gap requires a bounded change.
- Use “Saved library” or “Current knowledge.” Saved content can include valid delegated ordinary maintenance; it does not always mean personally reviewed.
- Preserve review-required/protected writing changes and exact Apply/Discard/Undo behavior.
- Keep current manual inbox Scan behavior. A new background watcher is not included.
- Upgrade the active desktop/writing consumers to GPUI Kit `=0.9.0`, migrate the relevant APIs and update the lockfile coherently. The inspected 0.8.0 pin is historical context. The owner explicitly required this upgrade; use spec §12.1 and verify native behavior after migration.

## Verification status of this handoff

The reference direction was approved by Evo on 5 November 2026. A read-only repository inventory grounded the preservation requirements at `e749a83e207c5b6d3c081d9f7d1fa6d6efed088c`. The written spec received independent source review. The PDF was rendered and inspected, and the HTML/JSON were checked structurally. Browser rendering of the interactive reference and native GPUI behavior remain unverified in the authoring session because preview access was unavailable.

This is a design/build handoff. It is not a claim of implementation, native acceptance, a Council score, a UI implementation merge or release. The receiving task and current repository instructions determine execution authority. Reuse the approved direction; do not reopen unrelated style exploration.
