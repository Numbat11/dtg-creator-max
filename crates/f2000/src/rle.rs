// SPDX-License-Identifier: GPL-3.0-or-later
//! Epson "TIFF mode" run-length coding, used by `ESC i` raster modes 1 and 5.
//!
//! A count byte `n < 0x80` is followed by `n + 1` literal bytes. A count byte `n >= 0x80` is
//! followed by one byte that repeats `257 - n` times, so `0x80` means 129 repeats (PackBits
//! would treat it as a no-op).
//!
//! [`encode_line`] reproduces Garment Creator 2.1.7's encoder exactly. That was checked
//! against all 8,091 strips in three real jobs:
//! - each raster line is coded on its own (no token crosses a line boundary);
//! - runs of 3 or more identical bytes become a repeat token of at most 128;
//! - everything else is literal, at most 127 bytes per token;
//! - the scan is greedy from left to right.

use std::fmt;

/// Longest literal token Garment Creator writes (the format allows 128).
pub const MAX_LITERAL: usize = 127;
/// Longest repeat token Garment Creator writes (the format allows 129).
pub const MAX_REPEAT: usize = 128;
/// Shortest run Garment Creator codes as a repeat; runs of 2 stay literal.
pub const MIN_REPEAT: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RleError {
    /// The input ended inside a token, at this input offset.
    Truncated { at: usize },
    /// The last token produced more bytes than wanted.
    Overrun { want: usize, got: usize },
}

impl fmt::Display for RleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RleError::Truncated { at } => write!(f, "RLE data ends inside a token at offset {at}"),
            RleError::Overrun { want, got } => {
                write!(f, "RLE data decodes to {got} bytes where {want} were expected")
            }
        }
    }
}

impl std::error::Error for RleError {}

/// Decode until exactly `want` bytes come out. Returns the bytes and how much input was used.
pub fn decode(input: &[u8], want: usize) -> Result<(Vec<u8>, usize), RleError> {
    let mut out = Vec::with_capacity(want);
    let used = decode_into(input, want, &mut out)?;
    Ok((out, used))
}

/// Like [`decode`], appending to `out`. Returns how much input was used.
pub fn decode_into(input: &[u8], want: usize, out: &mut Vec<u8>) -> Result<usize, RleError> {
    let start = out.len();
    let mut i = 0;
    while out.len() - start < want {
        let n = *input.get(i).ok_or(RleError::Truncated { at: i })? as usize;
        if n < 0x80 {
            let lit = input.get(i + 1..i + 2 + n).ok_or(RleError::Truncated { at: i })?;
            out.extend_from_slice(lit);
            i += 2 + n;
        } else {
            let b = *input.get(i + 1).ok_or(RleError::Truncated { at: i })?;
            out.resize(out.len() + 257 - n, b);
            i += 2;
        }
    }
    let got = out.len() - start;
    if got != want {
        return Err(RleError::Overrun { want, got });
    }
    Ok(i)
}

/// Encode one raster line exactly as Garment Creator 2.1.7 does, appending to `out`.
pub fn encode_line(line: &[u8], out: &mut Vec<u8>) {
    let n = line.len();
    let mut i = 0;
    let mut lit_start = 0;
    while i < n {
        let b = line[i];
        let mut j = i + 1;
        while j < n && line[j] == b && j - i < MAX_REPEAT {
            j += 1;
        }
        let run = j - i;
        if run >= MIN_REPEAT {
            push_literals(&line[lit_start..i], out);
            out.push((257 - run) as u8);
            out.push(b);
            i = j;
            lit_start = i;
        } else {
            i += 1;
        }
    }
    push_literals(&line[lit_start..], out);
}

/// Encode a strip of `raw.len() / bytes_per_line` lines, each line on its own.
pub fn encode_strip(raw: &[u8], bytes_per_line: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len() / 4);
    if bytes_per_line == 0 {
        return out;
    }
    for line in raw.chunks(bytes_per_line) {
        encode_line(line, &mut out);
    }
    out
}

fn push_literals(mut lit: &[u8], out: &mut Vec<u8>) {
    while !lit.is_empty() {
        let k = lit.len().min(MAX_LITERAL);
        out.push((k - 1) as u8);
        out.extend_from_slice(&lit[..k]);
        lit = &lit[k..];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enc(line: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        encode_line(line, &mut out);
        out
    }

    #[test]
    fn decodes_0x80_as_129_repeats() {
        let (out, used) = decode(&[0x80, 0xAB], 129).unwrap();
        assert_eq!(out, vec![0xAB; 129]);
        assert_eq!(used, 2);
    }

    #[test]
    fn decodes_literals_and_repeats() {
        let (out, used) = decode(&[0x02, 1, 2, 3, 0xFE, 9, 0xFF], 6).unwrap();
        assert_eq!(out, vec![1, 2, 3, 9, 9, 9]);
        assert_eq!(used, 6); // the trailing byte is not consumed
    }

    #[test]
    fn decode_reports_truncation_and_overrun() {
        assert_eq!(decode(&[0x05, 1, 2], 6), Err(RleError::Truncated { at: 0 }));
        assert_eq!(decode(&[0xFE, 7], 2), Err(RleError::Overrun { want: 2, got: 3 }));
    }

    #[test]
    fn runs_of_two_stay_literal() {
        assert_eq!(enc(&[5, 5, 6]), vec![0x02, 5, 5, 6]);
        assert_eq!(enc(&[5, 5, 5, 6]), vec![0xFE, 5, 0x00, 6]);
    }

    #[test]
    fn repeats_split_at_128_and_short_tails_go_literal() {
        let mut line = vec![0xFF; 130];
        line.push(0x3C);
        // 128 repeats (0x81), then the 2 left over plus the next byte as a 3-byte literal
        assert_eq!(enc(&line), vec![0x81, 0xFF, 0x02, 0xFF, 0xFF, 0x3C]);
    }

    #[test]
    fn literals_split_at_127() {
        let line: Vec<u8> = (0..200u32).map(|v| (v % 2) as u8 + (v % 7) as u8 * 3).collect();
        let out = enc(&line);
        assert_eq!(out[0], 126); // 127 literals
        assert_eq!(out[128], 200 - 127 - 1);
        assert_eq!(decode(&out, 200).unwrap().0, line);
    }

    #[test]
    fn encoder_round_trips_pseudo_random_lines() {
        let mut state = 0x2545_F491u32;
        for len in [1usize, 2, 3, 17, 255, 256, 1000, 4364] {
            let line: Vec<u8> = (0..len)
                .map(|_| {
                    state ^= state << 13;
                    state ^= state >> 17;
                    state ^= state << 5;
                    // mostly long runs of a few values, like halftoned dots
                    [0x00, 0xFF, 0xFF, 0xCF, 0x3F][(state >> 7) as usize % 5]
                        ^ if state % 11 == 0 { 0x30 } else { 0 }
                })
                .collect();
            let out = enc(&line);
            assert_eq!(decode(&out, len).unwrap(), (line, out.len()));
        }
    }

    #[test]
    fn strips_are_coded_per_line() {
        // two lines of 4 identical bytes must not merge into one 8-byte repeat
        assert_eq!(encode_strip(&[7; 8], 4), vec![0xFD, 7, 0xFD, 7]);
    }
}
