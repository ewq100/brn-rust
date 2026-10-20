# General search audit

- Checkout: `/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust`
- Actual HEAD: `cf22ccbb365f7328cdcd1ea3ad79b25acd1872e7` (`docs: checkpoint published CI candidate and remote progress`); parent/source production head is `bc8a08e6` as supplied in the task context. The only dirty paths are lead-owned `docs/work/active/vacation-campaign/campaign.json` and `docs/work/overview.md`; neither was edited.
- Scope: read-only review of general Notes Search and ⌘K launcher query, paging, asynchronous completion, and scope contract. No build, test, Git mutation, native interaction, or external call was performed.

## Finding

No actionable stale-result or wrong-scope defect found in this bounded pass.

General Notes Search starts a new request by resetting both lane request identities/cursors and clearing note hits (`threads_search.rs:150-160, 162-230`). Completion is accepted only for the request ID still awaited by a lane; superseded results return without mutation (`:233-281`). A fresh query suppresses prior chat pages while it is pending (`:453-458, 597-613`). Notes and chat Messages page independently, with each lane tracking its own cursor and in-flight request (`:283-328`). The launcher clears its displayed messages on each query and checks a monotonically increasing request ID before applying completion (`:330-374`). Opening ⌘K resets its query and dispatches the empty-query reset (`threads_ui.rs:2465-2476`).

The general-scope behavior matches the accepted contract: owner search includes saved final Messages, with general chats in general search and project/private chats restricted to their own scopes; archived/resolved chats retain scope and Trashed chats are excluded (`docs/architecture/threads-behavior.md:80-82`). The desktop uses the workspace owner search with `chat: None` (`threads_search.rs:194-203, 348-352`), leaving scope enforcement to the shared search service.

## Existing witnesses

- `threads_tiled_tests.rs:741-783` starts two note queries before completion and asserts only the newest query appears, then exercises Previous/Next.
- `threads_tiled_tests.rs:811-871` verifies general search returns saved chat Messages with context, opens the matching chat/message, and labels archived results.
- `threads_tiled_tests.rs:875-970` verifies note and chat lanes page independently, retain the other lane while paging, and restart both at page zero for a new query.
- `threads_omakase_tests.rs:1894-1938` verifies launcher Message search, archived labeling, query replacement with a no-match result, and activation.
- `threads_search.rs:330-374` has an explicit launcher request-generation guard. The launcher-specific overlap path lacks a direct delayed-completion test, but the shared worker request is cancel-on-drop and the guard is straightforward; this pass found no consequential unprotected behavior warranting a duplicate UI test.

The checked service owns canonical scope/privacy filtering and indexed search behavior. The desktop witnesses are sufficient for the general UI boundaries inspected here; no mirror test or production correction is selected.

## Limitations and next action

Source/test inspection only; tests were read, not run. Native behavior was not observed. No implementation follow-up is indicated unless a concrete failing general-search or launcher scope transition is reproduced.
