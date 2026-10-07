# Introduction and status

This handbook defines how BRN should look, read and behave as a whole product —
including capabilities that are not built yet — and records how far the current
desktop app follows it.

> **Status: working design baseline, pending owner review (2026-10-07).**
> It was produced autonomously from the product vision and earlier design work.
> The owner has not yet reviewed or accepted any part of it.

## Sources of authority

| Topic | Authority |
| --- | --- |
| What BRN must do | [Product vision](../../product/BRN_PRODUCT_VISION.md) and its dated owner amendments |
| Product words | [Product glossary](../../product/glossary.md) |
| Current progress and qualification | [Status](../../status.md) |
| How BRN looks and behaves (this book) | These pages; deliberate departures are recorded in [decisions](decisions.md) |
| Design values in code | [`tokens.rs`](../../../crates/brn-desktop/src/tokens.rs), generated into [foundations](foundations.md) |

When this book and the vision disagree, the vision wins; record the conflict in
[open preferences](open-questions.md) rather than designing around it.

## Readiness labels

Every important example in this book carries one product-readiness label.
Verification and owner acceptance are recorded **separately** in the
[verification record](verification.md); a label never implies either.

| Label | Meaning |
| --- | --- |
| **Implemented** | Present in production desktop code on this branch. Screenshots come from the real views. |
| **Prototype** | Exists only as the clickable HTML prototype. Communicates layout and interaction intent; it is not working functionality. |
| **Future design** | Described here (and possibly shown in the prototype) but not built. Assumptions are listed. |

Two more words are used precisely:

- **Verified** names a specific check and its scope (for example “widget tests
  pass” or “headless render inspected”). Rendering is not native interaction.
- **Accepted** is reserved for a recorded owner decision: *Accepted*,
  *Rejected with observation* or *Explicitly deferred*.

## What changed in the current app

The branch that introduced this handbook also restyled the existing desktop app
toward the direction below without changing workflow behaviour, approval
meaning, persistence or element identities. See [current screens](screens-current.md)
for before/after captures.
