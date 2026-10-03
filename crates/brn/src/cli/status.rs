//! `brn status`: workspace report without starting providers or building indexes.
use crate::cli::{error::CliError, CliFailure, Invocation, Output};
use brn_workflow::Workspace;
use std::path::Path;

fn canonical_dir(path: &Path) -> Result<String, CliError> {
    path.canonicalize()
        .map_err(|e| CliError::Workflow(format!("cannot canonicalize {}: {e}", path.display())))?
        .into_os_string()
        .into_string()
        .map_err(|_| CliError::Workflow("data directory path is not UTF-8".into()))
}

pub fn run(invocation: &Invocation, workspace: &Workspace) -> Result<Output, CliFailure> {
    let st = workspace.workspace_status();
    let data_dir = canonical_dir(&invocation.data_dir)?;
    let native = brn_workflow::native_retrieval_compiled();
    let data = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "data_dir": data_dir,
        "recovered_operations": workspace.recovered_operations,
        "active_index": {
            "present": st.active_present,
            "fingerprint": st.active_fingerprint,
            "error": st.index_error,
        },
        "capabilities": {
            "native_retrieval": native,
        },
    });
    let mut text = format!(
        "version: {}\ndata_dir: {}\nrecovered_operations: {}\nactive_index: {}\n",
        env!("CARGO_PKG_VERSION"),
        data_dir,
        workspace.recovered_operations,
        if st.active_present {
            "present"
        } else {
            "absent"
        },
    );
    if let Some(fingerprint) = &st.active_fingerprint {
        text.push_str(&format!("active_index_fingerprint: {fingerprint}\n"));
    }
    if let Some(index_error) = &st.index_error {
        text.push_str(&format!("active_index_error: {index_error}\n"));
    }
    text.push_str(&format!("native_retrieval: {native}\n"));
    Ok(Output { text, data })
}
