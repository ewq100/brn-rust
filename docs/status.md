# BRN Threads status

**Updated:** 10 October 2026. **Branch:** `rebuild/threads`. **Working draft:** [PR #114](https://github.com/ewq100/brn-rust/pull/114).

| Area | Actual state |
|---|---|
| Fixed reference baseline | `af9239c741c7ab0983e62f0253e607b9607727e6`, including PR #113. |
| Rebuild setup | Build assigned on the owner’s Mac; isolated checkout at `/Users/evokessler/repos/brn-threads`. [Implementation record](work/active/threads-rebuild/implementation.md) tracks current slices and reuse/deletion boundaries. |
| Independent review | Opus report received at `a2af886e5442af0cfa4b8884181446c7d98d1eea`; verdict ready with specified corrections. [Lead dispositions](work/active/threads-rebuild/review.md#build-lead-dispositions) resolve the findings into the target and plan. |
| Thread and agent design | [Concrete behavior](architecture/threads-behavior.md) defines thread/attention/run/Action boundaries and packaged runtime guidance. Packaged runtime guide/skill text is written; loader and run integration are in progress. |
| Import clarification | Full readable prose/structure, tables and meaningful figures remain required. Mathematical equation conversion/typesetting is outside the first release. |
| Dependencies | Actual pins remain Rust 1.98.1, Rig 0.43.0 and GPUI Kit 0.6.6. Rust 1.99.0, Rig 0.44.0 and Kit 0.7.1 are selected qualification targets. The plan also records bounded import/CI maintenance choices; no toolchain or dependency upgrade is implemented or tested. |
| New core and runtime | M1 final-use core is being implemented independently; its proofs and consumer switch are pending. |
| Checks and machine | See [evidence](work/active/threads-rebuild/evidence.md). Actual Mac preflight found Rustup, protoc and Apple tools. Rust 1.99.0 installed; unchanged-lock qualification in progress. Mac is locked; native interaction pending. |
| Product verification and owner acceptance | Not performed for the rebuild. Baseline evidence does not qualify the new architecture. |
| Integration | PR #114 remains a draft. No rebuild merge or release is implied. |

## Next action

Continue the assigned [plan](work/active/threads-rebuild/plan.md): finish compiler qualification, integrate/prove the small core, then agent/import/native slices. Exact selected provider/model/effort and credentials path are pending because selection lives in old data that this task must not inspect. Native interaction is pending manual Mac unlock; continue independent offline work.

Only the Threads rebuild is current. Older plans and unrelated PRs are historical or separate work.
