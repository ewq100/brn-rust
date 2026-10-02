# BRN: simple Rig-based notes app

Date: 2 October 2026

Status: Design sections approved by the user on 2 October 2026. Written specification awaiting user review. No implementation authorized yet.

Supersedes: the [Rig-first architecture reset specification](2026-10-01-rig-first-architecture-reset-design.md) and its [plan pack](../../work/active/rig-first-reset/plan.md). The original [reset handoff](../../architecture/brn-rig-first-architecture-reset.md) remains historical input.

## 1. Goal and scope

A simple personal app that keeps all my Markdown notes in one vault, searches them, and uses AI to help write and revise them. It is built for one user now; distribution to other people comes later (section 10).

Core rules:

1. **The vault folder is the truth for notes.** Notes are `.md` files; the app never keeps a second authoritative copy.
2. **The AI never writes to the vault.** It can only create proposals. A proposal reaches the vault only when the user approves it.
3. **The user's own notes are searchable automatically.** No per-note approval.
4. **Comments are a core feature.** They are review notes that live until the note's rewrite is approved.
5. **Provider choice is explicit.** ChatGPT subscription and GitHub Copilot subscription, both through Rig. No automatic fallback or account switching.

Out of scope: multiple vaults, sync/collaboration, coordination with other editors (NSFileCoordinator), version-history browsing (use git or Time Machine), archive/history AI tools, publication, MCP, graph retrieval, automatic conversation summarization, Keychain, Windows, API-key providers and release packaging. Section 10 lists what comes back before distribution.

## 2. Architecture

```text
brn-desktop ─┐
             ├─> brn-workflow ─┬─> brn-store      (brn.sqlite: user work)
brn CLI ─────┘                 ├─> brn-retrieval  (index.sqlite: rebuildable)
                               └─> brn-ai ─> Rig ─> ChatGPT | Copilot
```

- Existing crate boundaries stay. Their contents are replaced step by step, keeping the build green.
- `brn-provider` (Codex App Server) and `brn-core` (sample code) are deleted.
- Desktop and CLI call only `brn-workflow`. They never touch SQL, Rig or vault files directly.
- `brn-ai` is thin: provider connection, Rig agent construction, tool definitions and event mapping. It does not wrap Rig in further service layers.
- One app process owns a data folder at a time (the existing exclusive-ownership lock stays).

## 3. Data

### Vault

- One vault folder, chosen in settings.
- Only regular `.md` files up to the existing 1 MiB limit count as notes. Hidden files/folders (starting with `.`) and a top-level `archive/` folder are excluded. Symlinks are not followed.
- Known limit: a folder swapped for a symlink while a note is being read can still lead outside the vault; this needs write access inside the vault and is accepted.
- A note is identified by its vault-relative path with `/` separators. A rename made outside the app looks like a delete plus a new note.
- File bytes are preserved exactly (UTF-8, BOM, line endings, frontmatter). Non-UTF-8 files are skipped and listed as unreadable.

### SQLite files

A new data folder is used. Old data folders are left untouched.

| File | Contents | If corrupt or missing |
| --- | --- | --- |
| `index.sqlite` | `notes` (path, title, size, mtime, SHA-256), `passages`, `passages_fts`, `embeddings`, index schema version and embedding model identity | Deleted and rebuilt from the vault automatically. |
| `brn.sqlite` | `comments`, `reviews`, `review_changes`, `new_note_drafts`, `conversations`, `messages`, `unsaved_edits`, `settings` | Restored from the newest backup (below). |

`brn.sqlite` protection:

- On every start, run `PRAGMA quick_check`. If it fails, rename the file to `brn.sqlite.corrupt-<timestamp>`, restore the newest backup and tell the user which backup was restored.
- After a successful check, copy the database with SQLite's online backup API to `backups/brn-<timestamp>.sqlite`. Keep the 5 newest copies.
- Worst case after corruption: comments, reviews or chats from the current session are lost. Notes are never affected.

### Credentials

- Stored in an owner-only (`0700`) folder next to the data folder, written by Rig's own auth cache.
- The app checks the folder's owner and mode at start and refuses to use it if they're wrong.
- Never stored in SQLite, logs, test fixtures or error messages.

### Index refresh

- Scan the vault at startup, when the window gains focus, after the app's own writes and on a Refresh command.
- Use size and mtime to skip unchanged files; compare SHA-256 for the rest. Only changed notes are re-chunked and re-embedded. Removed notes are dropped from the index.
- Embedding runs in the background. The UI shows "indexing N notes". Search uses whatever is indexed so far.

## 4. Editing and saving

