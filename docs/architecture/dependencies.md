# Dependency boundaries

[rust-toolchain.toml](../../rust-toolchain.toml) and [Cargo.lock](../../Cargo.lock) are exact compiler and dependency authority. The current compiler pin is Rust 1.98.1. Versions below identify key integration boundaries, not a transitive license or distribution inventory.

| Boundary | Current components | Ownership |
|---|---|---|
| State and checked changes | Bundled SQLite through rusqlite 0.40.2 | `brn-threads-core` owns authoritative records, history, authorization, replay and recovery |
| Application service | Core, transient intake and derived retrieval | `brn-threads-app` shares business operations across CLI, desktop and agent |
| Provider runtime | Published Rig 0.44.0 and retained provider/auth transports | `brn-ai` owns bounded runtime/tools; no vendored Rig patch |
| Native surface | GPUI Kit 0.7.1 | `brn-desktop` optional native UI/test features; native observation remains separate |
| Conversion | BetterOffice 0.3.0, mail-parser 0.11.8, bounded image/HTML decoding; Poppler executables for PDF | `brn-intake` restricted helper and `brn-threads-intake` transient adapter |
| Search | SQLite FTS5; optional FastEmbed 7.1.0/ONNX and explicit model installer | `brn-retrieval` derived index; current application search uses keyword retrieval |
| Entry points | Thin commands and native host | `brn` and `brn-desktop` call the shared service |

Default builds keep optional native graphs out. Provider, converter and retrieval types do not replace the core's authority contract. Retained capability fixtures and [runtime qualification](../archive/threads-rebuild/evidence.md#rig-044-runtime-integration) remain relevant when those boundaries change.

Pin changes require relevant source/build/behavior evidence; no compiler upgrade is assigned by this document. Distribution would additionally need artifact provenance, notices, signing and clean-machine qualification. Those are separate from local build success. Earlier dependency snapshots are preserved at the [fixed historical baseline](../archive/README.md).
