use mail_parser::{MessageParser, MimeHeaders, PartType};
fn main() {
    let raw = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/fixture.eml")).unwrap();
    let message = MessageParser::default().parse(&raw).unwrap();
    assert_eq!(message.subject(), Some("Paint – värv"));
    assert_eq!(message.message_id(), Some("paint-2@example.test"));
    assert!(message.body_text(0).unwrap().contains("Värv: blue"));
    assert!(message.body_html(0).unwrap().contains("cid:paint-image"));
    assert_eq!(message.raw_message.as_ref(), raw);
    assert!(message.from().is_some() && message.to().is_some() && message.cc().is_some());
    assert!(message.header_raw("Date").unwrap().contains("+0300"));
    assert_eq!(message.date().unwrap().tz_hour, 3);
    assert_eq!(
        message.in_reply_to().as_text(),
        Some("paint-1@example.test")
    );
    assert_eq!(message.references().as_text_list().unwrap().len(), 2);
    let no_ids = MessageParser::default()
        .parse(b"Subject: same subject\r\nContent-Type: text/html\r\n\r\n<p>Only HTML</p>")
        .unwrap();
    assert_eq!(no_ids.message_id(), None);
    assert_eq!(no_ids.header_raw("Date"), None);
    assert!(matches!(
        no_ids.text_part(0).unwrap().body,
        PartType::Html(_)
    ));
    assert!(no_ids.body_text(0).unwrap().contains("Only HTML")); // generated alternative, not original text/plain
    println!(
        "subject={:?} date={:?} in_reply_to={:?} references={:?}",
        message.subject(),
        message.date(),
        message.in_reply_to(),
        message.references()
    );
    println!(
        "text_parts={} html_parts={} attachments={} parts={}",
        message.text_body_count(),
        message.html_body_count(),
        message.attachment_count(),
        message.parts.len()
    );
    for part in message.attachments() {
        println!(
            "attachment name={:?} cid={:?} bytes={}",
            part.attachment_name(),
            part.content_id(),
            part.contents().len()
        );
    }
    let doc = message
        .attachments()
        .find(|p| p.attachment_name() == Some("plan.docx"))
        .unwrap();
    assert_eq!(
        doc.contents(),
        std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/plan.docx")).unwrap()
    );
    let img = message
        .attachments()
        .find(|p| p.content_id() == Some("paint-image"))
        .unwrap();
    assert_eq!(img.contents(), b"synthetic-image-bytes");
    let unknown = message
        .attachments()
        .find(|p| p.attachment_name() == Some("unprocessed.bin"))
        .unwrap();
    assert_eq!(unknown.contents(), b"unsupported-attachment");
    println!(
        "PASS decoded header/body, timezone data, IDs, CID and exact attachments; no BRN ingestion/application exercised"
    );
}
