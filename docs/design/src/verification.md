# Verification and acceptance record

Baseline: `main` at `3d59cdd` (PR85). Branch: `ux/design-handbook`. Environment:
Apple Silicon macOS, Rust 1.98.1, locked/offline dependencies, gpui-kit 0.6.6.
Each line names what was checked and how; none of it is owner acceptance.

## Implemented app

| Check | Scope | Result |
| --- | --- | --- |
| `cargo test -p brn-desktop --features native-ui,native-retrieval,native-test-support` | Unit, state and native widget tests (synthetic interaction via the toolkit test context) | 301 passed + 7 CLI tests passed (baseline 299 + 7; new `utc_time` and right-click comment tests) |
| `cargo clippy -D warnings` | brn-desktop with default, `native-ui`, `native-ui,native-retrieval,native-test-support`, `native-capture` | Clean (upstream `block` future-incompatibility note only) |
| `cargo fmt --check` | Workspace | Clean |
| `bash scripts/verify-desktop-shell.sh --native` | Repository desktop gate: fmt, workspace build/clippy/tests, native builds, clippy for every native feature set, native tests, headless AppWorker startup | Passed (exit 0) after review fixes |
| Headless real-view captures (`scripts/design-capture.sh`) | Production view tree, real AppWorker over a seeded synthetic workspace, real macOS text shaping, headless Metal; dark, light, 720 pt narrow | 12 screens rendered and inspected by the agent across 3 rounds; issues fixed (icons outside embedded set, clipped text in narrow panes, decision-bar wrapping, note titles, settings height) |
| Navigation via real click routing | `open-dashboard`, `open-inbox`, `open-findings`, `open-activity` clicked in the capture tour | Surfaces opened as expected |
| Native window, keyboard, IME, VoiceOver, resizing by hand | Running `.app` on an unlocked Mac | **Not verified** — Screen Recording/Accessibility permission was unavailable to the agent; see the handoff below |

### Native validation handoff

```sh
git -C /path/to/brn-rust worktree list   # find the ux/design-handbook checkout
cd <that checkout>
python3 scripts/design-demo-workspace.py --root /tmp/brn-ux-review
cargo run -p brn-desktop --features native-ui --locked --offline -- \
  --data-dir /tmp/brn-ux-review/data --vault /tmp/brn-ux-review/vault
```

Check: navigator counts appear after opening Dashboard/Inbox; ⌘N, ⌘L, ⌘,, ⌘0,
⌥⌘0, ⇧⌘↩; resize to 480 × 480 (rails collapse, tabs appear); open a note, type,
navigate away (recovery guard still holds); open the paint proposal, scroll, the
decision bar stays visible; Light appearance in Settings; Esc closes Settings.

### Independent review

One read-only defect-first review of the code diff (`3d59cdd..HEAD`, crates and
capture tooling) by a separate reviewer agent. Findings and dispositions:

| Finding | Disposition |
| --- | --- |
| Welcome example buttons used `set_value`, which emits no change event, so a pending search could show stale results and typed text could be overwritten | Fixed: examples only fill an empty composer and invalidate pending search like typing |
| `home_relative` matched the home prefix without a path boundary | Fixed: uses `Path::strip_prefix` |
| Composer Stop shown only while active; Choose vault only before binding; group approval only for grouped proposals; Decided list collapsed | Reviewed as acceptable: same capabilities and unchanged disabled conditions |
| Decision bar, Inbox split, navigator counts, `utc_time`, theme keys, capture harness, seed script | Reviewed; no defects |

## Prototype

| Check | Scope | Result |
| --- | --- | --- |
| `scripts/design-prototype-check.js` (Playwright, headless Chromium) | All routes; journeys A (draft → Rewrite → approve → complete), Inbox group approval and cleanup gating, supersession; empty/loading/error/offline states; light theme; keyboard focus ring; ⌘N; 900 pt viewport; console errors | 31/31 checks passed in 3 consecutive runs; no console errors. Round 3 added the directions page (12 options, 7 interactions): 50/50 in 2 runs |

## Handbook

| Check | Result |
| --- | --- |
| `python3 scripts/design-handbook.py build` (mdBook 0.5.4) | Builds without warnings |
| `python3 scripts/check-markdown-links.py` | Passes |
| `python3 -m unittest discover -s scripts/tests` | Passes, including token drift and contrast tests |

### Owner feedback round 1 (2026-10-07)

Feedback: too busy and distracting; unclear what the vault rail is; model and
thinking effort should be in the chat box; select-and-right-click commenting;
should resemble the ChatGPT desktop app; theme liked. Implemented as D16–D18 and
re-verified: native tests 301 + 7 pass; clippy clean for all feature sets;
headless captures refreshed and inspected. A new widget test dispatches the
right-click *Comment on selection…* action on a real selection and checks that
the exact-selection comment dialog opens. Native right-click by hand remains to
be tried.

### Owner feedback round 2 (2026-10-08)

Feedback: composer at the bottom; propose terminal-style chat; writing review
needs work (hard-to-see highlighting, more prominent text, compact top, Word-like
comment highlights with hover). Implemented as D20–D21; D22 prototyped. Native
tests 302 + 7 pass (new: highlight/hover marks test). The capture tour now seeds
two anchored comments and adds a light-theme proposal screen. A re-entrant
editor update found during capture was fixed. The hover popup itself was not
exercised by hand.

### Owner feedback round 3 (2026-10-08)

Feedback: Dashboard, Inbox, Needs Review, the Review list and Decided are all
open issues for the user; consolidate them. Improve Settings and the AI writing
environment, with 3–4 options each. Prototyped as D24–D26 in
`prototypes/directions.html` (four options per topic, trade-offs shown on each).
Not implemented in the app; waiting for the owner's choices. Checked only in
headless Chromium.

## Owner acceptance

| Item | Decision | Date |
| --- | --- | --- |
| Design direction and decisions D1–D15 | *Pending review* | — |
| D16–D18 (minimal layout, composer pickers, right-click comments) | Owner-directed; result pending your next look | 2026-10-07 |
| D20–D21 (bottom composer, writing review) | Owner-directed; result pending your next look | 2026-10-08 |
| D22 terminal chat direction | Owner chose Hybrid (D23) | 2026-10-08 |
| D23 Hybrid chat as implemented | Pending your next look | — |
| D24–D26 round 3 directions | Awaiting your choice | — |
| Current-app restyle on this branch | *Pending review* | — |
| Future designs and prototype | *Pending review* | — |
