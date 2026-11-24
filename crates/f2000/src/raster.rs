// SPDX-License-Identifier: GPL-3.0-or-later
//! Dot planes: the printed dots of one page, one 2-bit-per-pixel bitmap per ink channel.
//!
//! [`decode_pages`] places every strip of a parsed job onto full-page planes.
//! [`encode_raster`] does the reverse, turning planes into the strip commands Garment Creator
//! 2.1.7 would write. [`rebuild`] swaps a job's raster commands for re-encoded ones; on the
//! reference jobs the result is byte for byte identical to the original.
//!
//! Garment Creator's raster layout, from the reference jobs:
//! - bands of [`BAND_LINES`] lines on a fixed grid from the top of the page, empty bands
//!   skipped, the last band cut short at the page length;
//! - within a band, channels in [`CHANNEL_ORDER`], skipping channels with no dots there;
//! - `ESC ( v` (relative, from the previous band) before a band's first strip only;
//!   `ESC ( $` (absolute x) before every strip; CR after every strip;
//! - each strip cropped to its dots: the left edge rounded down to 16 pixels, the right
//!   edge at the last byte with dots;
//! - the strip compressed (mode 5) with its width padded to an even number of bytes, unless
//!   that would be larger than the raw data, in which case it is sent raw (mode 4) at its
//!   exact, unpadded width.

use crate::prn::{self, Item, Strip, CR, FF};
use crate::rle::{self, RleError};
use std::collections::BTreeMap;
use std::fmt;

/// Lines per band in Garment Creator 2.1.7's jobs.
pub const BAND_LINES: usize = 16;
/// Left edges of strips are multiples of this many pixels.
pub const X_ALIGN_PX: usize = 16;
/// Order of channels within a band: white, then cyan, magenta, yellow, black. Cyan,
/// magenta, yellow and black are confirmed; no job yet mixes white with colour in one pass.
pub const CHANNEL_ORDER: [u8; 5] = [63, 2, 1, 4, 0];

/// Resolution and size of one page (pass), from `ESC ( U`, `( D`, `( C` and `( S`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PageFormat {
    /// Horizontal dots per inch, e.g. 1440.
    pub hres: u32,
    /// Vertical dots per inch: 1440 for the white pass, 720 for colour.
    pub vres: u32,
    /// Page width in pixels, if the job gave one.
    pub width_px: Option<u32>,
    /// Page length in lines (vertical units), if the job gave one.
    pub height_lines: Option<u32>,
}

/// One ink channel's dots: `height` lines of `width_bytes` bytes, 4 pixels per byte with
/// the leftmost pixel in the top two bits. Each pixel is a dot size, 0 (none) to 3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plane {
    pub width_bytes: usize,
    pub height: usize,
    pub data: Vec<u8>,
}

impl Plane {
    pub fn new(width_bytes: usize, height: usize) -> Plane {
        Plane { width_bytes, height, data: vec![0; width_bytes * height] }
    }

    pub fn row(&self, y: usize) -> &[u8] {
        &self.data[y * self.width_bytes..(y + 1) * self.width_bytes]
    }

    pub fn row_mut(&mut self, y: usize) -> &mut [u8] {
        &mut self.data[y * self.width_bytes..(y + 1) * self.width_bytes]
    }

    /// First and last byte columns with any dots in lines `y..y + lines`.
    pub fn ink_columns(&self, y: usize, lines: usize) -> Option<(usize, usize)> {
        let mut span: Option<(usize, usize)> = None;
        for row in y..(y + lines).min(self.height) {
            let r = self.row(row);
            let Some(first) = r.iter().position(|&b| b != 0) else { continue };
            let last = r.iter().rposition(|&b| b != 0).unwrap();
            span = Some(match span {
                None => (first, last),
                Some((a, b)) => (a.min(first), b.max(last)),
            });
        }
        span
    }
}

