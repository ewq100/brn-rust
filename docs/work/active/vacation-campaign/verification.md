# Pilot verification and review

Production source: `8c951d64884e2c21d8fffc631cd7f88380d0973e`. Final test-only Discard fixture: `609af544545a159638d0f8c0a6523df55e7608e0`. Subsequent checkpoint commits change documentation/evidence and formatting in two CLI test files only; production code and tested logic are unchanged. Repository-pinned Rust1.98.1, locked dependencies, Linux x86_64; synthetic fixtures only. Commands below run from the repository after sourcing `/workspace/brn-environment/activate.sh`. Process checks use the setup-only `run-with-reaping.py` launcher: this container's PID1 otherwise retains dead grandchildren. It preserves command exit status, signals and timeout assertions; it does not retry tests.

| Interface / command | Observed result | Receipt |
|---|---|---|
| `cargo test --locked -p brn-retrieval --features native,fastembed/ort-load-dynamic native::download_tests` | 12 actual installer tests passed; other filtered targets executed zero tests. This is not the full retrieval suite. | `vacation-install-dynamic-tests.log` |
| Each separate helper graph: `cargo test --manifest-path MANIFEST --all-targets --no-fail-fast --locked --offline --jobs 1` | Stock13 and adapted13 passed. Each includes1 ignored internal child explicitly executed by its parent. SDK equivalence covers all eight input kinds. | `vacation-stock-helper-tests3.log`, `vacation-adapted-helper-tests.log` |
| `cargo test --locked --offline -p brn-writing-ui --features native-test-support --all-targets` | Library44 and independent-host12 passed after private payload lint corrections. | `vacation-writing-checkpoint-tests.log` |
| `cargo test --locked --offline -p brn-desktop --features native-test-support --bin brn-desktop` | 309 passed,0 failed,1 ignored; that existing ignored case is separately executed below. | `vacation-desktop-checkpoint-final.log` |
| Existing ignored desktop/CLI reuse case, exact `--ignored` run with three declared synthetic fixture variables | 1 passed,309 filtered. CLI first leaves `Failed:ReconnectNeeded`, explicitly missing auth, no provider contact; desktop reuses the same chat and instruction, adds one run. | `vacation-cli-desktop-shared-test.log` |
| `cargo test --locked --offline -p brn --test document_modes --test agent_credentials --no-fail-fast` | 9 passed: both modes/all8 kinds, replay/fallback and credential refusal. | `vacation-cli-intake-tests3.log` |
| Intake library + document modes + intake safety selected targets | 10 passed,1 internal child ignored/invoked by parent. | `vacation-intake-host-tests.log` |
| Both helper graphs `cargo clippy --all-targets --locked --offline -- -D warnings` | Passed stock and adapted. Separate source roots/targets/identities retained. | `vacation-{stock,adapted}-helper-clippy.log` |
| Writing/desktop strict all-target Clippy | Passed on final callback-fixture material. | `vacation-ui-clippy2.log`, `vacation-ui-clippy-checkpoint.log` |
| `python3 -m unittest discover -s scripts/tests -v` under the subreaper | 175 tests,1 skipped,0 failures. | `vacation-tooling-final.log` |
| Format, intake/provider retirement and Markdown links | Passed; latest link pass543 files/2463 local links. | `vacation-fmt-final.log`, `vacation-*-retirement-final.log`, `vacation-links-final.log` |
| Complete setup script: rootless signed native/display tools, locked stock/adapted helpers, production desktop/CLI and all-target UI test build | Passed end-to-end. Native/display reuse separately passed twice. Production desktop hash still matches the graphical witness. The rebuilt standard CLI separately verified the same exactv3 text/empty recovery; both CLI build hashes are retained. Duplicate native activation was removed after source/fingerprint-input review; the corrected complete repeat passed. | `vacation-setup-complete.log`, `vacation-setup-repeat.log`, binary hash receipts |
| Production graphical Linux window | **Passed** actual XTEST edit, recovery guard before Save, canonicalv3 after Save, clean OS close, graphical reopen and independent read/empty recovery. | [Final source/binary-bound receipt](evidence/linux-note-final-receipt.json), [saved](evidence/linux-note-final-saved.png), [reopened](evidence/linux-note-final-reopened.png) |

