# Runtime and schema maintenance

State: qualified locally, observed 2026-10-23 03:25 UTC. Branch
`codex/runtime-schema-maintenance-20261023` is explicitly stacked on qualified
UI candidate `125f8ef053895e9dac1a54ecc3e8243188ccbc6c`; original UI branch
`codex/linux-native-gaps-20261023` is preserved. Upstream main was fetched and
remains `7b416591275f2552823cb18fe357f5a20617168a`. Local-only work: no new
publication, merge, release, live-account calls or Rig migration authorized.

The saved environment temporarily became unavailable. A supported shell check
failed before execution; after environment context returned, HEAD, clean status
and the UI commit object were verified. No workspace recreation or remote backup
claim was made. The previously denied portal action has not been retried.

## Scoped changes and source constraints

- Root Tokio lock 1.53.1 → 1.53.3; existing root `version="1"` constraints retained.
- Helper exact Tokio pin 1.53.2 → 1.53.3, stock and adapted locks aligned. Xberg's
  published caret 1.53.2 permits it; no third-party override or source patch.
- Direct `brn-ai` Schemars exact 1.2.2 → 1.2.3, matching derive. Helpers do not
  contain Schemars. No unrelated graph update or Tokio 1.54.0 selected.

Official [Tokio release](https://github.com/tokio-rs/tokio/releases/tag/tokio-1.53.3)
and [manifest](https://raw.githubusercontent.com/tokio-rs/tokio/tokio-1.53.3/tokio/Cargo.toml)
were inspected: runtime time/wakeup and semaphore/duplex behavior are relevant.
Official [Schemars release](https://github.com/GREsau/schemars/releases/tag/v1.2.3)
and [manifest](https://raw.githubusercontent.com/GREsau/schemars/v1.2.3/schemars/Cargo.toml)
were inspected: internally tagged enum property ordering and arbitrary-precision
schema fixes require contract tests, not blind snapshot updates.

The prior generated adapted helper passed source validation, then was preserved
at `/workspace/brn-environment/xberg-adapted-before-maintenance-20261023` before
regeneration. The generator's fail-closed source/manifest/lock checks are intact.

## Gates

Implementation: complete precise updates, five manifest/lock files, 12 lines
added and 12 removed. No app code or assertions changed. Both helper builds,
13 tests per variant (one restriction child entry point ignored and invoked by
its parent), strict helper Clippy, workspace build and strict workspace Clippy
passed. Workspace tests passed after the fixture-execution correction below:
1,308 passed, 0 failed, 5 ignored fixture/manual entry points across 106 test and
doc-test reports. Frozen schema equivalence, required-nullable fields,
pre-worker cancellation, in-flight MCP fences and process-group cancellation
passed. Offline provider feature tests also passed (500 passed, 0 failed, one child
fixture ignored), with strict Clippy. The scripted-document feature compiles and
passes strict Clippy, but its macOS-only module selects zero Linux tests: NOT RUN,
not a passing acceptance result. Linux document routes/all-eight-kind/both-engine
coverage passed in the workspace and helper suites. Native desktop tests passed
(352 passed, 0 failed, one manual shared-data
fixture ignored). Tooling ran 191 tests with one skip and no failures. Provider
and intake retirement, Markdown links, diff whitespace, offline Save/recovery/
search/export/backup/startup/refusal fixtures and the CLI journey (108 steps,
1,266 checks) passed. Production native build and strict Clippy passed.
Real-window smoke passed:
exact Unicode edit/save version 2/quit/reopen/OS close, accessible menu activation,
repeated Return/Space/Escape/pointer menu use, unchanged persisted state after
reopen and exact unrelated sentinel preservation. The disabled Rewrite action
remains unavailable, but AT-SPI still incorrectly reports enabled/sensitive;
this inherited AccessKit translation gap is not marked fixed. Synthetic CLI
help/error/headless startup also passed.

Independent source/lock review: no blocking findings for base `125f8ef0` plus
tracked diff SHA-256
`e6e1d54d6e7e137ecee288204478fb9bf818ed0d3d528ffcd45b4a15a96a1d44`.
Generated adapted lock exactly matches its committed candidate; existing schema
assertions remain intact, including ordered variants and required-nullable
fields. Final independent evidence review also found no blocking findings:
raw-log hashes, selected evidence checksums, test/skip counts, v2 persisted state,
empty recovery, unchanged sentinel and screenshot agreement were verified.
Inherited accessibility, picker and Mac-only limits remain explicit.

Publication: not authorized. Rig migration: not started; see the separate
[compatibility checklist](rig-checklist.md). A [local PR draft](draft-pr.md)
records the explicit stacked relationship. The campaign overview was regenerated
and checked without changes to its unrelated saved observations.

Logs: `/workspace/brn-environment/logs/runtime-schema-maintenance`.

## Fixture execution correction

At 02:59–03:01 UTC the first workspace test run failed with
`UnsafeCredentials`: the old temporary parent exposed `.git` metadata. It was
stopped (143) and its log preserved as
`workspace-tests-contaminated-parent-interrupted.log`. A second attempt could not
create the fresh parent from a narrow write grant (read-only parent), and the
workspace check exited 101 for missing temporary storage; its logs are retained
in `missing-fixture-parent/`. The pipeline was stopped rather than continuing
invalid tests. No passing claim attaches to either attempt.

After creating `/var/tmp/brn-runtime-schema-20261023` through supported filesystem
permission, preflight proved the sandbox injects `.git` at writable roots:
metadata exists inside that invocation but is absent from the actual new empty
directory. This is a harness boundary, not a Tokio/Schemars failure. The supported
`require_escalated` request for authorized synthetic checks was approved and the
single serial run resumed at 03:01:23 UTC. No validator, test assertion, credential,
provider budget, system security configuration or old fixture directory changed.
The earlier denied native-picker action was not retried.

## Final evidence and limits

[Reproduction commands](reproduce.md) identify features and fixture execution.
[Version inventory](evidence/versions.json), [log hashes/counts](evidence/log-receipts.json)
and [execution ledger](evidence/status.log) retain exact outcomes, including the
unsuccessful initial fixture attempts. Raw logs remain at the local path above;
`verify-storage.sh` itself was not run, its explicit constituent checks were.

The real native executable SHA-256 is
`918c6b154219dce119264b257f850abc9c2bcb572c640673e6eb53358813d004`.
[Saved screenshot](evidence/native-saved.png) and
[reopened screenshot](evidence/native-reopened.png) were inspected after settling.
[Native result](evidence/native-result.json), before/saved/reopened JSON snapshots,
and [disabled-node observation](evidence/native-disabled-observation.json) bind
visual claims to persisted and AT-SPI observations. All data is synthetic.

Not run/not established: macOS-only scripted document routes, Apple Silicon
signing witness (tooling skip), manual native fixture entries, Mac OS menu/close/
accessibility acceptance, Windows, remote CI, live accounts or Rig migration.
The denied native picker was not retried. This batch does not change the prior
picker/Restore-UI limitations or claim whole-product release readiness. The
prior UI branch is preserved separately; publication and merge remain pending
explicit authority for each branch.

Next: publish only after approval and reconcile the stack with the UI branch's
integration. A subsequent Rig migration needs a chosen official target and the
separate compatibility checklist; no migration has started.
