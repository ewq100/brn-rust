# Final integration preparation handoff

> **Revision 3, 18 October 2026:** two narrow precisions after Opus round 2 (owner-only authority for narrowing a current kind; case C judged on the synchronous outcome); see [opus2/handoff.md](opus2/handoff.md).
>
> **Revision 2, 18 October 2026:** corrected after Opus round 1 (CHANGES_REQUIRED). The text below is the revision 1 handoff, kept for history where still accurate; the current correction handoff is [opus1/handoff.md](opus1/handoff.md) and the point-by-point answer is [opus1/response.md](opus1/response.md). Superseded below: "ready next work" (final integration starts only after the section 2 precondition in [plan.md](plan.md), not after a first-HTML pin) and the lock statement (six existing moves, expected set computed from the exact handoff).

17 October 2026. Source-only plan and inventory complete. **Not** a build, verification, review or merge, and **Xberg is not done.** The commit that contains this file is identified by `git log -1 work/final-integration-20261017`; a file cannot name its own commit, so the SHA and tree are reported in the task result and re-derivable with `git rev-parse HEAD HEAD^{tree}`.

## Binding

- Checkout `/Users/evokessler/.t3/worktrees/brn-rust/t3-ca8b0a32`. Initial branch `t3/prepare-final-integration-plan` at `9b47b285d23bb5f89db02ba398244827032fab6b`, clean.
- New branch `work/final-integration-20261017` (absent before) created at exact `73ea9ca777e00d348966e37e8402f3eb68ecb20b`, tree `74d6dfb216585787e3d913ee45117868a2d2ceb9`, crates `bdc0a22a29e1aec741a78916ae5d0a7748531864`. One local commit adds only the files in this folder.
- Untouched: X2 checkout `t3-9527c407`, INT-W checkout `t3-239c29dc`, parent root and its `.vscode/` and `experiments/`, draft PR #149, every other ref. No fetch.

## What the plan establishes

1. The writing line (INT-W `d2d84e66`, source `0efd73f1`) and the capability line (X2 `73ea9ca7`, source `39a67e4f`) descend from CW2 `b617d784` and have never been combined. They merge with zero textual source conflicts and two documentation conflicts. The writing line changes nothing in brn-ai, brn-threads-core or the CLI, so those crates' bytes in the union equal the capability line's.
2. The textual merge is not a clean compile. One defect is confirmed statically: `start_request` (writing line) references `reviewed` from D4's MCP review, a local of `start_run`. Recommended fix and required witness are in [merge-simulation.md](merge-simulation.md#nf-1). Only native lanes will see it. Separately, the repository's link checker already fails on the X2 base (82 failures in two `source-audit/` files, absolute registry paths used as link targets); that is a docs-only fix (NF-6).
3. `origin/main` is `57c710cc` (two docs commits over `8f768943`), which adds status/AGENTS/strategic-programming rules the final integration must follow; `main` locally is stale. `work/x2-intake-20261017` has advanced by one docs-only commit to `781c34dc`.
4. Carried PDFUnavailable, SDK 474/4/1, widget 314/1/1 and the hosted main native-lane failure are listed with their exact tests, evidence, later passes and what would change them. None is called flaky, baseline or closed. The four required hosted contexts are the actual merge gate; plan for a red native lane.
5. Gate map G0 to G10 and one owner manual task N1 are in [plan.md](plan.md); live evidence is re-read, not rerun.

## Ready next work

Section 8 of the plan is the bounded next allocation for final integration, which starts only after the section 2 precondition (all current eight document kinds, host, fidelity, lifecycle, no old backend, extra-family dispositions, reviewed pinned family completion) is recorded by the principal. An optional technical TCI-1 exercise needs its own exact grant and is not final. Nothing in this handoff needs a permission answer.

## Limits

Static inspection only: Git, source and text reads, `git merge-tree --write-tree` (unreferenced objects in the shared object store), a Python lock-set comparison. No Cargo, build, test, lint, format, Store, CLI, auth, network, provider, GUI, mutation or live command. `threads-target.md`, `threads-behavior.md` and the long evidence logs were read by heading and amendment, not in full; see [binding.json](binding.json).

## Release

No heavy slot was acquired or held. All paths allocated to this task (this folder only) are released to the principal at commit. No process of mine remains. No PR, push, merge, release, native observation or paid start occurred.
