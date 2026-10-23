# Linux accessibility: source diagnosis and verification

This records the initial 0.7.3 batch. The subsequent authorized
[0.8.0 qualification](gpui08.md) supersedes the dependency-decision status below,
fixes toolkit disabled-node metadata and retains the Unix translation caveat.

Observation date: 2026-10-23. Base: merged main
`7b416591275f2552823cb18fe357f5a20617168a`; local work branch:
`codex/linux-native-gaps-20261023`. These findings distinguish toolkit metadata,
Linux accessibility translation, app semantics, and actual native observation.

## Implemented app changes

- `threads_components.rs` supplies `AccessibleDropdownMenu`; desktop dropdown
  callers use this bridge. An inert click handler advertises the trigger's Click
  action. GPUI's existing accessible Click dispatches mouse-down/up, and Kit's
  existing Popover owns opening, keyboard handling, dismissal, and focus. There
  is no second menu implementation. The helper returns `AnyElement` so callers
  with differently typed builder closures can share conditional branches.
- `threads_projects.rs` explicitly marks Project selection rows as Buttons beside
  their existing selection handler. Their existing full names and selected
  states remain intact. Shared display/navigation rows retain their original
  roles. In particular, Home's recent-note rows are clickable navigation rows;
  their existing acceptance contract does not classify them as Buttons.
- `project_selection_rows_expose_names_and_selection` exercises rendered Project
  rows, full names, selected states, and selection changes between two projects.

The initial blanket Button role in the shared row builder failed
`home_shows_the_intro_only_on_first_run`. That implementation was removed;
the existing Home assertion was preserved. An earlier opaque helper return type
also failed compilation and was corrected to `AnyElement`. These are corrected
failures, not omitted checks.

Root reports the subsequent complete desktop regression suite:
**352 passed, 0 failed, 1 ignored**. Existing menu keyboard, pointer, and
menu/dialog regressions passed. These toolkit tests do not establish actual
AT-SPI behavior or macOS acceptance. The subsequent native observations are recorded below.

## Disabled controls: two independent upstream gaps

**Disabled accessibility metadata remains unpassed.** App `.disabled(...)` already
suppresses activation through the existing toolkit. Correctly blocked activation
does not establish correct accessibility state reporting.

The current lockfile pins:

| Package | Version | Relevant boundary |
| --- | --- | --- |
| `gpui-kit`, `gpui-base`, `gpui-component` | 0.7.3 | Styled controls delegate to Base controls |
| `gpui-pre`, `gpui-pre-linux` | 0.3.9 | GPUI element metadata and Linux platform |
| `accesskit` | 0.24.1 | Accessibility node schema |
| `accesskit_consumer` | 0.38.0 | Node interpretation |
| `accesskit_unix` | 0.22.1 | Linux accessibility adapter |
| `accesskit_atspi_common` | 0.19.1 | AT-SPI state translation |

### Control metadata

In pinned `gpui-base-0.7.3/src/button.rs:210–255`, disabled buttons omit click
handlers and focus registration. The metadata regression at lines 499–554
explicitly records that the disabled flag is absent. GPUI's public Div builders
provide no `aria_disabled` setter. Styled Component buttons delegate to this
Base Button, so adding more app-level disabled guards does not fix the node.

