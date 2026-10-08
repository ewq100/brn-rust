use super::*;
use brn_intake::{ImageAsset, ImageOccurrence, SourceNode, digest, hex};
use gpui_kit::{
    Context, ImageFormat, ImageSource, Modifiers, Render, Resource, TestAppContext,
    VisualTestContext, point, test::TestWindowExt,
};
use std::sync::atomic::{AtomicUsize, Ordering};

fn asset() -> ImageAsset {
    let bytes = include_bytes!(
        "../../../../experiments/architecture-reassessment/p1-office-mime/fixtures/water-use.png"
    )
    .to_vec();
    let sha256 = digest(&bytes);
    let (width, height) = brn_intake::validate_png_image(&bytes).unwrap();
    ImageAsset {
        id: format!("asset-{}", hex(&sha256)),
        sha256,
        width,
        height,
        media_type: "image/png".into(),
        bytes,
    }
}

struct MarkdownProbe {
    text: String,
}
impl Render for MarkdownProbe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().w(px(700.)).child(safe_markdown(
            "reader-policy-test".into(),
            self.text.clone(),
            cx,
        ))
    }
}

#[gpui_kit::test]
fn inbox_reader_blocks_network_local_custom_and_data_images_in_markdown_and_html(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::component::init);
    let requests = Arc::new(AtomicUsize::new(0));
    let requested = requests.clone();
    cx.update(|cx| {
        cx.http_client()
            .as_fake()
            .replace_handler(move |old, request| {
                requested.fetch_add(1, Ordering::SeqCst);
                async move { old(request).await }
            })
    });
    let image = asset();
    let encoded = serde_json::to_value(&image).unwrap();
    let data = format!(
        "data:image/png;base64,{}",
        encoded["bytes"].as_str().unwrap()
    );
    let urls = [
        "https://example.invalid/track.png".to_owned(),
        "file:///private/tmp/untrusted.png".to_owned(),
        "/private/tmp/untrusted.png".to_owned(),
        "vault://private/image.png".to_owned(),
        data,
    ];
    let decoded = Arc::new(Image::from_bytes(ImageFormat::Png, image.bytes));
    for url in &urls {
        for text in [
            format!("![image]({url})"),
            format!("<span><img src=\"{url}\"></span>"),
            format!("<img src=\"{url}\">"),
        ] {
            // Put every resource form in the visible first row. An offscreen
            // image that simply never painted would not qualify this policy.
            let (_, visual) = cx.add_window_view(|_, _| MarkdownProbe { text });
            let visual: &mut VisualTestContext = visual;
            visual.run_until_parked();
            visual.update(|window, cx| {
                window.render_frame(cx);
            });
            visual.run_until_parked();
            assert_eq!(
                requests.load(Ordering::SeqCst),
                0,
                "untrusted markup reached the HTTP loader"
            );
            assert!(
                !visual.update(
                    |_, cx| ImageSource::Resource(Resource::Uri(url.clone().into()))
                        .is_asset_cached(cx)
                ),
                "untrusted URI reached image asset loading"
            );
            assert!(
                !visual.update(|_, cx| ImageSource::Image(decoded.clone()).is_asset_cached(cx)),
                "unqualified data URI decoded an image"
            );
        }
    }
    let text =
        "![reference][tracked]\n\n[tracked]: https://example.invalid/reference.png\n".to_owned();
    let (_, visual) = cx.add_window_view(|_, _| MarkdownProbe { text });
    let visual: &mut VisualTestContext = visual;
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
    });
    visual.run_until_parked();
    assert_eq!(
        requests.load(Ordering::SeqCst),
        0,
        "reference image reached HTTP loader"
    );
}

#[gpui_kit::test]
fn inbox_reader_links_never_open_any_url_or_local_target(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::init);
    for target in [
        "https://example.invalid/track",
        "file:///private/tmp/untrusted",
        "vault://private/note",
        "javascript:alert(1)",
    ] {
        let (_, visual) = cx.add_window_view(|_, _| MarkdownProbe {
            text: format!("[Blocked link]({target})"),
        });
        let visual: &mut VisualTestContext = visual;
        visual.run_until_parked();
        visual.update(|window, cx| {
            window.render_frame(cx);
        });
        visual.simulate_click(point(px(10.), px(10.)), Modifiers::default());
        assert_eq!(visual.opened_url(), None, "reader opened {target}");
    }
}

