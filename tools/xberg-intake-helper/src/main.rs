mod engine;
#[path = "../../../crates/brn-intake/src/helper_runtime.rs"]
mod helper_runtime;
mod project;

fn main() {
    #[cfg(target_os = "macos")]
    // SAFETY: this is the entry point, before input and before any BRN, Xberg,
    // Tokio or Rayon runtime/thread creation. Linked startup was witnessed with
    // one thread. CoreFoundation inserts this process-local marker during loader
    // initialization even for an empty exec environment. Remove ONLY that marker;
    // the shared runtime still rejects every other environment entry.
    unsafe {
        std::env::remove_var("__CF_USER_TEXT_ENCODING");
    }
    helper_runtime::main_with(handle);
}
fn handle(prepare: bool, bytes: &[u8]) -> Result<Vec<u8>, String> {
    if prepare {
        return Err("document helper does not support --prepare".into());
    }
    let request =
        serde_json::from_slice(bytes).map_err(|e| format!("invalid document request: {e}"))?;
    let result = engine::extract(&request)?;
    serde_json::to_vec(&result).map_err(|e| e.to_string())
}
