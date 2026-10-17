#![cfg(feature = "office-differential")]
#[path = "support/office.rs"]
mod office;
use brn_intake::{helper, *};
use office::*;
use serde_json::{Value, json};
use std::{collections::BTreeMap, future::Future, pin::pin, sync::Arc, task::{Context, Poll, Wake, Waker}, time::Duration};
use xberg::types::{document_structure::{ContentLayer, NodeContent}, extraction::ExtractedDocument};

// Candidate allowances are captured before current helper admission charges this package.
// E+P iteration, XML acceptance and repeated-pass equivalence remain hypotheses.
fn candidate(limits:&IntakeLimits, prior_bytes:usize, prior_parts:usize)->xberg::SecurityLimits {
    let e=limits.max_expanded_bytes.checked_sub(prior_bytes).unwrap();
    let p=limits.max_package_parts.checked_sub(prior_parts).unwrap();
    xberg::SecurityLimits {max_archive_size:e,max_compression_ratio:e,max_files_in_archive:p,max_nesting_depth:128,max_entity_length:e,max_content_size:e,max_iterations:e.checked_add(p).unwrap(),max_xml_depth:128,max_table_cells:e,max_pages:Some(p)}
}
fn config(limits:xberg::SecurityLimits)->xberg::ExtractionConfig {
    xberg::ExtractionConfig {
        use_cache:false,cache_namespace:None,enable_quality_processing:false,extraction_timeout_secs:None,
        max_concurrent_extractions:Some(1),content_filter:None,output_format:xberg::OutputFormat::Markdown,
        concurrency:Some(xberg::ConcurrencyConfig{max_threads:Some(1),max_concurrent_ocr:Some(1)}),
        postprocessor:Some(xberg::PostProcessorConfig{enabled:false,..Default::default()}),
        disable_ocr:true,ocr:None,force_ocr:false,force_ocr_pages:None,images:None,captioning:None,qr_codes:Some(false),
        chunking:None,language_detection:None,token_reduction:None,pages:None,include_document_structure:true,
        max_archive_depth:0,security_limits:Some(limits),
        url:xberg::UrlExtractionConfig{allow_local_file_inputs:false,allow_file_uris:false,..Default::default()},
        ..Default::default()
    }
}
fn run(bytes:&[u8], limits:xberg::SecurityLimits)->Result<ExtractedDocument,String> {
    struct Unpark(std::thread::Thread);
    impl Wake for Unpark {fn wake(self:Arc<Self>){self.0.unpark();} fn wake_by_ref(self:&Arc<Self>){self.0.unpark();}}
    let cfg=config(limits);
    assert!(!cfg.needs_image_data());assert_eq!(cfg.max_archive_depth,0);
    let mut future=pin!(xberg::extract(xberg::ExtractInput::from_bytes(bytes.to_vec(),DOCX_MIME,None),&cfg));
    let waker=Waker::from(Arc::new(Unpark(std::thread::current())));let mut cx=Context::from_waker(&waker);
    for _ in 0..32 {
        match future.as_mut().poll(&mut cx) {
            Poll::Pending=>std::thread::park_timeout(Duration::from_millis(50)),
            Poll::Ready(result)=>{
                let mut result=result.map_err(|e|e.to_string())?;
                if !result.errors.is_empty() || result.results.len()!=1 {return Err(format!("Xberg envelope {:?}; documents={}",result.errors,result.results.len()));}
                let doc=result.results.remove(0);assert_eq!(doc.mime_type,DOCX_MIME);
                assert!(doc.children.as_ref().is_none_or(Vec::is_empty));
                assert!(doc.images.as_ref().is_none_or(|images|images.iter().all(|i|i.data.is_empty())));
                return Ok(doc);
            }
        }
    }
    panic!("test executor ceiling reached: infrastructure failure, never an admission refusal")
}
fn extract(bytes:&[u8])->ExtractedDocument {run(bytes,candidate(&IntakeLimits::default(),0,0)).unwrap()}
fn atoms(doc:&ExtractedDocument,body_only:bool)->Vec<Atom> {
    let mut out=Vec::new();
    for node in &doc.document.as_ref().expect("structure requested").nodes {
        if body_only && node.content_layer!=ContentLayer::Body {continue;}
        match &node.content {
            NodeContent::Title{text}|NodeContent::Heading{text,..}|NodeContent::Paragraph{text}|NodeContent::ListItem{text}|NodeContent::Footnote{text}|NodeContent::Comment{text}=>out.push(Atom::Text(text.clone())),
            NodeContent::Table{grid}=>for c in &grid.cells {out.push(Atom::Cell{address:format!("R{}C{}",c.row+1,c.col+1),text:c.content.clone(),row_span:c.row_span,column_span:c.col_span});},
            _=>(),
        }
    }
    out
}
fn expected_body(case:&Case)->Vec<Atom> {
    let mut oracle=case.oracle.clone();oracle["stories"]=json!([oracle["stories"][0].clone()]);
    expected(&Case{name:case.name,bytes:vec![],oracle},&BTreeMap::new(),false,false)
}
#[derive(serde::Serialize)]
enum WarningPolicy { MissingReferenceGap, MissingCommentGap, SymbolLossGap, FieldLossGap, StoryLossGap, UnmappedPromotionGate }
fn warning_classes(doc:&ExtractedDocument)->Vec<Value> {
    doc.processing_warnings.iter().map(|w| {
        let message=w.message.as_ref();
        let policy=match w.source.as_ref() {
            "relationships" if message.contains(" cross-reference target(s) could not be resolved and were dropped from the document structure: ") => WarningPolicy::MissingReferenceGap,
            "docx" if message.starts_with("Comment reference id ") && message.ends_with(" has no matching entry in comments.xml; the comment text was dropped") => WarningPolicy::MissingCommentGap,
            "docx" if message.starts_with("Could not map w:sym character code ") && message.ends_with("; inserted a placeholder") => WarningPolicy::SymbolLossGap,
            "docx" if message.starts_with("A DOCX field instruction (w:fldChar begin) was never closed with a matching separate/end; ") => WarningPolicy::FieldLossGap,
            "docx" if (message.starts_with("Header relationship target '") || message.starts_with("Footer relationship target '")) && message.ends_with(" was dropped") => WarningPolicy::StoryLossGap,
            _ => WarningPolicy::UnmappedPromotionGate,
        };
        json!({"source":w.source,"message":w.message,"candidate_current_policy":policy,"location":"only supplied source/message; no guessed source range"})
    }).collect()
}
fn witness(doc:&ExtractedDocument)->Value {
    json!({"document":doc.document,"revisions":doc.revisions,"warnings":doc.processing_warnings,"warning_classes":warning_classes(doc),"images":doc.images,"metadata":doc.metadata})
}
#[test]
fn office_xberg_docx_full_independent_inventory_and_prefix() {
    for case in cases() {
        let doc=extract(&case.bytes);
        compare(if case.name=="prefix-variation"{"S7"}else{"S6"},&format!("{}-body",case.name),json!(expected_body(&case)),json!(atoms(&doc,true)));
        record("S6",&format!("{}-all-stories",case.name),json!({"atoms":expected(&case,&BTreeMap::new(),false,false),"stories":case.oracle["stories"],"conditional_gaps":case.oracle["conditional_gaps"]}),witness(&doc));
        // Image indices and paths are public evidence; no joins recovered from Markdown.
        let wanted:Vec<_>=array(&case.oracle,"figures").iter().enumerate().map(|(i,f)|json!({"image_index":i,"relationship_target":f["relationship_target"],"source_rid":f["relationship_id"],"content_range_required":true})).collect();
        let actual:Vec<_>=doc.document.as_ref().unwrap().nodes.iter().filter_map(|n|match &n.content {
            NodeContent::Image{image_index,description,src}=>Some(json!({"image_index":image_index,"description":description,"src":src,"attributes":n.attributes,"content_range":null})),_=>None
        }).collect();
        record("S1",&format!("{}-images",case.name),json!(wanted),json!(actual));
        if !array(&case.oracle,"tables").is_empty() {
            let wanted:Vec<_>=expected(&case,&BTreeMap::new(),false,false).into_iter().filter(|x|matches!(x,Atom::Cell{..})).collect();
            let actual:Vec<_>=atoms(&doc,false).into_iter().filter(|x|matches!(x,Atom::Cell{..})).collect();
            compare("S3",&format!("{}-cells",case.name),json!(wanted),json!(actual));
        }
        if !array(&case.oracle,"notes").is_empty() {
            record("S4","footnotes-typed-associations",json!({"notes":case.oracle["notes"],"fields":case.oracle["fields"],"bookmarks":case.oracle["bookmarks"]}),witness(&doc));
        }
    }
}
#[test]
fn office_xberg_docx_authored_revisions_comments_notes_and_warnings() {
    let bytes=revisions_comments();let doc=extract(&bytes);
    record("S2","xberg-comments",json!({"comments":[{"id":"7","author":"Iris","date":"2026-10-18T10:00:00Z","range_text":"Reviewed."},{"id":"8","author":"Mara","date":"2026-10-18T11:00:00Z","parent":"7","resolved":true}],"accepted_body":["Keep accepted.","Reviewed."]}),witness(&doc));
    record("S6","xberg-revisions",json!({"accepted":"Keep accepted.","inserted":{"text":"accepted","author":"Iris","date":"2026-10-18T10:00:00Z"},"deleted":"removed"}),json!({"body":atoms(&doc,true),"revisions":doc.revisions}));
    let notes=extract(&note_collisions());
    record("S4","xberg-note-collisions",json!({"literal":"Literal [^1] and [cmt:7]. ","typed_refs":["footnote:1","endnote:1","table/footnote:2"],"definitions":["Footnote one.","Cell note.","Endnote one."],"field":{"kind":"NOTEREF","bookmark":"rule","cached":"1"}}),witness(&notes));
    let escaped=package(&paragraph("Escape target."),&relationship("h","header","../../outside.xml"),&[]);
    record("S6","xberg-escaping-relationship",json!({"target":"../../outside.xml","policy":"no filesystem resolution; measure existing gap/refusal before promotion"}),json!({"current":helper::extract(request(&escaped,IntakeLimits::default())).map(|e|json!({"wording":markdown_wording(&e.markdown),"gaps":e.gaps})),"xberg":run(&escaped,candidate(&IntakeLimits::default(),0,0)).map(|d|witness(&d))}));
    let warnings=extract(&warning_document());
    record("S6","xberg-warning-policy",json!({"visible_text":"Bookmark label.","missing_bookmark":"located gap, ordinary partial input","missing_comment":"located missing-reference gap","unknown_warning":"promotion gate, no generic production refusal"}),json!({"body":atoms(&warnings,true),"warnings":warnings.processing_warnings,"classes":warning_classes(&warnings)}));
}
#[test]
fn office_xberg_docx_orphan_multitable_spans_and_repeated_figures() {
    let orphan=extract(&empty_orphan());
    record("S6","xberg-empty-orphan",json!({"body":["","Visible."],"related_header":"Bound header.","orphan_header":"Orphan header."}),witness(&orphan));
    let grids=extract(&grids());
    record("S3","xberg-grids",grid_source(),witness(&grids));
    let images=extract(&repeated_figures());
    let nodes:Vec<_>=images.document.as_ref().unwrap().nodes.iter().filter_map(|n|match &n.content {NodeContent::Image{image_index,..}=>Some(json!({"index":image_index,"attributes":n.attributes})),_=>None}).collect();
    assert_eq!(nodes.len(),2,"two independently authored drawing occurrences");assert_ne!(nodes[0]["index"],nodes[1]["index"]);
    record("S1","xberg-repeated-images",json!({"source_docpr_ids":[31,32],"relationship":"image","target":"media/same.png","caption_order":["Same caption.","Same caption."],"content_ranges_required":2}),json!({"nodes":nodes,"body":atoms(&images,true),"images":images.images}));
}
fn boundary_package()->Vec<u8> {
    let notes="<w:footnotes xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:footnote w:id=\"1\"><w:p><w:r><w:t>Optional note.</w:t></w:r></w:p></w:footnote></w:footnotes>";
    package(&paragraph("Boundary."),&relationship("fn","footnotes","footnotes.xml"),&[("word/footnotes.xml",notes)])
}
#[test]
fn office_limits_pre_admission_allowances_and_actual_iteration_boundary() {
    let bytes=boundary_package();let parts=ooxml_opc::unzip_parts(&bytes).unwrap();let expanded:usize=parts.iter().map(|(_,b)|b.len()).sum();
    let limits=IntakeLimits{max_expanded_bytes:expanded,max_package_parts:parts.len(),..Default::default()};
    let before=candidate(&limits,0,0);assert_eq!(before.max_archive_size,expanded);assert_eq!(before.max_files_in_archive,parts.len());
    let admitted=helper::extract(request(&bytes,limits.clone())).unwrap().consumed.unwrap();assert_eq!(admitted.expanded_bytes,expanded);
    assert_eq!(atoms(&run(&bytes,before.clone()).unwrap(),true),vec![Atom::Text("Boundary.".into())]);
    let wrongly_after=candidate(&limits,admitted.expanded_bytes,admitted.package_parts);assert_eq!(wrongly_after.max_archive_size,0);assert!(run(&bytes,wrongly_after).is_err());
    // Finite binary search on a fixed synthetic source measures consumed budget steps.
    let mut lo=0;let mut hi=before.max_iterations;
    while lo+1<hi {let mid=(lo+hi)/2;let mut l=before.clone();l.max_iterations=mid;if run(&bytes,l).is_ok(){hi=mid}else{lo=mid}}
    let mut at=before.clone();at.max_iterations=hi;assert!(run(&bytes,at).is_ok());
    let mut below=before.clone();below.max_iterations=lo;assert!(run(&bytes,below).is_err());
    record("S6","iteration-budget-fixed-source",json!({"candidate":expanded+parts.len(),"hypothesis":"E+P, not heap proof"}),json!({"first_success":hi,"last_refused":lo,"expanded":expanded,"parts":parts.len()}));
    let mut over=before.clone();over.max_files_in_archive=parts.len()-1;assert!(run(&bytes,over).is_err());
    let mut over=before;over.max_archive_size=expanded-1;assert!(run(&bytes,over).is_err());
}
#[test]
fn office_limits_central_actual_lies_zero_compressed_empty_and_descriptor() {
    let bytes=boundary_package();
    for valid in [stored(&bytes,true),descriptor(&bytes,"word/document.xml")] {
        let doc=extract(&valid);assert_eq!(atoms(&doc,true),vec![Atom::Text("Boundary.".into())]);
    }
    assert!(run(&size_lie(&bytes,"word/document.xml",0,true),candidate(&IntakeLimits::default(),0,0)).is_err());
    for (member,source) in [("word/document.xml",bytes.clone()),("word/footnotes.xml",bytes.clone()),("word/media/same.png",repeated_figures())] {for delta in [-1,1] {
        let malformed=size_lie(&source,member,delta,false);
        let current=helper::extract(request(&malformed,IntakeLimits::default()));
        let maintained=run(&malformed,candidate(&IntakeLimits::default(),0,0));
        record("S6",&format!("xberg-size-{member}-{delta}"),json!({"member":member,"central_minus_actual":delta,"required":member=="word/document.xml","policy":"reject inconsistent members before future promotion, valid descriptor is separate"}),json!({"current":current.map(|x|json!({"wording":markdown_wording(&x.markdown),"gaps":x.gaps})),"xberg":maintained.map(|x|witness(&x))}));
    }}
}
#[test]
fn office_limits_content_cells_depth_and_repeated_story_passes() {
    let bytes=package(&format!("{}{}",paragraph("One."),paragraph("Two.")),"",&[]);
    let limits=candidate(&IntakeLimits::default(),0,0);
    let mut l=limits.clone();l.max_content_size=7;assert!(run(&bytes,l).is_err());
    let mut l=limits.clone();l.max_entity_length=3;assert!(run(&bytes,l).is_err());
    let table="<w:tbl><w:tblGrid><w:gridCol/></w:tblGrid><w:tr><w:tc><w:p><w:r><w:t>Cell</w:t></w:r></w:p></w:tc></w:tr></w:tbl>";
    let bytes=package(&table.repeat(2),"",&[]);let mut l=limits.clone();l.max_table_cells=1;assert!(run(&bytes,l).is_err());
    let body=format!("{}{}{}","<w:sdt><w:sdtContent>".repeat(130),paragraph("Deep"),"</w:sdtContent></w:sdt>".repeat(130));
    assert!(run(&package(&body,"",&[]),limits.clone()).is_err());
    let mut dtd=ooxml_opc::unzip_parts(&bytes).unwrap();
    let main=&mut dtd.iter_mut().find(|(name,_)|name=="word/document.xml").unwrap().1;
    let mut with_dtd=b"<!DOCTYPE document [<!ENTITY value \"forbidden\">]>".to_vec();with_dtd.extend_from_slice(main);*main=with_dtd;
    assert!(run(&ooxml_opc::rezip_parts(&dtd).unwrap(),limits.clone()).is_err());
    let events=package(&format!("<w:p>{}</w:p>","<w:r/>".repeat(200_001)),"",&[]);
    let current=helper::extract(request(&events,IntakeLimits::default()));
    assert!(current.is_err(),"existing 200,000 XML event policy");
    record("S6","xml-event-policy",json!({"empty_run_elements":200001,"current_event_limit":200000,"candidate":"E+P may admit more events; not equivalence proof"}),json!({"current":current,"xberg":run(&events,limits.clone()).map(|d|witness(&d))}));
    let hdr=format!("<w:hdr xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">{}</w:hdr>",paragraph("Shared.").repeat(100));
    let rels=(0..16).map(|i|relationship(&format!("h{i}"),"header","header1.xml")).collect::<String>();
    let bytes=package(&paragraph("Body."),&rels,&[("word/header1.xml",&hdr)]);
    let parts=ooxml_opc::unzip_parts(&bytes).unwrap();let expanded:usize=parts.iter().map(|(_,b)|b.len()).sum();
    let l=candidate(&IntakeLimits{max_expanded_bytes:expanded,max_package_parts:parts.len(),..Default::default()},0,0);
    record("S6","repeated-header-pass-candidate",json!({"source_header_paragraphs":100,"relationships_to_same_target":16,"expanded":expanded,"candidate_iterations":l.max_iterations,"proof":"unproven acceptance/pass count, no automatic quota raising"}),json!(run(&bytes,l).map(|d|witness(&d))));
}
