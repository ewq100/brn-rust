# Development setup

## Check the machine

```sh
python3 scripts/development-preflight.py --native
```

The read-only preflight reports checkout identity, Git/Bash/Python/Cargo/Rustup, the pinned compiler, protoc and relevant Mac compiler tools.

[rust-toolchain.toml](../../rust-toolchain.toml) is compiler authority; it pins Rust **1.98.1** with rustfmt and Clippy. [Cargo.lock](../../Cargo.lock) records dependencies. To install it:

```sh
rustup toolchain install 1.98.1 --profile minimal --component rustfmt --component clippy
cargo fetch --locked
```

Native GPUI targets macOS Apple Silicon first and requires an unlocked graphical session for interaction checks. Linux headless checks do not establish native readiness.

For actual native Linux interaction in the saved cloud environment, follow the
[Linux display, build and acceptance workflow](linux-cloud-ui.md). It uses the
existing GPUI backend with Xvfb/Openbox and software Vulkan; Mac qualification
remains separate.

## Isolate build and application data

`bash scripts/setup-worktree.sh` prepares a checkout: it fetches locked crates, builds the document helpers once per helper change into a shared cache (`~/.cache/brn/document-helpers`, override with `BRN_HELPER_CACHE`), copies them beside the CLI and writes `.brn-dev/env.sh` with the helper paths, a resolved `TMPDIR` and an ignored synthetic data folder (`BRN_DEV_DATA_DIR`). T3 Code runs it on worktree creation through `t3.json`. The manual steps below are what it automates.

Use a task-owned Cargo target directory or a known compatible shared cache, plus a fresh explicit absolute BRN `--data-dir` without symlink ancestors. Use synthetic material. The core checks directory marker, application ID and supported schema before opening; incompatible old BRN data refuses without migration.

Current schema is **13**. CLI opens of schema 3–12 refuse with `upgrade_required`; `upgrade --backup-directory ABSOLUTE_DIR` upgrades after a verified external backup, and the desktop backs up before upgrading. Older executables refuse newer schemas rather than downgrade, so keep the pre-upgrade backup when rolling back source. Restored skill selections need their original packages reinstalled before use.

Build the two normal document helpers from the verified cached package and accepted optional bundle:

```sh
bash scripts/build-document-helpers.sh --offline --jobs 1 --target-root /private/tmp/brn-document-helpers
export BRN_XBERG_HELPER=/private/tmp/brn-document-helpers/artifacts/brn-xberg-helper
export BRN_XBERG_HELPER_BRN=/private/tmp/brn-document-helpers/artifacts/brn-xberg-helper-brn
```

Both overrides must be absolute. Without overrides, desktop/CLI resolve these two names beside their own executable. A missing or wrong selected artifact refuses without trying another converter. The app packagers require both artifacts and place them beside the desktop under Resources/bin. A CLI-only cargo build still needs these siblings or the two explicit absolute overrides. CI fetches both committed standalone locks during setup, then uses the offline builder and explicit helper/scripted lanes.

Native Settings → Documents or `brn --data-dir /absolute/workspace document-conversion --converter xberg|xberg-brn --ai off|on` saves checked preferences. Settings inspection explicitly shows stock/false even when canonical default serialization omits the field for historical hash compatibility. Every import snapshots both values. All eight kinds, including Text and Markdown, use the chosen normal helper with no Poppler discovery. AI Off needs no conversion credentials. AI On refines the selected output with fixed GPT-6 Luna/high and the existing explicit ChatGPT auth path; general chat settings stay independent. Useful-information's subsequent agent still requires its own credentials/authority. Reports retain actual captured converter/bundle/AI provenance; keyed full/chat/project replay checks that variant before conversion/Auth.

The superseded typed reader, prepared-page AI protocol and Poppler tooling are retired together. No old helper or renderer is needed for builds/imports. Historical failed PDF results remain historical; representative normal Xberg PDF success does not establish scanned/complex fidelity or resource proof. Encrypted PDF remains explicitly deferred, without a new refusal implementation claim.

Offline in-process CLI refinement qualification uses the nondefault `scripted-document-tests` alias for existing `brn-ai/scripted-runs`. Ordinary CLI/native entry points have no transport/endpoint override. Maintained verification/CI includes this explicit lane; it is not a live-provider or subprocess AI-success proof.


## Local instruction packages

A command can copy an instruction/reference folder into the current workspace’s managed `skills/` configuration:

```sh
brn --json --data-dir /absolute/workspace skills
brn --json --data-dir /absolute/workspace skill-install --source /absolute/package
brn --json --data-dir /absolute/workspace skill-inspect --name document-review
brn --json --data-dir /absolute/workspace skill-remove --name document-review --digest EXACT_SHA256
```

The management filesystem backend supports macOS/Linux; other platforms retain the known commands with an explicit unavailable result and no package action. Use real absolute directories without symlink ancestors. SKILL.md is a bounded strict single-line name/description frontmatter subset, **not YAML**. Values run literally to the end of the single line; hashes are text, not YAML comments, and values are not interpreted as YAML types. Only UTF-8 instruction text and confined local Markdown/text references are supported; scripts, dependencies, binary files, symlinks and executable files are refused. See [D3a subset and evidence](https://github.com/ewq100/brn-rust/blob/3e2f9f71526f974389784554f159d8f0c36cb6c6/docs/work/active/agent-capabilities/d3a/plan.md) and the synthetic [document-review example](../../agent/skill-examples/document-review/SKILL.md). Installing does not enable or load a skill; the four existing built-ins are unchanged. Different bytes under an installed name conflict; inspect the exact digest, remove it explicitly and reinstall to update.

Installed bytes are **machine-local instruction configuration excluded from current SQLite backups and Markdown exports**. Preserve original package folders separately for reinstall; BRN does not promise they are rebuildable after source loss. Ordinary reopen of that same workspace retains installed bytes. D3b must fail closed when a restored selection’s local package is unavailable; enablement, runtime frozen versions and full runtime acceptance remain later work. This stage-only exclusion does not change canonical Notes/receipt/asset backup guarantees.

Explicit built-in policy also pins the packaged instruction digest. A BRN update that changes that SKILL.md makes an older explicit binding unavailable; a selected binding refuses a run, and a bound preload is omitted. Inspect the new package with `skill-inspect --name NAME`, then explicitly use `skill-enable --id NAME --digest NEW_SHA256` (with the same `--project ID` where applicable) and select it again if wanted. Alternatively, `skill-clear --id NAME` in that same scope removes the explicit pin and restores the packaged default. BRN never changes the approved digest or re-enables a binding automatically.

D4’s first MCP backend supports one anonymous local `http://127.0.0.1:PORT/path` or `http://[::1]:PORT/path` connection. Remote/authenticated servers, DNS, redirects, proxies, executable transports and automatic retries are unsupported. Explicit owner discovery and exact known-behavior approval are separate from a transient per-invocation input review. Policy survives SQLite backup; reviewed inputs/grants/session state do not. No connection is attempted for unrelated runs without that grant. Self-reported server metadata cannot attest its implementation.
