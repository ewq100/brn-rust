# BRN's pinned Linux accessibility correction

This is the published `accesskit_atspi_common` 0.19.1 source, with the disabled
state translation from upstream [commit 6ee0558b](https://github.com/AccessKit/accesskit/commit/6ee0558b6315b3ef1594db24ce45a030ecac7cb5)
backported into `src/node.rs`. Disabled nodes must not expose AT-SPI Enabled or
Sensitive. This patch does not add actions, focus nodes or a new toolkit.

GPUI pre 0.3.9 uses AccessKit 0.24.1 / consumer 0.38.0 / Unix 0.22.1. The released
correction in common 0.21.0 requires a newer incompatible AccessKit family.
Keep the current family until GPUI supports that upgrade, then remove this path
patch and repeat the actual Linux state/action transition checks.

`BRN-PROVENANCE.json` records the published archive checksum, upstream source and
original file hashes. The three license files were obtained from that exact
upstream source revision. Apart from this document, provenance, those licenses,
the test module and its declaration, only the upstream state conditional changes.
No dependency versions or public APIs change.

The two regression tests cover disabled/enabled/read-only controls and repeated
state transitions. Both fail against the original conditional; all nine crate
unit tests pass with the correction. To run them independently of the workspace's
optional native graph, create a temporary Cargo package depending on this path,
copy the root lockfile to that package, then run:

```sh
cargo test --manifest-path /path/to/harness/Cargo.toml \
  -p accesskit_atspi_common --lib --offline --jobs 1
```

Native AT-SPI verification is recorded separately in the active precise-edit/UI
qualification record. Unit tests alone do not establish native accessibility.
