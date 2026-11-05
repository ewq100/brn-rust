# S5 Library, projects and search — evidence

Branch `work/ux-s5-library`, from `origin/work/ux-rebuild-20261105` at `7dc1c794`. Status: implemented and verified (automated). Native observation on synthetic data at the sizes below, in Light and Dark. Not owner-accepted, not merged. No PR opened (lead integrates).

## Commits

| Commit | Content |
|---|---|
| `bafb5019` | Library with Articles / All files, project workspace, chat lists and typed search results (first pass) |
| `c13e62dd` | Rows registered for tests; Library, project and launcher tests adapted; Library scope test |
| `41276986` | Topics note heads the Topics block, consistent counts, compact controls; `native_library_fixture` example |
| `edea21e5` | Compact controls at small sizes, project list "Active" heading, back-button row |
| `09c93276` | Source rows state the original's availability; readable last-change tooltip |
| `d3a0b3e0` | Library, Topics and launcher article opens start in Read |
| `b621a437` | Knowledge checks read as a page, not a form |
| `d7ed22ae` | Project document actions keep row chevrons aligned |
| `9fab9ba4` | Launcher finds the context project's own documents (AC-09) |
| `1bc5bc71` | Move chat marks the chat's current place |
| `3000d3ca` | Project notices name what changed |
| `49bc7e16` | Trash review names the project first |
| `f1332ae2` | Trash hint says what to do instead of repeating the subtitle |
| `f645b586` | Restore project keeps its natural width |
| `370bef0a` | Test that an archived article leaves the Library and search |
| `b9182dc4` | Library text hits open in Read mode; search headers aligned |
| this file | Evidence |

The first three commits are from an earlier helper; the rest continue the same slice.

## What moved where

**Library** (`threads_library.rs`, new; `notes_tile`, `threads_tile` and `all_threads_tile` moved out of `threads_tiles.rs`).

- Header "Your library", line "Current knowledge · N articles · N references and sources". Local views **Articles · N** (default) and **All files · N**.
- Articles holds current notes, excluding full imported references. All files also lists:
  - full imported references;
  - the Topics note;
  - retained Sources without a readable import, each stating availability ("Original file · …", "Web address · …", "Outlook item", or "Original not retained · BRN kept the recorded reference").
- Topics block: topics from the Topics note, each with its linked-note count; "Open Topics note" sits beside it. One article may appear under several topics without copies.
- Rows show title, a one-line description (first prose line, markup removed, never invented), and restrained facts:
  - "Updated today by delegated maintenance" or "Saved …";
  - "Linked from N";
  - Protected / imported where real.
  - No IDs or hashes. The last-change tooltip uses human time.
- One "Show" menu replaces the always-visible sections: Recent (default), A–Z, Most linked, Linked to open chats, Protected and imported. The list is virtualized (`uniform_list`).
- Find in your library:
  - a scope line ("current knowledge in your saved library: titles and note text; chat history is listed separately and is not current knowledge");
  - Titles (kind named per row);
  - In note text (Current knowledge);
  - In chats (Chat history · not current knowledge).
- Opening from rows, Topics, title or text hits, or a launcher article uses `open_from_library`: the same note and version, in Read with the assistant closed. A dirty or session-bound note stays in Write.
- Library ⋯: Knowledge checks, Recent changes, Open / Create Topics note. Row menu: Open, Related and sources, New chat about this note.
- `open_related(sources)` is the hook the document ⋯ menu (S4) can call. It opens the existing Connections view as the supporting pane.

**Knowledge checks** (`threads_connections.rs`): a titled page (`knowledge_panel(true)`) using the existing triage. Findings list affected records with Inspect / Open / review / draft follow-up. Heuristic kinds are worded as signals, not proof.

**All chats / Archived** (`threads_library.rs`): heading, All · N / Archived · N tabs, an "Any chat" filter menu (Open, Needs you, Linked note, Has task), a filter field, and Today / This week / Older groups. Rows are styled like Library rows.

**Projects** (`threads_projects.rs`):

