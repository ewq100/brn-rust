# Round 5 native re-check (slices C, D, F)

**Superseded for builds from round 6 on** (owner decisions of 16 October 2026). Use the [round 6 re-check](../round6/native-recheck.md): Archive no longer adds the notice (C), and search keeps an archived chat's Message, flagged (F). D is unchanged and is part B there.

For the orchestrator or the owner. Everything here is offline: `HOME` and `CODEX_HOME` point at empty folders, so no provider call is possible. Run it outside the agent sandbox, with no other agent driving the Mac. Each check uses a fresh copy, so the source stores stay untouched.

The two round 4 stores are the source for C and D: `/private/tmp/brn-dwr-data/r4-native-resume/data` and `r4-native-settings/data`. Each holds a killed intake Run (`Working`, fence 1) in "Information from b1-source", an unlocked lease file in `derived/run-leases/`, and an unretired candidate "Crane arrival — reported date". F builds its own store with the CLI.

## Setup

```sh
cd /Users/evokessler/repos/brn-rust/.worktrees/daily-workflow-reliability
export PATH=/opt/homebrew/opt/rustup/bin:$PATH CARGO_TARGET_DIR=/tmp/brn-dwr-target
cargo build -p brn --locked --offline
cargo build -p brn-desktop --features native-ui --locked --offline
R=/private/tmp/brn-dwr-data/r5-native   # must not exist yet
mkdir -p "$R/home" "$R/codex" "$R/tmp"
export HOME="$R/home" CODEX_HOME="$R/codex" TMPDIR="$R/tmp/"
cp -Rp /private/tmp/brn-dwr-data/r4-native-resume/data "$R/c"
cp -Rp /private/tmp/brn-dwr-data/r4-native-settings/data "$R/d"
```

Inspect a store only after BRN has quit, with `brn_inspect` from [evidence.md](../evidence.md#owner-manual-check) (it reads a scratch copy) and this candidate query:

```sh
cand() { local c; c="$(mktemp -d "$R/tmp/q.XXXX")"; cp -p "$1"/threads.sqlite3* "$c"/; \
  sqlite3 -header -column "$c/threads.sqlite3" "SELECT o.retired, o.host_key FROM candidate_targets t JOIN operations o ON o.id = t.operation;"; rm -rf "$c"; }
```

## C: archiving a chat whose Run died adds the notice

```sh
/tmp/brn-dwr-target/debug/brn-desktop --data-dir "$R/c"
```

1. Open "Information from b1-source" and click its Archive pill (`resolve-thread`, which calls `archive_chat`). Do not click Resume first; Resume would recover the orphan through the round 4 path instead.
2. *Expected on screen:* the chat stays reachable under NEEDS YOU (round 4 UI finding) with the review card "Review prepared changes" (proposal "Crane arrival — reported date", Show in note / Dismiss / Apply) and the blocker card "Run interrupted. Continue starts fresh.". The run line reads "Interrupted".
3. *Round 4 build, for comparison:* the review card only, no notice, and the lease file kept.
4. Optional: click Resume. "Questions" dispatches once and ends with its own provider Blocker; the archived chat gains nothing.
5. Close BRN with its window button, then run `brn_inspect "$R/c"; cand "$R/c"`.
   - *Expected in the store:* Run `Interrupted` at fence 2; the intake chat's attention is exactly Review then Blocker; the candidate row has `retired` 0; `derived/run-leases/` is empty.

A CLI dry run of steps 1 and 5 (`brn archive --thread <chat>` on a copy) gave exactly this store state during round close.

## D: a settings stop reads Interrupted

```sh
/tmp/brn-dwr-target/debug/brn-desktop --data-dir "$R/d"
```

1. On Home, click "Review changes first" (either choice works).
2. *Expected on screen:* the same review and blocker cards as C in "Information from b1-source", and the run line reads **"Interrupted"**. The status line still shows "Saved. Owner configured workspace and stopped prior invocations" (U8, unchanged).
3. *Round 4 build, for comparison:* the run line read "Cancelled" (U9), and the lease file stayed.
4. Close BRN, then run `brn_inspect "$R/d"; cand "$R/d"`.
   - *Expected in the store:* Run `Interrupted` at fence 2, one Review and one Blocker, the candidate unretired, `derived/run-leases/` empty. The CLI equivalent (`brn pause` on a copy) gave exactly this during round close.

## F: a saved chat message in search

```sh
B=/tmp/brn-dwr-target/debug/brn; F="$R/f/data"
$B --data-dir "$F" init
$B --data-dir "$F" configure --provider Codex --model gpt-6.1-sol --effort medium --auth-file "$R/f/missing-auth.json"
T=$($B --data-dir "$F" thread-start --title Replies | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
$B --data-dir "$F" ask --thread "$T" --prompt "Berth survey reply draft 1.4 m"   # Failed:ReconnectNeeded; the owner Message is saved
$B --data-dir "$F" search --query berth   # message_total 1, messages[0]: chat_title "Replies", role "owner"
/tmp/brn-dwr-target/debug/brn-desktop --data-dir "$F"
```

1. Open Notes and type `berth` in "Filter titles and search note text".
2. *Expected with this build (F-UI1):* only "No current note text matches. Try another word or clear the filter to browse.", with no status line and no result row. The service returns the Message in `page.messages`; the desktop does not render it yet. Record what appears, because the UI branch will change it.
3. Close BRN. `$B --data-dir "$F" archive --thread "$T"`, then `search --query berth`: `message_total` 0. `unarchive --thread "$T"`, then search again: the hit returns.

## After

Close BRN and confirm no `brn-desktop` process remains (`pgrep -fl brn-desktop`). Delete `$R` once the results are recorded. Keep the two `r4-native-*` stores.
