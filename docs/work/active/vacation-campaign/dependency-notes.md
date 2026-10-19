# Bounded native dependency research

## Carried block0.1.6 warning —19October

Current native Clippy and production builds pass, then emit a future incompatibility
warning for `block0.1.6`. The local Cargo report identifies
`_NSConcreteStackBlock: Class` in that crate as an uninhabited static. Rust's
[lint documentation](https://doc.rust-lang.org/nightly/rustc/lints/listing/deny-by-default.html)
and [tracking issue74840](https://github.com/rust-lang/rust/issues/74840) explain
that this category is being phased toward a hard error. The nightly documentation
does not change this repository's pinned Rust1.98.1 qualification.

Read-only Luna inspection traced the locked native graph from pinned gpui-kit0.7.3
through the macOS platform stack/Cocoa/Metal to block0.1.6. BRN source does not own
the reported extern static. The [block maintainer repository](https://github.com/SSheldon/rust-block)
identifies its Objective-C block interface. This is an upstream dependency concern,
not evidence of a current BRN runtime failure or vulnerability.

Disposition: retain the warning and current passing results; do not suppress it,
fork the native stack or change locks for this campaign without a concrete qualified
refresh. Revisit at the next native dependency/toolchain refresh and verify current
maintained release/fix options then.

Supplemental primary-source check on19October2026 found **no verified released fix**:
[published versions](https://docs.rs/crate/block/latest) list0.1.6, published8May2016,
as latest; the current
[maintainer manifest](https://raw.githubusercontent.com/SSheldon/rust-block/master/Cargo.toml)
still declares0.1.6 and its
[source](https://raw.githubusercontent.com/SSheldon/rust-block/master/src/lib.rs)
still contains the flagged static. No explicit maintainer migration recommendation
was found. A [2022 user issue mentioning block2](https://github.com/SSheldon/rust-block/issues/16)
is not such a recommendation; suitable successor qualification remains unknown.
This does not recommend adopting a different dependency.
The researcher did not independently recover the compiler version from these two
logs; the source-bound notice receipt separately retains the actual Rust version.
