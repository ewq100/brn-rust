# Historical archive

Current requirements live in the [Threads target](../architecture/threads-target.md), current gates in [status](../status.md) and current authority in [AGENTS.md](../../AGENTS.md). Historical instructions, approvals and commands below describe completed work and do not assign new tasks.

## Recent dated status

[Status history through18October2026](status-through-2026-10-18.md) preserves the
older status checkpoints, including superseded current snapshots. The maintained
status page keeps only the latest assignment, progress, limits and next action.

## Fixed baseline provenance

The complete pre-cleanup checkout is recoverable at commit **`3a3aa7ac6f2a70815ffc778a694663f729203a79`**, PR #120. Git preserves deleted files without keeping obsolete instructions and images in the maintained checkout.

- [Complete documentation](https://github.com/ewq100/brn-rust/tree/3a3aa7ac6f2a70815ffc778a694663f729203a79/docs)
- [Superseded active plans and build prompts](https://github.com/ewq100/brn-rust/tree/3a3aa7ac6f2a70815ffc778a694663f729203a79/docs/work/active)
- [Completed work](https://github.com/ewq100/brn-rust/tree/3a3aa7ac6f2a70815ffc778a694663f729203a79/docs/work/completed)
- [Audits](https://github.com/ewq100/brn-rust/tree/3a3aa7ac6f2a70815ffc778a694663f729203a79/docs/audits)
- [Earlier design specs](https://github.com/ewq100/brn-rust/tree/3a3aa7ac6f2a70815ffc778a694663f729203a79/docs/superpowers/specs)
- [Earlier architecture decisions](https://github.com/ewq100/brn-rust/tree/3a3aa7ac6f2a70815ffc778a694663f729203a79/docs/architecture/decisions)
- [Old UI backlog and screenshot evidence](https://github.com/ewq100/brn-rust/tree/3a3aa7ac6f2a70815ffc778a694663f729203a79/docs/ui)
- [Standalone experiments and original fixture provenance](https://github.com/ewq100/brn-rust/tree/3a3aa7ac6f2a70815ffc778a694663f729203a79/experiments)
- [Retired Rig comparison harness](https://github.com/ewq100/brn-rust/tree/3a3aa7ac6f2a70815ffc778a694663f729203a79/scripts/qualification/rig-decoder-comparison)
- [Historical MacBook campaign artifacts (55 tracked files)](https://github.com/ewq100/brn-rust/tree/3a3aa7ac6f2a70815ffc778a694663f729203a79/artifacts)

## Selected Threads proof retained locally

These nine reports were copied from `docs/work/active/threads-rebuild/` at the fixed baseline. Each names its original location; historical relative file links now address that original baseline path. Report bodies preserve dated evidence, including gates that were still pending when written. Later integration is reflected in current status and PR records.

| Report | Subject |
|---|---|
| [review.md](threads-rebuild/review.md) | Independent review and dispositions |
| [evidence.md](threads-rebuild/evidence.md) | Rebuild/runtime evidence |
| [phase0-evidence.md](threads-rebuild/phase0-evidence.md) | Phase 0/F1 evidence |
| [phase1-delegation-evidence.md](threads-rebuild/phase1-delegation-evidence.md) | First-use delegation |
| [phase1-steering-evidence.md](threads-rebuild/phase1-steering-evidence.md) | Stop-and-send |
| [phase1-backup-evidence.md](threads-rebuild/phase1-backup-evidence.md) | Atomic backup publication |
| [phase1-restore-evidence.md](threads-rebuild/phase1-restore-evidence.md) | Restore and missing-storage safety |
| [phase1-backup-folder-evidence.md](threads-rebuild/phase1-backup-folder-evidence.md) | Remembered backup folder |
| [rig-decoder-comparison.md](threads-rebuild/rig-decoder-comparison.md) | Bounded Rig decoder comparison |

The default Markdown gate exempts outgoing historical archive links only. Current links into these local reports still validate file and fragment existence; `--all` checks retained report links too. Remote baseline links identify preserved Git content and are not fetched by the local checker.