/// One page of a job: its format and the dots of each channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    /// Page number in the file (how many FFs come before it).
    pub index: usize,
    pub format: PageFormat,
    /// Bits per pixel of every strip on the page.
    pub bits: u8,
    pub planes: BTreeMap<u8, Plane>,
}

impl Page {
    /// Page length in lines: the job's own value, or else the last line with dots.
    pub fn height(&self) -> usize {
        match self.format.height_lines {
            Some(h) => h as usize,
            None => self.planes.values().map(|p| p.height).max().unwrap_or(0),
        }
    }

    /// Rows of 1440-dpi output each line of this page covers (1 for white, 2 for colour).
    pub fn vscale(&self) -> u32 {
        1440 / self.format.vres.max(1)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RasterError {
    MissingFormat { page: usize, what: &'static str },
    UnitsDiffer { page: usize, detail: String },
    UnalignedX { page: usize, x: u32 },
    MixedBits { page: usize },
    Rle { page: usize, err: RleError },
    NegativePosition { page: usize },
    SplitRaster { page: usize },
}

impl fmt::Display for RasterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RasterError::MissingFormat { page, what } => write!(f, "page {page}: no {what} before the raster data"),
            RasterError::UnitsDiffer { page, detail } => {
                write!(f, "page {page}: movement units differ from the raster resolution ({detail})")
            }
            RasterError::UnalignedX { page, x } => {
                write!(f, "page {page}: strip at x={x} doesn't start on a byte boundary")
            }
            RasterError::MixedBits { page } => write!(f, "page {page}: strips use different bit depths"),
            RasterError::Rle { page, err } => write!(f, "page {page}: {err}"),
            RasterError::NegativePosition { page } => write!(f, "page {page}: a strip sits above the page top"),
            RasterError::SplitRaster { page } => {
                write!(f, "page {page}: raster commands are interrupted by other commands")
            }
        }
    }
}

impl std::error::Error for RasterError {}

#[derive(Default, Clone, Copy)]
struct Units {
    page: Option<u32>,
    v: Option<u32>,
    h: Option<u32>,
    vres: Option<u32>,
    hres: Option<u32>,
    page_len: Option<u32>,
    width: Option<u32>,
}

impl Units {
    fn apply(&mut self, cmd: u8, d: &[u8]) {
        let u16_at = |i: usize| u16::from_le_bytes([d[i], d[i + 1]]) as u32;
        let u32_at = |i: usize| u32::from_le_bytes([d[i], d[i + 1], d[i + 2], d[i + 3]]);
        match (cmd, d.len()) {
            // ESC ( U: page, vertical, horizontal divisors of a base unit
            (b'U', 5) => {
                let base = u16_at(3);
                if d[0] > 0 && d[1] > 0 && d[2] > 0 {
                    self.page = Some(base / d[0] as u32);
                    self.v = Some(base / d[1] as u32);
                    self.h = Some(base / d[2] as u32);
                }
            }
            (b'U', 1) if d[0] > 0 => {
                let u = 3600 / d[0] as u32;
                (self.page, self.v, self.h) = (Some(u), Some(u), Some(u));
            }
            // ESC ( D: base, vertical divisor, horizontal divisor
            (b'D', 4) if d[2] > 0 && d[3] > 0 => {
                let base = u16_at(0);
                self.vres = Some(base / d[2] as u32);
                self.hres = Some(base / d[3] as u32);
            }
            (b'C', 4) => self.page_len = Some(u32_at(0)),
            (b'C', 2) => self.page_len = Some(u16_at(0)),
            (b'S', 8) => {
                self.width = Some(u32_at(0));
                self.page_len = self.page_len.or(Some(u32_at(4)));
            }
            (b'S', 4) => self.width = Some(u16_at(0)),
            _ => {}
        }
    }

