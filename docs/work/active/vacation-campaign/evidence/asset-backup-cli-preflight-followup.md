# BRN asset backup/restore CLI preflight follow-up

## Finding

No supported normal Xberg/BRN CLI fixture route for a retained PNG Asset is source-backed under the current production helper configuration. Defer the Asset-specific backup/restore run; do not try another fixture or backend by blind conversion.

This is a current production-configuration limitation, not a claim that the Xberg library or BRN storage format cannot represent images. The helper projection can turn returned PNG bytes into `ImageAsset`s, and the checked intake service can persist them, but the production helpers deliberately configure extraction so the SDK does not return retained image bytes. `document-conversion` selects only `xberg|xberg-brn` and AI on/off; there is no supported image-extraction switch.

## Evidence

- The actual public CLI attempt imported the synthetic Luna `footnotes.docx` (source SHA-256 `d753d9a9…a0fc4c`), whose inventory contains an embedded PNG and figure. The resulting [archived synthetic records output](asset-backup-cli-no-retained-image-raw.tar.gz) has 7 records and zero Assets. Its Source and Note report `Unchecked` with two `SDK returned no retained image bytes` gaps and an explicit gap that external image references were not fetched. The Note retains the unresolved `media/image1.png` Markdown link. This confirms the earlier fixture-specific uncertainty.
- Both helper identities compile the same `tools/xberg-intake-helper/src/engine.rs`; the build identity changes the backend/patch identity, not the helper policy. `config()` sets `images: None`, `captioning: None`, `qr_codes: Some(false)`, and `ocr_embedded_images: Some(false)`. The pinned Xberg source-map entry `X37` says `needs_image_data` depends on explicit images, captioning, QR codes, and embedded-image OCR, and warns that `images=None` alone is insufficient. The current combination leaves each route disabled.
- `tools/xberg-intake-helper/src/project.rs` only retains an image when the SDK result already includes nonempty PNG bytes. Empty data becomes a gap; unsupported formats become gaps. It never reads source file paths or fetches remote/external image links. Intake then persists only this returned `converted.assets` vector; there is no CLI command to add an arbitrary Asset.
- `tools/xberg-intake-helper/tests/eight_kinds.rs::all_non_security_sdk_options_keep_published_defaults` confirms there are no additional policy overrides beyond the explicit host/security list. The same helper test's eight ordinary fixture assertion expects zero returned Assets; Task1 implementation evidence explicitly says the ordinary eight fixtures return no retained image bytes under the chosen normal image configuration.
- Existing DOCX inventories establish that source fixtures contain pictures, but source image presence is not evidence of retained bytes. Current PDF/Office fixtures also enter the same fixed helper config; no existing helper test/inventory demonstrates a byte-bearing Asset from the production normal path. Trying PDF or `xberg-brn` would therefore be speculative and is outside this bounded audit.

## Consequence

The earlier proposed `intake → Asset bytes → backup → restore` witness cannot currently be executed end to end through supported normal CLI behavior. Preserve backup/restore coverage for canonical records and separately retain the existing core/app test that injects a synthetic byte-bearing conversion result. A future Asset round-trip acceptance needs a separately selected and qualified supported image path (and a stable fixture/check that yields nonempty PNG bytes) before backup/restore can test it. No second conversion, direct helper invocation, SQL seeding, or source change was performed here.
