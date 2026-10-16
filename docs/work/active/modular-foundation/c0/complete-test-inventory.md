# Complete executed test inventory

Exact-baseline macOS [job 116997504186](https://github.com/ewq100/brn-rust/actions/runs/38908335265/job/116997504186) supplies the complete default/helper/capability run. [Retained log](hosted-core-macos-baseline.log) contains only workflow test/build/lint/retirement/help/fixture steps, preserving each line’s job, step and UTC timestamp. Setup/post-job/account configuration is excluded. Source head SHA was verified by the run metadata in `hosted-core-metadata.json`. Local feature lanes remain local, with no claim of local Ubuntu execution.

The original local workspace run stopped at the failed PDF qualification: 1,012 passed / 1 failed / 6 ignored. Those counts remain accurate partial evidence. The complete hosted default run has 1,013 passed / 0 failed / 6 ignored. All six library doc-test targets actually ran with zero tests; CLI and desktop are binary-only (no doc-test target). No zero-test target is counted as semantic proof.

| Source / exact command / crate | Passed | Failed | Ignored | Doc targets executed |
|---|---:|---:|---:|---:|
| hosted / `cargo test --workspace --locked --offline` / brn | 160 | 0 | 0 | 0 |
| hosted / `cargo test --workspace --locked --offline` / brn-ai | 343 | 0 | 1 | 1 |
| hosted / `cargo test --workspace --locked --offline` / brn-desktop | 30 | 0 | 0 | 0 |
| hosted / `cargo test --workspace --locked --offline` / brn-intake | 15 | 0 | 0 | 1 |
| hosted / `cargo test --workspace --locked --offline` / brn-retrieval | 36 | 0 | 0 | 1 |
| hosted / `cargo test --workspace --locked --offline` / brn-threads-app | 210 | 0 | 1 | 1 |
| hosted / `cargo test --workspace --locked --offline` / brn-threads-core | 183 | 0 | 1 | 1 |
| hosted / `cargo test --workspace --locked --offline` / brn-threads-intake | 36 | 0 | 3 | 1 |
| hosted / `cargo test -p brn-ai --features capability-spike --all-targets --locked --offline` / brn-ai | 346 | 0 | 1 | 0 |
| hosted / `cargo test -p brn-intake --features helper --all-targets --locked --offline` / brn-intake | 64 | 0 | 0 | 0 |
| local / `cargo test -p brn-desktop --features native-ui --locked --offline` / brn-desktop | 95 | 0 | 0 | 0 |
| local / `cargo test -p brn-desktop --features native-ui,native-retrieval --locked --offline` / brn-desktop | 95 | 0 | 0 | 0 |
| local / `cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop threads_ui:: --locked --offline` / brn-desktop | 306 | 0 | 1 | 0 |
| local / `cargo test -p brn-retrieval --features native --lib --test model_download --locked --offline` / brn-retrieval | 35 | 0 | 0 | 0 |
| local / `cargo test -p brn-threads-app --features native-retrieval --locked --offline` / brn-threads-app | 210 | 0 | 1 | 1 |

Every executed binary and doc target, feature command and result is retained in [machine-readable inventory](complete-test-inventory.json). stderr headers and stdout summaries can interleave: the parser pairs the complete ordered header/result lists per workflow step, asserts equal lengths, and verifies all six doc results are zero. Native-only local commands use the prior recorded logs without redundant broad builds. Hosted UI widget failure remains a separate required-lane failure; this inventory does not claim four green lanes.