    fn format(&self, page: usize) -> Result<PageFormat, RasterError> {
        let vres = self.vres.or(self.v).ok_or(RasterError::MissingFormat { page, what: "ESC ( D or ( U resolution" })?;
        let hres = self.hres.or(self.h).ok_or(RasterError::MissingFormat { page, what: "ESC ( D or ( U resolution" })?;
        if self.v.is_some_and(|v| v != vres) || self.h.is_some_and(|h| h != hres) {
            return Err(RasterError::UnitsDiffer {
                page,
                detail: format!("units {:?}x{:?}, raster {hres}x{vres}", self.h, self.v),
            });
        }
        let scale = |n: u32, res: u32| self.page.map(|p| (n as u64 * res as u64 / p as u64) as u32);
        Ok(PageFormat {
            hres,
            vres,
            width_px: self.width.and_then(|w| scale(w, hres)),
            height_lines: self.page_len.and_then(|l| scale(l, vres)),
        })
    }
}

struct Placement<'a> {
    strip: &'a Strip,
    x: u32,
    y: i64,
}

struct PagePlan<'a> {
    index: usize,
    format: PageFormat,
    strips: Vec<Placement<'a>>,
}

/// Place every strip of a parsed job onto per-page, per-channel dot planes.
pub fn decode_pages(items: &[Item]) -> Result<Vec<Page>, RasterError> {
    // pass 1: where every strip goes
    let mut plans: Vec<PagePlan> = Vec::new();
    let mut units = Units::default();
    let (mut page, mut x, mut y) = (0usize, 0u32, 0i64);
    let mut current: Option<PagePlan> = None;
    for item in items {
        match item {
            Item::Ctl(FF) => {
                plans.extend(current.take());
                page += 1;
                (x, y) = (0, 0);
            }
            Item::Reset => (x, y) = (0, 0),
            Item::Command { cmd: b'$', data } if data.len() == 4 => {
                x = u32::from_le_bytes(data[..4].try_into().unwrap());
            }
            Item::Command { cmd: b'v', data } if data.len() == 4 => {
                y += i32::from_le_bytes(data[..4].try_into().unwrap()) as i64;
            }
            Item::Command { cmd, data } => units.apply(*cmd, data),
            Item::Raster(strip) => {
                if current.is_none() {
                    current = Some(PagePlan { index: page, format: units.format(page)?, strips: Vec::new() });
                }
                current.as_mut().unwrap().strips.push(Placement { strip, x, y });
            }
            _ => {}
        }
    }
    plans.extend(current.take());

    // pass 2: decode onto planes sized to the page (or to the dots, if bigger)
    let mut pages = Vec::with_capacity(plans.len());
    for plan in plans {
        let bits = plan.strips[0].strip.bits;
        if plan.strips.iter().any(|p| p.strip.bits != bits) {
            return Err(RasterError::MixedBits { page: plan.index });
        }
        let px_per_byte = (8 / bits.max(1)) as u32;
        let mut width_bytes = plan.format.width_px.map_or(0, |w| w.div_ceil(px_per_byte) as usize);
        let mut height = plan.format.height_lines.unwrap_or(0) as usize;
        for p in &plan.strips {
            if p.x % px_per_byte != 0 {
                return Err(RasterError::UnalignedX { page: plan.index, x: p.x });
            }
            if p.y < 0 {
                return Err(RasterError::NegativePosition { page: plan.index });
            }
            width_bytes = width_bytes.max((p.x / px_per_byte) as usize + p.strip.bytes_per_line as usize);
            height = height.max(p.y as usize + p.strip.lines as usize);
        }
        let mut planes: BTreeMap<u8, Plane> = BTreeMap::new();
        for p in &plan.strips {
            let raw = p.strip.raw().map_err(|err| RasterError::Rle { page: plan.index, err })?;
            let plane = planes.entry(p.strip.channel).or_insert_with(|| Plane::new(width_bytes, height));
            let (bx, nb) = ((p.x / px_per_byte) as usize, p.strip.bytes_per_line as usize);
            for (l, line) in raw.chunks(nb.max(1)).enumerate() {
                let row = plane.row_mut(p.y as usize + l);
                // strips of one channel can overlap only if a job is odd; merge with OR
                for (dst, src) in row[bx..bx + nb].iter_mut().zip(line) {
                    *dst |= *src;
                }
            }
        }
        pages.push(Page { index: plan.index, format: plan.format, bits, planes });
    }
    Ok(pages)
}

