# Development setup

**Document conversion, 18 October 2026:** local Tasks1–3 implement [stock Xberg default, optional BRN layer and independent AI conversion defaulting Off](../work/active/modular-foundation/x2/standard-default/feature-brief.md). Native Settings → Documents and the checked CLI command select subsequent imports; existing notes are unchanged. [Task3 evidence](../work/active/modular-foundation/x2/standard-default/task3/handoff.md) separates synthetic verification from pending Task4 distribution, whole-feature review, native-human and integration gates. No live-provider qualification is activated.

Read [AGENTS.md](../../AGENTS.md), [status](../status.md) and the active plan linked there. Inspect local/remote state and preserve unrelated work before choosing an isolated task branch/worktree.

## Check the machine

```sh
python3 scripts/development-preflight.py --native
```

The read-only preflight reports checkout identity, Git/Bash/Python/Cargo/Rustup, the pinned compiler, protoc and relevant Mac compiler tools. Missing optional workflow skills are informational.

[rust-toolchain.toml](../../rust-toolchain.toml) is compiler authority; it pins Rust **1.98.1** with rustfmt and Clippy. [Cargo.lock](../../Cargo.lock) records dependencies. When compiler installation belongs to the machine setup assignment:

```sh
rustup toolchain install 1.98.1 --profile minimal --component rustfmt --component clippy
cargo fetch --locked
```

Keep compiler/dependency changes separately scoped and qualified. Native GPUI targets macOS Apple Silicon first and requires an unlocked graphical session for interaction checks. Linux headless checks do not establish native readiness.

## Isolate build and application data

Use a task-owned Cargo target directory or a known compatible shared cache, plus a fresh explicit absolute BRN `--data-dir` outside Git. Use synthetic material. The core checks directory marker, application ID and supported schema before opening; incompatible old BRN data refuses without migration.

Current schema **10** adds canonical document converter/AI preferences to schema9’s scoped MCP policy and host-only external-data authority semantics (schema 8 introduced skill policy/fencing and delivered identities; schema 7 introduced scoped writing configuration). CLI opens of schema 3–9 refuse unchanged with `upgrade_required`; only explicit `upgrade --backup-directory ABSOLUTE_DIR` upgrades after an external verified backup. The desktop retains backup-first open using its remembered folder. Old backups upgrade only their unpublished restore copy. Preserve the pre-upgrade backup when rolling back source: schema-9 executables refuse schema10 rather than downgrade it. Scoped skill references and selection survive SQLite recovery; local instruction bytes do not. Preserve original packages and explicitly reinstall the exact approved bytes before using a restored selection. Missing/mismatched packages stay unavailable; installation alone never enables them.

Build the two normal document helpers from the verified cached package and accepted optional bundle:

```sh
bash scripts/build-document-helpers.sh --offline --jobs 1 --target-root /private/tmp/brn-document-helpers
export BRN_XBERG_HELPER=/private/tmp/brn-document-helpers/artifacts/brn-xberg-helper
export BRN_XBERG_HELPER_BRN=/private/tmp/brn-document-helpers/artifacts/brn-xberg-helper-brn
```

Both overrides must be absolute. Without overrides, desktop/CLI resolve these two names beside their own executable. A missing or wrong selected artifact refuses without trying another converter. Task4 owns final app/CLI packaging and maintained CI lanes; a CLI-only build does not supply the helpers automatically yet.

Native Settings → Documents or `brn --data-dir /absolute/workspace document-conversion --converter xberg|xberg-brn --ai off|on` saves checked preferences. Settings inspection explicitly shows stock/false even when canonical default serialization omits the field for historical hash compatibility. Every import snapshots both values. All eight kinds, including Text and Markdown, use the chosen normal helper with no Poppler discovery. AI Off needs no conversion credentials. AI On refines the selected output with fixed GPT-6 Luna/high and the existing explicit ChatGPT auth path; general chat settings stay independent. Useful-information's subsequent agent still requires its own credentials/authority. Reports retain actual captured converter/bundle/AI provenance; keyed full/chat/project replay checks that variant before conversion/Auth.

