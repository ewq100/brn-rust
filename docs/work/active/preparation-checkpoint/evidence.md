# Preparation checkpoint — 2026-10-06

## Early durable checkpoint

Scope: documentation, specifications, planning and normal integration only. Product implementation, H3–H5 experiments, dependency replacement, skill installation/build and the remaining V1 roadmap stay paused. One lead retains the selected Sol High model; no model/settings change or Astra escalation.

Initial checkout: local `main` at `18f3891e29e57d4459c663967d26115f0e57787d`, behind remote; only unrelated untracked `.DS_Store` files. Existing feature/review worktrees preserved. Preparation uses isolated `codex/preparation-checkpoint` from fetched `origin/main`.

Verified GitHub baseline: PR78 is MERGED, head `788d633e2fc79b11eb08eb55e3902c7d6e2cc6ae`, merged 2026-10-06T14:00:56Z at `a8deb9d8e94665b1731034675b490fb134aae091`. Remote main equals that merge. Product implementation remains PR77 `c75803832f3140347192bb08f2fdf13bb5fba1d4`; PR78 changed documentation only.

PR78 has no submitted GitHub reviews or inline comments; its PR body records a clean independent documentation review. Current main protection enforces strict required checks for Core and CLI Ubuntu/macOS, Native UI macOS, Native retrieval macOS (GitHub Actions app15368); no required PR approval setting, no branch ruleset, no force push. PR-head run37464020406 passed all four required checks and Documentation; Windows failed. Merged-main run37475432892 completed: four required checks, Documentation and supplementary Ubuntu UI passed; overall FAILURE from Windows core/UI/retrieval and supplementary Ubuntu native retrieval. No pending job. No portability work is authorized here. Sources: [PR78](https://github.com/ewq100/brn-rust/pull/78), [main CI](https://github.com/ewq100/brn-rust/actions/runs/37475432892), GitHub protection/rules APIs queried in this session.

Decisions: preserve frozen product/architecture and deterministic approval; improve existing reuse rule and H1–H5 rather than duplicate them. Use existing repository documents as cross-agent authority. Historical test counts remain historical; this preparation runs documentation checks only.

Unresolved at this checkpoint: task readiness and domain consistency still need relevant-code/contract inspection; hybrid design needs product-to-production and pinned Matt methods. Native/live/owner qualification remains pending. Next action: inspect these sources, refine the next settled specs and queue, then check/review/commit/push the bounded result.
