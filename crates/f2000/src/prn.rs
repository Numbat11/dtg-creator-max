// SPDX-License-Identifier: GPL-3.0-or-later
//! Lossless reader and writer for the F2000 `.prn` command stream.
//!
//! [`parse`] splits a file into [`Item`]s and [`write`] puts them back. For every reference
//! file the round trip is byte for byte identical, so nothing in the file is skipped or
//! normalised. See `docs/print-file-format.md` at the repository root for what each part means.

use crate::rle::{self, RleError};
use std::fmt;

pub const ESC: u8 = 0x1B;
pub const CR: u8 = 0x0D;
pub const LF: u8 = 0x0A;
pub const FF: u8 = 0x0C;
pub const NUL: u8 = 0x00;
/// Padding byte at the very start of a job.
pub const LEAD_IN: u8 = 0x11;

const REMOTE_END: &[u8] = &[ESC, 0, 0, 0];

/// One element of the command stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    /// `count` bytes of 0x11 at the start of the file.
    LeadIn(usize),
    /// A lone control byte: NUL, CR, LF or FF (FF ends a page).
    Ctl(u8),
    /// `ESC 0x01`: the "exit packet mode" preamble before the EJL lines.
    ExitPacketMode,
    /// One `@EJL ...` line, without its trailing LF.
    Ejl(Vec<u8>),
    /// `ESC @`: reset.
    Reset,
    /// `ESC ( R nL nH 00 "REMOTE1"`, then commands, then `ESC 00 00 00`.
    Remote { header: Vec<u8>, cmds: Vec<RemoteCmd> },
    /// `ESC ( R nL nH 00 "THUMBNAIL"`, then a u32 LE length and that many bytes of entries.
    Thumbnail { header: Vec<u8>, body: Vec<u8> },
    /// Any other `ESC ( c nL nH <data>` command, e.g. `(U`, `(D`, `(v`, `($`.
    Command { cmd: u8, data: Vec<u8> },
    /// `ESC U n` and `ESC r n` (one data byte), `ESC $` and `ESC \` (two data bytes).
    Short { cmd: u8, data: Vec<u8> },
    /// `ESC i`: one strip of raster data.
    Raster(Strip),
}

/// One remote-mode command: a two-letter code and its data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteCmd {
    pub code: [u8; 2],
    pub data: Vec<u8>,
}

impl RemoteCmd {
    pub fn code_str(&self) -> String {
        String::from_utf8_lossy(&self.code).into_owned()
    }
}

/// `ESC i <channel> <mode> <bits> <bytes/line u16> <lines u16>` and its data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Strip {
    /// Ink channel: 0 black, 1 magenta, 2 cyan, 4 yellow, 63 white.
    pub channel: u8,
    /// 0 raw, 1 bare RLE, 4 framed raw, 5 framed RLE.
    pub mode: u8,
    /// Bits per pixel (2: four dot sizes, 0 = none).
    pub bits: u8,
    pub bytes_per_line: u16,
    pub lines: u16,
    /// Framing byte before the u32 length in modes 4 and 5 (always 0 so far).
    pub flag: Option<u8>,
    /// The data as stored: RLE for modes 1 and 5, raw for 0 and 4.
    pub payload: Vec<u8>,
}

impl Strip {
    pub fn raw_len(&self) -> usize {
        self.bytes_per_line as usize * self.lines as usize
    }

    /// The uncompressed dot data, `bytes_per_line * lines` bytes.
    pub fn raw(&self) -> Result<Vec<u8>, RleError> {
        match self.mode {
            1 | 5 => rle::decode(&self.payload, self.raw_len()).map(|(v, _)| v),
            _ => Ok(self.payload.clone()),
        }
    }

