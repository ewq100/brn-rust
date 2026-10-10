# Threads transient intake

This adapter reuses the restricted `brn-intake-helper` DOCX protocol and the
maintained Poppler `pdftotext`/`pdfimages` executables. It returns derived Markdown,
necessary image assets, source hash/reference, text locators, chosen import intent
and typed coverage. No original DOCX/PDF bytes appear in the returned source records.
No shell, inherited environment, converter stderr or provider calls are used.

The host selects useful knowledge from `UsefulInformation` conversion and commits
that selection through the Threads service. `FullNote` conversion supplies one
logical document; the host creates one protected note with its assets and recorded
coverage. Conversion does not itself create notes or grant mutation authority.

## Qualified scope

The synthetic two-page `harbor-study.pdf` has 19 independently listed objects:
prose/qualifications, source headings, table header/rows, a meaningful raster flow
figure and caption, two references, and an appendix. Both rendered source pages were
visually checked. The DOCX process has 22 listed objects, including roles,
prerequisites, three ordered steps, warning, responsibility table, flow figure,
exception, reference and appendix. Its source paragraphs/table/inline-picture count
were checked independently of the maintained helper output. Inventories are
hand-authored, not generated from the converter's output. All content is synthetic.

PDF source pages retain ordered, fixed-width text blocks so ordinary table columns
remain readable; extracted raster figures link to their source page. This does not
qualify semantic Markdown tables, complex reading order, vector artwork, scans/OCR,
or original typography. Image masks are refused because composition is not
qualified. DOCX inherits the helper's accepted-revision structural conversion and
its explicit limitations for charts, SmartArt, shapes and unrepresented containers.
Mathematics-heavy conversion and a universal completeness detector are out of scope.

`Complete` means all objects in the caller's independently supplied inventory
matched and there were no detected substantive gaps. General full-note conversions
without an inventory are `Partial`, with an explicit unverified-coverage gap.
Useful conversions without one are `Unchecked`; source gaps remain visible.
Removing the necessary figure makes either qualified fixture `Partial`.
Inventory matching ignores Markdown presentation and whitespace; it is a bounded
qualification aid, not semantic verification for arbitrary documents.

## Bounds and lifecycle

Input, protocol output, image bytes/pixels, package expansion and wall-time limits
reuse the existing hard intake profile. One deadline covers the full conversion.
The host owns and kills each process group on cancellation, failure or timeout and
joins its pipes before returning. Poppler has CPU, file-size and descriptor limits
and a macOS sandbox that permits only runtime reads and its private run directory;
network and unrelated source/data reads are denied. Narrow root-directory/xattr
and Homebrew metadata reads are required for macOS dyld startup and dylib symlinks.
Poppler isolation is currently qualified on macOS; other platforms refuse it.

Use an explicit application temporary root outside Git and the knowledge store.
Run directories are private, randomly named and removed on success, failure and
cancellation. Cleanup failure is reported. `cleanup_expired()` removes only owned,
ordinary run directories at least 24 hours old, without following symlinks; call
it at host startup. It also runs before conversion. The original external source
is read-only and remains the owner's responsibility to resupply for retries.

Qualification tests require explicit executable paths, to avoid silently selecting
another converter. Example, once this crate is in the root workspace:

```sh
BRN_INTAKE_HELPER=/absolute/path/brn-intake-helper \
BRN_PDFTOTEXT=/opt/homebrew/bin/pdftotext \
BRN_PDFIMAGES=/opt/homebrew/bin/pdfimages \
cargo test -p brn-threads-intake
```

The adapter has been tested on Rust 1.98.1 against Poppler 26.10.0 and the existing
BetterOffice 0.3.0 helper. Process witnesses exercise denied external-file/network
access, empty environment and cancellation of an owned descendant. Integration
tests exercise both content inventories, deliberate missing figures, unchecked
full versus useful intent, input/output bounds, malformed PDF, timeout, raw-copy
cancellation cleanup and restart expiry. Two ignored test entries are child
process witnesses invoked by their parent tests, not skipped acceptance checks.

No ZIP dependency was changed: DOCX executes the maintained helper's existing ZIP 8
and bounded BetterOffice parser. Upgrading a direct ZIP pin alone would not replace
that reader. This qualification does not claim a ZIP 9 robustness upgrade or expand
supported input formats.
