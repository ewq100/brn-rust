# Bounded note-row accessibility source audit

## Identity and scope

- Repository: `/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust`
- Checked-out HEAD: `2a3a01c3ba88f5f358c6d5fba1c8fed3b7d147e`; branch `codex/agent-first-followthrough`.
- The checkout has task-owned documentation/evidence changes (`campaign.json`, `docs/work/overview.md`, and the native session evidence JSON); I did not change them. The source tree under review is the requested reviewed worker `28474ab705dceb718dedbdb96a027f2c5c36c371`, integrated main `75a66a74986792e882d1eb28c549ccacf8e7a807` per task context. The native evidence records binary source `75a66a...` and `brn-desktop` SHA-256 `7cb86bff4f9fc69558d7ed2c2672bbe6bbde4bac71a71d2405c01f9335ba354d`.
- Read-only inspection only. No GUI/CUA, build, tests, source edits, Git mutations, account/provider/network activity, or nested helpers.

## Observation versus inference

The lead's live observation is that visible Recent, Linked-to-open-chats, and All A–Z note rows can be opened by coordinate, while the reopened native window's full/diff accessibility tree includes Notes panel controls, filter text field, M/S letter buttons, and Create Topics note, but no note-row nodes or names. The campaign record explicitly bounds this as an actual Mac AX mismatch and says full VoiceOver usability was not tested. The checked-in native evidence JSON confirms the tested binary/source and session boundaries, but does not itself contain an AX snapshot or screenshot export. Thus omission is an observed native AX result relayed by the lead; no claim is made here about full VoiceOver operation.

The render path in `crates/brn-desktop/src/threads_tiles.rs:575-602` builds a `components::ListRow` with the title and metadata, calls `.build()`, and adds `.on_click(...)` plus a context menu. It does not add an explicit accessibility role or label at this call site. The same closure renders all note rows; its call sites cover Recent (`:709-712`), Linked (`:713-717`), Protected & imported (`:719-723`), Most linked (`:731-735`), and A–Z (`:737-740`). Search title rows also use that closure (`:642-646`, within the same function). The row's explicit click handler explains why coordinate activation works; the lack of explicit AX semantics at this call site is a plausible explanation for absent native nodes.

**Cause confidence: medium.** The native symptom and the source omission line up, and this is the common renderer for the observed sections. I did not inspect the external `gpui-kit::ListRow` implementation, so I cannot establish whether the component already supplies implicit semantics, whether the native bridge drops them, or whether the correct correction belongs inside the shared component. Static inference alone does not prove root cause.

## Existing meaningful witnesses

- `crates/brn-desktop/src/threads_pages_tests.rs:325-360` exercises a protected Notes-list row in a narrow panel, including visible row bounds and the protected chip. It is a useful preservation witness for compact layout and row identity, but does not assert an AX name, role, or focusability.
- `crates/brn-desktop/src/threads_pages_tests.rs:898-906` checks a Home recent-note row has a readable title/time label and explicitly is not a Button. This establishes the existing design treatment for Home rows, not the Notes tile's AX behavior.
- `crates/brn-desktop/src/threads_tiles.rs:864-875` shows Home uses the same `ListRow` builder and click-handler pattern, but is outside the reported missing-node observation.
- `crates/brn-desktop/src/threads_tiles.rs:696-705` creates the Notes A–Z letters as pills with click handlers; their explicit selected state and tooltip are distinct from the note rows. The live tree's M/S buttons demonstrate those neighboring controls are exposed.
- `crates/brn-desktop/src/threads_connections.rs:303-306` explicitly gives the Create Topics pill an aria label (`Topics · Create Topics note`), matching its observed named control and showing an established local naming pattern.
- `crates/brn-desktop/README.md:11` documents the Notes tile's filter/search, Recent, Linked, Protected & imported, and A–Z sections; it does not specify keyboard/AX behavior.

## Smallest source-supported next step

Have the implementation owner first inspect `gpui-kit::components::ListRow` semantics (the crate is a pinned registry dependency, `gpui-kit = 0.7.3`, in `crates/brn-desktop/Cargo.toml:22`). If the component does not expose a suitable accessible interactive-row API, the smallest candidate is to give the note row a named actionable semantic node at the shared `note_row` construction seam (`threads_tiles.rs:585-602`), preserving its title, metadata, selection state, protected marker, context menu, and click action. Do not choose a concrete role or duplicate announcement string until the component/bridge behavior is confirmed; a button-like role may conflict with the established non-button ListRow design witnessed on Home.

A useful regression seam is the existing native widget fixture in `threads_pages_tests.rs`: add assertions for a Notes-tile row's accessible name/role and keyboard focus/action, with one representative Recent row and one other section if the same renderer is relied upon. Preserve the narrow protected-row witness. Widget tests can show the framework tree's semantics but cannot establish macOS AX exposure; after a source change, the lead should repeat the same exact-binary native AX inspection. This audit makes no implementation dispatch recommendation beyond that investigation.