/// Garment Creator's crop for one band of one channel, from its first and last byte
/// columns with dots: returns (first byte column, bytes needed to reach the last dot).
pub fn crop(first: usize, last: usize, bits: u8) -> (usize, usize) {
    let px_per_byte = 8 / bits.max(1) as usize;
    let align = (X_ALIGN_PX / px_per_byte).max(1);
    let start = first / align * align;
    (start, last - start + 1)
}

/// `lines` lines of `nb` bytes from a plane, starting at byte column `bx` (zeros past the
/// plane's edges).
fn extract(plane: &Plane, y: usize, lines: usize, bx: usize, nb: usize) -> Vec<u8> {
    let mut raw = vec![0u8; nb * lines];
    for l in 0..lines.min(plane.height.saturating_sub(y)) {
        let row = plane.row(y + l);
        let end = (bx + nb).min(row.len());
        if end > bx {
            raw[l * nb..l * nb + (end - bx)].copy_from_slice(&row[bx..end]);
        }
    }
    raw
}

/// One strip the way Garment Creator writes it: compressed at an even width, or raw at the
/// exact width `need` when compressing the padded strip would make it larger.
pub fn garment_creator_strip(plane: &Plane, y: usize, lines: usize, bx: usize, need: usize, channel: u8, bits: u8) -> Strip {
    let even = need.div_ceil(2) * 2;
    let padded = extract(plane, y, lines, bx, even);
    let packed = rle::encode_strip(&padded, even);
    let (nb, mode, payload) = if packed.len() > padded.len() {
        (need, 4, extract(plane, y, lines, bx, need))
    } else {
        (even, 5, packed)
    };
    Strip { channel, mode, bits, bytes_per_line: nb as u16, lines: lines as u16, flag: Some(0), payload }
}

/// Turn a page's planes into strip commands the way Garment Creator 2.1.7 does.
pub fn encode_raster(page: &Page) -> Vec<Item> {
    let mut order: Vec<u8> = CHANNEL_ORDER.iter().copied().filter(|c| page.planes.contains_key(c)).collect();
    order.extend(page.planes.keys().copied().filter(|c| !CHANNEL_ORDER.contains(c)));
    let px_per_byte = 8 / page.bits.max(1) as u32;
    let height = page.height();
    let mut items = Vec::new();
    let mut last_y = 0usize;
    for y in (0..height).step_by(BAND_LINES) {
        let lines = BAND_LINES.min(height - y);
        let mut first_in_band = true;
        for &ch in &order {
            let plane = &page.planes[&ch];
            let Some((first, last)) = plane.ink_columns(y, lines) else { continue };
            let (bx, need) = crop(first, last, page.bits);
            if first_in_band {
                items.push(prn::move_down((y - last_y) as i32));
                last_y = y;
                first_in_band = false;
            }
            items.push(prn::set_x(bx as u32 * px_per_byte));
            items.push(Item::Raster(garment_creator_strip(plane, y, lines, bx, need, ch, page.bits)));
            items.push(Item::Ctl(CR));
        }
    }
    items
}

fn starts_raster(item: &Item) -> bool {
    matches!(item, Item::Raster(_) | Item::Command { cmd: b'v' | b'$', .. })
}

fn in_raster(item: &Item) -> bool {
    starts_raster(item) || matches!(item, Item::Ctl(CR))
}

