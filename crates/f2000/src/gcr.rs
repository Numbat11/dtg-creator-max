// SPDX-License-Identifier: GPL-3.0-or-later
//! Garment Creator job files (`.gcr`): the artwork PNG plus the job settings.
//!
//! Layout: five u32 LE values (version 1, 0, entry count 2, length of everything after the
//! fourth value, PNG length), then the PNG, then the settings as URL-encoded JSON running to
//! the end of the file.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gcr {
    pub version: u32,
    pub entries: u32,
    pub artwork_png: Vec<u8>,
    /// The settings JSON, percent-decoded.
    pub settings_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GcrError(pub String);

impl fmt::Display for GcrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not a Garment Creator job file: {}", self.0)
    }
}

impl std::error::Error for GcrError {}

pub fn parse(d: &[u8]) -> Result<Gcr, GcrError> {
    let u = |i: usize| -> Result<u32, GcrError> {
        d.get(i * 4..i * 4 + 4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
            .ok_or_else(|| GcrError("file too short".into()))
    };
    let (version, entries, rest_len, png_len) = (u(0)?, u(2)?, u(3)? as usize, u(4)? as usize);
    if rest_len != d.len().saturating_sub(16) {
        return Err(GcrError(format!("length field {rest_len} doesn't match the file size {}", d.len())));
    }
    let png = d.get(20..20 + png_len).ok_or_else(|| GcrError("artwork runs past the end".into()))?;
    Ok(Gcr {
        version,
        entries,
        artwork_png: png.to_vec(),
        settings_json: percent_decode(&d[20 + png_len..]),
    })
}

fn percent_decode(s: &[u8]) -> String {
    let hex = |b: u8| (b as char).to_digit(16);
    let mut out = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        if s[i] == b'%' && i + 2 < s.len() {
            if let (Some(h), Some(l)) = (hex(s[i + 1]), hex(s[i + 2])) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(s[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

impl Gcr {
    /// Top-level settings as (key, raw JSON value) pairs, in file order.
    pub fn settings(&self) -> Vec<(String, String)> {
        top_level_pairs(&self.settings_json)
    }

    pub fn setting(&self, key: &str) -> Option<String> {
        self.settings().into_iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
}

/// Split a JSON object into its top-level members without a full JSON parser. Values are
/// returned as their raw JSON text; string values keep their quotes.
fn top_level_pairs(json: &str) -> Vec<(String, String)> {
    let body = json.trim();
    let Some(body) = body.strip_prefix('{').and_then(|b| b.strip_suffix('}')) else { return Vec::new() };
    let mut members = Vec::new();
    let (mut depth, mut in_str, mut esc, mut start) = (0i32, false, false, 0usize);
    for (i, ch) in body.char_indices() {
        if in_str {
            match (esc, ch) {
                (true, _) => esc = false,
                (false, '\\') => esc = true,
                (false, '"') => in_str = false,
                _ => {}
            }
            continue;
        }
        match ch {
            '"' => in_str = true,
            '{' | '[' => depth += 1,
            '}' | ']' => depth -= 1,
            ',' if depth == 0 => {
                members.push(&body[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    members.push(&body[start..]);
    members
        .into_iter()
        .filter_map(|m| {
            let (k, v) = split_member(m)?;
            Some((k.trim().trim_matches('"').to_string(), v.trim().to_string()))
        })
        .collect()
}

fn split_member(m: &str) -> Option<(&str, &str)> {
    // the key is a string: find its closing quote, then the colon after it
    let m = m.trim_start();
    let rest = m.strip_prefix('"')?;
    let mut esc = false;
    for (i, ch) in rest.char_indices() {
        match (esc, ch) {
            (true, _) => esc = false,
            (false, '\\') => esc = true,
            (false, '"') => {
                let after = &rest[i + 1..];
                let colon = after.find(':')?;
                return Some((&rest[..i], &after[colon + 1..]));
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_small_job() {
        let settings = b"%7B%22inkSet%22%3A4%2C%22name%22%3A%22a%2Cb%22%2C%22list%22%3A%5B1%2C2%5D%7D";
        let png = b"\x89PNG\r\n\x1a\nxx";
        let mut d = Vec::new();
        for v in [1u32, 0, 2, (4 + png.len() + settings.len()) as u32, png.len() as u32] {
            d.extend_from_slice(&v.to_le_bytes());
        }
        d.extend_from_slice(png);
        d.extend_from_slice(settings);
        let g = parse(&d).unwrap();
        assert_eq!(g.artwork_png, png);
        assert_eq!(g.settings_json, r#"{"inkSet":4,"name":"a,b","list":[1,2]}"#);
        assert_eq!(
            g.settings(),
            vec![
                ("inkSet".to_string(), "4".to_string()),
                ("name".to_string(), "\"a,b\"".to_string()),
                ("list".to_string(), "[1,2]".to_string()),
            ]
        );
    }
}
