#!/bin/sh
# The converter runs in <fixture root>/scratch/threads-intake-<uuid>.
# Keep executable bytes immutable; only the parent's response data changes.
/bin/cat >/dev/null
/bin/cat ../../synthetic-response.json
