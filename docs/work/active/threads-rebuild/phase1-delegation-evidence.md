# Phase 1 delegation evidence

## Authority and baseline

Owner requested PR publication, CI-gated Phase 0 merge and continued building on 11 October 2026. Phase 0 native observation was explicitly deferred after repeated app-access denial; no native pass is inferred. This increment implements [the reviewed plan](phase1-delegation-plan.md) using GPT-6.1 Sol helpers and an independent review.

PR [#115](https://github.com/ewq100/brn-rust/pull/115) merged at `53e5289e7221a4c3b66c6c454550311e46617a78`. All four branch-required checks and documentation/tooling passed at exact head `7c95af16d1d377a29736f77a69f1e7343184271c` in [run 38134309814](https://github.com/ewq100/brn-rust/actions/runs/38134309814). The branch read confirmed enforcement for everyone of Core/CLI Linux, Core/CLI Mac, native UI Mac and native retrieval/combined Mac. The non-required Windows build failed on the documented baseline Unix credential API issue in unchanged `auth.rs`/`codex_auth.rs`; it was not suppressed or described as green.

The GitHub CLI token was invalid, and Git HTTPS push had no usable credential. The connected GitHub integration published blobs/tree/commit instead. Remote Git tree `6d85e96868aa29e563d4b7522ab6ef1414af34cc` was verified identical to the reviewed local `7e30e8b` tree before PR creation. This changes commit identity, not the tested file contents. No credential was extracted or modified. The merge tree matches that exact tree. The original checkout remains unchanged.

Phase 1 uses isolated worktree `brn-phase1-delegation`, branch `work/phase1-delegation`, fast-forwarded from the PR head to the identical-tree merge commit. The owner subsequently authorized delegation PR publication and merge after required CI, followed by further Phase 1 work. No release is authorized.

## Plan review

Independent GPT-6.1 Sol review: APPROVED. Review emphasized exact legacy serialization/replay, no generic configuration forgery, concurrent provider preservation, choice/run fencing in one transaction and native fixture isolation.

## Implementation and automated results

Implemented the defaulted/omitted-false choice marker without a SQL schema change. Choice and Pause patch current Settings inside the existing checked writer and fence all Working runs atomically. Provider Configure preserves all current policy fields, including when a stale form predates the choice. Missing/archived/unchosen Settings grant no standing maintenance; manual work, exact applicable instructions and owner review retain their existing boundaries. Host runtime input reports only a policy name from the captured Settings revision, never credential locations. Native Home/Settings and CLI use the same operation.

Observed passing local checks on the candidate, with the pinned toolchain and existing lockfile:

| Check | Result |
|---|---|
| Core and application integration suites | 61 core + 29 app tests passed, including the 5,000-note witness. |
| AI Threads suite | 40 tests passed, including two new host-policy/authority regressions. |
| CLI integration suite | 5 tests passed. |
| Default desktop suite | 17 tests passed. |
| Native component suite | 27 tests passed; overlaps default desktop coverage. |
| Strict all-target Clippy | Core, app, AI, CLI and native-test-support desktop passed. |
| Native UI build | Passed. |
| Formatting and diff checks | Passed. |
| Markdown links | 98 files and 380 local links checked; zero failures. |

The ten new core/app tests cover pending resume, both choices and reopen, exact legacy bytes/hash/receipt replay, changed-choice retries, lost responses, concurrent provider preservation in both directions, choice/Pause fencing and stale independent connections. Existing protection, review, Undo and current-input suites remain green. All native component fixtures persist an unsupported offline provider and empty credential paths before UI creation. No Phase 1 test called a live provider or used private data or real credentials.

Commands from `brn-phase1-delegation` (existing build caches, unchanged dependency lockfile):

```sh
CARGO_TARGET_DIR=../threads-macbook-checkout/target cargo test --locked --offline -p brn-threads-core -p brn-threads-app
CARGO_TARGET_DIR=../threads-macbook-checkout/target cargo test --locked --offline -p brn-ai threads
CARGO_TARGET_DIR=../threads-macbook-checkout/target cargo clippy --locked --offline -p brn-threads-core -p brn-threads-app -p brn-ai --all-targets -- -D warnings
CARGO_TARGET_DIR=../threads-macbook-local/target cargo test -p brn -p brn-desktop --locked --offline
CARGO_TARGET_DIR=../threads-macbook-local/target cargo test -p brn-desktop --features native-test-support --locked --offline
CARGO_TARGET_DIR=../threads-macbook-local/target cargo clippy -p brn -p brn-desktop --features brn-desktop/native-test-support --all-targets --locked --offline -- -D warnings
CARGO_TARGET_DIR=../threads-macbook-local/target cargo build -p brn-desktop --features native-ui --locked --offline
cargo fmt --all --check
python3 scripts/check-markdown-links.py
git diff --check
```

Observed before implementation: the fresh-workspace test failed because maintenance started true; the missing-Settings authority test returned Applied instead of NeedsReview. Both pass after implementation. Two CLI tests also failed before implementation because help lacked the choice command and Settings output omitted the false marker; both now pass. New replay/race tests exercise production persistence and independent connections, not duplicate policy calculations. Core/app and AI logs were recorded locally at `/tmp/phase1-core-app-final.log`, `/tmp/phase1-ai-test.log`, and `/tmp/phase1-clippy.log`.

## Independent candidate review

GPT-6.1 Sol returned APPROVED against merged baseline `53e5289`, reviewing the complete diff and new tests. No actionable code defects. One documentation ambiguity was resolved: generic provider configuration preserves policy; fully trusted owner ChangeRequests retain their existing Settings capability for bootstrap/host operations. Neither model tools nor the provider form can reach arbitrary owner Settings writes. Plan and core README now state this boundary explicitly. The reviewer inspected code and diff checks; test runs were performed by the implementation helpers. No production change followed approval.

## Phase 0 post-merge checks

[Main run 38134559985](https://github.com/ewq100/brn-rust/actions/runs/38134559985) completed at merge `53e5289`. All four required contexts passed again, plus documentation/tooling and Linux native UI. Overall run remains failure because the non-required Windows lanes and Linux native retrieval failed. Additional platform probes retain these limits: Linux model-install tests return `ExclusiveInstallUnavailable` because the unchanged implementation supports exclusive installation only on macOS; a Windows native setup received a Chocolatey HTTP 504. No required check, workflow job or protection was removed, waived or bypassed. Phase 0 is integrated for its qualified Mac scope; it is not an all-platform green build.

## Native and integration status

Native interaction remains unobserved because Computer Use access is unavailable. Automated native component tests do not substitute for native observation. This reviewed and tested increment is authorized for PR publication and CI-gated merge; broader Phase 1 storage/recovery and Stop-and-send work remain outstanding.

## Publication and integration

The owner explicitly requested publication, required-CI-gated merge and continued Phase 1 work. PR [#116](https://github.com/ewq100/brn-rust/pull/116) merged at `36ad2d61e5895c9a564dcff8f6eec1053376245b`. Published head `e6a26aaa2ebeded9a0508f89ce7750fb1ae18d34` has exact Git tree `09e7c1a7f08239847bd1663847824f16d1d5ae5c`, identical to local `e2f2e12` (implementation `7651303` plus authority documentation). Merge preserves the same tree.

[PR run 38136175465](https://github.com/ewq100/brn-rust/actions/runs/38136175465) passed all four required checks and documentation/tooling. Main branch required contexts were reread immediately before merge; none were waived or modified. Non-required Windows Core/CLI failed; the run is not represented as all-platform green. Post-merge verification is tracked in the next steering evidence.
