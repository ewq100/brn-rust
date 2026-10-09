# brn-desktop

> Requirements/qualification context (2026-10-07): this README describes implemented behavior, not mandatory limits or acceptance of the proposed replacement. The [owner amendment](../../docs/product/BRN_PRODUCT_VISION.md#owner-amendment--2026-10-07), [reassessment](../../docs/audits/BRN_ARCHITECTURE_REASSESSMENT_2026-10-07.md) and [proposed plan](../../docs/work/active/architecture-reassessment/plan.md) reopen mechanisms. Email enum/literal text is not real EML ingestion; broader conversion, AI draft freedom and practical reviewability remain gaps. No production behavior changed in this documentation task.

Desktop entry point, GPUI views and transient interaction state. All operations go through AppWorker. The headless startup check opens and joins the same current workflow without GPUI.

## Current workspace (native default)

The default is **`~/Library/Application Support/BRN-simple`**, with the exact
**`BRN-simple.credentials`** sibling. Startup never inspects/copies/migrates the
old BRN folder, signs in, discovers provider models, chooses a provider/model,
or downloads a model. Data directory creation/canonicalization and layout
loading precede GPUI; AppWorker alone opens authority, scans the vault and
loads/uses retrieval resources off GPUI. [`ai.rs`](src/ai.rs) holds only
presentation DTOs and operation/generation correlation.

Choose Vault uses the native folder chooser and sends BindVault to the owner.
Notes support small corrections with explicit **Save to Markdown / Cmd-S**.
Fresh disk text and protected recovery remain separate; later typing survives
older acknowledgements. Compare baseline/local/disk, observe disk, confirmed
reload, exclusive Save Copy and uncertain-save reconciliation use AppWorker.
An unused vault-relative `.md` destination creates a copy without switching or
resolving the original editor. Refresh/search report keyword-only results, unreadable notes and
exact embedding progress. History resumes from WorkStore even without a vault
or a valid current selection.

Session labels show last-activity age from recorded chat activity. Historical
unknown activity and a timestamp ahead of the current clock have distinct labels;
opening history or restarting does not make a session newly active. Turn timing
is available through the shared workflow/CLI. The history rail defaults to Active
and offers explicit Active/Archived filters and Archive/Restore for the selected
session. Archived history remains readable, including recorded budgets, and shows
“Archived — restore to continue”; new Ask, Inbox investigations and Rewrite require
Restore. Archive changes organization without deleting history or consequences.
Delete and automatic archival remain separate work.

Archive/Restore captures the selected session’s exact lifecycle version and one
operation UUID. Session navigation, New chat and filters freeze while it is pending;
correlated acknowledgements keep the selected history, filter, composer, editor,
review and partial/error buffers intact. Old list generations and unrelated or
older lifecycle replies cannot overwrite newer metadata. Failures remain visible
with Refresh history available; no new session or inference is started automatically.
Bound proposal Rewrite also reads its own session lifecycle, while absent legacy
session identities retain existing eligibility. Headless state/widget tests do not
establish interactive usability; the morning acceptance journey remains deferred.

After 500 ms without an edit, recovery submits the latest exact buffer to
WorkStore; only its acknowledgement establishes recoverability. Note-switch,
document close and guarded window close/Quit wait for the latest recovery and
admitted mutations. Recovery failure retains text and offers explicit retry;
pending navigation can be cancelled. These routes do not save Markdown.
Dock/system termination cannot veto exit and can lose unacknowledged typing.
The editor is implemented and automated verified; native/IME/accessibility
acceptance remains pending in [status](../../docs/status.md).

The Vault rail offers Current, Source, History and All browsing/search scopes.
Current is the default and opens the existing guarded note editor. Other scopes
open complete saved text in a separate read-only view, labeled with requested
scope/path and an exact Copy action; they expose no Save or proposal controls.
All can contain current notes but remains a read-only combined evidence view.
Scope changes update browsing/search without discarding an open document or
changing Ask's default current-knowledge behavior. Pagination/results and opening
replies are bound to scope/cursor/generation. Registered/recovered buffers retain
their direct guarded editing route regardless of the selected browsing scope.

**Sources** in the note editor and read-only evidence view expands saved-source
provenance in that document. It reads saved Markdown; unsaved editor changes are
explicitly excluded and remain intact. Each citation shows its UUID, observed
path(s), exact stored quote and Matched, Changed, Absent, Ambiguous or Incomplete
status. Changed/unavailable originals never replace the stored quote. Quotes are
read-only and **Copy exact quote** preserves their complete bytes. The bounded
panel offers Refresh and Close sources, with loading/empty/error states; it does
not navigate to a guessed source or expose a new mutation. Accepted document
navigation clears the panel; late inspection replies cannot reopen or replace it.

For manual source acceptance, use a fresh synthetic fixture from the
[CLI provenance scenario](../brn/README.md#durable-source-provenance), then launch
its native workspace. For the byte-preservation check, repeat capture/approval
with an additional fresh synthetic source containing BOM, CRLF and Unicode;
select byte range 0 through its complete UTF-8 byte count after identity assignment.
Open the saved knowledge note, type an unsaved correction
and select Sources: the saved quote/status must appear while typing stays intact.
Copy the quote and compare BOM/CRLF/Unicode bytes. Close/reopen the panel, then
change or duplicate the fixture source externally and Refresh; Changed/Ambiguous
must retain the original quote. Read the knowledge note through All and repeat
inspection in the read-only view. Navigate with an unacknowledged editor buffer
and confirm recovery is still required before the document changes. GUI and
owner acceptance remain separate from headless widget/state checks.

For manual acceptance, launch with a fresh synthetic data/vault pair containing
`current.md`, a `brn_kind: source` note with BOM/CRLF/Unicode, a
`brn_state: history` note and an archived original. Confirm Current lists only
current knowledge; select Source/History/All, search and open an original. Its
requested scope and Read only label must remain visible; typing must not change
the text, Copy must preserve full exact bytes, and Save must be absent. Switch
scopes rapidly during list/search/opening and confirm late replies do not change
the new view. Return to Current, type an unsaved correction and open evidence;
the existing recovery guard must acknowledge the latest buffer first. Repeat
with a full review/comment or unsent proposal form to check the retained-input
guard. Restart and compare fixture source bytes. GUI/IME/accessibility and owner
acceptance remain separate from state/widget checks.

Settings provides independent ChatGPT/Copilot account status, explicit Connect/
Disconnect, model discovery, provider/model selection and explicit low/medium/high
reasoning effort. Ask stays disabled until its effort choice is acknowledged.
A connected cache with
no display name stays “account name unavailable”; failed/cancelled Connect
refreshes actual status. Codes/links exist only in the active transient login
dialog; cancel, dismissal and every ending clear it and target its exact UUID.
The verification URL opens only when clicked. The device code is read-only but
selectable with standard keyboard Copy, and an explicit Copy code button copies
the exact current code. A stale or dismissed prompt cannot open a URL or copy.
Code expiry/reconnect are explicit retry states, never automatic login.
ChatGPT live chat remains conditionally qualified; a quota reset alone does not
establish availability.

Settings provides labelled investigation budget presets beside the provider/model
and effort selection: 4/8/16/32 tool rounds and 60/180/300/600 seconds. New Ask
and Inbox investigations default to 8 rounds and 300 seconds, display the choice
before submission and capture it with the request. Budget controls are disabled
while an investigation is active. Rewrite remains outside these investigation
controls. The shared workflow enforces the captured ceiling.

Active investigations show completed model turns and admitted tool rounds with
the frozen ceiling; a parallel tool batch counts as one round. Time expiry shows
“Time limit reached; stopping and finalizing” until settlement. Global Stop
remains available while work drains. Final history and retained Inbox analyses
show the recorded ceiling and terminal reason; older records show budget
unavailable, without substituting the current settings.

For deferred native acceptance, use a fresh synthetic workspace and acknowledged
provider/model/effort. Choose 4 rounds/60 seconds in Settings, start Ask or Propose
notes, and confirm the visible captured ceiling and disabled budget controls.
Observe progress and use Stop; keep the partial answer and pending review drafts.
Inspect a retained run after restart and confirm its original budget, then inspect
older history and confirm unavailable limits. Time expiry must remain visibly
stopping until local finalization and retain its distinct terminal reason.
Headless state/widget checks cover capture, frozen controls, progress, late-event
correlation and historical availability; they do not establish interactive
usability or owner acceptance. Overnight GUI qualification remains deferred.

Each Ask freezes provider, model and effort. Changing Settings affects new
requests; active and historical turns retain their recorded choice. Older history
shows unavailable effort rather than inventing a value. Only one Ask is active; notes/search/account/history
diagnostics are independent. Text/tool progress is provisional. Stopped/Failed
partials retain their terminal status and provider/model, including historical
selections. Only Finished establishes durable finalization. PersistenceFailed
retains an explicitly **not saved** partial in memory, with an explicit Copy
action. Further Ask is blocked while finalization is unacknowledged; copy that
partial before closing/restarting. Account/history diagnostics remain available.
Navigation keeps owned progress correlated by exact request UUID/generation;
reopening the active conversation shows its full partial answer even after
visiting other history. Its live Running history row is hidden in favor of the
provisional stream. Finished replaces that row once, and older queued history
snapshots cannot restore Running over a known terminal result. A different
conversation or new blank chat does not adopt that result. Stop intent survives
pre-admission cancellation acknowledgements. Composer edits invalidate only
search results, not the current answer or its follow-up conversation.

History also lists typed proposals. Full review shows every Markdown and Action
member, exact captured before/proposed text or complete Action record, source
versions and temporary comments. Action controls cover all 14 candidate fields;
incomplete UUID/date typing remains visible and blocks approval until corrected.

A clean, acknowledged supplemental Inbox Knowledge draft with one Create offers
**Attach predecessor for full review** and a separate Current knowledge path.
The owner selects the predecessor explicitly; attachment preserves successor
wording, identity, comments and evidence without inference or a vault write.
Typing and navigation wait for an exact validated structural acknowledgement.
Review then shows the editable successor and generated read-only History member;
separate exact approval creates the successor and makes that predecessor History.
Already-attached predecessors cannot be changed or removed through this control.
Conflicting local text or late observations remain available for explicit review
and discard. Failed attachment retains the path and original review for retry.

For deferred native acceptance, open a retained supplemental Inbox Knowledge
draft, acknowledge owner edits/comments, enter a saved Current predecessor path,
and attach it. Confirm pending typing/navigation guards, unchanged owner wording,
complete before/proposed History text and read-only History editing. Inspect the
full pair before separate approval; restart and confirm the same revised review.
Headless state/widget checks do not establish overnight GUI or owner acceptance.

Each eligible Draft new-note member offers **Revise new-note destination** with
its current target and a retained new path in the same folder. Acknowledge full
owner text/comments first. The exact submitted review, member, path and generation
bind the acknowledgement; pending revision fences typing and navigation. Failed
or malformed replies retain the original review and path input for explicit retry.
A same-path request keeps the exact version and timestamps. Knowledge/History
pairs retain their generated read-only History member; bound Source, direct intake,
visual, asset and Replace destinations have no rename control. This revises review
work only; the revised full proposal still needs separate exact approval.
Headless state/widget checks do not establish interactive usability; exercise
rename, error/retry, restart and exact approval in the deferred morning journey.

State editing offers Open/Waiting/Blocked; Completed work stays immutable. Full
edits recover after 500 ms through the same AppWorker boundary;
only acknowledgement establishes recoverability. Older replies preserve later
typing. Comments attach to the whole proposal or an exact UTF-8 selection;
unresolved anchors retain their old quote and require explicit reattachment.
Failed comment saves retain copyable draft text and guard leaving until it is
acknowledged or explicitly discarded. Rewrite freezes provider/model/effort and
supports Stop; a late result retains conflicting local text instead of replacing
it. These review operations keep vault knowledge unchanged. Exact approval opens
a full captured confirmation; group approval includes only its displayed records
and may stop after an earlier independent proposal.
Captured group confirmation numbers its complete snapshots and offers **Move
earlier / Move later** for consequences. Sources remain first and pinned;
selection follows each proposal across moves. Confirmation uses the latest
captured order with the original operation IDs and exact review stamps. Closing
the dialog discards only this transient order; stored groups and owner edits stay
intact. Headless state/widget checks do not establish interactive usability.
Controls remain guarded until
the application outcome and current review are acknowledged. Activity pages show
recorded successful changes and full historical approval snapshots; recovery
inspection/reconciliation reports actual pending/uncertain outcomes without
repeating installation. Failed application refreshes journals because an error
can follow recorded file effects. Activity exposes full historical Undo review;
an Applied snapshot exposes each original Trash member's exact restoration.
Action-only operations containing only Replace members offer complete before/
previous-detail review. Confirmation restores previous details as a new revision,
preserving immutable origin and history; changed or Completed Actions refuse.
Action-only compensation works without a vault; file confirmation, scoped Trash
restore and repair still require a bound vault.
Action creation and mixed file/Action Undo remain unsupported. Scoped Trash
restore remains a file operation. Preview never applies work, and stale previews
or mismatched receipts cannot acknowledge confirmation.
Interrupted operations offer separate full Finish/Restore review with captured
observations, retained comments and explicit direction. These requests are frozen
through confirmation; workflow can refuse later changed files. Errors retain
inspectable operation/attempt identities and never trigger automatic retry.
New proposal retains complete title/path/body widgets for Create, Replace or
Trash. Replace/Trash first load the exact saved source through AppWorker; stale
captures refuse creation. Switching to Trash preserves local body text until an
explicit discard. Creating review work freezes its full request and UUID; replay
may return a later edited review, and acknowledgements never replace later input.
Failed or unsubmitted input stays copyable and guards leaving. A changed payload
needs an explicit separate proposal; submission alone does not apply Markdown.
Unsubmitted form input is transient; only acknowledged creation is recoverable.
An acknowledged completed answer can explicitly prefill its full bytes and session
into this form. Failed, provisional or oversized answers cannot become truncated
drafts. AI writing uses a real stored seed Draft, comments and owned Rewrite.
GUI/IME/accessibility and owner acceptance are tracked separately.

### Native EML/DOCX and text Inbox


The guided Inbox uses the full document area, with a retained-item list and focused
**Import → Read email/attachments → Review proposed notes → Approve** stages.
A successful import appears and is selected without manual Refresh; reading starts
locally, with cancellation available. The readable toolkit view keeps document
links, HTML and image URLs inert. Only checked retained bytes supply images, joined
to their exact source and occurrence. Named attachment reading and original
inspection share that selection. Missing content and document fidelity gaps remain
visible; raw Markdown, MIME nodes, hashes and quotas are under Evidence details.
Paste, batch/analysis tools and original management remain secondary controls.
Advanced tools occupy a separate view with Back, avoiding duplicate control IDs.

Exact saved extractions can be discovered by retained capture ID after restart,
including when the original is unavailable. One saved version opens as labelled
historical evidence; multiple versions require a choice. Returning to an item can
restore its previously explicit version, never silently choose the latest version.
Sidebar presentation keeps imported/selected items visible beyond the FIFO page.
Source title/path suggestions are visible and editable before creation; per-item
input survives switching. Retained Source edits use the existing exact review.

**Propose notes** explicitly retains a pending extracted Source and investigates
its checked private binding using the acknowledged provider/model/effort. Changing
the view or selection generation before admission prevents later automatic model
submission. Reading, reopening and Source-only preparation call no provider.
**Save as Source only** retains an offline review draft; approval remains a separate
exact-review gesture, with original bytes retained. Related Source, Knowledge and
Action cards open existing review, comments/Rewrite and selected group approval.
The image-preserving guided AI path accepts a pending or exactly Applied
extracted Source. After offline Source-only approval/restart, Propose notes
reuses the retained extraction and pictures to prepare new Knowledge/Action
review without creating or approving another Source. Its binding retains the
approved Draft stamp from the receipt for the current Applied review version;
new admission and effects recheck original, Source bytes/assets and unique
identity. Rejected, applying or uncertain Sources require review/recovery first.
Sources never silently fall back to text-only analysis. Pasted text can
be saved as Source and investigated through the existing saved-Source tools;
these supported-state limits are explained before a guided AI request.

P2 adds an explicit EML/DOCX file chooser over `CaptureBinaryInbox`, with retry
bound to the same exact bytes. Extraction review shows decoded text, source/parent
relationships, every repeated image occurrence, quotas/consumption and gaps.
Owner-triggered original inspection uses Quick Look over exact retained bytes in
an owned temporary preview; explicit Close, replacing the preview or quitting
joins its process group. Navigation retains the owned preview until one of those
actions.
Unsupported XLSX attachments remain visible as retained/unprocessed.

A prepared pending Source can be selected for private investigation before
approval. The same owned Rig turn prepares grouped knowledge and related Action
reviews; comments and Rewrite keep existing version guards. Group checkboxes
select displayed stamps and require the matching pending Source prerequisite.
Review exposes full proposed changes, recorded interpretation/reasons and retained
evidence. Unselected members remain pending. No Source or Action is applied by
inspection, extraction, model completion or Rewrite. The historical saved-Source
analysis/inline-PNG annotation paths below remain available for old saved work.


Inbox captures deliberate exact UTF-8 text, Markdown, email and Teams copies
through AppWorker. Capture fields remain intact after acknowledgement, failure or
navigation. Inventory pages and original inspection expose workflow availability;
full originals/conversion previews are read-only and copyable without truncation.
Checked batches retain 1–8 complete original snapshots across page changes, with
explicit removal, progress, cancellation and exact failed-admission retry. Refresh
does not hide a retained batch retry. Late replies cannot replace a newer view.

Source preparation retains the entire typed original/conversion proof. Explicitly
open its form, create the review draft, then use existing exact approval. Source
body/kind/destination are fixed; title and temporary comments remain editable.
The Source review body stays read-only after creation. Unsent forms guard leaving;
opening consumes only successfully validated preparation. Source approval never
deletes the original. The [Inbox plan](../../docs/work/active/text-email-inbox/plan.md#native-manual-acceptance-scenario)
contains the reproducible native acceptance scenario and pending qualification.

Saved Source analysis uses the same owned Ask lane. In a saved Inbox Source's
Sources panel, **Inspect this Source for analysis** opens Inbox through the usual
unfinished-input guards and requests a fresh full proof from AppWorker. Inbox also
accepts an explicit saved Source path. Inspection does not start inference. Review
the complete read-only Source, choose an acknowledged provider/model and effort
in Settings, then explicitly analyze knowledge and Actions. The workflow qualifies
Inbox eligibility and constructs the domain prompt; the desktop only freezes the
typed request and correlates responses. Global Stop remains available after
navigation, and partial text remains provisional until local finalization.

For a saved inline-PNG Source, **Inspect saved PNG** requests the complete checked
image bytes and displays the actual PNG, original alt/title, occurrence and full
Source/asset device, inode, length and SHA-256 proofs. Conversion preview also
renders the actual image. Exact Source/asset approval preserves evidence; its
interpretation remains pending. **Interpret this PNG** explicitly uses the same
acknowledged provider/model/effort and owned Stop lane. The complete captured
Source and answer remain available; model wording is tentative.

After a Completed visual analysis, **Prepare tentative annotation** requests a
provider-free draft and shows its full proposed Source text. **Create annotation
review** and **Open annotation review** are separate explicit actions. The ordinary
proposal review permits owner wording edits and exact separate approval; preparation,
creation and navigation perform no durable Source write or automatic approval.
Changing the selected Source or analysis invalidates pending presentation replies.
Restarted analysis inspection requires fresh exact PNG inspection before annotation
preparation. Failed/interrupted/unfinalized work remains incomplete and preserves
evidence.

The retained analysis UUID can be inspected after restart without a model or
provider call. Its captured Source, recorded selection/status and full answer
remain separate from fresh Source inspection. Returned knowledge and Action
drafts open the ordinary guarded proposal review and require exact approval.
Retained conflicts show their title, summary and Open/Resolved/Dismissed state.
**Open in Needs Review** opens that exact finding through the existing unfinished
proposal/comment/editor recovery guards, including findings outside the current
page. Navigation is bound to the selected analysis and its recorded member; a
stale analysis/row cannot select another conflict. Needs Review retains both
saved quotations and separate fresh evidence observations. Resolve/Dismiss
changes the operational finding only, without changing knowledge or deleting
evidence. No drafts, findings, cancellation or successful model ending establish
complete semantic ingestion. Safe original-copy deletion remains later Stage7
work. Native/live/owner acceptance remains pending.

Manual scenario: approve a fresh synthetic email Source, browse it under Source
and open Sources, then inspect it for analysis. Confirm exact Copy and read-only
text, and that inspection alone makes no model request. With separately authorized
live scope and explicit selection/effort, start analysis, navigate away and Stop
or wait. Inspect the retained UUID, open each independent draft and edit/reject or
exactly approve it. Restart, inspect the same UUID with no model selection, and
compare the original Source/intake bytes. Do not use private/original data or infer
live/native qualification from automated widget tests.

### Manual Action review acceptance

Use a fresh explicit data directory and synthetic fixtures; no provider is needed.
While the desktop is closed, create the Action-only draft in the
[CLI scenario](../brn/README.md#manual-action-acceptance), then open this same data
directory in the native app after the CLI exits.

1. Select the draft in History. Inspect its full candidate fields and empty source
   list. Change title/description, Waiting, owner and both dates; type an incomplete
   UUID into a reference field. It must remain visible and block approval/navigation
   until corrected or explicitly discarded. Remove that incomplete reference.
2. Wait for the exact edit acknowledgement, add a whole-proposal comment and inspect
   the current version. Quit/restart; acknowledged full fields and comment remain.
3. Open Approve. Check the complete frozen snapshot, then approve. The Action appears
   only after Applied; Activity retains its approved origin and full candidate data,
   while temporary comments are removed. Quit/restart and compare the CLI full read.
4. Repeat with two captured group drafts, including a bound Markdown member. A new
   group arrival must stay outside the open confirmation. Only shown members apply;
   a refusal stops later members. Compare exact fixture bytes and Action records.

Native/live and owner acceptance are pending until these controls are exercised on
the unlocked Mac. Save safe actual captures in the
[UI screenshot index](../../docs/ui/screenshots/README.md). Owned Action Rewrite
supports full Action-only/mixed suggestions with a bound AI vault and explicit
provider/model/effort. All fields remain editable/copyable; later incomplete raw
typing stays a conflict until explicitly resolved. Rewrite suggestions require
ordinary exact approval to affect real Actions. Action-only all-Replace Undo uses
the same captured review and restores prior details as a new revision; native
interactive acceptance remains deferred to the morning journey.

### New Action and related follow-up proposals

**+ New Action…** opens the ordinary retained proposal form without needing a vault
or provider. It retains all 14 raw fields, including invalid/incomplete typing;
Create validates the complete typed request through AppWorker. Proposal and Action
UUIDs stay fixed through retries. The Action title also names its proposal.
Optional saved Markdown sources are explicitly captured through AppWorker, with
full ordered path/fingerprint proofs and exact source Copy. Recapturing one path
replaces only that proof. Unsettled source reads block creation/leaving; late,
misbound or internally inconsistent replies cannot replace current captures.
Changed saved sources refuse creation while retaining input/proofs.

Create makes review work only. Open current proposal uses the existing full
review/edit/comment/exact-approval path. Acknowledgement never overwrites later
field typing. Pending creation blocks discard/separation. A changed payload after
submission needs **Start separate proposal from retained input**, preserving full
raw fields/proofs with fresh proposal and Action UUIDs. Copy all raw Action input /
proofs also includes the immutable submitted request. Unacknowledged local input
is transient; explicit confirmed discard does not change stored proposals/Actions.

A Completed Dashboard selection offers **New related follow-up…**, seeding a fresh
Open Action with the selected UUID in Follows up. Its other fields start empty.
The completed record remains unchanged before and after approval of the new work.

Manual acceptance with fresh synthetic data, no provider: open New Action, enter a
Unicode title/description, owner, Waiting and both dates. Try an incomplete UUID:
it stays copyable and prevents creation/leaving. Clear it, create review work,
inspect/edit/comment and explicitly approve the complete snapshot. Dashboard must
show the Action only after approval. Complete it, then select it in Completed and
create/approve a separately identified follow-up. Quit/restart and compare both
full records: original Completed, new Open with its follows_up UUID. With a bound
synthetic vault, capture two sources, recapture one changed path, and verify other
proofs remain exact; changing a source after capture must refuse creation.
Native observation, owner acceptance and automated qualification are separate.

### Dashboard and identified Complete

Dashboard in History opens the shared Action snapshot without needing a vault or
provider. Active defaults to Open/Waiting/Blocked; eight filters include Completed,
Overdue, Follow-up and All. Counts cover all retained Actions at the displayed
OS-local civil date. Refresh today resets the first page; Older page carries that
returned date and cursor. Due dates become overdue the following day; follow-up
starts on the named day. Completed contributes to neither date signal. Dependency
observations retain ordered unfinished/missing targets without changing state.

Select an Action to read/select/copy its full record, immutable approved origin
and dependency observations. Complete… freezes its entire before record and a
new operation UUID once; only the explicit confirmation submits it. A changed
selection/refreshed page or busy/closing app refuses that capture. Completion
never edits/reopens Completed work; later follow-up is a new approved Action.
Acknowledgement validates the whole exact receipt and refreshes visible counts.

Attempted requests and typed failures remain selectable/copyable/retryable across
navigation for this app session. Retry uses the same UUID/full payload, independent
of current selection. Unknown/misbound replies cannot claim completion; a later
failure cannot regress an acknowledged receipt. Only one completion is admitted
at a time. These are transient presentation records; application-owned ordinary
receipts and checked startup reconciliation supply durable recovery. Admitted
completion drains on shutdown. Do not treat a failed response as proof of no effect.

Manual acceptance: while the desktop is closed, use the CLI Action scenario with
two fresh synthetic approved Actions and canonical past/today dates. Open the same
data folder, choose Dashboard and inspect Active, each state/date filter and All.
Inspect/copy every field and origin. Open Complete…, inspect the exact snapshot,
close it without confirming and verify state is unchanged; reopen and explicitly
confirm. Active/date counts withdraw that Action, Completed shows the same origin
and a completion time. Quit/restart and compare the CLI full record/receipt. Try
navigation with unfinished initial proposal/review/comment/editor input: the
existing guards must preserve it before Dashboard opens. Native/owner acceptance
remains separate from automated widget/worker checks. Later-domain dashboard
summaries remain following roadmap deliverables.

Saved documents also expose **Links**. This inspects saved Markdown while retaining
unsaved editor text, showing exact occurrence/reference-definition quotes, source
and target UUIDs/hashes, and resolved, absent, ambiguous or incomplete outcomes.
The Vault browser's **Relationships** uses the selected Current/Source/History/All
scope and distinguishes explicit links from inferred provenance candidates. Each
25-edge page is a fresh observation; Previous/Next/Refresh replace the page and
do not promise a snapshot across requests. Selected proofs are read-only and
**Copy exact quote** preserves their complete bytes. Closing or changing scope
discards late replies; known Save/Reload/application effects invalidate affected
observations. Inspection creates no proposal and writes no Markdown.

**Needs Review** offers **Findings** and **Citation evidence** modes. Citation
evidence inspects saved Current notes in bounded 25-consumer pages and lists
Changed, Absent, Ambiguous and Incomplete citation observations. Incomplete means
unavailable or uncertain lookup; these observations do not establish that a claim
is false or stale. No or empty citations do not create a row. Coverage shows the
complete diagnostic count and explicitly labels a truncated diagnostic list.
An empty page can still offer **Load more** when uninspected consumers remain.

Selecting a citation row opens its complete exact saved consumer, full saved-source
provenance and original quotes inside Needs Review. These use separate read-only
widgets and exact Copy actions. The request binds the row's complete consumer
hash; changed or malformed replies require Refresh. View, page, row and selection
generations reject late replies and callbacks. This mode offers no Finding
capture, Resolve, Dismiss or inference. Refresh starts a fresh observation;
continuations are bound to the workflow's evidence digest. A confirmed vault
rebind or changed root clears derived proof and requires Refresh. Existing guarded
navigation retains unfinished note recovery, proposal input, review/comment input
and the composer. Automated headless state/widget verification does not qualify
interactive native usability; that remains in the single morning acceptance task.

**Needs Review** in History lists tentative findings in fresh 25-record pages,
filtered by Open, Resolved, Dismissed or All. Select a row to read and copy its
complete retained record, original source fingerprints and exact quotes. Inspect
current evidence reports drift or unavailability separately; it never replaces
the retained proof. Resolve and Dismiss explicitly close the exact displayed
version and change only operational queue state. Knowledge corrections still
need a reviewed proposal. These historical controls work without a current vault.

An inspected ambiguous source or unresolved saved link offers **Keep finding**;
the workflow recaptures fresh evidence before retaining it. Capture and closure
outcomes survive navigation. Failed requests remain fully inspectable/copyable
and offer an explicit exact retry, including without an available vault or current
selection; no mutation retries automatically. Inspect a recorded outcome to load
its full finding. Needs Review navigation preserves unacknowledged note, review,
comment and initial proposal input through the existing guards.

For manual acceptance, use fresh synthetic data with a saved note containing
`[missing](missing.md)` and BOM/CRLF/Unicode text. Inspect Links, Keep the saved-link
finding, then open Needs Review. Copy the full proof and exact quote; restart and
confirm retention. Replace the source externally, Inspect current evidence and
confirm Changed with the original quote intact. Resolve the displayed finding,
then select Resolved and inspect its complete terminal record. Repeat Dismiss on
another finding. Temporarily remove only the synthetic vault and confirm retained
history/closure remain available. Try navigation with unacknowledged typing or a
full unsent proposal form; input must remain guarded. An active synthetic Ask
that refuses New proposal must leave Needs Review usable. GUI/IME/accessibility
and owner acceptance remain pending separately from state/widget verification.

To try this offline, use fresh disposable data and a vault containing a managed
current note with `[source](brn://note/<source-uuid>)` and a managed source note
under `archive/`. Open the current note, type without saving, then inspect Links
and copy its exact proof. Confirm the typing remains. Relationships in Current
should exclude the archived endpoint; All should include it. Switch scopes during
a request or close the pane and confirm a late reply does not reopen it. Open the
archived note in its read-only evidence view and inspect its Links. Refresh after
an external change; Save/Reload should clear affected old observations. Actual
GUI, accessibility and owner acceptance remain pending.

The initial Replace proposal form can **Prepare link** to saved evidence. Load
the exact current consumer, enter a scalar target path and literal label, then
**Inspect target**. A unique managed target may be current or archived; UUID
resolution follows a moved target. Preparation uses saved consumer bytes and
requires an empty proposed body or its exact captured text. Authored replacement
text and invalid multiline labels remain retained on refusal. It creates no
review record and changes no Markdown. A prepared form fixes destination/kind and
retains both complete source fingerprints; title and full body remain editable.
**Copy both full source bindings** preserves the complete proofs. An explicit
separate proposal keeps those proofs under a new UUID. **Create review draft** and
exact approval remain separate actions; changed sources refuse later admission or
application.

Manual acceptance uses a fresh managed current note and a managed archived source
with distinct `brn_id` UUIDs. Choose New proposal, Replace, enter the consumer path
and title, and load its exact source. Keep the body empty or use captured text;
inspect the archived target and prepare a Unicode/punctuation label. Verify both
full source proofs and the entire original prefix in the proposed body. Edit the
title/body, Create, inspect the full review and approve the exact version. Vault
bytes change only after approval; the archived source stays exact. On a separate
attempt, change title/body/target/label while inspection or preparation is pending:
late replies must retain current input. A changed consumer or target must refuse
without overwriting it. Try at 480×480 and Copy both proofs/full body. If the
consumer has unfinished editor work, the existing guards preserve recovery and
refuse approval over that work; resolve it explicitly and recapture fresh sources.
Actual unlocked GUI/IME/accessibility and owner acceptance remain pending.

Native retrieval offers a one-time prompt per stored consent decision, showing
pinned source, bytes/cost and destination. Decline makes no network request.
Later Download requires fresh explicit approval; its destination is not passed
as a startup load path. Cancel Download targets the active installation UUID.
Downloaded is not Installed; activation/indexing errors end progress honestly.

`--legacy` is unknown. Legacy database, sidecar and mixed authority markers
refuse before opening authority or writing a startup probe; empty directories
use the current workspace. Startup never opens or migrates old data.
`--codex` is unknown. `--model-dir` supplies an explicit local retrieval model
path; account and chat model selection remain independent Settings actions.

## Interfaces and source

[Entry point](src/main.rs), [layout model](src/layout.rs) and [tokens](src/tokens.rs) (GPUI-free, default-feature tests), [native shell](src/native/mod.rs) with [theme](src/native/theme.rs) and [regions](src/native/shell/mod.rs), [CLI tests](tests/cli.rs). Layout and appearance persist to `layout.json` in the data directory. See the [workspace shell decision](../../docs/architecture/decisions/2026-10-01-workspace-shell.md) and the [UI feature backlog](../../docs/ui/feature-backlog.md).

The native workspace has History and Vault rails around a document/chat centre.
Narrow windows collapse rails and use Document/Chat tabs; Focus hides the rails.
Closing a document waits for its latest buffer recovery acknowledgement. Later
edits survive older Save and recovery acknowledgements. Settings uses the toolkit
modal host for appearance,
rail widths and layout reset. It also shows the last usable internal-state backup,
copy time (unknown for the startup copy), retention warnings and backup failures.
**Back up current state** checks for changed state; **Refresh backup status** reads
the current status. Backup responses preserve editor, composer and proposal input,
and report failures separately from the operation that committed the work.
Layout preferences are stored separately from the authoritative workflow data.

The three dividers support pointer dragging and keyboard resizing: Tab among
chrome controls to focus, then ←/→ for 8 pt or ⇧←/→ for 32 pt. Grabs preserve
the pointer offset within the divider; only changed layouts persist when a drag
ends, including when the window deactivates. Tab/Shift-Tab inside multiline
composer/note editors indent/outdent (toolkit behaviour); leave editors
via ⌘L, menus, Escape or other applicable shortcuts.
The BRN, View and Navigate menus expose Settings
(⌘,), Quit (⌘Q), History (⌘0), Vault (⌥⌘0), Focus (⇧⌘↩), New Chat (⌘N),
Focus Composer (⌘L) and Cancel Running Action (⌘.). History navigation stays
independent of Ask. Quit waits for latest note recovery and joins admitted
workflow mutations. Escape remains local to editors and the
toolkit Settings dialog.

Dock/system termination cannot veto exit in the pinned GPUI toolkit. The final
hook submits no new recovery flush. It synchronously drains admitted AppWorker
mutations before returning GPUI's timed future; this can block the UI during
defensive system termination. Guarded close joins on the background executor.
Guarded window close, Quit, document close and note switch retain the current
buffer until recovery is acknowledged. Unacknowledged typing may be lost during
system termination. Restart reconciliation does not replay filesystem writes.

## Dependencies and features

Depends on workflow DTOs for GPUI-independent AI/editor state tests.
Default features are empty; `native-ui` enables GPUI; `native-retrieval` includes
native UI and workflow native retrieval. Normal native builds enable both.

## Verification

Run from the repository root:

```sh
cargo test -p brn-desktop --locked --offline
cargo test -p brn-desktop --features native-ui,native-retrieval --locked --offline
cargo test -p brn-desktop --features native-ui,native-retrieval,native-test-support --locked --offline
cargo build -p brn-desktop --features native-ui,native-retrieval --locked --offline
bash scripts/verify-desktop-shell.sh --native
# Existing disposable absolute data directory only:
cargo run -p brn-desktop --locked --offline -- --data-dir /absolute/disposable/data --headless-check startup
```

Native interaction requires macOS Apple Silicon and an unlocked session. Use explicit disposable data. Preserve dirty-state guards, focus/selection, exact original quotes and generation-aware response handling.

`native-test-support` enables the pinned toolkit's headless widget test context;
it is separate from normal native builds. Editor-state tests cover exact BOM/CRLF/Unicode text, byte limits and
receipt/close/retry scheduling. Native-feature tests round-trip the GPUI-kit
Rope backend through real AppWorker Save and restart recovery, and verify that
admitted Save drains before the defensive timed quit future.
They do not establish widget rendering, IME/accessibility behavior, OS chooser
usability or human acceptance. Those still require native observation.

Read the [architecture overview](../../docs/architecture/overview.md), [invariants](../../docs/architecture/invariants.md) and [verification guide](../../docs/development/verification.md) before changing contracts.
