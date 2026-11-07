# BRN Threads macOS alpha

Release candidate `v0.2.0-alpha.1` / build 3 is the first command-enabled Apple Silicon alpha. It combines the native tiled workspace, Rig 0.46, GitHub Copilot chat/web/Luna refinement, and the manual versioned updater. The qualified runtime/source checkpoint is `82657cabbf763739c03b0e479c4b343a0fee54e0`, including 264/265, auth repair272, provider refactor267 and the focused new-article review-target fix; release receipts identify the actual approved manifest/source commit. The package includes stock Xberg 1.3.10, the existing BRN-adapted helper and compiled runtime guides. No extra Copilot CLI/SDK binary, provider credential or embedding model ships. Local writing and keyword search work offline.

A model HTTP response is bounded to 20 minutes from send and 64 MiB of streamed bytes; exceeding the limit fails with “A model response reached BRN’s time/size limit. Continue starts fresh.” Stop remains immediate. Provider 403 messages say to check account access or reconnect. Account/model entitlement and policy restrictions stay explicit; BRN does not silently change providers/accounts.

The current schema is 13. Older stores require the existing backup-first upgrade; older executables refuse newer schemas. Notes, projects, history, assets and editor recovery remain canonical data outside the app bundle. Restore preserves drafts for the desktop owner and revokes tokens that controlled the original copy.

**Qualification status:** source82657cab has seven passing hosted checks and completed clean code/security receipts. Separately identified local A3/B4 packages passed platform/signatures, relocated helpers, headless local update and backup checks. Alternate-directory native Copilot tools/citations, genuine web search/Off/Stop, Luna refinement On, source-linked consolidation and referenced writing Apply/Undo/stale refusal passed. Normal quit/reopen on this new source awaits an unlocked screen; earlier-source native recovery/backup results remain separately scoped. Ordinary Finder/clean-user, actual browser/Gatekeeper, published stock HTTPS A→B and owner acceptance remain pending. Local update transport was substituted. Requested high effort is not proof of effective effort; actual second-account/in-flight account switch, policy/quota/absent-Luna denial, network outage and enabled helper/context-summary live paths on this new source remain unobserved. Use disposable material until native/owner acceptance; no Windows, Intel Mac or Linux release is supplied. Publication still requires explicit approval of the release manifest and the existing exact-main/required-CI/immutable-asset workflow. Qualification-source selection, a draft proposal or local ZIP does not authorize publication or invited distribution.

## Install and workspace

Use an Apple Silicon Mac running **macOS 15 or newer**. Qualification covers the macOS 15 arm64 runner; older macOS versions and other architectures have not been accepted. Extract the versioned ZIP and move `BRN Threads.app` to your chosen application folder. It has an ad-hoc signature for integrity, with no Developer ID certificate or Apple notarization; macOS may require explicit approval through its normal security UI. The signature does not establish publisher identity.

The Finder launcher supplies an explicit absolute workspace path on your Mac:

`~/Library/Application Support/BRN Threads Preview/workspace`

That location is stable across preview versions. The core creates a fresh workspace and validates its marker, application identity and supported schema on each open. Foreign/nonempty unmarked directories, symlink paths and marked stores with missing storage are refused; existing data is never migrated or copied from another BRN installation. Startup errors are written to `~/Library/Logs/BRN Threads/startup.log` and shown in TextEdit.

Settings → Documents selects **Xberg** (default) or **Xberg + BRN layer**, with independent AI conversion **Off** by default. Both supplied helpers cover PDF, DOCX, PPTX, XLSX, EML, HTML, Text and Markdown through the normal pinned API. Conversion Off needs no provider credentials; On uses the existing fixed dedicated conversion model after the selected helper. A missing/wrong helper refuses without fallback. Useful-information's subsequent agent still has its separate credential/authority requirement.

Both helpers live beside the desktop in `Contents/Resources/bin`; recipients need no build tree, developer override, Poppler or runtime source download. The optional copy contains only the four accepted Office patches and explicit UTF8 Text/Markdown identity projection; no new PDF/XLSX/EML/HTML fidelity repair is implied. Stock limitations remain visible. Encrypted PDF support is explicitly deferred; no new refusal or complete PDF fidelity claim is made.

## Update after a manual announcement

