# Saved chat Messages in owner search (round 5 slice F)

Owner search (`Workspace::search_page`, the CLI `search` and the desktop's `request_page`) now returns final `owner` and `assistant` chat Messages as their own list, beside note hits. Each hit carries its chat ID, chat title, role, version and a snippet. The derived `search.sqlite` moves to schema v3 with separate `chats`/`messages` tables and their own exact and stem FTS lanes. No note lane, count or embedding job reads them, and a v2 file is rebuilt and backfilled on the next sync. Core gains an owner-search scope (`MessageScope`, `message_scope`, `message_scopes_for`, `Snapshot::searchable_messages`) that re-checks every hit, and the agent's `search_notes` goes through a notes-only request. There is no `threads.sqlite3` schema, `main.rs`, `HELP`, exit-code, desktop production, `docs/status.md` or `Cargo.lock` change. Hits carry no sent or delivered state, and every new test has a recorded fail-before.

Watch for: the core re-check runs one full scan of the Notes table for each distinct chat among the hits. A spot-check measured about 7 ms per chat at 10,000 notes, and `user_scale` has no Messages, so the recorded 22.9 ms p95 does not cover this path (likely). Messages in a Resolved project chat show in project search, but that chat's own notes do not (confirmed, harmless). The doc on the public `SearchHandle::search` still says agent runs use it (confirmed, doc only).

**Verdict**: APPROVED

## High-level view

The index's per-chat scope tag (`general`, `project:<id>`, `chat:<id>`, `hidden`) is only a prefilter. Core `searchable_messages` decides every hit from fresh rows, so a stale tag can hide a Message or skew `message_total` but cannot show a Message outside its scope. Sync rescopes chats on Thread, Project, chat-note and Keep-in-BRN changes (`doc_chats` reads a note's previous `chat:` tag before the apply). F-A5 shows why the prefilter has to stay current: without the Thread rescope, 100 archived hits fill the 100-hit window and hide the one visible Message.

The scope follows note visibility, including project-only chats, knowledge-off, private provenance, Archive and Trash/Restore. One exception: `message_scope` ignores `ThreadState`, so a Resolved project chat's Messages show in project search while its chat-only notes do not. The difference stays inside the project.

The new cost is in the re-check. `message_scope_rows` scans every Note once per distinct chat among up to 200 candidates. That work happens on the search worker, so the UI does not block, but agent `search_notes` calls queue behind it on the same worker. The desktop also requests Messages on every notes search without rendering them (F-UI1).

The project Delete refusal ("run control changes must advance the fence") that the verification reports is pre-existing. The check exists at `771116f0`, and no round 5 commit touches `projects.rs` or that check.

<details>
<summary>Issues (3)</summary>

1. **Per-chat Notes scan in the message re-check** (likely; non-blocking): `Snapshot::message_scope_rows` runs one `json_extract(body,'$.data.Note.chat')=?` scan of every record per distinct chat. A query whose 100–200 candidates span many chats can add 0.3–1.4 s at 10,000 notes, and the desktop pays this on every keystroke search without rendering Messages. Collect all chats' notes in one scan (`… IN (SELECT value FROM json_each(?))`), and add a `user_scale` case with matching Messages across many chats so the p95 covers this path.
2. **Resolved project chats differ from their notes** (confirmed; non-blocking): `message_scope` ignores `ThreadState`, so a Resolved project chat's Messages show in project search, while `scope_allowed` hides that chat's chat-only notes there. No boundary is crossed. Either note the difference in the proposed `threads-behavior.md` sentence and the scope table, or align the two rules.
3. **Stale "agent runs" doc on owner search** (confirmed; doc only): `SearchHandle::search` is still documented as "Blocking search for the CLI and agent runs", but it, `request` and `request_page` now return owner Message results. Say these are owner-only and that agent hosts must use the `RunNoteSearch` impl.

</details>

<details>
<summary>Details</summary>

### Prefilter tags and the core re-check

