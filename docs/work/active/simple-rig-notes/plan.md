# Simple Rig-based notes app: roadmap

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement each step plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Codex App Server architecture with a simple notes app: Markdown vault, SQLite search, Rig chat with ChatGPT/Copilot, comments and AI rewrites approved by the user.

**Architecture:** Existing crate boundaries stay (`brn-store`, `brn-retrieval`, `brn-workflow`, `brn`, `brn-desktop`); a thin `brn-ai` is added; `brn-provider` and `brn-core` are deleted. New code is built beside the old code in a new data folder; old code is removed in the final cleanup step.

**Tech Stack:** Rust `1.98.1`, Rig `=0.43.0`, rusqlite `=0.40.2` (bundled, FTS5), fastembed `=7.1.0`, GPUI-kit `=0.6.6`, Tokio, `similar`.

**Spec:** [Simple Rig-based notes app](../../../superpowers/specs/2026-10-02-simple-rig-notes-design.md). Read it before any step plan.

## Global Constraints

- The vault folder is the truth for notes; only regular `.md` files up to 1 MiB, excluding hidden files/folders and top-level `archive/`.
- The AI never writes to the vault; only user Approve writes AI text.
- No automatic fallback between providers, models or accounts.
- `index.sqlite` is disposable; `brn.sqlite` holds user work, is checked at start and backed up (5 newest copies).
- Credentials live in an owner-only folder; never in SQLite, logs, fixtures or errors.
- Default tests are deterministic and offline with temporary folders. Live provider calls and model downloads only when the user asks.
- Old data folders, the original vault and existing trial workspaces are never modified.
- Every commit ends with `Co-authored-by: Copilot App <223556219+Copilot@users.noreply.github.com>`.
- Use `--locked` for Cargo commands on existing manifests; after adding a dependency, run once without `--locked` to update the lockfile, then commit it.

---

## Steps

| Step | Plan | Delivers | Needs |
| --- | --- | --- | --- |
| 1 | [Spike](spike.md) | Go/no-go: Rig login, streamed chat and a tool call for both providers in BRN's dependency graph. Throwaway code. | — |
| 2 | [Store and vault](store.md) | `WorkStore` (`brn.sqlite`, backups, restore, settings, unsaved edits) and the vault module (path rules, scan, read). Updated repository rules. | — |
| 3 | Search (written after steps 1–2) | `index.sqlite`: notes, passages, FTS5, local embeddings, fusion; non-UTF-8 notes listed as unreadable; CLI `search`/`notes list`; LanceDB removed. | 2 |
| 4 | AI chat (written after the spike) | `brn-ai`, Connect/Disconnect/select, read tools, saved conversations, CLI `ask`; `brn-provider` removed. | 1, 2 |
| 5 | Writing (written after step 4) | Comments, review mode, Address comments, new-note proposals, CLI `comments`/`review`. | 3, 4 |
| 6 | Cleanup (written after step 5) | Simple save path, removal of old import/approval/drafts/revisions/operation code and `brn-core`, documentation. | 5 |

Steps 1 and 2 can run in parallel. Plans for steps 3–6 are written when their inputs exist, because they depend on the spike's confirmed Rig APIs and the store/vault interfaces from step 2.

## Execution notes

- Execute on the branch the user chooses. Do not push or open PRs without explicit instruction.
- Record actual commands and results in [evidence](evidence.md) at the end of each step.
- The spike's live login and chat (step 1, tasks 3–4) need the user at the keyboard to enter the device codes.
