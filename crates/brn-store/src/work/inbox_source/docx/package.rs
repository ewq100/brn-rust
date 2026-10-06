//! Bounded ZIP32 inventory guard followed by the maintained ZIP decoder.
use super::{Failure, MAX_ENTRIES, MAX_EXPANDED, MAX_PART, Result, check_cancel};
use std::{collections::BTreeMap, io::Read, sync::atomic::AtomicBool};

struct Entry {
    name: String,
    method: u16,
    size: usize,
    compressed: usize,
    local: usize,
    central: usize,
    data: usize,
    end: usize,
}

fn word(bytes: &[u8], at: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(
        bytes
            .get(at..at + 2)
            .ok_or(Failure::Invalid)?
            .try_into()
            .map_err(|_| Failure::Invalid)?,
    ))
}
fn long(bytes: &[u8], at: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        bytes
            .get(at..at + 4)
            .ok_or(Failure::Invalid)?
            .try_into()
            .map_err(|_| Failure::Invalid)?,
    ))
}
fn signature(bytes: &[u8], at: usize, expected: u32) -> Result<()> {
    if long(bytes, at)? != expected {
        return Err(Failure::Invalid);
    }
    Ok(())
}
fn extras(bytes: &[u8]) -> Result<()> {
    let mut at = 0;
    while at < bytes.len() {
        let id = word(bytes, at)?;
        let len = word(bytes, at + 2)? as usize;
        at = at.checked_add(4 + len).ok_or(Failure::Invalid)?;
        if at > bytes.len() {
            return Err(Failure::Invalid);
        }
        // ZIP64, Unicode name replacement, AES and patched alternate names are
        // outside this fixed profile; never accept a renamed effective part.
        if matches!(id, 0x0001 | 0x7075 | 0x9901) {
            return Err(Failure::Unsupported);
        }
    }
    Ok(())
}

