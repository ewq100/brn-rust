# Round 6 native re-check (owner decisions 1–3)

This check is for the orchestrator, under the Mac lock on the coordination board (`.worktrees/coordination/BOARD.md`, outside Git). It replaces the C and F parts of the [round 5 re-check](../round5/native-recheck.md); part D there is unchanged and appears again as B below. Everything runs offline: `HOME` and `CODEX_HOME` point at empty folders, so no provider call is possible. Each part uses a fresh copy, so the two source stores stay untouched.

The source stores are `/private/tmp/brn-dwr-data/r4-native-resume/data` and `r4-native-settings/data`. Each holds a killed intake Run (`Working`, fence 1) in "Information from b1-source", an unlocked lease file in `derived/run-leases/`, and an unretired candidate "Crane arrival — reported date". Part D builds its own store with the CLI.

## Setup

```sh
cd /Users/evokessler/repos/brn-rust/.worktrees/daily-workflow-reliability
export PATH=/opt/homebrew/opt/rustup/bin:$PATH CARGO_TARGET_DIR=/tmp/brn-dwr-target
cargo build -p brn --locked --offline
cargo build -p brn-desktop --features native-ui --locked --offline
R=/private/tmp/brn-dwr-data/r6-native   # must not exist yet
mkdir -p "$R/home" "$R/codex" "$R/tmp"
export HOME="$R/home" CODEX_HOME="$R/codex" TMPDIR="$R/tmp/"
cp -Rp /private/tmp/brn-dwr-data/r4-native-resume/data "$R/a"
cp -Rp /private/tmp/brn-dwr-data/r4-native-settings/data "$R/b"
```

Inspect a store only after BRN has quit. Use `brn_inspect` from [evidence.md](../evidence.md#owner-manual-check) (it reads a scratch copy), `cand` from the [round 5 procedure](../round5/native-recheck.md#setup), and this chat-history query (`$1` is the data folder, `$2` the chat ID):

```sh
revs() { local c; c="$(mktemp -d "$R/tmp/q.XXXX")"; cp -p "$1"/threads.sqlite3* "$c"/; \
  sqlite3 -header -column "$c/threads.sqlite3" "SELECT version, json_extract(body,'\$.archived') AS archived, json_extract(body,'\$.data.Thread.attention') AS attention FROM revisions WHERE id='$2' ORDER BY version;"; rm -rf "$c"; }
```

## A: Archive of a killed chat adds its Review but no notice (decision 2)

```sh
/tmp/brn-dwr-target/debug/brn-desktop --data-dir "$R/a"
```

1. Open "Information from b1-source" and click its Archive pill. This is `resolve-thread`, which calls `archive_chat`. Don't click Resume first: Resume recovers the orphan, and that recovery adds the notice.
2. *Expected on screen:*
   - The status line reads "Chat archived. Work stopped and the queue is paused. Actions retain their progress."
   - The chat is reachable under NEEDS YOU with **only** the review card "Review prepared changes": proposal "Crane arrival — reported date", with Show in note, Dismiss and Apply.
   - There is **no** "Run interrupted. Continue starts fresh." card.
   - The run line reads "Interrupted".
   - Record how this build offers Unarchive. The UI branch makes it the single next step.
3. *Round 5 build, for comparison:* the review card plus the blocker card.
4. Close BRN with its window button, then run `brn_inspect "$R/a"; cand "$R/a"`.
   - *Expected in the store:*
     - the Run is `Interrupted` at fence 2;
     - the chat is archived, and its attention is exactly one Review (`record` = the candidate operation), with no Blocker;
     - the candidate row has `retired` 0;
     - `derived/run-leases/` is empty.

## B: a settings stop reads Interrupted and keeps the notice

```sh
/tmp/brn-dwr-target/debug/brn-desktop --data-dir "$R/b"
```

1. On Home, click "Review changes first".
2. *Expected on screen:*
   - In "Information from b1-source": the review card and the blocker card "Run interrupted. Continue starts fresh.".
   - The run line reads **"Interrupted"**.
   - The status line still shows "Saved. Owner configured workspace and stopped prior invocations" (U8, changed only on the UI branch).
3. Leave BRN open for C.

## C: Continue after a crash clears the notice when the Run starts (decision 1)

This continues in the window from B.

1. In "Information from b1-source", click Continue ("Continue with current context").
2. Offline, the new Run fails at once with `ReconnectNeeded`, so the moment between its start and its failure is too short to see. *Expected on screen after it ends:*
   - The review card is still there.
   - There is exactly one blocker card: "Run failed: The provider account needs reconnecting. Continue when the provider is ready.".
   - There is no "Run interrupted" card.
   - The run line reads "Failed".
3. Close BRN, then run `brn_inspect "$R/b"; cand "$R/b"; revs "$R/b" <intake chat ID>`. The chat ID is in `brn_inspect`'s attention rows.
   - *Expected in `revs`:*
     - The settings choice wrote attention `[Review, Blocker "Run interrupted…"]`.
     - The **next** revision, written by "Start a fresh invocation", is `[Review]` only. The notice cleared when the Run started.
     - The last revision, written by "Record terminal run result", is `[Review, Blocker "Run failed: …"]`, with the Blocker naming the new Run.
   - *Expected in the store:*
     - the old Run is `Interrupted` at fence 2, and the new Run is `Failed`;
     - the candidate is unretired;
     - `derived/run-leases/` is empty.

## D: search finds an archived chat's Message, flagged (decision 3)

```sh
B=/tmp/brn-dwr-target/debug/brn; F="$R/d/data"
$B --data-dir "$F" init
$B --data-dir "$F" configure --provider Codex --model gpt-6.1-sol --effort medium --auth-file "$R/d/missing-auth.json"
T=$($B --data-dir "$F" thread-start --title Replies | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
$B --data-dir "$F" ask --thread "$T" --prompt "Berth survey reply draft 1.4 m"   # Failed:ReconnectNeeded; the owner Message is saved
$B --data-dir "$F" archive --thread "$T"
$B --data-dir "$F" search --query berth   # message_total 1; messages[0].chat_title "Replies", "archived": true
/tmp/brn-dwr-target/debug/brn-desktop --data-dir "$F"
```

1. Open Notes and type `berth` in "Filter titles and search note text".
2. *Expected with this build (F-UI1):* only "No current note text matches. Try another word or clear the filter to browse.", with no status line and no result row. The service answer carries the Message with `archived: true`. The "In chats" group with its Archived label is built on the combined branch. Record what appears.
3. Close BRN, then run `$B --data-dir "$F" unarchive --thread "$T"` and search again: the same hit, now with `"archived": false`. Round 5 builds gave `message_total` 0 while the chat was archived.

Not covered natively: the owner's Stop of a lease-free Run (decision 2 applied to Stop). On this branch the desktop Stop acts only on its own live worker, which holds a lease. The UI branch's Stop of an unattached Run reaches it, and `killed_run_candidate.rs` covers it in-process.

## After

Close BRN, then confirm that no `brn-desktop` process remains (`pgrep -fl brn-desktop`). Once the results are recorded, delete `$R` and keep the two `r4-native-*` stores. Remove the Mac lock file and post on the board.
