# Mac-first CI cache scope selection

Observed main209 Mac Core35m49 versus main20811m28, repeating the same exact-key miss; main209 then saved a fresh1.327GB entry. Actual test-suite summed time changed only17.51s. The cause of intermittent absence remains unproven. Current cache-list totals8entries/9,571,651,328bytes; contemporaneous usage says11entries/11,062,576,769bytes, so do not treat snapshots as historical eviction proof.

Three observed Windows main cache entries total1,540,479,660bytes. Those lanes repeatedly fail in this Mac-first campaign; no Windows product qualification or port is selected. Choose one small CI candidate: leave jobs/checks/commands/timeouts/protection intact, but run the non-macOS Rust-cache step only on Linux. The separate macOS cache step stays unchanged. This avoids refreshing/uploading unqualified optional Windows graphs and copying them on PRs; existing entries remain until natural retention/eviction. No cache deletion or guaranteed reclaimed-space/speed claim is selected. A future selected Windows qualification can revisit caching.

This is resource prioritization, not a proven repair for missing Mac caches. Read-only configured-limit lookup returnedHTTP402(payment-method service requirement); do not add payment, change limits or infer the actual configured cap. Official GitHub docs describe a10GB default and oldest-last-access eviction when the configured limit is reached; that supports a pressure hypothesis, not proof for these observations:

- https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching
- https://docs.github.com/en/rest/actions/cache#get-github-actions-cache-storage-limit-for-a-repository

Only one existing composite-action condition/comment should change. No source/runtime prompt/dependency/model/paid infrastructure or gate weakening. Review complete actual delta, validate YAML and existing affected Python checks; Rust source is unchanged from independently qualified parent, so no local Rust replay. Publish only after parentPR210 integrates and reconcile exactsource. Hosted required head/main checks remain mandatory. Evaluate later natural cache observations honestly; no forced CI rerun or broad cleanup.
