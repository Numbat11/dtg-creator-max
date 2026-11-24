// SPDX-License-Identifier: GPL-3.0-or-later
//! Read, render and write Epson SC-F2000 print files (`.prn`), as saved by Epson Garment
//! Creator 2.1.7, and read Garment Creator job files (`.gcr`).
//!
//! - [`prn`]: lossless parse and write of the command stream.
//! - [`rle`]: Epson's run-length coding, matching Garment Creator's encoder exactly.
//! - [`raster`]: dot planes per page and channel; decode a job to planes and back.
//! - [`render`]: preview images and ink totals.
//! - [`gcr`]: job files (artwork plus settings).
//!
//! The format is described in `docs/print-file-format.md` at the repository root.

pub mod gcr;
pub mod png;
pub mod prn;
pub mod raster;
pub mod render;
pub mod rle;

use std::fmt;
use std::path::Path;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Parse(prn::ParseError),
    Raster(raster::RasterError),
    Gcr(gcr::GcrError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "{e}"),
            Error::Parse(e) => write!(f, "{e}"),
            Error::Raster(e) => write!(f, "{e}"),
            Error::Gcr(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}
impl From<prn::ParseError> for Error {
    fn from(e: prn::ParseError) -> Self {
        Error::Parse(e)
    }
}
impl From<raster::RasterError> for Error {
    fn from(e: raster::RasterError) -> Self {
        Error::Raster(e)
    }
}
impl From<gcr::GcrError> for Error {
    fn from(e: gcr::GcrError) -> Self {
        Error::Gcr(e)
    }
}

/// Read and parse a `.prn` file.
pub fn read_prn(path: impl AsRef<Path>) -> Result<Vec<prn::Item>, Error> {
    Ok(prn::parse(&std::fs::read(path)?)?)
}

/// Read a `.gcr` job file.
pub fn read_gcr(path: impl AsRef<Path>) -> Result<gcr::Gcr, Error> {
    Ok(gcr::parse(&std::fs::read(path)?)?)
}

/// Results of [`verify`]: each check passes when Garment Creator's bytes are reproduced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verification {
    pub file_len: usize,
    /// First differing offset after parse and write, or `None` if identical.
    pub container_diff: Option<usize>,
    /// Mode 4/5 strips, and how many re-compress to exactly the stored bytes.
    pub strips: usize,
    pub strips_identical: usize,
    /// First differing offset after decoding to dot planes and re-encoding, or `None`.
    pub rebuild_diff: Option<usize>,
    pub rebuild_len: usize,
}

impl Verification {
    pub fn all_identical(&self) -> bool {
        self.container_diff.is_none() && self.strips == self.strips_identical && self.rebuild_diff.is_none()
    }
}

/// Run the three byte-for-byte checks on one `.prn` file's bytes:
/// 1. parse and write the command stream;
/// 2. decompress and re-compress every framed strip;
/// 3. decode every page to dot planes and re-encode the raster.
pub fn verify(bytes: &[u8]) -> Result<Verification, Error> {
    let items = prn::parse(bytes)?;
    let container_diff = first_diff(&prn::write(&items), bytes);
    let mut strips = 0;
    let mut strips_identical = 0;
    for item in &items {
        if let prn::Item::Raster(s) = item {
            if s.flag.is_some() {
                strips += 1;
                let raw = s.raw().map_err(|err| prn::ParseError::Rle { at: 0, err })?;
                if prn::Strip::compress(s.channel, s.bits, s.bytes_per_line, s.lines, &raw) == *s {
                    strips_identical += 1;
                }
            }
        }
    }
    let pages = raster::decode_pages(&items)?;
    let rebuilt = prn::write(&raster::rebuild(&items, &pages)?);
    Ok(Verification {
        file_len: bytes.len(),
        container_diff,
        strips,
        strips_identical,
        rebuild_diff: first_diff(&rebuilt, bytes),
        rebuild_len: rebuilt.len(),
    })
}

/// First offset where `a` and `b` differ (including one being longer), or `None`.
pub fn first_diff(a: &[u8], b: &[u8]) -> Option<usize> {
    match a.iter().zip(b).position(|(x, y)| x != y) {
        Some(i) => Some(i),
        None if a.len() != b.len() => Some(a.len().min(b.len())),
        None => None,
    }
}
