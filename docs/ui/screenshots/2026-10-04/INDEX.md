# BRN native UI screenshots

- Date: 2026-10-04
- Sources: PR31 Current search and the corrected owned Luna Dialogs candidate with the PR32 dialog-layer fix over `13895d9`; corrected binary SHA-256 `ef27891907c492f089c695ac01f9d3752a100fa9e9938ac37bf774f7da3e36e2`
- Model qualification: gpt-6-luna (zero provider calls in these captures)
- Synthetic fixture: `/private/tmp/brn-v1-luna-live-v9s6idgl`
- [luna-live-01-current-search.jpg](luna-live-01-current-search.jpg): PR31 candidate (`869cf8a` / published `fcc5f633`) Vault workspace, Current scope search for `LUNA-PINE-604`; the matching saved current note and exact body are visible. No credentials or authentication screen is present.
- [luna-dialogs-startup.jpg](luna-dialogs-startup.jpg): Corrected dialog candidate at workspace ready, with Current, Source and History scopes visible and the optional local search model approval open. Synthetic fixture only; no authentication screen or provider call.
- [luna-dialogs-settings.jpg](luna-dialogs-settings.jpg): Settings open; appearance and connection section visible, with the synthetic Current/Source/History controls alongside it. No credentials or authentication screen.
- [luna-dialogs-settings-connections.jpg](luna-dialogs-settings-connections.jpg): Lower Settings content shows reasoning-effort guidance, conditional ChatGPT qualification text and the declined optional local-model download. No credentials or authentication screen.

Capture each BRN UI state during future native work and append its screen, source
commit/working change and synthetic fixture here. Keep original returned image
bytes. Exclude sign-in codes, tokens, credentials and private/original data.
Earlier captures without retained image bytes are unavailable. These captures
record observations, not owner acceptance.
