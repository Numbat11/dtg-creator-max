// SPDX-License-Identifier: GPL-3.0-or-later
//! The main window: printer bar, toolbar, garment view and the settings panel.

use crate::canvas::{self, View};
use crate::files;
use crate::model::{Inks, Job, Source, COLOUR_QUALITY, PLATENS, PRESETS, WHITE_QUALITY};
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Print,
    Layout,
    ColourReplacement,
}

pub struct App {
    job: Job,
    view: View,
    tab: Tab,
    status: String,
    about: bool,
    pending_open: Option<PathBuf>,
    /// Testing aid: save a screenshot of the window to this file, then quit.
    screenshot: Option<PathBuf>,
    frames: u32,
    shot: std::sync::Arc<std::sync::Mutex<Option<std::sync::Arc<egui::ColorImage>>>>,
}

const NOT_YET: &str = "Not available yet: DTG Creator Max can't make dots from artwork or talk to the printer yet. \
                       Those are the next pieces being built.";
const BLACK_BAR: Color32 = Color32::from_rgb(8, 8, 10);
const LINK: Color32 = Color32::from_rgb(40, 110, 200);

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, open: Option<PathBuf>, screenshot: Option<PathBuf>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        App {
            job: Job::default(),
            view: View::default(),
            tab: Tab::Print,
            status: "Open an image, a Garment Creator job (.gcr) or a print file (.prn) to begin.".into(),
            about: false,
            pending_open: open,
            screenshot,
            frames: 0,
            shot: Default::default(),
        }
    }

    fn open(&mut self, ctx: &egui::Context, path: PathBuf) {
        let mut job = Job { preset: self.job.preset, platen: self.job.platen, ..Job::default() };
        match files::open(ctx, &path, &mut job) {
            Ok(msg) => {
                self.job = job;
                self.status = msg;
                self.view.fit();
            }
            Err(e) => self.status = e,
        }
    }

    fn open_dialog(&mut self, ctx: &egui::Context) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Artwork, jobs and print files", files::OPEN_EXTENSIONS)
            .add_filter("Images", &["png", "jpg", "jpeg"])
            .add_filter("Garment Creator jobs", &["gcr"])
            .add_filter("Print files", &["prn"])
            .pick_file()
        {
            self.open(ctx, path);
        }
    }

    fn printer_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("printer")
            .exact_size(72.0)
            .frame(egui::Frame::new().fill(BLACK_BAR).inner_margin(egui::Margin::symmetric(16, 8)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Printer:").color(Color32::WHITE).size(12.0));
                        egui::ComboBox::from_id_salt("printer")
                            .width(320.0)
                            .selected_text("Epson SC-F2000 (White Edition)")
                            .show_ui(ui, |ui| {
                                let _ = ui.selectable_label(true, "Epson SC-F2000 (White Edition)");
                            });
                    });
                    ui.add_space(18.0);
                    // status ring and message
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(44.0), egui::Sense::hover());
                    ui.painter().circle_stroke(r.center(), 18.0, Stroke::new(5.0, Color32::from_gray(90)));
                    ui.vertical(|ui| {
                        ui.add_space(4.0);
                        ui.label(RichText::new("Off-line: no printer connected").color(Color32::WHITE).size(17.0));
                        ui.add(egui::ProgressBar::new(0.0).desired_width(380.0).desired_height(6.0));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Max), |ui| {
                        for (tab, label) in [(Tab::ColourReplacement, "Colour replacement"), (Tab::Layout, "Layout settings"), (Tab::Print, "Print settings")] {
                            let selected = self.tab == tab;
                            let text = RichText::new(label).size(15.0).strong().color(if selected {
                                Color32::from_rgb(70, 170, 255)
                            } else {
                                Color32::WHITE
                            });
                            if ui.add(egui::Label::new(text).sense(egui::Sense::click())).clicked() {
                                self.tab = tab;
                            }
                            if tab != Tab::Print {
                                ui.label(RichText::new("|").color(Color32::GRAY));
                            }
                        }
                    });
                });
            });
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        egui::Panel::top("toolbar").exact_size(34.0).show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                if ui.button("New").on_hover_text("Start a new job").clicked() {
                    self.job = Job { preset: self.job.preset, platen: self.job.platen, ..Job::default() };
                    self.status = "New job.".into();
                }
                if ui.button("Open…").on_hover_text("Open artwork, a Garment Creator job or a print file").clicked() {
                    self.open_dialog(&ctx);
                }
                ui.separator();
                ui.add_enabled(false, egui::Button::new("Save job")).on_disabled_hover_text("Saving jobs comes soon.");
                ui.add_enabled(false, egui::Button::new("Save print file")).on_disabled_hover_text(NOT_YET);
                ui.separator();
                if ui.button("Fit view").on_hover_text("Show the whole garment").clicked() {
                    self.view.fit();
                }
                if ui.add_enabled(self.movable(), egui::Button::new("Centre design")).clicked() {
                    self.job.centre_artwork();
                }
                ui.separator();
                if ui.button("About").clicked() {
                    self.about = true;
                }
            });
        });
    }

    fn movable(&self) -> bool {
        self.job.artwork.as_ref().is_some_and(|a| a.source != Source::PrintFile)
    }

    fn zoom_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("zoom").exact_size(30.0).show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.label("Zoom");
                ui.label(format!("{:.0} %", self.view.zoom * 100.0));
                if ui.small_button(" + ").on_hover_text("Zoom in (or pinch, or Ctrl + scroll)").clicked() {
                    self.view.zoom = (self.view.zoom * 1.25).min(12.0);
                }
                if ui.small_button(" − ").on_hover_text("Zoom out").clicked() {
                    self.view.zoom = (self.view.zoom / 1.25).max(0.25);
                }
                ui.separator();
                ui.label(RichText::new(&self.status).color(Color32::from_gray(200)));
            });
        });
    }

    fn side_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::right("settings")
            .exact_size(520.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(Color32::from_gray(238)).inner_margin(egui::Margin::same(12)))
            .show(ui, |ui| {
                ui.style_mut().visuals = egui::Visuals::light();
                egui::Panel::bottom("job")
                    .frame(egui::Frame::new().fill(Color32::from_gray(238)).inner_margin(egui::Margin::symmetric(0, 10)))
                    .show(ui, |ui| self.job_footer(ui));
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match self.tab {
                    Tab::Print => self.print_tab(ui),
                    Tab::Layout => self.layout_tab(ui),
                    Tab::ColourReplacement => colour_replacement_tab(ui),
                });
            });
    }

    fn job_footer(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Job name:").color(LINK));
            ui.add(egui::TextEdit::singleline(&mut self.job.name).desired_width(f32::INFINITY));
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_enabled(
                    false,
                    egui::Button::new(RichText::new("Print").size(18.0).color(Color32::WHITE))
                        .fill(Color32::from_rgb(80, 170, 235))
                        .min_size(Vec2::new(190.0, 46.0)),
                )
                .on_disabled_hover_text(NOT_YET);
                ui.add_space(12.0);
                ui.add(egui::DragValue::new(&mut self.job.copies).range(1..=999).prefix("Copies: "));
            });
        });
    }

    fn print_tab(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Presets").strong());
        egui::Frame::new().fill(Color32::WHITE).stroke(Stroke::new(1.0, Color32::from_gray(200))).show(ui, |ui| {
            ui.set_width(ui.available_width());
            for (i, p) in PRESETS.iter().enumerate() {
                let selected = self.job.preset == i;
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 30.0), egui::Sense::click());
                if selected {
                    ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(110, 190, 250));
                } else if resp.hovered() {
                    ui.painter().rect_filled(rect, 0.0, Color32::from_gray(232));
                }
                preset_icon(ui.painter(), rect.left_center() + Vec2::new(18.0, 0.0), p.inks);
                ui.painter().text(
                    rect.left_center() + Vec2::new(40.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    p.name,
                    egui::FontId::proportional(15.0),
                    Color32::from_gray(20),
                );
                let resp = if p.confirmed {
                    resp
                } else {
                    resp.on_hover_text("These settings haven't been checked against Epson's yet.")
                };
                if resp.clicked() {
                    self.job.preset = i;
                }
            }
        });
        let preset = self.job.preset();
        let has_white = preset.inks != Inks::Colour;
        let has_colour = preset.inks != Inks::WhiteOnly;
        let s = &mut self.job.settings;
        ui.add_space(8.0);
        egui::CollapsingHeader::new(RichText::new("Basic").strong()).default_open(true).show(ui, |ui| {
            settings_grid(ui, "basic", |ui| {
                row_label(ui, "Inks:");
                ui.label(preset.inks.label());
                ui.end_row();
                if has_colour {
                    row_label(ui, "Colour quality:");
                    combo(ui, "cq", &mut s.colour_quality, COLOUR_QUALITY);
                    ui.end_row();
                }
                if has_white {
                    row_label(ui, "White quality:");
                    combo(ui, "wq", &mut s.white_quality, WHITE_QUALITY);
                    ui.end_row();
                }
                row_label(ui, "Garment black:");
                ui.label(if preset.garment_black { "Pure black is left unprinted" } else { "Black is printed" });
                ui.end_row();
            });
        });
        egui::CollapsingHeader::new(RichText::new("Advanced").strong()).default_open(true).show(ui, |ui| {
            settings_grid(ui, "advanced", |ui| {
                row_label(ui, "Print direction:");
                let mut i = if s.bidirectional { 0 } else { 1 };
                combo(ui, "dir", &mut i, &["Both ways (faster)", "One way (finer)"]);
                s.bidirectional = i == 0;
                ui.end_row();
                row_label(ui, "Drying time:");
                ui.add(egui::DragValue::new(&mut s.drying_time_s).range(0..=300).suffix(" s"));
                ui.end_row();
                if has_white {
                    row_label(ui, "White layers:");
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut s.white_layers, 1, "1");
                        ui.radio_value(&mut s.white_layers, 2, "2");
                    });
                    ui.end_row();
                    row_label(ui, "Underbase inset:");
                    ui.add(egui::Slider::new(&mut s.underbase_inset, 0..=10));
                    ui.end_row();
                }
            });
        });
        egui::CollapsingHeader::new(RichText::new("Ink density").strong()).default_open(true).show(ui, |ui| {
            settings_grid(ui, "density", |ui| {
                if has_colour {
                    row_label(ui, "Colour:");
                    ui.add(egui::Slider::new(&mut s.density_colour, -10..=10));
                    ui.end_row();
                }
                if has_white {
                    row_label(ui, "Underbase white:");
                    ui.add(egui::Slider::new(&mut s.density_underbase, -10..=10));
                    ui.end_row();
                    row_label(ui, "Design white:");
                    ui.add(egui::Slider::new(&mut s.density_design_white, -10..=10));
                    ui.end_row();
                }
            });
        });
        if has_colour {
            egui::CollapsingHeader::new(RichText::new("Colour adjustments").strong()).default_open(true).show(ui, |ui| {
                settings_grid(ui, "colour", |ui| {
                    for (label, v) in [
                        ("Brightness:", &mut s.brightness),
                        ("Saturation:", &mut s.saturation),
                        ("Contrast:", &mut s.contrast),
                        ("Red level:", &mut s.red),
                        ("Green level:", &mut s.green),
                        ("Blue level:", &mut s.blue),
                    ] {
                        row_label(ui, label);
                        ui.add(egui::Slider::new(v, -50..=50));
                        ui.end_row();
                    }
                });
            });
        }
        let (c, w) = self.job.print_modes();
        let code = |m: Option<u32>| m.map_or("none".to_string(), |m| m.to_string());
        ui.add_space(6.0);
        ui.label(RichText::new(format!("Print modes: colour {}, white {}", code(c), code(w))).small().color(Color32::GRAY));
    }

    fn layout_tab(&mut self, ui: &mut egui::Ui) {
        settings_grid(ui, "platen", |ui| {
            row_label(ui, "Platen:");
            let mut i = self.job.platen;
            combo(ui, "platen", &mut i, &PLATENS.iter().map(|p| p.name).collect::<Vec<_>>());
            if i != self.job.platen {
                self.job.platen = i;
                self.job.clamp_artwork();
            }
            ui.end_row();
        });
        ui.add_space(8.0);
        let movable = self.movable();
        let keep = &mut self.job.keep_aspect;
        let Some(a) = &mut self.job.artwork else {
            ui.label("No design yet. Open an image, a job or a print file.");
            return;
        };
        ui.label(RichText::new("Design").strong());
        settings_grid(ui, "design", |ui| {
            row_label(ui, "File:");
            ui.label(&a.file_name);
            ui.end_row();
            row_label(ui, "Image:");
            ui.label(format!("{} × {} px at {:.0} dpi", a.pixels[0], a.pixels[1], a.dpi));
            ui.end_row();
        });
        if !movable {
            ui.add_space(6.0);
            ui.label("A print file shows the dots it will actually print, so it can't be moved or resized.");
            return;
        }
        ui.add_space(8.0);
        ui.label(RichText::new("Position and size (inches from the platen's top-left)").strong());
        let natural = a.natural_size_in();
        let mut changed_w = false;
        let mut changed_h = false;
        let mut scale = a.scale_percent();
        settings_grid(ui, "place", |ui| {
            row_label(ui, "Left:");
            ui.add(egui::DragValue::new(&mut a.pos_in[0]).speed(0.05).fixed_decimals(2).suffix(" in"));
            ui.end_row();
            row_label(ui, "Top:");
            ui.add(egui::DragValue::new(&mut a.pos_in[1]).speed(0.05).fixed_decimals(2).suffix(" in"));
            ui.end_row();
            row_label(ui, "Width:");
            changed_w = ui.add(egui::DragValue::new(&mut a.size_in[0]).speed(0.05).range(0.1..=40.0).fixed_decimals(2).suffix(" in")).changed();
            ui.end_row();
            row_label(ui, "Height:");
            changed_h = ui.add(egui::DragValue::new(&mut a.size_in[1]).speed(0.05).range(0.1..=40.0).fixed_decimals(2).suffix(" in")).changed();
            ui.end_row();
            row_label(ui, "Scale:");
            if ui.add(egui::DragValue::new(&mut scale).speed(0.5).range(1.0..=1000.0).fixed_decimals(1).suffix(" %")).changed() {
                a.size_in = [natural[0] * scale / 100.0, natural[1] * scale / 100.0];
            }
            ui.end_row();
            row_label(ui, "");
            ui.checkbox(keep, "Keep proportions");
            ui.end_row();
        });
        if *keep {
            let ratio = natural[1] / natural[0].max(1e-6);
            if changed_w {
                a.size_in[1] = a.size_in[0] * ratio;
            } else if changed_h {
                a.size_in[0] = a.size_in[1] / ratio;
            }
        }
        ui.add_space(6.0);
        let mut action = None;
        ui.horizontal_wrapped(|ui| {
            for (label, act) in [("Centre across", 0), ("Top of platen", 1), ("Fit platen", 2), ("Actual size", 3)] {
                if ui.button(label).clicked() {
                    action = Some(act);
                }
            }
        });
        let (pw, ph) = (self.job.platen().width_in, self.job.platen().height_in);
        if let Some(a) = &mut self.job.artwork {
            match action {
                Some(1) => a.pos_in[1] = 0.5,
                Some(2) => {
                    let k = (pw / natural[0]).min((ph - 0.5) / natural[1]);
                    a.size_in = [natural[0] * k, natural[1] * k];
                    a.pos_in = [0.0, 0.5];
                }
                Some(3) => a.size_in = natural,
                _ => {}
            }
        }
        if action.is_some_and(|a| a != 1) || action == Some(0) {
            self.job.centre_artwork();
        }
        self.job.clamp_artwork();
    }
}

