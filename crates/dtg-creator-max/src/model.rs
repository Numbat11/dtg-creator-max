// SPDX-License-Identifier: GPL-3.0-or-later
//! The job being edited: garment preset, platen, artwork placement and print settings.

/// A platen (the board the garment is loaded on). Sizes are the printable area.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Platen {
    pub name: &'static str,
    pub width_in: f32,
    pub height_in: f32,
    /// Garment Creator's `platenType`, where known.
    pub gc_type: Option<u32>,
}

pub const PLATENS: &[Platen] = &[
    Platen { name: "Medium  14 × 16 in", width_in: 14.0, height_in: 16.0, gc_type: Some(1) },
    Platen { name: "Large  16 × 20 in", width_in: 16.0, height_in: 20.0, gc_type: Some(3) },
    Platen { name: "Small  10 × 12 in", width_in: 10.0, height_in: 12.0, gc_type: None },
    Platen { name: "Extra small  7 × 8 in", width_in: 7.0, height_in: 8.0, gc_type: None },
    Platen { name: "Sleeve  4 × 4 in", width_in: 4.0, height_in: 4.0, gc_type: None },
    Platen { name: "Grooved, for polos and zips  14 × 16 in", width_in: 14.0, height_in: 16.0, gc_type: None },
];

/// Which inks a preset prints with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inks {
    Colour,
    ColourAndWhite,
    WhiteOnly,
}

impl Inks {
    pub fn label(self) -> &'static str {
        match self {
            Inks::Colour => "Colour",
            Inks::ColourAndWhite => "Colour + white underbase",
            Inks::WhiteOnly => "White only",
        }
    }
}

/// A garment preset. Values marked `confirmed` were read from jobs and print files made by
/// Epson's software; the others are placeholders until one is captured.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Preset {
    pub name: &'static str,
    pub shirt: [u8; 3],
    pub inks: Inks,
    /// Pure black in the artwork is left unprinted so the garment shows through.
    pub garment_black: bool,
    pub ink_set: u32,
    /// Print-mode base codes; quality adds 0, 1 or 2 (colour) or 10, 20 or 30 (white).
    pub colour_mode: Option<u32>,
    pub white_mode: Option<u32>,
    pub garment_id: i32,
    pub confirmed: bool,
}

pub const PRESETS: &[Preset] = &[
    Preset {
        name: "Black T-shirt (use the garment's black)",
        shirt: [22, 22, 24],
        inks: Inks::ColourAndWhite,
        garment_black: true,
        ink_set: 4,
        colour_mode: Some(300200),
        white_mode: Some(300200),
        garment_id: -20,
        confirmed: true,
    },
    Preset {
        name: "Light T-shirt (standard)",
        shirt: [238, 238, 236],
        inks: Inks::Colour,
        garment_black: false,
        ink_set: 0,
        colour_mode: Some(300000),
        white_mode: None,
        garment_id: -40,
        confirmed: true,
    },
    Preset {
        name: "Dark T-shirt (standard)",
        shirt: [34, 44, 74],
        inks: Inks::ColourAndWhite,
        garment_black: false,
        ink_set: 1,
        colour_mode: Some(300100),
        white_mode: Some(300100),
        garment_id: -60,
        confirmed: true,
    },
    Preset {
        name: "Dark T-shirt (white ink only)",
        shirt: [34, 44, 74],
        inks: Inks::WhiteOnly,
        garment_black: false,
        ink_set: 1,
        colour_mode: None,
        white_mode: Some(300100),
        garment_id: 0,
        confirmed: false,
    },
    Preset {
        name: "Light T-shirt (fast colour)",
        shirt: [238, 238, 236],
        inks: Inks::Colour,
        garment_black: false,
        ink_set: 0,
        colour_mode: Some(300000),
        white_mode: None,
        garment_id: 0,
        confirmed: false,
    },
];

pub const COLOUR_QUALITY: &[&str] = &["Level 1: 8 passes (fastest)", "Level 2: 10 passes", "Level 3: 16 passes (finest)"];
pub const WHITE_QUALITY: &[&str] = &["Fast: 1440 × 720 dpi", "Standard: 1440 × 1440 dpi", "Light layer: 1440 × 1440 dpi"];

