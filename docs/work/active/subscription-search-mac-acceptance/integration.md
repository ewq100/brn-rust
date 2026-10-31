# PR244 integration checkpoint

The owner’s 31 October “PR and merge” request selects normal integration of [PR244](https://github.com/ewq100/brn-rust/pull/244). It does not authorize release, another live allocation, UI interaction, unlocking, paid-account changes or bypassing checks. Diagnostic allocation003 remains closed at 4/10; its successful public-query evidence is bound to `fefc1e8b11a9d56a44ec5f56abc9bf4a1eb5ceae` and retained unchanged.

## Pre-publication reconciliation

- Main is `89e6221afc8d6b036b7b5a1bcfe4b7cc23871583`. Its four required contexts passed in [CI40990750144](https://github.com/ewq100/brn-rust/actions/runs/40990750144); three optional Windows jobs failed. The existing draft PR244 is at remote `2764b52f73f2269d3ba770516f5314b2056c8522`. All four required contexts passed for that older PR head, not the unpublished correction.
- Local HEAD before this integration correction was `215b402513e3244dc25e7105d8271ab968a00723`, five unpushed commits and a clean working tree. All open PRs were reconciled. PR239 stays paused; unrelated branches, worktrees and dirty files are preserved.
- Real native Mac interaction remains unobserved, as disclosed before the integration request. No buttons, focus, rendering, accessibility, IME, pickers or owner-acceptance pass is manufactured. Issue243 stays open.
- All four repository-declared exact-head CI contexts remain required: Ubuntu and Mac Core/CLI, Mac Native UI, and Mac Native Retrieval/Combined. Protection and ruleset reads return403 because of private-repository plan restrictions. No setting, visibility or check is weakened.
- **Actions admission approved:** after the earlier explicit exhausted-minutes hold and unavailable billing reads, the owner directly replied “Approved” to the request to proceed with required CI and merge. This clears that hold for normal PR244 publication/required CI and resulting-main verification. It does not authorize billing-setting changes, purchases, weakened checks, unrelated runs or another live model allocation. The approval is not evidence of the account’s actual dollar balance; cost remains unavailable unless exposed.

## Review finding and correction

Final independent review of the complete actual main-to-candidate diff found that `Server::rpc` and `drain_pre_turn` could discard `error`, unsafe item and unknown notifications after only the shared account/quota/warning checks. A later successful response could permit query dispatch. The post-dispatch collector did not cover this phase. This was a valid finding.

A local fake subprocess sent a synthetic error before a successful RPC response. Before correction, the regression failed: actual `Ok({})`, expected `Err(ProviderFailed)`. The original failed log stays local. No model call was used.

The correction shares a phase-aware pre-turn validator between RPC and queue draining. It keeps request/account/quota/warning stops, permits only supported startup metadata and idle lifecycle notices, rejects premature item/turn activity and unknown/malformed messages, and strictly matches RPC replies. Thread creation notices must describe an ephemeral idle thread with no turns, have bounded IDs and match an already known thread when one exists. They never establish identity; the verified `thread/start` response remains authoritative.

Four regressions exercise error/unsafe/unknown-before-success ordering, phase/correlation/idle creation boundaries, deterministic queued drain refusals and benign startup/RPC/drain behavior. Existing warning, admission, quota, moderation, unsafe-tool, deadline and process-cleanup tests remain applicable.

A fresh zero-turn metadata probe against the existing installed App Server confirmed the actual creation shape: `status.type=idle`, `ephemeral=true`, `turns=[]`. It verified the normal managed login, offered Sol medium and the restricted ephemeral boundary, then reaped both owned processes. It sent no `turn/start`, user query or model request. Raw metadata, account fields and probe logs stay local; only these sanitized observations are published.

## Verification and review

The correction’s provider source SHA256 is `4c66c42960e4bc0e0d8761e929b525cb5439d9532928acfbfdeb0ce076263524`. The existing independent Sol reviewer examined the complete candidate plus this correction and found the issue fixed, with no new actionable product findings. A final source/receipt/privacy review matched the actual gate logs, original failed regression and built binary; no inconsistencies remained. The lead retains its selected model, one existing reviewer and one serialized build slot; no helper or live-call allocation is added. Stop on a failing check or missing integration authority.

All five serialized offline gates passed on production source `22f3860eacd4b879338145c9ed707987006811df`: **35 provider tests and three private-file tests passed, none failed or ignored**, strict AI/application and desktop lint passed, and the qualification example built. Formatting, retirement checks, generated overview and Markdown links also passed. The [sanitized receipt](integration-offline-result.json) retains exact commands, source/binary hashes, the original red regression and review outcome. Use Rust1.98.1, `--locked --offline`, one build job, no incremental compilation, an explicit isolated target and a canonical synthetic temporary directory:

```text
cargo +1.98.1 test --locked --offline -p brn-ai subscription_search
cargo +1.98.1 test --locked --offline -p brn-ai --features capability-spike --example subscription-search
cargo +1.98.1 clippy --locked --offline -p brn-ai -p brn-threads-app --features brn-ai/capability-spike --all-targets -- -D warnings
cargo +1.98.1 clippy --locked --offline -p brn-desktop --features native-test-support --all-targets -- -D warnings
cargo +1.98.1 build --locked --offline -p brn-ai --features capability-spike --example subscription-search
cargo +1.98.1 fmt --all -- --check
python3 scripts/check-provider-retirement.py
python3 scripts/check-intake-retirement.py
python3 scripts/render-work-overview.py --record docs/work/active/subscription-search-mac-acceptance/record.json --check
python3 scripts/check-markdown-links.py
git diff --check
```

This new guard is **offline-qualified only** on the recorded source. Its production bytes differ from the live-tested predecessor. The single successful live query does not qualify the later guard, broader search quality or native behavior. No further live call is necessary to demonstrate the reproduced refusal defect; any materially new live test needs fresh authorization. At this approval checkpoint, publication and required exact-head CI are next; no merge or release is claimed. Current hosted results belong to the PR checks and subsequent source-bound integration receipt.


## Published head — hosted execution blocked

After the owner’s direct approval, the reviewed commits and approval checkpoint were pushed normally. Exact local/remote PR244 head: `8094dcf256f4ff31ad5a9914d30381bc492eee8d`. The PR description was updated; the PR remains draft, unmerged, and issue243 stays open. Production source remains identical to the reviewed/offline-qualified `22f3860e` source; no new live request was spent.

[CI41163642499](https://github.com/ewq100/brn-rust/actions/runs/41163642499) is terminal **failure before execution**. All seven jobs have zero steps. Their failure annotations say the account has a payment or spending-limit restriction. This is not a compiled/tested product failure and not a passing check. The separate macOS-capacity notices do not explain the shared blocking failure. The [sanitized hosted receipt](integration-ci-result.json) retains exact head, job IDs, timestamps and failure annotations.

All four required contexts remain unsatisfied. No blind rerun, billing/security/protection change, ready transition, merge or release is attempted. The owner needs to resolve GitHub account billing admission; the existing approval already covers continuing normal required CI/integration afterward. Actual charges are not exposed and are not inferred from the zero-step result. This local checkpoint is not pushed while admission remains blocked, avoiding another futile CI trigger; the PR and issue carry the remotely visible status.
