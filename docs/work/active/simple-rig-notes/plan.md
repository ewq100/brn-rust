# Simple Rig-based notes app: historical roadmap

> **Historical baseline, superseded 2026-10-03:** Steps 1–4 and their linked plans/evidence describe implemented work. Future Steps 5/6 and conflicting specification rules are retired. New work follows the frozen [architecture/invariants](../../../architecture/invariants.md), [current roadmap](../../../roadmap.md) and [development workflow](../../../development/workflow.md). Old execution/model/attribution instructions in this plan pack are not current requirements.

**Goal:** Replace the Codex App Server architecture with a simple notes app: Markdown vault, SQLite search, Rig chat with ChatGPT/Copilot, comments and AI rewrites approved by the user.

**Architecture:** Existing crate boundaries stay (`brn-store`, `brn-retrieval`, `brn-workflow`, `brn`, `brn-desktop`); a thin `brn-ai` is added; `brn-provider` and `brn-core` are deleted. New code is built beside the old code in a new data folder; old code is removed in the final cleanup step.

**Tech Stack:** Rust `1.98.1`, Rig `=0.43.0`, rusqlite `=0.40.2` (bundled, FTS5), fastembed `=7.1.0`, GPUI-kit `=0.6.6`, Tokio, `similar`.

**Historical spec:** [Simple Rig-based notes app](../../../superpowers/specs/2026-10-02-simple-rig-notes-design.md). Retained as implementation evidence, not the requirements source for new work.

## Historical constraints

- The vault folder is the truth for notes; only regular `.md` files up to 1 MiB, excluding hidden files/folders and top-level `archive/`.
- The AI never writes to the vault; only user Approve writes AI text.
- No automatic fallback between providers, models or accounts.
- `index.sqlite` is disposable; `brn.sqlite` holds user work, is checked at start and backed up (5 newest copies).
- Credentials live in an owner-only folder; never in SQLite, logs, fixtures or errors.
- Default tests are deterministic and offline with temporary folders. Live provider calls and model downloads only when the user asks.
- Old data folders, the original vault and existing trial workspaces are never modified.
- Use `--locked` for Cargo commands on existing manifests; after adding a dependency, run once without `--locked` to update the lockfile, then commit it.

---

## Steps

| Step | Plan | Delivers | Needs |
| --- | --- | --- | --- |
| 1 | [Spike](spike.md) | Go/no-go: Rig login, streamed chat and a tool call for both providers in BRN's dependency graph. Throwaway code. | — |
| 2 | [Store and vault](store.md) | `WorkStore` (`brn.sqlite`, backups, restore, settings, unsaved edits) and the vault module (path rules, scan, read). Updated repository rules. | — |
| 3 | [Search](search.md) | `index.sqlite`: notes, passages, FTS5, local embeddings, fusion; non-UTF-8 notes listed as unreadable; workflow `Library`; LanceDB removed. | 2 |
| 4 | [AI chat](chat.md) | `brn-ai`, Connect/Disconnect/select, read tools, saved conversations; CLI moves to the new data folder with `notes list`, `search`, `ask`; model download with consent; `brn-provider` removed. | 1, 2; step 3 Library for read/search tools |
| 5 | Superseded writing step | Historical proposal: comments, review mode, Address comments, new-note proposals, CLI `comments`/`review`. | Retired; use current roadmap |
| 6 | Superseded cleanup step | Historical proposal: simple save path, legacy removal and `brn-core` cleanup after writing. | Retired; safe Save/recovery now precedes legacy removal |

Steps 1–3 are implemented through `e24b104`; local-model inference was skipped
without assets. Step 4 is implemented and merged on `main@50f898a` through
PR #14: all eight tasks and the whole branch Opus-reviewed, with final fixes
approved through `dccc24a`.
Both consumers use AppWorker, and the production
provider crate is removed. [Evidence](evidence.md) distinguishes historical
checks from fresh integrated qualification. User/live/native acceptance remains
pending; Step 5 proposals/approval and Step 6
simple Markdown Save/legacy cleanup are not implemented or authorized here.
Their delivery order now comes from the current roadmap, not these steps.

## Historical execution notes

- Execute on the branch the user chooses. Do not push or open PRs without explicit instruction.
- Record actual commands and results in [evidence](evidence.md) at the end of each step.
- The spike's live login and chat (step 1, tasks 3–4) need the user at the keyboard to enter the device codes.
- Step 4's plan distinguishes offline implementation from live acceptance. ChatGPT remains conditional after the spike's quota-blocked chat checks; a later quota reset does not authorize a live recheck.
