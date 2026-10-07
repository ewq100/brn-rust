# P1 Office/MIME evaluation — 2026-10-07

Standalone evidence at `main@3d59cdd4d6379b526fed19f6a339cfd2dce2dead` (PR85 merged).
No product code/manifests, private input, model downloads or provider calls. This
executes the newly selected P1; historical finalization-only restrictions do not
apply. The proposed P2 specification is owned by the lead outside this folder.

**Recommendation: reuse BetterOffice DOCX/PPTX 0.3.0 and mail-parser 0.11.8 through
a small adapter for the selected partial-review profile.** Ordinary producer
DOCX content, meaningful exact PNG assets, repeated picture occurrences, actual
MIME parent/children and visible gaps work. A native constrained helper works on
this Apple Silicon host. This supports the bounded P2 design; it does not qualify
all Office fidelity, cleanup completeness, the production UI, lifetime maintenance
savings or a distributable installation.

## Useful evidence and actual limits

- [Fixtures](fixtures/) are benign synthetic producer output: python-docx1.2.0,
  python-pptx1.0.2, Pillow12.3.0 and openpyxl3.1.5. The 900x500 PNG is a readable
  measured-water-use chart (120 to72 litres/day,40% reduction). This replaces the
  previous non-image CID sentinel. [Fixture hashes](evidence/fixture-manifest.json)
  bind the final original bytes; experiment-local attributes prevent Git text
  normalization of EML and retained payload bytes.
- [DOCX result](evidence/harbor-docx/result.json) preserves the decision/owner/date,
  a3x3 table (including **two separate Pending cells**), review-date header and
  budget-constraint footer. Upstream wire objects preserve structure; flat text is
  supplemental. Text evidence has exact saved-wire pointers and upstream source
  ordinals, scoped by its containing wire story. Picture IDs1/2 each bind their
  title, part, relationship and source node to the same exact PNG blob. They remain
  two content occurrences and are displayed at their original paragraph positions.
- [PPTX result](evidence/harbor-pptx/result.json) has two slides: repeated PNG
  pictures3/4, a titled **non-picture** inspection dependency gate and presenter
  notes, plus a native chart of inspection capacity (2 slots on12October,0 on13).
  The title supplement scopes to each containing picture/inline and resolves the
  embedded relationship attribute by namespace/local name. It checks OPC image
  relationship namespace/Type; the non-picture title cannot become picture
  metadata. Slide text, notes and picture bytes are useful; the chart's **visual
  and its scheduling facts are omitted from the adapted review**. A prominent
  per-slide warning and chart part locator show this material gap. The original
  chart is readable in the retained preview. Generic uncertainty alone is not the
  evidence of this omission.
- [Single EML](evidence/single-eml/result.json) contains one selected DOCX and one
  inline CID PNG. [Plural EML](evidence/plural-eml/result.json) additionally has an
  XLSX attachment, retained byte-for-byte and visibly **unprocessed**. Standard
  library MIME decoding independently verifies exact attachment/CID bytes, actual
  decoded subject/message ID/date offset and parent associations. An attachment
  node is `parent-source + attachment-index`; its blob SHA is separate. The same
  DOCX bytes in two different messages get different parent/occurrence sources.
  CID is a retained email resource, not standalone-image import. This experiment
  does not implement HTML sanitization, MIME authentication or thread inference.
- [Oracle](evidence/oracle.json) passes against independently parsed producer ZIP
  relationships and stdlib MIME, rather than only searching sentinels in output.
  Four hostile packages are rejected before BetterOffice: traversal member path,
  DTD,17MiB declared inflation over16MiB budget, and513 members over512.

The profile is **partial**, never complete enough for binary-copy cleanup. Normal
body/header/footer/table/PNG and simple slide text/notes/PNG are demonstrated.
JPEG, arbitrary namespaces/OOXML root layouts, grouped/anchored complex drawings,
revision semantics, ruby, unknown containers, SmartArt and charts are not fully
qualified. rdocx0.15.0 remains the source/H4 comparator; ordinary content here did
not expose a material DOCX gap needing a new rdocx runtime campaign. The existing
H4/reuse-probe loss evidence remains valid; it was not repeated. XLSX conversion
remains out of scope; PPTX's embedded chart workbook does not establish XLSX intake.

## Human inspection

