# Dependency boundaries

[rust-toolchain.toml](../../rust-toolchain.toml) and [Cargo.lock](../../Cargo.lock) are exact compiler and dependency authority. The current compiler pin is Rust 1.98.1. Versions below identify key integration boundaries, not a transitive license or distribution inventory.

| Boundary | Current components | Ownership |
|---|---|---|
| State and checked changes | Bundled SQLite through rusqlite 0.40.2 | `brn-threads-core` owns authoritative records, history, authorization, replay and recovery |
| Application service | Core, transient intake and derived retrieval | `brn-threads-app` shares business operations across CLI, desktop and agent |
| Provider runtime | Published Rig 0.44.0 and retained provider/auth transports | `brn-ai` owns bounded runtime/tools; no vendored Rig patch |
| Native surface | GPUI Kit 0.7.3 (gpui-pre 0.3.9) | `brn-writing-ui` owns the reusable panel; desktop and the native-only writing_host example compose it with shared services. GPUI/native tests remain optional; native observation stays separate |
| Conversion | Pinned Xberg 1.3.10 stock / separately patched optional bundle, light PNG/Markdown validation | Two standalone restricted helpers; `brn-intake` light DTOs and `brn-threads-intake` process adapter |
| Search | SQLite FTS5; optional FastEmbed 7.1.0/ONNX and explicit model installer | `brn-retrieval` derived index; keyword search plus the optional native dense lane and explicitly installed model |
| Entry points | Thin commands, desktop and development example | `brn`, `brn-desktop` and the disposable `brn-writing-ui` writing_host example call the shared service; the example has no desktop dependency |

Default builds keep optional native graphs out. Provider, converter and retrieval types do not replace the core's authority contract. Retained capability fixtures and [runtime qualification](../archive/threads-rebuild/evidence.md#rig-044-runtime-integration) remain relevant when those boundaries change.

Pin changes require relevant source/build/behavior evidence; no compiler upgrade is assigned by this document. Distribution would additionally need artifact provenance, notices, signing and clean-machine qualification. Those are separate from local build success. Earlier dependency snapshots are preserved at the [fixed historical baseline](../archive/README.md).