- Open a note, edit, press Cmd-S.
- Unsaved text is copied to `unsaved_edits` (path, base hash, text) after about 1 second of idle typing. On reopening after a crash, the app offers it back. It's cleared after Save or Discard.
- **Save:** write a temp file in the note's folder, flush it, check that the note file still has the hash it had when opened, then rename the temp file over the note. If the hash changed, or the file is gone, don't save; offer Compare, Reload (discard my edits) or Save as copy.
- **Save as copy / new files:** created without overwriting an existing file (hard-link the temp file to the destination, which fails if the destination exists).
- **Known limit, documented:** another app's change made in the instant between the hash check and the rename can be overwritten. The app does not coordinate with other editors.
- Temp files use a non-`.md` name and are removed on failure; leftovers are cleaned at startup.

## 5. Comments

- Select text in a saved note (or in a review draft, section 6) and add a comment.
- Stored: note path, the quoted text, up to 40 characters before and after it, the comment body, and status `open` or `detached`.
- **Re-finding:** when the text changes, find every occurrence of the quote. If there's exactly one, the comment stays anchored. If there are several, keep only those whose surrounding text matches; if exactly one remains, it's anchored. Otherwise the comment becomes `detached`: still visible, can be deleted, never guessed.
- Comments can't be added to unsaved text; save first.
- When a review of the note is approved, all comments on that note are deleted. Discarding a review keeps the open comments.

## 6. Review mode (AI rewrites)

The vault file does not change until the user presses Approve.

1. **Address comments** (available when the note has open comments and no unsaved edits) sends the AI the note's saved text and its open comments. The AI must answer with `propose_rewrite` (full new text). If it doesn't, the turn fails and no review is created.
2. The note switches into review mode in place. Its text becomes a **draft**: the saved note plus the AI's changes, shown inline (removed text struck through in red, added text in green).
3. The app computes the changes itself as a word-level diff between the saved note (the review's **base**) and the draft. It links each change to the comment whose anchored quote it overlaps, if any.
4. Each change has **Accept** and **Reject**. Reject puts the base text back for that change. The user can type anywhere in the draft; changes created or modified by the user's own typing are marked accepted.
5. The user can add comments anywhere in the draft, on changed or unchanged text, and press Address comments again. The AI then rewrites the current draft (rejected changes already reverted). Changes are recomputed against the base. A change identical to an already accepted one stays accepted; everything else is undecided.
6. **Approve** writes the draft through the normal save path (section 4).
   - If changes are still undecided, it asks the user to decide them or accept the rest.
   - If the note on disk no longer matches the base, it does not write. It offers **Review against current version** (the base becomes the current file, changes are recomputed, all decisions reset) or Discard.
7. After Approve, the note's comments are deleted and the review is removed. **Discard** removes the review and keeps open comments.
8. Reviews, their changes, decisions and draft comments are stored in `brn.sqlite`, so a review survives a restart. There is at most one active review per note. While a note is in review, editing happens in the draft.

**New notes from the AI:** `propose_new_note` creates a pending draft shown in the same review view. Approve creates the file. The path must be valid (section 7) and must not exist at approval time; if it does, the user picks another path.

## 7. AI

### Providers and accounts

- Settings has **Connect ChatGPT** and **Connect Copilot**, using Rig's device login. The app shows the code and the verification link, then the connected account name. **Disconnect** deletes that provider's local credentials.
- The user picks the provider and model. There is no fallback to another provider, model or account.
- Normal use only refreshes saved credentials. If they can't be refreshed, the app shows "Reconnect needed" and sends nothing. It never starts a login on its own.
- Rig version: `rig = "=0.43.0"` (or the current release at spike time), features `agent`, `derive`, `reqwest`, `rustls`. Not `sqlite` or `fastembed`: their SQLite and fastembed versions conflict with BRN's (rusqlite 0.32 and fastembed 4.5 vs BRN's 0.40.2 and 7.1).

### Chat

- A chat panel for questions about the notes. Answers stream in; **Stop** cancels.
- Each turn is saved when it ends. A stopped or failed turn is saved with status `interrupted` or `failed` and its partial text. Nothing is retried automatically.
- Conversations can be continued after a restart. Messages record provider and model.
- **History sent to the model:** earlier user questions and assistant answers only, at most the last 20 turns. Old tool calls and results (old note text) are not resent; the AI re-reads current notes when it needs them.

### Tools

Code enforces every limit; instructions only explain how to use the tools.

| Tool | Behaviour |
| --- | --- |
| `search_notes(query, limit ≤ 10)` | Hybrid search (section 8). Returns note path, byte range and quoted passage. |
| `read_note(path)` | Current note text from the vault, capped at 50 KB with a "truncated" marker. |
| `list_notes(folder?, cursor?)` | Paths and titles, at most 200 per page. |
| `list_comments(path)` | Open comments on the note (or in its active review). |
| `propose_new_note(path, text)` | Creates a pending new-note draft. Never touches the vault. |
| `propose_rewrite(path, text)` | Creates or updates the note's review (section 6). Never touches the vault. |

- Paths must be vault-relative, end in `.md`, stay inside the vault, and must not be hidden or under `archive/`. Excluded notes are invisible to every tool.
- No shell, SQL, arbitrary file access or CLI invocation.
- At most 8 tool rounds per answer; then the turn ends with a clear "tool limit reached" message.
- **Threading:** the agent runs on a Tokio runtime in a background thread. Read tools use their own read-only connection to `index.sqlite` and read files directly. Proposal tools write to `brn.sqlite` through a separate connection (WAL mode, busy timeout), so a running chat never blocks saving notes.