## Exclusions

No changes made. No implementation selected or tested. No claim of VoiceOver qualification, owner acceptance, or broader list accessibility. Search result rows, chat rows, Home rows, and unrelated panel controls are outside the observed defect unless the implementation owner finds the shared component is its actual cause.

## Addendum: component and pinned accessibility plumbing

### Source evidence

There is no `ListRow` implementation in pinned `gpui-kit 0.7.3`. BRN's `crates/brn-desktop/src/threads_ui.rs:33-36` imports `threads_components.rs` as the UI `components` module. At `crates/brn-desktop/src/threads_components.rs:5`, that module glob re-exports `brn_writing_ui::components::*`, while the concrete `ListRow` is declared locally at `:92-105`; its `build` implementation is at `:144-192`. Therefore `components::ListRow` in `threads_tiles.rs` resolves to BRN's own component. The desktop dependency is exactly `gpui-kit = 0.7.3` (`crates/brn-desktop/Cargo.toml:22`, also locked in `Cargo.lock:3013-3027`).

The local component already supplies an accessible **name and selected state**: `row_frame` applies `.aria_label(full)` and `.aria_selected(selected)` (`threads_components.rs:32-56`), and `ListRow::build` assembles `full` from title, optional reason, metadata, and any announced chip (`:156-162`). The note row is not missing its label in BRN's semantic input. It is missing an explicit accessibility role and focus semantics at the row frame: `row_frame` does not call `.role(...)`, `.focusable()`, or `.tab_index(...)`. The Notes tile then attaches `.on_click(...)` at `threads_tiles.rs:597-602`.

The locked stack includes `gpui-fast 0.1.7` (`Cargo.lock:2701-2705`; `gpui-kit` lists `gpui-fast` among its resolved dependencies at `Cargo.lock:3013-3027`), and `gpui-kit 0.7.3` re-exports GPUI Fast when its `gpui-fast` feature is selected (`gpui-kit-0.7.3/src/lib.rs:100-103`). Its `StatefulInteractiveElement::role` API assigns an AccessKit role (`gpui-fast-0.1.7/src/elements/div.rs:1305-1317`); `aria_label` writes a label (`:1332-1335`); `focusable()` marks the element focusable (`:1517-1520`); and `.on_click` registers the click listener (`:1587-1597`). In `write_a11y_info`, click listeners add AccessKit `Action::Click`, and focusability adds `Action::Focus` (`:3575-3648`). GPUI Fast only registers an AccessKit focus handle when the element is focusable (`:2278-2296`, and marks it focusable in the accessibility update at `:2354-2364`). Focused clickable elements synthesize clicks from Enter/Space (`:3055-3090`). Thus a row that has only an aria label/selected state and a click handler has a name and click action in its framework node, but no role or focus action; keyboard reachability/action depends on adding focusability.

The macOS backend receives GPUI's AccessKit tree through `accesskit_macos::SubclassingAdapter` and forwards updates to the adapter (`gpui-fast-macos-0.1.7/src/window.rs:2392-2419`). The adapter hook does not show an explicit BRN-specific or row-specific filter. The framework itself documents that `GenericContainer` is filtered out of the accessibility tree (`gpui-fast-0.1.7/src/elements/div.rs:1305-1317` and `:1907-1913`); an unoverridden `Div` returns no explicit role (`:1907-1913`). That is consistent with the actual omission, but this static code does not prove the exact AccessKit-to-AX behavior responsible on the observed macOS build.

### Confidence update and recommendation

**Cause confidence: high for the BRN semantic omission; medium for the exact native pruning mechanism.** Unlike the first pass, the implementation source settles that the name is already provided and identifies the concrete gap: BRN's clickable row is a generic `Div` with no explicit role and no focusability. This matches the observed absent native row nodes. The Mac adapter is a standard AccessKit adapter in the inspected source; no bridge limitation that would prevent exposing a semantic role was found.

The smallest BRN-only seam is the shared `note_row` closure in `threads_tiles.rs:597-602`: after building the row and attaching its click handler, set the row role to `gpui_kit::Role::Button` and mark it focusable. This retains the prebuilt label/selected state, existing click behavior, context menu, and all section renderings without changing the general-purpose local `ListRow` or gpui-kit dependency. GPUI Fast's focused-clickable handling supplies Enter/Space activation, and its AccessKit node exposes Click and Focus actions. This is a recommendation, not an implementation decision. Verify framework test-tree role/label/focus/action in a focused widget test, then confirm the native AX node with the lead's exact binary. The Home test's non-Button assertion (`threads_pages_tests.rs:898-906`) describes a different Home presentation; it does not determine the role for the actionable Notes-tile rows and should not be used as the semantic design rule here.

No source, test, dependency, build, GUI, or Git state was changed during this follow-up.
