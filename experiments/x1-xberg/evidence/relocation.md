# Owner-selected fresh-clone location

Owner answer: **Create a fresh clone at the requested path.**

- Fresh clone: `/Users/evokessler/repos/brn-rust`.
- Current worktree: `/Users/evokessler/repos/brn-rust/.worktrees/x1-xberg`.
- Current branch: `work/x1-xberg`.
- Upstream origin/main at clone time: `7e9d0376d677ed2a46bdbcd8933f55f9e1746a02`, the unchanged qualification snapshot.
- Transferred original local commit: `f6092462065a6d614fe337f7a2639ca2e75b55a4`, directly descended from that snapshot. It was fetched from the existing local Git object database, without a push or use of the main checkout's untracked experiment files.
- GitHub origin: `https://github.com/ewq100/brn-rust.git`. The fresh clone has an independent object store with no alternates and no added permanent local remote.
- Preserved original worktree: `/Users/evokessler/Documents/Codex/brn-rust/.worktrees/x1-xberg`. Its files and branch were not changed by this relocation.

Verification before location-documentation edits compared all **132** tracked experiment files byte-for-byte with the preserved checkout and checked every recorded source SHA-256. All matched. `python3 experiments/x1-xberg/check_inventory.py` passed from the new worktree; its output is in [relocation-checks.log](relocation-checks.log).

REPORT.md and COMMANDS.md identify the current location and distinguish it from the original execution location. Captured metadata, build logs, image outputs, hashes and benchmark records retain their original provenance. No build, extraction, model call or performance rerun is claimed by this relocation. Gate statuses and recommendations are unchanged.

Only experiment documentation and relocation records are changed in the new task commit. The fresh clone's local `.git/info/exclude` ignores `/.worktrees/`; no tracked file outside `experiments/x1-xberg/` is edited. Original and unrelated worktrees are preserved. No push, PR, merge, installation or gate-3 work was performed.