Legacy explicit prepared-reader and restriction checks still build the maintained typed helper separately:

```sh
cargo build --locked -p brn-intake --features helper --bin brn-intake-helper
```

Those compatibility PDF checks require Poppler `pdftotext`, `pdfimages` and native `pdftoppm`, with explicit `BRN_INTAKE_HELPER`, `BRN_PDFTOTEXT`, `BRN_PDFIMAGES` and `BRN_PDFTOPPM` paths for reproducibility. They do not select the new production import route. The carried prepared-PDF Unavailable remains qualified separately from representative normal Xberg PDF success. Encrypted PDF is explicitly deferred; no new refusal implementation or verification is claimed.

Offline in-process CLI refinement qualification uses the nondefault `scripted-document-tests` alias for existing `brn-ai/scripted-runs`. Ordinary CLI/native entry points have no transport/endpoint override. Task4 must add this explicit lane to maintained verification/CI; it is not a live-provider or subprocess AI-success proof.

Provider/account checks follow the current assignment and normal authentication flow; keep credentials out of outputs. Successful builds and component tests are separate from native usability and owner acceptance.


## Local instruction packages (D3a management only)

An explicit owner command can copy an instruction/reference folder into the current workspace’s managed `skills/` configuration:

```sh
brn --json --data-dir /absolute/workspace skills
brn --json --data-dir /absolute/workspace skill-install --source /absolute/package
brn --json --data-dir /absolute/workspace skill-inspect --name document-review
brn --json --data-dir /absolute/workspace skill-remove --name document-review --digest EXACT_SHA256
```

The management filesystem backend supports macOS/Linux; other platforms retain the known commands with an explicit unavailable result and no package action. Use real absolute directories without symlink ancestors. SKILL.md is a bounded strict single-line name/description frontmatter subset, **not YAML**. Values run literally to the end of the single line; hashes are text, not YAML comments, and values are not interpreted as YAML types. Only UTF-8 instruction text and confined local Markdown/text references are supported; scripts, dependencies, binary files, symlinks and executable files are refused. See [D3a subset and evidence](../work/active/agent-capabilities/d3a/plan.md) and the synthetic [document-review example](../../agent/skill-examples/document-review/SKILL.md). Installing does not enable or load a skill; the four existing built-ins are unchanged. Different bytes under an installed name conflict; inspect the exact digest, remove it explicitly and reinstall to update.

Installed bytes are **machine-local instruction configuration excluded from current SQLite backups and Markdown exports**. Preserve original package folders separately for reinstall; BRN does not promise they are rebuildable after source loss. Ordinary reopen of that same workspace retains installed bytes. D3b must fail closed when a restored selection’s local package is unavailable; enablement, runtime frozen versions and full runtime acceptance remain later work. This stage-only exclusion does not change canonical Notes/receipt/asset backup guarantees.

Explicit built-in policy also pins the packaged instruction digest. A BRN update that changes that SKILL.md makes an older explicit binding unavailable; a selected binding refuses a run, and a bound preload is omitted. Inspect the new package with `skill-inspect --name NAME`, then explicitly use `skill-enable --id NAME --digest NEW_SHA256` (with the same `--project ID` where applicable) and select it again if wanted. Alternatively, `skill-clear --id NAME` in that same scope removes the explicit pin and restores the packaged default. BRN never changes the approved digest or re-enables a binding automatically.

D4’s first MCP backend supports one anonymous local `http://127.0.0.1:PORT/path` or `http://[::1]:PORT/path` connection. Remote/authenticated servers, DNS, redirects, proxies, executable transports and automatic retries are unsupported. Explicit owner discovery and exact known-behavior approval are separate from a transient per-invocation input review. Policy survives SQLite backup; reviewed inputs/grants/session state do not. No connection is attempted for unrelated runs without that grant. Self-reported server metadata cannot attest its implementation.