    /// Build a strip from dot data the way Garment Creator 2.1.7 does: mode 5 (framed RLE),
    /// or mode 4 (framed raw) when compressing would make it bigger.
    pub fn compress(channel: u8, bits: u8, bytes_per_line: u16, lines: u16, raw: &[u8]) -> Strip {
        debug_assert_eq!(raw.len(), bytes_per_line as usize * lines as usize);
        let packed = rle::encode_strip(raw, bytes_per_line as usize);
        // Seen: mode 4 only when RLE is larger. A tie has not been seen yet.
        let (mode, payload) = if packed.len() > raw.len() { (4, raw.to_vec()) } else { (5, packed) };
        Strip { channel, mode, bits, bytes_per_line, lines, flag: Some(0), payload }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    UnexpectedByte { at: usize, byte: u8 },
    UnknownEsc { at: usize, byte: u8 },
    UnknownRasterMode { at: usize, mode: u8 },
    Truncated { at: usize, what: &'static str },
    Rle { at: usize, err: RleError },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnexpectedByte { at, byte } => {
                write!(f, "unexpected byte 0x{byte:02x} at offset {at}")
            }
            ParseError::UnknownEsc { at, byte } => {
                write!(f, "unknown command ESC 0x{byte:02x} at offset {at}")
            }
            ParseError::UnknownRasterMode { at, mode } => {
                write!(f, "unknown raster mode {mode} at offset {at}")
            }
            ParseError::Truncated { at, what } => write!(f, "file ends inside {what} at offset {at}"),
            ParseError::Rle { at, err } => write!(f, "bad raster data at offset {at}: {err}"),
        }
    }
}

impl std::error::Error for ParseError {}

fn get<'a>(d: &'a [u8], from: usize, len: usize, what: &'static str) -> Result<&'a [u8], ParseError> {
    d.get(from..from + len).ok_or(ParseError::Truncated { at: from, what })
}

fn u16_at(d: &[u8], at: usize, what: &'static str) -> Result<u16, ParseError> {
    let b = get(d, at, 2, what)?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}