- Projects index with Active · N and Trash tabs.
- Project page:
  - "← All projects", title, description;
  - a status line ("BRN knowledge: on · Project instructions apply to every chat") with Instructions & settings;
  - Documents · N with New document / Add document;
  - Chats · N with New chat;
  - empty states for each.
- Project document rows: "Shared · also in Library" vs "Only in this project", with trailing Keep in BRN / Unlink actions.
- Move chat marks the current place "· current" (disabled).
- Notices name what changed ("Project moved to Trash.", "Project restored.", "Chat moved to …").
- The Trash review lists the project first, then its contents and per-task choices.
- The Trash list hint: "Select a project to review and restore it. Restoring never resumes work."
- The trashed project page shows the restore line and a natural-width Restore project button.

**Search** (`launcher_overlay` in `threads_shell.rs`, `lists::launcher`):

- Groups and scopes:

  | Group | Scope label |
  |---|---|
  | Articles | Current knowledge · saved library |
  | Files | |
  | This project | Project only · not in Library (current project context only) |
  | Chats | Conversations |
  | In chats history | |
  | Commands | |

- Each row's accessible name carries type, scope and archived state.
- Keys unchanged. No widening: a project-only document is only listed while its own project is the context, matched by its own title.

### CAP mapping (S5 areas)

| CAP | Outcome now |
|---|---|
| CAP-009, CAP-033 | Sidebar Search and ⌘K open the same launcher |
| CAP-010 | Launcher groups Articles / Files / This project / Chats / In chats history / Commands, with type + scope labels |
| CAP-011 | ↑ ↓ ↵ Esc unchanged |
| CAP-287 | "In chats history" group after label matches, archived state in its name |
| CAP-063–068 | All chats / Archived page (tabs, filter menu, search, groups, row menu, empty states) |
| CAP-209 | Library find: Titles (100 cap with "keep typing"), then note text and chats |
| CAP-210, CAP-212 | Show menu: Recent, A–Z, Most linked, Linked to open chats, Protected and imported |
| CAP-211 | A–Z letter pills retired; A–Z order in the virtualized list replaces them |
| CAP-213 | Row menu: Open, Related and sources, New chat about this note |
| CAP-214 | Row facts and Protected chip |
| CAP-215 | Library empty state |
| CAP-216 | Topics block; Library ⋯ Open / Create Topics note |
| CAP-203, CAP-204 | Library ⋯ → Recent changes (existing view) |
| CAP-217–222 | Library ⋯ → Knowledge checks page (existing triage, Inspect, review, follow-up, unavailable reference) |
| CAP-223, CAP-224 | Related and sources → existing Connections view as the supporting pane |
| CAP-240–258 | Project workspace as listed above; services unchanged |

## Shared-file touches (smallest hooks)

- `threads_ui.rs`: `run_launcher` routes Note to `open_from_library` and adds the `ProjectDoc` arm; `launcher_items` passes the context project's documents.
- `threads_tiles.rs`: bodies moved out; `knowledge_panel(true)` call.
- `threads_components.rs`: removed the now-unused `list_row`.
- `threads_dialogs_tests.rs`: one assertion expects "Standalone chat · current".
- `threads_omakase_tests.rs`, `threads_writing_tests.rs`, `threads_tiled_tests.rs`: adapted to the new Library ids and views (below).
- Outside the listed ownership: `crates/brn-threads-app/examples/native_library_fixture.rs` (synthetic fixture through checked services; starts no provider).

## Tests removed or replaced

| Before | After | Reason |
|---|---|---|
| `lists::notes_view_stays_bounded_at_five_thousand_notes` | `note_rows_and_title_filter_scale_to_five_thousand_notes` | `NotesView` (fixed sections, `SECTION_LIMIT`, `LETTER_LIMIT`) retired for the virtualized list; rows, protected count, delegated activity and title filter caps asserted at 5,000 |
| `notes_tile_sections_letters_and_filter_stay_bounded_with_many_notes` | `library_list_stays_virtualized_ordered_and_filterable_with_many_notes` | Sections and letter pills retired; asserts virtualization (< 60 of 240 rendered), recent-first, Linked to open chats = 6, A–Z start, filter |
| P1-private retained source checks on `notes-recent-`/`notes-kept-` ids | same check on `note-` rows in both Articles and All files | Ids changed; assertion strengthened to both views |
| Launcher group assertions "Notes" | "Articles" with scope "Current knowledge · saved library" | Spec §5 naming |
| `ThreadFilter::ALL` / `key()` users | filter menu choices | Pills replaced by one menu |

