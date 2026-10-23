# Native Linux UI in the saved cloud environment

BRN uses the existing pinned GPUI Linux backend. The saved Debian 13 x86_64
environment has Xvfb, Openbox, XTEST (`xdotool`), ImageMagick and Mesa llvmpipe
Vulkan. A hardware GPU, browser preview, Mac screen and provider account are not
required. This is a real desktop window; GPUI test-platform tests remain a
separate kind of evidence.

## Build

Inspect Git state and fetch current upstream before selecting an isolated branch.
The saved environment's old startup notes and binaries are not current-source
evidence. In each shell:

```bash
cd /workspace/brn-rust
source /workspace/brn-environment/activate.sh
python3 scripts/development-preflight.py --native
# Only when the current lockfile's packages are missing from the saved cache:
cargo fetch --locked
cargo build -p brn-desktop --features native-ui -p brn --locked --offline --jobs 1
```

The activation uses retained rootless Debian libraries, the pinned Rust toolchain,
software Vulkan and both document helpers. Do not change the lockfile to match an
old cache. Missing system tools can be restored with the saved environment's
`install-vacation.sh`; review its actions before running it. No system security
configuration or credentials are needed for the offline journey below.

## Display and application

Use an unused display number, and retain each PID so only this task's services
are stopped. Do not reuse or stop another task's desktop. Services must be
started again after restoring a filesystem snapshot. These commands assume the
activation above and a fresh log directory:

```bash
set -euo pipefail
brn_logs=$(mktemp -d /workspace/brn-environment/logs/native-ui.XXXXXX)
export DISPLAY=:98
if xwininfo -root >/dev/null 2>&1; then
  echo "Display $DISPLAY is already occupied; choose another number." >&2
  exit 1
fi
Xvfb "$DISPLAY" -screen 0 1280x900x24 -nolisten tcp > "$brn_logs/xvfb.log" 2>&1 &
brn_xvfb_pid=$!
for brn_attempt in {1..50}; do
  kill -0 "$brn_xvfb_pid"
  if xwininfo -root >/dev/null 2>&1; then break; fi
  sleep 0.1
done
kill -0 "$brn_xvfb_pid"
xwininfo -root
XDG_DATA_DIRS="$BRN_NATIVE_PREFIX/usr/share:/usr/share" \
  openbox --sm-disable --config-file "$BRN_NATIVE_PREFIX/etc/xdg/openbox/rc.xml" \
  > "$brn_logs/openbox.log" 2>&1 &
brn_wm_pid=$!
vulkaninfo --summary
brn_trial=$(mktemp -d /tmp/brn-linux-native.XXXXXX)
"$CARGO_TARGET_DIR/debug/brn" --json --data-dir "$brn_trial/data" init --purpose test
"$CARGO_TARGET_DIR/debug/brn-desktop" --data-dir "$brn_trial/data" \
  > "$brn_logs/desktop.log" 2>&1 &
brn_app_pid=$!
timeout 30s xdotool search --sync --onlyvisible --name '^BRN$'
import -window root "$brn_logs/window.png"
```

Allow a settled frame before capturing; finding the X11 window alone does not
prove that it has painted. Openbox supplies focus and close behavior that bare
Xvfb does not. The application title is deliberately just `BRN`, never a data
path or note title. The display is local with TCP disabled; do not add `-ac`,
VNC, remote ports or persistent access as a setup workaround.

## Native acceptance and separate portable checks

With actual pointer/keyboard input, create a synthetic note, edit its body,
Save, close the app with Alt-F4, relaunch the same binary/data, and open the note
from Notes. Capture the saved and reopened windows. Use the CLI only to inspect
the saved note/recovery, never to repair the UI outcome. Record source identity,
binary hashes, exact text and screenshots. Linux application shortcuts currently
use Super (`cmd` in GPUI), including Super+N and Super+S; some help text still
uses Mac glyphs. Control+A is the Linux text widget's select-all shortcut.

Run the relevant component aggregate separately:

```bash
python3 /workspace/brn-environment/run-with-reaping.py cargo test \
  -p brn-desktop --features native-test-support --bin brn-desktop --locked --offline --jobs 1
cargo clippy -p brn-desktop --features native-test-support --all-targets \
  --locked --offline --jobs 1 -- -D warnings
cargo fmt --all -- --check
git diff --check
python3 scripts/check-markdown-links.py
```

The saved activation's TMPDIR is a private `/var/tmp` fixture parent used by
credential-path tests, which refuse Git ancestors. If sandbox permissions deny
that directory, request the supported narrow filesystem permission; do not
weaken validation or count a refused test as passing. `/tmp` suffices for the
ordinary synthetic app workspace above.

After closing the app normally, stop only the recorded window-manager/display
PIDs. Retain the synthetic workspace and evidence for inspection.

Linux observation does not replace Mac close/menu/accessibility, IME, clipboard,
file-picker, signing or hardware-GPU qualification. Offline note editing does
not qualify live AI, retrieval-model installation, all product journeys or
owner acceptance.

## Linux accessibility and exact Unicode input

The retained task setup at `/workspace/brn-environment/a11y` supplies rootless
signed Debian AT-SPI GI and xclip packages. After normal activation, source
`/workspace/brn-environment/a11y/activate.sh` before launching the app. Its isolated
session bus is task-local; do not change host security settings or reuse another
machine's credentials. `query.py dump` reads actual platform nodes;
`query.py action --name EXACT --role ROLE` requires one unambiguous actionable
node. A successful action receipt alone is not acceptance: verify the resulting
settled view and persisted state. Some controls change from button to toggle
button after initial rendering; inspect the current tree.

Use clipboard paste for exact UTF-8/multiline fixtures. `xdotool type --file`
dropped Unicode/newlines here. Wait for copy/paste and rendering before reading
clipboard or taking evidence. This proves clipboard recovery, not IME input.
See the [full deferred inventory](../work/active/linux-cloud-native/deferred/README.md)
for portable results and original Mac-only gaps.

## File workflows and portal admission

Native file selection uses `org.freedesktop.portal.FileChooser` on the app's
session bus. A display alone does not supply a chooser. The saved task-local
setup under `/workspace/brn-environment/portal` contains signed Debian
`xdg-desktop-portal` and its GTK backend, extracted without a host install.
Its README documents activation and task-owned processes. Check actual caller
admission and a real dialog before claiming native import or backup-folder tests.

This environment's separate sandbox process boundaries currently produce
`AccessDenied: Portal operation not allowed: Unable to open /proc/<pid>/root`.
Do not spoof portal identity or disable validation. Obtain authority for a
legitimate setup correction before retrying denied access. CLI imports/restores
and opening restored content are useful separate checks, not picker acceptance.

For ordinary synthetic document conversion in a sandbox where the saved
activation's `/var/tmp` fixture parent is unavailable, set `TMPDIR=/tmp` for the
app/CLI process. This uses an already writable temporary directory; it does not
change credential-path tests, validation or host permissions. Keep conversion
AI off and choose full-note import for offline verification. The
[next batch evidence](../work/active/linux-native-gaps/README.md) records exact
results, source revisions, upstream accessibility gaps and remaining blockers.
