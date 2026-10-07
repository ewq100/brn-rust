# Guide for implementation agents

Follow this for any change to BRN's desktop UI. Product authority and the
development method remain [AGENTS.md](../../../AGENTS.md) and the
[development workflow](../../development/workflow.md); this page adds the design
rules.

## Before you change UI

1. Read the [entry point](../README.md) and only the sections your task needs.
2. Find the capability in the [product map](product-map.md): where it lives,
   its readiness, and the journeys it touches.
3. If it is a **future design**, start from [future designs](screens-future.md)
   and the [prototype](prototypes.md); confirm its assumptions are now true.
4. Check the [decisions](decisions.md) and their owner status.

## While you build

- Use tokens from `tokens.rs` and components from `native/ui.rs`. Do not add
  colours, font sizes or one-off button styles in views. If a needed value or
  component is missing, add it there and document it in this book.
- Keep element IDs and accessible labels stable; widget tests depend on them.
- Keep workflow behaviour, guards and approval meaning unchanged; UI work must
  not move logic out of `brn-workflow`.
- Lead with human titles; put exact identity in `meta`. Pair every tone with text.
- Give every surface its loading, empty, error, cancellation and success states.
- Long render functions can overflow the 2 MB test-thread stack in debug builds;
  split large element builders into `#[inline(never)]` section functions.
- Departing from a convention is allowed when needed: explain why in the change
  and record a decision here if it should become the new rule.

## After you build

- Run the desktop checks from [verification](../../development/verification.md)
  (fmt, clippy for each feature set, `native-test-support` tests).
- Run `scripts/design-capture.sh <dir>` and inspect the affected screens; update
  images in `src/images/current/` when appearance changed.
- If you changed tokens: `python3 scripts/design-handbook.py tokens`.
- Build the book: `python3 scripts/design-handbook.py build`; run
  `python3 scripts/check-markdown-links.py`.
- Update the [product map](product-map.md) readiness and the
  [verification record](verification.md). Never mark anything *Accepted*
  without a recorded owner decision.

## Review checklist

- [ ] Fits the information architecture: right region, one home, honest counts
- [ ] Uses shared tokens and components; no ad-hoc colours or sizes
- [ ] One primary command per region; button tiers correct; verbs and ellipses
- [ ] Every coloured state has text; contrast tokens unchanged or re-checked
- [ ] Knowledge kind visible (Current / Source / History / Draft / AI / web)
- [ ] Durable effects only through a named, versioned decision; consequences stated
- [ ] Loading, empty, error, cancellation, success and recovery states exist
- [ ] No typing lost on navigation, failure or close
- [ ] Keyboard reachable, visible focus, Esc closes dialogs, safe default focus
- [ ] Works at 480 × 480 and when rails auto-collapse; long text truncates or wraps safely
- [ ] Element IDs and tested accessible labels preserved; native tests pass
- [ ] Screenshots, product map and verification record updated; readiness labels honest