fn u32_at(d: &[u8], at: usize, what: &'static str) -> Result<u32, ParseError> {
    let b = get(d, at, 4, what)?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Split a whole `.prn` file into items.
pub fn parse(d: &[u8]) -> Result<Vec<Item>, ParseError> {
    let mut items = Vec::new();
    let lead = d.iter().take_while(|&&b| b == LEAD_IN).count();
    if lead > 0 {
        items.push(Item::LeadIn(lead));
    }
    let mut i = lead;
    while i < d.len() {
        if d[i..].starts_with(b"@EJL") {
            let end = d[i..].iter().position(|&b| b == LF).ok_or(ParseError::Truncated { at: i, what: "an EJL line" })?;
            items.push(Item::Ejl(d[i..i + end].to_vec()));
            i += end + 1;
            continue;
        }
        let b = d[i];
        if matches!(b, NUL | CR | LF | FF) {
            items.push(Item::Ctl(b));
            i += 1;
            continue;
        }
        if b != ESC {
            return Err(ParseError::UnexpectedByte { at: i, byte: b });
        }
        let c = *d.get(i + 1).ok_or(ParseError::Truncated { at: i, what: "an ESC command" })?;
        match c {
            0x01 => {
                items.push(Item::ExitPacketMode);
                i += 2;
            }
            b'@' => {
                items.push(Item::Reset);
                i += 2;
            }
            b'(' => {
                let cmd = get(d, i + 2, 1, "an ESC ( command")?[0];
                let n = u16_at(d, i + 3, "an ESC ( command")? as usize;
                let data = get(d, i + 5, n, "an ESC ( command")?;
                let mut j = i + 5 + n;
                if cmd == b'R' && data.get(1..8) == Some(b"REMOTE1".as_slice()) {
                    let mut cmds = Vec::new();
                    while !d[j..].starts_with(REMOTE_END) {
                        let code = get(d, j, 2, "a remote command")?;
                        let ln = u16_at(d, j + 2, "a remote command")? as usize;
                        let body = get(d, j + 4, ln, "a remote command")?;
                        cmds.push(RemoteCmd { code: [code[0], code[1]], data: body.to_vec() });
                        j += 4 + ln;
                    }
                    items.push(Item::Remote { header: data.to_vec(), cmds });
                    i = j + REMOTE_END.len();
                } else if cmd == b'R' && data.get(1..) == Some(b"THUMBNAIL".as_slice()) {
                    let total = u32_at(d, j, "a thumbnail")? as usize;
                    let body = get(d, j + 4, total, "a thumbnail")?;
                    items.push(Item::Thumbnail { header: data.to_vec(), body: body.to_vec() });
                    i = j + 4 + total;
                } else {
                    items.push(Item::Command { cmd, data: data.to_vec() });
                    i = j;
                }
            }
            b'i' => {
                let h = get(d, i + 2, 7, "a raster header")?;
                let (channel, mode, bits) = (h[0], h[1], h[2]);
                let bytes_per_line = u16::from_le_bytes([h[3], h[4]]);
                let lines = u16::from_le_bytes([h[5], h[6]]);
                let body = i + 9;
                let want = bytes_per_line as usize * lines as usize;
                let (flag, payload, used) = match mode {
                    0 => (None, get(d, body, want, "raw raster data")?, want),
                    1 => {
                        let used = rle::decode(&d[body..], want)
                            .map_err(|err| ParseError::Rle { at: body, err })?
                            .1;
                        (None, &d[body..body + used], used)
                    }
                    4 | 5 => {
                        let flag = get(d, body, 1, "a raster frame")?[0];
                        let len = u32_at(d, body + 1, "a raster frame")? as usize;
                        (Some(flag), get(d, body + 5, len, "raster data")?, 5 + len)
                    }
                    _ => return Err(ParseError::UnknownRasterMode { at: i, mode }),
                };
                items.push(Item::Raster(Strip {
                    channel,
                    mode,
                    bits,
                    bytes_per_line,
                    lines,
                    flag,
                    payload: payload.to_vec(),
                }));
                i = body + used;
            }
            b'U' | b'r' => {
                items.push(Item::Short { cmd: c, data: get(d, i + 2, 1, "an ESC command")?.to_vec() });
                i += 3;
            }
            b'$' | b'\\' => {
                items.push(Item::Short { cmd: c, data: get(d, i + 2, 2, "an ESC command")?.to_vec() });
                i += 4;
            }
            _ => return Err(ParseError::UnknownEsc { at: i, byte: c }),
        }
    }
    Ok(items)
}

/// Serialise items back into `.prn` bytes.
pub fn write(items: &[Item]) -> Vec<u8> {
    let mut out = Vec::new();
    for item in items {
        write_item(item, &mut out);
    }
    out
}

pub fn write_item(item: &Item, out: &mut Vec<u8>) {
    match item {
        Item::LeadIn(n) => out.resize(out.len() + n, LEAD_IN),
        Item::Ctl(b) => out.push(*b),
        Item::ExitPacketMode => out.extend_from_slice(&[ESC, 0x01]),
        Item::Ejl(line) => {
            out.extend_from_slice(line);
            out.push(LF);
        }
        Item::Reset => out.extend_from_slice(&[ESC, b'@']),
        Item::Remote { header, cmds } => {
            paren(out, b'R', header);
            for c in cmds {
                out.extend_from_slice(&c.code);
                out.extend_from_slice(&(c.data.len() as u16).to_le_bytes());
                out.extend_from_slice(&c.data);
            }
            out.extend_from_slice(REMOTE_END);
        }
        Item::Thumbnail { header, body } => {
            paren(out, b'R', header);
            out.extend_from_slice(&(body.len() as u32).to_le_bytes());
            out.extend_from_slice(body);
        }
        Item::Command { cmd, data } => paren(out, *cmd, data),
        Item::Short { cmd, data } => {
            out.extend_from_slice(&[ESC, *cmd]);
            out.extend_from_slice(data);
        }
        Item::Raster(s) => {
            out.extend_from_slice(&[ESC, b'i', s.channel, s.mode, s.bits]);
            out.extend_from_slice(&s.bytes_per_line.to_le_bytes());
            out.extend_from_slice(&s.lines.to_le_bytes());
            if let Some(flag) = s.flag {
                out.push(flag);
                out.extend_from_slice(&(s.payload.len() as u32).to_le_bytes());
            }
            out.extend_from_slice(&s.payload);
        }
    }
}

fn paren(out: &mut Vec<u8>, cmd: u8, data: &[u8]) {
    out.extend_from_slice(&[ESC, b'(', cmd]);
    out.extend_from_slice(&(data.len() as u16).to_le_bytes());
    out.extend_from_slice(data);
}

/// `ESC ( v`: move down by this many vertical units.
pub fn move_down(units: i32) -> Item {
    Item::Command { cmd: b'v', data: units.to_le_bytes().to_vec() }
}

/// `ESC ( $`: set the absolute horizontal position, in horizontal units.
pub fn set_x(x: u32) -> Item {
    Item::Command { cmd: b'$', data: x.to_le_bytes().to_vec() }
}

/// One entry of the thumbnail block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThumbEntry<'a> {
    pub id: u32,
    pub data: &'a [u8],
}

