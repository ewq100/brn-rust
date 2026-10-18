//! Normal document helpers share the existing restricted process lifecycle.
use crate::*;
use brn_intake::document::{DocumentExtraction, DocumentRequest, expected_engine_identity};

#[derive(Clone, Debug)]
pub struct DocumentHelpers {
    pub stock: PathBuf,
    pub brn: PathBuf,
}

impl DocumentHelpers {
    /// Trusted installation/development paths only. Existence and executable
    /// checks remain on the selected path in the existing process boundary.
    pub fn discover() -> Result<Self, ConversionError> {
        let executable = std::env::current_exe().map_err(|_| ConversionError::Unavailable)?;
        let resolve = |variable: &str, name: &str| {
            let path = std::env::var_os(variable)
                .map(PathBuf::from)
                .unwrap_or_else(|| executable.with_file_name(name));
            if path.is_absolute() {
                Ok(path)
            } else {
                Err(ConversionError::Unavailable)
            }
        };
        Ok(Self {
            stock: resolve("BRN_XBERG_HELPER", "brn-xberg-helper")?,
            brn: resolve("BRN_XBERG_HELPER_BRN", "brn-xberg-helper-brn")?,
        })
    }
}

/// Validated engine plus a fixed host-owned refinement label, never a model label.
pub fn document_conversion_label(backend: &str, ai: bool) -> Result<String, ConversionError> {
    Ok(format!(
        "{};ai={}",
        expected_engine_identity(backend).map_err(|_| ConversionError::Protocol)?,
        if ai { "gpt-6-luna/high" } else { "off" }
    ))
}
impl Converter {
    pub fn convert_document(
        &self,
        request: ConversionRequest<'_>,
        helpers: &DocumentHelpers,
        backend: &str,
        cancel: &AtomicBool,
    ) -> Result<ConversionResult, ConversionError> {
        self.limits
            .validate()
            .map_err(|_| ConversionError::Budget)?;
        if cancel.load(Ordering::Acquire) {
            return Err(ConversionError::Cancelled);
        }
        if request.bytes.is_empty() || request.bytes.len() > self.limits.max_input_bytes {
            return Err(ConversionError::Budget);
        }
        if request.source_reference.is_empty()
            || request.source_reference.len() > 8192
            || request.source_reference.contains('\0')
        {
            return Err(ConversionError::Invalid);
        }
        let helper = match backend {
            "xberg" => &helpers.stock,
            "xberg_brn" => &helpers.brn,
            _ => return Err(ConversionError::Protocol),
        };
        if !helper.is_absolute() {
            return Err(ConversionError::Unavailable);
        }
        // The shared restricted helper runtime is qualified only on macOS.
        // Refuse before cloning input, creating scratch, or dispatching a helper.
        if !cfg!(target_os = "macos") {
            return Err(ConversionError::Unavailable);
        }
        let input = DocumentRequest {
            schema: 1,
            backend: backend.into(),
            kind: match request.kind {
                DocumentKind::Pdf => "pdf",
                DocumentKind::Docx => "docx",
                DocumentKind::Pptx => "pptx",
                DocumentKind::Xlsx => "xlsx",
                DocumentKind::Eml => "eml",
                DocumentKind::Html => "html",
                DocumentKind::Text => "text",
                DocumentKind::Markdown => "markdown",
            }
            .into(),
            limits: self.limits.clone(),
            bytes: request.bytes.to_vec(),
        };
        input.validate().map_err(|_| ConversionError::Invalid)?;
        let deadline = Instant::now() + Duration::from_millis(self.limits.wall_time_ms);
        let temp = transient::TransientDir::create(&self.temp_root)?;
        let result = (|| {
            let output = process::run(
                helper,
                &[],
                Some(serde_json::to_vec(&input).map_err(|_| ConversionError::Protocol)?),
                &temp,
                process::ProcessBudget {
                    output_bytes: self.limits.max_output_bytes,
                    storage_bytes: self.limits.max_output_bytes + self.limits.max_input_bytes,
                    deadline,
                },
                cancel,
            )?;
            let extraction: DocumentExtraction =
                serde_json::from_slice(&output).map_err(|_| ConversionError::Protocol)?;
            extraction
                .validate_for(&input)
                .map_err(|_| ConversionError::Protocol)?;
            let assets = extraction
                .assets
                .iter()
                .map(|asset| {
                    Ok(RetainedAsset {
                        name: brn_intake::asset_file_name(asset)
                            .map_err(|_| ConversionError::Protocol)?,
                        media_type: asset.media_type.clone(),
                        sha256: asset.sha256,
                        width: asset.width,
                        height: asset.height,
                        bytes: asset.bytes.clone(),
                    })
                })
                .collect::<Result<Vec<_>, ConversionError>>()?;
            let result = ConversionResult {
                intent: request.intent,
                markdown: extraction.markdown,
                assets,
                source_sha256: extraction.source_sha256,
                source_reference: request.source_reference.into(),
                converter: document_conversion_label(backend, false)?,
                sources: vec![],
                coverage: Coverage {
                    status: CoverageStatus::Unchecked,
                    checked_items: vec![],
                    gaps: extraction
                        .gaps
                        .into_iter()
                        .map(|detail| CoverageGap {
                            kind: GapKind::Limitation,
                            locator: "document".into(),
                            detail,
                        })
                        .collect(),
                },
            };
            if serde_json::to_vec(&result)
                .map_err(|_| ConversionError::Protocol)?
                .len()
                > self.limits.max_output_bytes
            {
                return Err(ConversionError::Budget);
            }
            if cancel.load(Ordering::Acquire) {
                return Err(ConversionError::Cancelled);
            }
            if Instant::now() >= deadline {
                return Err(ConversionError::Timeout);
            }
            Ok(result)
        })();
        temp.cleanup()?;
        result
    }
}
