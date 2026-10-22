# BRN native future-incompatibility triage

## Finding

The saved Cargo future-incompatibility report diagnoses `block v0.1.6` at the registry source `block-0.1.6/src/lib.rs:64`:

```text
warning: static of uninhabited type
  static _NSConcreteStackBlock: Class;
= note: uninhabited statics cannot be initialized, and any access would be an immediate error
= warning: this was previously accepted by the compiler but is being phased out; it will become a hard error in a future release!
= note: ... issue #74840
```

The local source defines `enum Class { }` immediately before the foreign declaration, so `Class` is uninhabited. Cargo report command reproduced this exact saved diagnostic with pinned Rust/Cargo 1.98.1:

```text
cargo report future-incompatibilities --id 1 --package block@0.1.6
```

The saved log records successful build/test completion with this warning; the warning is not a failure under this checkout's pinned toolchain, Rust 1.98.1. It explicitly signals a future compatibility break, without naming the release in which the lint will become an error. This is a warning status, not a future-toolchain qualification.

## Selected dependency path

With `--locked --offline`, the relevant graph is present for `brn-desktop`'s `native-ui` feature on `aarch64-apple-darwin`:

```text
brn-desktop 0.1.0
└── gpui-kit 0.7.3
    └── gpui-pre-platform 0.3.9
        └── gpui-pre-macos 0.3.9
            └── cocoa 0.26.0
                └── block 0.1.6
```

The same `block` package is also reached through `cocoa-foundation 0.2.1` and `metal 0.33.0` (via `gpui-pre-apple 0.3.9`). The transitive dependency owner at the BRN boundary is the pinned optional `gpui-kit = 0.7.3` native UI stack; the lint itself is in `block`, not BRN source. A plain default-feature `cargo tree -i block` does not include this optional, target-specific native dependency; the feature and Apple target selectors are necessary.

The repeated warning log contains build paths naming another worktree (`architecture-status-route`), while the supplied shared target report names the same `block` source diagnostic. The dependency path above was independently resolved from the specified `vacation-hardening` checkout's lockfile and native feature graph.

## Action

No dependency, toolchain, or source change is justified for this bounded triage. Keep the current lockfile and Rust 1.98.1 pin; the evidence says this pin builds with a warning, while a later compiler may reject the dependency. Revisit when the pinned Rust toolchain is intentionally changed or an upstream `block` resolution is verified. At that point, check this exact native feature graph and diagnostic first, then consider a narrowly scoped upstream dependency resolution or a reviewed, ABI-correct patch if the warning is promoted to an error. The available evidence does not establish a fixed `block` release, a compatible replacement, or a need to upgrade the wider GPUI toolkit.

## Evidence and primary references

- Local saved report: `target/.future-incompat-report.json` in `/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust`.
- Local source: `/Users/evokessler/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/block-0.1.6/src/lib.rs:50-64`.
- Local graph: `cargo tree --locked --offline -p brn-desktop --features native-ui --target aarch64-apple-darwin -i block`.
- Saved repeated warning: `docs/work/active/vacation-campaign/evidence/knowledge-refresh-final.log` in the same checkout.
- Upstream crate repository: https://github.com/SSheldon/rust-block (primary project source; not used to infer a fixed version).
- Rust issue linked by the compiler: https://github.com/rust-lang/rust/issues/74840.
- Cargo's documented dependency override mechanism, cited in Cargo's saved report: https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html#the-patch-section.