[Upstream PR3531](https://github.com/longbridge/gpui-kit/pull/3531) introduces a
crate-private wrapper for the existing node. Inspection of the published
[gpui-base 0.8.0 crate](https://static.crates.io/crates/gpui-base/gpui-base-0.8.0.crate)
confirmed `src/a11y_disabled.rs` calls AccessKit's `set_disabled` and forwards the
existing element's hooks. It also places the wrapper inside `Observed` through
`map_inner`, so test snapshots report the same metadata. This avoids additional
layout/accessibility nodes and duplicate app-owned button semantics. The published
0.8.0 manifest still requires `gpui-pre =0.3.9`.

A transparent app wrapper around the styled Button is insufficient: its deferred
`ViewElement`, and the erased `AnyElement`, do not expose the inner Button's
accessibility node for amendment. Replacing that node in the app would duplicate
labels, identities, and control semantics.

### Linux state translation

Pinned `accesskit_atspi_common-0.19.1/src/node.rs:373–376` tests whether read-only
is supported, then uses `is_read_only_or_disabled`; the other branch inserts
`Enabled | Sensitive`. A disabled Button with no read-only support therefore
still receives enabled/sensitive even if its AccessKit disabled flag is fixed.

Inspection of the published
[accesskit_atspi_common 0.21.0 crate](https://static.crates.io/crates/accesskit_atspi_common/accesskit_atspi_common-0.21.0.crate)
confirmed `src/node.rs:376–382` first checks `!state.is_disabled()` before adding
read-only or enabled/sensitive states. Its manifest requires `accesskit 0.25.1`
and `accesskit_consumer 0.39.1`. The inspected
[accesskit_unix 0.24.0 crate](https://static.crates.io/crates/accesskit_unix/accesskit_unix-0.24.0.crate)
requires `accesskit 0.25.1` and common adapter 0.21.0.

In contrast, pinned `gpui-pre-linux-0.3.9/Cargo.toml:82–87` requires AccessKit
0.24 and Unix adapter 0.22. These newer adapter releases cannot simply replace
the pinned versions without evaluating the incompatible AccessKit family change.
**GPUI Kit 0.8.0 alone is insufficient to close the Linux state-reporting gap.**

## Proposed next decision and acceptance

Subject to the parent's dependency decision, evaluate published GPUI Kit 0.8.0
for control metadata, together with a compatible upstream release or narrowly
reviewed backport of the AT-SPI state-translation correction. This is a proposal,
not an authorized or completed upgrade. The temporary vendor experiment was
removed. No dependency or lockfile change is retained from this diagnosis.

After an approved correction, observe actual Linux AT-SPI nodes through
disabled → enabled → disabled transitions, checking state flags, actions, and
unchanged persisted state after attempted disabled activation. Also observe
menu activation/dismissal and Project selection through the accessibility
interface. Record source revision, native receipts, and outcomes separately
from toolkit regressions. Linux observations do not replace Mac-specific
accessibility, close/quit, or menu acceptance.

## Actual Linux native observations

The root rebuilt and launched the debug native production app with these local
changes over base `7b416591275f2552823cb18fe357f5a20617168a`. This source was still
uncommitted at observation; the eventual task commit identifies the retained
changes. Synthetic fixtures were used. The following outcomes were observed by
the root through actual Linux AT-SPI and native interaction:

| Journey | Result | Retained evidence |
| --- | --- | --- |
| Activate Project A through AT-SPI Click | PASS: Project A selected state became true | `project-a-activated-ax.json`, `project-a-activated.png` |
| Activate Project B through AT-SPI Click | PASS: Project B selected state became true | `project-b-activated-ax.json`, `project-b-activated.png` |
| Activate Open beside through AT-SPI Click, then Escape | PASS: menu opened and Escape removed it | `open-beside-ax-open-ax.json`, `open-beside-ax-open.png`, `a11y-menu-closed.png` |
| Activate Open beside again, choose Notes through the menu item, then Close Notes | PASS: Notes panel was created and subsequently closed | `open-beside-notes-ax.json`, `open-beside-notes.png` |
| Pointer-click Open beside once, then Escape | PASS: one click opened the menu and Escape dismissed it | Root native session observation; shared menu evidence above |
| Focus Open beside through AT-SPI, press Return, then Escape | PASS: Return opened the menu and Escape dismissed it | `open-beside-enter-ax.json` |
| Focus Open beside through AT-SPI, press Space, then Escape | PASS: Space opened the menu and Escape dismissed it | `open-beside-space-ax.json` |
| Activate Projects panel actions through AT-SPI Click, then Escape | PASS: the overflow menu opened and Escape dismissed it | `project-menu-ax-ax.json` |

Selected raw AT-SPI JSON and settled screenshots are retained in
[evidence/a11y](evidence/a11y/), with [SHA-256 checksums](evidence/a11y/SHA256SUMS).
They were copied without modification from
`/workspace/brn-environment/logs/linux-native-gaps/`. Screenshots supplement
actual interface/state observations; they do not establish accessibility alone.
The raw directory also retains `native-build.log` and desktop regression logs.
The root additionally reports strict Clippy for all targets with `-D warnings`
passed.

These observations establish the specified Project Click/selection and menu
journeys only. They do **not** establish Project Focus/Tab behavior, disabled
metadata correctness. Space activation and the Projects overflow menu were
subsequently observed by the root as recorded above. Disabled metadata remains
explicitly unpassed, and
Mac-specific acceptance remains separate.
