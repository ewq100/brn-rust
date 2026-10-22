# Native app artifact preflight (read-only)

**Recommendation:** use the maintained local `scripts/make-macos-app.sh` around one fresh, pinned, offline `brn-desktop` native-ui build, with the already source-qualified stock/adapted helper artifacts as explicit inputs. Do not use the preview-release packager for this local MAC-13 session. No build, packaging, workspace, app launch, process/desktop access, or security change was performed here.

The inspected checkout is `/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust`, HEAD `f28a164f393348cb8b6e3878af90816aba8c928e`. Its only observed working changes at the end were lead-owned `campaign.json`, `overview.md`, and two MAC-13 attempt-2 evidence files; I changed none. Read-only `git diff --quiet` comparisons returned 0 for the full runtime source path set against product source `3827d6a13953533c7322fd7157d975a32dee5cb2`, and for helper sources against qualified helper-source baseline `3b3f9135c318a4a7959e591f2c0774b8177f9266`. Thus the documentation-only HEAD does not change the selected product/helper source. Native availability remains deferred; this report does not authorize launch.

## Minimal future artifact path

After reconciling current main/source and the approved MAC-13 data receipt, the shortest existing build is the pinned ordinary desktop binary only:

```sh
rustup run 1.98.1 cargo build -p brn-desktop --bin brn-desktop --features native-ui --locked --offline --jobs 1
```

This reuses the checkout's existing Cargo target cache and does not fetch dependencies. The current `target/debug/brn-desktop` exists (165,987,488 bytes, modified Oct 22), but has no inspected source-bound build receipt, so its timestamp/file presence is not enough to adopt it. Preserve that pre-existing output and all previous app/crash artifacts before a future build overwrites the default target path; do not `cargo clean`. Record the exact checkout/source identity, command outcome and produced binary hash. `scripts/verify-desktop-shell.sh` includes `native-test-support`, lint/tests, and broader gates; it is not needed just to produce an ordinary production binary and is not part of this artifact-only preflight.

The existing CLI build receipt `evidence/mac13-cli-build.json` is for source `3827d6a…`, not the desktop binary. It independently records matching helper siblings and hashes:

- `/Users/evokessler/.codex/artifacts/brn-mac13-3827d6a1/brn-xberg-helper` — `4f46c81f3e649b4f86bd4a66c109fc86889e3fefac83b8777d9956662e74a36b`
- `/Users/evokessler/.codex/artifacts/brn-mac13-3827d6a1/brn-xberg-helper-brn` — `24662cd03614003e42687d7ec070c3e219a0ccbf65d38c2bf1160457fbce9ccc`

The receipt binds the helper install/hash receipts and says their sources were unchanged to the product baseline; the live read-only source comparison also returned equal. Recheck the actual helper files against the receipt before packaging. The CLI itself is not needed beside a native app.

Then create one fresh absent local artifact path with the existing explicit-data wrapper, supplying the selected already-prepared synthetic root and the exact build/helper paths:

```sh
bash scripts/make-macos-app.sh \
  --output /absolute/task-artifacts/BRN-MAC13-Private.app \
  --binary /absolute/checkout/target/debug/brn-desktop \
  --xberg-helper /absolute/path/brn-xberg-helper \
  --xberg-helper-brn /absolute/path/brn-xberg-helper-brn \
  --data-dir /private/tmp/<approved-private-fixture>/data
```

The script creates an **unsigned local** `.app`, copies the three supplied executables under `Contents/Resources/bin`, and writes a launcher with that absolute `--data-dir`; it refuses a pre-existing output and does not create or migrate the data directory. It does not launch or sign the app. Its synthetic test (`scripts/test-make-macos-app.sh`) checks path quoting, exact explicit data-dir forwarding, collision refusal and visible failure, but was not run here. The packager itself does not establish source identity: hash the produced desktop binary and both helpers before and after copying; retain a small adjacent receipt with checkout/source, command, file paths, hashes, and explicit `native_observed:false`/`owner_accepted:false`. Read-only executable artifact copies should remain executable (the tooling guide gives `0555` as an example).

For a later authorized session, invoke only the newly frozen wrapper's exact `Contents/MacOS/BRN-Threads` path with a fresh task-owned `BRN_LAUNCH_LOG_DIR`, after checking native availability and reconciling the current source/artifact and data receipt. This avoids Finder's implicit portable workspace and avoids sharing the default startup log. Do not start it now; do not touch PID 63750, wrapper 63737, or `/private/tmp/brn-native-20261020T134854Z/writing-review`.

`package-macos-preview.py` is not the minimal route: it is for release packaging, expects version/manifest/notices and `lipo`/`otool`/`codesign`/`ditto`, ad-hoc signs and archives. It does not help preserve the task's explicit local data route and would add release-signing machinery without authorization.

**Limit:** The current target binary was not source-bound; this preflight selects a future exact-source build rather than declaring a runnable artifact ready. The attempt-2 data and resulting app path were not opened or altered; whether that fixture receives lead/Sol approval remains separate. No claim is made that the Mac is unlocked or that a future app can reach the stale-copy state.
