use brn_intake::document::{DocumentExtraction, DocumentRequest};
use xberg::{ExtractInput, ExtractionConfig, OutputFormat, SecurityLimits};

/// Normal SDK text policy; only explicit execution/security/service settings differ.
pub fn config(request: &DocumentRequest) -> ExtractionConfig {
    let limits = &request.limits;
    let e = limits.max_expanded_bytes;
    ExtractionConfig {
        use_cache: false,
        extraction_timeout_secs: None,
        output_format: OutputFormat::Markdown,
        max_concurrent_extractions: Some(1),
        concurrency: Some(xberg::ConcurrencyConfig {
            max_threads: Some(1),
            max_concurrent_ocr: Some(1),
        }),
        disable_ocr: true,
        ocr: None,
        force_ocr: false,
        force_ocr_pages: None,
        ocr_embedded_images: Some(false),
        images: None,
        captioning: None,
        qr_codes: Some(false),
        chunking: None,
        language_detection: None,
        token_reduction: None,
        max_archive_depth: 0,
        max_embedded_file_bytes: Some(limits.max_decoded_bytes as u64),
        security_limits: Some(SecurityLimits {
            max_archive_size: e,
            max_compression_ratio: e,
            max_files_in_archive: limits.max_package_parts,
            max_nesting_depth: limits.max_mime_depth,
            max_entity_length: e,
            max_content_size: e,
            max_iterations: e.saturating_add(limits.max_package_parts),
            max_xml_depth: 128,
            max_table_cells: e,
            max_pages: Some(limits.max_package_parts),
        }),
        url: xberg::UrlExtractionConfig {
            allow_local_file_inputs: false,
            allow_file_uris: false,
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn runtime() -> Result<tokio::runtime::Runtime, String> {
    tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(2)
        .thread_stack_size(16 * 1024 * 1024)
        .enable_all()
        .build()
        .map_err(|e| e.to_string())
}
pub fn extract(request: &DocumentRequest) -> Result<DocumentExtraction, String> {
    request.validate()?;
    if request.backend != env!("BRN_BACKEND") {
        return Err("document helper backend mismatch".into());
    }
    let runtime = runtime()?;
    let result = runtime
        .block_on(xberg::extract(
            ExtractInput::from_bytes(request.bytes.clone(), request.mime_type()?, None),
            &config(request),
        ))
        .map_err(|e| format!("Xberg extraction failed: {e}"))?;
    drop(runtime);
    if !result.errors.is_empty() || result.results.len() != 1 {
        return Err("Xberg did not return exactly one successful document".into());
    }
    crate::project::project(request, result.results.into_iter().next().unwrap())
}
