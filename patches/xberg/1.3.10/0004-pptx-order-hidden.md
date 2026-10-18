# Xberg 1.3.10: PPTX presentation order and hidden slides

Verified candidate, 18 October 2026; independent actual review pending. Apply [0004](0004-pptx-order-hidden.patch) after accepted 0001 then 0003, against archive SHA-256 `f9859ca06d9ee171c65c91cfbb0a25287a45622e7628f1b45ba5353b473b6f0d`. Reserved PDF 0002 is not composed or qualified.

The patch follows direct PresentationML `sldIdLst` declarations through exact internal OPC slide relationships. Present empty sequences stay empty. Missing source slide parts keep their declared ordinals and existing partial warnings. Only absence of the presentation part permits checked numeric recovery; invalid declarations do not silently invoke that recovery.

The same parsed slide DOM supplies unqualified `show`: absent/true/1 is visible, false/0 is hidden. Hidden content is retained and the existing public `PageInfo.hidden` is populated when pages are requested. Invalid lexical state stays unknown with a warning. A private attribute query checks the actual absence of an attribute namespace because roxmltree's string lookup accepts any namespace.

The exact authorized amendment fixes speaker notes and comments association: follow each actual slide's internal relationship, resolve relative to that slide, then use the unchanged bounded reader and text parsers. Missing optional relations are normal; invalid bindings and unreadable declared parts produce warnings. Real empty comment parts retain `Some(empty)`; random orphan parts are not assigned by ordinal filename. Authors, revision deltas and zero-based declared slide anchors are preserved.

Six upstream files change. The DOCX span/namespace implementation, shared PPTX math converter, renderer, public types, package manifests/lock and root BRN routes are unchanged. This patch does not dispatch Xberg in BRN or retire a reader.

[Source manifest](0004-source-manifest.json) binds the six compiled files, apply order, test driver and cached archive. The standalone harness has 21 actual public-API controls: identical final driver produced before **2 passed / 19 failed**, final **21 passed / 0 failed**. The unchanged namespace/S3 target passes **25/25**; 30 accepted DOCX/consumer values and 24 DOCX inputs match completely. Strict all-target harness Clippy passes; this does not claim dependency-wide lint.

The archive declares MIT and attributes Na'aman Hirschfeld and Tobias Perlstein. Its packaged README links outside the archive for LICENSE; no license file is packaged. This repository stores the narrow patch, not the upstream tree. Full custody and remaining semantic/resource/runtime/integration gates remain qualified. See the [response](../../../docs/work/active/modular-foundation/x2/office1/pptx-order4/response.md) for preserved attempts and limits.
