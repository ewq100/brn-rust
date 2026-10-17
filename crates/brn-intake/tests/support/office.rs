//! Test-only source oracles and comparisons. Never regenerates expectations from a converter.
#![allow(dead_code)] // Shared by separately feature-gated integration targets.
use brn_intake::{prepared::{BlockContent, PreparedDocument}, *};
use serde_json::{Value, json};
use std::{collections::BTreeMap, io::{Cursor, Write}};

pub const DOCX_MIME: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
pub struct Case { pub name: &'static str, pub bytes: Vec<u8>, pub oracle: Value }
pub fn cases() -> Vec<Case> {
    let inputs: [(&str, &[u8], &str); 4] = [
        ("harbor", include_bytes!("../fixtures/harbor.docx"), include_str!("../fixtures/office/harbor.inventory.json")),
        ("features", include_bytes!("../fixtures/features.docx"), include_str!("../fixtures/office/features.inventory.json")),
        ("prefix-variation", include_bytes!("../fixtures/prefix-variation.docx"), include_str!("../fixtures/office/prefix-variation.inventory.json")),
        ("footnotes", include_bytes!("../../../brn-threads-intake/tests/fixtures/luna/footnotes.docx"), include_str!("../fixtures/office/footnotes.inventory.json")),
    ];
    let mut result: Vec<_> = inputs.into_iter().map(|(name, bytes, text)| {
        let oracle: Value = serde_json::from_str(text).unwrap();
        assert_eq!(oracle["source"]["sha256"], hex(&digest(bytes)));
        Case { name, bytes: bytes.to_vec(), oracle }
    }).collect();
    // Source bridge authored in T0 notes. Reuses all 22 existing inventory entries.
    let process: Value = serde_json::from_str(include_str!("../../../brn-threads-intake/tests/fixtures/harbor-process.inventory.json")).unwrap();
    assert_eq!(process["expected_assets"],1);
    assert_eq!(hex(&digest(include_bytes!("../../../brn-threads-intake/tests/fixtures/harbor-process.docx"))),"9ddfe6b3e116b3334513b5048e4c326682da3ec886c87fdbcfe3b311d23a0b87");
    let items = process["items"].as_array().unwrap();
    assert_eq!(items.len(),22);
    let mut blocks = Vec::new();
    for (i, item) in items.iter().enumerate() {
        if (11..15).contains(&i) {
            if i == 11 { blocks.push(json!({"key":"process-table","kind":"table","table":"t1"})); }
            continue;
        }
        if i == 15 { blocks.push(json!({"key":"process-image","kind":"figure","occurrence":"f1"})); }
        let mut text = item["needle"].as_str().unwrap().to_owned();
        let mut b = json!({"key":format!("process-{i}"),"kind":"paragraph","text":text});
        if (6..9).contains(&i) {
            text = text[3..].to_owned();
            b = json!({"key":format!("process-{i}"),"kind":"list_item","text":text,"level":0,"label":format!("{}.",i-5)});
        } else if [1,3,5,10,16,18,20].contains(&i) { b["kind"] = json!("heading"); b["level"] = json!(1); }
        blocks.push(b);
    }
    let values = [["Gate","Owner","Time limit"],["Check","Coordinator","5 min"],["Test","Inspector","3 min"],["Release","Duty lead","2 min"]];
    let cells: Vec<_> = values.iter().enumerate().flat_map(|(r,row)| row.iter().enumerate().map(move |(c,t)| json!({"address":format!("R{}C{}",r+1,c+1),"text":t,"row_span":1,"column_span":1}))).collect();
    result.push(Case { name:"harbor-process", bytes:include_bytes!("../../../brn-threads-intake/tests/fixtures/harbor-process.docx").to_vec(), oracle:json!({
        "stories":[{"key":"body","kind":"body","blocks":blocks}],"tables":[{"key":"t1","cells":cells}],
        "figures":[{"key":"f1","asset":"a1","part":"word/document.xml","paragraph":12,"docpr_id":"1","title":null,"description":null,"caption_item":16,"relationship_id":"rId9","relationship_target":"media/image1.png"}],
        "assets":[{"key":"a1","sha256":"750a38a07b42f0d11cb1a7b1953405b457e105f38af6c15eb70ed66ac90a0c76","png_ihdr_pixels":[720,200]}],
        "links":[],"fields":[],"notes":[],"charts":[]
    })});
    result
}
pub fn array<'a>(v: &'a Value, key: &str) -> &'a [Value] { v[key].as_array().map(Vec::as_slice).unwrap_or(&[]) }
pub fn string(v: &Value, key: &str) -> String { v[key].as_str().unwrap_or("").to_owned() }
pub fn by_key<'a>(v: &'a Value, group: &str, key: &Value) -> &'a Value {
    array(v,group).iter().find(|x| x["key"] == *key).unwrap()
}
pub fn request(bytes: &[u8], limits: IntakeLimits) -> HelperRequest {
    HelperRequest { kind:"docx".into(), bytes:bytes.to_vec(), limits:Some(limits) }
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum Atom {
    Text(String), Cell { address:String, text:String, row_span:u32, column_span:u32 }, Figure(String),
}
pub fn text_of(atom: &Atom) -> Option<String> {
    match atom { Atom::Text(t) | Atom::Cell{text:t,..} => Some(t.clone()), Atom::Figure(_) => None }
}
pub fn source_text(b: &Value, oracle: &Value, labels: &BTreeMap<String,String>, links: bool) -> String {
    if let Some(t) = b["text"].as_str() { return t.to_owned(); }
    array(b,"inlines").iter().map(|i| {
        if let Some(t) = i["text"].as_str() { t.to_owned() }
        else if i.get("field").is_some() { string(by_key(oracle,"fields",&i["field"]),"cached_text") }
        else if i.get("link").is_some() {
            let l = by_key(oracle,"links",&i["link"]);
            if links { format!("{} ({})",string(l,"text"),string(l,"target")) } else { string(l,"text") }
        } else { labels.get(i["note"].as_str().unwrap()).map(|l| format!("[^{l}]")).unwrap_or_default() }
    }).collect()
}
pub fn expected(case: &Case, labels: &BTreeMap<String,String>, figures: bool, links: bool) -> Vec<Atom> {
    let mut atoms = Vec::new();
    for story in array(&case.oracle,"stories") {
        for b in array(story,"blocks") {
            match b["kind"].as_str().unwrap() {
                "native_chart" => (), // Conditional located-gap gate below, never assumed represented.
                "table" => for c in array(by_key(&case.oracle,"tables",&b["table"]),"cells") {
                    atoms.push(Atom::Cell { address:string(c,"address"),text:string(c,"text"),row_span:c["row_span"].as_u64().unwrap() as u32,column_span:c["column_span"].as_u64().unwrap() as u32 });
                },
                "figure" => if figures {
                    let f = by_key(&case.oracle,"figures",&b["occurrence"]);
                    atoms.push(Atom::Figure(string(by_key(&case.oracle,"assets",&f["asset"]),"sha256")));
                },
                _ => {
                    let mut text = source_text(b,&case.oracle,labels,links);
                    if matches!(story["kind"].as_str(),Some("footnote"|"endnote")) { text = text.trim_start_matches(' ').into(); }
                    if !text.is_empty() { atoms.push(Atom::Text(text)); }
                }
            }
        }
    }
    atoms
}
fn expected_stories(case:&Case,labels:&BTreeMap<String,String>,figures:bool,links:bool)->Vec<Vec<Atom>> {
    array(&case.oracle,"stories").iter().map(|story| {
        let mut oracle=case.oracle.clone();oracle["stories"]=json!([story]);
        expected(&Case{name:case.name,bytes:Vec::new(),oracle},labels,figures,links)
    }).filter(|story|!story.is_empty()).collect()
}
fn assert_story_sequences(actual:&[Atom],stories:&[Vec<Atom>],context:&str) {
    // Only a completeness/local-order assertion. Never assign source IDs from text.
    // T0 deliberately specifies no OPC-global order between independent stories.
    let mut covered=vec![false;actual.len()];
    for story in stories {
        let found=(0..=actual.len().saturating_sub(story.len())).find(|&start| {
            actual.get(start..start+story.len())==Some(story.as_slice()) &&
                covered[start..start+story.len()].iter().all(|used|!*used)
        });
        let start=found.unwrap_or_else(||panic!("{context}: complete ordered story missing: {story:?}; actual={actual:?}"));
        covered[start..start+story.len()].fill(true);
    }
    assert!(covered.iter().all(|used|*used),"{context}: unexpected/duplicated substantive atoms: {actual:?}");
}
pub fn prepared_atoms(doc: &PreparedDocument) -> Vec<Atom> {
    let mut out = Vec::new();
    for b in &doc.blocks {
        match &b.content {
            BlockContent::Paragraph{text} | BlockContent::Heading{text,..} | BlockContent::ListItem{text,..} => out.push(Atom::Text(text.clone())),
            BlockContent::Footnote{text,..} => out.push(Atom::Text(text.trim_start_matches(' ').into())),
            BlockContent::Table{rows} => for c in rows.iter().flatten() { out.push(Atom::Cell{address:c.address.clone(),text:c.text.clone(),row_span:c.row_span,column_span:c.column_span}); },
            BlockContent::Figure{asset_id,..} => out.push(Atom::Figure(hex(&doc.assets.iter().find(|a| a.id==*asset_id).unwrap().sha256))),
            BlockContent::Metadata{..} => (),
            BlockContent::PageText{..} => panic!("DOCX unexpectedly produced page text"),
        }
    }
    out
}
// Interpret emitted Markdown only for complete wording comparison, never provenance joins.
fn md_text(node: &markdown::mdast::Node) -> String {
    use markdown::mdast::Node;
    match node {
        Node::Text(t) => t.value.clone(), Node::InlineCode(t) => t.value.clone(),
        Node::FootnoteReference(_) | Node::Image(_) => String::new(),
        _ => node.children().map(|c| c.iter().map(md_text).collect()).unwrap_or_default(),
    }
}
pub fn markdown_wording(text: &str) -> Vec<String> {
    use markdown::mdast::Node;
    fn html_cells(handle: &markup5ever_rcdom::Handle, out: &mut Vec<String>) {
        use markup5ever_rcdom::NodeData;
        fn text(h: &markup5ever_rcdom::Handle) -> String {
            match &h.data { NodeData::Text{contents} => contents.borrow().to_string(), _ => h.children.borrow().iter().map(text).collect() }
        }
        if let NodeData::Element{name,..} = &handle.data {
            if matches!(name.local.as_ref(),"td"|"th") { out.push(text(handle).trim().into()); return; }
        }
        for child in handle.children.borrow().iter() { html_cells(child,out); }
    }
    fn walk(n: &Node, out: &mut Vec<String>) {
        match n {
            Node::Paragraph(_) | Node::Heading(_) | Node::TableCell(_) => { let t=md_text(n); if !t.is_empty() { out.push(t); } },
            Node::Html(html) => {
                use html5ever::tendril::TendrilSink;
                let dom = html5ever::parse_document(markup5ever_rcdom::RcDom::default(),Default::default()).one(html.value.clone());
                html_cells(&dom.document,out);
            },
            _ => if let Some(c)=n.children() { for x in c { walk(x,out); } },
        }
    }
    let ast = markdown::to_mdast(text,&markdown::ParseOptions::gfm()).unwrap();
    let mut out=Vec::new(); walk(&ast,&mut out); out
}
pub fn check_current(case: &Case) {
    let e = helper::extract(request(&case.bytes,IntakeLimits::default())).unwrap(); e.validate().unwrap();
    let d = helper::prepared::prepare(request(&case.bytes,IntakeLimits::default())).unwrap(); d.validate().unwrap();
    assert_eq!(e.original_sha256,digest(&case.bytes)); assert_eq!(d.source_sha256,e.original_sha256);
    assert_eq!(e.sources[0].bytes,case.bytes); assert_eq!(e.sources[0].text,e.markdown);
    let mut labels = BTreeMap::new();
    let definitions:Vec<_> = d.blocks.iter().filter_map(|b| if let BlockContent::Footnote{label,text}=&b.content {Some((label,text))}else{None}).collect();
    assert_eq!(definitions.len(),array(&case.oracle,"notes").len());
    for n in array(&case.oracle,"notes") {
        let kind=if n["kind"]=="endnote" {"en-"} else {"fn-"};
        let suffix=hex(&digest(string(n,"source_id").as_bytes()));
        let matching:Vec<_>=definitions.iter().filter(|(label,_)| label.starts_with(kind) && label.ends_with(&suffix[..16])).collect();
        assert_eq!(matching.len(),1,"distinct typed source-note identity");
        let (label,text)=matching[0];
        let story=array(&case.oracle,"stories").iter().find(|s|s["key"]==n["definition_story"]).unwrap();
        assert_eq!(text.trim_start_matches(' '),string(&story["blocks"][0],"text").trim_start_matches(' '));
        labels.insert(string(n,"key"),(*label).clone());
    }
    let has_images = e.occurrences.len()==array(&case.oracle,"figures").len();
    if !has_images {
        assert!(e.occurrences.is_empty(),"partially missing occurrences: {}",case.name);
        assert!(e.gaps.iter().any(|g|g.contains("unresolved")||g.contains("ambiguous")));
        assert!(d.gaps.iter().any(|g|!g.locator.is_empty() && g.detail.contains("Image")));
        println!("CURRENT_PARTIAL {}: {} independently expected figures unavailable; gaps={:?}",case.name,array(&case.oracle,"figures").len(),e.gaps);
    }
    assert_eq!(e.assets.len(),if has_images {array(&case.oracle,"assets").len()} else {0});
    assert_eq!(d.assets.len(),e.assets.len());
    record("S1",&format!("current-{}-occurrences",case.name),json!({"figures":case.oracle["figures"],"assets":case.oracle["assets"]}),json!({"occurrences":e.occurrences,"gaps":e.gaps,"prepared_figures":d.blocks.iter().filter(|b|matches!(b.content,BlockContent::Figure{..})).collect::<Vec<_>>()}));
    assert_story_sequences(&prepared_atoms(&d),&expected_stories(case,&labels,has_images,true),&format!("{} schema2",case.name));
    let wanted:Vec<Vec<Atom>>=expected_stories(case,&BTreeMap::new(),false,false).iter().map(|story|story.iter().filter_map(text_of).map(Atom::Text).collect()).collect();
    let actual:Vec<_>=markdown_wording(&e.markdown).into_iter().filter(|t|!t.starts_with("[image unavailable;")).map(Atom::Text).collect();
    assert_story_sequences(&actual,&wanted,&format!("{} schema1 complete wording",case.name));
    let wanted_roles: Vec<_> = array(&case.oracle,"stories").iter().flat_map(|story| {
        array(story,"blocks").iter().filter_map(|b| {
            let text=source_text(b,&case.oracle,&labels,true);
            match b["kind"].as_str() {
                Some("heading") => Some(json!({"kind":"heading","text":text,"level":b["level"]})),
                Some("list_item") => Some(json!({"kind":"list_item","text":text,"level":b["level"],"label":b["label"]})),
                _ => None,
            }
        })
    }).collect();
    // Title style has no invented depth; compare explicitly authored headings/lists in order.
    let role_texts: Vec<_> = wanted_roles.iter().map(|v|v["text"].as_str().unwrap()).collect();
    let actual_roles: Vec<_> = d.blocks.iter().filter_map(|b| match &b.content {
        BlockContent::Heading{text,level} if role_texts.contains(&text.as_str()) => Some(json!({"kind":"heading","text":text,"level":level})),
        BlockContent::ListItem{text,level,label} => Some(json!({"kind":"list_item","text":text,"level":level,"label":label})),
        _ => None,
    }).collect();
    assert_eq!(actual_roles,wanted_roles,"{} complete authored heading/list roles",case.name);
    let wanted_links: Vec<_> = array(&case.oracle,"links").iter().map(|l|json!({"text":l["text"],"target":l["target"]})).collect();
    let ast=markdown::to_mdast(&e.markdown,&markdown::ParseOptions::gfm()).unwrap();
    fn links(n:&markdown::mdast::Node,out:&mut Vec<Value>) {
        if let markdown::mdast::Node::Link(l)=n {out.push(json!({"text":md_text(n),"target":l.url}));}
        if let Some(children)=n.children() {for child in children {links(child,out);}}
    }
    let mut actual_links=Vec::new();links(&ast,&mut actual_links);
    assert_eq!(actual_links,wanted_links,"{} exact visible link wording and destination",case.name);
    record("S6",&format!("current-{}-full-context",case.name),json!({"stories":case.oracle["stories"],"notes":case.oracle["notes"],"fields":case.oracle["fields"],"conditional_gaps":case.oracle["conditional_gaps"]}),json!({"blocks":d.blocks,"gaps":d.gaps,"sources":e.sources.iter().map(|s|json!({"id":s.id,"parent":s.parent,"locator":s.locator,"status":s.status,"media_type":s.media_type})).collect::<Vec<_>>()}));
    for (f,o) in array(&case.oracle,"figures").iter().zip(&e.occurrences) {
        let a=e.assets.iter().find(|a|a.id==o.asset_id).unwrap(); let wanted=by_key(&case.oracle,"assets",&f["asset"]);
        assert_eq!(hex(&a.sha256),string(wanted,"sha256")); assert_eq!(a.sha256,digest(&a.bytes));
        assert_eq!([a.width,a.height],[wanted["png_ihdr_pixels"][0].as_u64().unwrap() as u32,wanted["png_ihdr_pixels"][1].as_u64().unwrap() as u32]);
        assert_eq!(o.source_id,e.sources[0].id); assert!(o.locator.contains(&string(f,"relationship_id"))); assert!(o.locator.contains(&string(f,"relationship_target")));
        assert!(e.markdown[o.start..o.end].contains(&asset_file_name(a).unwrap()));
    }
    for pair in e.occurrences.windows(2) { assert!(pair[0].end<=pair[1].start); assert_ne!(pair[0].locator,pair[1].locator); }
    for c in array(&case.oracle,"charts") { assert!(e.gaps.iter().any(|g|g.contains(&string(c,"part"))),"{} chart omission must be located",case.name); }
    if !array(&case.oracle,"fields").is_empty() { assert!(d.gaps.iter().any(|g|g.detail.contains("NOTEREF")&&!g.locator.is_empty())); }
}

const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
pub fn paragraph(text: &str) -> String { format!("<w:p><w:r><w:t>{text}</w:t></w:r></w:p>") }
pub fn package(body: &str, rels: &str, extras: &[(&str,&str)]) -> Vec<u8> {
    let mut parts: Vec<(String, Vec<u8>)> = vec![
        ("[Content_Types].xml".into(),br#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/><Default Extension="png" ContentType="image/png"/><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#.to_vec()),
        ("_rels/.rels".into(),br#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="main" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#.to_vec()),
        ("word/document.xml".into(),format!("<w:document xmlns:w=\"{W}\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:w14=\"http://schemas.microsoft.com/office/word/2010/wordml\"><w:body>{body}</w:body></w:document>").into_bytes()),
        ("word/_rels/document.xml.rels".into(),format!("<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">{rels}</Relationships>").into_bytes()),
    ];
    // Known fixture parts receive their OPC content types; no custom format registry.
    let mut types=String::from_utf8(parts[0].1.clone()).unwrap();
    for (name,_) in extras {
        let kind=if name.ends_with("commentsExtended.xml") {Some("application/vnd.ms-word.commentsExtended+xml".to_owned())}
            else if name.ends_with("footnotes.xml") {Some("application/vnd.openxmlformats-officedocument.wordprocessingml.footnotes+xml".to_owned())}
            else if name.ends_with("endnotes.xml") {Some("application/vnd.openxmlformats-officedocument.wordprocessingml.endnotes+xml".to_owned())}
            else if name.ends_with("comments.xml") {Some("application/vnd.openxmlformats-officedocument.wordprocessingml.comments+xml".to_owned())}
            else if name.contains("header") {Some("application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml".to_owned())} else {None};
        if let Some(kind)=kind {types=types.replace("</Types>",&format!("<Override PartName=\"/{name}\" ContentType=\"{kind}\"/></Types>"));}
    }
    parts[0].1=types.into_bytes();
    parts.extend(extras.iter().map(|(n,t)|(n.to_string(),t.as_bytes().to_vec())));
    ooxml_opc::rezip_parts(&parts).unwrap()
}
pub fn relationship(id: &str, kind: &str, target: &str) -> String {
    format!("<Relationship Id=\"{id}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/{kind}\" Target=\"{target}\"/>")
}
pub fn stored(bytes: &[u8], extra_empty: bool) -> Vec<u8> {
    let mut parts=ooxml_opc::unzip_parts(bytes).unwrap();
    if extra_empty { parts.push(("opaque/empty.bin".into(),vec![])); }
    zip_parts(parts)
}
pub fn zip_parts(parts:Vec<(String,Vec<u8>)>)->Vec<u8> {
    let mut writer=zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (n,b) in parts { writer.start_file(n,zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored)).unwrap(); writer.write_all(&b).unwrap(); }
    writer.finish().unwrap().into_inner()
}
fn u16_at(b:&[u8],i:usize)->usize {u16::from_le_bytes(b[i..i+2].try_into().unwrap()) as usize}
fn u32_at(b:&[u8],i:usize)->usize {u32::from_le_bytes(b[i..i+4].try_into().unwrap()) as usize}
fn set32(b:&mut[u8],i:usize,v:usize) {b[i..i+4].copy_from_slice(&u32::try_from(v).unwrap().to_le_bytes());}
fn central(bytes:&[u8],name:&str)->(usize,usize) {
    let e=bytes.windows(4).rposition(|w|w==b"PK\x05\x06").unwrap(); let mut i=u32_at(bytes,e+16);
    while i<e { assert_eq!(&bytes[i..i+4],b"PK\x01\x02"); let n=u16_at(bytes,i+28); if &bytes[i+46..i+46+n]==name.as_bytes(){return(i,e)} i+=46+n+u16_at(bytes,i+30)+u16_at(bytes,i+32); }
    panic!("member absent")
}
/// Mutates only a known small fixture's central record, preserving CRC/payload.
pub fn size_lie(bytes:&[u8],name:&str,delta:i32,zero_compressed:bool)->Vec<u8> {
    let mut b=stored(bytes,false);let(i,_)=central(&b,name);
    let size=u32_at(&b,i+24);set32(&mut b,i+24,size.checked_add_signed(delta as isize).unwrap());
    if zero_compressed {set32(&mut b,i+20,0);} b
}
/// Valid signed data descriptor: central sizes stay true, local CRC/sizes zero.
pub fn descriptor(bytes:&[u8],name:&str)->Vec<u8> {
    let mut b=stored(bytes,false);let(i,e)=central(&b,name);let local=u32_at(&b,i+42);
    let insert=local+30+u16_at(&b,local+26)+u16_at(&b,local+28)+u32_at(&b,i+20);
    let mut desc=b"PK\x07\x08".to_vec();desc.extend_from_slice(&b[i+16..i+28]);
    b[local+6]|=8;b[local+14..local+26].fill(0);let start=u32_at(&b,e+16);b[i+8]|=8;
    let mut j=start;while j<e {let offset=u32_at(&b,j+42);if offset>=insert {set32(&mut b,j+42,offset+16);}j+=46+u16_at(&b,j+28)+u16_at(&b,j+30)+u16_at(&b,j+32);}
    set32(&mut b,e+16,start+16);b.splice(insert..insert,desc);b
}
#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum EvidenceClass { S1, S2, S3, S4, S6, S7 }
#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum Measurement { EvidenceOnly, EqualProjection, DifferentProjection }
#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Observation {
    pub class:EvidenceClass, pub witness:String, pub measurement:Measurement,
    pub expected:Value, pub actual:Value,
}
pub fn record(class:&str,witness:&str,expected:Value,actual:Value) {
    measured(class,witness,expected,actual,Measurement::EvidenceOnly);
}
pub fn compare(class:&str,witness:&str,expected:Value,actual:Value) {
    let result=if expected==actual {Measurement::EqualProjection} else {Measurement::DifferentProjection};
    measured(class,witness,expected,actual,result);
}
fn measured(class:&str,witness:&str,expected:Value,actual:Value,measurement:Measurement) {
    let typed=match class {
        "S1"=>EvidenceClass::S1,"S2"=>EvidenceClass::S2,"S3"=>EvidenceClass::S3,
        "S4"=>EvidenceClass::S4,"S6"=>EvidenceClass::S6,"S7"=>EvidenceClass::S7,
        _=>panic!("unallocated class"),
    };
    let observed=Observation{class:typed,witness:witness.into(),measurement,expected,actual};
    let registry:Value=serde_json::from_str(include_str!("../fixtures/office/parity-registry.json")).unwrap();
    assert!(registry["classes"][class].is_object());
    let pin=&registry["observations"][witness];
    if pin.is_null() {println!("O1_UNPINNED_OBSERVATION {}",serde_json::to_string(&observed).unwrap());}
    else {assert_eq!(serde_json::to_value(&observed).unwrap(),*pin);}
    // Equal projected atoms still cannot qualify missing provenance or resource gates.
}

pub fn revisions_comments() -> Vec<u8> {
    let body = concat!(
        "<w:p><w:r><w:t xml:space=\"preserve\">Keep </w:t></w:r><w:ins w:id=\"1\" w:author=\"Iris\" w:date=\"2026-10-18T10:00:00Z\"><w:r><w:t>accepted</w:t></w:r></w:ins>",
        "<w:del w:id=\"2\" w:author=\"Iris\"><w:r><w:delText>removed</w:delText></w:r></w:del><w:r><w:t>.</w:t></w:r></w:p>",
        "<w:p w14:paraId=\"00000001\"><w:commentRangeStart w:id=\"7\"/><w:r><w:t>Reviewed.</w:t></w:r><w:commentRangeEnd w:id=\"7\"/><w:r><w:commentReference w:id=\"7\"/></w:r></w:p>"
    );
    let comments = format!("<w:comments xmlns:w=\"{W}\" xmlns:w14=\"http://schemas.microsoft.com/office/word/2010/wordml\"><w:comment w:id=\"7\" w:author=\"Iris\" w:date=\"2026-10-18T10:00:00Z\"><w:p w14:paraId=\"000000A1\"><w:r><w:t>Reviewer note.</w:t></w:r></w:p></w:comment><w:comment w:id=\"8\" w:author=\"Mara\" w:date=\"2026-10-18T11:00:00Z\"><w:p w14:paraId=\"000000A2\"><w:r><w:t>Reply note.</w:t></w:r></w:p></w:comment></w:comments>");
    let ext = "<w15:commentsEx xmlns:w15=\"http://schemas.microsoft.com/office/word/2012/wordml\"><w15:commentEx w15:paraId=\"000000A1\" w15:done=\"0\"/><w15:commentEx w15:paraId=\"000000A2\" w15:paraIdParent=\"000000A1\" w15:done=\"1\"/></w15:commentsEx>";
    let rels=format!("{}<Relationship Id=\"commentsEx\" Type=\"http://schemas.microsoft.com/office/2011/relationships/commentsExtended\" Target=\"commentsExtended.xml\"/>",relationship("comments","comments","comments.xml"));
    package(body,&rels,&[("word/comments.xml",&comments),("word/commentsExtended.xml",ext)])
}
pub fn note_collisions() -> Vec<u8> {
    let body=concat!(
        "<w:p><w:r><w:t xml:space=\"preserve\">Literal [^1] and [cmt:7]. </w:t></w:r><w:bookmarkStart w:id=\"1\" w:name=\"rule\"/><w:r><w:footnoteReference w:id=\"1\"/></w:r><w:bookmarkEnd w:id=\"1\"/><w:r><w:endnoteReference w:id=\"1\"/></w:r></w:p>",
        "<w:p><w:fldSimple w:instr=\" NOTEREF rule \\h \"><w:r><w:t>1</w:t></w:r></w:fldSimple></w:p>",
        "<w:tbl><w:tblGrid><w:gridCol w:w=\"2000\"/></w:tblGrid><w:tr><w:tc><w:p><w:r><w:t>Cell</w:t><w:footnoteReference w:id=\"2\"/></w:r></w:p></w:tc></w:tr></w:tbl>"
    );
    let foot=format!("<w:footnotes xmlns:w=\"{W}\"><w:footnote w:id=\"1\">{}</w:footnote><w:footnote w:id=\"2\">{}</w:footnote></w:footnotes>",paragraph("Footnote one."),paragraph("Cell note."));
    let end=format!("<w:endnotes xmlns:w=\"{W}\"><w:endnote w:id=\"1\">{}</w:endnote></w:endnotes>",paragraph("Endnote one."));
    package(body,&format!("{}{}",relationship("fn","footnotes","footnotes.xml"),relationship("en","endnotes","endnotes.xml")),&[("word/footnotes.xml",&foot),("word/endnotes.xml",&end)])
}
pub fn empty_orphan() -> Vec<u8> {
    let header=format!("<w:hdr xmlns:w=\"{W}\">{}</w:hdr>",paragraph("Bound header."));
    let orphan=format!("<w:hdr xmlns:w=\"{W}\">{}</w:hdr>",paragraph("Orphan header."));
    let body=format!("<w:p/>{}<w:sectPr><w:headerReference w:type=\"default\" r:id=\"hdr\"/></w:sectPr>",paragraph("Visible."));
    package(&body,&relationship("hdr","header","header1.xml"),&[("word/header1.xml",&header),("word/header42.xml",&orphan)])
}
pub fn grid_source() -> Value {
    json!({"top_level_tables":[
        {"rows":[
            [{"address":"R1C1","text":"Wide","row_span":1,"column_span":2}],
            [{"address":"R2C1","text":"A"},{"address":"R2C2","text":"B"}],
            [{"address":"R3C1","blocks":["Outer",{"nested_table":[["Inner"]]},""]},{"address":"R3C2","text":"Extra"}]
        ]},
        {"rows":[[{"address":"R1C1","text":"C"},{"address":"R1C2","text":"D"}]]},
        {"rows":[
            [{"address":"R1C1","text":"Vertical","row_span":2,"column_span":1},{"address":"R1C2","text":"Right1"}],
            [{"address":"R2C1","vertical_merge_continuation":true,"text":""},{"address":"R2C2","text":"Right2"}]
        ]}
    ],"unspecified_origin_spans":[1,1],"nested_grid_count":1,"projection":"Source cells and nested blocks; no current-reader flattening assumed"})
}
pub fn grids() -> Vec<u8> {
    let grid="<w:tblGrid><w:gridCol w:w=\"1000\"/><w:gridCol w:w=\"1000\"/></w:tblGrid>";
    let cell=|text:&str|format!("<w:tc>{}</w:tc>",paragraph(text));
    let body=format!("<w:tbl>{grid}<w:tr><w:tc><w:tcPr><w:gridSpan w:val=\"2\"/></w:tcPr>{}</w:tc></w:tr><w:tr>{}{}</w:tr><w:tr><w:tc>{}<w:tbl><w:tblGrid><w:gridCol w:w=\"1000\"/></w:tblGrid><w:tr>{}</w:tr></w:tbl><w:p/></w:tc>{}</w:tr></w:tbl><w:tbl>{grid}<w:tr>{}{}</w:tr></w:tbl><w:tbl>{grid}<w:tr><w:tc><w:tcPr><w:vMerge w:val=\"restart\"/></w:tcPr>{}</w:tc>{}</w:tr><w:tr><w:tc><w:tcPr><w:vMerge/></w:tcPr><w:p/></w:tc>{}</w:tr></w:tbl>",paragraph("Wide"),cell("A"),cell("B"),paragraph("Outer"),cell("Inner"),cell("Extra"),cell("C"),cell("D"),paragraph("Vertical"),cell("Right1"),cell("Right2"));
    package(&body,"",&[])
}
pub fn repeated_figures() -> Vec<u8> {
    let image=|id:u32|format!("<w:p><w:r><w:drawing><wp:inline xmlns:wp=\"http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing\" xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:pic=\"http://schemas.openxmlformats.org/drawingml/2006/picture\"><wp:extent cx=\"900000\" cy=\"500000\"/><wp:docPr id=\"{id}\" name=\"repeat-{id}\" descr=\"Same description\"/><a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/picture\"><pic:pic><pic:nvPicPr><pic:cNvPr id=\"0\" name=\"same.png\"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed=\"image\"/></pic:blipFill><pic:spPr/></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p>");
    let b=package(&format!("{}{}{}{}",paragraph("Same caption."),image(31),paragraph("Same caption."),image(32)),&relationship("image","image","media/same.png"),&[]);
    let mut parts=ooxml_opc::unzip_parts(&b).unwrap();parts.push(("word/media/same.png".into(),include_bytes!("../fixtures/water-use.png").to_vec()));ooxml_opc::rezip_parts(&parts).unwrap()
}
pub fn warning_document() -> Vec<u8> {
    package("<w:p><w:hyperlink w:anchor=\"missing\"><w:r><w:t>Bookmark label.</w:t></w:r></w:hyperlink><w:r><w:commentReference w:id=\"99\"/></w:r></w:p>","",&[])
}
