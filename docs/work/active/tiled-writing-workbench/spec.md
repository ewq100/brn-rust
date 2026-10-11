# Tiled writing workbench — accepted scope

The owner accepted the tiled prototype and requested implementation with GPT-6.1 Sol subagents and a PR. The leading flow is writing/refining one note beside its full conversation. Keep the current GPUI stack, one editor entity, checked writes, exact-candidate review, and receipt Undo. Do not add the prototype's paragraph buttons: select the actual word, sentence, or paragraph instead.

## Required behavior

- A note opens in a compact tiled workspace with readable document left and full selected linked chat/composer right; narrow views preserve access to both. Review keeps chat available. Preserve note buffers, selection context, and separate composer drafts across navigation.
- Selecting a supported saved passage opens a comment composer. Add comment persists its exact note/version/range/quote without starting AI or navigating away. Multiple comments accumulate as pending feedback. Excluded comments remain available. Unsaved buffers and stale/unmapped passages are explicit; never silently save or guess a location.
- Rewrite with AI captures the saved note, included pending comments, and all posted owner/assistant messages from the selected linked chat into one bounded durable request. The UI names the scope. Later feedback stays outside this snapshot. Too-large context fails explicitly rather than silently truncating. Unposted drafts are not secretly submitted.
- The explicit rewrite command requests a complete note proposal, even under ordinary maintenance delegation. It does not grant direct/protected write authority. The exact proposal is reviewable with the existing edited-candidate, Apply/dismiss and Undo services. Enforce snapshot versions and the existing dirty guards. Continue or refresh must not turn the saved rewrite proposal into an automatically applied change.
- Generating/dismissing proposals does not resolve comments. After an actual successful Apply receipt, the owner may explicitly mark captured comments addressed. Retain their history; later comments stay pending. Undo must not leave those comments falsely treated as addressed.
- Full chat remains a real conversation, with existing provider status, errors, cancellation and Stop-and-send behavior. Saved rewrite requests survive failed startup; retry/continuation must preserve the explicit proposal-only intent and avoid duplicate comments/messages.

## Exclusions and qualification

No framework/dependency replacement, second document store, CRDT, new draft domain entity, arbitrary hunk acceptance, paragraph buttons, provider/account changes, real user-data access, release, or prototype HTML in production. Reuse existing record types where adequate; no schema bump is planned. Current ordinary chat/delegation remains unchanged outside this explicit writing command. Native interaction and live-provider acceptance remain unobserved until separately performed; headless native tests are not owner acceptance. The previously deferred native observation remains deferred. This assignment's delivery is a PR; no merge is required by the current request.
