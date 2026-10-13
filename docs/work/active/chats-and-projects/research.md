# Polished chat experience — discovery research

**Date:** 13 October 2026. **Status:** Primary-source research and recommendations, not accepted requirements or implementation evidence. Supports the chat-first stage of [the feature brief](spec.md). Sources describe documented behavior; no private account UI was inspected.

## Owner scope clarified during research

The owner wants **both quiet mode and a message queue**, and explicitly wants **both queueing and steering**. The owner subsequently decided that **Stop stops current work and pauses the queue, with waiting messages saved until explicit Resume**, and **failure or restart also preserves a paused queue until explicit Resume**. **Enter queues during an answer; a separate explicit Steer action redirects current work.** Attachment persistence remains unanswered.

## ChatGPT findings

OpenAI's current prompting guide distinguishes **Steer**, which adds information to the current Codex run, from **Queue**, which saves it for the next run. ChatGPT desktop offers a default follow-up setting and a shortcut for the other behavior. Queued messages appear above the composer and can be edited, reordered, sent or deleted. This documentation concerns Codex/desktop work, rather than promising the same controls in every ordinary ChatGPT surface. [OpenAI prompting guide](https://learn.chatgpt.com/docs/prompting).

ChatGPT's file handling is not simply a continuing link to the original file on the user's computer. OpenAI's plugin APIs upload a user-selected file and return a file identifier; optionally save it to Library; and provide temporary download URLs. This proves the plugin file model, not that all chat attachments are automatically saved in Library. [OpenAI plugin file APIs](https://developers.openai.com/plugins/reference).

OpenAI's Enterprise Work documentation separates conversation retention, Library files, project files and transient uploads. Deleting a chat does not delete Library files; project files remain until project deletion under applicable rules; Enterprise transient uploads outside Library can expire after 48 hours unless another setting applies. This is Enterprise/Work evidence, not a universal personal-chat expiry or an indefinite availability guarantee. BRN must decide its own local attachment retention explicitly. [ChatGPT Work retention](https://learn.chatgpt.com/docs/enterprise/chatgpt-work-overview).

## Documented patterns

### Sending while an answer is running

