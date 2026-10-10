# BRN Threads status

**Updated:** 11 October 2026. **Branch:** `rebuild/threads`. **Working draft:** [PR #114](https://github.com/ewq100/brn-rust/pull/114).

| Area | Actual state |
|---|---|
| Fixed reference baseline | `af9239c741c7ab0983e62f0253e607b9607727e6`, including PR #113. |
| Rebuild setup | Build assigned on the owner’s Mac; isolated checkout at `/Users/evokessler/repos/brn-threads`. [Implementation record](work/active/threads-rebuild/implementation.md) tracks current slices and reuse/deletion boundaries. |
| Independent review | Opus report received at `a2af886e5442af0cfa4b8884181446c7d98d1eea`; verdict ready with specified corrections. [Lead dispositions](work/active/threads-rebuild/review.md#build-lead-dispositions) resolve the findings into the target and plan. |
| Thread and agent design | [Concrete behavior](architecture/threads-behavior.md) defines thread/attention/run/Action boundaries and packaged runtime guidance. Packaged runtime guide/skill text is written; loader/runtime qualification is in progress. |
| Import clarification | Full readable prose/structure, tables and meaningful figures remain required. Mathematical equation conversion/typesetting is outside the first release. |
| Dependencies | Rust1.99 unchanged-lock builds/lints/state tests passed; pin stays1.98.1 pending interaction. Rig0.44 has a reproduced malformed-stream blocker;0.43 patch remains. Kit0.7.1 source/build trial passed; consumer upgrade is in progress. |
| New core and runtime | M1 final-use core has38 passing locked tests and scoped review fixes. Transient PDF/DOCX/EML/Markdown intake has10 passing acceptance tests. Runtime/app consumer switch is in progress. |
| Checks and machine | See [evidence](work/active/threads-rebuild/evidence.md). Actual Mac preflight found Rustup, protoc and Apple tools. Compiler/library trials are recorded in evidence. Mac is locked; native interaction pending. |
| Product verification and owner acceptance | Offline core/import evidence exists. New runtime live smoke passed through Codex/gpt-6.1-sol. Native interaction and owner acceptance are pending. |
| Integration | PR #114 remains a draft. No rebuild merge or release is implied. |

## Next action

Continue the assigned [plan](work/active/threads-rebuild/plan.md): finish runtime/app/native consumer qualification and coherent retirement, run remaining headless/live journeys, and record the tested source commit. One hands-on Mac interaction task remains pending; no unlock attempted.

Only the Threads rebuild is current. Older plans and unrelated PRs are historical or separate work.
