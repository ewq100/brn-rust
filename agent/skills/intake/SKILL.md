---
name: intake
description: Process supplied source information or verify a host full-note import, preserving useful meaning, coverage and references.
---

# Intake

1. Read the current related notes, source references, thread and Action with `read_records`. Use the bounded `read_note` continuation or section reads when returned Markdown is truncated. For an owner-supplied source available in this invocation, use `read_source`; its text is already converted by the host, and its coverage and exact figure links describe that conversion.
2. For **Use information**, retain the facts, qualifications, requests, decisions and dependencies that help current work. Update relevant ordinary notes, add thread context or suggest an Action; one source need not create one note or attention item. Prepare the smallest useful typed change group with `prepare_changes`, linking the actual Source record and including its observed version as an input. Select meaningful figures by using their exact returned links in Note Markdown; the host verifies and stores the selected bytes. Preserve reported claims separately from owner decisions, then apply and check the actual receipt.
3. For **Import full note**, use the host's Import full note workflow. That operation owns conversion, faithful content preservation, coverage reporting and creation of the protected note. The Threads tools can inspect its saved result with `read_records` and `read_note`; they do not run arbitrary converters or manufacture a full-import classification. If the owner requested this intent but the host import has not occurred, state the required host operation and retain the request rather than claiming a summary completed it.
4. Verify a full import against the available converted source and coverage: inspect substantive wording and structure, headings, steps, roles, prerequisites, tables, meaningful figures/captions and references. Read relevant sections beyond the first chunk. Preserve all known gaps and their locations; an omission means partial coverage, and unchecked coverage does not prove completeness. A short summary or model assertion is not proof of full coverage. Report the actual coverage outcome and any remaining inspection requirement.

Full imports start protected through the host workflow without a second creation approval. Save section navigation, source reference, intent and coverage with the readable note. Linked summaries may remain ordinary working notes. Protection does not establish that imported claims are true or that a process became owner policy. Any subsequent protected edit uses this invocation's explicit host grant and the checked change boundary from the base guide.

Original files and messages remain external. Intake reads those supplied sources; it does not delete, move, send or mark them handled. Retain only useful results, requested full-note content/assets, source references and processing outcomes. If interrupted work needs uncaptured input, reread it or ask the owner to supply it again.

## Example

The host imports a process DOCX as a full note. Read its protected saved note and coverage, including the final section through `read_note`, and verify the ordered steps, roles, prerequisites, table and references. Preserve a missing figure's location and report partial if it did not convert. Keep the DOCX at its external location.

## Boundaries

- A source request does not authorize changing a protected decision or making an owner commitment.
- A missing substantive section or table means partial coverage; a summary does not fulfill full-note import.
- Clearing an internal intake item does not alter the external file or email.
