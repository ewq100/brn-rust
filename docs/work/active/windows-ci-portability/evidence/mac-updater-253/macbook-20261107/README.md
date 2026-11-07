# MacBook continuation — 7 November 2026 UTC

This conversation owns B's continuation of #253 and draft PR268 on
`work/mac-alpha-acceptance-253`. The owner selected this MacBook for all remaining
work. The previous Mac mini execution is superseded; it must not publish or write
the branch concurrently. Takeover was posted on [#253](https://github.com/ewq100/brn-rust/issues/253#issuecomment-6701756635)
before branch edits. No second B thread was found in this MacBook project. The
old host's live activity could not be independently inspected: the linked remote
environment tool is unavailable and SSH stopped at host authentication. This
limitation is recorded, not treated as proof that a remote process stopped.

The original [Mac mini evidence](../alpha-a921e325/README.md) remains unchanged.
This directory holds only new MacBook observations. The existing selected task
record remains the authority for current gates; this is supporting evidence.

[Preparation checks](preparation-checks.json): native development preflight,
16 overview tests, generated overview, Markdown links and whitespace pass.
All27 files in the original alpha evidence checksum list match, and all five
unrelated task objects are preserved. Transferred package verification and native
observations remain pending.

## Identity and refreshed gates

- MacBook `EMB-60VVL9KC`: arm64, macOS26.5/build25F71, compatible with arm64/macOS15+
  artifacts. [Takeover receipt](takeover.json), [development preflight](preflight.json).
- Isolated task checkout: `/Users/evokessler/repos/brn-mac-alpha-253`.
  Initial branch/PR head `94c11669f9148bcd95910bd32ca7675aefc7b348`;
  remote main `c388da267a9e41cfe9d65051cd27b95817c9a523`.
  The main checkout's unrelated `.vscode/` and `experiments/` were preserved.
- Fixed runtime/package source remains `a921e32533dc03134ed8a2001bd82f27194c3302`.
  Runtime crates, scripts, tools, agent, Cargo manifests/lock and CI have no delta
  from that source at the initial proposal head.
- All seven [proposal CI lanes](https://github.com/ewq100/brn-rust/actions/runs/42302755459)
  passed on94c11669; [captured receipt](proposal-ci-checkpoint.json).
  The current-head [code receipt](https://github.com/ewq100/brn-rust/pull/268#issuecomment-6697165573)
  and separate [security receipt](https://github.com/ewq100/brn-rust/pull/268#issuecomment-6697213944)
  report no findings. Review summary completed; zero review threads. The private
  full security report was not inspected. These results apply to94c11669;
  subsequent documentation commits require their own applicable checks/review.
- PR268 is draft/open/unmerged. No explicit owner release approval was found in
  the inspected issue/PR record. Published releases are still legacy preview.1/.2;
  neither proves a command-enabled alpha or stock HTTPS update.

## Transfer and resource observations

The existing `brn-rust` SSH alias resolves to `evos-mac-mini.local`. A batch-mode,
strict-host-key connection exited255: no trusted ED25519 host key. No trust bypass
or credential transfer occurred. The handoff directory is absent on this MacBook.
The owner was asked to copy the complete Mac mini folder
`/Users/Shared/BRN-alpha-a921e325-acceptance` to that same MacBook path, including
A/B signed ZIPs, manifests/checksums, public synthetic fixtures and checklist.

D and E were directly notified of B's ownership and fixed source. D's targeted
Rust tests/Clippy finished; D and E each report no heavy build, timing run or native
session planned. Both have read-only Sol/Opus Council calls using shared accounts;
B has made no provider call and will coordinate the actual Copilot session when
the clean profile is ready. Neither experiment is a release dependency.

Only the missing test CLI was built successfully: clean detached source checkout
`/Users/evokessler/repos/brn-mac-alpha-253-source` at exacta921e325, dedicated target
`/private/tmp/brn-mac-alpha-253-20261107/target`, locked/offline Rust1.98.1,
deployment15.0, jobs1. [Build/binary/smoke receipt](cli-build-and-smoke.json) and
[build log](build-cli.log.gz). Arm64/system-library/minimum15 identity passed;
machine help, synthetic initialization/note creation, backup SQLite quick_check
and equal notes after fresh restore passed. Synthetic data was removed.

Verified persistent test-only copy:
`/Users/Shared/BRN-alpha-a921e325-macbook-test-tools/brn`, SHA256
`46a99f7af3ad37c0f7ebc3ecedd7233dad3e4bc20dd1a3db6d53e133e4de21e3`.
No D/E binary or mutable target was borrowed, and no desktop/helper package was
rebuilt. B's build/smoke processes have finished; B holds no host reservation.
This smoke check does not establish app/native/helper/import/provider acceptance.

## Native execution

Use the [current MacBook session instructions](native-session.md), derived from
A/B's original checklist and #252 acceptance. Only the normal owner account and
managed account were found; neither is used for acceptance. The owner was asked
to create/sign into a disposable standard test account and remain available for
normal Gatekeeper approval and Copilot device sign-in. No native action has run.

Verify transferred bytes against the committed
[package identities](../alpha-a921e325/package-identity.json) before extraction or
launch. Check both external manifests and checksum files, ZIP size/hash, embedded
manifest/Info.plist, every packaged binary/helper and unchanged updater, notices
index, arm64/deployment identity and strict deep codesign verification. Preserve
the originals and extract only into a new task-owned directory. Record the actual
delivery route/quarantine state; copied ZIPs cannot prove browser-download
Gatekeeper behavior. Normal app journeys may proceed before publication.

Native acceptance, owner acceptance, publication and real stock HTTPS A→B remain
pending independently. Finish available copied-package acceptance first, then
present the concrete alpha.1/build3 decision with source/limits. Approved hosted
artifacts will be rebuilt by the existing workflow from the actual approved main
manifest commit; the local a921e325 ZIP hashes are not promised hosted identities.
