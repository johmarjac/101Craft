//! Value interpretation, searching, checksums and comparing.

use md5::Digest;

use crate::i18n::{self, Lang};

use crate::tr;

// ---------------------------------------------------------------- inspector

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InspKind {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
    Bin,
    Char,
    Utf16,
    DosDate,
    DosTime,
    UnixTime,
    FileTime,
}

pub const INSPECTOR_ROWS: &[(InspKind, &str)] = &[
    (InspKind::I8, "insp.i8"),
    (InspKind::U8, "insp.u8"),
    (InspKind::I16, "insp.i16"),
    (InspKind::U16, "insp.u16"),
    (InspKind::I32, "insp.i32"),
    (InspKind::U32, "insp.u32"),
    (InspKind::I64, "insp.i64"),
    (InspKind::U64, "insp.u64"),
    (InspKind::F32, "insp.f32"),
    (InspKind::F64, "insp.f64"),
    (InspKind::Bin, "insp.bin"),
    (InspKind::Char, "insp.char"),
    (InspKind::Utf16, "insp.utf16"),
    (InspKind::DosDate, "insp.dosdate"),
    (InspKind::DosTime, "insp.dostime"),
    (InspKind::UnixTime, "insp.unixtime"),
    (InspKind::FileTime, "insp.filetime"),
];

impl InspKind {
    pub fn size(self) -> usize {
        match self {
            InspKind::I8 | InspKind::U8 | InspKind::Bin | InspKind::Char => 1,
            InspKind::I16 | InspKind::U16 | InspKind::Utf16 | InspKind::DosDate | InspKind::DosTime => 2,
            InspKind::I32 | InspKind::U32 | InspKind::F32 | InspKind::UnixTime => 4,
            InspKind::I64 | InspKind::U64 | InspKind::F64 | InspKind::FileTime => 8,
        }
    }
    pub fn editable(self) -> bool {
        !matches!(
            self,
            InspKind::DosDate | InspKind::DosTime | InspKind::UnixTime | InspKind::FileTime | InspKind::Utf16
        )
    }
}

fn read_u64(data: &[u8], pos: usize, n: usize, le: bool) -> Option<u64> {
    let b = data.get(pos..pos + n)?;
    let mut buf = [0u8; 8];
    buf[..n].copy_from_slice(b);
    if !le {
        buf[..n].reverse();
    }
    Some(u64::from_le_bytes(buf))
}

/// Days since 1970-01-01 to (year, month, day).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn fmt_unix(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}",
        rem / 3600,
        (rem / 60) % 60,
        rem % 60
    )
}

/// Short, readable float text: plain for ordinary sizes, scientific otherwise.
pub fn fmt_float(f: f64, digits: usize) -> String {
    if f.is_nan() || f.is_infinite() {
        return format!("{f}");
    }
    let a = f.abs();
    if a == 0.0 || (1e-4..1e9).contains(&a) {
        let s = format!("{f:.digits$}");
        let s = if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s };
        if s.len() > 24 { format!("{f:.6e}") } else { s }
    } else {
        format!("{:.*e}", digits.min(8) - 2, f)
    }
}

