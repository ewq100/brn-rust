# BRN trust patterns

These patterns make BRN's trust model (vision §2–§4, §21, §23 and the
2026-10-07 amendment) visible. They apply to every surface.

## Kinds of knowledge

| Kind | Badge | Tone | Editable? | Example |
| --- | --- | --- | --- | --- |
| Current knowledge | `Current · editable` | Success | Small corrections with explicit Save | `projects/serna.md` |
| Source (evidence) | `Source · read only` | Info | Never; copy exact text | imported email, meeting notes |
| History | `History · read only` | Neutral | Never | superseded plan v1 |
| AI draft / proposal | `Proposal · Draft` | Attention | Yes, before approval | reply to Anna |
| AI interpretation | `interpretation · tentative` | AI | Via its own proposal | visual description |
| Web fact (future) | `web` | Neutral | Via capture proposal | supplier lead time |

A document always shows its kind badge in the `view_header`. Read-only views
offer *Copy exact …* and never offer Save or proposal controls.

## Claim provenance (future design)

In answers and drafts, important claims carry a small chip:
`vault` (current knowledge), `source`, `web`, `proposed` (BRN inference),
`unknown` (not established) or `conflict`. Chips open the exact evidence
(path, quote, version, retrieval date for web). Normal reading stays clean: one
chip per claim, never a footnote wall. Unknown and conflicting facts are also
stated in words (“has not been decided”). See the [prototype](prototypes.md).

## Proposals, review and approval

- A proposal view shows **everything that will change**: each member (Create,
  Replace, Trash, asset, Action) with its kind badge, before text when present,
  full proposed text, captured source versions and comments.
- A trust callout states the consequence: “Proposal edits and Rewrite do not
  change vault knowledge. Approval is a separate exact full-proposal operation.”
- The **decision bar** is pinned at the bottom: a sentence naming the exact
  version, *Reject* (outline), *Review captured group approval…* when grouped,
  and *Review exact approval…* (primary). Approval opens a confirmation that
  shows the captured proofs.
- Any edit, comment or Rewrite produces a new version; the bar always names the
  version that would be applied.
- Grouped consequences (Inbox, maintenance) are independent proposals reviewed
  together with an optional *Approve selected/all*; application stops at the
  first refusal and reports each result.

## Drafts and Rewrite

Drafts are private and persisted, but never masquerade as current knowledge.
Comments are temporary review notes, deleted after successful approval. Select
text in the proposed text and right-click → *Comment on selection…* (D18). Rewrite
runs on the selected model with a visible phase (“Rewriting · knowledge
unchanged”), can be stopped, and replaces the whole draft as a new version.

## Actions

- State badges: Open (Info), Waiting (Attention), Blocked (Danger), Completed
  (Success); date signals *overdue* (Danger) and *follow-up* (Attention) are
  separate badges because they can overlap.
- **Completion is always the owner's explicit act** (*Complete…* → confirm).
  Approving a reply never completes its Action; the UI says so.
- Follow-up work after completion is a new related Action (*New related
  follow-up…*).

## Inbox and originals

- Show what is waiting first, oldest first, with the original's kind, received
  time and availability.
- Processing produces previews and proposals; partial or uncertain conversion is
  stated with what is missing, and the original stays retained.
- Original-copy cleanup is a separate, explicit, recoverable step after an
  approved Source still proves exact preservation.

## AI work visibility

- The header phase badge and Stop are the global signal; the answer shows a
  one-line activity trail (future: steps, time, tokens against the budget).
- Each turn records and shows provider, model and effort exactly as used. Older
  history shows “unavailable” instead of guessing.
- Suggestions from BRN (focus order, possible outcomes, recommendations) use the
  AI tone and say “you decide”; they never look like approved facts.
