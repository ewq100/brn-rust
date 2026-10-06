//! One bounded ordinary inline DrawingML PNG. No IO and no pixel transformations.
use super::super::PngImageFacts;
use super::{Failure, Result, check_cancel};
use flate2::{Decompress, FlushDecompress, Status};
use roxmltree::Node;
use std::{collections::BTreeSet, io::Cursor, sync::atomic::AtomicBool};

const ENCODED_LIMIT: usize = 1024 * 1024;
const PNG: &[u8] = b"\x89PNG\r\n\x1a\n";
const WP: &str = "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing";
const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const PIC: &str = "http://schemas.openxmlformats.org/drawingml/2006/picture";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

fn be(bytes: &[u8]) -> u32 {
    u32::from_be_bytes(bytes.try_into().unwrap())
}

pub(super) fn validate_png(bytes: &[u8], cancel: &AtomicBool) -> Result<PngImageFacts> {
    check_cancel(cancel)?;
    if bytes.len() > ENCODED_LIMIT {
        return Err(Failure::Limit);
    }
    if !bytes.starts_with(PNG) {
        return Err(Failure::Invalid);
    }
    let mut at = 8usize;
    let mut facts = None;
    let mut idat = false;
    let mut idat_closed = false;
    let mut ended = false;
    let mut stream_ended = false;
    let mut inflate = Decompress::new(true);
    let mut scratch = [0u8; 32768];
    // Filtered scanlines never exceed the maximum 16-bit RGBA pixels plus
    // Adam7 scanline overhead. This bounds a malicious expanding IDAT too.
    let mut raw_limit = 0u64;
    let mut metadata_expanded = 0u64;
    let mut chunks = 0usize;
    let mut color = 0;
    let mut bit_depth = 0;
    let mut palette_entries = None;
    let mut singletons = BTreeSet::new();
    while at < bytes.len() {
        check_cancel(cancel)?;
        chunks += 1;
        if chunks > 1024 {
            return Err(Failure::Limit);
        }
        let header = bytes.get(at..at + 8).ok_or(Failure::Invalid)?;
        let size = be(&header[..4]) as usize;
        let kind = &header[4..];
        let end = at
            .checked_add(size)
            .and_then(|v| v.checked_add(12))
            .ok_or(Failure::Limit)?;
        let chunk = bytes.get(at + 4..end - 4).ok_or(Failure::Invalid)?;
        if !kind.iter().all(u8::is_ascii_alphabetic)
            || !kind[2].is_ascii_uppercase()
            || crc32fast::hash(chunk) != be(bytes.get(end - 4..end).ok_or(Failure::Invalid)?)
        {
            return Err(Failure::Invalid);
        }
        let data = &bytes[at + 8..end - 4];
        if !matches!(kind, b"IDAT" | b"tEXt" | b"zTXt" | b"iTXt") && !singletons.insert(kind) {
            return Err(Failure::Invalid);
        }
        validate_chunk_shape(kind, data, idat, color, bit_depth, palette_entries)?;
        if facts.is_none() && kind != b"IHDR" {
            return Err(Failure::Invalid);
        }
        if idat && kind != b"IDAT" {
            idat_closed = true;
        }
        match kind {
            b"IHDR" => {
                if facts.is_some() || data.len() != 13 {
                    return Err(Failure::Invalid);
                }
                let width = be(&data[..4]);
                let height = be(&data[4..8]);
                if width == 0 || height == 0 {
                    return Err(Failure::Invalid);
                }
                if width > 4096 || height > 4096 || u64::from(width) * u64::from(height) > 4_194_304
                {
                    return Err(Failure::Limit);
                }
                let channels = match data[9] {
                    0 | 3 => 1u64,
                    2 => 3,
                    4 => 2,
                    6 => 4,
                    _ => return Err(Failure::Invalid),
                };
                let depth = u64::from(data[8]);
                color = data[9];
                bit_depth = data[8];
                if !matches!(depth, 1 | 2 | 4 | 8 | 16)
                    || data[10] != 0
                    || data[11] != 0
                    || data[12] > 1
                {
                    return Err(Failure::Invalid);
                }
                raw_limit = if data[12] == 0 {
                    ((u64::from(width) * channels * depth).div_ceil(8) + 1) * u64::from(height)
                } else {
                    [
                        (0, 0, 8, 8),
                        (4, 0, 8, 8),
                        (0, 4, 4, 8),
                        (2, 0, 4, 4),
                        (0, 2, 2, 4),
                        (1, 0, 2, 2),
                        (0, 1, 1, 2),
                    ]
                    .into_iter()
                    .map(|(x, y, dx, dy)| {
                        let w = u64::from(width).saturating_sub(x).div_ceil(dx);
                        let h = u64::from(height).saturating_sub(y).div_ceil(dy);
                        if w == 0 || h == 0 {
                            0
                        } else {
                            ((w * channels * depth).div_ceil(8) + 1) * h
                        }
                    })
                    .sum()
                };
                facts = Some(PngImageFacts { width, height });
            }
            b"PLTE" => {
                palette_entries = Some(data.len() / 3);
            }
            b"acTL" | b"fcTL" | b"fdAT" => return Err(Failure::Unsupported),
            b"iCCP" | b"zTXt" | b"iTXt" => {
                let compressed = compressed_metadata(kind, data)?;
                if let Some(compressed) = compressed {
                    let remaining = (8 * 1024 * 1024u64)
                        .checked_sub(metadata_expanded)
                        .ok_or(Failure::Limit)?;
                    metadata_expanded +=
                        validate_zlib(compressed, remaining, kind == b"iTXt", cancel)?;
                }
            }
            b"IDAT" => {
                if idat_closed {
                    return Err(Failure::Invalid);
                }
                idat = true;
                let mut input = data;
                loop {
                    check_cancel(cancel)?;
                    if stream_ended {
                        if !input.is_empty() {
                            return Err(Failure::Invalid);
                        }
                        break;
                    }
                    let before_in = inflate.total_in();
                    let before_out = inflate.total_out();
                    let status = inflate
                        .decompress(input, &mut scratch, FlushDecompress::None)
                        .map_err(|_| Failure::Invalid)?;
                    let consumed = (inflate.total_in() - before_in) as usize;
                    let produced = inflate.total_out() - before_out;
                    input = &input[consumed..];
                    if inflate.total_out() > raw_limit {
                        return Err(Failure::Invalid);
                    }
                    if status == Status::StreamEnd {
                        stream_ended = true;
                        continue;
                    }
                    if consumed == 0 && produced == 0 {
                        if !input.is_empty() {
                            return Err(Failure::Invalid);
                        }
                        break;
                    }
                    if input.is_empty() && produced < scratch.len() as u64 {
                        break;
                    }
                }
            }
            b"IEND" => {
                if !data.is_empty()
                    || !idat
                    || !stream_ended
                    || inflate.total_out() != raw_limit
                    || end != bytes.len()
                {
                    return Err(Failure::Invalid);
                }
                ended = true;
            }
            _ if kind[0].is_ascii_uppercase() && kind != b"PLTE" => {
                return Err(Failure::Unsupported);
            }
            _ => {}
        }
        at = end;
    }
    if !ended {
        return Err(Failure::Invalid);
    }
    let facts = facts.ok_or(Failure::Invalid)?;
    // Expose png's explicit BadAncillaryChunk events; its high-level reader
    // intentionally discards these. The preceding streaming checks bound every
    // compressed metadata expansion before this parser is allowed to allocate.
    {
        let mut strict = png::StreamingDecoder::new();
        strict.set_ignore_adler32(false);
        strict.set_ignore_crc(false);
        strict.set_skip_ancillary_crc_failures(false);
        let mut input = bytes;
        while !input.is_empty() {
            check_cancel(cancel)?;
            let (consumed, event) = strict.update(input, None).map_err(|_| Failure::Invalid)?;
            if matches!(event, png::Decoded::BadAncillaryChunk(_)) || consumed == 0 {
                return Err(Failure::Invalid);
            }
            input = &input[consumed..];
        }
    }
    // Hard32MiB decoder-allocation budget, based on the pinned implementation:
    // no full raster buffer is allocated. The now-dropped streaming parser has
    // one ICC inflate capped at8MiB by preflight (<16MiB Vec capacity), <=1MiB
    // encoded metadata (raw-buffer/clone/Latin1 expansion <6MiB), <=1024 chunk
    // records and fixed decoder state (<2MiB). Its peak is below24MiB. The row
    // decoder skips already-validated compressed text/ICC metadata; its charged
    // 8MiB limit is a second guard, not the source of the hard-budget claim.
    // At <=4096 dimensions, unfilter storage is bounded by128KiB shift-back +
    // lookback + a <=32769-byte row +8192-byte growth (Vec capacity <1MiB).
    // Remaining row/scratch/metadata/fixed-state storage stays below4MiB. The
    // phases are sequential, so their allocations never accumulate to32MiB.
    let mut options = png::DecodeOptions::default();
    options.set_ignore_crc(false);
    options.set_ignore_adler32(false);
    let mut decoder = png::Decoder::new_with_options(Cursor::new(bytes), options);
    decoder.set_ignore_text_chunk(true);
    decoder.set_ignore_iccp_chunk(true);
    decoder.set_limits(png::Limits {
        bytes: 8 * 1024 * 1024,
    });
    let map = |e| match e {
        png::DecodingError::LimitsExceeded => Failure::Limit,
        _ => Failure::Invalid,
    };
    let mut reader = decoder.read_info().map_err(map)?;
    let passes = if reader.info().interlaced {
        vec![
            (0, 0, 8, 8),
            (4, 0, 8, 8),
            (0, 4, 4, 8),
            (2, 0, 4, 4),
            (0, 2, 2, 4),
            (1, 0, 2, 2),
            (0, 1, 1, 2),
        ]
    } else {
        vec![(0, 0, 1, 1)]
    };
    let mut samples = passes.into_iter().flat_map(|(x, y, dx, dy)| {
        let width = facts.width.saturating_sub(x).div_ceil(dx);
        let height = facts.height.saturating_sub(y).div_ceil(dy);
        std::iter::repeat_n(width, if width == 0 { 0 } else { height as usize })
    });
    loop {
        check_cancel(cancel)?;
        let Some(row) = reader.next_interlaced_row().map_err(map)? else {
            break;
        };
        let width = samples.next().ok_or(Failure::Invalid)?;
        if color == 3 {
            let palette = palette_entries.ok_or(Failure::Invalid)?;
            let depth = usize::from(bit_depth);
            for sample in 0..width as usize {
                let bit = sample * depth;
                let index = usize::from(
                    (row.data()[bit / 8] >> (8 - depth - bit % 8)) & ((1u16 << depth) - 1) as u8,
                );
                if index >= palette {
                    return Err(Failure::Invalid);
                }
            }
        }
    }
    if samples.next().is_some() {
        return Err(Failure::Invalid);
    }
    reader.finish().map_err(map)?;
    check_cancel(cancel)?;
    Ok(facts)
}

