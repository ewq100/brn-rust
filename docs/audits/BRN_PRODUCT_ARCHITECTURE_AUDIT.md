# BRN product and architecture audit

**Date:** 2026-10-03\
**Audited:** `ewq100/brn-rust`, branch `main`, HEAD `f164470`\
**Requirements baseline:** [BRN product vision](../product/BRN_PRODUCT_VISION.md), cited below as "Vision §N"\
**Type:** Read-only audit. No product code, dependencies, plans or data were changed. This report is the only file written.

**How to read this:** Sections 1, 6–9, 15, 18 and 19 are written for the product owner. The other sections hold the technical evidence behind them.

**Short glossary**

| Term | Meaning in this report |
| --- | --- |
| Vault | The folder of ordinary Markdown (`.md`) note files. |
| `brn.sqlite` (work database) | BRN's internal database for things that are not notes: chats, settings, unsaved typing. Backed up automatically. |
| `index.sqlite` (search index) | Disposable search data rebuilt from the vault. Losing it loses nothing. |
| Rig | The open-source Rust library BRN uses to talk to AI models. |
| "Simple" architecture | The newest generation of BRN code (default when the app starts). |
| "Legacy" architecture | The previous generation, still in the codebase, already scheduled for deletion. |
| Proposal | A change BRN prepares (new note, rewrite, action, profile update…) that takes effect only when the user presses Approve. |
| Module / seam | A self-contained piece of code / the place where one piece plugs into another and could be swapped. |

---

# 1. Executive summary

### The verdict

**The repository is a usable foundation that is partially misaligned with the product, and it is still far from the product.**

- **What works today** is a careful, well-tested app that can *read* a Markdown vault, search it, and chat about it with ChatGPT or GitHub Copilot. It cannot change any note. That is roughly a tenth of the product described in the vision.
- **None of the vision's distinctive features exist yet** in the default app: Inbox, document conversion, actions, Needs Review, proposals with Approve, projects, people, note types, "current vs history", web research, clickable sources, the graph.
- **The newest layer is a good base.** It uses the right overall shape: Markdown files are the truth; one internal database with automatic backups; a disposable search index; a thin AI layer where the user explicitly chooses the provider and model and BRN never silently switches. This should be kept.
- **About half of the Rust code is the previous architecture** (≈31,000 of ≈62,000 lines including tests). It was built for requirements the vision no longer has: safe simultaneous editing with other editors, per-note permission before AI may read a note, version history kept inside the database, and "publication". It is already scheduled for deletion but is still present. It is the largest single source of complexity.
- **The next planned step conflicts with the vision.** The current plan's "Step 5" designs word-by-word Accept/Reject of each AI change. The vision (§23) asks for whole-proposal approval with comments → Rewrite → Approve. This should be redesigned before anyone builds it.

### Is BRN fundamentally misdesigned?

No. The main seams are sound: user interfaces talk only to one application layer; that layer owns storage, search and AI; notes stay ordinary files. What is missing is the product's own vocabulary: proposals, actions, review items, note types and lifecycle, provenance. These can be added on top of the existing base without a rewrite.

### Where the complexity comes from

1. **Five architecture generations in about one week** (28 Sep → 3 Oct 2026). Each left code, plans and rules behind.
2. **Very defensive data-safety machinery for workflows the product no longer supports**, mostly in the legacy code (for example ≈5,000 lines coordinating file saves with other running editors).
3. **Heavy process and documentation**: ≈17,000 lines of Markdown documentation for ≈62,000 lines of code, much of it dense, historical or superseded.
4. **Some extra ceremony in the new code's background-work machinery.** Most of it is justified; it should not be copied into every new feature.

In short: **over-built infrastructure, under-built product.**

### The five decisions the owner needs to make

1. **Make the vision the only requirements baseline.** Formally retire plans that conflict with it: per-change review, the graph-database engine, "publication", and the old roadmap.
2. **Delete the legacy architecture** as soon as the new app can save a note. Confirm that nothing in old BRN data folders needs keeping.
3. **Approve one general "proposal" mechanism** as the backbone for every durable change: notes, actions, profiles, archive moves and relationships.
4. **Decide AI-provider legitimacy and the web-search source.** The subscription logins borrow other products' sign-in identities, and web search needs a chosen service.
5. **Clarify two approval rules the vision leaves open:** whether "clear" relationships may be written without approval, and whether an explicit spoken command ("create an action…") counts as approval.

Section 18 has details and options.

### What should NOT be changed

- Markdown vault as the durable truth, read byte-for-byte.
- The `brn.sqlite` design: integrity check, five rolling backups, automatic restore.
- The disposable `index.sqlite` with keyword search plus local "meaning" search. The embedding model must change for Estonian (§17).
- The thin Rig adapter with explicit provider/model, no fallback, and protected credential files.
- "Everything goes through `brn-workflow`" layering, plus the JSON command-line tool that lets agents and tests drive the app.
- Deterministic offline tests with throwaway data.

---

# 2. Audit baseline

| Item | Observed |
| --- | --- |
| Branch | `main` |
| HEAD | `f164470` (2026-10-03 07:11 +0300) "docs: sync merged AI chat and preserve local product records" |
| Last code merge | PR #14 at `50f898a` (Step 4 "AI chat") |
| Worktree | Clean before and after the audit (`git status --short` empty) |
| Other local branch | `evokessler-ericcp-rig-architecture-reset` (worktree at `c0a3526`): **fully merged** into `main`, with no unmerged commits |
| Remote trial branches | `trial/*`, `feature/*`, `codex/drafts-checkpoint`: historical experiments, not current work |

### Migration state

The repository is **mid-migration** under the [simple Rig-based notes roadmap](../work/active/simple-rig-notes/plan.md):

| Step | Content | State |
| --- | --- | --- |
| 1 Spike | Rig login/stream/tool for both providers | Done. Copilot GO; ChatGPT CONDITIONAL (quota-blocked) ([findings](../../experiments/rig-spike/FINDINGS.md)) |
| 2 Store and vault | `brn.sqlite` (WorkStore), vault rules | Done |
| 3 Search | `index.sqlite`, FTS5, local embeddings, fusion; LanceDB removed | Done |
| 4 AI chat | `brn-ai`, Connect/Select, read tools, saved chats; Codex App Server removed | Done, merged PR #14 |
| 5 Writing | comments, review mode, proposals | **Not started** (design conflicts with vision, see §4 and Appendix A) |
| 6 Cleanup | simple Save, delete legacy code and `brn-core` | **Not started** |

Both architectures ship in one binary. The desktop defaults to the simple app; `--legacy` opens the old one (`crates/brn-desktop/src/native/mod.rs:210-290`). The CLI routes each command by the data folder's type (`crates/brn/src/cli/mod.rs:1125-1163`).

Recent relevant commits: `9263adc` retire App Server; `a6bf6f4` desktop simple chat; `bef8d55` CLI cutover to AppWorker; `cd40c77` Rig streamed read-only chat; `d0d03ca` WorkStore conversations; `180451f` replace LanceDB; `732e476` adopt simple-notes rules.

### What was inspected

Root README, AGENTS.md, status, documentation index, roadmap, active work, all six design generations, architecture overview/invariants/decisions/dependencies, verification guide, and simple-notes plans and evidence. All seven workspace crates were traced in code: store schemas, vault rules, index, Rig adapter, auth, tools, workers, CLI parser and routing, and desktop mode dispatch and screens. Experiments were read at README/findings level. Four read-only analyst sub-agents surveyed crates in parallel. Every finding used in this report was re-checked directly in code (citations below).

### Fresh verification run during this audit

Environment: macOS, `rustc 1.98.1`, `TMPDIR=/Users/evokessler/.brn-task5-fixtures` (outside Git), default features, offline.

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | Pass, no warnings |
| `cargo test --workspace --locked --offline` | **674 passed, 0 failed, 0 ignored**, 63 result groups (≈46 s). Includes empty groups (`brn-core` lib, `brn-flow`, native-disabled `local_embedder`/`model_download`) and one test that self-skips on APFS but reports pass (per prior evidence) |
| `bash scripts/verify-end-to-end.sh --fixtures-only` | Retirement check passed; **47 fixture assertions passed** (simple vault + legacy local; no account/model/network) |

**Not run or not verified:** native-feature builds/tests (`native-ui`, `native-retrieval`); any graphical use; live ChatGPT/Copilot through the product; model download or real embedding inference; performance at 5,000 notes; behavior with an iCloud/Dropbox-synced vault; `verify-storage.sh` and the launcher test. **No user acceptance of any feature is recorded anywhere in the repository.**

---

# 3. Current architecture

### In plain English

Today BRN is one Mac application, plus a command-line tool used mainly by AI coding agents and automated tests. Inside it are two generations of the product.

- **The simple app (default).** You choose a vault folder. BRN reads every ordinary `.md` file (except hidden files and a top-level `archive/` folder) and builds a search index. You can then chat. The AI can search, read and list notes, but it cannot change anything. Chats are saved in `brn.sqlite`, which is checked and backed up every time the app opens. You choose ChatGPT or Copilot and a model in Settings. There is no Save button, no proposals, and no actions.
- **The legacy app (`--legacy`).** An older design that imports documents *into the database*, has a heavily protected note editor (it coordinates saves with other running editors and recovers from crashes mid-save), plus drafts, immutable checkpoints, and anchored comments. Its AI was removed in Step 4, so it is now an editor without AI.
- **Experiments** (`experiments/`) are standalone trial programs from earlier phases. They are not built with the app.

### Component map

```text
Legend: [S] simple/current production path   [L] legacy, scheduled for removal
        [H] historical/standalone              [P] planned, not built

 brn-desktop (GPUI)                       brn CLI (custom parser, --json envelope)
 ├─ simple views [S] ai.rs, native/simple.rs   ├─ simple cmds [S] ai, models, ask, notes list/show PATH,
 ├─ shared shell/layout/theme [S]               │              search/status/conversations (by folder type)
 └─ legacy editor/drafts/comments [L]           └─ legacy cmds [L] import, documents, index, drafts,
    notes.rs, drafts.rs, comments.rs                           comments, revisions, notes save/recovery/…
           │                                            │
           ▼                                            ▼
 brn-workflow ────────────────────────────────────────────────────────────────
 ├─ App + AppWorker + ChatWorker [S]   one owner thread + chat thread + install thread
 ├─ Library (index refresh/search) [S]  AiTools (read-only tools) [S]  vault/ [S]  models [S]
 ├─ Workspace + Worker [L]  notes/* (NSFileCoordinator save protocol, crash reconciliation) [L]
 └─ brn-flow binary [L]
           │                         │                              │
           ▼                         ▼                              ▼
 brn-store                    brn-retrieval                  brn-ai [S] ──► Rig 0.43 ──► ChatGPT | Copilot
 ├─ work/ (brn.sqlite) [S]    ├─ note_index (index.sqlite)[S] auth, chat, tools, errors
 │   4 tables                 │   5 tables, FTS5 + vectors
 └─ Store (brn.sqlite3) [L]   └─ Index (generations) [L]
     28 tables                    used only by legacy Workspace

 brn-core [L]  sample in-memory shell, used only by desktop headless sample checks
 experiments/{codex-app-server, editor-trial, retrieval-trial, rig-spike} [H]
 [P] proposals, comments (simple), Save (simple), everything in Vision §5–§43 not listed above
```

### Size by generation (tracked Rust, source + tests)