Read the original and adapted PDFs side by side:

| Original | Adapted evidence | Observed difference |
| --- | --- | --- |
| [DOCX original](evidence/original-docx/harbor.pdf) | [DOCX adapted](evidence/adapted-docx.pdf) | Two pages each; table values/header/footer and both chart occurrences present; reflowed styling/pagination |
| [PPTX original](evidence/original-pptx/harbor.pdf) | [PPTX adapted](evidence/adapted-pptx.pdf) | Two slides/pages each; notes additionally visible; diagram gate text retained; second-slide native chart replaced by a prominent omission notice |

All eight final page PNGs were visually inspected for legibility, meaningful
content and omissions. The reflowed evidence viewer operates on upstream wire
objects; it is not a new OOXML interpreter or DrawingML renderer. The original
preview uses locally available LibreOfficeDev26.8.0.0.alpha0
`2c87e51eeaa2b413ff4ae097b2705eea1995d8e5`, outside the measured native helper.
Python/LibreOffice are evaluation/inspection tools, **not native-helper runtime
dependencies or a selected production renderer**. Fonts use an isolated temporary
fontconfig cache; final render stderr is saved. Preview fidelity/production
renderer packaging still need explicit qualification.

## Actual helper containment and cancellation

[Runner](runner.py) clears the environment, uses a fresh input/output directory,
sets3CPU seconds/64FDs/32MiB per output file, and applies a5second process-group
wall watchdog. The native helper starts its trusted Rust runtime, reads only the
host-created policy, then calls `sandbox_init` **before input read, MIME or Office
parsing**. Activation failure exits70 without output. Input reading itself is
bounded to8MiB+1; admitted input<=8MiB, aggregate ZIP inflation<=16MiB, <=512members,
XML depth<=128 and <=200,000events per XML part. Upstream parser budgets remain
additional, not the outer ZIP admission mechanism.

The [deny-default policy](evidence/sandbox.sb) permits data reads only from the
single per-run input root and system runtime library locations, writes only to its
output root, and process fork solely for the cancellation witness. It permits no
network, Mach service lookup or executable launch. The trusted policy/binary paths
are host arguments, not MIME filenames. Saved [negative-access output](evidence/restriction.stdout.txt)
shows `EPERM` for synthetic vault/credential canaries and the repository manifest;
loopback connection to a live harness listener also gets `EPERM`. No real vault or
credential contents were inspected.

[Measurements](evidence/measurements.json) include successful conversion inside
that policy, fail-closed activation, a5second stalled-tree watchdog witness and
manual cancellation of a parent+child+grandchild. All three PIDs were gone after
process-group termination. This proves the tested inherited process group; it
is not a general OS exploit defense or a promise about arbitrary detached child
creation. Hard RSS limits are **not established on macOS**; measured RSS, bounded
input/inflation/events, CPU/time and output caps are the demonstrated controls.
MIME0.11.8 does not provide the newer advertised pre-parse part/depth caps. No
large/adversarial MIME corpus or total-output manifest validation is claimed.

Initial `sandbox-exec` before runtime startup aborted in macOS27 ignition before
`main` (SIGABRT,737,280RSS bytes, empty stdout; crash frames `ignition_halt/boot_boot`,
termination namespace0x23/code2). Narrow runtime allowances did not fix startup;
[failed raw measurements/policy](evidence/failed-sandbox-launch/measurements.json)
are kept separately and excluded from converter/cancellation success. Applying the
same data restrictions natively after trusted runtime startup resolved the blocker.
The precise macOS ignition cause remains unknown.

An automatic approval reviewer rejected one diagnostic request for unrestricted
file reads because it could expose private data/credentials. That command **was
not executed** and was not retried. The successful native path retained deny-default
data permissions; there is no remaining request for that rejected broad access.

## Cost, integration and reproduction

Complete upstream `*.wire.json` payloads remain ordinary readable JSON files, but
experiment-local Git attributes suppress their enormous text diffs. Review the
small `result.json` summaries and adapter alongside the saved raw wire as needed;
no raw bytes were removed or changed for this presentation choice.

