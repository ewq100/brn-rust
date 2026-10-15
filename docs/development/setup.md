# Development setup

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

Build the maintained intake helper before Mac workspace tests:

```sh
cargo build --locked -p brn-intake --features helper --bin brn-intake-helper
```

PDF qualification needs Poppler `pdftotext`, `pdfimages` and native `pdftoppm`. Select explicit `BRN_INTAKE_HELPER`, `BRN_PDFTOTEXT`, `BRN_PDFIMAGES` and `BRN_PDFTOPPM` executable paths for reproducible checks; shell wrappers cannot run inside the restricted renderer. The renderer uses a host-written system-font configuration and disables disposable disk font caches. [Verification](verification.md) describes gates. Default checks do not include every optional native feature.

Document imports other than Markdown/text use the existing explicit ChatGPT authentication path and a dedicated `gpt-6-luna` high-effort request. General chat model settings remain independent. No account, model or converter fallback is implicit. Local preparation/checking have separate 5-second budgets (20 seconds for PDF, within the existing 30-second hard limit); the provider request has a separate 180-second deadline. Full-page previews and originals remain transient; readable Markdown and linked figures pass through the shared checked import service. PDF layout and scan transcription retain source-coverage limitations.

Provider/account checks follow the current assignment and normal authentication flow; keep credentials out of outputs. Successful builds and component tests are separate from native usability and owner acceptance.

## Linux UI widget checks

The existing GPUI widget suite can run without a display server or GPU. It uses synthetic windows and input events, offline provider fixtures, and the same checked application service as the desktop. Native build dependencies are still required; the Linux package list is maintained in the [Rust setup action](../../.github/actions/setup-rust/action.yml).

From the repository root, after activating the pinned toolchain and any local native dependency prefix:

```sh
bash scripts/verify-desktop-shell.sh --widgets-only
```

This mode builds and runs the desktop's feature-gated widget tests with `native-ui,native-test-support`. It refuses a run with no discovered or successfully executed widget tests. It does not build the intake helper or run the full desktop lint gate; the script's default mode retains those broader checks.

Use a private synthetic `TMPDIR` whose ancestors do not contain `.git`. This includes avoiding platform metadata roots in managed environments: if both `/workspace/.git` and `/tmp/.git` exist, use a private directory under `/var/tmp` with the supported filesystem permissions. Do not relax credential-path checks to accommodate a fixture location.

Headless widget success covers component behavior, layout, focus routing and application operations. It does not prove real font rasterization, OS input/IME, clipboard, file dialogs or macOS usability. See the [Linux UI evidence](../work/active/linux-ui/evidence.md) for the selected milestone and current qualification.

The separate Linux renderer witness requires a usable Vulkan/OpenGL driver; Mesa software Vulkan can run it without a display or hardware GPU:

```sh
cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop threads_ui::rendering_tests::linux_rendered_note_edit_save_and_reopen --locked --offline -- --ignored --exact --nocapture
```

Confirm exactly one test passed. Optional `BRN_UI_SCREENSHOT_DIR` must name a new absolute directory in an existing parent; the test reserves it and refuses existing paths. The check verifies real Unicode text pixels and Save/reopen, using the actual HiDPI scale. Platform font substitution means these are Linux screenshots rather than Mac golden images.
