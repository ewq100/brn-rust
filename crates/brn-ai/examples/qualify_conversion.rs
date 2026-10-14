//! Explicit synthetic/public-fixture qualification through production conversion.
//! Never runs automatically; every input/account/evidence path is supplied by the owner.
use brn_ai::conversion::{MODEL, plan_for_prepared};
use brn_threads_intake::{ConversionRequest, Converter, DocumentKind, ImportIntent, PdfTools};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::Instant,
};
use tokio_util::sync::CancellationToken;

fn write_json(
    path: &Path,
    value: &impl serde::Serialize,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    if paths
        .first()
        .is_some_and(|path| path == Path::new("--recheck"))
    {
        return recheck_saved(&paths[1..]);
    }
    if paths.len() != 4 || paths.iter().any(|path| !path.is_absolute()) {
        return Err("Usage: qualify_conversion ABSOLUTE_HELPER ABSOLUTE_SYNTHETIC_INPUT ABSOLUTE_AUTH_FILE ABSOLUTE_FRESH_OUTPUT_DIRECTORY".into());
    }
    let helper = &paths[0];
    let input = &paths[1];
    let auth_file = &paths[2];
    let output = &paths[3];
    let kind =
        match input
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("html" | "htm") => DocumentKind::Html,
            Some("docx") => DocumentKind::Docx,
            Some("eml") => DocumentKind::Eml,
            Some("xlsx") => DocumentKind::Xlsx,
            Some("pptx") => DocumentKind::Pptx,
            Some("pdf") => DocumentKind::Pdf,
            _ => return Err(
                "Select a structured synthetic/public fixture: HTML, DOCX, EML, XLSX, PPTX or PDF"
                    .into(),
            ),
        };
    let metadata = fs::metadata(input)?;
    if !metadata.is_file() || metadata.len() > brn_intake::MAX_INPUT_BYTES as u64 {
        return Err("Fixture exceeds the bounded input profile".into());
    }
    // A pre-existing directory is a hard error: failed receipts are never overwritten.
    fs::create_dir(output)?;
    let started = Instant::now();
    let bytes = fs::read(input)?;
    let reference = input.to_str().ok_or("Fixture path must be UTF-8")?;
    let mut converter = Converter {
        helper_path: helper.clone(),
        pdf_tools: PdfTools::discover(),
        temp_root: output.join("scratch"),
        limits: brn_threads_intake::import_limits(kind),
    };
    // Qualification-only explicit tightened/hard-bounded local profile; production uses import_limits.
    if let Ok(value) = std::env::var("BRN_QUALIFY_PREP_MS") {
        converter.limits.wall_time_ms = value.parse()?;
        converter.limits.validate().map_err(std::io::Error::other)?;
    }
    let request = ConversionRequest {
        kind,
        intent: ImportIntent::FullNote,
        bytes: &bytes,
        source_reference: reference,
        inventory: None,
    };
    let flag = AtomicBool::new(false);
    let prepared = match converter.prepare(request, &flag) {
        Ok(prepared) => prepared,
        Err(error) => {
            write_json(
                &output.join("run.json"),
                &json!({"model":MODEL,"effort":"high",
                "status":"preparation_failed","error":error.to_string(),"elapsed_ms":started.elapsed().as_millis()}),
            )?;
            return Err(format!("Fixture preparation failed: {error}").into());
        }
    };
    let prepared_ms = started.elapsed().as_millis();
    write_json(&output.join("prepared.json"), &prepared)?;
    fs::write(
        output.join("draft.md"),
        brn_intake::prepared::format(&prepared)?,
    )?;
    let model_started = Instant::now();
    let plan = match plan_for_prepared(&prepared, Some(auth_file), None, CancellationToken::new())
        .await
    {
        Ok(plan) => plan,
        Err(error) => {
            write_json(
                &output.join("run.json"),
                &json!({"model":MODEL,"effort":"high",
                "status":"provider_failed","error":error.to_string(),"prepared_ms":prepared_ms,
                "model_ms":model_started.elapsed().as_millis(),"elapsed_ms":started.elapsed().as_millis()}),
            )?;
            return Err(format!("Fixture model conversion failed: {error}").into());
        }
    };
    let model_ms = model_started.elapsed().as_millis();
    write_json(&output.join("plan.json"), &plan)?;
    let candidate = match brn_intake::prepared::apply_plan(&prepared, &plan) {
        Ok(candidate) => candidate,
        Err(error) => {
            fs::write(output.join("plan-error.txt"), &error)?;
            let receipt = json!({"model":MODEL,"effort":"high","status":"plan_rejected",
                "error":error,"blocks":prepared.blocks.len(),"pages":prepared.pages.len(),
                "prepared_assets":prepared.assets.len(),"prepared_ms":prepared_ms,
                "model_ms":model_ms,"elapsed_ms":started.elapsed().as_millis()});
            write_json(&output.join("run.json"), &receipt)?;
            println!("{receipt}");
            return Err(
                "Typed model plan failed deterministic acceptance; receipt retained".into(),
            );
        }
    };
    write_json(&output.join("candidate.json"), &candidate)?;
    fs::write(output.join("candidate.md"), &candidate.markdown)?;
    let blocks = prepared.blocks.len();
    let pages = prepared.pages.len();
    let assets = prepared.assets.len();
    let checker = brn_intake::prepared::check_candidate(&prepared, &candidate);
    if let Err(error) = checker {
        fs::write(output.join("checker-error.txt"), &error)?;
        let receipt = json!({"model":MODEL,"effort":"high","status":"candidate_rejected",
            "error":error,"blocks":blocks,"pages":pages,"prepared_assets":assets,
            "prepared_ms":prepared_ms,"model_ms":model_ms,"elapsed_ms":started.elapsed().as_millis()});
        write_json(&output.join("run.json"), &receipt)?;
        println!("{receipt}");
        return Err("Candidate failed deterministic acceptance; failed receipt retained".into());
    }
    let request = ConversionRequest {
        kind,
        intent: ImportIntent::FullNote,
        bytes: &bytes,
        source_reference: reference,
        inventory: None,
    };
    match converter.finish_candidate(request, prepared, candidate, &flag) {
        Ok(result) => {
            write_json(&output.join("result.json"), &result)?;
            fs::write(output.join("result.md"), &result.markdown)?;
            let receipt = json!({"model":MODEL,"effort":"high","status":"accepted_candidate",
                "coverage":format!("{:?}",result.coverage.status),"retained_assets":result.assets.len(),
                "blocks":blocks,"pages":pages,"prepared_assets":assets,"prepared_ms":prepared_ms,
                "model_ms":model_ms,"elapsed_ms":started.elapsed().as_millis()});
            write_json(&output.join("run.json"), &receipt)?;
            println!("{receipt}");
            Ok(())
        }
        Err(error) => {
            fs::write(output.join("finish-error.txt"), error.to_string())?;
            let receipt = json!({"model":MODEL,"effort":"high","status":"finish_failed",
                "error":error.to_string(),"blocks":blocks,"pages":pages,"prepared_assets":assets,
                "prepared_ms":prepared_ms,"model_ms":model_ms,"elapsed_ms":started.elapsed().as_millis()});
            write_json(&output.join("run.json"), &receipt)?;
            println!("{receipt}");
            Err("Candidate failed asset/coverage acceptance; failed receipt retained".into())
        }
    }
}

