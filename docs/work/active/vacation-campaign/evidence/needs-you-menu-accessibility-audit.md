# BRN Needs you menu accessibility source audit

Scope: read-only source audit at `codex/agent-first-followthrough`, HEAD `abb55cdd41cd00bd4388e635f6b91aa7b5f6a9fc`. Existing working-tree changes were present in `docs/work/active/vacation-campaign/campaign.json`, `docs/work/overview.md`, and untracked `docs/work/active/vacation-campaign/evidence/pr186-publication.json`; they were left untouched. No build, test, GUI, account, provider, network, or source/Git operation was performed.

## Finding

There is a concrete BRN/component accessibility-name gap for the Needs you rows, with high source confidence. In `crates/brn-desktop/src/threads_shell.rs` around 1087–1113, each row is built with `PopupMenuItem::element(...)`. Its custom child `div` gets `.aria_label(text.clone())`, but the outer `PopupMenuItem` has no accessible-label setter call. The click callback is attached to the `PopupMenuItem` and routes to `view.open_thread`, so the actionable wrapper and labeled child are distinct nodes.

The active pinned component source is `gpui-component 0.7.3`, pulled by the optional `gpui-kit = 0.7.3` native UI dependency in `crates/brn-desktop/Cargo.toml`; `Cargo.lock` pins `gpui-pre 0.3.9` transitively. These versions identify the source generation, not proof that a particular binary enabled `native-ui`.

In `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-component-0.7.3/src/menu/popup_menu.rs`, `PopupMenuItem::a11y_label()` returns `None` for `ElementItem` (around lines 272–278). `PopupMenu::render_item` applies `.aria_label(...)` to the outer `MenuItemElement` only when that method returns `Some` (around lines 1208–1268). `ElementItem` then renders the caller's custom element as a child (around lines 1286–1317). The enclosing menu item still has `Role::MenuItem` in `menu_item.rs` around lines 92–101, but gets no label through this path. The custom child label does not set the wrapper's label.

This matches the supplied native observation: the “Needs you” heading was exposed, while the visible row could not be activated by named item and required screen-coordinate activation. That observation supports the source finding; this audit makes no broader VoiceOver or macOS bridge claim.

## Existing guarantees and test seam

`crates/brn-desktop/src/threads_pages_tests.rs`, `a_needs_you_title_stays_beside_a_long_reason_one_line_each` (around lines 216–285), covers full label text, title/reason one-line sizing, truncation within the menu, tooltip intent, and click routing. Its `item.label()` assertion may observe the custom child and therefore does not prove the outer menu item's accessible name. `each_home_item_keeps_a_keyboard_route` (around lines 1175–1265) verifies the badge-to-menu ↓/Enter route and that Home content remains keyboard reachable. Preserve those seams and add an assertion aimed specifically at the enclosing menu item's accessibility name/role, rather than the child label.

## Bounded proposal

Prefer the component's standard `PopupMenuItem::new(full_text)` path for this row if its rendered long-label layout can satisfy the existing 420 px menu and one-line/clipping constraints. The component's standard item path uses the full visible label as the outer menu item's accessible label and retains the same handler and keyboard menu routing. Re-run the retained long-title/reason and route tests after any implementation; inspect the resulting accessibility tree on the requested Mac before claiming native acceptance. If the standard item cannot preserve the current truncation and tooltip behavior, the pinned public `PopupMenuItem` API offers no custom accessible-label setter for the outer `ElementItem`; do not paper over that with a generic framework or platform bridge change. A minimal capability addition would then need to expose/set the outer ElementItem label at the component seam, with separate component and BRN tests.

## Limits

This is source evidence plus the native observation supplied in the task. It is not an independent GUI reproduction, VoiceOver audit, or verification of other menu routes. No source fix is included.

## Follow-up: existing-capability label node route

The proposed GPUI/AccessKit adapter is source-supported and appears to be the smallest route that preserves the existing custom visual row, tooltip, truncation, click handling, and menu keyboard behavior. In `gpui-pre 0.3.9`, `Div::aria_value` stores the string value in accessibility info (`src/elements/div.rs` around lines 1423–1430). `Text` elements with an ID also become `Role::Label` and expose their text through `Node::set_value` (`src/elements/text.rs` around lines 189–192 and `write_a11y_info` around 1100–1108); plain SharedString text without an ID does not produce that label node.

`accesskit_consumer 0.38.0` confirms the mechanism: a MenuItem with no explicit `labelled_by` implicitly finds labels among descendants (`src/node.rs` around lines 702–719); its descendant filter includes `Role::Label`, descends through `Role::GenericContainer`, and excludes unrelated subtrees (around lines 693–699). A `Role::Label` contributes its value as the name (`label_comes_from_value`, `has_label`, and `write_label`, around lines 727–778). So changing the existing custom line root to `.role(Role::Label).aria_value(text.clone())` supplies a descendant label node for the outer MenuItem's computed name while keeping the custom child layout intact. The parent menu wrapper is the MenuItem role; leave the custom label node as `Role::Label`, rather than making it another nested MenuItem.

The pinned `accesskit_macos 0.26.3` adapter returns `Node::label()` as the platform node title (`src/node.rs` around lines 302–312), and its `accessibilityTitle` implementation uses that wrapper title (around lines 509–518). Thus the consumer-computed descendant label should flow to the macOS title/name field. This is a strong adapter-path explanation, not proof of the current or fixed native tree.

A focused retained test can assert that the custom row's accessibility node has `Role::Label`, its value is the complete text, and the outer menu item computes the same label; keep the current bounds, tooltip, click, and Home keyboard-route assertions. If the UI test harness cannot inspect the computed parent name, a component-level/accessibility-tree seam can test the AccessKit computation without changing app behavior. The supplied actual-Mac check remains the confirmation that VoiceOver finds each item by its name; do not elevate source/test success to native acceptance.

Caveat: do not put `aria_label` on the Role::Label node as the only mechanism. AccessKit's `label_comes_from_value` uses the label node's value, while its direct `label` field is not what this route consumes. Set `.aria_value(...)` with the full original string. No source/build/test/native change was made during this follow-up.