/// Entries of a thumbnail body: (u32 id, u32 length, data) repeated. Stops at the first
/// entry that doesn't fit.
pub fn thumbnail_entries(body: &[u8]) -> Vec<ThumbEntry<'_>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 8 <= body.len() {
        let id = u32::from_le_bytes(body[i..i + 4].try_into().unwrap());
        let len = u32::from_le_bytes(body[i + 4..i + 8].try_into().unwrap()) as usize;
        let Some(data) = body.get(i + 8..i + 8 + len) else { break };
        out.push(ThumbEntry { id, data });
        i += 8 + len;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_stream_round_trips() {
        let mut d = vec![LEAD_IN; 5];
        d.extend_from_slice(b"\0\0\0\x1b\x01@EJL 1284.4\n@EJL\n\x1b@");
        d.extend_from_slice(b"\x1b(R\x08\x00\x00REMOTE1JS\x04\x00\x00\x00\x00\x00\x1b\x00\x00\x00");
        d.extend_from_slice(b"\x1b(R\x0a\x00\x00THUMBNAIL\x0b\x00\x00\x00\x01\x00\x00\x00\x03\x00\x00\x00PNG");
        d.extend_from_slice(b"\x1b(G\x01\x00\x01\x1bU\x00\x1b(v\x04\x00\x10\x00\x00\x00");
        d.extend_from_slice(b"\x1b($\x04\x00\x10\x27\x00\x00");
        // mode 5: 2 bytes x 2 lines, framed RLE
        d.extend_from_slice(b"\x1bi\x3f\x05\x02\x02\x00\x02\x00\x00\x04\x00\x00\x00\xff\xaa\xff\x00\r\x0c");
        let items = parse(&d).unwrap();
        assert_eq!(write(&items), d);
        let Item::Raster(s) = &items[items.len() - 3] else { panic!() };
        assert_eq!(s.raw().unwrap(), vec![0xAA, 0xAA, 0, 0]);
        let body = items
            .iter()
            .find_map(|it| if let Item::Thumbnail { body, .. } = it { Some(body) } else { None })
            .unwrap();
        assert_eq!(thumbnail_entries(body), vec![ThumbEntry { id: 1, data: b"PNG" }]);
    }

    #[test]
    fn compress_falls_back_to_raw_for_specks() {
        // one byte per line: RLE needs 2 bytes per line, so the raw frame wins
        let s = Strip::compress(1, 2, 1, 16, &[0x30; 16]);
        assert_eq!((s.mode, s.payload.len()), (4, 16));
        // a wide solid strip compresses
        let s = Strip::compress(63, 2, 64, 16, &[0xFF; 64 * 16]);
        assert_eq!((s.mode, s.payload.len()), (5, 32));
        assert_eq!(s.raw().unwrap(), vec![0xFF; 64 * 16]);
    }
}
