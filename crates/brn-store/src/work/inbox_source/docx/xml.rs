//! Fixed resource guard before complete namespace-aware DOM validation.
use super::{Failure, MAX_DEPTH, MAX_XML, MAX_XML_ITEMS, Result, check_cancel};
use quick_xml::{Reader, events::Event};
use std::sync::atomic::AtomicBool;

#[derive(Default)]
pub(super) struct Budget {
    bytes: usize,
    items: usize,
}
impl Budget {
    pub(super) fn parse<'a>(
        &mut self,
        bytes: &'a [u8],
        cancel: &AtomicBool,
    ) -> Result<roxmltree::Document<'a>> {
        check_cancel(cancel)?;
        self.bytes = self.bytes.checked_add(bytes.len()).ok_or(Failure::Limit)?;
        if self.bytes > MAX_XML {
            return Err(Failure::Limit);
        }
        if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
            return Err(Failure::Unsupported);
        }
        let text = std::str::from_utf8(bytes).map_err(|_| Failure::Invalid)?;
        // roxmltree reserves from raw delimiter counts, including delimiters
        // inside CDATA/comments. A node limit alone does not bound that reserve.
        if text.bytes().filter(|b| matches!(b, b'<' | b'=')).count() > MAX_XML_ITEMS {
            return Err(Failure::Limit);
        }
        let mut reader = Reader::from_str(text);
        reader.config_mut().check_end_names = true;
        let mut depth = 0usize;
        loop {
            check_cancel(cancel)?;
            let event = reader.read_event().map_err(|_| Failure::Invalid)?;
            match event {
                Event::Start(ref tag) | Event::Empty(ref tag) => {
                    self.items += 1;
                    let mut attributes = 0;
                    for attribute in tag.attributes() {
                        attribute.map_err(|_| Failure::Invalid)?;
                        attributes += 1;
                        self.items += 1;
                        if attributes > 64 || self.items > MAX_XML_ITEMS {
                            return Err(Failure::Limit);
                        }
                    }
                    // Empty elements still have a nesting depth.
                    if depth + 1 > MAX_DEPTH {
                        return Err(Failure::Limit);
                    }
                    if matches!(event, Event::Start(_)) {
                        depth += 1;
                    }
                }
                Event::End(_) => {
                    depth = depth.checked_sub(1).ok_or(Failure::Invalid)?;
                }
                Event::DocType(_) => return Err(Failure::Unsupported),
                Event::Decl(declaration) => {
                    if let Some(encoding) = declaration.encoding() {
                        let encoding = encoding.map_err(|_| Failure::Invalid)?;
                        if !encoding.eq_ignore_ascii_case(b"UTF-8") {
                            return Err(Failure::Unsupported);
                        }
                    }
                }
                Event::Eof => {
                    if depth != 0 {
                        return Err(Failure::Invalid);
                    }
                    break;
                }
                _ => self.items += 1,
            }
            if self.items > MAX_XML_ITEMS {
                return Err(Failure::Limit);
            }
        }
        check_cancel(cancel)?;
        roxmltree::Document::parse_with_options(
            text,
            roxmltree::ParsingOptions {
                allow_dtd: false,
                nodes_limit: MAX_XML_ITEMS as u32,
                entity_resolver: None,
            },
        )
        .map_err(|_| Failure::Invalid)
    }
}