// Some decoder parsers accept excess palette/ancillary bytes or ignore bad
// ancillary values. Check exact recognized chunk shapes and required ordering.
fn validate_chunk_shape(
    kind: &[u8],
    data: &[u8],
    idat: bool,
    color: u8,
    depth: u8,
    palette: Option<usize>,
) -> Result<()> {
    let len = match kind {
        b"gAMA" => Some(4),
        b"cHRM" => Some(32),
        b"sRGB" => Some(1),
        b"pHYs" => Some(9),
        b"tIME" => Some(7),
        b"cICP" => Some(4),
        b"mDCV" => Some(24),
        b"cLLI" => Some(8),
        b"sBIT" => Some(match color {
            0 => 1,
            2 | 3 => 3,
            4 => 2,
            6 => 4,
            _ => return Err(Failure::Invalid),
        }),
        b"bKGD" => Some(match color {
            0 | 4 => 2,
            2 | 6 => 6,
            3 => 1,
            _ => return Err(Failure::Invalid),
        }),
        _ => None,
    };
    if len.is_some_and(|len| data.len() != len) {
        return Err(Failure::Invalid);
    }
    if matches!(
        kind,
        b"PLTE"
            | b"sBIT"
            | b"bKGD"
            | b"tRNS"
            | b"hIST"
            | b"gAMA"
            | b"cHRM"
            | b"sRGB"
            | b"iCCP"
            | b"pHYs"
            | b"cICP"
            | b"mDCV"
            | b"cLLI"
    ) && idat
    {
        return Err(Failure::Invalid);
    }
    if matches!(
        kind,
        b"gAMA" | b"cHRM" | b"sRGB" | b"iCCP" | b"sBIT" | b"cICP" | b"mDCV"
    ) && palette.is_some()
    {
        return Err(Failure::Invalid);
    }
    match kind {
        b"PLTE"
            if matches!(color, 0 | 4)
                || data.is_empty()
                || data.len() > 768
                || !data.len().is_multiple_of(3)
                || color == 3 && data.len() / 3 > (1usize << depth) =>
        {
            return Err(Failure::Invalid);
        }
        b"IDAT" if color == 3 && palette.is_none() => return Err(Failure::Invalid),
        b"tRNS" => {
            let valid = match color {
                0 => data.len() == 2,
                2 => data.len() == 6,
                3 => palette.is_some_and(|p| !data.is_empty() && data.len() <= p),
                _ => false,
            };
            if !valid
                || matches!(color, 0 | 2)
                    && data
                        .as_chunks::<2>()
                        .0
                        .iter()
                        .any(|v| u32::from(u16::from_be_bytes(*v)) >= (1u32 << depth))
            {
                return Err(Failure::Invalid);
            }
        }
        b"hIST" if palette.is_none_or(|p| data.len() != p * 2) => return Err(Failure::Invalid),
        b"bKGD" if color == 3 && palette.is_none_or(|p| usize::from(data[0]) >= p) => {
            return Err(Failure::Invalid);
        }
        b"gAMA" if be(data) == 0 => return Err(Failure::Invalid),
        b"sRGB" if data[0] > 3 => return Err(Failure::Invalid),
        b"pHYs" if data[8] > 1 => return Err(Failure::Invalid),
        b"sBIT"
            if data
                .iter()
                .any(|v| *v == 0 || *v > if color == 3 { 8 } else { depth }) =>
        {
            return Err(Failure::Invalid);
        }
        b"tIME"
            if !(1..=12).contains(&data[2])
                || !(1..=31).contains(&data[3])
                || data[4] > 23
                || data[5] > 59
                || data[6] > 60 =>
        {
            return Err(Failure::Invalid);
        }
        _ => {}
    }
    Ok(())
}

