# Linux UI testing evidence

**Date:** 15 October 2026. **Baseline:** `771116f0f5b8607bf3ffc1239da53d0e64e174f2`, Rust 1.98.1, Debian 13 x86_64 cloud environment. Branch: `linux-ui-cloud`. See [plan](plan.md) for scope. Logs are retained on the implementation host under `/workspace/brn-environment/logs/linux-ui/`.

## Environment

Cargo artifacts and installation outputs live outside the checkout in `/workspace/brn-environment`. Synthetic fixtures use `/var/tmp/brn-rust-cloud-fixtures`: this cloud platform mounts `.git` metadata at both `/workspace` and `/tmp`, and provider credential-path tests correctly reject those ancestors. No provider credentials or real user data are needed for the widget suite.

The machine does not permit system package installation as the session user. Native dependencies were installed under `/workspace/brn-environment/native/root` using official Debian HTTPS repositories, signed indexes and normal package checksum verification. A separate fresh installation and repeat both passed. Independent review matched all 57 downloaded archives to signed package indexes and verified all three InRelease signatures with the installed Debian keyring. No signature, checksum or TLS verification was disabled.

Clang 19, CMake 3.31.6, protoc 3.21.12, LLD and required pkg-config libraries passed probes. Mesa 25.0.7 exposes a CPU llvmpipe Vulkan device without a display server or hardware GPU. The saved native installer reuses a verified installation without extracting over active files. Rust/dependency declarations and the lockfile remain unchanged from the selected baseline.

## Results

| Check | Current-run result |
|---|---|
| Initial widget compilation without native activation | Failed at missing `fontconfig.pc`; activating the verified local native prefix resolved the setup failure. |
| Existing unmodified suite: `cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop threads_ui:: --locked --offline` | **141 passed, 4 failed.** All four failures were traced to Mac-only select-all fixture keys on Linux (`baseline-widgets.log`). |
| `bash scripts/verify-desktop-shell.sh --widgets-only` with `BRN_VERIFY_OUTPUT_DIR` | **145 passed, 0 failed, 1 ignored**, 2 non-UI entry-point tests filtered. Discovery found 94 feature-gated widget-module tests, including the new ignored renderer witness; the suite also includes pure UI/state tests. Actual widget execution is required by the runner (`widgets-fixed.log`). |
| Strict native desktop Clippy, all targets, `native-ui,native-test-support`, locked/offline, `-D warnings` | Passed (`clippy.log`); repeated successfully in the full desktop gate. |
| `cargo fmt --all -- --check` | Passed (`format.log`). |
| Complete saved `install-linux-ui.sh` replay | Passed: core workspace/helper build, toolchain/native validation and exact widget test binary compilation (`install-replay.log`). |
| Full `bash scripts/verify-desktop-shell.sh` with evidence recording | Helper/native desktop builds and strict lint passed; **147 passed, 0 failed, 1 ignored**, no filtered tests (`full-desktop.log`). |
| Opt-in `linux_rendered_note_edit_save_and_reopen` | **1 passed**, 0 failed/ignored. Real software rendering, Unicode input, checked Save and store reopen; **8,342 document-body pixels changed** (`rendered-02.log`, repeated in `rendered-final.log`). |
| `bash scripts/verify-end-to-end.sh --fixtures-only` | Passed fresh Save/replay, recovery, search, export, backup, desktop startup/reopen and foreign-data refusal (`smoke-final.log`). |
| Python development tooling suite | 152 tests completed: **151 passed, 1 skipped** (Apple Silicon signing witness). Includes 10 new runner regressions and the Linux CI-summary applicability check (`tooling.log`). |
| Markdown links and `git diff --check` | Passed. |

The four original failures were document Read selection/comment anchoring, clearing an old Read selection before editing, exact Save-decision text replacement, and clearing search input. The test driver now uses GPUI's native select-all and line-start keys: Command on Mac, Ctrl/Home on Linux. All existing assertions and original Mac inputs remain intact; no production command binding was changed.

The new `--widgets-only` runner rejects missing widgets, pure-unit-only discovery/execution, entirely ignored runs, and propagates Cargo failures. The full default gate retains helper/build/lint/all-desktop checks. Linux and Mac now share widget execution in the proposed PR matrix; Windows keeps its existing state lane. The local CI summary counts Linux UI as applicable; hosted branch protection has not been changed.

## Real rendering

The optional renderer test uses GPUI's real Linux text system, assets and WGPU headless screenshot readback. It starts an offline synthetic note, enters Write through the UI, dispatches Unicode input, checks that storage remains unchanged before Save, compares the common document body after blurring the caret, clicks Save, checks a newer revision and reopens the store. Screenshot dimensions/crops use the actual window scale: the first trial exposed the framework's 2× HiDPI scale, which was corrected before successful runs. This is a rendering witness rather than a golden-image comparison.

The representative [saved-note screenshot](linux-rendered-note.png) is synthetic. Its SHA-256 is `500136893b56ea8081e1e6f7e498305fd98fb3b91231454754fde88d6779db2a`. The complete three-image sequences are retained in `rendered-02/` and the current-run directory printed in `rendered-final.log`.

The renderer witness is intentionally ignored in the ordinary suite and was explicitly executed twice. The ordinary suite's single ignored result is this optional witness, not an existing widget disabled to make Linux pass. See [setup](../../../development/setup.md) for the opt-in command.

## Review and reusable setup

An independent helper approved the input/module changes, renderer/HiDPI checks, runner safeguards, CI/evidence wiring and setup integrity, and inspected the saved screenshot. It independently reran the runner regressions and examined the successful widget evidence receipt. The lead separately reviewed the small CI-summary applicability/test change authored by that helper.

Complete `install_script` and `start_skill` updates were saved in the cloud configuration draft. They include native activation, private fixture permissions, offline widget readiness and opt-in software rendering. Both install and documented startup commands were exercised. Saving the draft does not publish it; environment settings review/save and publication remain user actions. Fresh-task restoration has not been tested.

## Limits

The 145-test headless UI suite does not inspect pixels; the single rendered Linux journey is separate evidence. Mac builds/input/IME/dialogs, package signing and native converter restrictions require their existing Mac qualification. Application-wide Command shortcuts/labels and platform font selection remain future parity work. No live providers, embedding-model downloads or owner data were used. This assignment does not remove Mac gates or claim full Mac/Linux parity. Hosted CI and PR creation have not been verified; the environment denied the GitHub API read needed for PR publication.