| Area | Legacy (to remove) | Simple (to keep/extend) |
| --- | --- | --- |
| `brn-store` | 6,184 src + 5,110 tests | 997 src + 1,121 tests |
| `brn-workflow` | 5,833 src + 1,368 internal + 3,034 tests | 3,217 src + 1,672 internal + 1,908 tests |
| `brn` CLI | ≈1,047 src (+ legacy parts of shared files) + 5,352 tests | ≈785 tests (+ shared parser) |
| `brn-desktop` | 2,420 legacy-only (+ legacy branches in 5,978 shared) | 2,493 simple-only |
| `brn-retrieval` | 390 + 195 tests | ≈1,750 src + ≈780 tests |
| `brn-core` | 521 | – |
| `brn-ai` | – | 3,445 (incl. 899 test module) |

Total tracked crate Rust: 61,821 lines. Legacy-attributable: **≈31,000 (about half)**. Experiments add 3,269 standalone lines.

### Data stores today

| Store | Tables | Owner | Evidence |
| --- | --- | --- | --- |
| `brn.sqlite` (simple WorkStore) | `settings`, `unsaved_edits`, `conversations`, `messages` | Simple | `crates/brn-store/src/work/mod.rs:26-56` |
| `index.sqlite` (simple) | `notes`, `passages`, `passages_fts`, `embeddings`, `meta` | Simple, derived | `crates/brn-retrieval/src/note_index/schema.rs:15-45` |
| `brn.sqlite3` (legacy Store) | 15 core (`sources`, `versions`, `operations`, `sessions`, `messages`, `chat_turns`, `drafts`, `draft_revisions`, `draft_comments`, …) + 13 note tables (`note_vaults`, `notes`, `note_save_intents`, `note_recovery_pairs`, `note_search_snapshots`, …) | Legacy | `crates/brn-store/src/lib.rs:35-56`, `crates/brn-store/src/notes.rs:350-368` |

The two databases are kept apart by "mode markers" and a shared owner lock (`crates/brn-store/src/workspace_mode.rs:5-81`), so one data folder can never hold both.

---

# 4. Product vision vs current system

**Status key:** **IV** implemented + verified (automated tests sufficient for this kind of behavior) · **II** implemented, verification incomplete (needs native/live/user checks not yet done) · **PI** partially implemented · **FE** foundation exists · **PO** planned only · **M** missing · **NM** implemented but does not match product intent · **XC** existing complexity the product does not require.

No row has user acceptance. "Verified" never means the owner has used it.

### Core knowledge model

| Capability | Required (Vision) | Current implementation | Status | Evidence | Main gap |
| --- | --- | --- | --- | --- | --- |
| Markdown vault as durable knowledge | §1, §36 | Simple reads `.md` exactly; no write path | PI (read: IV) | `crates/brn-workflow/src/vault/read.rs:51-92`; tests `crates/brn-workflow/tests/vault.rs` | No Save, no approved writes |
| Source evidence | §4.1 | Simple: none. Legacy imports `.md/.txt` **into SQLite** `sources/versions` | M (simple) / NM (legacy) | `crates/brn-store/src/lib.rs:35-50`; `crates/brn-workflow/src/lib.rs:226-305` | Sources must be vault files, separate from interpretation |
| Curated current knowledge | §4.2 | No note types, status or metadata | M | `library.rs:312-337` (only skips frontmatter for titles) | Needs simple frontmatter conventions |
| Historical/archive knowledge | §4.3 | Top-level `archive/` excluded from everything | FE | `crates/brn-workflow/src/vault/scan.rs:31-39`, `vault/path.rs:65-81` | No labeled history access; spec forbids it |
| Current vs superseded | §9.1–9.2 | None beyond the folder rule | M | – | Needs `status`/`supersedes` + index filter |
| Provenance | §21 | Search hits carry path, byte range, quote, hash; answers are prose only; nothing persisted | FE | `crates/brn-ai/src/tools.rs:18-36`; `chat.rs:39-42`; `work/mod.rs:42-53` | Citations in answers; per-turn evidence |
| Conflicts | §9.3 | None | M | – | Review item + answer behavior |
| Staleness | §9.5 | None | M | – | Maintenance job |
| Source authority | §9.4 | None | M | – | Agent rules + metadata (type, date, status) |

### Inbox and ingestion

| Capability | Required | Current | Status | Evidence | Main gap |
| --- | --- | --- | --- | --- | --- |
| Inbox | §7, §7.1 | None | M | keyword scan: no `inbox` in `crates/` | Whole concept |
| DOCX / PDF / PPTX | §7 | None; scanner ignores non-`.md` | M | no `docx`/`pdf`/`pptx` in `crates/` | Converters |
| Email / Teams text | §7 | Only by saving a `.md` manually | M | – | Paste/drop intake + source note |
| URLs | §7, §22 | None | M | – | Fetch + convert + provenance |
| Images, diagrams, tables | §7.2 | Non-`.md` files are not notes | M | `vault/scan.rs:57-95` | Assets folder + AI interpretation |
| Semantic conversion, source preservation | §7.1 | Legacy stores exact imported bytes in SQLite | NM (legacy) | `crates/brn-store/src/workflow.rs:120-540` | Wrong home (SQLite, not vault) |
| Proposed placement / metadata / relationships / actions | §7.1 | None | M | – | Proposal model |

### Actions

| Capability | Required | Current | Status | Evidence | Main gap |
| --- | --- | --- | --- | --- | --- |
| Persistent actions, states Open/Waiting/Blocked/Completed | §12.2 | None. `worker::Action` is an internal command enum, not a user action | M | `crates/brn-workflow/src/worker.rs:26-159` | Whole concept |
| Due / follow-up dates, waiting since | §12.3, §27 | None | M | – | Fields |
| Dependencies, parent/sub-actions | §12.7–12.8 | None | M | – | Two link tables |
| Manual creation, extraction from sources | §12.1 | None | M | – | Proposal kinds + agent tools |
| Explicit completion | §12.4 | None | M | – | "Sent it" → complete |

### People and projects

| Capability | Required | Current | Status | Evidence | Main gap |
| --- | --- | --- | --- | --- | --- |
| Person profiles, aliases, identity resolution | §14 | None (no `person`/`people` in code) | M | keyword scan | Person notes + alias index |
| Project profile + dynamic context + lifecycle | §13 | None | M | – | Project notes + context assembly |

### Chat

| Capability | Required | Current | Status | Evidence | Main gap |
| --- | --- | --- | --- | --- | --- |
| Multiple sessions, resume | §19.1 | Many saved conversations, New Chat, select; one AI answer at a time app-wide | II | `crates/brn-desktop/src/native/simple.rs:424-474`; `crates/brn-workflow/src/chat_worker.rs:416` | Native acceptance; concurrency choice (§18) |
| Session persistence | §19.1 | Turns saved with provider/model/status; running → interrupted on restart | IV | `work/mod.rs:42-53`, `work/mod.rs:115-125`; tests `tests/work_chat.rs` | – |
| Fresh new sessions | §19.1 | History is per conversation, last 20 Q/A pairs | IV | `crates/brn-ai/src/chat.rs:128-139` | – |
| Archive after 30 days, Archive/Restore/Delete | §19.3 | No archive/delete API; conversations have no last-activity time and messages have no timestamps | M | `work/mod.rs:37-53`; `work/chat.rs:377-425` | Timestamps + lifecycle |
| Capture durable outcomes | §19.2 | None (AI has no write or proposal tools) | M | `crates/brn-ai/README.md:118-119` | Proposal tools + pre-delete check |

### Writing and review

| Capability | Required | Current | Status | Evidence | Main gap |
| --- | --- | --- | --- | --- | --- |
| Full-document proposal | §23 | Spec Step 5 `propose_rewrite`/`propose_new_note` | PO | spec lines 96–110, 136 | Not built |
| Comments | §23, §24 | Legacy anchored comments on legacy drafts (≈1,900 lines incl. anchors and desktop state) | NM / XC (legacy); PO (simple) | `crates/brn-store/src/comments.rs`, `anchors.rs`; `crates/brn-desktop/src/comments.rs` | Simple comments on proposals |
| Rewrite | §23 | Spec "Address comments" | PO | spec §6 | Not built |
| Atomic approval | §23 | Spec plans **per-change word-level Accept/Reject** | PO, **conflicts** | spec lines 98–99 | Redesign as whole-proposal approval |
| Grouped proposals, Approve all | §23 | None | M | – | Proposal groups |
| Source immutability | §8 | None in simple | M | – | `type: source` convention |
| Rewrite as new current document | §8 | None | M | – | Proposal creating a new note + superseding the old one |

### Search and retrieval

| Capability | Required | Current | Status | Evidence | Main gap |
| --- | --- | --- | --- | --- | --- |
| Conversational retrieval | §20 | Agent tools `search_notes` (hybrid), `read_note` (50 KB), `list_notes`; hits re-validated against current files | II | `crates/brn-ai/src/tools.rs:74-215`; `crates/brn-workflow/src/ai_tools.rs:60-72` | Live use unverified |
| Current-state filtering | §9.1, §20 | Folder exclusion only | PI | `vault/path.rs:65-81` | Status-aware filter |
| Archive/history access | §4.3, §20 | Explicitly excluded from every tool | M; spec conflicts | spec line 138 | Explicit, labeled history scope |
| Source retrieval ("what did Anna actually say?") | §4.1 | No source/curated distinction | M | – | `type: source` + scope |
| Scale ≈5,000 active notes | §20, §44 | Brute-force vector scan; no benchmark | FE | `note_index/embeddings.rs:211-257` | Benchmark; archive excluded from vectors |
| Rebuildability | §34 | Corrupt/outdated index rebuilt; foreign files refused | IV | `note_index/schema.rs:69-168`; tests `tests/note_index.rs` | – |
| Mixed English/Estonian | §38 | FTS5 `unicode61 remove_diacritics 2`; embeddings `all-MiniLM-L6-v2` (English-trained) | NM (semantic) | `crates/brn-retrieval/src/native.rs:1`, `native/download.rs:14` | Multilingual embedding model |

### Web

| Capability | Required | Current | Status | Evidence | Main gap |
| --- | --- | --- | --- | --- | --- |
| Autonomous web research, web provenance, web-vs-vault conflicts, promotion of web facts | §22 | None | M | no `web_search` in `crates/` | Web tool (source decision §18), URL/publisher/date provenance |

### Proactive maintenance

| Capability | Required | Current | Status | Evidence | Main gap |
| --- | --- | --- | --- | --- | --- |
| Event-driven checks | §10.1 | None | M | – | Part of Inbox processing |
| Checks during normal work | §10.2 | None | M | – | Agent rule + `flag_for_review` tool |
| Weekly maintenance on launch | §10.3 | None. `NoteScheduler` only retries failed legacy saves | M | `crates/brn-desktop/src/notes.rs:60-100` | Last-run time + job |
| Needs Review queue | §11 | None | M | – | Review items table + badge |

### Relationships and graph

| Capability | Required | Current | Status | Evidence | Main gap |
| --- | --- | --- | --- | --- | --- |
| Proactive note relationships | §18 | No link or frontmatter parsing | M | `library.rs:312-337` | Frontmatter + link extraction |
| Visual graph | §43 | None. Old roadmap planned a graph **engine** (Cognee-RS/GraphRAG) | M; old plan NM | `docs/roadmap.md:28-37` | View over derived links table |

### AI and provider system

