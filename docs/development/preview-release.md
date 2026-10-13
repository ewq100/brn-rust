# BRN Threads macOS preview

`v0.1.0-preview.2` is a macOS Apple Silicon preview of the Omakase tiled workspace. Write, Chat and Solo layouts arrange resizable tiles for threads, notes, chat, linked work, Actions, What changed, Settings and compare, with a ⌘K launcher, status bar and five themes. Notes are typed in place with margin comments; AI rewrites are reviewed as tracked changes in the note and applied only as shown, with Undo. Threads resolve and reopen on reply. The download contains the native desktop, maintained intake helper and compiled runtime guides. Local writing and keyword search work offline. Configure the selected provider/account in Settings for inference; no provider credentials or embedding models ship in the archive.

Since `v0.1.0-preview.1`: the Omakase shell and in-place review ([#125](https://github.com/ewq100/brn-rust/pull/125)), GPUI Kit 0.7.3 with refreshed Markdown typography and selection/context-menu fixes ([#126](https://github.com/ewq100/brn-rust/pull/126)), and safer resumption of interrupted release drafts ([#124](https://github.com/ewq100/brn-rust/pull/124)). The storage schema is unchanged, so the same workspace opens in this version.

This is a preview with **native interaction, live-provider acceptance and broader cold-restart/recovery acceptance still deferred or pending**. Builds and deterministic state tests establish their recorded scope, not daily-use acceptance. Use disposable material until those journeys have been accepted. No Windows, Intel Mac or Linux release is supplied.

## Install and workspace

Use an Apple Silicon Mac running **macOS 15 or newer**. Qualification covers the macOS 15 arm64 runner; older macOS versions and other architectures have not been accepted. Extract the versioned ZIP and move `BRN Threads.app` to your chosen application folder. It has an ad-hoc signature for integrity, with no Developer ID certificate or Apple notarization; macOS may require explicit approval through its normal security UI. The signature does not establish publisher identity.

The Finder launcher supplies an explicit absolute workspace path on your Mac:

`~/Library/Application Support/BRN Threads Preview/workspace`

That location is stable across preview versions. The core creates a fresh workspace and validates its marker, application identity and supported schema on each open. Foreign/nonempty unmarked directories, symlink paths and marked stores with missing storage are refused; existing data is never migrated or copied from another BRN installation. Startup errors are written to `~/Library/Logs/BRN Threads/startup.log` and shown in TextEdit.

DOCX, EML and PPTX intake use the bundled helper. PDF conversion requires the external Poppler executables `pdftotext` and `pdfimages`; install Poppler separately if needed. Finder lookup checks common Homebrew prefixes as well as the inherited path. Explicit `BRN_PDFTOTEXT` and `BRN_PDFIMAGES` overrides remain authoritative. Scanned/complex/math PDF fidelity remains unqualified, and any partial import coverage stays visible.

## First-use provider setup

The desktop preview uses existing credentials and has no sign-in dialog. For the Codex / ChatGPT route, sign in with your ChatGPT account through the installed Codex CLI's normal browser flow (`codex login`). BRN requires a file-based subscription login: OpenAI's [credential-storage instructions](https://learn.chatgpt.com/docs/auth#credential-storage) describe `cli_auth_credentials_store = "file"` in Codex configuration, which stores `auth.json` under your configured `CODEX_HOME` (normally `~/.codex`). A login stored only in the OS keyring cannot be read by this preview. Keep the authentication file private; enter its path, never its contents.

Open **Settings → Provider and model**. Select **Codex / ChatGPT**, enter your chosen account-supported **Model** identifier, choose **low**, **medium** or **high** reasoning effort, and enter the full absolute path to your existing subscription `auth.json` in **Codex auth file** (expand `~` to your actual home directory). Click **Save settings**. BRN reads the selected file in memory; it refuses API-key accounts, unsafe/symlink credential paths and missing/expired credentials without another-account fallback. Refresh expired authentication through Codex's normal login flow. The preview does not qualify the provider merely because settings saved successfully.

For **GitHub Copilot**, click **Connect GitHub Copilot…**. BRN opens GitHub's device sign-in page, copies the one-time code to the clipboard and stores the resulting credentials in the **Copilot credentials folder** (default `~/.config/brn/copilot` when the field is empty), then selects Copilot and loads the models your Copilot plan offers. Choose one from the composer's model menu (**Load GitHub Copilot models** refreshes the list). BRN reloads the list at startup and will not start a Copilot run with a model the list does not offer. The **Codex auth file** applies only to Codex / ChatGPT, so it can stay set. Live provider acceptance remains deferred.

Separately choose **Working knowledge → Maintain ordinary notes** or **Review changes first**. Saving provider settings does not grant maintenance authority. Fresh workspaces remain pending until this policy is chosen; protected-note changes continue to require their applicable instruction or exact review.

## Receipts and reproduction

Release assets are the ZIP, its `.manifest.json` and `SHA256SUMS`. The manifest records exact source SHA, version, locked dependency hash, selected features, binary hashes, linked libraries, signing status and qualification limits. The archive also contains `Contents/Resources/build-manifest.json`. Verify downloaded files against `SHA256SUMS`; checksums are integrity receipts, not publisher authentication.

On an Apple Silicon Mac with the [repository setup](setup.md), build from an approved clean commit:

```sh
export MACOSX_DEPLOYMENT_TARGET=15.0
cargo rustc --release -p brn-desktop --bin brn-desktop --features native-ui --locked --offline -- -C link-arg=-mmacosx-version-min=15.0
cargo rustc --release -p brn-intake --features helper --bin brn-intake-helper --locked --offline -- -C link-arg=-mmacosx-version-min=15.0
python3 scripts/package-macos-preview.py \
  --binary "$PWD/target/release/brn-desktop" \
  --intake-helper "$PWD/target/release/brn-intake-helper" \
  --source-sha "$(git rev-parse HEAD)" --output-dir "$PWD/dist-preview"
python3 scripts/tests/test_macos_packaging.py --qualify-app "$PWD/dist-preview/BRN Threads.app"
```

Packaging requires Python 3.11+, an absent output directory and Apple's `lipo`, `otool`, `codesign` and `ditto`. Both binaries must be arm64-only and link exclusively to `/usr/lib` or `/System/Library`. Packaging signs each binary and the bundle ad hoc, verifies it, creates the ZIP, extracts it to another location and verifies its signature again. It never opens the native app. Fixture qualification runs only headless startup/foreign-data refusal and a bounded synthetic helper conversion.

## Publication gate

[The preview workflow](../../.github/workflows/preview-release.yml) runs after `BRN CI` completes on main. GitHub's standard `macos-15` runner is [Apple Silicon](https://docs.github.com/en/actions/reference/runners/github-hosted-runners). [`workflow_run`](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#workflow_run) can run even when an informational platform job fails and grants its own narrowly declared token permissions. The workflow never weakens or changes the CI jobs or branch protection.

Publication requires a same-repository main push whose exact SHA is still current main, a changed and enabled [approved version manifest](../../release/preview.json), and successful completion of all four [required CI jobs](verification.md#ci-and-integration) in that exact run attempt. Overall CI failures remain visible. No preview is inferred merely from an ordinary main commit.

The workflow builds from that SHA and uses only its ephemeral `GITHUB_TOKEN`, with `actions: read` and `contents: write`; build commands receive no token environment or persisted checkout credentials. Release creation uses the [official releases API](https://docs.github.com/en/rest/releases/releases), and assets are checked against the [reported SHA-256 digest and size](https://docs.github.com/en/rest/releases/assets). Publication first creates an unpublished draft. After all uploads verify, it rechecks current main, CI and the full approved manifest identity immediately before publishing the prerelease. A mismatch leaves the draft unpublished. The workflow never overwrites assets, retargets tags or replaces a published release. A retry may resume only an exact matching draft and matching uploads.
