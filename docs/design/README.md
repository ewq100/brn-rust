# BRN design handbook

Canonical UX/UI source for BRN. **Status: working design baseline pending owner
review** — nothing here is owner-accepted unless the
[decision record](src/decisions.md) says so.

The Markdown in `src/` is the only written source. The browsable HTML is
generated from it; do not maintain a separate prose copy.

## Read only what your task needs

| You are… | Read |
| --- | --- |
| Changing any desktop UI | [Agent guide and review checklist](src/agents.md), then the rows below |
| Adding or restyling a control, list, header, badge or message | [Components](src/components.md), [foundations](src/foundations.md) |
| Adding a view, rail item or navigation route | [Information architecture](src/information-architecture.md) |
| Touching proposals, approval, Sources, drafts, Actions or provenance | [BRN trust patterns](src/brn-patterns.md) |
| Handling loading, errors, cancellation, recovery, keyboard or resizing | [Interaction patterns](src/interaction.md) |
| Building a planned capability (Today, projects, people, graph, sessions…) | [Product map](src/product-map.md), [future designs](src/screens-future.md), [prototype](src/prototypes.md) |
| Checking why something looks the way it does | [Design decisions](src/decisions.md) |

Design values live in [`crates/brn-desktop/src/tokens.rs`](../../crates/brn-desktop/src/tokens.rs);
shared view components live in [`crates/brn-desktop/src/native/ui.rs`](../../crates/brn-desktop/src/native/ui.rs).

## Build and open

```sh
# once, without changing global tools:
cargo install mdbook --version 0.5.4 --locked --root target/design-tools
python3 scripts/design-handbook.py open    # regenerate tokens, build, open
python3 scripts/design-handbook.py check   # fail if generated token files are stale
```

The book builds to `target/design-handbook/index.html`. The clickable
prototype is `src/prototypes/brn-prototype.html` and opens directly in a browser.

Refresh implemented-app screenshots from the real views (headless, synthetic data):

```sh
scripts/design-capture.sh target/design-captures
```
