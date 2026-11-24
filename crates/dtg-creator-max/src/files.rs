// SPDX-License-Identifier: GPL-3.0-or-later
//! Opening artwork images, Garment Creator job files (.gcr) and print files (.prn).

use crate::model::{Artwork, Job, Source, PLATENS, PRESETS};
use eframe::egui::{self, ColorImage, TextureOptions};
use std::path::Path;

/// Longest side of the on-screen texture; larger artwork is scaled down for display only.
const MAX_TEXTURE: u32 = 2048;
/// Garment Creator's placement unit: 1/360 inch.
const GC_UNIT: f32 = 360.0;

pub const OPEN_EXTENSIONS: &[&str] = &["gcr", "prn", "png", "jpg", "jpeg"];

/// Open any supported file into a job. Returns a short status message.
pub fn open(ctx: &egui::Context, path: &Path, job: &mut Job) -> Result<String, String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let bytes = std::fs::read(path).map_err(|e| format!("Couldn't read {}: {e}", path.display()))?;
    let file_name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let stem = path.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    match ext.as_str() {
        "gcr" => open_gcr(ctx, &bytes, file_name, stem, job),
        "prn" => open_prn(ctx, &bytes, file_name, stem, job),
        _ => open_image(ctx, &bytes, file_name, stem, job),
    }
}

fn texture(ctx: &egui::Context, name: &str, img: image::RgbaImage) -> egui::TextureHandle {
    let img = if img.width().max(img.height()) > MAX_TEXTURE {
        image::imageops::thumbnail(&img, scale_dim(img.width(), img.height()).0, scale_dim(img.width(), img.height()).1)
    } else {
        img
    };
    let size = [img.width() as usize, img.height() as usize];
    ctx.load_texture(name, ColorImage::from_rgba_unmultiplied(size, img.as_raw()), TextureOptions::LINEAR)
}

fn scale_dim(w: u32, h: u32) -> (u32, u32) {
    let k = MAX_TEXTURE as f32 / w.max(h) as f32;
    (((w as f32 * k).round() as u32).max(1), ((h as f32 * k).round() as u32).max(1))
}

/// Resolution from a PNG's pHYs chunk, in dots per inch.
fn png_dpi(bytes: &[u8]) -> Option<f32> {
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return None;
    }
    let mut i = 8;
    while i + 8 <= bytes.len() {
        let len = u32::from_be_bytes(bytes[i..i + 4].try_into().ok()?) as usize;
        let tag = &bytes[i + 4..i + 8];
        if tag == b"pHYs" && len >= 9 {
            let d = bytes.get(i + 8..i + 17)?;
            let px = u32::from_be_bytes(d[0..4].try_into().ok()?);
            return (d[8] == 1 && px > 0).then(|| px as f32 * 0.0254);
        }
        if tag == b"IDAT" {
            return None;
        }
        i += 12 + len;
    }
    None
}

fn decode(bytes: &[u8]) -> Result<image::RgbaImage, String> {
    image::load_from_memory(bytes).map(|i| i.to_rgba8()).map_err(|e| format!("Couldn't decode the image: {e}"))
}

fn open_image(ctx: &egui::Context, bytes: &[u8], file_name: String, stem: String, job: &mut Job) -> Result<String, String> {
    let img = decode(bytes)?;
    let (dpi, note) = match png_dpi(bytes) {
        Some(d) if d >= 10.0 => (d, String::new()),
        _ => (300.0, " (no resolution stored, assumed 300 dpi)".to_string()),
    };
    let pixels = [img.width() as usize, img.height() as usize];
    let mut art = Artwork {
        texture: texture(ctx, &file_name, img),
        file_name: file_name.clone(),
        source: Source::Image,
        pixels,
        dpi,
        pos_in: [0.0, 0.5],
        size_in: [pixels[0] as f32 / dpi, pixels[1] as f32 / dpi],
    };
    // too big for the platen: fit it
    let (pw, ph) = (job.platen().width_in, job.platen().height_in - 0.5);
    let k = (pw / art.size_in[0]).min(ph / art.size_in[1]).min(1.0);
    art.size_in = [art.size_in[0] * k, art.size_in[1] * k];
    job.artwork = Some(art);
    job.name = stem;
    job.centre_artwork();
    job.clamp_artwork();
    Ok(format!("Opened {file_name}: {} × {} px at {dpi:.0} dpi{note}", pixels[0], pixels[1]))
}

fn num(g: &f2000::gcr::Gcr, key: &str) -> Option<f32> {
    g.setting(key)?.trim().parse().ok()
}