### Errors

Plain, typed messages: reconnect needed, rate/quota limit, network failure, model refused, invalid tool use, tool limit reached. Raw HTTP bodies, tokens and device codes never appear in messages or logs.

## 8. Search

- **Passages:** the existing chunker; each passage keeps note path, note hash, byte range and exact text.
- **Keyword:** FTS5 with the `unicode61 remove_diacritics 2` tokenizer and BM25 ranking. Query terms are quoted and combined with OR; user input is never treated as SQL.
- **Semantic:** the existing local MiniLM model (384 dimensions) through fastembed 7.1, loaded from a local folder. The app asks once before downloading it. Normalized vectors are stored as blobs in `index.sqlite`; search computes dot products in Rust over all vectors (fast enough for a personal vault). If the model identity changes, embeddings are rebuilt.
- **Hybrid:** take the top 50 from each, combine with the existing reciprocal-rank fusion (k = 60, ties broken by passage ID).
- If the model isn't installed, search runs keyword-only and the UI and tool results say so.
- LanceDB, Arrow and the `protoc` build requirement are removed.

## 9. CLI, testing and order of work

### CLI

The `brn` CLI stays a thin front end over `brn-workflow`, with a JSON output mode: `notes list/show/save`, `search`, `ask`, `conversations`, `comments add/list/delete`, `review start/show/accept/reject/approve/discard`, `ai connect/disconnect/status/select`. Everything can be scripted and tested without the desktop.

### Testing

All default tests are deterministic and offline, and use temporary vault and data folders.

- Save: exact bytes, conflict on external change or deletion, Save as copy never overwrites, leftover temp cleanup.
- Comments: re-finding with unique, duplicate and missing quotes; detaching; clearing on approve.
- Review: diff and change linking, accept/reject, typing marks accepted, second AI round, approve with undecided changes, approve after an external change, restart mid-review.
- Tools: archive, hidden and outside-vault paths are invisible; 50 KB cap; 8-round limit.
- Index: rebuild after deleting `index.sqlite`; only changed notes re-embedded; keyword-only mode without the model.
- Database: corrupt `brn.sqlite` restored from backup; only 5 backups kept.
- AI flows: Rig's test support (fake completion model or cassette replay) with fixed responses, for both providers' request formats. No network.
- Secrets: synthetic tokens never appear in logs, errors or stored messages.
- Live ChatGPT/Copilot checks are manual and run only when the user asks.

### Order of work

Each step is one reviewable PR:

1. **Spike (throwaway):** Rig login, a streamed answer and one tool call for both ChatGPT and Copilot, built in the same dependency graph as rusqlite 0.40.2, fastembed 7.1 and GPUI. Confirm how the account name is obtained for each provider. Go/no-go before step 4.
2. **Store and vault:** new `brn.sqlite` schema with integrity check and backups; vault path rules, scanning and reading. Update `AGENTS.md` and the architecture invariants to these rules.
3. **Search:** `index.sqlite` (notes, passages, FTS5, embeddings) refreshed from the vault scan, plus fusion; remove LanceDB.
4. **AI chat:** `brn-ai`, Connect/Disconnect/select, read tools, saved conversations, CLI `ask`; remove `brn-provider` and Codex configuration.
5. **Writing:** comments, review mode, Address comments, new-note proposals, CLI `comments`/`review`.
6. **Cleanup:** simplified save path (drop NSFileCoordinator/NSFilePresenter); remove old import, approval, drafts, checkpoints, revision-candidate and operation-journal code and `brn-core`; update architecture, status and roadmap documents; mark superseded documents historical.

Steps 2 and the spike can run in parallel. Step 3 needs 2. Step 4 needs 2 and the spike. Step 5 needs 3 and 4. Step 6 is last.

## 10. Before distribution (later)

- Review OpenAI's and GitHub's terms for third-party use of the subscription logins. Rig authenticates with the Codex CLI's and the Copilot editor plugin's OAuth client IDs and calls their private endpoints. If that isn't allowed for a distributed app, ship with API-key providers instead.
- Add an explicit API-key provider option (Rig supports it directly).
- Bundle the embedding model, sign and notarize the app, and test on a clean Mac.
- Port saving and credential folders to Windows.

## 11. Repository rules that change

These replace the conflicting rules in `AGENTS.md` and the architecture invariants in step 2:

- Notes are vault Markdown files; `index.sqlite` is disposable; `brn.sqlite` holds user work and is backed up.
- The AI writes only proposals; only user Approve writes AI text to the vault.
- Comments are temporary review notes, deleted when the note's review is approved.
- Exact note bytes are preserved; comments are never re-anchored by guessing.
- Deterministic offline tests by default; live provider checks only on request.
