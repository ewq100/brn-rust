#!/bin/bash
# snap.sh NAME [command lines...] : apply observation commands, wait, capture.
E=/private/tmp/brn-ui-polish-design
name=$1; shift
pid=$(pgrep -f "Observe.app/Contents/Resources/bin/brn-desktop --data-dir" | head -1)
if (($#)); then printf '%s\n' "$@" > $E/observe.cmd; fi
sleep ${SNAP_WAIT:-1.5}
$E/input activate "$pid" >/dev/null
sleep 0.6
$E/wincap shot "$pid" "$E/shots/$name.png"