[Cost report](evidence/costs.json), [enabled arm64 feature tree](evidence/tree-features.txt),
[package metadata](evidence/metadata.json) and separate clean/warm build logs preserve
actual versions/checksums/licenses and measurement scope. `costs.json` is authoritative
for final byte/time/LOC values. Runtime measurements include native sandbox activation
and output serialization; first-process timing has uncontrolled OS caches, not a
cold machine guarantee. RSS is per-run child peak, not aggregate application RSS.
Clean compilation has an empty target with public source cache populated; build RSS
is maximum child/descendant RSS, not summed concurrent build memory.


Measured final helper runs, after build competition ended (first process has uncontrolled OS caches):

| Input | First process ms | Warm process ms | Maximum RSS MiB |
| --- | --- | --- | --- |
| harbor.docx | 28.291 | 25.697, 25.782 | 44.80 |
| harbor.pptx | 10.759 | 10.302, 10.135 | 9.75 |
| single.eml | 26.320 | 26.453, 26.294 | 44.86 |
| plural.eml | 26.526 | 30.972, 26.964 | 44.91 |

Final isolated clean-target compilation: 35.805 seconds, 1606.5 MiB maximum child RSS; warm no-change build: 0.095 seconds. There are 56 locked registry packages and 53 enabled normal registry packages on arm64; 10 exact name/version pairs differ from the root lock (8 new package names). Enabled source archives total 6082560 bytes; incremental exact-pair archives total 2479002 bytes. Release helper: 5156192 bytes; stripped: 4297440 bytes; helper-only gzip archive: 1881572 bytes. Against a stripped empty Rust helper, additional stripped bytes are 3924400. Main adapter/helper is 400 lines, with 69 runner and 62 evidence-viewer lines separately. Tests/fixture/cost scripts are not a production-code estimate. Earlier [simultaneous-build measurements](evidence/measurements-concurrent-build.json) remain raw evidence, excluded from the representative timing table.

The helper reuses upstream models. Remaining BRN work is the bounded helper
supervisor/activation, shared admission, source-node/blob/occurrence snapshot DTOs,
scoped title access and evidence view, omission profile, durable parser output and
existing exact-approval integration. Safety checks remain shared; old extraction
records need historical readers. There is no second approval system. A small
native surface is plausible versus expanding the strict custom Office interpreter,
but measured LOC is not a proof of lower total maintenance cost. Full BRN binary
link delta, renderer payload, signing/notarization, clean-machine install/update/
rollback and delta-update transfer bytes are unmeasured. The helper-only stripped
binary/archive sizes are installation/full-replacement-update payload **estimates
from actual isolated artifacts**, not a delivered BRN installer.

Reproduce from the repository root (no private inputs or live providers):

```sh
export PATH=/opt/homebrew/opt/rustup/bin:$PATH
P1=experiments/architecture-reassessment/p1-office-mime
PY=/Users/evokessler/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3
"$PY" "$P1/fixtures.py"
CARGO_HOME=/private/tmp/brn-p1-cargo CARGO_TARGET_DIR=/private/tmp/brn-p1-target cargo build --manifest-path "$P1/Cargo.toml" --locked --release
CARGO_HOME=/private/tmp/brn-p1-cargo CARGO_TARGET_DIR=/private/tmp/brn-p1-target cargo clippy --manifest-path "$P1/Cargo.toml" --locked --offline --all-targets -- -D warnings
cargo fmt --manifest-path "$P1/Cargo.toml" --check
python3 "$P1/runner.py"
python3 "$P1/oracle.py"
python3 "$P1/costs.py"
```

Only first dependency fetch needs public network. The harness needs permission to
apply its own macOS sandbox and inspect/terminate its synthetic process group;
inside an outer sandbox it may be rejected independently. It never needs to read
the home directory. Regenerating Office fixtures changes ZIP timestamps/hashes;
rerender originals and rerun outputs/oracle together. Use `soffice --headless
-env:UserInstallation=file:///private/tmp/brn-p1-lo-profile --convert-to pdf
--outdir "$P1/evidence/original-docx" "$P1/fixtures/harbor.docx"` and the analogous
PPTX command with `original-pptx`, then `"$PY" "$P1/render.py"`. `render.py` creates
an isolated temporary fontconfig config/cache at `/private/tmp/brn-p1-*`
paths on this host; the final artifacts are already saved. [Interim raw outputs](evidence/interim-output.tar.gz)
are compressed to avoid duplicate payloads; they are superseded by extension-bearing
final output directories and are not part of the final oracle.