fn colour_replacement_tab(ui: &mut egui::Ui) {
    ui.label(RichText::new("Colour replacement").strong());
    ui.add_space(4.0);
    ui.label(
        "Pick a colour in the design and swap it for another before printing, for example to match a \
         brand colour exactly. This comes after the colour engine.",
    );
}

fn preset_icon(p: &egui::Painter, c: egui::Pos2, inks: Inks) {
    let dots = [Color32::from_rgb(0, 174, 239), Color32::from_rgb(236, 0, 140), Color32::from_rgb(255, 222, 0), Color32::from_gray(30)];
    match inks {
        Inks::WhiteOnly => {
            p.circle_stroke(c, 8.0, Stroke::new(1.5, Color32::from_gray(150)));
        }
        _ => {
            for (i, col) in dots.iter().enumerate() {
                let off = [Vec2::new(-4.5, -4.5), Vec2::new(4.5, -4.5), Vec2::new(-4.5, 4.5), Vec2::new(4.5, 4.5)][i];
                p.circle_filled(c + off, 5.0, *col);
            }
            if inks == Inks::ColourAndWhite {
                p.circle(c, 4.0, Color32::WHITE, Stroke::new(1.0, Color32::from_gray(150)));
            }
        }
    }
}

fn settings_grid(ui: &mut egui::Ui, id: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::Grid::new(id).num_columns(2).min_col_width(170.0).spacing([12.0, 8.0]).show(ui, add);
}

