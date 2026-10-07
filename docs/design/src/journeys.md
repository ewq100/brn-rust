# Core journeys

The vision's acceptance scenarios (§48) drive the design. Each journey lists the
surfaces it crosses and its readiness. Detailed screens are in
[current screens](screens-current.md) and [future designs](screens-future.md);
interactive versions are in the [prototype](prototypes.md).

## A · Email request to completed work (vision §5, Scenario A)

1. **Intake** — The owner adds Anna's email to the **Inbox**. It appears first in
   *Waiting*, with kind, received time and availability. *(Implemented for text
   and email copies; real `.eml` import is planned.)*
2. **Processing** — BRN converts it and proposes a **review group**: Source,
   thread link, person/project links, Actions with dates, project update and a
   detected conflict. Partial conversion of an attachment is called out and its
   original retained. *(Processing and individual proposals implemented; the
   grouped list is a prototype.)*
3. **Review** — The owner unchecks one item and approves the rest from one
   decision bar. Results are reported per proposal. *(Prototype; group approval
   exists per proposal in the app.)*
4. **Later** — “Are there any open actions?” — the answer lists them with
   claim chips; the Dashboard/Today shows the overdue reply. *(Answer
   implemented without chips; Dashboard implemented.)*
5. **Draft** — “Write an answer.” The activity trail shows search → sources →
   web → drafting, under a visible budget, with Stop. The result is a **reply
   proposal** that labels vault facts, Source quotes, BRN proposals, unknowns
   and the colour conflict. *(Prototype.)*
6. **Iterate** — Comments → *Rewrite with comments* → version 2. *(Implemented
   for proposals; claim labels prototype.)*
7. **Approve** — The decision bar names version 2. Approval records the reply;
   the UI states the Action stays Open. *(Implemented for proposals.)*
8. **Complete** — After sending outside BRN, *I sent it — complete Action…* →
   confirm. *(Completion implemented on the Dashboard; the shortcut from the
   approved reply is a prototype.)*
9. **Capture** — BRN offers to propose the new project knowledge. *(Prototype.)*

## B · New process document (§6, Scenario B)

Ask in chat → BRN drafts a **note proposal** grounded in the vault, marking
known, proposed and web content → comments and Rewrite → approve atomically.
*(Proposal review and Rewrite implemented; claim labels and web are future.)*

## C · Inbox DOCX (§7, Scenario C)

DOCX in Inbox → conversion preview with preserved images and separate AI
interpretation → Source and asset proposals → approve → explicit, recoverable
original cleanup; incomplete conversion keeps the original and says why.
*(Bounded DOCX and one inline PNG implemented; broader visuals future.)*

## D/E · Supersession and unresolved conflict (§9, Scenarios D–E)

A finding in **Needs Review** shows both sides with exact evidence and BRN's
tentative recommendation. Supersession opens a proposed update with a diff;
approval keeps the previous value as History. An unresolved conflict stays
visible, and answers say it is unresolved. *(Inbox-conflict findings
implemented; supersession proposal UI is a prototype.)*

## F · Work planning (§26, Scenario F)

“What should I focus on today?” → **Today** lists a suggested order with a
reason per item (due dates, who is waiting, blockers), clearly marked as a BRN
suggestion that the owner decides. *(Prototype.)*

## G · Session capture and lifecycle (§19, Scenario G)

BRN suggests durable outcomes inline (“Possible outcome to capture…”), each
becoming a proposal. Chats archive after 30 days of inactivity; Restore is one
click; Delete warns about uncaptured outcomes and never deletes captured
knowledge. *(Prototype.)*

## State coverage

| State | Where it is designed |
| --- | --- |
| Empty | Welcome checklist (implemented); empty Inbox, Needs Review, Dashboard (implemented); empty Today (prototype) |
| Loading | Hints in each surface (implemented); skeletons (prototype) |
| Error | Danger callouts with retry (implemented); provider failure in chat (prototype) |
| Cancellation | Stop with provisional output (implemented); stopped draft (prototype) |
| Success | Status-strip sentence; Applied/Completed badges (implemented) |
| Recovery | Activity: Undo, Reconcile, Finish/Restore repair (implemented) |
| Offline / AI unavailable | Disabled AI actions with reasons (prototype; partial in app) |