| Capability | Required | Current | Status | Evidence | Main gap |
| --- | --- | --- | --- | --- | --- |
| Explicit provider/model selection | §28 | `Selection{provider, model}`, frozen per Ask | II | `crates/brn-ai/src/lib.rs:22-39`; `chat_worker.rs:20-32` | Live/native checks |
| Thinking/reasoning effort | §28 | None | M | no effort parameter in `crates/` | Setting + pass-through |
| ChatGPT | §28 (e.g. GPT-6.1 Sol) | Hard-coded to `gpt-5.5` (model listing failed in spike); never completed a live answer (quota) | NM | `crates/brn-ai/src/lib.rs:31`; `auth.rs:380-387`; `experiments/rig-spike/FINDINGS.md` "ChatGPT" | Allow chosen models; live check |
| Copilot | §28 | Dynamic model list; spike live pass; product path not live-tested | II | `auth.rs:389-405` | Live check |
| Rig | §28 | Thin adapter; three real wire formats tested offline | IV (offline) | `crates/brn-ai/src/provider_formats_tests.rs` | – |
| Bounded subagents | §28 | None | M | – | Delegate tool, read-only |
| No silent fallback | §28 | Enforced in code and tests | IV | `chat.rs:73-106`; `crates/brn-ai/README.md:101` | – |
| Retry of transient failures | §28, §30 | Deliberately none; `retry_after` only shown | NM | spec line 121; `crates/brn-ai/src/error.rs:46-82` | Small same-model retry |
| Activity visibility | §29 | `ToolStarted{name}` event; UI shows "Tool started" for the live turn only | PI | `chat.rs:185-195`; `native/simple.rs:670-686` | Readable trail, optionally saved |

### User interface

| Capability | Required | Current | Status | Evidence | Main gap |
| --- | --- | --- | --- | --- | --- |
| Chat-first home | §25 | Chat is the default centre view | II | `native/simple.rs:619-634` | Native acceptance |
| Dashboard, Inbox, Open Actions, Needs Review, Projects, People | §25 | None | M | `ui-cli` survey; no such views | All |
| Vault browser (secondary) | §24 | Read-only saved-note list with paging | PI | `native/simple.rs:476-562` | Folders/types |
| Secondary editor | §24 | Simple: read-only. Legacy: full editor | PI (legacy only) | `native/simple.rs:564-617`; `shell/centre.rs:177-357` | Simple Save (Step 6) |

### Reliability

| Capability | Required | Current | Status | Evidence | Main gap |
| --- | --- | --- | --- | --- | --- |
| Vault write safety | §32 | Simple: no writes (safe by omission); planned temp-file + hash check + rename. Legacy: much stronger multi-editor protocol | PO (simple) / XC (legacy) | spec §4; `crates/brn-workflow/src/notes/save.rs`, `notes/macos.rs` | Simple Save |
| Internal DB backup and restore | §34 | `quick_check`, app ID, online backup each open, keep 5, restore newest | IV | `work/mod.rs:190-264`; `work/backup.rs:51-128`; tests `tests/work.rs` | – |
| Rebuildable indexes | §34 | Yes | IV | above | – |
| Crash recovery | §34 | Unsaved typing can be stored; running chats marked interrupted; read/clear UI not built | PI | `work/edits.rs:31-79`; `app_worker.rs:749-765` | Step 6 |
| Undo / recent recovery | §33 | None in simple | M | – | Keep previous bytes on approval |
| Trash | §35 | None | M | no `trash` in `crates/` | Hidden trash folder |
| External edits while closed | §32 | Startup scan updates index (size+mtime, then SHA-256) | FE | `library.rs:170-235` | Surface changes that affect proposals/actions |
| Conflict detection while open | §32 | Planned base-hash check (simple); legacy has it | PO | spec §4 | Build with Save |

### Portability, distribution, other

| Capability | Required | Current | Status | Evidence | Main gap |
| --- | --- | --- | --- | --- | --- |
| Normal Markdown, exact bytes | §36 | Yes for reading | IV | `vault/read.rs` | – |
| Assets as ordinary files | §36, §7.2 | No assets convention | M | – | `assets/` folder |
| Simple frontmatter conventions | §16, §36 | None (and an older design forbade BRN frontmatter) | M | Appendix A7 | Documented convention |
| Activity history | §42 | None | M | no `activity` in `crates/` | Append-only log |
| Personal working profile | §37 | None | M | – | One preferences note |
| Offline behavior | §31 | Reading and keyword search work offline; AI errors are typed | II | `library.rs:115-134` | Native check |
| AI credentials safety | §41 | Owner-only `0700` folder, `0600` files, never in logs | IV | `auth.rs:64-119`, `auth.rs:550-610` | – |
| macOS, trusted users, manual install | §39 | Unsigned local `.app` script; per-folder data and credentials | FE | `scripts/make-macos-app.sh` | Reproducible bundle, setup notes |
| No collaboration, mobile or own sync | §40, §47 | None (aligned) | IV | – | – |
| External vault sync (iCloud/Dropbox/Git) | §40 | Not tested | FE | – | Check simple Save on a synced folder |

---

# 5. User journey walkthroughs

Each walkthrough follows the vision's story through the **default (simple) app as it exists on `main@f164470`**. ✅ supported · 🟡 partly · ❌ not supported.

### J1. Serna email → action → grounded reply → comments → Rewrite → Approve → send → complete → promote knowledge (Vision §5, Scenario A)

| Step | Support | Where/why |
| --- | --- | --- |
| Add the email to BRN | 🟡 | Only by saving it yourself as a `.md` file in the vault. The next scan, focus or Refresh indexes it (`library.rs:170-235`). No Inbox. |
| BRN links it to Serna, Anna and the thread | ❌ | No projects, people, threads or metadata. |
| BRN proposes an action with dates | ❌ | No actions, no proposals. |
| "Are there any open actions?" | ❌ | No action store. The AI can only search notes. |
| "Write an answer", grounded in vault, labeled known / unknown / conflicting / web | 🟡 | The AI can search and read notes and stream an answer (`crates/brn-ai/src/chat.rs:108-151`). There are no structured citations, no unknown/conflict labels beyond what the model chooses to write, and no web. Archived material is invisible. |
| Comments → Rewrite → repeat | ❌ | Not in simple mode. Legacy has comments on drafts, but no AI. |
| Approve final reply | ❌ | No proposals or write path. |
| User sends manually; says "sent it" → action completes | ❌ | No actions. |
| Propose promotion of new knowledge into the Serna project | ❌ | No proposal tools. |

**Stops at:** step 2. Only "draft an answer in chat" works, without provenance.

### J2. Create a paint-purchasing process from the vault plus optional web (Vision §6, Scenario B)

| Step | Support | Where/why |
| --- | --- | --- |
| Search current vault knowledge | ✅ | Hybrid keyword + meaning search, or keyword-only without the model (`library.rs:115-134`). |
| Find analogous processes, roles, policies | 🟡 | Only by text similarity. There is no `type: process` or role metadata to filter on. |
| Grounded first draft | 🟡 | Drafted as chat text. |
| Lightweight provenance; known vs proposed | ❌ | Answer is prose; no source chips; nothing saved (`work/mod.rs:42-53`). |
| Web search when helpful | ❌ | No web tool. |
| Comment → Rewrite cycles | ❌ | Not built. |
| Approve atomically → new note in the vault | ❌ | No write path. |
| Relationships updated through proposals | ❌ | No relationships. |

**Stops at:** the draft. The user would have to copy the chat text into a file by hand.

### J3. DOCX/PDF/PPTX Inbox → semantic Markdown → images/interpretation → relationships/actions → approval (Vision §7, Scenario C)

| Step | Support | Where/why |
| --- | --- | --- |
| Drop a file into the Inbox | ❌ | No Inbox. Non-`.md` files are ignored by the scanner (`vault/scan.rs:57-95`). |
| Convert to Markdown, preserve images/tables, interpret visuals | ❌ | No converters. There is no evidence that image input works through the subscription endpoints (needs a spike). |
| Propose type, metadata, relationships, destination, actions | ❌ | No metadata model, no proposals. |
| Approve all or selected; Markdown becomes durable | ❌ | No write path. |
| Delete intake copy, or keep it and flag incomplete conversion | ❌ | No intake state. |

**Stops at:** step 1. Legacy `import` accepts only `.md/.txt` and stores them in SQLite (`crates/brn-workflow/src/lib.rs:226-305`), which is the wrong home under Vision §36.

### J4. New source contradicts approved current knowledge (Vision §9, Scenarios D and E)

| Step | Support | Where/why |
| --- | --- | --- |
| Know which knowledge is "approved current" | ❌ | No status or type. Every non-archived note is equally "current". |
| Detect likely supersession (dark red → blue) | ❌ | No event-driven check. The AI would notice only if asked and only if both notes are found. |
| Propose update; keep old value as history | ❌ | No proposals, no history convention. |
| Unresolved conflict (Anna: blue; meeting: green) visible in answers and Needs Review | ❌ | No review queue, no conflict record. |
| Resolve | ❌ | – |

**Stops at:** step 1. This is the area most dependent on missing domain concepts (status, provenance, review items).

### J5. "What should I focus on today?" (Vision §26, Scenario F)

| Step | Support | Where/why |
| --- | --- | --- |
| Look at open/waiting/blocked/overdue/follow-up actions | ❌ | No actions. |
| Project importance, who is waiting, recent communications | ❌ | No projects, people or communication metadata. |
| Suggest an order and explain why | 🟡 | The AI can reason over whatever notes it finds (for example a to-do note the user keeps by hand). |

**Stops at:** step 1.

### J6. Multi-session chat → capture durable outcomes (Vision §19, Scenario G)

| Step | Support | Where/why |
| --- | --- | --- |
| Several sessions, resume, new ones start fresh | ✅ (offline-tested) | `native/simple.rs:424-474`; `work/chat.rs:377-425`. Only one answer can stream at a time (`chat_worker.rs:416`). |
| "Save this decision" / "create an action" | ❌ | AI is read-only by design today (`chat.rs:117-123` preamble: "You cannot write notes"). |
| BRN proactively proposes outcomes | ❌ | No proposal tools. |
| Archive after 30 days; Restore; Delete with "uncaptured outcomes" warning | ❌ | No timestamps, archive or delete (`work/mod.rs:37-53`). |
| Durable knowledge independent of the session | n/a | Nothing is captured yet. |

**Stops at:** capture.

### J7. Project/person current-state view ("What is the current state of Serna?", "What am I waiting for from Anna?") (Vision §13, §14)

| Step | Support | Where/why |
| --- | --- | --- |
| Durable project/person profile | 🟡 | Only if the user writes a `Serna.md` by hand. There is no project/person type. |
| Dynamic context: open/waiting actions, recent comms, conflicts, related people | ❌ | No actions, relationships or review items to assemble. |
| Identity resolution (Anna Smith / Anna S. / email) | ❌ | No aliases. |
| Coherent answer | 🟡 | Text-search-based answer from whatever notes match. |
| Project/person view in the UI | ❌ | No views. |

**Stops at:** the dynamic context.

### J8. Visual note-relationship graph (Vision §18, §43)

| Step | Support | Where/why |
| --- | --- | --- |
| Relationships between notes | ❌ | No link or frontmatter parsing anywhere (keyword scan for `wikilink`/`[[` only in a CLI test fixture). |
| BRN maintains relationships proactively | ❌ | – |
| Graph view | ❌ | No graph data or renderer. |

