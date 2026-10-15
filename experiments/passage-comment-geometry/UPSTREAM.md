# Toolkit source and notices

The generated `.vendor/gpui-base/` is **gpui-base 0.7.3**, from the exact
crates.io archive already pinned in BRN's root lockfile:
SHA-256 `2e9955b6f4b048fd50ca15fc8e225d80bb00ba9ebc8fecdddde5ad26dbef9270`.
The archive's `.cargo_vcs_info.json` records repository
<https://github.com/longbridge/gpui-kit>, revision
`67265a36e260ffe03c5aaebdcd8d654dbf892eab`, directory `crates/base`.

The reviewable patch modifies these upstream files only:

- `crates/base/src/text/range_highlight.rs`
- `crates/base/src/text/inline.rs`
- `crates/base/src/text/state.rs`
- `crates/base/src/text/text_view.rs`

The crate declares **Apache-2.0**. Its complete `LICENSE-APACHE` is retained
in the materialized copy and reproduced in [LICENSE-TOOLKIT-APACHE](LICENSE-TOOLKIT-APACHE).
No NOTICE file is present in the published archive. The patch retains original
code and documentation and marks the added experimental API in its diff.
No Zorite source or dependency is included. No new external dependency is needed
by the extension. BRN rendering policy and synthetic host snippets originate
from this repository at the report's snapshot, not from Zorite.

The experiment author owns this prototype. Integration requires the writing
owner's review and the shared dependency owner's approval. Prefer a tested
upstream API/release over a permanent BRN fork. Until then the four-file patch
requires rebasing and regression checks on every gpui-base update; its use of
internal reveal/Inline layout is the main maintenance cost.
