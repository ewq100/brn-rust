use std::{
    error::Error,
    fs,
    future::Future,
    path::PathBuf,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
    thread,
};
use xberg::{
    ExtractInput, ExtractionConfig, ImageExtractionConfig, OutputFormat, PageConfig, PdfConfig,
    SecurityLimits,
};

// The pdf-only build has no Tokio runtime. Drive its local extraction future
// using the standard Wake contract; no runtime feature or extra crate is needed.
struct ThreadWake(thread::Thread);
impl Wake for ThreadWake {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.unpark();
    }
}
fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(ThreadWake(thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => thread::park(),
        }
    }
}
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 3 {
        return Err("usage: xberg-probe INPUT.pdf OUTPUT_DIRECTORY".into());
    }
    let input = PathBuf::from(&args[1]);
    let output = PathBuf::from(&args[2]);
    fs::create_dir_all(&output)?;
    let bytes = fs::read(&input)?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err("fixture exceeds experiment input cap".into());
    }
    let config = ExtractionConfig {
        use_cache: false,
        disable_ocr: true,
        enable_quality_processing: false,
        output_format: OutputFormat::Plain,
        pages: Some(PageConfig {
            extract_pages: true,
            keep_native_content: true,
            ..Default::default()
        }),
        images: Some(ImageExtractionConfig {
            extract_images: true,
            inject_placeholders: false,
            auto_adjust_dpi: false,
            run_ocr_on_images: false,
            max_images_per_page: Some(100),
            ..Default::default()
        }),
        pdf_options: Some(PdfConfig {
            extract_images: true,
            extract_tables: false,
            ..Default::default()
        }),
        security_limits: Some(SecurityLimits {
            max_pages: Some(20),
            ..Default::default()
        }),
        extraction_timeout_secs: Some(30),
        ..Default::default()
    };
    fs::write(
        output.join("config.json"),
        serde_json::to_vec_pretty(&config)?,
    )?;
    let result = block_on(xberg::extract(
        ExtractInput::from_bytes(bytes.clone(), "application/pdf", Some("fixture.pdf".into())),
        &config,
    ))?;
    fs::write(
        output.join("errors.json"),
        serde_json::to_vec_pretty(&result.errors)?,
    )?;
    if !result.errors.is_empty() || result.results.len() != 1 {
        return Err("extraction returned errors or unexpected document count".into());
    }
    let doc = &result.results[0];
    fs::write(output.join("content.txt"), &doc.content)?;
    let pages = doc.pages.as_ref().ok_or("missing per-page evidence")?;
    for page in pages {
        fs::write(
            output.join(format!("page-{}.txt", page.page_number)),
            &page.content,
        )?;
    }
    let mut images = Vec::new();
    for (index, image) in doc.images.iter().flatten().enumerate() {
        let extension = match image.format.as_ref() {
            "png" => "png",
            "jpeg" | "jpg" => "jpg",
            "webp" => "webp",
            _ => "bin",
        };
        let name = format!("image-{index:03}.{extension}");
        fs::write(output.join(&name), &image.data)?;
        images.push(serde_json::json!({"file": name, "format": image.format, "page": image.page_number,
            "index": image.image_index, "width": image.width, "height": image.height,
            "bbox": image.bounding_box, "is_mask": image.is_mask, "bytes": image.data.len(), "ocr": image.ocr_result.is_some()}));
    }
    xberg::pdf::render::install_pdf_render_diagnostics();
    let session = xberg::pdf::PdfRenderSession::open(&bytes, None)?;
    if session.page_count() > 20 {
        return Err("render page cap exceeded".into());
    }
    let mut render_warnings = Vec::new();
    for index in 0..session.page_count() {
        fs::write(
            output.join(format!("render-{}.png", index + 1)),
            session.render_page_to_png(index, Some(150))?,
        )?;
        render_warnings.extend(xberg::pdf::render::take_xberg_native_pdf_render_warnings());
    }
    let summary = serde_json::json!({"pages": pages, "metadata": doc.metadata, "images": images,
        "counts": doc.counts, "warnings": doc.processing_warnings, "render_warnings": render_warnings,
        "render_page_count": session.page_count()});
    fs::write(
        output.join("result.json"),
        serde_json::to_vec_pretty(&summary)?,
    )?;
    Ok(())
}
