# Decision 3 caveat: are archived chats' chat-only notes excluded from owner search, and why?

I checked this on `b76cc5a7` before changing any decision 3 code. I read the code, ran `git log -S` and `git blame`, read the docs named below, and confirmed the result with tests: `message_search_scope.rs` `an_archived_project_chats_notes_stay_project_documents`, and the agent legs of `search_messages.rs`.

**Result: no contract reason exists, because no archive-based exclusion of chat-only notes exists.** Owner search already shows an archived or legacy Resolved chat's chat-only notes wherever their scope allows. The only archive-based exclusion in owner search is the one round 5 slice F wrote for Messages (`message_scope`). It was meant to mirror notes, but notes were never excluded. So decision 3 needs these changes:

- Messages: an archived chat's Messages keep their normal scope.
- Labels: a structured `archived` label on both kinds of hit.
- Agent retrieval: no change.

Review-F issue 2 ("a Resolved project chat's chat-only notes do not [show in project search]") misread the code: it saw the Open check and missed the project-document rule.

## What decides a chat-only note's visibility

- `crates/brn-threads-core/src/context.rs:20-42`, `document_allowed`. In a project context it first admits every **project document** (line 25). Only then does it fall back to `scope_allowed` (line 41).
- `crates/brn-threads-core/src/projects.rs:154`, `project_document_ids`. The project's documents include every non-archived Note whose `chat` is one of the project's chats (lines 176-179). The chat's own archived flag and `ThreadState` are not read.
- `context.rs:14-19`, `scope_allowed`. Line 18 admits a sibling chat's material only while that chat is not archived and `ThreadState::Open`. For chat-only Notes this check is never reached in a project context, because the document rule answers first. It does govern Threads, Messages (line 99), Runs, Sources and Actions, which agent runs read through the same predicate.
- Owner and agent note search share the re-check: `Snapshot::searchable_notes` (`reader.rs`) calls `visible`, used at `crates/brn-threads-app/src/search.rs:998`. The project total uses `record_in_chat`, the same function (`search.rs:1041`).

| Chat-only note of chat C | Search context | Shown before round 6 | Why |
|---|---|---|---|
| C general (open, archived or Resolved) | general (desktop, CLI default) | no | privacy: chat-only content is never general |
| C general (open, archived or Resolved) | C itself (`brn search --thread C`) | yes | `scope_allowed`, own chat (line 16) |
| C in project P (open, archived or Resolved) | P or any chat of P | yes | project document (`projects.rs:176-179`) |
| C in project P | general, another project, or another general chat | no | project boundary |
| C in a project in Trash | any | no | the project is archived, and project Delete archives the chat's exclusive notes |

Messages before round 6 (`reader.rs:84-87`): hidden for any archived chat, in every context. Resolved was ignored, so a Resolved project chat's Messages did show in project search.

## History

- `git log -S`/`git blame`: both `scope_allowed` line 18 and the `project_document_ids` chat-note rule come from `3569e6cb` ("Implement checked Projects across desktop runtime and CLI", 14 October 2026). Before that commit, a chat-only note was visible only in its own chat, archived or not (`n.chat.as_deref().is_none_or(|owner| Some(owner) == chat)`).
- Requirement: "It can search relevant earlier chats in that project" (`docs/work/active/chats-and-projects/spec.md:41`, item 7 at `:85`). "Private context, shared-document access and relevant earlier discussion use the same predicate in exact IDs, history, sources/assets, search, connections and candidate inputs" (`docs/architecture/threads-behavior.md:48`).
- `docs/work/active/projects/plan.md` and `docs/work/active/projects/evidence.md` give no reason for the Open/non-archived condition. The other `ThreadState::Open` test (`chat.rs:85`, `active_chat`, from `73211572`) refuses new work in an archived chat. It is not a read rule.
- Slice F's Message rule is in `docs/work/active/daily-workflow-reliability/round5/plan.md:424` ("`Hidden` if the record is not a Thread, or is archived"). It was written to mirror notes and cites no contract.
- `docs/work/active/knowledge-retrieval/plan.md:34` and `:148` exclude archived **notes** (records with `archived: true`) and superseded notes. A note in an archived chat is not itself archived.

## Boundaries decision 3 keeps

| Contract | Text | Kept |
|---|---|---|
| Chat-only privacy | "General lists/search/Markdown export exclude chat-only content" (`threads-behavior.md:46`); "General search/export excludes private content" (`invariants.md:36`) | Yes. Note visibility is unchanged. An archived chat's Messages keep the scope the open chat has, which is `Chat(c)` when the chat holds private context. |
| Project boundary | "Project-only material is not general BRN knowledge and is not automatically available to unrelated projects" (`chats-and-projects/spec.md:43`) | Yes. An archived project chat's Messages stay `Project(p)`. |
| Trash | Trash "excludes them from ordinary active use" (`chats-and-projects/spec.md:61`) | Yes. A chat whose project is in Trash stays `Hidden`. |
| Protection | Invariant 7 | Not involved. Search is read-only. |
| Agent context | `threads-behavior.md:48`, invariant 16 | Unchanged. `context.rs` is not edited, agent `search_notes` gets no Messages and no label, and an exact read of an archived sibling's Message is still refused. |

## A contract text to correct at integration

`threads-target.md:136` says: "Default retrieval … excludes pending candidates, superseded revisions, archived material, and scratch work unless requested". With it, `chats-and-projects/spec.md:70` says: "Finding archived material follows the existing explicit-access rule for archived material". These are currency rules, not scope, privacy or protection rules. Decision 3 changes them for owner search only, with a structured Archived label. Agent retrieval of Messages keeps them. Invariant 11 (`invariants.md:15`, archived notes) stays literally true, because archived Note records stay out.

Proposed correction: "Owner search also lists an archived (or legacy Resolved) chat's saved Messages under that chat's normal scope, and labels them and its chat-only notes Archived. Agent retrieval is unchanged."
