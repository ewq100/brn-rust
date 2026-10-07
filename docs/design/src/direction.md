# Experience principles and design direction

## Who BRN is for and how it is used

One owner (and later a small trusted group) on a Mac, doing knowledge work in
mixed English and Estonian: email and Teams threads, meetings, documents,
projects, people and unfinished Actions. BRN is open for long stretches beside
other work. The owner converses with it, reviews what it proposes, and expects
it to be exact about what is known, where it came from and what changes.

Product priorities (vision §45) decide trade-offs: correctness, trust, data
safety, provenance and predictable approval come before usability, which comes
before polish. The design aims for all of them, but never buys polish with
ambiguity.

## Direction: *calm workbench* — refined terminal chrome, editorial centre

BRN keeps the previously approved **refined-terminal** identity
([earlier handoff](decisions.md#d1-keep-the-refined-terminal-identity-correct-its-typography)):
square panes, thin lines, dark-first palette, restrained colour, monospace for
exact identity. This round makes it calmer and more legible:

- **Proportional type for everything people read** (chrome, chat, prose) and
  **monospace only for exact identity** (paths, UUIDs, model ids, byte ranges,
  counts, Markdown source). The handoff asked for this; the earlier build used
  12 pt Menlo everywhere.
- **Hierarchy through type, space and grouping, not boxes.** Rows are
  left-aligned and quiet; only the one main command in a region is filled.
- **Colour means state.** Six semantic tones, always paired with words.
- **Conversation in the middle, work around it.** The chat is always one glance
  away; operational surfaces open beside it, never replacing it.

## Experience principles

1. **Conversation first, work visible.** Chat is the default centre. Dashboard,
   Inbox, Needs Review and the review queue are one click away with honest
   counts, so the owner never hunts for unfinished work.
2. **Provenance is the texture.** Every claim, row and document shows what kind
   of knowledge it is — current, Source, History, AI-proposed, web, unknown or
   conflicting — and lets the owner inspect where it came from.
3. **Nothing durable changes without a visible decision.** Proposals end in one
   decision bar that names exactly what will be applied. Trust callouts state
   consequences in plain words (“knowledge unchanged until approval”).
4. **Calm by default, specific when it matters.** Unrelated findings become a
   count, not an interruption. When something affects the current task, it is
   shown in place with the evidence.
5. **Human first, exact underneath.** Lead with titles and sentences; keep
   exact identities one line below in monospace — never hidden, never first.
6. **Fail clearly and recoverably.** Every failure says what succeeded, what
   failed and what stayed unchanged, and offers an explicit retry. No silent
   provider/model switch; no lost typing.
7. **One home per job.** A capability appears in one place with one name.
   Duplicated controls and synonyms are defects.

## Voice and terminology

- Use the [glossary](../../product/glossary.md) words exactly: *Source*,
  *current knowledge*, *History*, *proposal*, *approval*, *Action*, *finding*,
  *Needs Review*, *Archive*, *Trash*, *Restore* (always with its context).
- Sentence case for labels and buttons. Verbs for commands (“Approve…”,
  “Inspect Source”). An ellipsis means a confirmation or form follows.
- Say what happens to knowledge: “Knowledge unchanged”, “Applies exactly
  version 2”, “Original retained”.
- Prefer “you” for the owner and “BRN” for the assistant; never “the AI”.
- Mixed-language content is shown as written; never translate quotes.
