# Local baseline results

macOS local execution at unchanged production tree `8f768943`; all commands serialized. `results.json` records exact commands, environment and elapsed wall seconds including compilation. Harness suite durations exclude compilation. The runner continues to independent lanes after failures; inspect each captured exit (the initial runner continued after failure and returned zero; the checked-in runner now returns nonzero for any failed command).

| Lane | Commands | Wall seconds | Failed commands |
|---|---:|---:|---:|
| helper | 3 | 78.081 | 0 |
| core | 7 | 177.566 | 1 |
| native-ui | 5 | 179.951 | 0 |
| native-retrieval | 6 | 168.134 | 0 |

## Exact commands

| Log | Exit | Seconds | Command |
|---|---:|---:|---|
| [00-helper.log](00-helper.log) | 0 | 22.888 | `cargo build -p brn-intake --features helper --bin brn-intake-helper --locked --offline` |
| [01-helper.log](01-helper.log) | 0 | 10.413 | `cargo clippy -p brn-intake --features helper --all-targets --locked --offline -- -D warnings` |
| [02-helper.log](02-helper.log) | 0 | 44.78 | `cargo test -p brn-intake --features helper --all-targets --locked --offline` |
| [03-core.log](03-core.log) | 0 | 1.653 | `cargo fmt --all -- --check` |
| [04-core.log](04-core.log) | 0 | 24.163 | `cargo build --workspace --locked --offline` |
| [05-core.log](05-core.log) | 0 | 13.445 | `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` |
| [06-core.log](06-core.log) | 101 | 117.824 | `cargo test --workspace --locked --offline` |
| [07-core.log](07-core.log) | 0 | 14.653 | `cargo test -p brn-ai --features capability-spike --all-targets --locked --offline` |
| [08-core.log](08-core.log) | 0 | 3.541 | `cargo clippy -p brn-ai --features capability-spike --all-targets --locked --offline -- -D warnings` |
| [09-core.log](09-core.log) | 0 | 2.287 | `bash scripts/verify-end-to-end.sh --fixtures-only` |
| [10-native-ui.log](10-native-ui.log) | 0 | 37.325 | `cargo build -p brn-desktop --features native-ui --locked --offline` |
| [11-native-ui.log](11-native-ui.log) | 0 | 22.375 | `cargo clippy -p brn-desktop --features native-ui --all-targets --locked --offline -- -D warnings` |
| [12-native-ui.log](12-native-ui.log) | 0 | 58.099 | `cargo test -p brn-desktop --features native-ui --locked --offline` |
| [13-native-ui.log](13-native-ui.log) | 0 | 13.853 | `cargo clippy -p brn-desktop --features native-ui,native-test-support --all-targets --locked --offline -- -D warnings` |
| [14-native-ui.log](14-native-ui.log) | 0 | 48.299 | `cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop threads_ui:: --locked --offline` |
| [15-native-retrieval.log](15-native-retrieval.log) | 0 | 18.08 | `cargo test -p brn-retrieval --features native --lib --test model_download --locked --offline` |
| [16-native-retrieval.log](16-native-retrieval.log) | 0 | 55.021 | `cargo test -p brn-threads-app --features native-retrieval --locked --offline` |
| [17-native-retrieval.log](17-native-retrieval.log) | 0 | 14.457 | `cargo build -p brn --features native-retrieval --locked --offline` |
| [18-native-retrieval.log](18-native-retrieval.log) | 0 | 21.586 | `cargo build -p brn-desktop --features native-ui,native-retrieval --locked --offline` |
| [19-native-retrieval.log](19-native-retrieval.log) | 0 | 14.336 | `cargo clippy -p brn-desktop --features native-ui,native-retrieval --all-targets --locked --offline -- -D warnings` |
| [20-native-retrieval.log](20-native-retrieval.log) | 0 | 44.654 | `cargo test -p brn-desktop --features native-ui,native-retrieval --locked --offline` |

## Counts per crate and feature command

Counts aggregate executed test binaries under each exact feature command; examples and doc tests count only actual tests. A zero-test example contributes zero, not proof. The workspace command stops at intake qualification, so later binaries/doc tests may not execute; these numbers are observed counts, not an exhaustive listing. No newly ignored tests were introduced.

| Command log / crate | Passed | Failed | Ignored |
|---|---:|---:|---:|
| 02-helper.log / brn-intake | 64 | 0 | 0 |
| 06-core.log / brn | 160 | 0 | 0 |
| 06-core.log / brn-ai | 343 | 0 | 1 |
| 06-core.log / brn-desktop | 30 | 0 | 0 |
| 06-core.log / brn-intake | 15 | 0 | 0 |
| 06-core.log / brn-retrieval | 36 | 0 | 0 |
| 06-core.log / brn-threads-app | 210 | 0 | 1 |
| 06-core.log / brn-threads-core | 183 | 0 | 1 |
| 06-core.log / brn-threads-intake | 35 | 1 | 3 |
| 07-core.log / brn-ai | 346 | 0 | 1 |
| 12-native-ui.log / brn-desktop | 95 | 0 | 0 |
| 14-native-ui.log / brn-desktop | 306 | 0 | 1 |
| 15-native-retrieval.log / brn-retrieval | 35 | 0 | 0 |
| 16-native-retrieval.log / brn-threads-app | 210 | 0 | 1 |
| 20-native-retrieval.log / brn-desktop | 95 | 0 | 0 |

