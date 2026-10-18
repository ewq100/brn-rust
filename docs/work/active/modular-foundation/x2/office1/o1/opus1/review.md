Rendering note: the complete report below differs from [the verbatim raw report](opus-review-round1.txt) only by link destination normalization. All file links are relative, including links into the external cached registry; original line labels and report text are retained.

**O1 test/evidence candidate: CHANGES_REQUIRED — one bounded P2 test-contract correction.** The empirical measurements are sound and useful. I found no production-route change, feature leak, oracle replacement, or material evidence-integrity problem.

**Production Office cutover: NOT READY.** Correcting the test contract below would not approve Office promotion, reader retirement, or all-document integration.

Confirmed clean `work/x2-office-prep-20261018` at `53546fecf46226c0684250a1091057b5276fc15b`, tree `45fe98de3289f45ad394caf4eb2a919379325ce4`. Cached `origin/main` is `57c710cc`; no remote refresh occurred. Accepted HTML material is ancestral. This remains the task-local combination, not main integration.

I read the original briefs, both complete prior reports, corrections/allocation records, relevant canonical contracts, Office source map, T0 dispositions, new tests, graph/measurement receipts, raw observations and relevant pinned dependency source. Review operations were read-only Git, text, bounded fixture inspection and hashing. No edits, Cargo, builds, tests, network, helper execution or delegation.

**Required correction — P2: put the DTD requirement on the checked boundary.**

[office_differential.rs:674](../../../../../../../../crates/brn-intake/tests/office_differential.rs) constructs an unused internal declaration, calls bare `xberg::extract`, records successful extraction, then requires `dtd_result.is_err()`. This applies BRN’s pre-operation rejection requirement to an operation that deliberately bypasses BRN admission.

The acceptance observation is genuine. It proves **bare Xberg accepts this unused declaration**. It does not prove entity expansion, external access, or an unavoidable upstream-patch requirement.

The current caller map matters:

- [helper.rs:395](../../../../../../../../crates/brn-intake/src/helper.rs) performs ZIP/path/count/actual-expanded-byte admission. It does **not** reject document XML DTDs.
- [helper.rs:422](../../../../../../../../crates/brn-intake/src/helper.rs) subsequently invokes BetterOffice’s bounded XML/wire parser.
- That parser explicitly rejects `Event::DocType` at [xml.rs:682](../../../../../../../../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/betteroffice-docx-parse-0.3.0/src/xml.rs), before producing the wire result.

The smallest correct permanent disposition is:

1. Preserve the original failed executions and their source associations unchanged.
2. Permanently assert the observed bare-backend acceptance, including its meaningful returned cells, while retaining **“checked pre-admission required; promotion gate open.”**
3. Exercise the **same unused-declaration payload** through both existing current public schemas and require a DTD-specific refusal.
4. When a checked Xberg adapter exists, require refusal before backend invocation there.

This preserves the product requirement and independent source facts. It does not waive DTD rejection or demand an upstream patch merely to make the differential suite useful. Update the current disposition in [measurement.md:43](../measurement.md) accordingly; retain historical **6 passed / 1 failed** evidence. Any later passing result must have its own source-qualified receipt.

No production change or broad rerun is necessary to resolve this review finding.

**Oracle and permanent-test assessment**

The source oracles are independent of Xberg output. All four T0 inventories remain byte-identical to `54b33726`; Harbor-process is reused, including all 22 inventory items and the independently authored cell bridge. Prior reports’ preserved raw bytes also match their originals.

The current-reader comparisons are substantive:

- Complete wording and local story order are compared, with coverage checks against unexpected or duplicated atoms.
- Cells retain addresses and spans; authored heading/list roles, labels and link destinations receive explicit comparisons.
- Scoped note definitions, source bytes, image hashes/dimensions, repeated occurrences and schema-1 occurrence locators/ranges are checked.
- Markdown parsing supplies wording comparisons, not provenance.

See [support/office.rs:381](../../../../../../../../crates/brn-intake/tests/support/office.rs). Prefix images remain explicitly partial, rather than being removed from the independent source expectation.

The registry correctly distinguishes `EqualProjection`, `DifferentProjection` and `EvidenceOnly`. Its separate expected/actual hashes prove preservation and drift detection; **they do not prove fidelity between incompatible evidence shapes**. The code and report acknowledge that limit. Direct assertions additionally establish important observed properties: prefix body loss, comment-context absence, span loss, nested-table flattening, note collisions/missing table association, orphan-story behavior and distinct repeated-image indices.

Captured current metadata is meaningful evidence, but opaque context hashes should continue to be described as reviewed, pinned observations—not independent semantic equality assertions. Before promotion, source/story/range/caption associations still require explicit closure. I found no concrete incorrect baseline association in the reviewed captures.

Unknown observations fail after pinning. Unknown warning classes remain promotion gates. No blanket-warning refusal was introduced. Apart from the DTD assertion, capture has become useful permanent regression coverage without replacing source expectations.

**Graph, isolation and fixture assessment**

The feature isolation is correct at [Cargo.toml:10](../../../../../../../../crates/brn-intake/Cargo.toml):

