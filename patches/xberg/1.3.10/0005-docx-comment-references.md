# Xberg 1.3.10: source-backed DOCX comment references

Verified candidate, 18 October 2026; new independent actual review pending. Apply [0005](0005-docx-comment-references.patch) after accepted 0001 → 0003 → 0004 against archive SHA-256 `f9859ca06d9ee171c65c91cfbb0a25287a45622e7628f1b45ba5353b473b6f0d`. Reserved PDF 0002 remains uncomposed.

The existing URI-aware `w:commentReference` Empty-event handler now retains IDs in a private paragraph-local map, parallel to typed note references. Only the main document's map is copied to `Document`; table references are excluded from that map. The exporter consumes these IDs instead of scanning rendered `[cmt:…]` text. Authored literal markers remain exact text and cannot create semantic references or missing-target warnings.

Real IDs retain order and multiplicity. The flat validation vector, empty-ID export guard, marker rendering, `cmt{id}` definition keys, existing internal CommentRef/public FootnoteReference relationship class, page/element mapping, definitions, footnotes/endnotes and namespace/budget logic stay unchanged. No public API, dependency, renderer, reference framework or BRN route is added.

Identical final driver: **before 5/7 → final 12/0**. Unchanged namespace/S3 **25/0** and order4 **21/0**; complete accepted legacy values **30/30** and DOCX inputs **24/24** equal. Supplementary order4 captures are **86/86** whole-value equal. Strict all-target harness Clippy passes; this is not dependency-wide lint or whole upstream cfg(test) qualification.

[Manifest](0005-source-manifest.json) binds the two compiled files, genuine harness, archive and patch stack. Full 765 package files reproduce from the cached archive and four patches; nine accepted archive-external test/fixture adjuncts remain unchanged separately. The new standalone test source and all raw source/log/artifact snapshots are frozen under `/private/tmp/brn-x2-docx-comment5-20261018`.

Upstream declares MIT and attributes Na'aman Hirschfeld and Tobias Perlstein; no LICENSE file is packaged and README links outside the archive. This is a narrow patch distribution. Author/date/range/thread metadata, non-body reference association, broader Office/XML/resource/runtime/license/integration and all-eight adoption gates remain open. See the [bounded response](../../../docs/work/active/modular-foundation/x2/office1/comment5/response.md).
