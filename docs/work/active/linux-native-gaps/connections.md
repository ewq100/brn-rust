# Connections native retirement evidence

Observed 2026-10-23 on ordinary Linux native source
`7b416591275f2552823cb18fe357f5a20617168a`, before this batch's production
changes. MAC14's Linux equivalent now has an observed pre-completion retirement
frame as well as successful settled Unlink and Undo. Original macOS acceptance
remains pending. No production timing change, injected delay, scheduler hook,
provider request or personal vault was used.

The fresh synthetic fixture uses the existing `native_connections_fixture`
example: private neighbor retained in Project A and linked into Project B,
general center in B, related Link, and unrelated sentinel. The native actor opened
the center through B, retained Connections alongside Projects, and clicked the
private neighbor's actual Unlink control. The ordinary Xvfb display was recorded
continuously before input at 60 fps, 1280×900, with lossless FFV1. This observes
screen content, unlike the earlier delayed screenshots.

| Recording / one-based frame | Decoded PTS seconds | Observed native content |
| --- | --- | --- |
| First / 66 | 1.083 | [Pending](evidence/connections/unlink-pending.png): Project saved; B has one document; Connections shows Finding connections… with old neighbor and old count absent; general center remains open. |
| First / 89 | 1.467 | [Settled](evidence/connections/unlink-settled.png): same center and B; 0 of 0, Nothing with this relation; no error banner. |
| Repeat / 51 | 0.833 | [Before](evidence/connections/repeat-before.png): restored neighbor, 1 of 1, B has two documents. |
| Repeat / 133 | 2.200 | [Pending](evidence/connections/repeat-pending.png): Project saved; B has one document; old neighbor and count absent while Finding connections… is visible. |

PTS values were read with ffprobe from the original recordings, not inferred
from click time. Input logs retain monotonic before/after and wall-clock timestamps;
there is no synchronized video/input epoch. These frames establish that retirement
was visibly presented before the replacement result was displayed; they do not claim
millisecond input latency or backend execution duration. First pending-to-settled
selected frames are 0.384 seconds apart. The repeat pending frame is independently
observed; the selected repeat recording does not establish its settled transition.

The first native Changes Undo produces [settled 1 of 1](evidence/connections/connections-undo.png),
restores the exact neighbor and B's two documents. The second Unlink/Undo was also
performed. Its [Undo screenshot](evidence/connections/connections-repeat-undo.png)
is still pending, so it is **not** evidence of repeated settled restoration.
The actor subsequently quit normally, relaunched the same fixture, opened the
center through Project B's document, and captured [settled repeated restoration](evidence/connections/connections-repeat-project-restored.png):
the exact private neighbor is visible, 1 of 1. The final snapshot confirms B
version 7, restored membership and unchanged complete Note records. This resolves
the repeated settled-restoration check after relaunch; the earlier pending frame
remains preserved and is not reclassified as settled.

Opening the center from general Notes scope after relaunch had instead shown
0 of 0 (`connections-repeat-restored-settled.png` in the local logs). That is a
separate scope observation, not evidence of failed Project B restoration. The
accepted restoration screenshot follows the actor's explicit Project B document
navigation; the Notes sidebar visible in that screenshot alone does not identify
the retained document scope.

## Durable comparison

Independent comparison of all three complete Note records (private neighbor,
center, sentinel) across before, first Unlink, first Undo, repeated Undo and final
after-relaunch snapshots
passed exact structured equality, including ID, version, archived state, every
Note field, exact Unicode Markdown, original chat/provenance and import metadata.
This is not merely a rendered text comparison. The retained relation is not removed
by Project membership Unlink. Project B versions advance 3 → 4 → 5 → 6 → 7;
unlinked versions have only the center, compensated versions restore both IDs.

| Operation | Receipt | Committed UTC | Undo relationship |
| --- | --- | --- | --- |
| First Unlink | `112a4556-8672-483a-b1f5-c1aad7c6c556` | 01:55:32.651 | — |
| First Undo | `781769a6-2a2b-4b8a-8bdd-04d7e11a5d62` | 01:56:34.600 | exact first Unlink |
| Second Unlink | `66f8e8a0-2fcb-462e-b6c1-1efac97acf48` | 01:57:38.681 | — |
| Second Undo | `74d43570-b4d9-4100-9090-419654d7302b` | 01:57:43.926 | exact second Unlink |

[Raw snapshots and hash receipt](evidence/connections/receipt.json) retain the
comparison inputs, recording hashes and frame PTS. Recovery/candidate arrays stay
empty. The Notes' original history remains in the snapshots; new operations write
only Project B. No raw SQL mutation was used. Visible scoped disappearance plus
membership evidence is retained; this packet does not contain a separate direct
`record_visible_in_chat` CLI invocation.

## Reproduction

Launch the ordinary native app against a fresh synthetic fixture with the existing
cloud environment activation and display `:98`. Arrange the actual center,
Connections and Projects panes as above. Start recording before clicking Unlink:

```sh
ffmpeg -y -f x11grab -framerate 60 -video_size 1280x900 -i :98 \
  -t 4 -c:v ffv1 /tmp/connections-unlink.mkv
```

Run capture concurrently with native input; the observed actor waited approximately
one second after starting first capture and 0.7 seconds for repeat capture before
its actual xdotool click. Repeat capture used `-t 3`. Those waits are in the
external harness, not the app. Use the actual current target position, not stale
coordinates. Preserve before/after timestamps around input, source/binary identity,
and checked CLI records/history snapshots. Decode frames without an fps filter:

```sh
mkdir -p /tmp/connections-frames
ffmpeg -i /tmp/connections-unlink.mkv -fps_mode passthrough \
  /tmp/connections-frames/%04d.png
ffprobe -v error -select_streams v:0 -show_frames \
  -show_entries frame=best_effort_timestamp_time -of json /tmp/connections-unlink.mkv
```

Original recordings remain at
`/workspace/brn-environment/logs/linux-native-gaps/connections-{unlink,repeat}.mkv`;
only selected unchanged PNGs, small raw snapshots/logs and SHA256 receipt are
included in the repository. FFmpeg reported unavailable X shared-memory capture
and successfully continued without shared memory; all 240/180 frames were encoded.
No source fix is warranted: synchronous result invalidation already exists in
`refresh_connections`; this batch closes the earlier native observation gap.
