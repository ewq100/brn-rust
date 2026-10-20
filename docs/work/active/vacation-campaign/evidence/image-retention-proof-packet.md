# Offline embedded-image retention proof

Append the single function in `/tmp/brn-image-retention-proof-test.rs` to `tools/xberg-intake-helper/tests/eight_kinds.rs` in the isolated `codex/image-retention-proof` checkout, then run only that test:

```sh
CARGO_NET_OFFLINE=true cargo test \
  --manifest-path tools/xberg-intake-helper/Cargo.toml \
  --test eight_kinds footnotes_docx_retains_the_inventory_png_when_explicitly_enabled \
  --locked --offline --jobs 1
```

The test uses the existing `request`, `fixture`, `engine::runtime`, and `project::project` helpers. Its extraction call follows the existing normal SDK test (`xberg::extract(ExtractInput::from_bytes(...), &config)`) with the lockfile-pinned `xberg = 1.3.10` API. It changes only `ExtractionConfig.images`: extraction on and image OCR off; it asserts the helper's OCR disablement, embedded-image OCR off, QR off, captioning unset, and security limits remain in place. It also checks the returned image has no nested OCR result. It calls the checked existing projector and `DocumentExtraction::validate_for` afterward.

The independent fixture inventory is `crates/brn-intake/tests/fixtures/office/footnotes.inventory.json`: source `footnotes.docx` SHA-256 `d753d9a98afe1771c0d0488871e322900ac97f74a5390feb9911fcadc9a0fc4c`; one `word/media/image1.png` asset, 13,308 bytes, SHA-256 `86e6c9325de51d5c6b84dfd5aad8f5b4d09cfccc447a1626f8be3380e7700599`, PNG IHDR dimensions 940×340; relationship target `media/image1.png`; figure caption `Figure 1. Label placement follows sealing; the three boxes show the required order.` The inventory marks behavior unexecuted and its authorship review pending. The test prints and optionally captures raw/projected metadata before assertions, including source path, format, dimensions, byte count, and computed byte hash; it never serializes image payloads.

A pass would establish only that this pinned SDK call returns the inventory PNG and the existing projection retains its bytes, replaces the exact Markdown target with the content-addressed PNG name, preserves the authored caption, and passes envelope validation for this fixture/configuration. A failure is diagnostic: metadata is emitted before checks and the projector is not modified to accommodate it. This is an in-process SDK/projection proof. It does not qualify the helper subprocess/protocol, default published configuration, production CLI/UI behavior, OCR or semantic image interpretation, broader document fidelity, other formats, or any provider/model path. No fixture conversion, model use, network access, or repository change is part of this packet.