Selected raw logs, including original failures, are retained compressed under `evidence/logs/`; the manifest records uncompressed hashes. Full local logs remain under `/workspace/brn-environment/logs/`. Counts are per invocation, not unique repository coverage. Component and writing-host GPUI tests use the toolkit test platform; only the explicit production-window receipt establishes OS input/pixels.

## Independent judgments

| Scope | Reviewer / disposition |
|---|---|
| Atomic installer | Requested Luna high: source-approved occupied-target and contention behavior; lead executed real installer tests. |
| Review owner | Requested Sol high: caught partial-selection card Apply bypass; corrected common Apply ingress then source-approved. |
| Note owner | Requested Sol high: caught automatic refresh adopting another session and explicit Recover admission regressions; corrections source-approved, including private Box followup. |
| Native seccomp | Justified Astra high escalation: approved original BPF and final exact fcntl/socketpair followup after accumulator/jump/highword/thread checks. Lead executed both current graphs. |
| CI/platform/caller integration | Requested Sol high: no actionable regression in baseline through8c951d64; exact supported predicates, helper preparation, separate graphs, platform input keys and CI-summary names checked. |
| Discard fixture | Requested Sol high: approved stationary callback fixture, unchanged real clicks and Enter/Cancel/Confirm/durable assertions. Entrance interaction remains unqualified. |

Spawn controls accepted requested models/reasoning; actual runtime model/effort and speed are not independently exposed. These are bounded source reviews plus separately attributed lead execution, not owner acceptance or release approval.

## Retained failures and limits

- The default native retrieval build failed downloading pinned ONNX: Rustls UnknownIssuer; normal verified curl independently returned403. No TLS/checksum bypass, model/provider download or native-retrieval readiness claim. The pinned dynamic-loader feature exercises the unchanged real installer.
- Linux converter runtime first denied needed descriptor cloning, then Tokio's internal unnamed socketpair. Actual traces justified narrow rules; policy was reassessed and independently reviewed before continuing. External file/TCP/Unix endpoints, executable/file-backed mappings, exec and group escape remain denied. Address-space limit is process virtual memory, not a cgroup/RSS bound.
- Initial desktop299/6/1/3filtered retained four missing-helper failures, one Mac Undo key and one Stop dialog click failure. Later308/0/1 passed. Final worker admission added a case; next full run308/1/1 failed Discard Cancel closure. Unchanged exact built-binary probe passed. Source supports moving click geometry, but does not prove either intermittent failure's cause. Only the two affected resting-callback fixtures enable reduced motion; no safety assertion, real click or production animation changed, and PR161's completed repair was not reopened.
- Two early U4 canaries used a keyboard helper that redraws readonly before typing. Corrected event-order tests apply a genuine GPUI EditorState change before redraw; they are toolkit integration, not OS keyboard proof. The independent X11 journey is separately recorded.
- The queued file named `vacation-discard-dialog-baseline-probe.log` executed after609af544 and is a corrected-fixture check; only `baseline-direct.log` identifies the unchanged built-binary probe.
- Final receipt audit caught an earlier formatting check that had been reported as passed without inspecting its nonzero exit. Its diff only reordered/wrapped two CLI test files. Applied repository `cargo fmt --all` and reran the actual check; the original `vacation-fmt-checkpoint.log` is retained separately from final format evidence. No behavioral rerun is inferred from formatting.
- Initial tooling failures came from an obsolete hardcoded Cargo cache and nonreaping container PID1. Use the actual cache and subreaper; original logs retained. Initial CLI qualification lacked a cached dependency, then one harness stripped helper environment; locked fetch and explicit fixture injection fixed those prerequisites without weakening assertions.
- OS route inventory remains incomplete;25 provisional domains,2 implemented pilots,0 fully qualified under full campaign acceptance. No full workspace, hosted current Mac/Linux CI, IME, clipboard, accessibility, hardware GPU, native-human, packaging or usable release-candidate claim. Historical optional Windows/Linux retrieval failures remain open.
- Unattended continuation stays inactive: remote listing401, no actual quota telemetry or verified native goal/wake/14day hosting. No supervisor/remote job or next wake. Current-session source work is real; a saved configuration draft is not publication or a fresh-task restoration test.