**Stops at:** step 1. The old roadmap's graph plan (Cognee-RS/GraphRAG engines, `docs/roadmap.md:28-37`) does not match the vision's "graph is a view, not a datastore" (§43).

---

# 6. What to KEEP

| Keep | Why it fits the vision | Evidence |
| --- | --- | --- |
| **Layering:** desktop and CLI → `brn-workflow` → store / retrieval / AI | Exactly the "one owner of behavior" shape the vision's approval rule needs: there is a single place to enforce "no durable change without approval". Views never touch SQL, files or Rig. | `crates/brn-desktop/src/native/simple.rs:86-145`; `docs/architecture/overview.md` |
| **Markdown vault as truth, exact-byte reads, safe path rules** | Vision §1, §36. Hidden-file, symlink, traversal and size rules are sensible and tested. | `crates/brn-workflow/src/vault/*.rs`; `tests/vault.rs` |
| **`brn.sqlite` WorkStore design** (integrity check, app ID, ordered migrations, online backup on open, keep 5, restore newest, owner lock) | Matches Vision §34 almost word for word. Small (997 lines) and well tested. Add tables to it; don't replace it. | `crates/brn-store/src/work/mod.rs:22-278`; `work/backup.rs`; `tests/work.rs` |
| **Disposable `index.sqlite`** (FTS5 keyword + local vectors + rank fusion, rebuild on damage, incremental refresh) | Vision §20, §44: "simple personal-scale indexing that can be rebuilt". Avoids LanceDB/graph DBs. | `crates/brn-retrieval/src/note_index/*`; `crates/brn-workflow/src/library.rs` |
| **Thin Rig adapter (`brn-ai`)** with explicit `Selection`, frozen per turn, no fallback, typed errors, Stop, tool-round budget, `ToolStarted` events | Vision §28–30. Thin, not an extra service layer. Its seam (`ReadTools` trait implemented in workflow) is the right pattern to extend with more tools. | `crates/brn-ai/src/chat.rs`, `tools.rs`, `lib.rs` |
| **Credential protection** (owner-only folder outside Git, data and vault; device codes never logged; per-provider Disconnect) | Vision §41. The size of `auth.rs` (1,551 lines) is justified: it closes a real Rig weakness (caches written `0644`). | `crates/brn-ai/src/auth.rs:64-119, 292-373`; spike findings |
| **Fresh re-validation of AI evidence** (search hits re-read and hash-checked before going to the model) | Directly serves "never present stale/fabricated knowledge as current" (Vision §3). | `crates/brn-workflow/src/ai_tools.rs:60-72` |
| **AppWorker ownership model** (one owner thread; chat on its own lane; cancellable; joined on shutdown) | Keeps the UI responsive and shutdown safe. Extend it with a job queue rather than adding lanes (§7). | `crates/brn-workflow/src/app_worker.rs`, `chat_worker.rs` |
| **CLI with `--json` envelope** over the same workflow | Lets AI agents and tests drive every capability headlessly. That matters for an owner who relies on AI coding agents. | `crates/brn/src/cli/out.rs:47-69` |
| **Offline, deterministic tests with throwaway folders**; live calls only on request | Protects the owner's real vault and accounts. 674 tests pass in ≈46 s. | §2 of this report |
| **GPUI shell** (rails, centre, settings, theme) | Rougher UI is acceptable (Vision §45). Reuse the shell for dashboard and review screens. | `crates/brn-desktop/src/native/shell/*` |
| **Planned simple Save** (temp file, base-hash check, rename; never overwrite on create) | Exactly Vision §32: detect external change, don't overwrite, no multi-editor coordination. | simple-notes spec §4 |
| **Planned comment anchoring by quote + surrounding text, never guessed** | Enough for comments on proposals and notes; honest when text moved. | simple-notes spec §5 |

---

# 7. What to SIMPLIFY

Each item lists: current approach, simpler approach, what the user would notice, migration difficulty, and risk.

### S1. Planned per-change review → whole-proposal review *(highest priority; not yet built)*

- **Current (planned):** spec §6 diffs the AI rewrite word by word. Each change gets Accept/Reject; typing marks changes accepted; changes are recomputed on every AI round; Approve asks about undecided changes. Tables: `reviews`, `review_changes`, `new_note_drafts` (spec lines 96–110).
- **Simpler:** a proposal holds the full proposed text. The user reads it (optionally with a read-only "what changed" highlight), edits directly if they like, adds comments, presses **Rewrite**, and finally **Approve** or **Reject** the whole proposal.
- **User notices:** exactly the vision's flow (§23). Fewer buttons. No per-word decisions.
- **Difficulty:** low. It is design-only today.
- **Risk:** low. The user loses fine-grained per-change rejection, which the vision explicitly does not require for v1.
- **If nothing changes:** the team builds the most intricate piece of the old plan for a behavior the owner said is not wanted.

### S2. Separate "review" and "new-note draft" stores → one general proposal model

- **Current (planned):** note rewrites and new notes have their own tables; nothing else (actions, profile updates, archive moves, relationships) has any proposal form.
- **Simpler:** one `proposals` table (title, origin such as session/inbox/maintenance, group, round, status, rationale, provenance), one `proposal_items` table, and one `proposal_comments` table. Items are either **file changes** (create / replace / move / trash a Markdown file, each with the expected current hash) or **record changes** (create/update an action or review item). Every durable change in Vision §2 maps onto those two item kinds.
- **User notices:** one consistent review screen for chat outputs, Inbox results and maintenance findings, with Approve, Reject and Approve all.
- **Difficulty:** low–medium (new code, but one concept instead of several).
- **Risk:** moderate design risk if over-generalized. Keep only the two item kinds.

### S3. Legacy anchored-comment engine → comments scoped to a proposal round

- **Current:** legacy comments keep immutable original selections, exact edit traces, checkpoint snapshots, ambiguity states and compare-and-swap status versions (`crates/brn-store/src/comments.rs` 730 lines, `anchors.rs` 750, `crates/brn-desktop/src/comments.rs` 410, plus ≈1,000 lines of tests).
- **Simpler:** a comment is `{proposal or note, round, quote, ~40 characters before/after, body, open|addressed|detached}`. Rewrite consumes the open comments. Comments disappear when the proposal is approved or rejected, as in spec §5.
- **User notices:** nothing negative. Comments behave as the vision describes.
- **Difficulty:** low (build simple; delete legacy).
- **Risk:** low.

### S4. "Current vs history" by folder only → status in frontmatter, enforced by the index

- **Current:** only a top-level `archive/` folder is excluded, and from *everything*, including deliberate history questions (`vault/path.rs:65-81`; spec line 138).
- **Simpler and more complete:** BRN-managed frontmatter `status: current | superseded | archived` (plus `supersedes:` links). The index stores the status. Search defaults to current material and accepts an explicit `include_history` option that labels results as historical. Keep `archive/` as an extra rule, so a file there always counts as archived.
- **User notices:** "What is the Serna plan?" uses v3 only. "How did the plan change?" can see v1–v2, clearly labeled. Superseding a document no longer means moving files and breaking links.
- **Difficulty:** medium (metadata parsing + index columns + tool parameter).
- **Risk:** low. Files remain ordinary Markdown.

### S5. Background-work machinery: extend, don't multiply

- **Current:** AppWorker plus ChatWorker plus an install thread; idempotent Ask replay by UUID; tool-drain fencing while swapping the embedding model; one AI turn at a time app-wide (`app_worker.rs:606-659`, `chat_worker.rs:401-454`, `chat_worker.rs:416`). ≈2,150 production lines plus ≈2,000 test lines.
- **Simpler going forward:** keep it, but give the AI lane a small **job queue** (chat turn, Inbox processing, maintenance scan, session-capture check) instead of adding a new lane per feature. Do not copy the UUID-replay ceremony into each new command unless the CLI really needs it.
- **User notices:** weekly maintenance and Inbox processing can wait politely behind a chat instead of being refused with "another AI turn is active".
- **Difficulty:** medium.
- **Risk:** medium. Concurrency code is delicate; change it with tests first.

### S6. Model download with consent → bundled multilingual model *(option)*

- **Current:** a downloader with pinned digests, a consent prompt, decline/cancel paths and a keyword-only fallback (`crates/brn-retrieval/src/native/download.rs` 429 lines; `crates/brn-workflow/src/models.rs` 242 lines; desktop prompts).
- **Simpler:** ship the embedding model inside the app bundle for the trusted group (spec §10 already lists "bundle the embedding model" for distribution). The downloader, the consent state and much of the keyword-only signaling could go.
- **User notices:** semantic search works out of the box; no download dialog.
- **Difficulty:** low–medium (bundle step in `make-macos-app.sh`).
- **Risk:** a larger app (≈100–500 MB depending on the multilingual model chosen). Licence notice needed.

### S7. Documentation and process → small, plain, current