## Slow-suite evidence and limits

The harness reports duration per test binary; this is **not** an individual-test timing ranking. Slow individual tests are sampled separately from the slow binaries, with process wall time and commands in `slow-tests.json`; compilation and scheduling contaminate small differences. No global fastest/slowest claim, flaky rate, or benchmark improvement is inferred.

| Log / crate / suite | Harness seconds |
|---|---:|
| 06-core.log / brn / Running tests/agent_bounds.rs | 10.9 |
| 06-core.log / brn-threads-core / Running tests/edit_tokens.rs | 5.55 |
| 16-native-retrieval.log / brn-threads-app / Running tests/agent_journeys.rs | 4.46 |
| 06-core.log / brn-threads-app / Running tests/agent_journeys.rs | 4.4 |
| 07-core.log / brn-ai / Running unittests src/lib.rs | 2.96 |
| 06-core.log / brn-ai / Running unittests src/lib.rs | 2.93 |
| 14-native-ui.log / brn-desktop / Running unittests src/main.rs | 2.93 |
| 16-native-retrieval.log / brn-threads-app / Running tests/retrieval_eval.rs | 2.73 |
| 16-native-retrieval.log / brn-threads-app / Running tests/user_scale.rs | 2.37 |
| 06-core.log / brn-threads-app / Running tests/retrieval_eval.rs | 2.31 |
| 16-native-retrieval.log / brn-threads-app / Running tests/agent_eval_journeys.rs | 2.27 |
| 06-core.log / brn-threads-app / Running tests/agent_eval_journeys.rs | 2.18 |

## Existing ignored witnesses

- 06-core.log / brn-ai: `test provider_formats_tests::provider_stderr_tests::provider_stderr_child ... ignored, only the parent test launches this isolated synthetic stderr fixture`
- 06-core.log / brn-threads-app: `test native_check_store_with_a_killed_intake_and_its_candidate ... ignored, writes a store for the owner's native check; set BRN_DWR_NATIVE_DATA and BRN_DWR_NATIVE_MODE`
- 06-core.log / brn-threads-core: `test backup::tests::stage_holder_child ... ignored, runs only as the child of the SIGKILL stage test`
- 06-core.log / brn-threads-intake: `test process::tests::process_tree_child ... ignored, subprocess witness only`
- 06-core.log / brn-threads-intake: `test process::tests::renderer_probe_child ... ignored, subprocess witness only`
- 06-core.log / brn-threads-intake: `test process::tests::sandbox_probe_child ... ignored, subprocess witness only`
- 07-core.log / brn-ai: `test provider_formats_tests::provider_stderr_tests::provider_stderr_child ... ignored, only the parent test launches this isolated synthetic stderr fixture`
- 14-native-ui.log / brn-desktop: `test threads_ui::intake_tests::a_desktop_supply_reuses_the_chat_a_cli_useful_intake_left ... ignored, needs BRN_DWR_SHARED_DATA, BRN_DWR_SHARED_SOURCE and BRN_DWR_SHARED_CHAT from a CLI useful intake`
- 16-native-retrieval.log / brn-threads-app: `test native_check_store_with_a_killed_intake_and_its_candidate ... ignored, writes a store for the owner's native check; set BRN_DWR_NATIVE_DATA and BRN_DWR_NATIVE_MODE`

## Failures and qualification boundaries

- Default workspace exit 101: `prepared_rust_fixtures_keep_source_and_preview_bindings_in_owned_scratch` at `qualification.rs:470`, `Pdf preparation failed: Unavailable`. Native `pdftoppm` missing; explicit native `pdftotext` and `pdfimages` are present. No wrapper/fake renderer and no fixture suppression. This is a local environment gap, not a production fix.
- Exact-baseline hosted macOS native-widget failure remains a separate pre-existing failure even though the same local command passes (306 passed, 1 existing ignored, 3 filtered). One local pass cannot establish cause or a flake rate.
- Linux core is hosted evidence only. No local Linux/Windows/native-observation/live-provider/model qualification claimed.
- Native retrieval `model_download` cases refuse occupied/cancelled installs without fetching. Model directories explicitly empty; no model installation or evaluation performed.
- Existing `block 0.1.6` future-compatibility warning is retained in native logs; strict lint exits passed.
