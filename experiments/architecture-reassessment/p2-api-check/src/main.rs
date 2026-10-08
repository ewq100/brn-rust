use docx_edit::structured::{
    ExportOptions, MarkdownOptions, RevisionView, StorySelection, export_docx_structured,
    render_docx_markdown,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{env, fs, path::Path};
fn main() {
    let args: Vec<_> = env::args().collect();
    if args[1] == "--guards" {
        let fixtures = Path::new(&args[2]);
        let rows: Vec<_> = ["hostile-path.docx", "hostile-inflate.docx", "hostile-members.docx", "hostile-dtd.docx"].iter().map(|name| {
            let b = fs::read(fixtures.join(name)).unwrap();
            json!({"fixture":name,"opc_16mib":ooxml_opc::unzip_parts_with_limits(&b,16*1024*1024).map(|p|p.len()),"structured":export_docx_structured(&b,&ExportOptions::new(RevisionView::Accepted)).map(|c|json!({"stories":c.stories.len(),"diagnostics":c.diagnostics})).map_err(|e|e.to_string())})
        }).collect();
        println!("{}", serde_json::to_string_pretty(&rows).unwrap());
        return;
    }
    let bytes = fs::read(&args[1]).unwrap();
    let out = Path::new(&args[2]);
    fs::create_dir_all(out).unwrap();
    let options = ExportOptions {
        stories: Some(vec![
            StorySelection::Body,
            StorySelection::Headers,
            StorySelection::Footers,
            StorySelection::Footnotes,
            StorySelection::Endnotes,
            StorySelection::Comments,
        ]),
        ..ExportOptions::new(RevisionView::Accepted)
    };
    let content = export_docx_structured(&bytes, &options).unwrap();
    let limits = docx_parse::ParseLimits {
        max_xml_bytes: 16 * 1024 * 1024,
        max_xml_events: 200_000,
        max_xml_depth: 128,
        ..Default::default()
    };
    let (wire, parsed_parts) = docx_parse::parse_docx_s9_wire_parts_with_limits(
        &bytes,
        docx_parse::S9ParseOptions::default(),
        &limits,
    )
    .unwrap();
    let from_package =
        docx_edit::structured::export_package_structured(wire, &parsed_parts, &options).unwrap();
    assert_eq!(content, from_package);
    let markdown = render_docx_markdown(&content, &MarkdownOptions::default()).unwrap();
    fs::write(
        out.join("structured.json"),
        serde_json::to_vec_pretty(&content).unwrap(),
    )
    .unwrap();
    fs::write(
        out.join("markdown.json"),
        serde_json::to_vec_pretty(&markdown).unwrap(),
    )
    .unwrap();
    let parts = ooxml_opc::unzip_parts_with_limits(&bytes, 16 * 1024 * 1024).unwrap();
    let assets: Vec<_> = parts
        .iter()
        .filter(|(p, _)| p.starts_with("word/media/"))
        .map(|(p, b)| json!({"part":p,"bytes":b.len(),"sha256":format!("{:x}",Sha256::digest(b))}))
        .collect();
    #[cfg(feature = "facade")]
    {
        let doc = betteroffice_docx::Document::open(&bytes).unwrap();
        assert_eq!(doc.export_structured(&options).unwrap(), content);
        assert_eq!(doc.export_markdown(&options).unwrap(), markdown);
    }
    fs::write(
        out.join("package.json"),
        serde_json::to_vec_pretty(
            &json!({"parts":parts.len(),"assets":assets,"facade_checked":cfg!(feature="facade")}),
        )
        .unwrap(),
    )
    .unwrap();
}
