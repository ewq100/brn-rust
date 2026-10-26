#!/usr/bin/env bash
# PREPARED ONLY: root must qualify the final integrated binaries before explicit launch.
# Synthetic Linux observation; no model confirmation, Send, picker, portal or user files.
set -euo pipefail
brn_evidence=/workspace/brn-next-evidence/native
if [[ ${1:-} != --inside-session ]]; then
  [[ $# == 5 ]] || { echo 'usage: run-outcomes.sh ABS_DESKTOP ABS_CLI ABS_FIXTURE SOURCE_COMMIT ABS_CHECKOUT' >&2; exit 2; }
  [[ $1 = /* && -x $1 && $2 = /* && -x $2 && $3 = /* && -x $3 && $5 = /* ]] || exit 2
  [[ $(git -C "$5" rev-parse HEAD) == "$4" ]] || { echo 'Checkout does not match qualified source' >&2; exit 1; }
  git -C "$5" diff --quiet
  git -C "$5" diff --cached --quiet
  # Verify final source names rather than silently accepting the preparation-time candidate.
  python3 - "$5" <<'SOURCE'
from pathlib import Path
import sys
root=Path(sys.argv[1])
writing=(root/'crates/brn-desktop/src/threads_writing.rs').read_text()
capture=(root/'crates/brn-desktop/src/threads_capture.rs').read_text()
assert '"capture-outcomes", "Capture outcomes…"' in writing
assert '"Capture useful outcomes?"' in capture
assert '"capture-outcomes-cancel"' in capture and '.label("Cancel")' in capture
assert '"Propose outcomes"' in capture
SOURCE
  export BRN_OUTCOMES_DESKTOP=$1 BRN_OUTCOMES_CLI=$2 BRN_OUTCOMES_FIXTURE=$3 BRN_OUTCOMES_COMMIT=$4 BRN_OUTCOMES_CHECKOUT=$5
  export BRN_OUTCOMES_RUN
  BRN_OUTCOMES_RUN=$(mktemp -d "$brn_evidence/outcomes.XXXXXX")
  echo "$BRN_OUTCOMES_RUN"
  export XDG_CONFIG_HOME="$BRN_OUTCOMES_RUN/config" XDG_CACHE_HOME="$BRN_OUTCOMES_RUN/cache"
  mkdir -p "$XDG_CONFIG_HOME" "$XDG_CACHE_HOME"
  exec timeout --signal=TERM --kill-after=5s 180s dbus-run-session -- bash "$0" --inside-session
fi
source /workspace/brn-batch-evidence/environment.sh
source /workspace/brn-environment/native/activate-native.sh
brn_run=$BRN_OUTCOMES_RUN
export XDG_CONFIG_HOME="$brn_run/config" XDG_CACHE_HOME="$brn_run/cache"
mkdir -p "$XDG_CONFIG_HOME" "$XDG_CACHE_HOME"
brn_app_pid= brn_wm_pid= brn_xvfb_pid=
cleanup() {
  for brn_pid in "$brn_app_pid" "$brn_wm_pid" "$brn_xvfb_pid"; do
    [[ -z $brn_pid ]] || kill "$brn_pid" 2>/dev/null || true
  done
}
trap cleanup EXIT
trap 'exit 130' INT TERM
brn_selected_display=
for brn_display in {91..110}; do
  if [[ ! -e /tmp/.X11-unix/X$brn_display && ! -e /tmp/.X${brn_display}-lock ]]; then
    brn_selected_display=:$brn_display; break
  fi
done
[[ -n $brn_selected_display ]] || { echo 'No unused display' >&2; exit 1; }
export DISPLAY=$brn_selected_display
Xvfb "$DISPLAY" -screen 0 1440x1100x24 -nolisten tcp >"$brn_run/xvfb.log" 2>&1 & brn_xvfb_pid=$!
for brn_attempt in {1..50}; do
  kill -0 "$brn_xvfb_pid"
  if xwininfo -root >/dev/null 2>&1; then break; fi
  sleep 0.1
done
xwininfo -root >"$brn_run/display.txt"
XDG_DATA_DIRS="$BRN_NATIVE_PREFIX/usr/share:/usr/share" openbox --sm-disable --config-file "$BRN_NATIVE_PREFIX/etc/xdg/openbox/rc.xml" >"$brn_run/openbox.log" 2>&1 & brn_wm_pid=$!
# Normal AT-SPI activation only on this new disposable session bus; no global configuration.
gdbus call --session --dest org.a11y.Bus --object-path /org/a11y/bus --method org.freedesktop.DBus.Properties.Set org.a11y.Status IsEnabled '<true>' >"$brn_run/a11y-enable.txt"
gdbus call --session --dest org.a11y.Bus --object-path /org/a11y/bus --method org.freedesktop.DBus.Properties.Set org.a11y.Status ScreenReaderEnabled '<true>' >>"$brn_run/a11y-enable.txt"
"$BRN_OUTCOMES_FIXTURE" --output "$brn_run/fixture" >"$brn_run/fixture.log" 2>&1
brn_data=$brn_run/fixture/workspace
assert_local_day() {
  python3 - "$brn_run/fixture/manifest.json" <<'DAY'
import datetime,json,sys
expected=json.load(open(sys.argv[1]))['local_dates']['today']
assert datetime.datetime.now().astimezone().date().isoformat()==expected, 'Local day changed; use a new fixture'
DAY
}
assert_local_day
for brn_kind in records history candidates; do
  "$BRN_OUTCOMES_CLI" --json --data-dir "$brn_data" "$brn_kind" >"$brn_run/$brn_kind-before.json"
done
printf '%s\n' "$BRN_OUTCOMES_COMMIT" >"$brn_run/source-commit.txt"
sha256sum "$BRN_OUTCOMES_DESKTOP" "$BRN_OUTCOMES_CLI" "$BRN_OUTCOMES_FIXTURE" >"$brn_run/binary-sha256.txt"
sha256sum "$0" "$brn_evidence/atspi-gio.py" >"$brn_run/observation-script-sha256.txt"
"$BRN_OUTCOMES_DESKTOP" --data-dir "$brn_data" >"$brn_run/desktop.log" 2>&1 & brn_app_pid=$!
brn_window=$(timeout 30s xdotool search --sync --onlyvisible --name '^BRN$' | head -1)
xdotool windowactivate --sync "$brn_window"
sleep 2
capture() {
  sleep 2
  import -window root "$brn_run/$1.png"
  /usr/bin/python3 "$brn_evidence/atspi-gio.py" dump >"$brn_run/$1-tree.json"
}
action() {
  # Closed allowlist: this script cannot confirm a Run or invoke any picker.
  case "$1" in Tasks|Chats|'Synthetic outcomes conversation'|'Capture outcomes…'|Cancel) ;; *) echo 'Refusing unplanned UI action' >&2; exit 2;; esac
  /usr/bin/python3 "$brn_evidence/atspi-gio.py" action --name "$1" >"$brn_run/action-$2.json"
  sleep 0.5
}
capture 01-startup
action Tasks tasks
assert_local_day
capture 02-followups
# Persist what AT reports without equating absent static text with a rendered-group failure.
python3 - "$brn_run" <<'GROUPS'
import json,sys
from pathlib import Path
root=Path(sys.argv[1]); manifest=json.loads((root/'fixture/manifest.json').read_text())
tree=json.loads((root/'02-followups-tree.json').read_text())
names=[n.get('name','') for n in tree['nodes']]
report={'expected_groups':manifest['expected_groups'],'tree_complete':tree['complete'],
        'surfaced_group_names':{label:any(label in name for name in names) for label in manifest['expected_groups']},
        'qualification':'Inspect screenshot and actual AT tree; presence alone does not prove grouping association or accessibility.'}
(root/'followups-observation.json').write_text(json.dumps(report,indent=2)+'\n')
GROUPS
action Chats chats
action 'Synthetic outcomes conversation' open-chat
capture 03-chat
action 'Capture outcomes…' capture-confirmation
capture 04-capture-confirmation
# NEVER invoke Propose outcomes. Cancel is the sole dialog action under this observation.
action Cancel cancel-capture
capture 05-capture-cancelled
python3 - "$brn_run/05-capture-cancelled-tree.json" <<'CANCEL'
import json,sys
nodes=json.load(open(sys.argv[1]))['nodes']
assert not any(n.get('name')=='Propose outcomes' and n.get('states',{}).get('showing') for n in nodes), 'Capture dialog still exposed after Cancel'
CANCEL
xdotool key --clearmodifiers alt+F4
for brn_attempt in {1..30}; do
  if ! kill -0 "$brn_app_pid" 2>/dev/null; then break; fi
  sleep 0.1
done
kill -0 "$brn_app_pid" 2>/dev/null && { echo 'Desktop did not close normally' >&2; exit 1; }
assert_local_day
for brn_kind in records history candidates; do
  "$BRN_OUTCOMES_CLI" --json --data-dir "$brn_data" "$brn_kind" >"$brn_run/$brn_kind-after.json"
done
python3 - "$brn_run" <<'UNCHANGED'
import json,sys
from pathlib import Path
root=Path(sys.argv[1])
for kind in ['records','history','candidates']:
 before=json.loads((root/f'{kind}-before.json').read_text())['result']
 after=json.loads((root/f'{kind}-after.json').read_text())['result']
 assert before==after, f'{kind} changed during read-only native observation'
 if kind=='records':
  assert not any('Run' in r['data'] for r in after), 'Unexpected Run'
(root/'invariance.json').write_text(json.dumps({'records_unchanged':True,'history_unchanged':True,'candidates_unchanged':True,'runs':0,'model_calls':'none initiated by this sequence'},indent=2)+'\n')
UNCHANGED
printf '%s\n' 'Observation sequence completed. Inspect screenshots/AT association and report its limits; not Mac or semantic acceptance.' >"$brn_run/sequence-complete.txt"