pub fn inspect(data: &[u8], pos: usize, kind: InspKind, le: bool) -> String {
    let Some(u) = read_u64(data, pos, kind.size(), le) else {
        return "—".into();
    };
    match kind {
        InspKind::I8 => (u as u8 as i8).to_string(),
        InspKind::U8 => (u as u8).to_string(),
        InspKind::I16 => (u as u16 as i16).to_string(),
        InspKind::U16 => (u as u16).to_string(),
        InspKind::I32 => (u as u32 as i32).to_string(),
        InspKind::U32 => (u as u32).to_string(),
        InspKind::I64 => (u as i64).to_string(),
        InspKind::U64 => u.to_string(),
        InspKind::F32 => fmt_float(f32::from_bits(u as u32) as f64, 7),
        InspKind::F64 => fmt_float(f64::from_bits(u), 15),
        InspKind::Bin => format!("{:08b}", u as u8),
        InspKind::Char => {
            let c = u as u8;
            if (0x20..0x7f).contains(&c) {
                format!("'{}'", c as char)
            } else {
                format!("\\x{c:02X}")
            }
        }
        InspKind::Utf16 => match char::from_u32(u as u32 & 0xFFFF) {
            Some(c) if !c.is_control() => format!("'{c}'"),
            _ => format!("U+{:04X}", u as u16),
        },
        InspKind::DosDate => {
            let v = u as u16;
            format!(
                "{:04}-{:02}-{:02}",
                1980 + (v >> 9) as u32,
                (v >> 5) & 0x0F,
                v & 0x1F
            )
        }
        InspKind::DosTime => {
            let v = u as u16;
            format!("{:02}:{:02}:{:02}", v >> 11, (v >> 5) & 0x3F, (v & 0x1F) * 2)
        }
        InspKind::UnixTime => fmt_unix(u as u32 as i64),
        InspKind::FileTime => {
            if u == 0 {
                "—".into()
            } else {
                let secs = (u / 10_000_000) as i64 - 11_644_473_600;
                fmt_unix(secs)
            }
        }
    }
}

/// Parse a value typed into the inspector back into bytes.
pub fn encode(text: &str, kind: InspKind, le: bool) -> Option<Vec<u8>> {
    let t = text.trim();
    let int = || -> Option<i128> {
        let (neg, body) = match t.strip_prefix('-') {
            Some(r) => (true, r),
            None => (false, t),
        };
        let v = if let Some(h) = body.strip_prefix("0x").or_else(|| body.strip_prefix("0X")) {
            i128::from_str_radix(h, 16).ok()?
        } else {
            body.parse::<i128>().ok()?
        };
        Some(if neg { -v } else { v })
    };
    let n = kind.size();
    let bits: u64 = match kind {
        InspKind::I8 | InspKind::I16 | InspKind::I32 | InspKind::I64 => {
            let v = int()?;
            let lim = 1i128 << (n * 8 - 1);
            if v < -lim || v >= lim {
                return None;
            }
            v as i64 as u64
        }
        InspKind::U8 | InspKind::U16 | InspKind::U32 | InspKind::U64 => {
            let v = int()?;
            if v < 0 || v >= (1i128 << (n * 8)) {
                return None;
            }
            v as u64
        }
        InspKind::F32 => (t.replace(',', ".").parse::<f32>().ok()?).to_bits() as u64,
        InspKind::F64 => (t.replace(',', ".").parse::<f64>().ok()?).to_bits(),
        InspKind::Bin => u8::from_str_radix(t, 2).ok()? as u64,
        InspKind::Char => {
            let s = t.trim_matches('\'');
            let c = s.chars().next()?;
            if (c as u32) > 255 {
                return None;
            }
            c as u64
        }
        _ => return None,
    };
    let mut b = bits.to_le_bytes()[..n].to_vec();
    if !le {
        b.reverse();
    }
    Some(b)
}

// ---------------------------------------------------------------- search

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SearchKind {
    Text,
    TextUtf16,
    Hex,
    Int32,
}

impl SearchKind {
    pub fn label(self, lang: Lang) -> String {
        match self {
            SearchKind::Text => i18n::get(lang, "search.text"),
            SearchKind::TextUtf16 => i18n::get(lang, "search.utf16"),
            SearchKind::Hex => i18n::get(lang, "search.hex"),
            SearchKind::Int32 => i18n::get(lang, "search.int32"),
        }
    }
}

pub fn parse_hex(s: &str) -> Option<Vec<u8>> {
    let clean: String = s
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ',' && *c != '-')
        .collect();
    let clean = clean.trim_start_matches("0x");
    if clean.is_empty() || clean.len() % 2 != 0 {
        return None;
    }
    (0..clean.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&clean[i..i + 2], 16).ok())
        .collect()
}