- `office-differential` is nondefault and explicitly enables `helper` plus `xberg/office`.
- Required-feature registrations are present.
- The existing defaults-off Xberg HTML dependency row is unchanged.
- There is no unconditional Office dev-dependency or production Office dispatch change.

My semantic lock comparison confirms **24 added identities, zero removals, zero existing version/checksum/source changes**. After resolving dependency-name disambiguation, existing dependency-edge additions occur only at Xberg. Emitted records distinguish ordinary `html` from explicit `html/notebook/office`; this is stronger evidence than the earlier all-platform metadata alone. No contrary evidence warrants repeating the parent’s archive audit.

The synthetic ZIP changes are appropriately scoped. Size lies alter central declarations while preserving payload/CRC; the descriptor builder preserves true central values, sets descriptor flags, zeros local CRC/sizes and adjusts offsets. Stored-member and descriptor controls preserve complete visible wording. These controls establish their specific ZIP properties, not universal OPC/ZIP conformance. Directory semantics and broader valid-input coverage remain later qualifications.

The test code is bounded and maintainable for this assignment. Most expansion from the compact preparation comes from formatting, explicit fixtures and stronger assertions. I found no material second-reader framework, provenance facade or security waiver. The executor ceiling limits pending polling; it does not bound a synchronous parser poll’s CPU or memory. The report correctly makes no such resource claim.

**Execution and evidence truth**

The reported results are consistent with the retained logs:

- Ordinary all-target command 33: **103 passed**.
- Final affected current target commands 54/56: **8 passed on each graph**.
- Union all-target command 25: **85 passed / 1 DTD failure**, stopping before later targets.
- Explicit union regressions: HTML **27**, protocol **6**, restricted helper **6**, PPTX **12** passed.
- Final Office command 55: **6 passed / 1 unchanged DTD failure**.
- Final strict all-target lint passes on both graphs.

These are composed, source-qualified results, not a final wholly green union run. Passing 103 current-reader tests establishes no Xberg Office parity.

I verified 265 source/log/receipt/inventory/dep-info hash checks, all 26 preserved source-history versions, and all 64 registry observations against their raw captures. Final commands 54–58 match the committed tracked input hashes. The input receipt records 183 calls and 38 distinct byte hashes; shared semantic cases and paired malformed cases have matching current/Xberg input hashes.

The six `current-size-*` normalizations remove only envelope `output_bytes`; raw values remain preserved. Array order and wording are retained. This is an explicit limited exclusion, not fidelity normalization.

I found no missing small evidence artifact or hash discrepancy in these checks. Preserved executable existence/sizes were checked; I did not rehash large executables. No evidence inspected contradicted the stated source/command associations.

Host/AI/native reruns were reasonably omitted for this test-only delta. Union restricted-helper successes exercise legacy Office and actual HTML, **not Xberg Office sandbox qualification**.

**Smallest next production allocation**

Start with checked admission and resource behavior, separately from semantic output repairs:

- Extend the existing `admit_package` path to verify central-declared versus actual non-directory member lengths, preserving descriptor/empty-member controls and explicitly handling directories. Preserve the old malformed-acceptance measurements as historical evidence.
- Establish the complete Xberg XML read set before its XML allocations: required/optional document parts, relationships, bound and conventional orphan headers/footers, styles/theme and metadata. Preserve DTD refusal and the existing valid-input envelope; do not blindly impose one new aggregate quota over additional parts.
- Audit repeated-read/pass accounting. The **24/23** fixed threshold and **7302 versus 7301** shared-header witness are useful measurements, not a general E+P proof. Do not raise quotas automatically.

A maintained low-level XML seam already exists: public `parse_xml` plus `ParseBudget` at [xml.rs:576](../../../../../../../../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/betteroffice-docx-parse-0.3.0/src/xml.rs). It builds a tree and repairs stray ampersands; it is not an exposed allocation-free count scanner. Reuse its established policy where appropriate, or allocate a narrow audited non-retaining validation seam. Neither the full BetterOffice wire/export path nor the sanitizer should be presented as a metadata-only replacement.

Then allocate measured semantic seams:

- Namespace-aware DOCX matching around Xberg parser `parse_body`; the prefix witness now demonstrates actual loss.
- Rich table-grid population around `extractors/docx.rs:319` and the existing native-grid attribute/public span fields.
- Structured note/comment references and comment context, replacing marker-derived association.
- Stable story/part/rId/occurrence facts propagated from parsing.
- A BRN renderer over corrected Xberg semantic nodes to produce existing blocks and exact output ranges.

Existing public span fields, image indices, attributes and annotations are usable seams. Missing source identity/comment/reference facts require maintained or explicitly audited source changes; they cannot be manufactured by rendering flattened Markdown. No patch version, manifest owner or upstream publication authority is implied by this recommendation.

The original nine plan objections remain closed for bounded development. The observed S1/S2/S3/S4/S6/S7 gaps remain valid promotion gates. **S5/O2, PDF/resources, real restricted Xberg Office, host/nested attachments, other document families, retirement, native/platform qualification, full all-document readiness and main integration remain pending.** No owner quality-loss decision is ready.

Approval of the corrected O1 test step would approve empirical engineering evidence only. It would not approve Office production or retirement of the current reader.