fn inventory(bytes: &[u8], cancel: &AtomicBool) -> Result<(Vec<Entry>, usize)> {
    check_cancel(cancel)?;
    if bytes.len() > crate::work::inbox::MAX_INBOX_BINARY_BYTES {
        return Err(Failure::Limit);
    }
    if bytes.len() < 22 {
        return Err(Failure::Invalid);
    }
    let start = bytes.len().saturating_sub(22 + u16::MAX as usize);
    let mut footer = None;
    for at in start..=bytes.len() - 22 {
        if bytes[at..at + 4] == [0x50, 0x4b, 0x05, 0x06]
            && at + 22 + word(bytes, at + 20)? as usize == bytes.len()
            && footer.replace(at).is_some()
        {
            return Err(Failure::Invalid);
        }
    }
    let footer = footer.ok_or(Failure::Invalid)?;
    let count = word(bytes, footer + 10)? as usize;
    let central_size = long(bytes, footer + 12)? as usize;
    let central = long(bytes, footer + 16)? as usize;
    if count == u16::MAX as usize
        || central == u32::MAX as usize
        || central_size == u32::MAX as usize
    {
        return Err(Failure::Unsupported);
    }
    if word(bytes, footer + 4)? != 0
        || word(bytes, footer + 6)? != 0
        || word(bytes, footer + 8)? as usize != count
        || count == 0
        || central.checked_add(central_size) != Some(footer)
        || central_size < count * 46
    {
        return Err(Failure::Invalid);
    }
    if count > MAX_ENTRIES {
        return Err(Failure::Limit);
    }
    let mut entries = Vec::with_capacity(count);
    let mut names = std::collections::BTreeSet::new();
    let mut at = central;
    let mut expanded = 0usize;
    for _ in 0..count {
        check_cancel(cancel)?;
        signature(bytes, at, 0x02014b50)?;
        let version = word(bytes, at + 6)?;
        let flags = word(bytes, at + 8)?;
        let method = word(bytes, at + 10)?;
        if version > 20
            || flags & !0x080e != 0
            || !matches!(method, 0 | 8)
            || method == 0 && flags & 6 != 0
        {
            return Err(Failure::Unsupported);
        }
        let crc = long(bytes, at + 16)?;
        let compressed = long(bytes, at + 20)? as usize;
        let size = long(bytes, at + 24)? as usize;
        let name_len = word(bytes, at + 28)? as usize;
        let extra_len = word(bytes, at + 30)? as usize;
        let comment_len = word(bytes, at + 32)? as usize;
        if word(bytes, at + 34)? != 0 {
            return Err(Failure::Unsupported);
        }
        let mode = long(bytes, at + 38)? >> 16;
        let local = long(bytes, at + 42)? as usize;
        let next = at
            .checked_add(46 + name_len + extra_len + comment_len)
            .ok_or(Failure::Invalid)?;
        if next > footer || name_len == 0 || name_len > 1024 {
            return Err(Failure::Invalid);
        }
        let raw = bytes
            .get(at + 46..at + 46 + name_len)
            .ok_or(Failure::Invalid)?;
        let name = std::str::from_utf8(raw).map_err(|_| Failure::Invalid)?;
        // OPC names have one literal, contained interpretation. No extraction,
        // lossy name decoding, slash rewriting or percent-name alias adoption.
        if !name.is_ascii()
            || name.starts_with('/')
            || name.contains(['\\', '%', ':'])
            || name.bytes().any(|b| b.is_ascii_control())
            || name
                .strip_suffix('/')
                .unwrap_or(name)
                .split('/')
                .any(|p| p.is_empty() || matches!(p, "." | ".."))
            || !names.insert(name.to_ascii_lowercase())
        {
            return Err(Failure::Invalid);
        }
        let directory = name.ends_with('/');
        if mode & 0o170000 != 0 && mode & 0o170000 != if directory { 0o040000 } else { 0o100000 } {
            return Err(Failure::Unsupported);
        }
        if directory && size != 0 {
            return Err(Failure::Invalid);
        }
        if size > MAX_PART {
            return Err(Failure::Limit);
        }
        expanded = expanded.checked_add(size).ok_or(Failure::Limit)?;
        if expanded > MAX_EXPANDED {
            return Err(Failure::Limit);
        }
        if compressed > bytes.len() || method == 0 && compressed != size {
            return Err(Failure::Invalid);
        }
        extras(&bytes[at + 46 + name_len..at + 46 + name_len + extra_len])?;
        signature(bytes, local, 0x04034b50)?;
        if word(bytes, local + 4)? != version
            || word(bytes, local + 6)? != flags
            || word(bytes, local + 8)? != method
            || word(bytes, local + 26)? as usize != name_len
        {
            return Err(Failure::Invalid);
        }
        let local_extra = word(bytes, local + 28)? as usize;
        let data = local
            .checked_add(30 + name_len + local_extra)
            .ok_or(Failure::Invalid)?;
        if data > central || bytes.get(local + 30..local + 30 + name_len) != Some(raw) {
            return Err(Failure::Invalid);
        }
        extras(&bytes[local + 30 + name_len..data])?;
        let lc = long(bytes, local + 14)?;
        let ls = long(bytes, local + 18)? as usize;
        let lu = long(bytes, local + 22)? as usize;
        let mut end = data.checked_add(compressed).ok_or(Failure::Invalid)?;
        if end > central {
            return Err(Failure::Invalid);
        }
        if flags & 8 == 0 {
            if (lc, ls, lu) != (crc, compressed, size) {
                return Err(Failure::Invalid);
            }
        } else {
            if ![0, crc].contains(&lc) || ![0, compressed].contains(&ls) || ![0, size].contains(&lu)
            {
                return Err(Failure::Invalid);
            }
            let signed = long(bytes, end)? == 0x08074b50;
            let offset = if signed
                && long(bytes, end + 4)? == crc
                && long(bytes, end + 8)? as usize == compressed
                && long(bytes, end + 12)? as usize == size
            {
                4
            } else {
                0
            };
            if long(bytes, end + offset)? != crc
                || long(bytes, end + offset + 4)? as usize != compressed
                || long(bytes, end + offset + 8)? as usize != size
            {
                return Err(Failure::Invalid);
            }
            end += 12 + offset;
            if end > central {
                return Err(Failure::Invalid);
            }
        }
        entries.push(Entry {
            name: name.into(),
            method,
            size,
            compressed,
            local,
            central: at,
            data,
            end,
        });
        at = next;
    }
    if at != footer {
        return Err(Failure::Invalid);
    }
    let mut ranges: Vec<_> = entries.iter().map(|e| (e.local, e.end)).collect();
    ranges.sort_unstable();
    let mut cursor = 0;
    for (begin, end) in ranges {
        if begin != cursor || end <= begin {
            return Err(Failure::Invalid);
        }
        cursor = end;
    }
    if cursor != central {
        return Err(Failure::Invalid);
    }
    Ok((entries, central))
}