/// Replace each page's raster commands with [`encode_raster`] of the matching page.
/// Everything else (headers, settings, thumbnail, page setup) is kept as it is.
pub fn rebuild(items: &[Item], pages: &[Page]) -> Result<Vec<Item>, RasterError> {
    let mut out = Vec::with_capacity(items.len());
    let mut page = 0usize;
    let mut done_page: Option<usize> = None;
    let mut i = 0;
    while i < items.len() {
        if starts_raster(&items[i]) {
            let start = i;
            while i < items.len() && in_raster(&items[i]) {
                i += 1;
            }
            if done_page == Some(page) {
                return Err(RasterError::SplitRaster { page });
            }
            done_page = Some(page);
            match pages.iter().find(|p| p.index == page) {
                Some(p) => out.extend(encode_raster(p)),
                None => out.extend_from_slice(&items[start..i]),
            }
            continue;
        }
        if items[i] == Item::Ctl(FF) {
            page += 1;
        }
        out.push(items[i].clone());
        i += 1;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crop_matches_garment_creator() {
        // the left edge snaps down to a 16-pixel (4-byte) boundary
        assert_eq!(crop(2464, 2464, 2), (2464, 1));
        assert_eq!(crop(2465, 2465, 2), (2464, 2));
        assert_eq!(crop(2467, 2481, 2), (2464, 18));
    }

    #[test]
    fn strips_pad_only_when_compressed() {
        let mut plane = Plane::new(16, 16);
        // a speck in one byte: raw, one byte wide
        plane.row_mut(3)[4] = 0x30;
        let s = garment_creator_strip(&plane, 0, 16, 4, 1, 63, 2);
        assert_eq!((s.mode, s.bytes_per_line), (4, 1));
        // a solid 5-byte run: compressed, padded to 6 bytes
        let mut solid = Plane::new(16, 16);
        for y in 0..16 {
            solid.row_mut(y)[4..9].fill(0xFF);
        }
        let s = garment_creator_strip(&solid, 0, 16, 4, 5, 63, 2);
        assert_eq!((s.mode, s.bytes_per_line), (5, 6));
        // noise that RLE can't shrink: raw at the exact odd width
        for y in 0..16 {
            for x in 4..7 {
                plane.row_mut(y)[x] = [0x31, 0x8C, 0x47][(x + y) % 3];
            }
        }
        let s = garment_creator_strip(&plane, 0, 16, 4, 3, 63, 2);
        assert_eq!((s.mode, s.bytes_per_line, s.payload.len()), (4, 3, 48));
    }

    fn page_with(planes: Vec<(u8, Plane)>, height: u32) -> Page {
        Page {
            index: 0,
            format: PageFormat { hres: 1440, vres: 1440, width_px: Some(256), height_lines: Some(height) },
            bits: 2,
            planes: planes.into_iter().collect(),
        }
    }

    #[test]
    fn encode_then_decode_round_trips() {
        let mut white = Plane::new(64, 40);
        let mut cyan = Plane::new(64, 40);
        for y in 3..37 {
            white.row_mut(y)[10..30].fill(0xFF);
            if y % 5 == 0 {
                cyan.row_mut(y)[33] = 0x30;
            }
        }
        let page = page_with(vec![(63, white), (2, cyan)], 40);
        let mut items = vec![
            Item::Command { cmd: b'U', data: vec![8, 2, 2, 0x40, 0x0B] },
            Item::Command { cmd: b'D', data: vec![0x40, 0x38, 10, 10] },
            Item::Command { cmd: b'C', data: 10u32.to_le_bytes().to_vec() },
            Item::Command { cmd: b'S', data: [64u32.to_le_bytes(), 10u32.to_le_bytes()].concat() },
        ];
        let raster = encode_raster(&page);
        // 3 bands (0, 16, 32), the last one 8 lines; white before cyan in each band
        let strips: Vec<(u8, u16)> = raster
            .iter()
            .filter_map(|it| if let Item::Raster(s) = it { Some((s.channel, s.lines)) } else { None })
            .collect();
        assert_eq!(strips, vec![(63, 16), (2, 16), (63, 16), (2, 16), (63, 8), (2, 8)]);
        items.extend(raster);
        items.push(Item::Ctl(FF));
        let decoded = decode_pages(&items).unwrap();
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].format.height_lines, Some(40));
        assert_eq!(decoded[0].planes, page.planes);
        assert_eq!(rebuild(&items, &decoded).unwrap(), items);
    }
}
