# Current combined integration binding

18 October 2026. This record describes local implementation and verification, awaiting parent-owned whole-combined review and publication gates. The [approved amendment](../../modular-foundation/x2/standard-default/final-integration-amendment.md) governs execution. The [original preparation plan](../plan.md) and all 16 imported reports remain immutable historical records.

| Binding | Actual value |
|---|---|
| Assigned worktree, unchanged throughout | `/Users/evokessler/.t3/worktrees/brn-rust/t3-e61d5192` |
| Initial branch / HEAD / status | `t3/final-combined-integration-verify` / `9b47b285d23bb5f89db02ba398244827032fab6b` / clean |
| Initial and last fetched origin/main | `57c710ccec2b007c4d8a25c9f1d1b5c1c539907c` |
| Task branch, created absent at exact local base | `work/final-combined-20261018` at `124be848f03c53ddbcde32d3fccbc828e3334367` |
| Exact 16-file historical import | `927b0575fafb7acdef7f91ec6373ec0aaddec156` from `a1840aaf8fb2739600c14177ef30bba546cdce80` |
| INT-W no-fast-forward merge | `f84e83f9329d021bcef3f79b2beee3d9184894b8`, parent `d2d84e665e64de14a15f4704eb0868c553c2885a` |
| Last main no-fast-forward merge | `f89d8eff06314a87087da81ce900c9e8aa0f714e`, main `57c710ccec2b007c4d8a25c9f1d1b5c1c539907c` |
| Production integration / first counted workspace source | `ed3a650cb4352b83f9a9229e95cd66b10f6dab9b` |
| Final tested material / embedded package source | `2dd7cc604425658ad60f52734e4f8aa2f6b7fa3d` |
| Final material tree | `bf586dacd438ae20ce623d29a75091e58d590bd4` |
| Root Cargo target | `/private/tmp/brn-final-integration-20261018-target` |
| Standalone helper targets | Root target plus `/document-helpers/stock` and `/document-helpers/adapted` |
| Raw evidence | `/private/tmp/brn-final-combined-integration-20261018` |

The initial old base was recorded, never built or mutated. There was no extra worktree, reset, force update, copied checkout, cross-worktree Cargo target, review delegation, push, PR, merge into main or publication. Existing prep/X2/INT-W refs and other checkouts remain preserved. Only the authorized origin/main fetches used external network. Builds used Rust 1.98.1, one serialized job, locked/offline caches, fresh synthetic 0700 temporary directories and verified cached ORT with downloads disabled.

The final documentation commit is deliberately separate from the tested material. It changes one historical href to its locally verified main-pinned source, adds current evidence and updates the current status header. It does not rebuild or retag the package. Its actual commit is reported in the final parent handoff rather than predicted here.

See [implementation](implementation.md), [check index](check-index.json), [source and artifact pin](source-pin.json), [handoff](handoff.md) and [release](release.json).
