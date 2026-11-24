// SPDX-License-Identifier: GPL-3.0-or-later
//! The garment view: rulers, a T-shirt, the platen with its 1-inch grid, and the artwork.
//!
//! World coordinates are inches with the origin at the centre of the platen, y down.

use crate::model::{Job, Source};
use eframe::egui::{self, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Vec2};

const RULER: f32 = 22.0;
/// Inches of garment the default view fits on screen.
const FIT_INCHES: f32 = 35.0;
const BACKGROUND: Color32 = Color32::from_rgb(48, 50, 54);
const PLATEN_EDGE: Color32 = Color32::from_rgb(0, 224, 236);
const ACCENT: Color32 = Color32::from_rgb(255, 160, 40);

pub struct View {
    /// 1.0 = the whole shirt fits.
    pub zoom: f32,
    /// Screen offset of the world origin from the centre of the view, in points.
    pub pan: Vec2,
    dragging_art: bool,
}

impl Default for View {
    fn default() -> Self {
        View { zoom: 1.0, pan: Vec2::ZERO, dragging_art: false }
    }
}

impl View {
    pub fn fit(&mut self) {
        self.zoom = 1.0;
        self.pan = Vec2::ZERO;
    }
}

struct Xf {
    origin: Pos2,
    ppi: f32,
}

impl Xf {
    fn to_screen(&self, x: f32, y: f32) -> Pos2 {
        Pos2::new(self.origin.x + x * self.ppi, self.origin.y + y * self.ppi)
    }
    fn to_world(&self, p: Pos2) -> (f32, f32) {
        ((p.x - self.origin.x) / self.ppi, (p.y - self.origin.y) / self.ppi)
    }
}

fn shade(c: [u8; 3], d: i32) -> Color32 {
    let f = |v: u8| (v as i32 + d).clamp(0, 255) as u8;
    Color32::from_rgb(f(c[0]), f(c[1]), f(c[2]))
}

fn ellipse(xf: &Xf, cx: f32, cy: f32, rx: f32, ry: f32, n: usize) -> Vec<Pos2> {
    (0..n)
        .map(|i| {
            let t = i as f32 / n as f32 * std::f32::consts::TAU;
            xf.to_screen(cx + rx * t.cos(), cy + ry * t.sin())
        })
        .collect()
}

/// Our own simple T-shirt, drawn from convex pieces so it fills correctly.
fn draw_shirt(p: &egui::Painter, xf: &Xf, colour: [u8; 3]) {
    let light = colour.iter().map(|&v| v as u32).sum::<u32>() > 380;
    let fill = shade(colour, 0);
    let edge = Stroke::new(1.5, shade(colour, if light { -45 } else { 40 }));
    let seam = Stroke::new(1.0, shade(colour, if light { -28 } else { 22 }));
    let pts = |v: &[(f32, f32)]| v.iter().map(|&(x, y)| xf.to_screen(x, y)).collect::<Vec<_>>();
    let body = [(-3.6, -16.2), (3.6, -16.2), (10.2, -14.6), (10.8, -9.0), (10.8, 16.0), (-10.8, 16.0), (-10.8, -9.0), (-10.2, -14.6)];
    let right = [(10.2, -14.6), (16.0, -8.6), (13.4, -5.2), (10.8, -9.0)];
    let left: Vec<(f32, f32)> = right.iter().rev().map(|&(x, y)| (-x, y)).collect();
    for poly in [pts(&right), pts(&left)] {
        p.add(Shape::convex_polygon(poly, fill, edge));
    }
    p.add(Shape::convex_polygon(pts(&body), fill, edge));
    // sleeve seams
    p.line_segment([xf.to_screen(10.2, -14.6), xf.to_screen(10.8, -9.0)], seam);
    p.line_segment([xf.to_screen(-10.2, -14.6), xf.to_screen(-10.8, -9.0)], seam);
    // the neck: inside of the back, then the collar band
    p.add(Shape::convex_polygon(ellipse(xf, 0.0, -15.9, 3.2, 1.05, 48), shade(colour, if light { -30 } else { 14 }), Stroke::NONE));
    p.add(Shape::closed_line(ellipse(xf, 0.0, -15.9, 3.4, 1.25, 48), Stroke::new(0.3 * xf.ppi, fill)));
    p.add(Shape::closed_line(ellipse(xf, 0.0, -15.9, 3.6, 1.45, 48), seam));
}