Zed's Agent Panel streams replies with tool-use indicators. Messages submitted during generation enter a queue by default and are sent after generation finishes. Users can edit/remove queued messages. An explicit **Steer** choice delivers a message at the next step boundary; it is limited to Zed's own agent because external-agent boundaries are not detectable. **Send Now** or Stop can interrupt immediately. These are three distinct behaviors: finish then answer, steer ongoing work, or interrupt it. [Zed Agent Panel](https://zed.dev/docs/ai/agent-panel).

Carbon AI Chat also documents sequential queued sends when its send handler returns a promise. It shows a loading indicator when the reply or first stream chunk is delayed and a visible error on timeout. Stop triggers a cancellation signal; the host must actually stop work and finalize the response. A stopped partial answer can remain with a **Response stopped** accessibility announcement. Its experimental message update path supports streaming and regeneration by stable message identity. This is a UI/backend contract, not proof that all upstream processing stops. [Carbon server communication](https://chat.carbondesignsystem.com/tag/latest/docs/documents/Server_communication.html).

### Reading without losing your place

Microsoft Web Chat documents following incoming messages while at the end, and pausing automatic scrolling for sufficiently long output so users can read it. It provides explicit scroll navigation and treats reaching the end as acknowledgement. Its API recognizes streaming revisions as one activity rather than unrelated messages. These are useful precedents for a stable transcript. [Microsoft Web Chat hooks](https://github.com/microsoft/BotFramework-WebChat/blob/main/docs/HOOKS.md).

Zed provides jump/navigation controls, editable sent prompts, copying selected text/plain Markdown or a whole response, and opening a thread as Markdown. Completion notifications and sounds can each be disabled. Those notification controls differ from queueing messages. [Zed Agent Panel](https://zed.dev/docs/ai/agent-panel).

### Attachment clarity

Claude supports file selection, drag-and-drop and pasting images. It distinguishes individual-chat uploads from project files available across conversations. Its documentation explicitly distinguishes text extraction from visual understanding: non-PDF documents are text-only; PDF visual support depends on size. This supports showing conversion coverage instead of implying that attaching a document makes every image/table available. It does not establish BRN's retention contract. [Claude file uploads](https://support.claude.com/en/articles/8241126-upload-files-to-claude).

Carbon's experimental upload flow shows file chips, allows cancelling pending uploads, disables Send while uploads are in flight, and marks rejected uploads as errors. It separately explains persisted references versus in-memory files: raw files do not survive serialized history. The visible chip alone therefore does not prove restored content is usable. [Carbon structured data and uploads](https://chat.carbondesignsystem.com/tag/latest/docs/documents/Server_communication.Structured_data.html).

### Accessibility and motion

W3C's status-message guidance uses polite announcements without requiring focus to move. ARIA defines a log as a meaningful sequence of added entries and a status as advisory information. These are web semantics; BRN's native UI needs equivalent macOS accessibility behavior, not literal HTML roles. [W3C status technique](https://www.w3.org/WAI/WCAG21/Techniques/aria/ARIA22), [WAI-ARIA 1.2](https://www.w3.org/TR/wai-aria-1.2/#log).

W3C's animation guidance recommends honoring reduced-motion preferences and removing unnecessary interaction animations. Its pause/stop/hide guidance concerns automatically moving/updating information alongside other content, with explicit applicability conditions. It does not require every user-requested streamed answer to be treated as forbidden motion. [Animation from interactions](https://www.w3.org/WAI/WCAG21/Understanding/animation-from-interactions), [Pause, Stop, Hide](https://www.w3.org/WAI/WCAG21/Understanding/pause-stop-hide.html).

## BRN baseline and gaps

At isolated baseline `082ac14f58e78f78a03fff59c89ec562b72d63ca`, chat text is already rendered as Markdown, including a provisional streamed answer. The successful final response supplies one durable assistant Message; interruption/failure is not completion. Existing tests and contracts do not establish smooth scrolling or native selection while streaming. [Chat renderer](../../../../crates/brn-desktop/src/threads_writing.rs), [response contract](../../../architecture/threads-behavior.md#implemented-response-and-authority-contract).

Current **Stop-and-send** durably posts an instruction, fences/cancels the exact run, waits for its worker, then starts fresh. It is one pending replacement, not a queue or same-run steering. Other destinations preserve separate session drafts, but ordinary Ask is blocked while the desktop worker runs. `ComposerDrafts` explicitly holds session state, so persistence of unsent chat drafts across restart needs separate qualification/implementation. [Desktop dispatch and handoff](../../../../crates/brn-desktop/src/threads_ui.rs), [draft state](../../../../crates/brn-desktop/src/threads_state.rs), [handoff witnesses](../../../../crates/brn-desktop/src/threads_steering_tests.rs).

Current full-file import creates protected general-BRN notes and retained converted assets. That path cannot simply be relabeled “chat-only attachment”: general retrieval, source access and retained history must enforce the selected chat context. Current Thread state is Open/Resolved; normal chat history and Archive are product changes. [Import service](../../../../crates/brn-threads-app/src/intake.rs), [record types](../../../../crates/brn-threads-core/src/types.rs).

## Recommendations to discuss for BRN

These are proposed product qualities, not owner decisions:

| Area | Candidate behavior |
|---|---|
| Composer | Remain editable during work; support multiline text, paste/drop attachments and visible Send/Queue/Stop actions. Preserve unsent text and attachment state when navigating away. |
| Streaming | One stable assistant response area, readable Markdown as content arrives, honest compact progress, and a clear terminal state. Avoid transcript replacement that jumps the viewport or destroys text selection. |
| Reading | Follow output while the user is following the latest response; stop following when they read older content. Show a return-to-latest control and retain reading position when reopening. |
| Queue/steer | Both are owner-selected scope. Show queued text separately from sent messages; allow edit/remove and show order. Distinguish an ordinary queued follow-up, steering the active work, and Stop-and-send. |
| Stop/retry | Label interrupted output as incomplete. Keep saved user messages, attachments and accepted changes. Retry must not duplicate the original request or automatically replay a completed write. |
| Attachments | Show preparing/ready/failed status, conversion gaps and removal; distinguish “usable in this chat” from explicit **Keep in BRN**. A restored attachment must have retrievable content, not merely a remembered filename. |
| Actions | Copy selection/answer and retry the latest failed answer are useful foundations. Historical prompt editing and alternative-response navigation need a defined history model before inclusion. |
| Calm/accessibility | Offer compact progress rather than distracting activity; respect macOS reduced motion; keyboard-accessible controls with names/focus states; announce meaningful state changes rather than every token. |

Acceptance should include long streamed answers while scrolled up, copy/selection during streaming, failure after partial output, repeated Stop, navigation/restart with a draft or queued messages, attachment conversion failure, and VoiceOver/reduced-motion behavior. These are candidate acceptance situations, not claims of current BRN support.

## Consequential unknowns

1. What does quiet mode hide or delay: intermediate activity, streamed answer text, completion notifications, or some combination? It must not conceal actual requests for decisions/review.
2. Enter-as-Queue with a separate Steer action is settled. Stop, failure and restart pause with saved messages until explicit Resume; Archive uses the same stop-and-pause contract. Consecutive-message batching remains a recommendation needing a defined message-history policy if included; the confirmed scope does not request automatic merging. Provider/model changes still follow the existing explicit-selection and run-fencing contract.
3. Should supported attachment content remain usable when the original disappears? This was already raised in discovery and remains unanswered by asking for competitor research.
4. Does stopped partial text become durable history, and how is Retry related to it? Preserve the established one-saved-final-answer and checked-write rules when deciding this.
5. Is latest-answer retry sufficient for v1, or are historical editing and branch navigation required? No competitor convention settles BRN's history meaning.

No source supplies a universal definition of “polished.” The recurring useful qualities are legible streaming, a usable composer, stable reading position, visible state, deliberate interruption and reliable recovery. Technical planning follows owner decisions on consequential lifecycle questions.
