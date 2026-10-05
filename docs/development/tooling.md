# Development tooling

The [BRN workflow](workflow.md) remains authority. These small helpers inspect
and record existing gates; they do not configure global skills or authorize
accounts, downloads, private data, settings changes or product changes.

## Checkout preflight

```sh
python3 scripts/development-preflight.py --native
```

Reports checkout/branch/commit/tree, dirty snapshot, executable paths, installed
pinned toolchain, native compiler availability and optional local router/PR
skills. Missing required tools or the pinned toolchain exit nonzero. A missing
skill is informational: the repository workflow is portable without it.
`--router-skill /absolute/path/SKILL.md` selects another existing router path.
The default uses `CODEX_HOME` when set, then the user's `.codex` directory.
No installation or global configuration is performed. Preserve useful local
rules and explicitly supplied overrides; do not infer another Mac's setup.
On the qualified Mac mini, add `/opt/homebrew/opt/rustup/bin` to the command's
PATH if Cargo is not already visible.

## Existing gate evidence

```sh
mkdir -m 700 /private/tmp/brn-disposable-evidence-example
export BRN_VERIFY_OUTPUT_DIR=/private/tmp/brn-disposable-evidence-example
bash scripts/verify-end-to-end.sh --retirement-only
# Also supported: the existing storage and desktop-shell gates, including --native.
```

Choose a fresh explicit disposable output parent outside the checkout. It must
already exist, be absolute and have no symlink components. The gate creates
one exclusive child; it never deletes retained evidence. `record.json` is
atomically published with commit/tree or dirty-content snapshot, exact gate
argv/features, start/end UTC, terminal and exit code; `output.log` preserves
combined output. The same script executes its normal commands. Record paths
and synthetic fixture/build paths are explicit environment fields.

`passed`, `failed`, `interrupted` and `running` are distinct. SIGINT/SIGTERM are
forwarded to the gate's process group and terminal evidence is retained.
An uncatchable kill can leave `running`, which is incomplete evidence and must
never be reused as success. `checkout_changed=true` invalidates the initial
identity as evidence for the final tree. Do not run a gate while editing its
relevant code. Logs are for synthetic local checks, never private/live payloads.

Reuse a completed result only when its relevant code, manifests, lockfiles,
features, scripts and environment are unchanged. Pin the old identity and
compare named relevant paths against the candidate; state that the result is
reused. Documentation-only changes need documentation checks rather than another
Rust rebuild. A changed relevant path or unresolved failure needs fresh checks.
The snapshot records a state; it does not decide relevance for the lead.

## Exact CI summary

```sh
python3 scripts/ci-summary.py run --run 37325847448 --attempt 1 \
  --commit 966ad90e453f18413edb2bf2f9b95813f4ecc9a0
python3 scripts/ci-summary.py compare-logs /absolute/old.log /absolute/new.log
```

The read-only summary pins the requested full commit, run and attempt, validates
that identity, paginates all jobs and distinguishes the four applicable Mac/shared
PR jobs from informational Windows and non-Mac native probes. Missing, cancelled,
pending or failed applicable jobs cannot establish a pass. Unknown jobs require
review. The overall red result and failed job conclusions remain visible; a failed
or unfinished overall run exits nonzero. The documentation/tooling job is checked
when present; older runs identify it as absent rather than retroactively passing it.

GitHub can queue an automatic PR run for several minutes. No matching exact-head
run is pending evidence, never success or a reason to immediately create duplicate
manual runs. This helper never dispatches runs.

Log comparison retains line order, compiler errors, assertions and backtrace
symbols. It normalizes only ANSI presentation, GitHub ISO timestamp prefixes,
explicit `pid=` values and numbered Rust backtrace-frame addresses.
Hexadecimal assertion values remain unchanged. Differences remain a
unified diff and exit nonzero. Equivalent failures are still failures; inspect
source/features and full logs before concluding an old platform defect is unchanged.

## Documentation gate

```sh
python3 scripts/check-markdown-links.py
python3 scripts/check-markdown-links.py --all
python3 -m unittest discover -s scripts/tests -v
```

The default checks Git-tracked and non-ignored new current Markdown. It checks
local files and Markdown fragments, reference links, encoded paths, duplicate
GitHub heading anchors, Unicode headings and explicit HTML anchors. Code fences and
inline-code link samples are excluded; external URLs are not
fetched. Every missing file/fragment is reported before the nonzero exit.

Outgoing links in these explicitly historical areas are retained without making
them current CI requirements: `docs/superpowers/`, `docs/audits/`,
`docs/architecture/decisions/`, `docs/work/completed/`, `experiments/`, and the
active historical folders `rig-first-reset`, `simple-rig-notes`,
`markdown-note-editing`, `ui-slice-2-chat-polish`. **Incoming links from current
Markdown to those historical files/fragments are always checked.** The `--all`
audit reports historical failures openly. New current work, including Inbox and
knowledge foundations, remains in the default gate. CI runs this gate and the
synthetic tooling tests on every PR/main/manual event.

## Offline catalog observation

```sh
cargo run -p brn-ai --features capability-spike --example provider-capabilities \
  --locked --offline -- --synthetic-catalog
```

The existing qualification example emits built-in synthetic envelope/container
shape, raw count and exact wire-order IDs. Missing/malformed containers differ
from an actual empty catalog; invalid/duplicate IDs yield no partial ID list.
Only fixed diagnostic fields are retained. This route reads no files, credentials
or account state and makes no calls. It does not qualify Luna availability or
change the product discovery parser. **This is synthetic observation coverage,
not integration with actual live model discovery.** The example's existing live
path calls `Auth::client` and runs completion probes; it does not call
`Auth::models`.

The existing discovery API returns only `Vec<ModelOption>` (`id` and
`live_qualified`). A future authorized harness caller could retain that processed
list's count and IDs. It cannot reconstruct raw envelope shape/count: ChatGPT's
private `subscription_models` consumes the body, validates it, then filters and
sorts entries; Copilot's `list_models` exposes only decoded typed data. Transport
injection is currently private and test-only. Retaining safe raw observations
requires a separately scoped Auth/transport observation interface; this concrete
finding was returned to the lead and that part stopped without changing product
interfaces or duplicating authentication/network routes. Past failed extraction
remains historical evidence; no actual catalog or Luna availability is newly
qualified.

## Main protection proposal

Read-only inspection on 2026-10-05 found account `admin=true`, main protection
HTTP404 and no rulesets. Actual PR54 run `37325847448` attempt1 and commit check
runs confirmed these four PR contexts from GitHub Actions app15368:

- `Core and CLI (ubuntu-24.04)`
- `Core and CLI (macos-15)`
- `Native UI build and state (macos-15)`
- `Native retrieval and combined build (macos-15)`

The [exact proposed configuration](main-protection-proposal.json) requires
those checks, strict update-to-date evaluation and enforcement for administrators.
It includes no main-only or failing informational platform job. Existing workflow
jobs stay red on failure; no `continue-on-error` is added.

**The lead alone may apply settings after independent review.** This payload is
for the observed unprotected branch only. Re-read protection/rulesets immediately
before application; if protections now exist, preserve them and update only the
required-check configuration rather than sending this creation payload. No
repository setting was applied by this helper. Consider adding the new PR
Documentation/tooling check separately after its real context is verified.