/// Everything on the Print settings tab.
#[derive(Debug, Clone, PartialEq)]
pub struct PrintSettings {
    pub colour_quality: usize,
    pub white_quality: usize,
    pub bidirectional: bool,
    pub drying_time_s: u32,
    pub white_layers: u32,
    /// How far the underbase is pulled in from the design's edge (Garment Creator's
    /// `underShapeWidth`).
    pub underbase_inset: u32,
    pub density_colour: i32,
    pub density_underbase: i32,
    pub density_design_white: i32,
    pub brightness: i32,
    pub saturation: i32,
    pub contrast: i32,
    pub red: i32,
    pub green: i32,
    pub blue: i32,
}

impl Default for PrintSettings {
    fn default() -> Self {
        PrintSettings {
            colour_quality: 0,
            white_quality: 1,
            bidirectional: true,
            drying_time_s: 0,
            white_layers: 1,
            underbase_inset: 2,
            density_colour: 0,
            density_underbase: 0,
            density_design_white: 0,
            brightness: 0,
            saturation: 0,
            contrast: 0,
            red: 0,
            green: 0,
            blue: 0,
        }
    }
}

/// Where the artwork came from.
#[derive(Debug, Clone, PartialEq)]
pub enum Source {
    Image,
    Job,
    /// A finished print file: shown as its actual dots, and not movable.
    PrintFile,
}

/// The design on the platen.
pub struct Artwork {
    pub texture: eframe::egui::TextureHandle,
    pub file_name: String,
    pub source: Source,
    /// Pixel size of the original image and its resolution.
    pub pixels: [usize; 2],
    pub dpi: f32,
    /// Top-left corner, in inches from the platen's top-left corner.
    pub pos_in: [f32; 2],
    /// Printed size in inches.
    pub size_in: [f32; 2],
}

impl Artwork {
    /// Size at 100 %: pixels at the image's own resolution.
    pub fn natural_size_in(&self) -> [f32; 2] {
        [self.pixels[0] as f32 / self.dpi, self.pixels[1] as f32 / self.dpi]
    }

    pub fn scale_percent(&self) -> f32 {
        100.0 * self.size_in[0] / self.natural_size_in()[0].max(1e-6)
    }
}

pub struct Job {
    pub name: String,
    pub copies: u32,
    pub preset: usize,
    pub platen: usize,
    pub settings: PrintSettings,
    pub artwork: Option<Artwork>,
    pub keep_aspect: bool,
}

impl Default for Job {
    fn default() -> Self {
        Job {
            name: "untitled".into(),
            copies: 1,
            preset: 0,
            platen: 0,
            settings: PrintSettings::default(),
            artwork: None,
            keep_aspect: true,
        }
    }
}

impl Job {
    pub fn preset(&self) -> &'static Preset {
        &PRESETS[self.preset]
    }

    pub fn platen(&self) -> &'static Platen {
        &PLATENS[self.platen]
    }

    /// Keep the artwork inside the platen.
    pub fn clamp_artwork(&mut self) {
        let (pw, ph) = (self.platen().width_in, self.platen().height_in);
        if let Some(a) = &mut self.artwork {
            a.pos_in[0] = a.pos_in[0].clamp(0.0, (pw - a.size_in[0]).max(0.0));
            a.pos_in[1] = a.pos_in[1].clamp(0.0, (ph - a.size_in[1]).max(0.0));
        }
    }

    /// Centre the artwork across the platen, keeping its height position.
    pub fn centre_artwork(&mut self) {
        let pw = self.platen().width_in;
        if let Some(a) = &mut self.artwork {
            a.pos_in[0] = ((pw - a.size_in[0]) / 2.0).max(0.0);
        }
    }

    /// The full print-mode codes for the current preset and quality settings.
    pub fn print_modes(&self) -> (Option<u32>, Option<u32>) {
        let p = self.preset();
        let colour = p.colour_mode.map(|m| m + self.settings.colour_quality as u32);
        let white = p.white_mode.map(|m| m + 10 * (self.settings.white_quality as u32 + 1));
        (colour, white)
    }
}