The source candidate bundles `brn-update`; previously published previews do not.
Install the first command-enabled version once by downloading its announced ZIP.
There are no automatic update notifications or background updater. Use only the
explicit version announced by the owner in the repository's
[releases](https://github.com/ewq100/brn-rust/releases). The examples below name
`v0.2.0-alpha.2` as a command example, not an available or approved release.

Save your work, finish or cancel active tasks, make a normal BRN backup, then quit
BRN normally. In Terminal, for an installation in `/Applications`, run:

```sh
"/Applications/BRN Threads.app/Contents/Resources/bin/brn-update" v0.2.0-alpha.2
```

For a user-owned installation in `~/Applications`, run:

```sh
"$HOME/Applications/BRN Threads.app/Contents/Resources/bin/brn-update" v0.2.0-alpha.2
```

Substitute the announced tag and your actual installation path. A non-writable
application folder is refused; install in your own `~/Applications` folder instead.
Do not use `sudo`. No Python, GitHub CLI, Rust, Homebrew or Apple signing account is
needed on the recipient's Mac.

The command displays installed/requested versions, stages its own script outside
the app, and downloads the exact tag's manifest, `SHA256SUMS` and ZIP from
`ewq100/brn-rust` over HTTPS. It checks checksums, bundle ID `dev.brn.threads`,
version/build ordering, macOS 15+, arm64 binaries, system library dependencies,
embedded receipts and the ad-hoc bundle signature before replacing the app.
Checksums downloaded alongside a ZIP detect corruption/consistency errors; neither
these checksums nor ad-hoc signing independently authenticate the publisher.

The old app and transaction files stay in the hidden `.BRN Threads.app.update-*`
directory beside the installation. Keep that path from the terminal output until
the new app and its data have been checked. The command prints a normal `open`
command; it does not automatically reopen BRN. It never force-quits BRN, resumes
an agent, changes security settings, strips quarantine, copies credentials or
touches workspace/settings files. A second updater and normal launcher refuse
while the transaction lock exists.

If a download or replacement fails, or the terminal/update is interrupted, run
the **exact staged recovery command printed by that attempt**, after its process
has exited. The lock file `.BRN Threads.app.update-lock` beside the installation
also contains that command. Recovery restores the previous bundle when the
installation path is absent; if an app is already installed, recovery retains it.
It releases only its own lock. Preserve transaction files when requesting help.
After successful recovery, retry the original versioned command if necessary.

Reopen BRN normally and check the same notes, drafts, projects, history, assets,
settings and provider selection. The existing workspace path and external Copilot
credential location remain unchanged. Expired/revoked external credentials may
need normal reconnection. macOS can require its normal per-app **System Settings →
Privacy & Security → Open Anyway** approval again after replacement; follow
[Apple's per-app guidance](https://support.apple.com/en-us/102445). A terminal
update does not guarantee bypassing Gatekeeper or managed-device policy.

The updater does not migrate or downgrade databases. BRN's checked startup owns
backup-first schema upgrade. An old bundle is usable for rollback only while the
store schema remains compatible. After migration, use a matching pre-upgrade
backup with its compatible app, or a forward fix; never open a newer store with
an old binary or overwrite current work with an old snapshot automatically.
Report problems at [the issue tracker](https://github.com/ewq100/brn-rust/issues),
including version/build, macOS version, redacted terminal error and reproduction;
exclude credentials and private workspace contents. Publication and final native
Copilot/install/update acceptance remain separate from these source instructions.

## First-use provider setup

The desktop preview uses existing credentials and has no sign-in dialog. For the Codex / ChatGPT route, sign in with your ChatGPT account through the installed Codex CLI's normal browser flow (`codex login`). BRN requires a file-based subscription login: OpenAI's [credential-storage instructions](https://learn.chatgpt.com/docs/auth#credential-storage) describe `cli_auth_credentials_store = "file"` in Codex configuration, which stores `auth.json` under your configured `CODEX_HOME` (normally `~/.codex`). A login stored only in the OS keyring cannot be read by this preview. Keep the authentication file private; enter its path, never its contents.

Open **Settings → Provider and model**. Select **Codex / ChatGPT**, enter your chosen account-supported **Model** identifier, choose **low**, **medium** or **high** reasoning effort, and enter the full absolute path to your existing subscription `auth.json` in **Codex auth file** (expand `~` to your actual home directory). Click **Save settings**. BRN reads the selected file in memory; it refuses API-key accounts, unsafe/symlink credential paths and missing/expired credentials without another-account fallback. Refresh expired authentication through Codex's normal login flow. The preview does not qualify the provider merely because settings saved successfully.

For **GitHub Copilot**, click **Connect GitHub Copilot…**. BRN opens GitHub's device sign-in page, copies the one-time code to the clipboard and stores the resulting credentials in the **Copilot credentials folder** (default `~/.config/brn/copilot` when the field is empty), then selects Copilot and loads the models your Copilot plan offers. Choose one from the composer's model menu (**Load GitHub Copilot models** refreshes the list). BRN reloads the list at startup and will not start a Copilot run with a model the list does not offer. The **Codex auth file** applies only to Codex / ChatGPT, so it can stay set; Copilot never needs Codex credentials, an API key or a terminal. No Copilot CLI or SDK binary needs to be installed: the packaged app uses its existing Rig transport.

With Copilot selected, the same signed-in Copilot account serves chat, BRN tools, helpers, **Allow web access** and document refinement. BRN records the account when each request starts; if another account signs in meanwhile, the request stops with a reconnect message instead of continuing on the other account or falling back to Codex. **Allow web access** uses Copilot's own hosted web search on the chat's selected model: BRN sends only the search query (no notes, documents or history), offers no other tool, and returns only answers backed by real cited sources. Models that do not support Copilot web search (for example Claude models) report that clearly. Each web search is one Copilot request on the chat's model and effort, so it counts toward the plan's premium-request allowance at that model's multiplier; if Copilot refuses search for the account or organisation policy, BRN says so instead of asking you to reconnect. **Refine with GPT-6 Luna** uses `gpt-6-luna` at high effort on the selected account; with refinement off, extraction never contacts the provider.

Separately choose **Working knowledge → Maintain ordinary notes** or **Review changes first**. Saving provider settings does not grant maintenance authority. Fresh workspaces remain pending until this policy is chosen; protected-note changes continue to require their applicable instruction or exact review.

## Receipts and reproduction

Release assets are the ZIP, its `.manifest.json` and `SHA256SUMS`. The manifest records exact source SHA, version, locked dependency hash, selected features, binary hashes, linked libraries, signing status and qualification limits. The archive also contains `Contents/Resources/build-manifest.json`. Verify downloaded files against `SHA256SUMS`; checksums are integrity receipts, not publisher authentication.

On an Apple Silicon Mac with the [repository setup](setup.md), build from an approved clean commit:

```sh
export MACOSX_DEPLOYMENT_TARGET=15.0
cargo rustc --release -p brn-desktop --bin brn-desktop --features native-ui --locked --offline -- -C link-arg=-mmacosx-version-min=15.0
cargo build --release -p brn --locked --offline
bash scripts/build-document-helpers.sh --offline --jobs 1 --target-root "$PWD/target/document-helpers"
cargo tree --locked --offline -p brn-desktop --features native-ui --target aarch64-apple-darwin --edges normal,build --prefix none --format '{p}' > target/desktop-target-graph.txt
python3 scripts/package-macos-preview.py --prepare-notices "$PWD/target/distribution-notices" \
  --graph target/desktop-target-graph.txt \
  --graph target/document-helpers/receipts/stock-target-graph.txt \
  --graph target/document-helpers/receipts/adapted-target-graph.txt
python3 scripts/package-macos-preview.py \
  --binary "$PWD/target/release/brn-desktop" \
  --xberg-helper "$PWD/target/document-helpers/artifacts/brn-xberg-helper" \
  --xberg-helper-brn "$PWD/target/document-helpers/artifacts/brn-xberg-helper-brn" \
  --notices "$PWD/target/distribution-notices" \
  --source-sha "$(git rev-parse HEAD)" --output-dir "$PWD/dist-preview"
python3 scripts/tests/test_macos_packaging.py --qualify-app "$PWD/dist-preview/BRN Threads.app" \
  --host-cli "$PWD/target/release/brn"
```

Packaging requires Python 3.11+, an absent output directory and Apple's `lipo`, `otool`, `codesign` and `ditto`. All three binaries must be arm64-only and link exclusively to `/usr/lib` or `/System/Library`. Packaging signs each binary and the bundle ad hoc, verifies it, creates the ZIP, extracts it to another location and verifies its signature again. It never opens the native app. Fixture qualification runs headless startup/foreign-data refusal and both relocated helpers on all eight representative typed inputs. With --host-cli, it also copies that test-only CLI beside copies of the signed relocated helpers, then imports all eight in both modes with AI Off and no developer helper overrides; the CLI is not shipped in the app. Fixture bytes remain test inputs; source/build directories are excluded from executable discovery, not deleted. The bundle retains target graphs, locks, checksum-verified registry source archives (including their legal notices), optional patches and BRN helper sources under Resources/notices.

## Publication gate

[The preview workflow](../../.github/workflows/preview-release.yml) runs after `BRN CI` completes on main. GitHub's standard `macos-15` runner is [Apple Silicon](https://docs.github.com/en/actions/reference/runners/github-hosted-runners). [`workflow_run`](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#workflow_run) can run even when an informational platform job fails and grants its own narrowly declared token permissions. The workflow never weakens or changes the CI jobs or branch protection.

Publication requires a same-repository main push whose exact SHA is still current main, a changed and enabled [approved version manifest](../../release/preview.json), and successful completion of all four [required CI jobs](verification.md#ci-and-integration) in that exact run attempt. Overall CI failures remain visible. No preview is inferred merely from an ordinary main commit.

The workflow builds from that SHA and uses only its ephemeral `GITHUB_TOKEN`, with `actions: read` and `contents: write`; build commands receive no token environment or persisted checkout credentials. Release creation uses the [official releases API](https://docs.github.com/en/rest/releases/releases), and assets are checked against the [reported SHA-256 digest and size](https://docs.github.com/en/rest/releases/assets). Publication first creates an unpublished draft. After all uploads verify, it rechecks current main, CI and the full approved manifest identity immediately before publishing the prerelease. A mismatch leaves the draft unpublished. The workflow never overwrites assets, retargets tags or replaces a published release. A retry may resume only an exact matching draft and matching uploads.
