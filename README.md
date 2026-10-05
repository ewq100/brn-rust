# BRN Rust desktop

BRN is a local-first macOS notes application in development, with Markdown
editing/recovery, saved-note search, explicit Rig ChatGPT/Copilot chat and a shared
AppWorker for native and CLI consumers. Stages 1–4 are integrated, including whole
proposal review, exact approval and recoverable application. Stage 5 knowledge
foundations and Stage 6 Actions/dashboard foundations are integrated; native,
live-model and owner qualification remain open. Stage 7 Inbox is active. Complete
V1 delivery is not claimed.

The production workspace uses the frozen six crates; legacy production paths
and App Server are removed. Existing old data stays untouched. Integrated main
is `48941d3ae2c16dd014b6cb0f69a01b8c4ef60fa0` (PR60). Recoverable original-copy
lifecycle work is implemented on a separate, unmerged review branch; the
[status](docs/status.md) distinguishes that snapshot from current corrections.

Start with [current status](docs/status.md) for implemented capabilities and qualification gaps. Agents should read [AGENTS.md](AGENTS.md); the [documentation index](docs/README.md) routes setup, architecture, testing and task history.

The original TypeScript BRN and vault remain separate. This is a private trial repository; no license or public release decision has been made. The frozen [target architecture](docs/architecture/overview.md#frozen-target) serves the [product vision](docs/product/BRN_PRODUCT_VISION.md); the [roadmap](docs/roadmap.md) defines the remaining outcomes and prerequisites. The [vault format](docs/architecture/vault-format.md) describes durable Markdown and managed metadata. The former publication/graph-engine roadmap is retired.

## Quick start

Use the Rust toolchain pinned in [rust-toolchain.toml](rust-toolchain.toml):

```sh
cargo build --workspace --locked
cargo run -p brn -- --help
bash scripts/verify-end-to-end.sh
```

Set `TMPDIR` to an explicit existing disposable synthetic fixture parent outside
Git before verification (credential safety refuses repository-local paths).
`brn` is the owner-operated shared-workflow CLI with the owner's full command
authority; see its [reference](crates/brn/README.md). Its approval, Save,
completion and removal commands are not standing authorization for an agent.
Future external agents use read/propose capabilities unless the owner explicitly
delegates more authority, through the same workflow boundary.
The integrated script uses cached dependencies and synthetic current-vault
Save/recovery/read/search fixtures, without accounts or model assets. `--fixtures-only` avoids repeating a completed workspace gate;
`--retirement-only` checks production references without builds.
See [verification](docs/development/verification.md) for native-specific checks.

## Native personal trial

Create an explicit disposable data directory outside Git repositories, then
launch the simple note editor/chat UI:

```sh
trial_data="$HOME/BRN-disposable-trial"
mkdir -p "$trial_data"
cargo run -p brn-desktop --features native-ui,native-retrieval --locked -- --data-dir "$trial_data"
```

The normal native desktop includes `native-ui,native-retrieval`; enabling the
installer does not download a model. Choose a vault, read/refresh/search saved
Markdown and explicitly select an account/provider/model in Settings before Ask.
Account Connect and model Download require explicit actions; offline trials
must not click them. Declining model consent creates no network request and
keeps keyword-only retrieval. Open a synthetic note for small corrections;
Save/Cmd-S explicitly writes Markdown. Automatic recovery and guarded close
preserve acknowledged unfinished edits in BRN without saving the vault file.
Compare/reload, exclusive Save Copy and uncertain-save reconciliation are local.

Without an explicit data directory, native startup uses
`~/Library/Application Support/BRN-simple`, with `BRN-simple.credentials` next to
it. Old/mixed authority markers, including sidecars/backups, refuse before SQLite
opens. Retired flags and commands are unknown. No old data is copied or migrated.

Simple Save detects changed file/root identities and preserves exact UTF-8 bytes,
including BOM, line endings and frontmatter. Conflicts retain recovery text;
missing originals are not recreated. Acknowledged recovery is distinct from
Markdown Save, and force termination can lose unacknowledged typing. User suitability, IME/accessibility and broader
qualification remain open as recorded in [status](docs/status.md).

## Local Finder launcher

After building, create a new unsigned local app bundle with explicit paths:

```sh
cargo build -p brn-desktop --features native-ui,native-retrieval --locked
bash scripts/make-macos-app.sh \
  --output /absolute/path/BRN-Trial.app \
  --binary /absolute/path/to/target/debug/brn-desktop
```

Use the actual `CARGO_TARGET_DIR` binary if sharing a build cache. The output bundle must not already exist. The launcher copies only the binary, never old data. Omit `--data-dir` for the
new BRN-simple default, or supply an existing disposable absolute directory.
An optional `--model-dir` is an explicitly
verified existing model, not a download destination. This is a local trial launcher, not a signed distribution. [Usability evidence](docs/work/completed/desktop-usability/evidence.md) records its tested behavior.
