// SPDX-License-Identifier: GPL-3.0-or-later
//! DTG Creator Max: open-source software for printing on the Epson SureColor SC-F2000.
//!
//! usage: dtg-creator-max [file]                    open a .gcr, .prn or image at start-up
//!        dtg-creator-max [file] --screenshot out.png   save the window as a PNG and quit

mod app;
mod canvas;
mod files;
mod model;

use std::path::PathBuf;

fn main() -> eframe::Result {
    let mut open = None;
    let mut screenshot = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--screenshot" {
            screenshot = args.next().map(PathBuf::from);
        } else {
            open = Some(PathBuf::from(a));
        }
    }
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("DTG Creator Max")
            .with_inner_size([1500.0, 950.0])
            .with_min_inner_size([1100.0, 700.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "DTG Creator Max",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, open, screenshot)))),
    )
}
