use mail_parser::{MessageParser, MimeHeaders};
use quick_xml::{NsReader, events::Event};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::{Cursor, Read},
    path::{Path, PathBuf},
};
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn save(out: &Path, name: &str, b: &[u8]) {
    std::fs::write(out.join(name), b).unwrap();
}
// Shared outer admission is intentionally separate from upstream parser budgets.
fn package(data: &[u8]) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let mut zip = zip::ZipArchive::new(Cursor::new(data)).map_err(|e| e.to_string())?;
    if zip.len() > 512 {
        return Err("member budget 512".into());
    }
    let mut sum = 0u64;
    let mut parts = BTreeMap::new();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).map_err(|e| e.to_string())?;
        let name = f.name().to_owned();
        if name.starts_with('/')
            || name.contains('\\')
            || name.split('/').any(|p| p == ".." || p.is_empty())
            || name.contains(':')
        {
            return Err("unsafe member path".into());
        }
        if parts.contains_key(&name) {
            return Err("duplicate member".into());
        }
        sum = sum.checked_add(f.size()).ok_or("inflated overflow")?;
        if sum > 16 * 1024 * 1024 {
            return Err("inflated budget 16MiB".into());
        }
        let mut bytes = Vec::new();
        f.by_ref()
            .take(16 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 != f.size() {
            return Err("declared size mismatch".into());
        }
        if name.ends_with(".xml") || name.ends_with(".rels") {
            let mut r = NsReader::from_reader(bytes.as_slice());
            let mut depth = 0usize;
            let mut events = 0usize;
            loop {
                events += 1;
                if events > 200_000 {
                    return Err("XML event budget".into());
                }
                match r.read_resolved_event().map_err(|e| e.to_string())?.1 {
                    Event::DocType(_) => return Err("DTD forbidden".into()),
                    Event::Start(_) => {
                        depth += 1;
                        if depth > 128 {
                            return Err("XML depth budget 128".into());
                        }
                    }
                    Event::End(_) => depth = depth.saturating_sub(1),
                    Event::Eof => break,
                    _ => (),
                }
            }
        }
        parts.insert(name, bytes);
    }
    Ok(parts)
}
fn attrs(e: &quick_xml::events::BytesStart<'_>, r: &NsReader<&[u8]>) -> BTreeMap<String, String> {
    e.attributes()
        .map(|a| {
            let a = a.unwrap();
            (
                String::from_utf8_lossy(a.key.as_ref()).into_owned(),
                a.decoded_and_normalized_value(quick_xml::XmlVersion::Implicit1_0, r.decoder())
                    .unwrap()
                    .into_owned(),
            )
        })
        .collect()
}
// Raw supplement binds title to one containing picture/inline, never scans free title lists.
fn occurrences(parts: &BTreeMap<String, Vec<u8>>, source: &str, out: &Path) -> Vec<Value> {
    let mut result = vec![];
    for (part, bytes) in parts.iter().filter(|(p, _)| {
        p.ends_with(".xml") && (p.starts_with("word/") || p.starts_with("ppt/slides/"))
    }) {
        let parent = Path::new(part).parent().unwrap();
        let rel_path = parent.join("_rels").join(format!(
            "{}.rels",
            Path::new(part).file_name().unwrap().to_str().unwrap()
        ));
        let mut rels = BTreeMap::new();
        if let Some(b) = parts.get(rel_path.to_str().unwrap()) {
            let mut r = NsReader::from_reader(b.as_slice());
            loop {
                match r.read_resolved_event().unwrap() {
                    (
                        quick_xml::name::ResolveResult::Bound(ns),
                        Event::Empty(e) | Event::Start(e),
                    ) if e.local_name().as_ref() == b"Relationship"
                        && ns.as_ref()
                            == b"http://schemas.openxmlformats.org/package/2006/relationships" =>
                    {
                        let a = attrs(&e, &r);
                        if a.get("TargetMode").map(String::as_str) != Some("External") && a.get("Type").is_some_and(|t|t=="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image") && let (Some(id),Some(target))=(a.get("Id"),a.get("Target")) {rels.insert(id.clone(),target.clone());}
                    }
                    (_, Event::Eof) => break,
                    _ => (),
                }
            }
        }
        let mut r = NsReader::from_reader(bytes.as_slice());
        let mut depth = 0usize;
        let mut current: Option<(usize, BTreeMap<String, String>, String)> = None;
        loop {
            let (ns, event) = r.read_resolved_event().unwrap();
            let ns = match ns {
                quick_xml::name::ResolveResult::Bound(n) => n.as_ref().to_vec(),
                _ => vec![],
            };
            match event {
                Event::Start(ref e) | Event::Empty(ref e) => {
                    let empty = matches!(event, Event::Empty(_));
                    if !empty {
                        depth += 1;
                    }
                    let local = e.local_name();
                    let is_container=(local.as_ref()==b"inline"||local.as_ref()==b"anchor") && ns==b"http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" || local.as_ref()==b"pic" && ns==b"http://schemas.openxmlformats.org/presentationml/2006/main";
                    if is_container {
                        current = Some((depth, BTreeMap::new(), String::new()));
                    }
                    if let Some((_, meta, rid)) = &mut current {
                        if local.as_ref() == b"docPr" || local.as_ref() == b"cNvPr" {
                            let a = attrs(e, &r);
                            for (k, v) in a {
                                meta.entry(k).or_insert(v);
                            }
                        }
                        if local.as_ref() == b"blip"
                            && ns == b"http://schemas.openxmlformats.org/drawingml/2006/main"
                        {
                            for a in e.attributes() {
                                let a = a.unwrap();
                                let (ans, local) = r.resolver().resolve_attribute(a.key);
                                if matches!(ans,quick_xml::name::ResolveResult::Bound(n) if n.as_ref()==b"http://schemas.openxmlformats.org/officeDocument/2006/relationships")
                                    && local.as_ref() == b"embed"
                                {
                                    *rid = a
                                        .decoded_and_normalized_value(
                                            quick_xml::XmlVersion::Implicit1_0,
                                            r.decoder(),
                                        )
                                        .unwrap()
                                        .into_owned();
                                }
                            }
                        }
                    }
                }
                Event::End(_) => {
                    if current.as_ref().is_some_and(|c| c.0 == depth) {
                        let (_, meta, rid) = current.take().unwrap();
                        if !rid.is_empty() {
                            let Some(target) = rels.get(&rid) else {
                                depth = depth.saturating_sub(1);
                                continue;
                            };
                            let mut path = PathBuf::new();
                            for c in parent.join(target).components() {
                                match c {
                                    std::path::Component::ParentDir => {
                                        path.pop();
                                    }
                                    _ => path.push(c),
                                }
                            }
                            let target = path.to_str().unwrap();
                            let Some(asset) = parts.get(target) else {
                                depth = depth.saturating_sub(1);
                                continue;
                            };
                            let digest = hash(asset);
                            save(out, &format!("{digest}.png"), asset);
                            result.push(json!({"source":source,"part":part,"picture_id":meta.get("id"),"name":meta.get("name"),"title":meta.get("title"),"description":meta.get("descr"),"relationship":rid,"asset_part":target,"asset_sha256":digest,"occurrence":result.len()+1}));
                        }
                    }
                    depth = depth.saturating_sub(1);
                }
                Event::Eof => break,
                _ => (),
            }
        }
    }
    result
}
fn text(v: &Value, items: &mut Vec<String>) {
    match v {
        Value::Object(o) => {
            for (k, v) in o {
                if (k == "text" || k == "notes") && v.is_string() {
                    items.push(v.as_str().unwrap().into())
                } else {
                    text(v, items)
                }
            }
        }
        Value::Array(a) => {
            for v in a {
                text(v, items)
            }
        }
        _ => (),
    }
}
fn text_evidence(v: &Value, path: &str, ordinal: Option<u64>, items: &mut Vec<Value>) {
    let ordinal = v.get("sourceOrdinal").and_then(Value::as_u64).or(ordinal);
    match v {
        Value::Object(o) => {
            for (k, v) in o {
                let path = format!("{path}/{k}");
                if (k == "text" || k == "notes") && v.is_string() {
                    items.push(json!({"wire_pointer":path,"source_ordinal":ordinal,"text":v}));
                } else {
                    text_evidence(v, &path, ordinal, items)
                }
            }
        }
        Value::Array(a) => {
            for (i, v) in a.iter().enumerate() {
                text_evidence(v, &format!("{path}/{i}"), ordinal, items)
            }
        }
        _ => (),
    }
}
fn office(data: &[u8], kind: &str, source: &str, out: &Path) -> Value {
    let parts = match package(data) {
        Ok(p) => p,
        Err(e) => return json!({"status":"rejected","reason":e}),
    };
    let (wire, gaps) = if kind == "docx" {
        let options = docx_parse::S9ParseOptions {
            source_ordinals: true,
            ..Default::default()
        };
        match docx_parse::parse_docx_s9_wire_with_limits(data, options, &Default::default()) {
            Ok(v) => (
                serde_json::to_value(v).unwrap(),
                vec![
                    "Reflowed evidence view, not original pagination/layout. Charts, SmartArt, drawings, revisions and unrepresented story/container text are not qualified. Inspect retained original.",
                ],
            ),
            Err(e) => return json!({"status":"rejected","reason":e.to_string()}),
        }
    } else {
        match pptx_parse::parse_pptx(data) {
            Ok(v) => (
                serde_json::to_value(v).unwrap(),
                vec![
                    "Reflowed slide/notes evidence view, not a DrawingML renderer. Spatial dependencies, chart appearance and inherited layout are not qualified. Inspect retained slide preview.",
                ],
            ),
            Err(e) => return json!({"status":"rejected","reason":e.to_string()}),
        }
    };
    let occurrences = occurrences(&parts, source, out);
    let mut texts = vec![];
    let mut evidence = vec![];
    if kind == "docx" {
        for key in [
            "document",
            "headerEntries",
            "footerEntries",
            "footnotes",
            "endnotes",
        ] {
            text(&wire["document"]["package"][key], &mut texts);
            text_evidence(
                &wire["document"]["package"][key],
                &format!("/document/package/{key}"),
                None,
                &mut evidence,
            )
        }
    } else {
        text(&wire["slides"], &mut texts);
        text_evidence(&wire["slides"], "/slides", None, &mut evidence)
    }
    let chart_parts = parts
        .keys()
        .filter(|p| p.contains("/charts/") && p.ends_with(".xml"))
        .cloned()
        .collect::<Vec<_>>();
    texts.retain(|s| !s.is_empty());
    save(
        out,
        &format!("{source}.wire.json"),
        serde_json::to_vec_pretty(&wire).unwrap().as_slice(),
    );
    json!({"status":"partial","source":source,"converter":format!("BetterOffice-{kind}-0.3.0"),"text":texts,"text_evidence":evidence,"occurrences":occurrences,"gaps":gaps,"charts_without_visual_preview":chart_parts,"original_sha256":hash(data)})
}
#[link(name = "sandbox")]
unsafe extern "C" {
    fn sandbox_init(
        profile: *const std::ffi::c_char,
        flags: u64,
        error: *mut *mut std::ffi::c_char,
    ) -> i32;
    fn sandbox_free_error(error: *mut std::ffi::c_char);
    fn fork() -> i32;
}
fn main() {
    let mut args0 = std::env::args().collect::<Vec<_>>();
    if args0.get(1).map(String::as_str) == Some("--sandbox") {
        let profile = std::ffi::CString::new(std::fs::read(&args0[2]).unwrap()).unwrap();
        let mut err = std::ptr::null_mut();
        if unsafe { sandbox_init(profile.as_ptr(), 0, &mut err) } != 0 {
            if !err.is_null() {
                eprintln!(
                    "sandbox_init: {}",
                    unsafe { std::ffi::CStr::from_ptr(err) }.to_string_lossy()
                );
                unsafe { sandbox_free_error(err) }
            }
            std::process::exit(70);
        }
        args0.drain(1..3);
    }
    if args0.get(1).map(String::as_str) == Some("--restriction-check") {
        for path in &args0[2..] {
            println!("read {} {:?}", path, std::fs::read(path).map(|b| b.len()));
        }
        println!(
            "network {:?}",
            std::net::TcpStream::connect("127.0.0.1:49731")
        );
        return;
    }
    if args0.get(1).map(String::as_str) == Some("--cancel-tree") {
        println!("pid={}", std::process::id());
        if unsafe { fork() } == 0 {
            println!("pid={}", std::process::id());
            if unsafe { fork() } == 0 {
                println!("pid={}", std::process::id());
            }
        }
        loop {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
    let args = args0;
    let input = Path::new(&args[1]);
    let out = Path::new(&args[2]);
    std::fs::create_dir_all(out).unwrap();
    let mut data = Vec::new();
    std::fs::File::open(input)
        .unwrap()
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut data)
        .unwrap();
    assert!(data.len() <= 8 * 1024 * 1024, "input cap 8MiB");
    let ext = input.extension().unwrap().to_str().unwrap();
    let source = hash(&data);
    let result = if ext == "eml" {
        let m = MessageParser::default().parse(&data).unwrap();
        let mut children = vec![];
        for (i, p) in m.attachments().enumerate() {
            let bytes = p.contents();
            let id = hash(bytes);
            let name = p.attachment_name().unwrap_or("unnamed");
            save(out, &format!("{id}.original"), bytes);
            let node = format!("{source}-attachment-{i}");
            let extraction = if name.ends_with(".docx") {
                office(bytes, "docx", &node, out)
            } else if p.content_id().is_some() {
                json!({"status":"retained-inline","reason":"CID resource retained; no standalone-image import"})
            } else {
                json!({"status":"unprocessed","reason":"unsupported attachment; original bytes retained"})
            };
            children.push(json!({"parent":source,"node":node,"mime_attachment_index":i,"filename":name,"cid":p.content_id(),"sha256":id,"bytes":bytes.len(),"extraction":extraction}));
        }
        json!({"source":source,"converter":"mail-parser-0.11.8/full_encoding","subject":m.subject(),"date_raw":m.header_raw("Date"),"message_id":m.message_id(),"text":m.body_text(0),"html":m.body_html(0),"children":children})
    } else {
        office(&data, ext, &source, out)
    };
    save(
        out,
        "result.json",
        serde_json::to_vec_pretty(&result).unwrap().as_slice(),
    );
    println!("{}", serde_json::to_string(&result).unwrap());
}