Added: `articles_exclude_full_imports_and_files_include_them`, `the_topics_note_heads_topics_and_stays_in_all_files`, `facts_name_delegated_maintenance_and_never_claim_review`, `sources_with_a_readable_import_are_not_listed_twice`, `source_rows_state_where_the_original_is_without_repeating_the_name`, `descriptions_come_from_the_first_prose_line_without_markup`, `launcher_lists_the_context_project_documents_after_files`, `an_archived_article_leaves_the_library_and_search`, `library_articles_files_sources_checks_and_search_scope`, `project_notices_name_what_changed`, `the_trash_review_names_the_project_before_its_contents`, plus assertions:

- Move chat "· current";
- the Trash hint label;
- Restore natural width (< 240 px);
- a clean note opens from a text hit in Read mode.

The Restore-width, Trash-ordering and Read-mode assertions were each confirmed to fail without their fix.

## Commands and results

`rustc 1.98.1`, `--locked --jobs 4`, `TMPDIR` under `/private/tmp`, document helpers exported as `BRN_XBERG_HELPER` / `BRN_XBERG_HELPER_BRN`. Final run at the head of this branch:

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy -p brn-writing-ui --features native-ui,native-test-support --all-targets -- -D warnings` | pass |
| `cargo test -p brn-writing-ui --features native-ui,native-test-support` | 48 + 12 + 0 passed |
| `cargo clippy -p brn-desktop --features native-ui,native-test-support --all-targets -- -D warnings` | pass |
| `cargo test -p brn-desktop --features native-ui,native-test-support` | 399 passed, 0 failed, 1 ignored |
| `cargo clippy -p brn-threads-app --examples -- -D warnings` | pass |
| `cargo build -p brn-desktop --features native-ui` | pass |
| `git diff --check` | clean |

## Native observation

Device: MacBook Pro (Mac17,9, Apple M5 Pro, 24 GB), macOS 26.5, built-in 3024×1964 Retina display. Debug build of `brn-desktop`. Fresh data from `native_library_fixture --output /private/tmp/brn-s5-fixture` (two projects, current articles with one updated by delegated maintenance, a protected full imported reference with its Source, a loose Source, topic pages under the Topics note, a project-only document, chats, an archived chat). Light and Dark were switched with the ⌘K appearance commands, never the OS setting. Window captures (`screencapture -l`) in `/Users/evokessler/repos/brn-ux-rebuild-artifacts/shots/s5/`:

- Light 1280×928:
  - Library: `library-articles`, `library-files`, `library-menu`, `library-open-read`, `knowledge-checks`;
  - Projects: `projects-index`, `project-workspace`, `move-chat`, `chat-moved`, `project-trash-review`, `project-trashed`, `projects-trash`, `project-restore`, `project-restored`;
  - Search and chats: `search`, `search-project-context`, `all-chats`, `all-chats-archived`.
- Dark 1280×928: `library-articles`, `library-files`, `projects-index`, `project-workspace`, `search`, `search-project-context`, `knowledge-checks`.
- 900×628, Light: `library-articles`, `project-workspace`, `search`. Dark: `library-articles`, `project-workspace`, `search`, `knowledge-checks`.
- 5,000 notes (Light 1280): `library-5000`, `library-5000-scrolled`, `library-5000-filter`, `library-hit-read`, `search-5000`.

The brief asked for 1280×900; captures are 1280×928 (VE-03 framing unchanged). No clipping was seen at 900×628.

Fixed after native observation:

- Library opens went to Write.
- The source-row availability line repeated the name.
- Knowledge checks looked like a form.
- Document action columns pushed chevrons out of line.
- The launcher missed project-only documents in their own project.
- Move chat did not show the current place.
- The generic "Project saved." notice.
- The Trash review listed the project last.
- The Trash hint repeated the subtitle.
- Restore project stretched full width.
- Library search headers were 9 px out of line with their rows.

### Acceptance cases

| AC | Native | Automated |
|---|---|---|
| AC-08 | Articles default; All files shows the full imported reference, the Topics note and sources with availability; a row opens the same note in Read with the assistant closed | `library_articles_files_sources_checks_and_search_scope`, `articles_exclude_full_imports_and_files_include_them` |
| AC-09 | Global ⌘K "harbor": Articles / Files / Chats with scopes. From Harbor expansion, ⌘K "budget": "This project · Project only · not in Library → Expansion budget draft"; ↵ opens it. Not listed from Crew onboarding or globally | `launcher_lists_the_context_project_documents_after_files`, the project tail of the Library test |
| AC-26 | Source rows state "Original file / Web address / Outlook item / Original not retained · BRN kept the recorded reference"; Sources toggle in Related. Removing access to an original was not exercised natively | `source_rows_state_where_the_original_is_without_repeating_the_name`, `sources_with_a_readable_import_are_not_listed_twice` |
| AC-27 | Not exercised natively: the desktop has no article-archive control (archival is a checked write). Context switching between two articles and projects was exercised and kept their own state | `an_archived_article_leaves_the_library_and_search` (archived note leaves Library rows and launcher Articles); existing archived-chat search tests |
| AC-36 | Moved "Expansion planning" Harbor → Crew onboarding 2027 ("· current" marked). Trash review lists the project, its contents and task choices; moved to Trash ("Project moved to Trash."), listed under Trash, restored ("Project restored.") with chats back and nothing resumed | `threads_projects_tests` (16), `project_notices_name_what_changed`, `the_trash_review_names_the_project_before_its_contents` |
| AC-40 | Library ⋯ → Knowledge checks → Check knowledge; findings with affected records; Inspect shows the record and evidence; heuristics worded as signals | Library test (Knowledge checks reachable), existing connections/triage tests |
| §11 5,000 notes | Fixture with `--notes 5000` (9.8 s to seed). Library opened showing 5,007 articles / 5,010 files. 100 wheel events of 1,200 px (3.6 s) scrolled about 700 rows, with rows drawn at once in the capture. Typing "04321" in Find showed the title and text hit, and ⌘K "note 0432" listed 10 articles, both within the driver's ~1 s step. Not frame-timed; no target is claimed | `note_rows_and_title_filter_scale_to_five_thousand_notes`, `library_list_stays_virtualized_…` |

## Known gaps

- **Outside S5 ownership:**
  - A project-only document's assistant reads "New chat about <raw id>". `chat_tile` (`threads_writing.rs`) and the tile title (`threads_shell.rs`) look titles up in `note_rows`, which exclude project-only documents. This predates S5.
  - While a project document is open, the sidebar highlights Library (lead, sidebar).
- No desktop control archives an article, so AC-27's "archive then search" is automated only.
- Removing or denying an original (AC-26) was not exercised natively.
- The active segmented tab keeps its focus ring after a mouse click (Projects Active/Trash).
- A protected item shows both a Protected fact and a lock mark.
- GPUI tooltips can stay after scrolling, and can sit at the window edge.
- Text-hit snippets show the raw heading marker ("# Synthetic note …"); this comes from the search service.
- No "In chats history" group appeared natively for "harbor" (the fixture's messages did not match); it is covered by tests.
- Native checks shared the screen with other helpers' apps. Focus stealing and pointer movement caused retries and one transient hover highlight. Captures were retaken clean.
- `docs/work/active/ui-polish/plan.md` still describes the earlier Trash wording.
- Large text and scaling, IME, and VoiceOver were not exercised.

## Lead fixes at integration

Two gaps outside S5's files were fixed by the lead while merging. Both are covered by assertions in `library_articles_files_sources_checks_and_search_scope`, and neither has been rechecked natively yet:

- A project-only document's assistant now uses the document's title, not its id. `note_title` falls back to the open document's own record.
- While a project document is open, the sidebar now marks its project instead of Library (`focused_project`).