struct ReaderProbe {
    extraction: Extraction,
    images: BTreeMap<(String, [u8; 32]), Arc<Image>>,
}
impl Render for ReaderProbe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        super::render(&self.extraction, &self.images, None, cx)
    }
}

#[gpui_kit::test]
fn inbox_reader_keeps_repeated_pictures_with_their_document_and_marks_unread_attachments(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::component::init);
    let image = asset();
    let link = format!(
        "![water use]({})",
        brn_intake::asset_file_name(&image).unwrap()
    );
    let metadata = "# Email\n\nFrom\n\n```\nMira <mira@example.invalid>\n```\n";
    let document = format!(
        "# Harbor pilot\n\nWater use reduced from 120 to 72 L/day.\n\n{link}\n\nRepeated chart\n\n{link}\n"
    );
    let markdown = format!("{metadata}\n{document}");
    let original = include_bytes!(
        "../../../../experiments/architecture-reassessment/p1-office-mime/fixtures/single.eml"
    )
    .to_vec();
    let docx = include_bytes!(
        "../../../../experiments/architecture-reassessment/p1-office-mime/fixtures/harbor.docx"
    )
    .to_vec();
    let occurrences = markdown
        .match_indices(&link)
        .enumerate()
        .map(|(index, (start, _))| ImageOccurrence {
            id: format!("occurrence-{index}"),
            source_id: "document".into(),
            asset_id: image.id.clone(),
            locator: format!("document/image/{index}"),
            alt: Some("Water use before and after".into()),
            start,
            end: start + link.len(),
        })
        .collect();
    let extraction = Extraction {
        limits: Default::default(),
        consumed: None,
        schema: 1,
        converter: brn_intake::CONVERTER.into(),
        original_sha256: digest(&original),
        markdown,
        sources: vec![
            SourceNode {
                id: "email".into(),
                parent: None,
                name: "harbor.eml".into(),
                media_type: "message/rfc822".into(),
                locator: "original".into(),
                status: "partial".into(),
                bytes: original,
                text: metadata.into(),
            },
            SourceNode {
                id: "container".into(),
                parent: Some("email".into()),
                name: "unnamed MIME part".into(),
                media_type: "multipart/mixed".into(),
                locator: "mime/container".into(),
                status: "container".into(),
                bytes: vec![],
                text: String::new(),
            },
            SourceNode {
                id: "document".into(),
                parent: Some("container".into()),
                name: "harbor.docx".into(),
                media_type: DOCX.into(),
                locator: "mime/document".into(),
                status: "partial".into(),
                bytes: docx,
                text: document,
            },
            SourceNode {
                id: "spreadsheet".into(),
                parent: Some("container".into()),
                name: "forecast.xlsx".into(),
                media_type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
                    .into(),
                locator: "mime/spreadsheet".into(),
                status: "unprocessed".into(),
                bytes: b"opaque retained spreadsheet fixture".to_vec(),
                text: String::new(),
            },
        ],
        assets: vec![image.clone()],
        occurrences,
        gaps: vec!["document: Original layout and charts require inspection.".into()],
    };
    extraction.validate().unwrap();
    let images = BTreeMap::from([(
        (image.media_type, image.sha256),
        Arc::new(Image::from_bytes(ImageFormat::Png, image.bytes)),
    )]);
    let (_, visual) = cx.add_window_view(|_, _| ReaderProbe { extraction, images });
    let visual: &mut VisualTestContext = visual;
    visual.run_until_parked();
    visual.update(|window, cx| {
        window.render_frame(cx);
        for id in [
            "inbox-reader-source-0",
            "inbox-reader-source-2",
            "inbox-reader-source-3",
            "inbox-reader-picture-0",
            "inbox-reader-picture-1",
            "inbox-reader-gaps",
        ] {
            assert!(
                window.try_find(id).is_some(),
                "missing readable evidence widget {id}"
            );
        }
        assert!(
            window.try_find("inbox-reader-source-1").is_none(),
            "MIME container presented as an attachment"
        );
    });
}