- **Current:** ≈17,000 lines of documentation. Six design generations. Superseded plan packs still under `docs/work/active/` (for example `rig-first-reset/`, ≈1,500 lines). Evidence files record SHA-256 hashes of diffs and reviewer verdicts. Status and invariants are dense jargon (`docs/status.md`, `docs/architecture/invariants.md`). Several docs disagree with code (Appendix B).
- **Simpler:** the vision (requirements) + one plain-language status page + one current architecture page + short decision records + a vision-based build order. Move superseded plans to `docs/work/completed/` (history). Evidence = commands and results.
- **User notices:** the owner can read the status and know where things stand.
- **Difficulty:** low (documentation only).
- **Risk:** low. Keep history in Git.
- **If nothing changes:** every future AI agent must read conflicting generations and may "faithfully" re-implement superseded rules. This has already happened once (Step 5's design).

### S8. Legacy invariants → a one-page vision-aligned rule set

- **Current:** `docs/architecture/invariants.md` mostly describes legacy guarantees: publication contract, per-version search eligibility, coordination and exchange.
- **Simpler:** after legacy removal, restate about ten rules: vault truth; AI proposes only; approval writes; base-hash check; no fallback; current-by-default retrieval; provenance kept; offline tests; credentials never logged; backups.
- **Difficulty:** low. **Risk:** low.

---

# 8. What to REMOVE / RETIRE

### 8.1 Safe to remove now (nothing in the simple path depends on it)

| Item | Why | Evidence | Note |
| --- | --- | --- | --- |
| `experiments/` (4 standalone trials, 3,269 lines) and their scripts `verify-trial.sh`, `verify-editor-trial.sh`, `verify-retrieval-trial.sh`, `verify-retrieval-state.sh` | Historical; outside the workspace; superseded by product code | `Cargo.toml` members; `docs/README.md` | Tag the commit first; keep `rig-spike/FINDINGS.md` text in docs |
| Superseded plan packs under `docs/work/active/` (`rig-first-reset/`, paused UI brainstorm) | Marked superseded or paused; confuse agents | `docs/work/active/README.md` | Move to `completed/`, don't delete |
| `docs/roadmap.md` milestones 13–18 | Comment-batch, publication, graph engine, old packaging plan; replaced by vision | `docs/roadmap.md:13-45` | Replace with §15 build order |

### 8.2 Remove after replacement (needs simple Save + proposals/comments first)

| Item | Size | Replaced by | Evidence |
| --- | --- | --- | --- |
| Legacy `Store` (`brn.sqlite3`: sources, versions, operations, sessions, drafts, comments, 13 note-registry tables) | 6,184 src + 5,110 tests | WorkStore tables (§12) | `crates/brn-store/src/{lib,notes,drafts,comments,anchors,workflow}.rs` |
| Legacy `Workspace`, `Worker`, `notes/*` save protocol (NSFileCoordinator/Presenter, descriptor vault ownership, atomic exchange, crash reconciliation), legacy drafts/comments | 5,833 src + 1,368 internal + 3,034 tests | Simple Save + proposal apply | `crates/brn-workflow/src/{worker.rs,notes/}` |
| `brn-flow` binary | 129 | `brn` CLI | `crates/brn-workflow/src/main.rs` |
| Legacy retrieval `Index` (generations, profiles, approval-bound evidence) | 390 + 195 tests | `note_index` | `crates/brn-retrieval/src/lib.rs` |
| Legacy desktop editor/drafts/comments state and legacy branches of the shell | 2,420 + parts of 5,978 shared | Simple views | `crates/brn-desktop/src/{notes,drafts,comments}.rs`; `native/mod.rs` |
| Legacy CLI commands (`import`, `documents`, `index build`, `drafts`, `comments`, `revisions`, `notes save/recovery/compare/reload/relink/save-copy/approve-for-search`) | ≈1,050 src + 5,352 tests | Simple CLI commands for proposals/Save | `crates/brn/src/cli/*`; `crates/brn/README.md` |
| `brn-core` sample shell and desktop "headless sample checks" | 521 | Nothing (sample code) | `crates/brn-core/README.md`; `crates/brn-desktop/src/main.rs:1-4` |
| `--legacy` launch mode | – | – | `crates/brn-desktop/src/main.rs:24, 75` |

**Requirement check:** these pieces serve multi-editor coordination, per-note "approve for search", SQLite-held source versions, immutable checkpoint history and publication. None of these is a vision requirement. Several contradict it: Vision §2 (autonomous search), §32 (no multi-editor coordination), §36 (knowledge not trapped in SQLite).

### 8.3 Temporary migration duplication (disappears with 8.2)

Two workers (AppWorker / Worker), two stores (WorkStore / Store), two chat histories (`conversations/messages` vs `sessions/messages/chat_turns`), two unsaved-edit mechanisms (`unsaved_edits` vs `note_buffers`), two search paths (`note_index` vs `Index`), mode markers and mixed-mode refusal (`workspace_mode.rs`), desktop dual mode, CLI routing by folder type (`cli/mod.rs:1125-1163`), and the App Server retirement guard `scripts/check-provider-retirement.py` (useful until legacy is gone).

### 8.4 Uncertain — investigate before removal

| Item | Question | Suggested check |
| --- | --- | --- |
| NSFileCoordinator-based saving | The vision allows a vault synced by iCloud/Dropbox/Git (§40). iCloud may evict files (placeholders) or create conflict copies. Plain write-then-rename is usually fine, but this is unverified. | Short spike: simple Save on an iCloud Drive and a Dropbox vault, including evicted files and edits during sync. Do **not** port the legacy protocol unless the spike shows real loss. |
| Legacy evidence model (`chat_turns.evidence_json`, `EvidenceCurrentness`) | Simple chat dropped per-turn evidence, which the vision's provenance needs. | Reuse the *idea* (store evidence with each answer), not the code. |
| Legacy operation journal (`operations` table) | Approving a proposal that writes a file *and* records an action needs crash-safe ordering. | A small "applying" marker per proposal is enough (§12); don't port the journal. |
| Model downloader | Remove only if S6 (bundling) is chosen. | Owner preference on app size. |
| Old BRN data folders | Is anything there worth keeping? | Owner decision D5 (§18). |

---

# 9. What is MISSING

The minimum concept for each gap, and where it should live. **Markdown** = vault file/frontmatter. **SQLite** = `brn.sqlite` operational state. **Derived** = rebuildable `index.sqlite`. **Agent** = AI reasoning plus instructions, with no new storage.

| # | Missing capability | Minimum necessary concept | Lives in |
| --- | --- | --- | --- |
| 1 | **Write path** (manual Save; apply approved changes) | One vault-writer module: temp file → base-hash check → rename; create-only for new files; keep the replaced bytes for undo | Markdown (+ short-lived recovery copy in data dir) |
| 2 | **Proposals** (all durable changes) | Proposal + items (file change / record change) + comments + round + group; Approve, Reject, Rewrite, Approve all | SQLite |
| 3 | **Note conventions** | Small documented frontmatter: `id`, `type`, `status`, `project`, `people`, `related`, `supersedes`, `source{kind,origin,url,retrieved}`, `aliases`, `updated` | Markdown; parsed into Derived |
| 4 | **Current / superseded / archived + history access** | `status` + `supersedes`; retrieval scope `current` (default) or `include_history` (labeled) | Markdown + Derived filter |
| 5 | **Source notes and assets** | `type: source` notes treated as immutable by convention; `assets/` folder for extracted images | Markdown |
| 6 | **Inbox** | Intake folder in the data dir for disposable copies, plus `inbox_items` rows (state: waiting / processing / proposed / done / incomplete) | Files in data dir + SQLite |
| 7 | **Converters** | DOCX/PPTX (zip + XML), PDF text/images, email/Teams paste, URL fetch → Markdown + assets; visuals interpreted by the AI; "incomplete" flag keeps the original | Workflow code + Agent |
| 8 | **Actions** | `actions` (title, status Open/Waiting/Blocked/Completed, owner, person, project, source note, due, follow-up, waiting since, priority, completed at, parent) + `action_deps` | SQLite (optional linked note for big ones) |
| 9 | **Needs Review** | `review_items` (kind, summary, evidence refs, status). The AI may create these without approval because they are not durable knowledge (Vision §10.2) | SQLite |
| 10 | **Conflicts** | A review item of kind `conflict` with both evidence refs. The agent reads open conflicts and says "unresolved". The resolution is a proposal that edits the note | SQLite + Agent |
| 11 | **People and projects** | Ordinary notes with `type: person/project`; aliases in frontmatter; a context assembler that queries index + actions + review items (no new subsystem) | Markdown + Derived + Agent |
| 12 | **Relationships** | Edges derived from frontmatter fields and Markdown links → `links` table; uncertain ones become review items or proposals | Markdown → Derived |
| 13 | **Graph view** | A view over `links` (notes as nodes) | UI over Derived |
| 14 | **Provenance in answers** | Tools return short citation ids; the answer text carries markers; per-turn `evidence` list saved with the message (path/range/hash or URL/publisher/date + category: vault-current, source, web, inferred, history) | SQLite (per message) |
| 15 | **Web** | `web_search` + `fetch_url` tools that return URL, publisher and retrieval date; promotion is a proposal that writes a note with `source.url` | Agent tools; Markdown on approval |
| 16 | **Subagents** | A `delegate(task)` tool running a second Rig agent on the helper model with read-only tools; it cannot propose | brn-ai + workflow tools |
| 17 | **Reasoning effort** | A setting stored with the selection and passed to Rig per request (Rig support must be confirmed for both endpoints) | SQLite settings |
| 18 | **Retry** | Up to N retries on the same provider/model for transient errors *before any text has been shown*; otherwise fail clearly and keep partial text | brn-ai |
| 19 | **Activity trail** | Richer `ToolStarted` events ("Searching vault: 'Serna supplier'", "Reading projects/serna.md"); optionally saved per turn | Events (+ SQLite) |
| 20 | **Session lifecycle** | `last_active_at`, `archived_at` on conversations; timestamps on messages; Delete runs a capture check first | SQLite |
| 21 | **Session outcome capture** | The same proposal tools, used from chat; a "capture check" agent job before delete or archive | Agent + SQLite |
| 22 | **Scheduled maintenance** | `last_maintenance_at` setting; on launch, if older than 7 days, queue a maintenance agent job that only writes review items and proposals | SQLite + Agent job |
| 23 | **Activity history** | Append-only `activity` rows written when a proposal is applied (what, when, which proposal/session, which note/action) | SQLite (exportable) |
| 24 | **Trash and undo** | Hidden vault folder `.trash/<date>/…` for deletions; keep replaced bytes for recent approvals; Undo = a proposal restoring them | Markdown (hidden folder) + recovery copies |
| 25 | **Working profile** | One note (for example `profile/working-preferences.md`, `type: preference`) loaded into the agent instructions; changes only by proposal | Markdown |
| 26 | **Dashboard and queues UI** | Lists over SQLite (actions, inbox, review items, pending proposals) + active projects from the index | UI |
| 27 | **Multilingual retrieval** | Multilingual embedding model (supported by fastembed) | Derived |
| 28 | **Export of operational data** | `brn export --json` for actions, sessions and activity | CLI |
| 29 | **Existing-vault analysis** | A one-off agent job proposing types, relationships and archive candidates in batches | Agent + proposals |

Deliberately **not** listed as separate subsystems: "source authority" (agent rules + metadata), "staleness" (a maintenance-job check), "identity resolution" (aliases + agent asking when unsure), and "planning" (agent reasoning over actions). These need instructions and data, not new engines.

---

# 10. Data ownership / source of truth

| Data | Should live (single authority) | Today | Flag |
| --- | --- | --- | --- |
| Curated note text | Vault `.md` | Vault (read only) **and** legacy SQLite managed-note registry/buffers | ⚠ duplicate during migration |
| Source evidence (converted email/DOCX/…) | Vault `.md`, `type: source`; assets in `assets/` | Legacy: SQLite `sources/versions`; simple: none | ⚠ wrong home in legacy (Vision §36) |
| Original intake files | Data-dir Inbox folder (disposable) | None | – |
| Note type, status, current/superseded | Frontmatter | Folder `archive/` only; legacy per-note search permission tables | ⚠ ambiguous today |
| Relationships | Frontmatter + Markdown links (authority); `links` table (derived) | None | – |
| Person/project profiles, aliases | Vault notes | None | – |
| Working preferences | Vault note | None | – |
| Promoted web facts | Vault note with `source.url/publisher/retrieved` | None | – |
| Actions, dependencies | `brn.sqlite` (export later) | None | – |
| Needs Review items, conflicts | `brn.sqlite` | None | – |
| Proposals, rounds, comments | `brn.sqlite` | Legacy drafts/revisions/comments (legacy DB); simple none | ⚠ do not add simple comments while legacy comments still exist without a removal date |
| Activity history | `brn.sqlite` | None | – |
| Chat sessions and messages (+ evidence) | `brn.sqlite` | WorkStore **and** legacy `sessions/messages/chat_turns` | ⚠ duplicate during migration |
| Unsaved typing | `brn.sqlite` `unsaved_edits` | WorkStore **and** legacy `note_buffers` | ⚠ duplicate during migration |
| Settings (vault, provider, model, effort, last maintenance) | `brn.sqlite` `settings` | WorkStore `settings` | ✓ |
| AI credentials | Owner-only folder next to data dir | Same | ✓ |
| Search index, embeddings, links | `index.sqlite` (derived) | `index.sqlite` **and** legacy index generations | ⚠ duplicate during migration |
| Backups of internal DB | `data/backups/` (5) | Same | ✓ |
| Deleted notes | Hidden `.trash/` in vault | None | – |
| Recent replaced versions (undo) | Data dir, short retention | Legacy recovery pairs only | – |
| Embedding model files | App bundle or `data/models` | `data/models` | ✓ |
| Layout preferences | `layout.json` (presentation only) | Same | ✓ |

**Rule to keep:** if losing it would lose knowledge, it belongs in Markdown. If it is work-in-progress or task tracking, it belongs in `brn.sqlite` (backed up, exportable). If it can be recomputed, it belongs in `index.sqlite`.

---

# 11. Complexity assessment

> **Has BRN become over-engineered relative to the desired product?**
> **Yes in its legacy half and its process; mostly no in its new half; and it is under-built in the product's own domain.**

| Subsystem | Justified complexity | Temporary | Unnecessary for this product | Missing |
| --- | --- | --- | --- | --- |
| **Internal DB (WorkStore)** | Integrity check, backups, restore, owner lock, app ID (≈1,000 lines) | – | – | Tables for proposals, actions, review, activity, inbox |
| **Legacy Store** | – | Whole store until Step 6 | Per-note search approval, SQLite-held source versions, publication-ready revision history, 13-table save/recovery registry (≈11,300 lines with tests) | – |
| **Vault file writing** | Atomic rename, base-hash conflict check, create-without-overwrite, fsync (planned simple Save) | Legacy protocol until Step 6 | Coordination with simultaneous editors, descriptor-relative vault ownership, inode identity, atomic exchange, uncertain-outcome reconciliation (≈5,000 lines + ≈4,000 test lines) | Simple Save itself; trash; undo |
| **Comments / drafts** | Quote + context anchoring (planned) | Legacy until Step 6 | Edit-trace anchor mapping, immutable checkpoints, candidate revisions, CAS status | Simple comments on proposals |
| **Retrieval (`note_index`)** | FTS5 + vectors + fusion + rebuild + freshness checks | Legacy `Index` | – | Metadata, status filter, history scope, links, multilingual model |
| **AI adapter (`brn-ai`)** | Auth hardening, typed errors, round budget, wire-format tests | – | – | Effort, retry, web, subagents, proposal tools, citations |
| **App/AppWorker/ChatWorker** | Responsive UI, cancellation, safe Disconnect, shutdown joins | – | Some ceremony (UUID replay ledger, tool-drain fences on model swap); keep but don't replicate | Job queue for non-chat AI work |
| **Model installer** | Digest-checked download with consent | – | Possibly all of it if the model is bundled (S6) | Multilingual model |
| **Desktop** | Shell, layout, theme | Dual-mode branches | Some layout sophistication (three resizable rails, Split/Tabs/focus modes) beyond what a "rougher UI" v1 needs; keep, don't extend | Dashboard, queues, review screen, graph, project/person views |
| **CLI** | JSON envelope; headless parity | Legacy commands, routing by folder type | – | Proposal/action/inbox commands |
| **`brn-core`** | – | – | Entire crate (sample code) | – |
| **Documentation/process** | Evidence of what was actually verified | Superseded plan packs | Six generations of specs, SHA-256-level evidence, jargon-dense status/invariants | Plain-language status; vision-based build order |

**Bottom line in plain words:** the code BRN still carries protects against situations the product no longer promises to handle, such as two editors saving the same file at the same instant. Meanwhile the things the product does promise (proposals, actions, review queue, current-vs-history, sources) have no code at all. Removing the first and building the second are both needed. Neither needs a rewrite of the new core.

---

# 12. Minimum target architecture

The design goal is the **simplest architecture that can deliver the full vision while reusing what exists**. No new crates are strictly required. One optional small crate for note conventions is mentioned below. Nothing here is "for later".

### 12.1 Modules and dependency direction

```text
 brn-desktop (GPUI views)          brn CLI (--json)
        │  commands/events               │
        └──────────────┬─────────────────┘
                       ▼
 brn-workflow  — the application (only module that changes state)
 ┌──────────────────────────────────────────────────────────────────────────┐
 │ AppWorker      one owner; lanes: app, AI jobs (queue), installs          │
 │ ├─ Jobs        agent runs: chat turn | inbox processing | maintenance |  │
 │ │              capture check | vault analysis       (uses brn-ai)        │
 │ ├─ Tools       given to the agent:                                       │
 │ │              read  : search(scope), read_note, list_notes, links,      │
 │ │                      actions, review_items, project/person context     │
 │ │              write : propose_note, propose_rewrite, propose_action,    │
 │ │                      propose_action_update, propose_move/archive,      │
 │ │                      flag_for_review (operational, no approval needed) │
 │ │              web   : web_search, fetch_url                             │
 │ │              help  : delegate(task) → helper model, read-only tools    │
 │ ├─ Proposals   create / comment / rewrite / approve / reject / approve-all│
 │ │              ← the ONLY path from AI output to durable state           │
 │ ├─ Context     assembles "today", project and person pictures (read-only)│
 │ ├─ Inbox       intake + converters → proposed source notes + assets      │
 │ └─ Vault       scan, read, safe write (base hash), move, trash, assets,  │
 │                Markdown conventions (frontmatter, links)                 │
 └──────┬─────────────────────┬──────────────────────────┬──────────────────┘
        ▼                     ▼                          ▼
 brn-store::work        brn-retrieval::note_index   brn-ai ──► Rig ──► ChatGPT | Copilot
 brn.sqlite             index.sqlite (derived)      thin: auth, agent runner, tool
 (operational state)    notes+metadata, passages,   adapter, effort, retry-before-
                        FTS5, embeddings, links      output, delegate, events
```

Dependency direction stays as today. UI/CLI depend on workflow. Workflow depends on store, retrieval and ai. **brn-ai never depends on store or retrieval**: tools are injected through a trait, extending today's `ReadTools` seam (`crates/brn-ai/src/tools.rs`). Retrieval never depends on store. The Markdown-convention parser (frontmatter + links) is needed by both workflow (to write) and retrieval (to index). It can be a tiny shared crate (for example `brn-notes`) or live in `brn-retrieval`. Decide this in the implementation plan.

**Seams that matter** (two real adapters each):

- Provider seam in `brn-ai`: ChatGPT and Copilot.
- Tool seam between `brn-ai` and workflow: real tools vs offline fakes in tests.
- Converter seam: one function per format, returning `Markdown + assets + incomplete flags`.

Do not add seams with one adapter: no storage repository interface, no pluggable search backend.

### 12.2 What lives where

| Markdown vault (durable knowledge) | `brn.sqlite` (operational, backed up) | `index.sqlite` (derived, disposable) |
| --- | --- | --- |
| Curated notes: project, person, decision, process, meeting, topic | `settings` (vault, provider, model, effort, last maintenance) | `notes` (+ type, status, dates, project, people, aliases, id) |
| Source notes (`type: source`), immutable by convention | `conversations` (+ last_active, archived), `messages` (+ timestamps, evidence) | `passages`, `passages_fts`, `embeddings` |
| Draft outputs, e.g. approved replies (`type: draft`; see D6) | `proposals`, `proposal_items`, `proposal_comments` | `links` (edges from frontmatter + Markdown links) |
| Working-preferences note | `actions`, `action_deps` | `meta` (schema/model identity) |
| `assets/` images and files | `review_items` | |
| `archive/` (optional) and `status:` frontmatter | `inbox_items` | |
| `.trash/` (hidden) | `activity` | |
| | `unsaved_edits` | |

### 12.3 Note conventions (recommended starting point)

```yaml
---
id: 01JA2X…              # BRN-managed, stable across renames
type: project            # project | person | decision | process | source | meeting | topic | draft
status: current          # current | superseded | archived
project: [serna]         # by id
people: [anna-smith]
related: [01J9…]
supersedes: [01J8…]
source:                  # sources and promoted web facts
  kind: email            # email | teams | docx | pdf | pptx | url | text
  origin: "Anna Smith <anna@company.com>"
  url: https://…
  retrieved: 2026-10-03
aliases: ["Anna S.", "anna@company.com"]   # person notes
updated: 2026-10-03
---
```

All fields are optional. Unknown fields are preserved byte-for-byte. BRN writes these fields **only through approved proposals** (but see decision D1). An earlier design rule forbade BRN fields in frontmatter (Appendix A7). The vision §16 supersedes it.

### 12.4 How proposals and approval work

```text
 agent tool call ──► Proposals.create (validate paths, ids, current base hashes)
                       │  status = pending, grouped by origin (session / inbox item / maintenance run)
 user: edit text, comment, Rewrite ──► agent job with proposal + open comments → new round, same proposal
 user: Reject ──► status = rejected (kept briefly for history)
 user: Approve (or Approve all, one proposal at a time)
   1. re-check every file item's base hash  → mismatch: nothing applied, show "changed since proposed"
   2. mark proposal "applying" (SQLite)
   3. write files: temp → rename (create-only for new); keep replaced bytes for undo;
      deletions move to .trash/
   4. one SQLite transaction: record items (actions, review items), proposal = approved, activity row
   5. refresh index for touched paths
   restart while "applying": compare file hashes → finish (4) or report what is unchanged
```

Each proposal is atomic on its own. A batch is a group of independent proposals (Vision §23). Completing an action after "sent it" is a direct user command, so no separate proposal step is needed (Vision §12.4). Creating an action from an explicit command is decision D2.

### 12.5 How retrieval works

`search(query, scope, filters)`:

- **Scope.** Default `current`: status current, not under `archive/`. `include_history` adds superseded/archived items, labeled "historical". `sources_only` serves "what did Anna actually say?".
- **Filters.** Type, project and person (from frontmatter).
- **Ranking.** FTS5 + vectors + reciprocal-rank fusion, as today. Hits are re-validated against the current file, as today.
- **Every hit carries** path, id, type, status, date and quote, so the agent can label provenance.

Embeddings are computed for current notes first. History can be keyword-only if the archive grows large.

### 12.6 How relationships work

On index refresh, edges are extracted from frontmatter (`project`, `people`, `related`, `supersedes`) and from Markdown/wiki links into `links`. Graph view = notes as nodes, `links` as edges. New relationships are proposed by the agent, either inside the Inbox proposal group or as a review item. They are written to frontmatter only by approval, or automatically for "clear" relationships if D1 allows it. No graph database.

### 12.7 Where Rig sits

Only inside `brn-ai`, used for: subscription auth, building the agent (instructions + tools + history + effort), streaming, hooks (round budget, activity events), retry before first output, and helper agents for `delegate`. Workflow never imports Rig types. That keeps provider churn contained, as today.

---

# 13. Current vs target architecture

```text
 CURRENT (main@f164470)                              TARGET
 ─────────────────────────────────────────           ─────────────────────────────────────────
 Desktop: simple reader/chat + legacy editor         Desktop: chat-first + dashboard + queues
 CLI: simple + legacy commands                                 + proposal review + graph + editor
                                                     CLI: same commands as desktop (JSON)
 Workflow:                                           Workflow:
   AppWorker (simple)  + Worker (legacy)               AppWorker (+ AI job queue)
   read-only AI tools                                  read + propose + review + web + delegate
   no write path       + legacy save protocol          Proposals (only path to durable change)
                                                       Vault (simple safe write, trash, conventions)
                                                       Inbox/converters, Context, Jobs
 Store: WorkStore (4 tables) + legacy Store (28)     Store: WorkStore (+ proposals, actions,
                                                       review, inbox, activity, session lifecycle)
 Retrieval: note_index + legacy Index                Retrieval: note_index (+ metadata, status
                                                       scopes, links, multilingual model)
 AI: Rig, 3 read tools, fixed ChatGPT model          AI: Rig, effort, retry-before-output,
                                                       chosen models, delegate, rich events
 brn-core, brn-flow, experiments                     (removed)
```

### Conceptual changes required

1. **Legacy architecture deleted.** One store, one worker, one search path, one UI mode.
2. **A write path exists**, used by manual Save and by approval only.
3. **Proposal becomes the universal unit of durable change**, with comments, Rewrite rounds, grouping and Approve all. This replaces planned per-change review.
4. **Notes gain a small metadata convention**, and the index understands it: type, status, relationships, sources.
5. **Retrieval becomes scope-aware**: current by default, history explicit and labeled, sources addressable.
6. **Operational records added**: actions, review items, inbox items, activity, session lifecycle.
7. **AI gains write-by-proposal, web, delegation, effort, limited retry, and citations.** It never gains direct writes.
8. **AI work beyond chat** (Inbox, maintenance, capture checks) runs as queued jobs on the existing AI lane.
9. **Embedding model becomes multilingual.**

---

# 14. Migration strategy

Principle: **incremental replacement with a green build at every step.** Delete legacy as early as safely possible, so that new work never has to coexist with it.

| Phase | What happens | Build/test state | Untouched |
| --- | --- | --- | --- |
| **0. Re-baseline (docs only)** | Owner accepts the vision as the only requirements source and answers §18. Mark simple-notes spec §6 (per-change review) and the "archive invisible to every tool" and "no retry" rules as superseded. Replace `docs/roadmap.md` with §15. Fix AGENTS.md and the other stale docs (Appendix B). Move superseded plans to `completed/`. | No code change | All code |
| **1. Simple Save, then delete legacy** | Build the simple vault writer (spec §4) with recovery read/clear. Run the iCloud/Dropbox spike (§8.4). Then remove legacy Store, Workspace, Worker, `notes/*`, `brn-flow`, legacy CLI commands, legacy desktop, `--legacy`, mode markers, `brn-core`, legacy `Index` and `experiments/` (tagged). | Test count drops by ≈15,000 lines of legacy tests. Remaining suite still passes. Re-run the fixture gate without the legacy half. | WorkStore, note_index, brn-ai, AppWorker, shell |
| **2. Proposal core** | Proposal tables and module; proposal tools for notes; comments; Rewrite; Approve/Reject/Approve all; activity log; trash; undo of last approvals. Desktop review screen; CLI `proposals …`. | New offline tests with a fake model (Rig test support already used in `provider_formats_tests.rs`) | Retrieval, auth |
| **3. Knowledge conventions** | Frontmatter parsing/writing; index metadata, status, scopes, links; citations + per-turn evidence; multilingual embeddings | Index rebuild is automatic (derived) | Proposal core |
| **4+. Product slices** | Actions, Inbox, Needs Review, maintenance, context views, web, delegation, sessions, graph, per §15 | Each slice adds tables via WorkStore migrations; nothing is rewritten | Earlier slices |

**What can remain untouched throughout:** the WorkStore open/check/backup/restore logic, the vault read rules, the FTS/vector/fusion search core, credential handling, the provider seam, the shell/theme, and the CLI envelope.

**Why delete legacy before building features.** Today every change must reason about two stores, two workers and two UI modes. Each new feature built alongside legacy would add to that, and deleting later becomes harder. Phase 1 roughly halves the code an agent must understand.

**Precondition.** Owner decision D5: confirm that no data in legacy folders must be kept. If some must, write a one-time export of legacy drafts/notes to Markdown first. That is a small script, not a migration subsystem.

---

# 15. Recommended build order (user-visible outcomes)

Each stage is testable offline with fake models and synthetic vaults, and then by the owner on the Mac.

| # | Outcome the owner can test | Main pieces |
| --- | --- | --- |
| 1 | **"I can open my vault, read, search, chat, fix a typo and Save; BRN refuses to overwrite a file changed elsewhere. The old mode is gone."** | Simple Save, recovery, legacy removal, docs reset |
| 2 | **"I pick the provider, the exact model (for example GPT-6.1 Sol) and the thinking effort; brief provider hiccups are retried on the same model; I see 'Searching vault → reading 3 notes → drafting'."** | Model list fix, effort, retry-before-output, activity events (plus a live check of both providers, if authorized) |
| 3 | **"I ask BRN to draft a document; it appears as a proposal; I comment, press Rewrite, then Approve; the note appears in my vault; Activity shows it; I can undo or recover from Trash."** | Proposal core, comments, Rewrite, apply, activity, trash |
| 4 | **"Answers show small clickable source markers; 'What is the Serna plan?' uses only the current version; history is used only when I ask, and is labeled; Estonian questions find Estonian notes."** | Frontmatter conventions, status scopes, citations, multilingual embeddings |
| 5 | **"I can say 'create an action to ask Anna for the spec, due Friday'; it shows in Open Actions with Waiting/Blocked states and dependencies; 'sent it' completes it; 'What should I focus on today?' explains its order."** | Actions, dashboard (actions part), context for "today" |
| 6 | **"I paste an email into the Inbox; BRN proposes a source note, links Serna and Anna, suggests an action with dates, and I approve all or some."** | Inbox (text/email/Markdown), grouped proposals, project/person notes + aliases |
| 7 | **"I drop a DOCX, PDF, PPTX or URL; BRN converts it with images and short descriptions; if conversion is incomplete, the original stays and I'm told why."** | Converters, assets, visual interpretation |
| 8 | **"BRN notices contradictions and stale facts: in context when relevant, otherwise quietly in Needs Review; a weekly check runs on launch if it was missed."** | Review items, conflict handling, maintenance job, AI job queue |
| 9 | **"'What's the state of Serna?' and 'What am I waiting for from Anna?' give one coherent picture; project and person pages show it."** | Context assembler, project/person views |
| 10 | **"BRN can search the web when useful, marks web facts as web, and lets me promote a useful fact into a note with its URL and date; helper models do bounded research."** | Web tools (per D3), delegate subagent |
| 11 | **"Sessions archive after 30 days; I can restore or delete; before deleting, BRN proposes anything worth keeping; it learns my preferences only through approved changes."** | Session lifecycle, capture check, working-profile note |
| 12 | **"I can explore a graph of how my notes connect, and confirm suggested links."** | Links table, graph view, relationship review |
| 13 | **"BRN reviews my existing messy vault and proposes types, links, folders and archive candidates in batches."** | Vault-analysis job |
| 14 | **"A trusted colleague installs BRN from a bundle, signs in, chooses a vault, and upgrades without losing data."** | Reproducible bundle, bundled model (S6), setup notes, upgrade test |

Stages 5–7 can swap order if the Inbox matters more than manual actions day to day. Stage 3 must come before any stage that writes (5–13).

---

# 16. Things NOT to build (v1)

**From the vision's non-goals (§47):** direct email/Teams sending; Outlook/Gmail/Teams connectors; internal calendar; background daemon; macOS notifications; mobile; collaboration; BRN multi-device sync; recurring actions; enterprise scalability; Git-style history UI; full Markdown IDE; simultaneous multi-editor coordination; silent provider fallback; automatic durable AI writes; App-Store-grade distribution.

**Additional, from this audit:**

| Don't build | Why |
| --- | --- |
| Per-change (word-level) Accept/Reject review | Vision §23 approves whole proposals |
| Porting the legacy save protocol (NSFileCoordinator/Presenter, atomic exchange, uncertain-outcome reconciliation) | Serves multi-editor coordination (non-goal); run the sync spike instead |
| Per-note "approve for search" permissions | Vision §2: BRN searches the vault autonomously |
| "Publication" contracts or exact-bytes publication approval | Not a vision concept; approval of a proposal is enough |
| Immutable checkpoint/candidate revision history in SQLite | Vision §33: no version UI; Git/Time Machine for deep history |
| Graph database or graph-RAG engines (Cognee-RS, GraphRAG-rs), LanceDB, sqlite-vec | Vision §43–44: graph is a view; brute-force vectors suffice at this scale (benchmark first) |
| A central "context engine/planner" or custom memory/summarization layer | The agent plus scoped tools suffices (also concluded in the historical reset handoff) |
| A generic workflow/job scheduler or plugin/MCP system | One AI job queue with four job kinds is enough |
| A storage "repository" interface or pluggable search backends | One adapter each: hypothetical seams |
| Numeric source-authority scores | Vision §9.4 explicitly prefers contextual judgment |
| One Markdown file per action | Vision §12.10 |
| Multiple vaults, Windows port, Keychain integration, API-key provider plumbing | Not required for v1. API keys only if decision D4 requires them |
| Concurrent streaming of several chats | Not required unless the owner says so (D7); queue instead |
| Complex anchor remapping of comments across rewrites | Rewrite consumes comments; quote + context is enough |

---

# 17. Risks (ranked by impact)

| # | Risk | Kind | Impact | Likelihood | Evidence | Mitigation |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | **AI access depends on subscription logins that reuse the Codex CLI's and Copilot editor plugin's OAuth identities and private endpoints.** They may break without notice or be disallowed, especially when shared with a trusted group and used on company data. ChatGPT has never completed a live answer in this codebase (quota-blocked) and is pinned to one model. | Provider / legal | Very high (the product is AI-centric) | Medium | Spec §10; `experiments/rig-spike/FINDINGS.md`; `crates/brn-ai/src/lib.rs:31` | Decision D4; keep the provider seam thin; live-verify both providers before building on them |
| 2 | **Trust failures: answers state facts without sources, or mix superseded and current knowledge.** Today answers carry no citations, and "current" means "not in `archive/`". | AI hallucination / provenance | Very high (Vision §3) | High until stages 3–4 | `chat.rs:39-42`; `vault/path.rs:65-81` | Status scopes, per-turn evidence, citation markers, review items for conflicts |
| 3 | **Plan churn: a sixth redesign before the product exists.** Five generations in a week; the current plan already conflicts with the vision. | Architecture / organizational | High | Medium | §1; Appendix A | Freeze on the vision; outcome-based build order; docs reset |
| 4 | **Legacy left in place while features are added**, so complexity compounds and agents follow superseded rules. | Complexity / maintainability | High | High if Phase 1 is skipped | §3 sizes; Appendix B | Phase 1 first |
| 5 | **Data loss when write paths arrive**: approval writes several things; vault synced by iCloud/Dropbox; crash mid-approval. | Data loss | High | Low–medium | No simple write path yet | Base-hash check, "applying" marker, keep replaced bytes, trash, sync spike, WorkStore backups (already present) |
| 6 | **Ingestion quality**: PDF/PPTX fidelity in pure Rust is limited; image interpretation via subscription endpoints is unverified. | Dependency / product | Medium–high | Medium | No converters; no multimodal evidence | Spike per format; "incomplete" flag keeps originals (Vision §7.3) |
| 7 | **English-only semantic search** degrades Estonian retrieval. | Product | Medium | High (certain today) | `crates/brn-retrieval/src/native.rs:1` | Multilingual fastembed model |
| 8 | **AI work beyond chat blocked** by the "one AI turn at a time" rule (maintenance, Inbox processing). | Architecture | Medium | High once those features start | `chat_worker.rs:416` | Job queue (S5) |
| 9 | **Rig moves fast** (pinned `=0.43.0`); reasoning effort, web tools, image input and agent-as-tool support are not evidenced in this codebase. | Dependency | Medium | Medium | `crates/brn-ai/Cargo.toml:8-24` | Short Rig capability spike before stages 2/7/10 |
| 10 | **GPUI toolkit maturity** (pre-1.0 `gpui-kit 0.6.6`, upstream `block v0.1.6` future-incompatibility warning); a graph view needs custom drawing; IME/accessibility unverified. | Dependency / UI | Medium | Medium | `docs/status.md`; `docs/architecture/dependencies.md` | Keep UI simple; graph as a plain node-link canvas |
| 11 | **Index freshness edge case**: an edit that keeps both file size and modification time is not re-indexed. Evidence returned to the AI is still hash-checked, so it surfaces as a "stale index" error rather than a wrong answer. | Correctness | Low | Low | `library.rs:188-216`; `ai_tools.rs:60-72` | Accept, or full re-hash on Refresh |
| 12 | **Scale**: brute-force vector scan with no benchmark; worst case of 1 MiB notes is ≈3 M passages. | Performance | Low (typical vaults are far smaller) | Low | `note_index/embeddings.rs:211-257` | Benchmark at 5,000 notes; embed current notes only |
| 13 | **Operational**: unsigned app, manual installs, credentials in files (by design), model download. | Operational | Low–medium | Medium | `scripts/make-macos-app.sh` | Stage 14; setup notes; bundle model |

---

# 18. Open product decisions

Only questions the vision does not already answer.

| # | Decision | Why it is open | Options (recommendation in **bold**) |
| --- | --- | --- | --- |
| **D1** | May BRN add **"clear" relationships** to notes without approval? | Vision §18 says clear relationships "may be added automatically", but §2 says every Markdown change needs approval. | (a) **Keep automatic relationships only in the derived index, and write them into notes only via proposals**; (b) allow automatic frontmatter edits for relationships only, logged in Activity; (c) always ask |
| **D2** | Does an explicit command ("create an action to…", "save this decision") count as approval? | §12.1 says actions become real only after approval; §12.4 accepts "sent it" as sufficient for completion. | (a) **Explicit command = approval; show it done with Undo**; (b) always show a proposal card to click |
| **D3** | Which **web-search source**? | §22 requires web use but names no service. Queries may reveal work topics to a third party. | (a) **Provider-built-in web search if the subscription endpoints support it (spike)**; (b) a third-party search API (needs a key; extra data processor); (c) fetch only URLs the user supplies |
| **D4** | Are the **ChatGPT/Copilot subscription logins** acceptable for company data and for the trusted group? | §41 says "corporate-approved provider"; spec §10 flags terms-of-service concerns. | (a) Owner-only use now, revisit before sharing; (b) **plan an enterprise/API-key route for the group**; (c) accept as is |
| **D5** | Is anything in **old BRN data folders / legacy mode** worth keeping? | Not addressed by the vision. | (a) **No: delete legacy outright**; (b) one-time export to Markdown first |
| **D6** | Where do **approved outbound replies** live? | §5: the user approves a reply and sends it manually; storage is not specified. The vision lists a `draft` note type. | (a) **Vault note (`type: draft`, linked to thread/action), so history is searchable**; (b) only with the action/session |
| **D7** | Does "multiple sessions at the same time" mean **several answers streaming at once**? | §19.1 is ambiguous. | (a) **Many open sessions, one answer at a time, others queued**; (b) true parallel streaming |
| **D8** | **Inbox location**: a Finder-visible drop folder, in-app paste/drag only, or both? | §7 doesn't say. | (a) **Both: a drop folder in BRN's data area plus paste/drag** |
| **D9** | **Bundle the embedding model** in the app (bigger app, no download prompt)? | Vision §39 wants understandable setup; size trade-off. | (a) **Bundle**; (b) keep the consent download |

---

# 19. Final recommendation

In plain language, in order:

1. **Keep the new foundation.** Notes as plain files, the self-healing internal database, the rebuildable search index, and the thin AI connection with explicit provider choice. These fit the vision and are well tested. *If we replaced them, we would spend weeks rebuilding what already works.*

2. **Reset the plan to the vision (documentation only, about a day).** Mark the per-change review design, the "archive is invisible" rule, the "never retry" rule, the graph-engine roadmap and "publication" as retired. Write a one-page plain-language status. *You would notice: status you can read. If we do nothing: the next AI agent builds the per-word review screen you don't want.*

3. **Finish simple Save, then delete the old architecture.** Answer D5 first. *Technically:* remove ≈31,000 lines (half the code) and one of the two databases. *You would notice:* nothing missing, and one app mode instead of two. *Why:* every future feature gets roughly twice as easy to build and review. *If we do nothing:* two systems keep growing side by side and cost more each week.

4. **Build the proposal backbone next.** One general "suggested change" with comments, Rewrite, Approve, Reject, Approve all, Activity history, Trash and Undo. *You would notice:* BRN can finally create and update notes, and only when you approve. *Why:* every remaining feature (Inbox, actions, conflicts, profiles, session capture, web facts) produces proposals. *If we skip it:* each feature invents its own approval rules.

5. **Teach notes what they are.** A small, readable metadata header (type, current/superseded, project, people, source), current-by-default search with labeled history, clickable sources, and an Estonian-capable search model. *You would notice:* "What is the Serna plan?" uses only v3, with sources you can click. *Why:* this is the core of the trust model (Vision §3, §9, §21).

6. **Then build the work features in this order:** actions and the dashboard → Inbox for emails, then documents → Needs Review with weekly checks → project/person pictures → web and helper models → session lifecycle → the graph → vault clean-up → packaging for colleagues (§15).

7. **Before stages 2, 7 and 10, run three short spikes:** both providers live (with your authorization), image/document input through the providers, and web search availability. These determine D3 and D4 and protect against the highest-impact risk (§17 #1).

**What not to do:** don't start another architecture redesign; don't port the old file-coordination code; don't add graph databases or new storage engines; don't let the AI write anything without a proposal.

---

# Appendix A — Plans and specs that conflict with the vision

| # | Existing decision | Location | Vision | Recommendation |
| --- | --- | --- | --- | --- |
| A1 | Word-level diff with per-change Accept/Reject; typing marks changes accepted | `docs/superpowers/specs/2026-10-02-simple-rig-notes-design.md:98-99` (§6) | §23 atomic proposal approval, no per-change acceptance | Supersede §6 with whole-proposal review |
| A2 | Out of scope: "archive/history AI tools", "graph retrieval" | same spec, line 21 | §4.3, §20 history usable when relevant and labeled; §43 graph view | Add history scope; graph as view |
| A3 | "Nothing is retried automatically." | same spec, line 121 | §28, §30 small same-model retry for transient failures | Retry before first output |
| A4 | "Excluded notes [archive] are invisible to every tool." | same spec, line 138 | §4.3 archive used when relevant, labeled historical | Explicit `include_history` scope |
| A5 | Proposals only for note rewrite/new note | same spec §6, §7 tools | §2 lists actions, profiles, archive state, reorganization, conflicts, web facts | General proposal model (S2) |
| A6 | Search requires explicit per-note approval ("Retrieval approval is explicit") | `docs/architecture/overview.md:38`; `docs/architecture/invariants.md:35`; CLI `notes approve-for-search` | §2 BRN searches the vault autonomously | Retire with legacy |
| A7 | "Do not insert mandatory Brain-specific IDs into Markdown or change existing frontmatter." | `docs/superpowers/specs/2026-09-30-markdown-first-ai-workspace-design.md:37` | §16 BRN manages type, frontmatter, status and relationships | Supersede; changes still go through proposals |
| A8 | Publication milestone; graph-engine qualification (Cognee-RS, GraphRAG-rs) | `docs/roadmap.md:20-37`; `invariants.md:60` | §43 graph is a view, not storage; §44 avoid enterprise infrastructure; no "publication" concept | Replace roadmap |
| A9 | Product framed as "import, grounded questions, drafting, review, approve exact publication content" | `docs/roadmap.md:7` | §1 chat-first work and knowledge assistant | Replace with vision |
| A10 | ChatGPT limited to `gpt-5.5` | `crates/brn-ai/src/lib.rs:31`; `auth.rs:380-387` | §28 user selects model (e.g. GPT-6.1 Sol) | Allow the user's choice (the spike routed `gpt-6.1-sol`) |
| A11 | English-only embedding model | `crates/brn-retrieval/src/native.rs:1` | §38 mixed English/Estonian | Multilingual model |
| A12 | Imported sources stored as SQLite versions | `crates/brn-store/src/lib.rs:35-50` | §36 knowledge not trapped in SQLite; §4.1 sources preserved | Sources as vault notes |
| A13 | Comments + re-anchoring as a "core feature" across edits | spec §5; legacy `anchors.rs` | §23 comments feed Rewrite | Keep the simple quote+context form only |

# Appendix B — Documentation that disagrees with code

| # | Document | Claim | Code reality |
| --- | --- | --- | --- |
| B1 | `AGENTS.md:20` | Repository map lists `crates/brn-provider` (Codex App Server) | Crate removed; `Cargo.toml` members include `crates/brn-ai`, which the map omits |
| B2 | `docs/architecture/overview.md:110` | Rig chat "implemented on `task-4-ai-chat`" | Merged to `main` (PR #14) |
| B3 | `docs/ui/feature-backlog.md:3` | "this repository uses the Codex App Server" | Retired (`9263adc`) |
| B4 | `docs/roadmap.md` | Next outcome is milestone 13 (comment-batch revision), then publication, then graph engines | Current plan is simple-notes Steps 5–6; the vision supersedes both |
| B5 | `docs/work/active/README.md` | Lists superseded `rig-first-reset` and merged `markdown-note-editing` under active work | Both are historical |
| B6 | `README.md` (last paragraph of "Native personal trial") | Describes draft working copies, checkpoints and comments as current behavior | Legacy-mode only; the default app has none |
| B7 | `docs/architecture/invariants.md` | Most rules describe legacy guarantees (publication, per-version eligibility, exchange) | Simple path follows the spec, not these rules (the transition note at the top acknowledges this) |

# Appendix C — Evidence index

- **Fresh checks:** §2 (fmt, strict clippy, 674 workspace tests, 47 fixture assertions; all offline, default features). Logs are kept in the audit session, not committed.
- **Schemas:** `crates/brn-store/src/work/mod.rs:26-56`; `crates/brn-store/src/lib.rs:35-56`; `crates/brn-store/src/notes.rs:350-368`; `crates/brn-retrieval/src/note_index/schema.rs:15-45`.
- **AI core:** `crates/brn-ai/src/chat.rs:73-151` (preamble, three tools, 20-pair history, `max_turns(9)`, no invalid-tool retries); `crates/brn-ai/src/lib.rs:16-91`; `crates/brn-ai/src/auth.rs:374-405`.
- **Simple workflow:** `crates/brn-workflow/src/app_worker.rs:28-65` (commands), `:765-800` (Ask: refresh → validate → submit); `chat_worker.rs:416` (single active turn); `library.rs:170-299`; `ai_tools.rs:60-112`; `vault/*.rs`.
- **Desktop/CLI:** `crates/brn-desktop/src/native/simple.rs` (chat, history, vault rail, read-only reader, settings); `native/mod.rs:210-290` (mode dispatch); `crates/brn/src/cli/mod.rs:1125-1163` (routing by folder type).
- **Absence checks:** case-insensitive searches across `crates/**/*.rs` for `docx`, `pdf`, `pptx`, `inbox`, `needs_review`, `web_search`, `subagent`, `graph`, `trash`, `person|people`, `activity` returned no matches. `reasoning|effort|thinking` matched only unrelated "best-effort" comments and a test fixture field.
- **History of design generations:** `docs/architecture/decisions/2026-09-28-architecture-baseline.md`; `docs/superpowers/specs/2026-09-30-…`, `2026-10-01-open-and-safely-edit-…`, `2026-10-01-rig-first-architecture-reset-design.md`, `2026-10-02-simple-rig-notes-design.md`; `docs/architecture/brn-rig-first-architecture-reset.md`.

