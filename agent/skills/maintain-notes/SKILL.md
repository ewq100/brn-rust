---
name: maintain-notes
description: Compile source-grounded ordinary knowledge, save a reusable synthesis, or review affected pages and stale candidates.
---

# Maintain the working wiki

Ordinary knowledge is a maintained account of useful subjects, connected to evidence and related work. Page purpose and protection are independent: a project, concept or comparison can be ordinary or protected; a protected reference reports claims without certifying them. Roles below are writing conventions within the existing Note model. Preserve the owner's personal writing and drafts according to the actual request.

## Read and choose the result

1. Read current related Notes, Sources, Links, Thread and Actions with `read_records`. Search the subject's explicit names and relevant terms; the bounded result is not a complete workspace catalog. Read relevant full passages with `read_note` at their observed versions and `read_source` for available supplied evidence. Finish with enough evidence to identify the subject, its scope, existing account, material qualifications and unresolved disagreements. Missing or partial evidence stays visible.
2. Choose **update**, **create**, or **keep in thread** before writing. Update the existing page when evidence changes or extends the same subject and scope. Create a separate page when it has a stable distinct subject, likely future use and enough support to be useful independently. A passing name, repeated source or polished one-off answer does not by itself earn a page. Similar wording is not proof of identical subjects. Keep transient analysis in its thread; honor an explicit request for a saved note or deliverable.
3. Write a coherent current account rather than append unrelated source summaries. Use only the structure that helps this subject:
   - **Project/entity:** identity and scope, supported current facts and dates, related work, open questions and evidence.
   - **Concept/process:** explanation or steps, distinctions, conditions and exceptions, useful examples and evidence.
   - **Synthesis/comparison:** the question, supported agreements and differences, explicitly labeled inference, uncertainty and evidence.
   Preserve useful existing detail and owner wording. Split a page only when its subjects are independently useful and the current request permits it. A source-focused summary is useful when that source's argument or identity deserves separate treatment; source intake has no page quota.

## Ground and connect the account

4. Keep material claims attributable to the actual source or accepted note revision, with a passage/section and relevant dates where available. Preserve names, quantities, units, ranges, conditions, negative facts and minority evidence. Distinguish **source reports**, **owner decisions**, and **AI inference** in prose. Later ingestion is not later evidence or higher authority. Explain incompatible dates, scopes or claims instead of silently choosing the latest-looking statement. Read through generated synthesis to underlying evidence when validating important claims; repeated AI restatement adds no independent corroboration.
5. Use typed Link records for supported note-to-note and note-to-source relationships and link the result to relevant work. Use concise relation names describing the supported connection, such as `source`, `related`, `explains` or `compares`; reuse an existing equivalent link. Name readable sources and locations in the note where they support the explanation. Record the observed exact versions of direct inputs in the change request, using the base guide's Current/Historical contract. A link and an input version establish provenance and concurrency footing, not that a claim is true. If a needed original is unavailable, state that limit; retain a necessary supplied excerpt or use the host's full-import workflow when that intent is requested.

## Commit and reassess

6. Prepare the smallest coherent typed write group with `prepare_changes`: Notes, Links, source references, related Thread and supported Action context. Apply using `apply_changes` and inspect the actual outcome. Follow the base guide for authority, exact review, dirty guards, stale inputs and receipt replay. Report a meaningful result with its receipt; routine success stays in What changed. With missing consequential authority or a protected conflict, preserve the protected record and use the conflict playbook for one concrete owner choice. A conflicting ordinary page cannot displace a protected decision.
7. For a stale candidate, reread current targets and relevant evidence; reassess what belongs in the account. Use `refresh_candidate` for a new immutable replacement, then inspect its application under current authority. Archive superseded ordinary material only when the relationship is clear and the host permits the checked `archived` operation. Preserve source history and explicitly qualify a replacement; refresh grants no broader authority.

## Queries and bounded maintenance

When a question produces a useful new comparison or explanation, save it only if it meets the page admission rule and current authority allows it. Link the inputs actually used. A requested draft has its own audience and purpose; it need not become a concept page.

When asked to review knowledge, or given affected records after an input change, inspect those subjects and their directly linked context within the run budget. Look for duplicate subjects, unsupported claims, lost qualifications, contradictory evidence, missing useful links and recorded dependencies needing refresh. Report what was inspected, corrected and left uncertain. Hashes and versions show observed input changes; they do not establish world freshness. A bounded review does not certify the whole wiki or create an external-source monitor or recurring schedule.

## Examples

Two project emails confirm a meeting and add a qualified delivery estimate. Update the existing project account, preserving both evidence references, dates and direct inputs. A subsequently ingested older planning note adds a contact but does not silently replace the newer estimate. Preserve a conflicting protected commitment and add one focused Conflict reason. Apply the supported ordinary update and report its actual receipt.

The owner asks for a saved comparison of two procedures. Read both references and retain each procedure's thresholds and exceptions. Save one reusable comparison with links and exact inputs to both. Label a recommendation about convenience as inference; a rule absent from one source remains unspecified. An unrelated arithmetic question stays in conversation.
