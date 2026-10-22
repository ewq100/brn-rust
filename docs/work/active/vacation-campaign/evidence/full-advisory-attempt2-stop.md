# Current-lock advisory scan result

Status: **incomplete**. The approved attempt-2 runner reached and completed the first root-lock audit, then stopped on its fail-closed database-commit validation. The other three selected lockfiles were not scanned. This is not a full audit result.

## Completed first lock command

Tool: official `cargo-audit 0.22.2` arm64 binary at `/private/tmp/brn-advisory-audit-20261022/bin-attempt2/cargo-audit`; the official archive digest had been verified before extraction. Attempt-2 runner SHA-256: `12b2c30b8262feeae700af4de36fd86b8079611dc89337d71b06baf9b7ea1096`.

The first input was the immutable main-213 root lock snapshot `/private/tmp/brn-advisory-audit-20261022/packet-attempt2/locks/main213-root-lock.Cargo.lock`, 249,732 bytes, SHA-256 `fc1ca2cf08b51f357dccd277b2fdb5491454c7bb87bf96e1026322d6b5f85d89`. The command used `--no-fetch --no-yanked --format json`, no target filters, no severity threshold, and no ignored advisories. It exited 1, with successful owned-process cleanup (`group_absent=true`, `leader_reaped=true`); stderr was empty. The bounded raw JSON is retained at `/private/tmp/brn-advisory-audit-20261022/packet-attempt2/commands/07-audit-main213-root-lock.stdout` (28,230 bytes; SHA-256 `e1853a8d957aa7906fa7005ce09f5d3ade54279565eb44f6a1ee0ff458dd349e`).

That valid-looking report contains 943 dependency entries, seven vulnerability records, and eight `unmaintained` warnings. The seven records are: `rustls 0.23.45` (`RUSTSEC-2026-0346`, `-0347`, `-0348`, `-0349`); `rustls-webpki 0.103.15` (`RUSTSEC-2026-0382`); `webpki-root-certs 1.0.9` (`RUSTSEC-2026-0383`); and `webpki-roots 1.0.9` (`RUSTSEC-2026-0381`). These are name/version matches only; this packet makes no reachability or exposure conclusion.

## Why the runner stopped

The fresh task-local official RustSec advisory DB clone has `HEAD` `59ef7abf8ce56ea80fe7eba97ba3835449009fa4`, commit date `2026-10-22T09:21:52+02:00`, and a clean worktree. The audit report's `database.last-commit` is `null`, so the runner's planned equality check against the frozen clone `HEAD` failed before it saved parsed JSON or started the other lock scans.

This null is explained by the cached RustSec 0.33.0 source: `Database::open(path)` initializes `latest_commit: None`; only `Database::load_from_repo(repo)` populates it. With `--no-fetch`, cargo-audit loads the path with `Database::open`, so report-level last-commit cannot establish the cloned repo revision. The clone's Git `HEAD` and clean status were independently recorded in the task receipt. The default audit settings also report informational warning categories (`unmaintained`, `unsound`, `notice`); the vulnerability query itself was unfiltered (`ignore=[]`, `target_arch=[]`, `target_os=[]`, `severity=null`).

## Preservation and limits

Raw attempt-2 packet and receipt: `/private/tmp/brn-advisory-audit-20261022/packet-attempt2/`. The first-attempt packet/receipt and report were preserved separately; the original report is `/tmp/brn-full-advisory-scan-attempt1-stop.md`. No third attempt was made.

All four source lock hashes were rechecked and match their pre-run observations. The export checkout remains at `003edd71518300f545fbde2d71bf850470f7ff0e`; the lead checkout advanced during the task from `250348fcd3919c4982784e30ec49a94c1bcf36ce` to `da1386b355ce8dd910609fda497894e1137af74b` with lead-owned documentation/evidence changes visible. The shared `~/.cargo/advisory-db` remains at its original HEAD `7eebec69c352c7191b1f13eb95dd510eeca5d1de`; the scan used only the task-local clone. No source or lockfile was edited by this task.

The fail-closed behavior prevented completion of the four-lock packet. Do not interpret unscanned locks as clean; a further invocation would require a separately selected correction to use the recorded task-local DB `HEAD` as provenance while accepting the report's source-defined `last-commit=null` for `--no-fetch`.
