# Development tooling

Use [workflow](workflow.md) and [verification](verification.md) for task scope and gates. Helpers report evidence; they do not supply provider access, settings authority or release permission.

## Checkout and gate records

```sh
python3 scripts/development-preflight.py --native
```

Read-only preflight reports checkout identity, required tools, pinned compiler and Mac prerequisites. Optional local skills are informational. It changes no global configuration.

```sh
mkdir -m 700 /private/tmp/brn-disposable-evidence-example
export BRN_VERIFY_OUTPUT_DIR=/private/tmp/brn-disposable-evidence-example
bash scripts/verify-end-to-end.sh --retirement-only
```

Choose a fresh absolute disposable output parent outside Git, already present and without symlink components. Supported storage, end-to-end and desktop-shell gates create one exclusive child containing an atomic `record.json` and combined `output.log`. Identity includes commit/tree or dirty-content snapshot, exact gate/features, start/end UTC, terminal state and exit code. Preserve retained evidence and avoid editing relevant paths during a gate.

For direct focused commands, retain command/environment, source identity, exit status and output during execution too. Reconstructed transcript excerpts must disclose missing/truncated output and retrospective source bindings; they are not original raw logs.

Passed, failed, interrupted and running are distinct. SIGINT/SIGTERM are forwarded to the gate group. An uncatchable kill can leave running, which is incomplete. `checkout_changed=true` invalidates the initial identity as proof for the final tree. Use synthetic inputs; keep private/provider payloads out of these logs.

Reuse a result only with an explicit unchanged relevant-path/environment comparison and label it reused. Documentation-only changes need documentation checks; relevant code/manifests/features or unresolved failures require fresh checks. Evidence records do not decide relevance themselves.

## Exact CI summary and protection

```sh
python3 scripts/ci-summary.py run --run RUN_ID --attempt ATTEMPT --commit FULL_COMMIT
python3 scripts/ci-summary.py compare-logs /absolute/old.log /absolute/new.log
```

The read-only summary validates the exact requested run/attempt/commit, paginates jobs and distinguishes required Mac/shared jobs from informational Windows/non-Mac probes. Missing, pending, cancelled, skipped or failed applicable jobs cannot prove success. Unknown jobs require inspection. Overall failures remain visible even when required contexts pass. Documentation/tooling is checked when present; old absence is not retroactive success.

An exact-head PR run can take time to queue; absence is pending evidence, not a pass or reason to dispatch duplicate runs. Log comparison preserves ordering, compiler/assertion errors and symbols, normalizing only presentation timestamps/ANSI, explicit process IDs and numbered backtrace addresses. Equivalent failures remain failures.

Main protection requires these unchanged contexts:

- `Core and CLI (ubuntu-24.04)`
- `Core and CLI (macos-15)`
- `Native UI build and state (macos-15)`
- `Native retrieval and combined build (macos-15)`

Read actual protection and exact candidate checks before integration; preserve strict/current protection and visible platform failures. Historical unprotected-main configuration proposals are archived, not setup instructions.

## Documentation gate

```sh
python3 scripts/check-markdown-links.py
python3 scripts/check-markdown-links.py --all
python3 -m unittest discover -s scripts/tests -v
```

The default checks tracked and non-ignored new current Markdown files, local targets/fragments, reference links, encoded paths, duplicate/Unicode heading anchors and explicit HTML anchors. Code fences and inline-code samples are excluded; external URLs are not fetched. Missing files/fragments produce a nonzero result.

Only outgoing links under `docs/archive/` are historical exemptions. Incoming current links to archive files/fragments always validate. `--all` audits outgoing archive links too. CI checks Markdown, synthetic tooling tests and shell syntax on PR/main/manual events. See the [archive index](../archive/README.md) for exact deleted-history provenance.

## Retained qualification and packaging helpers

`brn-ai`'s optional `capability-spike` examples retain bounded provider fixtures. The synthetic catalog path reads no credentials/files and makes no calls:

```sh
cargo run -p brn-ai --features capability-spike --example provider-capabilities \
  --locked --offline -- --synthetic-catalog
```

Synthetic envelope/count/wire-order observations do not qualify live discovery or model availability. Real discovery returns processed model options; raw provider transport observation requires a separately scoped interface. Live examples follow the current assignment's provider authority.

`scripts/qualification/threads-runtime/live-journeys.py` remains the current bounded runtime journey harness. The retired standalone Rig comparison remains at its fixed baseline and in [historical results](../archive/threads-rebuild/rig-decoder-comparison.md).

For a native qualification artifact, preserve the source-bound original and copy the app plus its matching helpers. Read-only executable copies retain execute permission (for example0555); verify both hashes and executable access before marking the set ready to launch. A matching hash with0444 permissions is a retained file, not a runnable executable.

Local app packaging/launcher helpers remain maintained; packaging is separate from signing/release. [Size reporting](size-report.md) measures explicitly supplied paths, not installed-size savings from source cleanup.