// png0.18.1 deliberately ignores malformed ICC compression and lazily stores
// compressed text. Validate each stream completely before its decoder can see it.
fn compressed_metadata<'a>(kind: &[u8], data: &'a [u8]) -> Result<Option<&'a [u8]>> {
    let keyword_end = data.iter().position(|b| *b == 0).ok_or(Failure::Invalid)?;
    if !(1..=79).contains(&keyword_end) {
        return Err(Failure::Invalid);
    }
    let tail = &data[keyword_end + 1..];
    if kind != b"iTXt" {
        if tail.first() != Some(&0) {
            return Err(Failure::Invalid);
        }
        return Ok(Some(&tail[1..]));
    }
    if tail.len() < 2 || tail[0] > 1 || tail[1] != 0 {
        return Err(Failure::Invalid);
    }
    let language_end = tail[2..]
        .iter()
        .position(|b| *b == 0)
        .ok_or(Failure::Invalid)?
        + 2;
    if !tail[2..language_end].is_ascii() {
        return Err(Failure::Invalid);
    }
    let translated = &tail[language_end + 1..];
    let translated_end = translated
        .iter()
        .position(|b| *b == 0)
        .ok_or(Failure::Invalid)?;
    std::str::from_utf8(&translated[..translated_end]).map_err(|_| Failure::Invalid)?;
    let text = &translated[translated_end + 1..];
    if tail[0] == 0 {
        std::str::from_utf8(text).map_err(|_| Failure::Invalid)?;
        Ok(None)
    } else {
        Ok(Some(text))
    }
}
fn validate_zlib(mut input: &[u8], limit: u64, utf8: bool, cancel: &AtomicBool) -> Result<u64> {
    let mut decoder = Decompress::new(true);
    let mut output = [0u8; 32768];
    let mut text = Utf8Check::default();
    loop {
        check_cancel(cancel)?;
        let before_in = decoder.total_in();
        let before_out = decoder.total_out();
        let status = decoder
            .decompress(input, &mut output, FlushDecompress::Finish)
            .map_err(|_| Failure::Invalid)?;
        input = &input[(decoder.total_in() - before_in) as usize..];
        if utf8 {
            for byte in &output[..(decoder.total_out() - before_out) as usize] {
                text.byte(*byte)?;
            }
        }
        if decoder.total_out() > limit {
            return Err(Failure::Limit);
        }
        if status == Status::StreamEnd {
            if !input.is_empty() || utf8 && text.remaining != 0 {
                return Err(Failure::Invalid);
            }
            return Ok(decoder.total_out());
        }
        if decoder.total_in() == before_in && decoder.total_out() == before_out {
            return Err(Failure::Invalid);
        }
    }
}