/// Rechecks one retained typed plan after a local host correction; never opens auth or transport.
fn recheck_saved(paths: &[PathBuf]) -> Result<(), Box<dyn std::error::Error>> {
    if paths.len() != 4 || paths.iter().any(|path| !path.is_absolute()) {
        return Err("Usage: qualify_conversion --recheck ABSOLUTE_HELPER ABSOLUTE_SYNTHETIC_INPUT ABSOLUTE_SAVED_RECEIPT_DIRECTORY ABSOLUTE_FRESH_OUTPUT_DIRECTORY".into());
    }
    let started = Instant::now();
    let input = &paths[1];
    let saved = &paths[2];
    let output = &paths[3];
    // Preserve original failed/provider receipts and refuse replacement output directories.
    fs::create_dir(output)?;
    let flag = AtomicBool::new(false);
    let prepared_bytes = read_receipt(&saved.join("prepared.json"), 128 * 1024 * 1024)?;
    let plan_bytes = read_receipt(&saved.join("plan.json"), 1024 * 1024)?;
    let prepared: brn_intake::prepared::PreparedDocument = serde_json::from_slice(&prepared_bytes)?;
    let plan: brn_intake::prepared::ConversionPlan = serde_json::from_slice(&plan_bytes)?;
    let kind = match prepared.format.as_str() {
        "html" => DocumentKind::Html,
        "docx" => DocumentKind::Docx,
        "eml" => DocumentKind::Eml,
        "xlsx" => DocumentKind::Xlsx,
        "pptx" => DocumentKind::Pptx,
        "pdf" => DocumentKind::Pdf,
        _ => return Err("Saved fixture format is not supported".into()),
    };
    let converter = Converter {
        helper_path: paths[0].clone(),
        pdf_tools: PdfTools::discover(),
        temp_root: output.join("scratch"),
        limits: brn_threads_intake::import_limits(kind),
    };
    let bytes = brn_threads_intake::read_input_file(
        input,
        converter.limits.max_input_bytes as usize,
        &flag,
    )?;
    let reference = input.to_str().ok_or("Fixture path must be UTF-8")?;
    write_json(&output.join("prepared.json"), &prepared)?;
    write_json(&output.join("plan.json"), &plan)?;
    fs::write(
        output.join("draft.md"),
        brn_intake::prepared::format(&prepared)?,
    )?;
    let checked = (|| -> Result<_, Box<dyn std::error::Error>> {
        let candidate = brn_intake::prepared::apply_plan(&prepared, &plan)?;
        write_json(&output.join("candidate.json"), &candidate)?;
        fs::write(output.join("candidate.md"), &candidate.markdown)?;
        let request = ConversionRequest {
            kind,
            intent: ImportIntent::FullNote,
            bytes: &bytes,
            source_reference: reference,
            inventory: None,
        };
        Ok(converter.finish_candidate(request, prepared, candidate, &flag)?)
    })();
    let receipt = match checked {
        Ok(result) => {
            write_json(&output.join("result.json"), &result)?;
            fs::write(output.join("result.md"), &result.markdown)?;
            json!({"model":MODEL,"effort":"high","mode":"offline_recheck",
                "status":"accepted_candidate","coverage":format!("{:?}",result.coverage.status),
                "retained_assets":result.assets.len(),"saved_receipt":saved,
                "provider_calls":0,"recheck_ms":started.elapsed().as_millis()})
        }
        Err(error) => {
            fs::write(output.join("checker-error.txt"), error.to_string())?;
            json!({"model":MODEL,"effort":"high","mode":"offline_recheck",
                "status":"recheck_rejected","error":error.to_string(),"saved_receipt":saved,
                "provider_calls":0,"recheck_ms":started.elapsed().as_millis()})
        }
    };
    write_json(&output.join("run.json"), &receipt)?;
    println!("{receipt}");
    if receipt["status"] == "accepted_candidate" {
        Ok(())
    } else {
        Err("Saved typed plan failed local recheck; receipt retained".into())
    }
}

fn read_receipt(path: &Path, max_bytes: usize) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use std::io::Read;
    let file = fs::File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > max_bytes as u64 {
        return Err("Saved receipt exceeds its bounded profile".into());
    }
    let mut bytes = Vec::new();
    file.take(max_bytes as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.is_empty() || bytes.len() > max_bytes {
        return Err("Saved receipt exceeds its bounded profile".into());
    }
    Ok(bytes)
}