pub fn pattern(text: &str, kind: SearchKind, le: bool) -> Option<Vec<u8>> {
    if text.is_empty() {
        return None;
    }
    match kind {
        SearchKind::Text => Some(text.as_bytes().to_vec()),
        SearchKind::TextUtf16 => Some(
            text.encode_utf16()
                .flat_map(|u| if le { u.to_le_bytes() } else { u.to_be_bytes() })
                .collect(),
        ),
        SearchKind::Hex => parse_hex(text),
        SearchKind::Int32 => encode(text, InspKind::I32, le).or_else(|| encode(text, InspKind::U32, le)),
    }
}

fn eq_byte(a: u8, b: u8, ci: bool) -> bool {
    if ci {
        a.eq_ignore_ascii_case(&b)
    } else {
        a == b
    }
}

fn matches_at(data: &[u8], i: usize, pat: &[u8], ci: bool) -> bool {
    data.len() >= i + pat.len() && pat.iter().enumerate().all(|(k, p)| eq_byte(data[i + k], *p, ci))
}

pub fn find_next(data: &[u8], pat: &[u8], from: usize, ci: bool) -> Option<usize> {
    if pat.is_empty() || data.len() < pat.len() {
        return None;
    }
    let last = data.len() - pat.len();
    (from..=last)
        .chain(0..from.min(last + 1))
        .find(|&i| matches_at(data, i, pat, ci))
}

pub fn find_prev(data: &[u8], pat: &[u8], from: usize, ci: bool) -> Option<usize> {
    if pat.is_empty() || data.len() < pat.len() {
        return None;
    }
    let last = data.len() - pat.len();
    let start = from.min(last + 1);
    (0..start)
        .rev()
        .chain((start..=last).rev())
        .find(|&i| matches_at(data, i, pat, ci))
}

pub fn find_all(data: &[u8], pat: &[u8], ci: bool, limit: usize) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    if pat.is_empty() || data.len() < pat.len() {
        return out;
    }
    let mut i = 0;
    while i + pat.len() <= data.len() {
        if matches_at(data, i, pat, ci) {
            out.push((i, pat.len()));
            if out.len() >= limit {
                break;
            }
            i += pat.len();
        } else {
            i += 1;
        }
    }
    out
}

// ---------------------------------------------------------------- checksums

pub const CHECKSUMS: &[(&str, &str)] = &[
    ("CRC-32", "checksum.crc32"),
    ("Adler-32", "checksum.adler32"),
    ("Sum (8-bit)", "checksum.sum8"),
    ("Sum (16-bit)", "checksum.sum16"),
    ("Sum (32-bit)", "checksum.sum32"),
    ("XOR (8-bit)", "checksum.xor8"),
    ("MD5", "checksum.md5"),
    ("SHA-1", "checksum.sha1"),
    ("SHA-256", "checksum.sha256"),
];

pub fn checksum(name: &str, data: &[u8]) -> String {
    match name {
        "CRC-32" => {
            let mut h = crc32fast::Hasher::new();
            h.update(data);
            format!("{:08X}", h.finalize())
        }
        "Adler-32" => {
            let (mut a, mut b) = (1u32, 0u32);
            for chunk in data.chunks(5552) {
                for &x in chunk {
                    a += x as u32;
                    b += a;
                }
                a %= 65_521;
                b %= 65_521;
            }
            format!("{:08X}", (b << 16) | a)
        }
        "Sum (8-bit)" => format!("{:02X}", data.iter().fold(0u8, |s, &x| s.wrapping_add(x))),
        "Sum (16-bit)" => format!("{:04X}", data.iter().fold(0u16, |s, &x| s.wrapping_add(x as u16))),
        "Sum (32-bit)" => format!("{:08X}", data.iter().fold(0u32, |s, &x| s.wrapping_add(x as u32))),
        "XOR (8-bit)" => format!("{:02X}", data.iter().fold(0u8, |s, &x| s ^ x)),
        "MD5" => hex(&md5::Md5::digest(data)),
        "SHA-1" => hex(&sha1::Sha1::digest(data)),
        "SHA-256" => hex(&sha2::Sha256::digest(data)),
        _ => String::new(),
    }
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02X}")).collect()
}

