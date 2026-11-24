// SPDX-License-Identifier: GPL-3.0-or-later
//! Preview images and ink totals from dot planes.
//!
//! Previews are 90 dpi: each preview pixel sums the dots of a 16 x 16 block at 1440 dpi.
//! A colour-pass line (720 dpi) covers two 1440-dpi rows, so it counts twice. The preview
//! estimates how much fabric the dots cover, which is approximate; the ink totals are exact.

use crate::raster::Page;
use std::collections::BTreeMap;

/// Preview pixel size in 1440-dpi pixels.
pub const BLOCK_PX: usize = 16;
/// Dot area per block when every pixel is covered, in [`AREA`] units.
const FULL_BLOCK: f64 = (BLOCK_PX * BLOCK_PX * 100) as f64;
/// How much of its pixel each dot size covers on the garment, in percent. Ink spreads, so
/// even small dots cover most of their pixel. These weights only shape the preview.
const DOT_AREA: [u16; 4] = [0, 60, 85, 100];
/// Preview-only gamma for ink spreading into the fabric (1.0 = none).
const SPREAD: f64 = 1.6;

/// Channel numbers in an F2000 White Edition job.
pub const BLACK: u8 = 0;
pub const MAGENTA: u8 = 1;
pub const CYAN: u8 = 2;
pub const YELLOW: u8 = 4;
pub const WHITE: u8 = 63;

pub fn channel_name(ch: u8) -> &'static str {
    match ch {
        BLACK => "black",
        MAGENTA => "magenta",
        CYAN => "cyan",
        YELLOW => "yellow",
        WHITE => "white",
        _ => "unknown",
    }
}

/// Sum of the four 2-bit dot sizes in a byte (0 to 12).
const INK: [u8; 256] = {
    let mut t = [0u8; 256];
    let mut b = 0;
    while b < 256 {
        t[b] = ((b & 3) + ((b >> 2) & 3) + ((b >> 4) & 3) + ((b >> 6) & 3)) as u8;
        b += 1;
    }
    t
};

/// Covered area of the four pixels in a byte, in percent of one pixel each (0 to 400).
const AREA: [u16; 256] = {
    let mut t = [0u16; 256];
    let mut b = 0;
    while b < 256 {
        t[b] = DOT_AREA[b & 3] + DOT_AREA[(b >> 2) & 3] + DOT_AREA[(b >> 4) & 3] + DOT_AREA[(b >> 6) & 3];
        b += 1;
    }
    t
};

/// Ink per channel, in dot-size units at 1440 x 1440 dpi. Same numbers as
/// `tools/prn_render.py` writes to `channels.txt`.
pub fn ink_totals(pages: &[Page]) -> BTreeMap<u8, u64> {
    let mut totals = BTreeMap::new();
    for page in pages {
        let vs = page.vscale() as u64;
        for (&ch, plane) in &page.planes {
            let sum: u64 = plane.data.iter().map(|&b| INK[b as usize] as u64).sum();
            *totals.entry(ch).or_insert(0) += sum * vs;
        }
    }
    totals
}

/// Three RGB previews of a job, each `width * height * 3` bytes.
pub struct Preview {
    pub width: usize,
    pub height: usize,
    /// White ink coverage, white on black.
    pub white: Vec<u8>,
    /// Colour inks on a white background.
    pub colour: Vec<u8>,
    /// Colour over the white underbase on a black garment.
    pub on_black: Vec<u8>,
}

pub fn preview(pages: &[Page]) -> Preview {
    let width = pages
        .iter()
        .flat_map(|p| p.planes.values().map(|pl| pl.width_bytes * 4))
        .max()
        .unwrap_or(0)
        .div_ceil(BLOCK_PX);
    let height = pages
        .iter()
        .map(|p| p.height() * p.vscale() as usize)
        .max()
        .unwrap_or(0)
        .div_ceil(BLOCK_PX);
    let mut grids: BTreeMap<u8, Vec<u32>> = BTreeMap::new();
    for page in pages {
        let vs = page.vscale() as usize;
        for (&ch, plane) in &page.planes {
            let grid = grids.entry(ch).or_insert_with(|| vec![0; width * height]);
            for y in 0..plane.height {
                let row = (y * vs / BLOCK_PX).min(height.saturating_sub(1));
                let cells = &mut grid[row * width..(row + 1) * width];
                for (bx, &b) in plane.row(y).iter().enumerate() {
                    if b != 0 {
                        cells[bx * 4 / BLOCK_PX] += AREA[b as usize] as u32 * vs as u32;
                    }
                }
            }
        }
    }
    let cov = |ch: u8, i: usize| {
        grids.get(&ch).map_or(0.0, |g| 1.0 - (1.0 - (g[i] as f64 / FULL_BLOCK).min(1.0)).powf(SPREAD))
    };
    let mut white = Vec::with_capacity(width * height * 3);
    let mut colour = Vec::with_capacity(width * height * 3);
    let mut on_black = Vec::with_capacity(width * height * 3);
    for i in 0..width * height {
        let w = cov(WHITE, i);
        let (k, m, c, y) = (cov(BLACK, i), cov(MAGENTA, i), cov(CYAN, i), cov(YELLOW, i));
        let rgb = [255.0 * (1.0 - c) * (1.0 - k), 255.0 * (1.0 - m) * (1.0 - k), 255.0 * (1.0 - y) * (1.0 - k)];
        let wv = (255.0 * w).round() as u8;
        white.extend_from_slice(&[wv, wv, wv]);
        colour.extend(rgb.iter().map(|v| v.round() as u8));
        on_black.extend(rgb.iter().map(|v| (v * w).round() as u8));
    }
    Preview { width, height, white, colour, on_black }
}
