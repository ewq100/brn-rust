# Reading-quality fixture

Use this fixture with [specification §6.5.1](../BRN_UX_Rebuild_Spec.md#651-long-article-reading-and-outline), [§8](../BRN_UX_Rebuild_Spec.md#8-layout-and-sizing), AC-26, AC-41, AC-42 and evidence case VE-08.

**Status: source material prepared; native import, implementation and all native acceptance checks are Not run.** Every claim and number is synthetic. These files require no network access and contain no owner data.

## Files and coverage

- [Article](reading-quality-article.md): several screens of prose; H1–H4; one H2-to-H4 skip; two “Review” headings under different parents; long and Unicode headings; quote; ordered and unordered lists; fenced code with fake headings; normal and wide tables.
- [Local chart](assets/reading-quality-chart.png): five invented median times, exactly matching the normal table; descriptive alternative text and a caption are in the article. Keep this relative path intact.
- Article links to **Source A** and **Source B** are same-document anchors. They test link presentation, local navigation and return position only. They do not create BRN evidence records, prove source access, or imply that Markdown import stores provenance.

## Set up an isolated native test

1. Create a fresh, explicitly synthetic BRN workspace/data directory through the build's existing supported setup. Keep owner workspaces and this repository fixture unchanged. Record platform, build/commit, source filename and the actual import/service route.
2. Import a copy of the Markdown and PNG through the existing supported BRN document-intake, CLI or checked-service route. Use the existing asset/attachment mechanism to resolve the relative image when supported. Do not write SQLite directly or add a new importer to make this test appear to pass. If the route cannot retain an element, report that limitation and use an existing supported service harness where available; mark unsupported native coverage separately.
3. For **AC-26/AC-42 recorded-source identity and access**, create separate synthetic source records through existing supported BRN services. Populate them with the Source A/B text and connect the article through the supported provenance/link mechanism. Record the actual article, source, revision and access scope identities. Do not substitute the in-article anchor links for this setup. If the mechanism is unavailable, that part remains Not run or blocked with its reason.
4. Keep a runtime copy with no real Markdown headings for the empty-outline case. Remove heading markers only outside fences, preserving the code example as code. Keep another synthetic document with similar heading names to catch cross-document stale targets. Neither variant needs to be committed to this fixture directory.

## Native validation sequence

Run the sequence in **Light and Dark**, at **1280 × 900** and **900 × 600 usable client area**. Capture evidence for each actual combination; a static preview is not native evidence.

| Check | Expected outcome |
|---|---|
| First open and navigation | Outline starts closed. Open it by its labeled control; use pointer and keyboard. Both Review entries reach their own sections; the long label is readable, Unicode survives, and fake code headings are absent. The skipped level does not invent content. |
| Scroll and focus | Selected/current heading follows reading position without stealing focus. Targets appear below the fixed toolbar. Closing a compact outline with Escape returns focus; selecting a heading returns to that section. |
| Unsaved heading rename/remove | In the existing editor, rename the first Review and remove another heading marker without saving. The applicable displayed draft outline refreshes; the second Review remains independent. Read/Edit, pane and theme changes preserve text, selection and recovery state. No stale entry jumps to a similar title. |
| Save, discard and reopen | Exercise the existing checked Save and discard/recovery routes. Outline and save labels reflect the actual displayed content generation. Reopen the same document and verify identity and preferences; switch to the other document and back. |
| History, proposal, Apply and Undo | Display an older revision and a supported rewrite candidate. Identify which surface the outline describes, or hide it if ambiguous. Checked Apply and receipt Undo refresh the correct headings. Preserve conflict/recovery behavior; do not confuse editor Undo with saved-operation Undo. |
| Compact supporting panels | Request Outline, assistant, sources and comments one at a time. Hide incompatible supporting views temporarily, preserving their drafts/selection/scroll. Never stack sheets, duplicate a composer or squeeze prose. Restore explicit pane preferences when width permits. |
| Reading and evidence | Prose wraps; code and wide table scroll locally; image keeps its ratio and legible alt/caption. Local anchors return predictably. Separately test recorded sources against their actual identity, scope and availability limit. |
| Empty and fault states | The runtime no-heading variant shows “No headings yet”. Use controlled runtime asset/source resolver faults or a supported test harness for unavailable cases; preserve article, recovery and return position. Restore the fault afterward. Do not break repository paths or replace source identity to simulate failure. |

Record **Not run / In progress / Pass / Partial / Fail / Blocked / Not applicable**, concrete observations and evidence for each native case; document the scope reason for Not applicable. Keep source preparation, successful import, automated checks and native observations separate. This fixture establishes none of the latter by its presence alone.
