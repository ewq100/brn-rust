# Embedded image capability witness

The current production converter keeps its published normal image defaults. This isolated synthetic experiment enabled only image extraction in a temporary test; OCR, captioning, QR, security limits and production source stayed unchanged. Nothing was saved into a BRN database.

The pinned SDK returned the exact embedded PNG (13,308 bytes and its independently checked SHA256), but reported drawing dimensions547×197 rather than the PNG header940×340. It also returned a second empty placeholder. The first one-test attempt failed at a count assertion: one embedded file in an inventory does not require every SDK image entry to contain bytes. That assertion alone is not evidence of an SDK defect.

The second and final diagnosed attempt emitted checked-projection metadata before the unchanged assertion. The existing projector refused the actual image with `document image dimensions/budget`. This confirms the alternative setting cannot simply be enabled in BRN. Neither failure was changed into a green claim; both exact patches, scripts, output and metadata are retained in the [raw archive](image-retention-proof-raw.tar.gz) and [receipt](image-retention-proof.json).

No production implementation is selected. The existing normal defaults, tests, provenance and canonical guards remain. A future image-retention candidate must reconcile the selected normal stock policy and explicit optional adaptation bundle, bind any changed policy honestly, distinguish image pixels from layout extents and separately qualify the restricted subprocess and complete intake/asset journey. This pre-research does not establish that candidate, general format fidelity or resource safety at every limit. No owner decision is needed to continue other authorized campaign work.

See [source assessment](image-retention-preresearch.md) and [bounded packet](image-retention-proof-packet.md). The retained-asset backup/restore witness remains valid for its recorded synthetic stored data; it does not become import qualification.
