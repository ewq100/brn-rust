# BRN Threads status

**Updated:** 10 October 2026. **Branch:** `rebuild/threads`. **Working draft:** [PR #114](https://github.com/ewq100/brn-rust/pull/114).

| Area | Actual state |
|---|---|
| Fixed reference baseline | `af9239c741c7ab0983e62f0253e607b9607727e6`, including PR #113. |
| Rebuild setup | Canonical target, coherent agent/documentation entry points, plan and build prompt prepared and published. |
| Independent review | Opus report received at `a2af886e5442af0cfa4b8884181446c7d98d1eea`; verdict ready with specified corrections. [Lead dispositions](work/active/threads-rebuild/review.md#build-lead-dispositions) resolve the findings into the target and plan. |
| Thread and agent design | [Concrete behavior](architecture/threads-behavior.md) defines thread/attention/run/Action boundaries and packaged runtime guidance. Planned runtime guide/skill files are not yet implemented. |
| Import clarification | Full readable prose/structure, tables and meaningful figures remain required. Mathematical equation conversion/typesetting is outside the first release. |
| Dependencies | Current code remains Rig 0.43.0 and GPUI Kit 0.6.6. Rig 0.44.0 and Kit 0.7.1 are selected qualification targets; source inspection supports the plan, but upgrades are not implemented or tested. |
| New core and runtime | Not implemented. Existing code remains the reference/reuse starting point. |
| Checks and machine | See [evidence](work/active/threads-rebuild/evidence.md). This preparation environment lacks Rust/Rustup, protoc and a native Mac session; the build agent must preflight its actual machine. |
| Product verification and owner acceptance | Not performed for the rebuild. Baseline evidence does not qualify the new architecture. |
| Integration | PR #114 remains a draft. No rebuild merge or release is implied. |

## Next action

Assign the updated [build prompt](work/active/threads-rebuild/build-prompt.md). The [plan](work/active/threads-rebuild/plan.md) starts with the small final-use core and concrete proofs; qualify dependencies before their agent/native integration slices. No further general architecture review or owner questionnaire is a prerequisite.

Only the Threads rebuild is current. Older plans and unrelated PRs are historical or separate work.