#[derive(Default)]
struct Utf8Check {
    remaining: u8,
    min: u8,
    max: u8,
}
impl Utf8Check {
    fn byte(&mut self, byte: u8) -> Result<()> {
        if self.remaining != 0 {
            if !(self.min..=self.max).contains(&byte) {
                return Err(Failure::Invalid);
            }
            self.remaining -= 1;
            self.min = 0x80;
            self.max = 0xbf;
        } else {
            (self.remaining, self.min, self.max) = match byte {
                0x00..=0x7f => (0, 0, 0),
                0xc2..=0xdf => (1, 0x80, 0xbf),
                0xe0 => (2, 0xa0, 0xbf),
                0xed => (2, 0x80, 0x9f),
                0xe1..=0xec | 0xee..=0xef => (2, 0x80, 0xbf),
                0xf0 => (3, 0x90, 0xbf),
                0xf4 => (3, 0x80, 0x8f),
                0xf1..=0xf3 => (3, 0x80, 0xbf),
                _ => return Err(Failure::Invalid),
            };
        }
        Ok(())
    }
}

pub(super) struct Inline {
    pub relationship_id: String,
    pub alt_text: Option<String>,
    pub title: Option<String>,
}

fn ns(node: Node<'_, '_>, namespace: &str, name: &str) -> bool {
    node.has_tag_name((namespace, name))
        || namespace
            .strip_prefix("http://schemas.openxmlformats.org/")
            .and_then(|suffix| suffix.strip_prefix("drawingml/2006/"))
            .is_some_and(|suffix| {
                node.has_tag_name((
                    format!("http://purl.oclc.org/ooxml/drawingml/{suffix}").as_str(),
                    name,
                ))
            })
}
fn kids<'a, 'i>(node: Node<'a, 'i>) -> Result<Vec<Node<'a, 'i>>> {
    if node
        .children()
        .any(|n| n.is_text() && !n.text().unwrap_or("").trim().is_empty())
    {
        return Err(Failure::Invalid);
    }
    Ok(node.children().filter(Node::is_element).collect())
}
fn attrs(node: Node<'_, '_>, allowed: &[&str]) -> Result<()> {
    if node
        .attributes()
        .any(|a| a.namespace().is_some() || !allowed.contains(&a.name()))
    {
        return Err(Failure::Unsupported);
    }
    Ok(())
}
fn one<'a, 'i>(node: Node<'a, 'i>, namespace: &str, name: &str) -> Result<Node<'a, 'i>> {
    let children = kids(node)?;
    if children.len() != 1 || !ns(children[0], namespace, name) {
        return Err(Failure::Unsupported);
    }
    Ok(children[0])
}
fn empty(node: Node<'_, '_>) -> Result<()> {
    if !kids(node)?.is_empty() {
        return Err(Failure::Unsupported);
    }
    Ok(())
}
fn extent(node: Node<'_, '_>, keys: &[&str]) -> Result<()> {
    attrs(node, keys)?;
    empty(node)?;
    for key in keys {
        if node
            .attribute(*key)
            .and_then(|v| v.parse::<u64>().ok())
            .is_none_or(|v| v == 0)
        {
            return Err(Failure::Invalid);
        }
    }
    Ok(())
}