fn row_label(ui: &mut egui::Ui, text: &str) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.label(RichText::new(text).color(LINK));
    });
}

fn combo(ui: &mut egui::Ui, id: &str, value: &mut usize, options: &[&str]) {
    egui::ComboBox::from_id_salt(id)
        .width(260.0)
        .selected_text(options.get(*value).copied().unwrap_or(""))
        .show_ui(ui, |ui| {
            for (i, o) in options.iter().enumerate() {
                ui.selectable_value(value, i, *o);
            }
        });
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if let Some(path) = self.pending_open.take() {
            self.open(&ctx, path);
        }
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).collect());
        if let Some(path) = dropped.into_iter().next() {
            self.open(&ctx, path);
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::O)) {
            self.open_dialog(&ctx);
        }

        self.printer_bar(ui);
        self.side_panel(ui);
        self.toolbar(ui);
        self.zoom_bar(ui);
        egui::CentralPanel::default().frame(egui::Frame::new()).show(ui, |ui| {
            canvas::show(ui, &mut self.view, &mut self.job);
        });

        if self.about {
            egui::Window::new("About DTG Creator Max").collapsible(false).open(&mut self.about).show(&ctx, |ui| {
                ui.label(format!("DTG Creator Max {}", env!("CARGO_PKG_VERSION")));
                ui.label("Open-source software for printing on the Epson SureColor SC-F2000.");
                ui.label("Free software under the GNU GPL, version 3 or later.");
                ui.hyperlink("https://github.com/Numbat11/dtg-creator-max");
                ui.add_space(6.0);
                ui.label(RichText::new("Not made, endorsed or supported by Epson.").small());
            });
        }

        self.take_screenshot(&ctx);
    }
}

impl App {
    fn take_screenshot(&mut self, ctx: &egui::Context) {
        let Some(path) = self.screenshot.clone() else { return };
        self.frames += 1;
        if self.frames == 5 {
            let slot = self.shot.clone();
            ctx.request_screenshot(move |image| *slot.lock().unwrap() = Some(image));
        }
        let taken = self.shot.lock().unwrap().take();
        if let Some(image) = taken {
            let rgb: Vec<u8> = image.pixels.iter().flat_map(|p| [p.r(), p.g(), p.b()]).collect();
            if let Err(e) = f2000::png::write_rgb(&path, image.size[0], image.size[1], &rgb) {
                eprintln!("couldn't save the screenshot: {e}");
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint();
    }
}