pub fn hex_spaced(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02X}")).collect::<Vec<_>>().join(" ")
}

// ---------------------------------------------------------------- compare

#[derive(Clone, Debug)]
pub struct DiffRange {
    pub start: usize,
    pub len_a: usize,
    pub len_b: usize,
}

/// Byte-by-byte comparison; trailing extra bytes count as one difference.
pub fn compare(a: &[u8], b: &[u8], limit: usize) -> Vec<DiffRange> {
    let mut out = Vec::new();
    let n = a.len().min(b.len());
    let mut i = 0;
    while i < n {
        if a[i] != b[i] {
            let s = i;
            while i < n && a[i] != b[i] {
                i += 1;
            }
            out.push(DiffRange {
                start: s,
                len_a: i - s,
                len_b: i - s,
            });
            if out.len() >= limit {
                return out;
            }
        } else {
            i += 1;
        }
    }
    if a.len() != b.len() {
        out.push(DiffRange {
            start: n,
            len_a: a.len() - n,
            len_b: b.len() - n,
        });
    }
    out
}

pub fn human_size(n: usize, lang: Lang) -> String {
    if n < 1024 {
        tr!(lang, "unit.bytes", n = n)
    } else if n < 1024 * 1024 {
        format!("{:.1} KB", n as f64 / 1024.0)
    } else if n < 1024 * 1024 * 1024 {
        format!("{:.1} MB", n as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", n as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// Parse an address like "0x1A0", "1A0h", "416" or "+16" (relative).
pub fn parse_offset(s: &str, current: usize, hex_default: bool) -> Option<usize> {
    let t = s.trim().replace(['_', ' '], "");
    let (rel, body) = if let Some(r) = t.strip_prefix('+') {
        (1i64, r.to_string())
    } else if let Some(r) = t.strip_prefix('-') {
        (-1, r.to_string())
    } else {
        (0, t.clone())
    };
    let v = if let Some(h) = body.strip_prefix("0x").or_else(|| body.strip_prefix("0X")) {
        u64::from_str_radix(h, 16).ok()?
    } else if let Some(h) = body.strip_suffix('h').or_else(|| body.strip_suffix('H')) {
        u64::from_str_radix(h, 16).ok()?
    } else if hex_default {
        u64::from_str_radix(&body, 16).ok()?
    } else {
        body.parse::<u64>().ok()?
    } as i64;
    Some(match rel {
        0 => v.max(0) as usize,
        r => (current as i64 + r * v).max(0) as usize,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums() {
        assert_eq!(checksum("CRC-32", b"123456789"), "CBF43926");
        assert_eq!(checksum("Adler-32", b"Wikipedia"), "11E60398");
        assert_eq!(checksum("MD5", b""), "D41D8CD98F00B204E9800998ECF8427E");
    }

    #[test]
    fn values() {
        let d = [0x78, 0x56, 0x34, 0x12];
        assert_eq!(inspect(&d, 0, InspKind::U32, true), "305419896");
        assert_eq!(encode("0x12345678", InspKind::U32, true).unwrap(), d.to_vec());
        assert_eq!(fmt_unix(0), "1970-01-01 00:00:00");
    }

    #[test]
    fn search() {
        let d = b"abcABCabc";
        assert_eq!(find_all(d, b"abc", false, 10).len(), 2);
        assert_eq!(find_all(d, b"abc", true, 10).len(), 3);
        assert_eq!(find_next(d, b"abc", 1, false), Some(6));
        assert_eq!(find_prev(d, b"abc", 6, false), Some(0));
        assert_eq!(parse_offset("0x10", 0, false), Some(16));
        assert_eq!(parse_offset("+4", 10, false), Some(14));
    }
}
