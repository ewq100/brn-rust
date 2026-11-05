# BRN UX rebuild

**Selected design:** simple, clean, ChatGPT-inspired native UX using GPUI Kit **0.9.0**, with complete System, Light and Dark appearance.

**Revision 1.1:** Evo added the required 0.9.0 upgrade on 5 November 2026. The build agent must complete the compatibility migration and lockfile update as an early part of the assigned rebuild. See specification §12.1; the approved visual references remain applicable.

This is the owner-requested repository copy of the BRN-UX-002 design handoff, published on 5 November 2026. It defines the full UX rebuild target. Publishing these references does not start production implementation or replace another active task.

## Start here

1. Read [the handoff guide](README_START_HERE.md) and [full UX specification](BRN_UX_Rebuild_Spec.md).
2. Inspect [the visual reference PDF](BRN_Design_Reference.pdf), [offline interactive reference](BRN_Interactive_Reference.html) and [design tokens](BRN_Design_Tokens.json).
3. Read [the related Wiki Library specification](BRN_Wiki_Library_Spec.md) for article consolidation and evidence behavior.
4. Use [the build-agent prompt](BUILD_AGENT_PROMPT.txt) when assigning implementation.

Download or open the HTML from a local checkout to use its preview controls; GitHub's source view does not run the prototype. The PDF contains drawn design references. The HTML uses temporary sample state. Neither establishes native implementation or acceptance.

## Repository integration

Follow [AGENTS.md](../../../AGENTS.md) and the current owner task. The [canonical target](../../architecture/threads-target.md) and [architecture invariants](../../architecture/invariants.md) continue to own product and data contracts. This handoff supplies the selected presentation and interaction target; the receiving build agent reconciles any canonical documentation changes during implementation.

The inspected main baseline is `e749a83e207c5b6d3c081d9f7d1fa6d6efed088c`. This documentation publication leaves the existing selected task route and ongoing code changes intact. Keep execution progress in the actual rebuild task record after assignment, rather than duplicating it here.

The related BRN-WIKI-001 handoff is included locally so build agents do not need access to the original ChatGPT file. It retains its 4 November baseline and scope; this publication does not start a separate benchmark campaign or consolidate owner data.

The spec includes 40 acceptance cases and a staged migration plan. Retain supported behavior throughout the rebuild, including save/recovery, queued chat, writing review, exact Apply and Undo, project scope and manual intake. Preserve the approved clean hierarchy across all screens and both themes.
