# Copilot authentication cache writes during account changes

C found this runtime defect while qualifying [refactor #263](https://github.com/ewq100/brn-rust/issues/263).
It is a separate behavior repair, outside the provider extraction and B's fixed
`a921e325` alpha artifacts. The owner confirmed A done and instructed C to continue;
C owns this necessary shared-file repair. [Coordination](https://github.com/ewq100/brn-rust/issues/253#issuecomment-6701841948).

When a Copilot token exchange yields to a different sign-in (or sign-out), Rig's
combined-record save previously restored the earlier GitHub token into BRN's
bootstrap file. The operation's later account comparison then saw that restored
old value. An in-process HTTP fixture reproduces the overwrite on untouched
`c388da26`, whose runtime equals A's `a921e325`; it is not an extraction regression.
[Original failure and fixture](evidence/provider-auth-write-263/manifest.json) are
retained. No real account or network request was used.

The short-lived legacy token adapter now remembers its last observed Copilot
bootstrap digest. A later load, save or invalidation delete refuses a different
bootstrap before modifying the credential/cache files. Its own successful partial
login, refresh and invalidation writes advance that observation, so the normal
login sequence continues. ChatGPT retains its existing contract. No new auth or
discovery request, provider/model fallback, retry, persistent session store or
raw-credential interface is introduced. Existing unsafe-file validation and error
precedence remain before writes. The refused operation uses the existing
account-changed error and makes no model request with the changed account.

Regression tests inject both a new sign-in and sign-out during token exchange,
require the callback to execute, preserve the newer bootstrap state, refuse the
client and prevent stale session writes. Another test rejects stale reload/save/
delete while preserving a newer account and cache; a fresh operation can use the
new account. Existing partial-login and invalidation tests remain required.

This guards changes observed between adapter accesses; it does not introduce a
cross-process filesystem transaction for arbitrary external writers. Existing
Stop, response limits and checked state/Undo services are unchanged. Earlier
live/native receipts retain their original source scope; this repair has no fresh
installed-app, second-account or owner-acceptance observation. Hosted CI and
current-head code/security review must complete before normal integration.

Local candidate: 765 workspace library tests passed with two parent-invoked
subprocess witnesses ignored; AI contributes 637 of those tests. Strict workspace
all-target Clippy, formatting and Markdown links passed. Complete maintained
storage/end-to-end offline verification was started separately and is pending at
publication; its eventual result and hosted review/CI receipts belong in this PR.
