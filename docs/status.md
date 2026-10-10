# BRN Threads status

**Updated:** 10 October 2026. **Branch:** `rebuild/threads`.

| Area | Actual state |
|---|---|
| Baseline | `af9239c741c7ab0983e62f0253e607b9607727e6`, current main observed during setup; includes PR #113. |
| Rebuild setup | Canonical target, current agent/documentation entry points, build plan, review brief, and prompts prepared. |
| New core and runtime | Not implemented. Existing code remains the reference/reuse starting point. |
| External Opus/Astra review | Not run by this setup task. See [review record](work/active/threads-rebuild/review.md). |
| Setup checks | Recorded in [setup evidence](work/active/threads-rebuild/evidence.md). |
| Native/toolchain readiness | Must be checked on the build agent's machine. This setup environment has no Rust/Rustup, protoc, or native Mac session. |
| Product verification and owner acceptance | Not performed for the rebuild. Baseline evidence does not qualify the new architecture. |
| Integration | This setup does not merge or release the rebuild. |

## Next action

Recommended: assign one independent review using the [review brief](work/active/threads-rebuild/review-brief.md), then assign the [build prompt](work/active/threads-rebuild/build-prompt.md). The build lead resolves findings and proceeds through [the plan](work/active/threads-rebuild/plan.md).

Only the Threads rebuild is current. Other open PRs and old plans were not selected, merged, or closed by this setup.