fn draw_rulers(p: &egui::Painter, rect: Rect, content: Rect, xf: &Xf, platen_w: f32, platen_h: f32) {
    let bg = Color32::from_rgb(226, 226, 226);
    let ink = Color32::from_rgb(40, 40, 40);
    let font = FontId::proportional(10.0);
    p.rect_filled(Rect::from_min_max(rect.min, Pos2::new(rect.max.x, rect.min.y + RULER)), CornerRadius::ZERO, bg);
    p.rect_filled(Rect::from_min_max(rect.min, Pos2::new(rect.min.x + RULER, rect.max.y)), CornerRadius::ZERO, bg);
    // label every inch if there's room, otherwise every 2 or 5
    let step = [1, 2, 5, 10].into_iter().find(|s| *s as f32 * xf.ppi >= 22.0).unwrap_or(10);
    let half = xf.ppi >= 30.0;
    let (x0, _) = xf.to_world(content.min);
    let (x1, _) = xf.to_world(content.max);
    for i in (x0.floor() as i32)..=(x1.ceil() as i32) {
        let x = xf.to_screen(i as f32, 0.0).x;
        if x < content.min.x || x > content.max.x {
            continue;
        }
        let major = i % step == 0;
        let len = if major { 9.0 } else { 5.0 };
        p.line_segment([Pos2::new(x, rect.min.y + RULER - len), Pos2::new(x, rect.min.y + RULER)], Stroke::new(1.0, ink));
        if major {
            p.text(Pos2::new(x + 2.0, rect.min.y + 2.0), egui::Align2::LEFT_TOP, i.to_string(), font.clone(), ink);
        }
        if half {
            let xh = xf.to_screen(i as f32 + 0.5, 0.0).x;
            p.line_segment([Pos2::new(xh, rect.min.y + RULER - 3.0), Pos2::new(xh, rect.min.y + RULER)], Stroke::new(1.0, ink));
        }
    }
    let (_, y0) = xf.to_world(content.min);
    let (_, y1) = xf.to_world(content.max);
    for i in (y0.floor() as i32)..=(y1.ceil() as i32) {
        let y = xf.to_screen(0.0, i as f32).y;
        if y < content.min.y || y > content.max.y {
            continue;
        }
        let major = i % step == 0;
        let len = if major { 9.0 } else { 5.0 };
        p.line_segment([Pos2::new(rect.min.x + RULER - len, y), Pos2::new(rect.min.x + RULER, y)], Stroke::new(1.0, ink));
        if major {
            p.text(Pos2::new(rect.min.x + 2.0, y + 1.0), egui::Align2::LEFT_TOP, i.to_string(), font.clone(), ink);
        }
        if half {
            let yh = xf.to_screen(0.0, i as f32 + 0.5).y;
            p.line_segment([Pos2::new(rect.min.x + RULER - 3.0, yh), Pos2::new(rect.min.x + RULER, yh)], Stroke::new(1.0, ink));
        }
    }
    // mark the platen's extent on both rulers
    let mark = Color32::from_rgb(0, 170, 190);
    let a = xf.to_screen(-platen_w / 2.0, -platen_h / 2.0);
    let b = xf.to_screen(platen_w / 2.0, platen_h / 2.0);
    p.line_segment([Pos2::new(a.x.max(content.min.x), rect.min.y + RULER - 2.0), Pos2::new(b.x.min(content.max.x), rect.min.y + RULER - 2.0)], Stroke::new(3.0, mark));
    p.line_segment([Pos2::new(rect.min.x + RULER - 2.0, a.y.max(content.min.y)), Pos2::new(rect.min.x + RULER - 2.0, b.y.min(content.max.y))], Stroke::new(3.0, mark));
    p.rect_filled(Rect::from_min_size(rect.min, Vec2::splat(RULER)), CornerRadius::ZERO, bg);
}

