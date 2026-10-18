#[path = "../helper_runtime.rs"]
mod helper_runtime;

fn main() {
    helper_runtime::main_with(handle);
}
fn handle(prepare: bool, input: &[u8]) -> Result<Vec<u8>, String> {
    let request: brn_intake::HelperRequest =
        serde_json::from_slice(input).map_err(|e| format!("invalid helper request: {e}"))?;
    let bytes = if prepare {
        serde_json::to_vec(&brn_intake::helper::prepared::prepare(request)?)
    } else {
        serde_json::to_vec(&brn_intake::helper::extract(request)?)
    }
    .map_err(|e| e.to_string())?;
    Ok(bytes)
}
