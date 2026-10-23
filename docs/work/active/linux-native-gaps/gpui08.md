# Released GPUI Kit 0.8.0 qualification

Owner explicitly approved GPUI Kit 0.8.0 in this branch after the package audit.
Starting local commit: `c20eb310a9e939b342998fd4c8c121f724dcb2d5`.
Earlier evidence in this directory remains source-specific 0.7.3 evidence;
results below concern the 0.8.0 successor. No publication/merge authority is added.

## Dependency and API scope

Both desktop and writing UI pin official crates.io `gpui-kit =0.8.0`.
Base/Component/assets/macros follow the released toolkit. Its optional fast
family requires 0.1.10, explaining new locked WGPU/Naga 30 and iOS packages.
The native consumer dependency tree still selects `gpui-pre 0.3.9`, including
pre-linux/pre-platform/pre-wgpu. Fast is not in that selected tree.
AccessKit 0.24.1/common 0.19.1/unix 0.22.1 remain unchanged. Tokio 1.53.1,
Schemars 1.2.2 and Rig 0.44.0 remain unchanged; maintenance/Rig migrations are
separate work. No upstream main, vendored dependency or parser fork is used.

Official released source includes [PR3531](https://github.com/longbridge/gpui-kit/pull/3531):
Base's crate-private `a11y_disabled` wrapper marks the existing node and preserves
Observed test metadata. The existing unavailable-skill test now also asserts
its rendered control reports disabled, alongside its existing unchanged-policy
activation check. This is toolkit metadata, not AT-SPI translation acceptance.

The released `gpui-component/src/menu/dropdown_menu.rs` is unchanged from 0.7.3;
Popover still opens through its existing mouse-down handler. Retain the app's
Click-advertisement bridge and scoped Project row roles. New Popover content
key routing makes keyboard menu checks particularly relevant. No app API migration was required.

[PR3606](https://github.com/longbridge/gpui-kit/pull/3606) and
[PR3585](https://github.com/longbridge/gpui-kit/pull/3585) are later, unreleased
fixes and are not included or claimed. 0.8.0 still removes disabled-button focus
binding; Input retains its existing outer-frame semantics. Mac acceptance and
full Linux focus/disabled semantics cannot be inferred from the upgrade.

## Verification

The root qualified the 0.8.0 working-tree successor to
`c20eb310a9e939b342998fd4c8c121f724dcb2d5`. These observations do not apply
retroactively to the earlier 0.7.3 binary. The native result records binary
SHA-256 `0be363ada8494400f9d81fa95e30f93f07da4e81dbadbac161e34ecc0a4c4e2c`.
The raw `result.json` SHA-256 is `4768835ebb9eda4ef258ae3e07f977cee2fd404a4d87cb568494d77afff725a0`;
it contains no separate source-tree hash. The eventual retained commit must
identify the qualified source changes.

| Check | Result |
| --- | --- |
| Full desktop suite | PASS: 352 passed, 0 failed, 1 ignored; includes rendered disabled metadata assertion |
| Writing UI suites | PASS: 44 unit tests and 12 toolkit example-host tests |
| Debug production native build | PASS |
| Strict Clippy, all targets, `-D warnings` | PASS |
| Selected native dependency tree | `gpui-pre 0.3.9`; no selected Fast backend |
| Native exact Unicode edit/save v2/quit/reopen/OS close | PASS; saved/reopened snapshots and settled screenshots retained |
| Actual AT-SPI menu item activation; repeated Return/Space/Escape/pointer flows | PASS |
| Project A/B AT-SPI Click and selected state | PASS |
| Disabled Rewrite | PARTIAL: no action exposed; Linux still reports enabled/sensitive |
| Connections Unlink/Undo | PASS for observed retirement and Undo: frame 0104 shows pending, frame 0124 shows zero; Undo restored 1 of 1, with complete Note snapshots unchanged |
| Read previously restored disposable copy | PASS: v2 content viewed natively; complete records/history/recovery/candidates snapshots byte-identical before and after |

Reading the restored copy is not a new restore operation or a native chooser
acceptance claim. The native file chooser remains **blocked** by the previously
recorded portal caller-inspection AccessDenied; this upgrade does not authorize
retry or bypass. Mac-specific acceptance remains **pending**. Project Focus/Tab
and the later unreleased focus fixes are not established by these results.
Disabled toolkit metadata now passes its rendered regression, but Linux AT-SPI
state translation remains unpassed; the released toolkit did not change the
pinned AccessKit adapter family.

Independent source/lock audit found no unrelated update or blocking API issue.
Selected raw receipts, small screenshots, and two Connections frames are retained
in [evidence/gpui08](evidence/gpui08/). Videos and runtime process/control files
are excluded. [SHA256SUMS](evidence/gpui08/SHA256SUMS) covers retained files;
[provenance.json](evidence/gpui08/provenance.json) records original and tracked
hashes. Copied terminal logs have ANSI sequences removed, line endings normalized,
and trailing whitespace trimmed; JSON/screenshots are unchanged. Original logs
remain in `/workspace/brn-environment/logs/linux-native-gaps/` and
`/workspace/brn-environment/logs/gpui08-native/`.