/// Draw the view and handle dragging, panning and zooming. Returns true if the artwork moved.
pub fn show(ui: &mut egui::Ui, view: &mut View, job: &mut Job) -> bool {
    let rect = ui.available_rect_before_wrap();
    let content = Rect::from_min_max(rect.min + Vec2::splat(RULER), rect.max);
    let response = ui.allocate_rect(content, Sense::click_and_drag());
    let painter = ui.painter_at(rect);

    let base_ppi = (content.width().min(content.height()) / FIT_INCHES).max(1.0);
    let ppi = base_ppi * view.zoom;
    // the shirt's middle (y = 0) sits a little below the platen centre; keep the view on it
    let centre = content.center() + view.pan;
    let xf = Xf { origin: centre, ppi };

    let platen = *job.platen();
    let (pw, ph) = (platen.width_in, platen.height_in);
    // platen top-left in world inches
    let (px0, py0) = (-pw / 2.0, -ph / 2.0);

    // interaction
    let mut moved = false;
    let art_rect = job.artwork.as_ref().map(|a| {
        Rect::from_min_max(
            xf.to_screen(px0 + a.pos_in[0], py0 + a.pos_in[1]),
            xf.to_screen(px0 + a.pos_in[0] + a.size_in[0], py0 + a.pos_in[1] + a.size_in[1]),
        )
    });
    let movable = job.artwork.as_ref().is_some_and(|a| a.source != Source::PrintFile);
    if response.drag_started() {
        view.dragging_art = movable
            && response.interact_pointer_pos().zip(art_rect).is_some_and(|(p, r)| r.contains(p));
    }
    if response.dragged() {
        let d = response.drag_delta();
        if view.dragging_art {
            if let Some(a) = &mut job.artwork {
                a.pos_in[0] += d.x / ppi;
                a.pos_in[1] += d.y / ppi;
            }
            job.clamp_artwork();
            moved = true;
        } else {
            view.pan += d;
        }
    }
    if response.drag_stopped() {
        view.dragging_art = false;
    }
    if response.hovered() {
        let (zoom, scroll) = ui.input(|i| (i.zoom_delta(), i.smooth_scroll_delta));
        if zoom != 1.0 {
            // zoom about the pointer
            if let Some(p) = response.hover_pos() {
                let before = p - centre;
                let new_zoom = (view.zoom * zoom).clamp(0.25, 12.0);
                let k = new_zoom / view.zoom;
                view.pan += before - before * k;
                view.zoom = new_zoom;
            }
        } else if scroll != Vec2::ZERO {
            view.pan += scroll;
        }
    }

    // drawing
    painter.rect_filled(content, CornerRadius::ZERO, BACKGROUND);
    let clip = painter.with_clip_rect(content);
    draw_shirt(&clip, &xf, job.preset().shirt);

    let platen_rect = Rect::from_min_max(xf.to_screen(px0, py0), xf.to_screen(px0 + pw, py0 + ph));
    let light = job.preset().shirt.iter().map(|&v| v as u32).sum::<u32>() > 380;
    let grid = if light { Color32::from_rgba_unmultiplied(80, 80, 80, 70) } else { Color32::from_rgba_unmultiplied(200, 200, 200, 55) };
    for i in 1..(pw as i32) {
        let x = xf.to_screen(px0 + i as f32, 0.0).x;
        clip.line_segment([Pos2::new(x, platen_rect.min.y), Pos2::new(x, platen_rect.max.y)], Stroke::new(1.0, grid));
    }
    for i in 1..(ph as i32) {
        let y = xf.to_screen(0.0, py0 + i as f32).y;
        clip.line_segment([Pos2::new(platen_rect.min.x, y), Pos2::new(platen_rect.max.x, y)], Stroke::new(1.0, grid));
    }
    let centre_line = Stroke::new(2.0, grid.gamma_multiply(1.8));
    clip.line_segment([Pos2::new(platen_rect.center().x, platen_rect.min.y), Pos2::new(platen_rect.center().x, platen_rect.max.y)], centre_line);
    clip.line_segment([Pos2::new(platen_rect.min.x, platen_rect.center().y), Pos2::new(platen_rect.max.x, platen_rect.center().y)], centre_line);

    if let (Some(a), Some(r)) = (&job.artwork, art_rect) {
        let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        clip.image(a.texture.id(), r, uv, Color32::WHITE);
        let hovered = response.hover_pos().is_some_and(|p| r.contains(p));
        if movable && (hovered || view.dragging_art) {
            clip.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.5, ACCENT), StrokeKind::Outside);
        }
    }
    clip.rect_stroke(platen_rect, CornerRadius::ZERO, Stroke::new(3.0, PLATEN_EDGE), StrokeKind::Outside);

    draw_rulers(&painter, rect, content, &xf, pw, ph);

    if movable && response.hover_pos().zip(art_rect).is_some_and(|(p, r)| r.contains(p)) {
        ui.ctx().set_cursor_icon(if view.dragging_art { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
    }
    moved
}