```
sync(): changed Message ──► upsert/remove in messages (+FTS triggers)
        changed Thread / chat Note / doc_chats / Message ──► rescope those chats
        any Project change ──► records() once, rescope every chat
search: message_scopes(context) ─► tags ─► message_hits (2 lanes, RRF, ≤100/lane)
        ─► searchable_messages(ids, context)   [authority, fresh rows]
        ─► version check (retry on mismatch) ─► page at the notes' offset
```

The one documented gap fails closed. A chat-only note that was never indexed (for example archived) and then loses its `chat` leaves the chat tagged `chat:<id>`. Its Messages stay hidden from general search until the chat's next Thread or Message change.

### Cost of `searchable_messages` at scale

`message_scope_rows(&chats)` runs this query once for each distinct chat among the ranked candidates, up to 200 after RRF fusion of two 100-row lanes:

```sql
SELECT body FROM records WHERE json_type(body,'$.data.Note') IS NOT NULL
  AND json_extract(body,'$.data.Note.chat')=?1
```

No expression index exists on `$.data.Note.chat`. Adding one would be a `threads.sqlite3` schema change, which the slice rules out. The planner walks the `context_import_source` partial index, which still visits every Note. As a narrow spot-check, I ran Python `sqlite3` 3.53.4 in memory with 10,000 Note bodies of about 1.8 KB plus 200 Threads and 4,000 Messages. It measured 7.2 ms for 1 chat, 69 ms for 10, 342 ms for 50 and 683 ms for 100. The SQLite build differs from the bundled one, so the absolute numbers are approximate, but the cost grows linearly with the number of chats. `user_scale.rs` creates no chats or Messages, so its recorded "query p95 22.9 ms (owner path, with the message lanes)" ran with empty lanes and no re-check scans. One scan for all chats removes the per-chat factor without any schema change.

### Resolved project chats

`message_scope` returns `Project(p)` for any non-archived member chat. For notes, `scope_allowed` admits another project chat's content only when that Thread is `Open`. So in project search, a Resolved project chat's Messages appear and its chat-only notes do not. This probably helps the owner find old replies, but it contradicts the "mirrors notes" wording in the code docs, the verification scope table and the proposed architecture sentence.

### Test coverage

The 22 new tests (8 core, 3 retrieval, 9 app, 1 CLI, 1 desktop) cover every scope case in the plan, plus crowding, Keep in BRN and agent isolation. Each cited fail-before line lands on the named assertion. Not tested: Messages across many chats at scale (see above), a Resolved project chat, a Message edited or archived after indexing at the app level (covered only in retrieval and core), and Trash and Restore of a project chat that has a Run. The last one is blocked by the pre-existing Delete refusal, which the slice records for its own fix.

</details>

<details>
<summary>File map</summary>

- `crates/brn-threads-core/src/reader.rs`: `MessageScope`, `SEARCHABLE_MESSAGE_ROLES`, `message_searchable`, `message_scope`, `message_scopes_for`, `Snapshot::{message_scope_rows, message_scopes, searchable_messages}`.
- `crates/brn-threads-core/src/context.rs`: `chat_private` extracted; `chat_has_private_context` unchanged in behavior.
- `crates/brn-threads-core/src/lib.rs`: re-exports.
- `crates/brn-retrieval/src/search_index.rs`: schema v3 message/chat tables, FTS lanes and triggers, `ChatScope`, `MessageInput`, `MessageHit`, `Changes` fields, `message_hits`, `message_count`, `doc_chats`.
- `crates/brn-threads-app/src/search.rs`: sync of Messages and chat scopes, `message_search`, `MessageMatch`, `SearchResults` fields, `notes_request` for agents.
- `crates/brn-threads-app/src/lib.rs`: `SearchPage.{messages, message_total, message_cursor}`, `ChatMessageHit`.
- Tests: `brn-threads-core/tests/message_search_scope.rs` (new), `brn-retrieval/tests/search_index.rs` (+3), `brn-threads-app/tests/search_messages.rs` (new), `brn/tests/search_messages.rs` (new), `brn-desktop/src/threads_intake_tests.rs` (+1).
- `docs/work/active/daily-workflow-reliability/round5/verification.md`: slice F section.

Full diff: `git -C /Users/evokessler/repos/brn-rust/.worktrees/daily-workflow-reliability show f5872774`.

</details>
