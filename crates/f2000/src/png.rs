// SPDX-License-Identifier: GPL-3.0-or-later
//! Minimal PNG writing and reading of the IHDR size, enough for previews and thumbnails.

use std::io::{self, Write};

const SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// Encode 8-bit RGB pixels (`width * height * 3` bytes) as a PNG.
pub fn encode_rgb(width: usize, height: usize, rgb: &[u8]) -> Vec<u8> {
    assert_eq!(rgb.len(), width * height * 3);
    let mut raw = Vec::with_capacity(height * (width * 3 + 1));
    for row in rgb.chunks(width * 3) {
        raw.push(0); // filter: none
        raw.extend_from_slice(row);
    }
    let mut out = SIGNATURE.to_vec();
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&(width as u32).to_be_bytes());
    ihdr.extend_from_slice(&(height as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]); // 8-bit, RGB, deflate, no filter, no interlace
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6));
    chunk(&mut out, b"IEND", &[]);
    out
}

pub fn write_rgb(path: &std::path::Path, width: usize, height: usize, rgb: &[u8]) -> io::Result<()> {
    std::fs::File::create(path)?.write_all(&encode_rgb(width, height, rgb))
}

/// Width and height from a PNG's IHDR, if `data` is a PNG.
pub fn size(data: &[u8]) -> Option<(u32, u32)> {
    if !data.starts_with(SIGNATURE) || data.get(12..16) != Some(b"IHDR".as_slice()) {
        return None;
    }
    let w = u32::from_be_bytes(data.get(16..20)?.try_into().ok()?);
    let h = u32::from_be_bytes(data.get(20..24)?.try_into().ok()?);
    Some((w, h))
}

fn chunk(out: &mut Vec<u8>, tag: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(tag);
    out.extend_from_slice(data);
    let crc = crc32(&out[start..]);
    out.extend_from_slice(&crc.to_be_bytes());
}

fn crc32(data: &[u8]) -> u32 {
    const TABLE: [u32; 256] = {
        let mut t = [0u32; 256];
        let mut n = 0;
        while n < 256 {
            let mut c = n as u32;
            let mut k = 0;
            while k < 8 {
                c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
                k += 1;
            }
            t[n] = c;
            n += 1;
        }
        t
    };
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c = TABLE[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_matches_the_standard_check_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn writes_a_png_whose_size_reads_back() {
        let png = encode_rgb(3, 2, &[0; 18]);
        assert_eq!(size(&png), Some((3, 2)));
        // IEND chunk's CRC is fixed
        assert!(png.ends_with(&[0, 0, 0, 0, b'I', b'E', b'N', b'D', 0xAE, 0x42, 0x60, 0x82]));
    }
}