fn open_gcr(ctx: &egui::Context, bytes: &[u8], file_name: String, stem: String, job: &mut Job) -> Result<String, String> {
    let g = f2000::gcr::parse(bytes).map_err(|e| e.to_string())?;
    let img = decode(&g.artwork_png)?;
    let pixels = [img.width() as usize, img.height() as usize];
    // placement is in 1/360 in on the platen
    let size_in = [
        num(&g, "resizeWidth").unwrap_or(pixels[0] as f32) / GC_UNIT,
        num(&g, "resizeHeight").unwrap_or(pixels[1] as f32) / GC_UNIT,
    ];
    let pos_in = [num(&g, "imagePosX").unwrap_or(0.0) / GC_UNIT, num(&g, "imagePosY").unwrap_or(0.0) / GC_UNIT];
    let real_w = num(&g, "realWidth").unwrap_or(pixels[0] as f32);
    let dpi = pixels[0] as f32 * GC_UNIT / real_w.max(1.0);
    if let Some(t) = num(&g, "platenType") {
        if let Some(i) = PLATENS.iter().position(|p| p.gc_type == Some(t as u32)) {
            job.platen = i;
        }
    }
    if let Some(id) = num(&g, "garmentId") {
        if let Some(i) = PRESETS.iter().position(|p| p.confirmed && p.garment_id == id as i32) {
            job.preset = i;
        }
    }
    let s = &mut job.settings;
    if let Some(m) = num(&g, "printModeColor") {
        s.colour_quality = (m as u32 % 10).min(2) as usize;
    }
    if let Some(m) = num(&g, "printModeWhite").filter(|m| *m > 0.0) {
        s.white_quality = ((m as u32 % 100) / 10).clamp(1, 3) as usize - 1;
    }
    let int = |k: &str| num(&g, k).map(|v| v as i32);
    s.density_colour = int("dutyC").unwrap_or(0);
    s.density_underbase = int("dutyUnderW").unwrap_or(0);
    s.density_design_white = int("dutyDesignW").unwrap_or(0);
    s.underbase_inset = int("underShapeWidth").unwrap_or(2).max(0) as u32;
    s.white_layers = int("whiteLayerNum").unwrap_or(1).max(1) as u32;
    s.drying_time_s = int("dryingTime").unwrap_or(0).max(0) as u32;
    s.bidirectional = g.setting("uniD").as_deref() != Some("true");
    s.brightness = int("lightness").unwrap_or(0);
    s.saturation = int("saturation").unwrap_or(0);
    s.contrast = int("contrast").unwrap_or(0);
    s.red = int("redAdj").unwrap_or(0);
    s.green = int("greenAdj").unwrap_or(0);
    s.blue = int("blueAdj").unwrap_or(0);
    job.name = g.setting("jobName").map(|v| v.trim_matches('"').to_string()).unwrap_or(stem);
    job.artwork = Some(Artwork {
        texture: texture(ctx, &file_name, img),
        file_name: file_name.clone(),
        source: Source::Job,
        pixels,
        dpi,
        pos_in,
        size_in,
    });
    Ok(format!(
        "Opened job {file_name}: {} on the {} platen",
        job.preset().name,
        job.platen().name.split_whitespace().next().unwrap_or("")
    ))
}

fn open_prn(ctx: &egui::Context, bytes: &[u8], file_name: String, stem: String, job: &mut Job) -> Result<String, String> {
    let items = f2000::prn::parse(bytes).map_err(|e| format!("Not a readable print file: {e}"))?;
    let pages = f2000::raster::decode_pages(&items).map_err(|e| e.to_string())?;
    if pages.is_empty() {
        return Err("The print file has no printed dots.".into());
    }
    let has_white = pages.iter().any(|p| p.planes.contains_key(&f2000::render::WHITE));
    let p = f2000::render::preview(&pages);
    // colour ink as colour, white ink alone as white; opacity from ink coverage
    let mut rgba = Vec::with_capacity(p.width * p.height * 4);
    for i in 0..p.width * p.height {
        let w = p.white[i * 3] as f32 / 255.0;
        let c = &p.colour[i * 3..i * 3 + 3];
        let colour_cov = 1.0 - c.iter().copied().min().unwrap_or(255) as f32 / 255.0;
        let alpha = w.max(colour_cov);
        rgba.extend_from_slice(&[c[0], c[1], c[2], (alpha * 255.0).round() as u8]);
    }
    let img = image::RgbaImage::from_raw(p.width as u32, p.height as u32, rgba).ok_or("preview size mismatch")?;
    let dpi = (1440 / f2000::render::BLOCK_PX) as f32;
    let width_in = p.width as f32 / dpi;
    if let Some(i) = PLATENS.iter().position(|pl| (pl.width_in - width_in).abs() < 0.2) {
        job.platen = i;
    }
    job.preset = if has_white { 0 } else { 1 };
    job.name = stem;
    job.artwork = Some(Artwork {
        texture: texture(ctx, &file_name, img),
        file_name: file_name.clone(),
        source: Source::PrintFile,
        pixels: [p.width, p.height],
        dpi,
        pos_in: [0.0, 0.0],
        size_in: [width_in, p.height as f32 / dpi],
    });
    let inks: Vec<&str> = {
        let mut chs: Vec<u8> = pages.iter().flat_map(|pg| pg.planes.keys().copied()).collect();
        chs.sort();
        chs.dedup();
        chs.into_iter().map(f2000::render::channel_name).collect()
    };
    Ok(format!("Opened print file {file_name}: {} page(s), inks: {}", pages.len(), inks.join(", ")))
}
