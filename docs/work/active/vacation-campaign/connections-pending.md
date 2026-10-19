# Connections query replacement

Switching an open Note from Direct to Two hops previously left its old results
visible under the new heading while the replacement request loaded. The existing
generation/cancellation guard already prevented late response publication. Source
`d98374f4dc4eea5af1f6096703b37768b5a42709`, integrated at `98188942`, clears the
old result at request invalidation. Existing rendering now shows pending until the
matching result arrives. Search, visibility, relation filters, generation and
Knowledge policies are unchanged; no new owner/framework is introduced.

The existing tiled Connections test is extended, keeping its first-hop privacy,
relation-filter and recenter assertions. An independent second-hop Note is linked
through Operations. The actual Depth 2 control's immediate frame must show pending
and hide old rows/status before executor draining; after draining, that exact Note
must appear in hop2 with Operations as `via`. The total test count is unchanged.

The expanded test failed against unchanged production at the intended pending
assertion, then passed1/0 with the one-line fix. The [retained test-only patch](evidence/connections-test-only-baseline.patch)
applies to exact baseline92fdfbaa in an isolated index and changes only the test.
The first missing-element failure is also retained, with its initial patch; it is
not silently replaced by the later explicit assertion. Independent Sol approved
complete98188942 without findings. Final desktop321/0/1, strict all-target Clippy,
format/whitespace and a production native-ui build without test support passed.

[Commands](evidence/connections-commands.txt) and [result/hash receipt](evidence/connections-qualification.json)
retain original failures, final logs, source identity and immutable artifact hash.
The lead checked all tracked Rust/manifests and root lock/toolchain against the
qualified source. This is actual toolkit-control/rendered-frame qualification,
not Mac OS input or native acceptance. The ignored external CLI case remains
unrun. Hosted final-head and resulting-main checks remain separate.