pub(super) fn inline(drawing: Node<'_, '_>, cancel: &AtomicBool) -> Result<Inline> {
    check_cancel(cancel)?;
    attrs(drawing, &[])?;
    let inline = one(drawing, WP, "inline")?;
    attrs(inline, &["distT", "distB", "distL", "distR"])?;
    if inline.attributes().any(|a| a.value() != "0") {
        return Err(Failure::Unsupported);
    }
    let mut docpr = None;
    let mut graphic = None;
    let mut inline_extent = None;
    let mut seen = BTreeSet::new();
    for n in kids(inline)? {
        check_cancel(cancel)?;
        if !seen.insert((n.tag_name().namespace(), n.tag_name().name())) {
            return Err(Failure::Invalid);
        }
        if ns(n, WP, "extent") {
            extent(n, &["cx", "cy"])?;
            inline_extent = Some((n.attribute("cx"), n.attribute("cy")));
        } else if ns(n, WP, "effectExtent") {
            attrs(n, &["l", "t", "r", "b"])?;
            empty(n)?;
            if n.attributes().any(|a| a.value() != "0") {
                return Err(Failure::Unsupported);
            }
        } else if ns(n, WP, "docPr") {
            attrs(n, &["id", "name", "descr", "title"])?;
            empty(n)?;
            if n.attribute("id")
                .and_then(|v| v.parse::<u32>().ok())
                .is_none()
                || n.attribute("name").is_none()
            {
                return Err(Failure::Invalid);
            }
            docpr = Some(n);
        } else if ns(n, WP, "cNvGraphicFramePr") {
            attrs(n, &[])?;
            for lock in kids(n)? {
                if !ns(lock, A, "graphicFrameLocks") {
                    return Err(Failure::Unsupported);
                }
                attrs(lock, &["noChangeAspect"])?;
                empty(lock)?;
            }
        } else if ns(n, A, "graphic") {
            graphic = Some(n);
        } else {
            return Err(Failure::Unsupported);
        }
    }
    if !seen.iter().any(|(_, name)| *name == "extent") {
        return Err(Failure::Invalid);
    }
    let docpr = docpr.ok_or(Failure::Invalid)?;
    let graphic = graphic.ok_or(Failure::Invalid)?;
    attrs(graphic, &[])?;
    let data = one(graphic, A, "graphicData")?;
    attrs(data, &["uri"])?;
    if ![PIC, "http://purl.oclc.org/ooxml/drawingml/picture"]
        .contains(&data.attribute("uri").ok_or(Failure::Invalid)?)
    {
        return Err(Failure::Unsupported);
    }
    let pic = one(data, PIC, "pic")?;
    attrs(pic, &[])?;
    let mut relationship = None;
    let mut pic_seen = BTreeSet::new();
    for n in kids(pic)? {
        if !pic_seen.insert(n.tag_name().name()) {
            return Err(Failure::Invalid);
        }
        if ns(n, PIC, "nvPicPr") {
            attrs(n, &[])?;
            let mut seen = BTreeSet::new();
            for p in kids(n)? {
                if !seen.insert(p.tag_name().name()) {
                    return Err(Failure::Invalid);
                }
                if ns(p, PIC, "cNvPr") {
                    attrs(p, &["id", "name"])?;
                    empty(p)?;
                    if p.attribute("id")
                        .and_then(|v| v.parse::<u32>().ok())
                        .is_none()
                        || p.attribute("name").is_none()
                    {
                        return Err(Failure::Invalid);
                    }
                } else if ns(p, PIC, "cNvPicPr") {
                    attrs(p, &[])?;
                    for lock in kids(p)? {
                        if !ns(lock, A, "picLocks") {
                            return Err(Failure::Unsupported);
                        }
                        attrs(lock, &["noChangeAspect", "noChangeArrowheads"])?;
                        empty(lock)?;
                    }
                } else {
                    return Err(Failure::Unsupported);
                }
            }
            if seen != BTreeSet::from(["cNvPr", "cNvPicPr"]) {
                return Err(Failure::Invalid);
            }
        } else if ns(n, PIC, "blipFill") {
            attrs(n, &[])?;
            let mut seen = BTreeSet::new();
            for p in kids(n)? {
                if !seen.insert(p.tag_name().name()) {
                    return Err(Failure::Invalid);
                }
                if ns(p, A, "blip") {
                    empty(p)?;
                    if p.attributes().any(|a| {
                        !([
                            Some(R),
                            Some("http://purl.oclc.org/ooxml/officeDocument/relationships"),
                        ]
                        .contains(&a.namespace())
                            && a.name() == "embed")
                            && !(a.namespace().is_none()
                                && a.name() == "cstate"
                                && matches!(
                                    a.value(),
                                    "none" | "email" | "screen" | "print" | "hqprint"
                                ))
                    }) {
                        return Err(Failure::Unsupported);
                    }
                    if p.attributes().filter(|a| a.name() == "embed").count() != 1 {
                        return Err(Failure::Invalid);
                    }
                    relationship = p
                        .attribute((R, "embed"))
                        .or_else(|| {
                            p.attribute((
                                "http://purl.oclc.org/ooxml/officeDocument/relationships",
                                "embed",
                            ))
                        })
                        .map(str::to_owned);
                } else if ns(p, A, "stretch") {
                    attrs(p, &[])?;
                    let fill = one(p, A, "fillRect")?;
                    attrs(fill, &[])?;
                    empty(fill)?;
                } else {
                    return Err(Failure::Unsupported);
                }
            }
            if seen != BTreeSet::from(["blip", "stretch"]) {
                return Err(Failure::Invalid);
            }
        } else if ns(n, PIC, "spPr") {
            attrs(n, &["bwMode"])?;
            if n.attribute("bwMode").is_some_and(|v| v != "auto") {
                return Err(Failure::Unsupported);
            }
            let mut seen = BTreeSet::new();
            for p in kids(n)? {
                if !seen.insert(p.tag_name().name()) {
                    return Err(Failure::Invalid);
                }
                if ns(p, A, "xfrm") {
                    attrs(p, &["rot", "flipH", "flipV"])?;
                    if p.attributes().any(|a| {
                        if a.name() == "rot" {
                            a.value() != "0"
                        } else {
                            !matches!(a.value(), "0" | "false")
                        }
                    }) {
                        return Err(Failure::Unsupported);
                    }
                    let mut seen = BTreeSet::new();
                    for t in kids(p)? {
                        if !seen.insert(t.tag_name().name()) {
                            return Err(Failure::Invalid);
                        }
                        if ns(t, A, "off") {
                            attrs(t, &["x", "y"])?;
                            empty(t)?;
                            if t.attribute("x") != Some("0") || t.attribute("y") != Some("0") {
                                return Err(Failure::Unsupported);
                            }
                        } else if ns(t, A, "ext") {
                            extent(t, &["cx", "cy"])?;
                            if Some((t.attribute("cx"), t.attribute("cy"))) != inline_extent {
                                return Err(Failure::Unsupported);
                            }
                        } else {
                            return Err(Failure::Unsupported);
                        }
                    }
                    if seen != BTreeSet::from(["off", "ext"]) {
                        return Err(Failure::Invalid);
                    }
                } else if ns(p, A, "prstGeom") {
                    attrs(p, &["prst"])?;
                    if p.attribute("prst") != Some("rect") {
                        return Err(Failure::Unsupported);
                    }
                    let av = one(p, A, "avLst")?;
                    attrs(av, &[])?;
                    empty(av)?;
                } else {
                    return Err(Failure::Unsupported);
                }
            }
            if seen != BTreeSet::from(["xfrm", "prstGeom"]) {
                return Err(Failure::Invalid);
            }
        } else {
            return Err(Failure::Unsupported);
        }
    }
    if pic_seen != BTreeSet::from(["nvPicPr", "blipFill", "spPr"]) {
        return Err(Failure::Invalid);
    }
    Ok(Inline {
        relationship_id: relationship.ok_or(Failure::Invalid)?,
        alt_text: docpr.attribute("descr").map(str::to_owned),
        title: docpr.attribute("title").map(str::to_owned),
    })
}
