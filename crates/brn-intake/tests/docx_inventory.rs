#![cfg(feature = "helper")]
#[path = "support/office.rs"]
mod office;
use brn_intake::{helper, prepared::BlockContent, *};
use office::*;
use serde_json::json;

#[test]
fn current_docx_inventory_complete_ordered_schema1_and_schema2() {
    for case in cases() { check_current(&case); }
}
#[test]
fn current_docx_inventory_comparator_detects_loss_and_rebinding() {
    // Mutation of synthetic evidence, not a compiler-error or keyword-only witness.
    let case=cases().remove(0);
    let doc=helper::prepared::prepare(request(&case.bytes,IntakeLimits::default())).unwrap();
    let original=prepared_atoms(&doc);
    let mut changed=doc.clone();changed.blocks.remove(1);assert_ne!(prepared_atoms(&changed),original);
    let mut changed=doc.clone();
    for block in &mut changed.blocks { if let BlockContent::Table{rows}=&mut block.content { rows[1][1].text="wrong number".into(); rows[0][0].column_span=2;break; } }
    assert_ne!(prepared_atoms(&changed),original);
    let mut changed=doc.clone();let images:Vec<_>=changed.blocks.iter().enumerate().filter(|(_,b)|matches!(b.content,BlockContent::Figure{..})).map(|(i,_)|i).collect();
    assert_eq!(images.len(),2);changed.blocks.remove(images[1]);assert_ne!(prepared_atoms(&changed),original);
    let mut evidence=helper::extract(request(&case.bytes,IntakeLimits::default())).unwrap();
    evidence.occurrences[0].source_id="nonexistent-source".into();
    assert!(evidence.validate().is_err(),"source rebinding must fail DTO validation");
}
#[test]
fn current_docx_inventory_revisions_comments_source_context() {
    let bytes=revisions_comments();let e=helper::extract(request(&bytes,IntakeLimits::default())).unwrap();
    let d=helper::prepared::prepare(request(&bytes,IntakeLimits::default())).unwrap();
    let wording:Vec<_>=prepared_atoms(&d).iter().filter_map(text_of).collect();
    assert_eq!(wording,["Keep accepted.","Reviewed.","Reviewer note.","Reply note."]);
    assert_eq!(markdown_wording(&e.markdown),wording);
    let context:Vec<_>=d.blocks.iter().filter_map(|b|match &b.content {BlockContent::Metadata{name,value} if name=="Source comment context"=>Some(json!({"locator":b.locator,"value":value})),_=>None}).collect();
    // Exact opaque source-range formatting awaits measurement; never reconstruct it from Markdown.
    record("S2","current-comments",json!({"author":"Iris","id":"7","date":"2026-10-18T10:00:00Z","range_text":"Reviewed.","reply":{"id":"8","parent":"7","resolved":true}}),json!(context));
}
#[test]
fn current_docx_inventory_repeated_targets_keep_two_exact_occurrences() {
    let bytes=repeated_figures();let e=helper::extract(request(&bytes,IntakeLimits::default())).unwrap();
    let d=helper::prepared::prepare(request(&bytes,IntakeLimits::default())).unwrap();
    assert_eq!(e.assets.len(),1);assert_eq!(e.assets[0].bytes,include_bytes!("fixtures/water-use.png"));
    assert_eq!(e.occurrences.len(),2);assert_eq!(e.occurrences[0].asset_id,e.occurrences[1].asset_id);
    assert_ne!(e.occurrences[0].locator,e.occurrences[1].locator);assert!(e.occurrences[0].end<e.occurrences[1].start);
    assert_eq!(prepared_atoms(&d),vec![Atom::Text("Same caption.".into()),Atom::Figure(hex(&e.assets[0].sha256)),Atom::Text("Same caption.".into()),Atom::Figure(hex(&e.assets[0].sha256))]);
}
#[test]
fn current_docx_inventory_empty_orphan_notes_and_grid_boundaries() {
    for (name,bytes,source) in [
        ("current-orphan",empty_orphan(),json!({"body":["","Visible."],"bound_header":"Bound header.","orphan_header":"Orphan header."})),
        ("current-notes",note_collisions(),json!({"literal":"Literal [^1] and [cmt:7]. ","footnote_1":"Footnote one.","endnote_1":"Endnote one.","table_note_2":"Cell note.","noteref":{"bookmark":"rule","cached":"1"}})),
        ("current-grids",grids(),grid_source()),
    ] {
        let d=helper::prepared::prepare(request(&bytes,IntakeLimits::default())).unwrap();d.validate().unwrap();
        let e=helper::extract(request(&bytes,IntakeLimits::default())).unwrap();e.validate().unwrap();
        record("S6",name,source,json!({"atoms":prepared_atoms(&d),"blocks":d.blocks,"gaps":d.gaps,"schema1":e.markdown,"schema1_gaps":e.gaps}));
    }
}
#[test]
fn current_office_admission_existing_hostile_inputs_and_job_limits() {
    for bytes in [include_bytes!("fixtures/hostile-dtd.docx").as_slice(),include_bytes!("fixtures/hostile-path.docx"),include_bytes!("fixtures/hostile-inflate.docx"),include_bytes!("fixtures/hostile-members.docx")] {
        assert!(helper::extract(request(bytes,IntakeLimits::default())).is_err());
        assert!(helper::prepared::prepare(request(bytes,IntakeLimits::default())).is_err());
    }
    let bytes=package(&paragraph("Boundary."),"",&[]);
    let count=ooxml_opc::unzip_parts(&bytes).unwrap().len();let expanded:usize=ooxml_opc::unzip_parts(&bytes).unwrap().iter().map(|(_,b)|b.len()).sum();
    let at=IntakeLimits{max_package_parts:count,max_expanded_bytes:expanded,..Default::default()};
    let e=helper::extract(request(&bytes,at.clone())).unwrap();assert_eq!(e.consumed.unwrap().expanded_bytes,expanded);
    for limits in [IntakeLimits{max_package_parts:count-1,..at.clone()},IntakeLimits{max_expanded_bytes:expanded-1,..at}] { assert!(helper::extract(request(&bytes,limits.clone())).is_err());assert!(helper::prepared::prepare(request(&bytes,limits)).is_err()); }
    for kind in ["docm","dotx","pptm","ppsx","application/msword"] {assert!(helper::extract(HelperRequest{kind:kind.into(),bytes:bytes.clone(),limits:None}).is_err());}
}
#[test]
fn current_office_admission_descriptors_empty_members_and_malformed_baseline() {
    let note="<w:footnotes xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:footnote w:id=\"1\"><w:p><w:r><w:t>Optional note.</w:t></w:r></w:p></w:footnote></w:footnotes>";
    let bytes=package(&paragraph("Boundary."),&relationship("fn","footnotes","footnotes.xml"),&[("word/footnotes.xml",note)]);
    for valid in [descriptor(&bytes,"word/document.xml"),stored(&bytes,true)] {assert!(helper::extract(request(&valid,IntakeLimits::default())).is_ok());assert!(helper::prepared::prepare(request(&valid,IntakeLimits::default())).is_ok());}
    for (member,source) in [("word/document.xml",bytes.clone()),("word/footnotes.xml",bytes.clone()),("word/media/same.png",repeated_figures())] {for delta in [-1,1] {
        let malformed=size_lie(&source,member,delta,false);
        let a=helper::extract(request(&malformed,IntakeLimits::default()));let b=helper::prepared::prepare(request(&malformed,IntakeLimits::default()));
        record("S6",&format!("current-size-{member}-{delta}"),json!({"member":member,"central_minus_actual":delta,"contract":"inconsistent member; measure existing acceptance, no production fix"}),json!({"schema1":a.map(|e|json!({"wording":markdown_wording(&e.markdown),"usage":e.consumed})),"schema2":b.map(|d|prepared_atoms(&d))}));
    }}
    assert!(helper::extract(request(&size_lie(&bytes,"word/document.xml",0,true),IntakeLimits::default())).is_err());
}
#[test]
fn current_office_admission_alias_dtd_depth_and_relationship_count() {
    let bytes=package(&paragraph("Admitted."),"",&[]);
    let original=ooxml_opc::unzip_parts(&bytes).unwrap();
    for alias in ["word/./document.xml","WORD/document.xml","../escape.xml"] {
        let mut parts=original.clone();parts.push((alias.into(),b"opaque".to_vec()));let bad=zip_parts(parts);
        assert!(helper::extract(request(&bad,IntakeLimits::default())).is_err());
        assert!(helper::prepared::prepare(request(&bad,IntakeLimits::default())).is_err());
    }
    let mut dtd=original.clone();dtd.iter_mut().find(|(n,_)|n=="word/document.xml").unwrap().1=br#"<!DOCTYPE document [<!ENTITY value "not allowed">]><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>&value;</w:t></w:r></w:p></w:body></w:document>"#.to_vec();
    assert!(helper::extract(request(&ooxml_opc::rezip_parts(&dtd).unwrap(),IntakeLimits::default())).is_err());
    let body=format!("{}{}{}","<w:sdt><w:sdtContent>".repeat(130),paragraph("Deep"),"</w:sdtContent></w:sdt>".repeat(130));
    assert!(helper::extract(request(&package(&body,"",&[]),IntakeLimits::default())).is_err());
    let rels=(0..513).map(|i|relationship(&format!("r{i}"),"image","media/missing.png")).collect::<String>();
    let bytes=package(&paragraph("Many relationships."),&rels,&[]);
    record("S6","current-relationship-count",json!({"relationship_count":513,"policy":"measure existing parse/count behavior; no new relationship quota invented"}),json!(helper::extract(request(&bytes,IntakeLimits::default())).map(|e|json!({"usage":e.consumed,"gaps":e.gaps,"wording":markdown_wording(&e.markdown)}))));
}