fn complete_deflate(bytes: &[u8], size: usize, cancel: &AtomicBool) -> Result<()> {
    use flate2::{Decompress, FlushDecompress, Status};
    let mut decoder = Decompress::new(false);
    let mut buffer = [0u8; 16 * 1024];
    loop {
        check_cancel(cancel)?;
        let before = (decoder.total_in(), decoder.total_out());
        let status = decoder
            .decompress(
                &bytes[before.0 as usize..],
                &mut buffer,
                FlushDecompress::Finish,
            )
            .map_err(|_| Failure::Invalid)?;
        if decoder.total_out() > size as u64 {
            return Err(Failure::Invalid);
        }
        if status == Status::StreamEnd {
            return if decoder.total_in() == bytes.len() as u64 && decoder.total_out() == size as u64
            {
                Ok(())
            } else {
                Err(Failure::Invalid)
            };
        }
        if before == (decoder.total_in(), decoder.total_out()) {
            return Err(Failure::Invalid);
        }
    }
}

pub(super) fn load(bytes: &[u8], cancel: &AtomicBool) -> Result<BTreeMap<String, Vec<u8>>> {
    let (entries, central) = inventory(bytes, cancel)?;
    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|_| Failure::Invalid)?;
    if archive.len() != entries.len() || archive.central_directory_start() != central as u64 {
        return Err(Failure::Invalid);
    }
    let mut parts = BTreeMap::new();
    let mut actual_total = 0usize;
    for (i, expected) in entries.iter().enumerate() {
        check_cancel(cancel)?;
        let mut file = archive.by_index(i).map_err(|_| Failure::Invalid)?;
        if file.name_raw() != expected.name.as_bytes()
            || file.name() != expected.name
            || file.header_start() != expected.local as u64
            || file.central_header_start() != expected.central as u64
            || file.data_start() != Some(expected.data as u64)
            || file.size() != expected.size as u64
            || file.compressed_size() != expected.compressed as u64
            || file.encrypted()
        {
            return Err(Failure::Invalid);
        }
        let mut content = Vec::new();
        let mut buffer = [0u8; 16 * 1024];
        loop {
            check_cancel(cancel)?;
            // Ordinary read reaches true EOF, including the decoder's CRC check.
            let n = file.read(&mut buffer).map_err(|_| Failure::Invalid)?;
            if n == 0 {
                break;
            }
            actual_total = actual_total.checked_add(n).ok_or(Failure::Limit)?;
            if content.len() + n > MAX_PART || actual_total > MAX_EXPANDED {
                return Err(Failure::Limit);
            }
            if content.len() + n > expected.size {
                return Err(Failure::Invalid);
            }
            content.extend_from_slice(&buffer[..n]);
        }
        if content.len() != expected.size {
            return Err(Failure::Invalid);
        }
        if expected.method == 8 {
            complete_deflate(
                &bytes[expected.data..expected.data + expected.compressed],
                expected.size,
                cancel,
            )?;
        }
        if !expected.name.ends_with('/') {
            parts.insert(expected.name.clone(), content);
        }
    }
    Ok(parts)
}
