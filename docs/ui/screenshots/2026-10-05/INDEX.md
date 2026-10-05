# Native Inbox Source analysis — 2026-10-05

Original unmodified JPEG bytes captured through computer use from the local unsigned
BRN Inbox Analysis app. Baseline `65494cdb417cbcc9c85a149958e6f9ea241c98cd`, branch
`codex/v1-native-inbox-analysis`; the captured working Rust manifest is
`a713da244aa4876ae8874ec3c0f71ce38db6a4141234e13e731164707d07cc68`.
The final candidate subsequently clarifies one explanatory sentence in the analysis
panel. These original captures are retained without replacement or alteration.

Fixture: fresh owned `/private/tmp/brn-source-gates-a8nlvkxf/native-inbox-analysis-ui`
data/vault/credential/log folders. The CLI captured a synthetic email, converted it,
prepared its typed Source draft and exactly approved it. The complete saved Source
is 599 UTF-8 bytes and contains the exact 127-byte original with BOM, CRLF and Unicode.
No account connection, model discovery, provider call, download or private/original
data was used. Credential directory remained empty; Source bytes survived normal
app quit unchanged.

| Original screenshot | Observed screen |
| --- | --- |
| [01-ready.jpg](native-inbox-analysis/01-ready.jpg) | Workspace Ready, no selected model, retained applied Source proposal. |
| [02-source-provenance.jpg](native-inbox-analysis/02-source-provenance.jpg) | Source scope read-only document and saved-provenance entry to analysis inspection. |
| [03-analysis-proof.jpg](native-inbox-analysis/03-analysis-proof.jpg) | Guarded entry opened Inbox, loaded complete Source proof/path/hash, and left Analyze disabled without acknowledged model/effort. |

ScreenCaptureKit intermittently returned `SCStreamErrorDomain -3812`. Raising the
window enabled the recorded Source/provenance/analysis navigation; subsequent
typing and retained-lookup observation could not be qualified. No successful capture
was discarded. Exact Copy/read-only/Start/partial behavior passed separate native
widget tests; actual streaming/model/cancellation/group review and owner acceptance
remain pending. Screenshots establish only the observations above.
