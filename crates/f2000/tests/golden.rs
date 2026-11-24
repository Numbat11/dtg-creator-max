// SPDX-License-Identifier: GPL-3.0-or-later
//! Golden tests against files made by Epson's software.
//!
//! Those files are never part of the repository: they hold customers' artwork and Epson's
//! data. The tests read a private reference set from `$DTG_REFERENCE`, or from `reference/`
//! at the repository root (git-ignored; a symlink works):
//! - `prn/*.prn`: print files saved by Garment Creator 2.1.7;
//! - `prn/numbatweb-comau-white-lge.prn` and `.gcr`: a matched job and print file;
//! - `jobs/*.gcr`: older job files;
//! - `nozzle/H2000_NzlCheck_Wh.prn`: Epson's nozzle-check file.
//!
//! A test whose files are missing prints a note and passes, so the tests run anywhere.

use f2000::{gcr, prn, raster, render};
use std::path::{Path, PathBuf};

fn reference() -> PathBuf {
    std::env::var_os("DTG_REFERENCE")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../reference"))
}

fn read_or_skip(path: &Path) -> Option<Vec<u8>> {
    match std::fs::read(path) {
        Ok(b) => Some(b),
        Err(_) => {
            eprintln!("skipped: {} not found", path.display());
            None
        }
    }
}

fn prn_files() -> Vec<PathBuf> {
    let dir = reference().join("prn");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map(|rd| rd.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x == "prn")).collect())
        .unwrap_or_default();
    files.sort();
    if files.is_empty() {
        eprintln!("skipped: no .prn files in {}", dir.display());
    }
    files
}

/// Every saved job: parse + write, re-compress every strip, and rebuild from dot planes
/// must all reproduce Garment Creator's bytes exactly.
#[test]
fn saved_jobs_reproduce_byte_for_byte() {
    for path in prn_files() {
        let bytes = std::fs::read(&path).unwrap();
        let v = f2000::verify(&bytes).unwrap();
        assert!(v.all_identical(), "{}: {v:?}", path.display());
        assert!(v.strips > 1000, "{}: only {} strips", path.display(), v.strips);
    }
}

/// The matched pair: page formats and exact ink per channel.
#[test]
fn numbatweb_job_pages_and_ink() {
    let Some(bytes) = read_or_skip(&reference().join("prn/numbatweb-comau-white-lge.prn")) else { return };
    let items = prn::parse(&bytes).unwrap();
    let pages = raster::decode_pages(&items).unwrap();
    assert_eq!(pages.len(), 2);

    // page 0: white underbase, 1440 x 1440, 14 in wide, 3414/360 in long
    let w = &pages[0];
    assert_eq!((w.index, w.format.hres, w.format.vres), (0, 1440, 1440));
    assert_eq!((w.format.width_px, w.format.height_lines), (Some(20160), Some(13656)));
    assert_eq!(w.planes.keys().copied().collect::<Vec<_>>(), vec![63]);

    // page 1: colour, 1440 wide x 720 tall, same physical size
    let c = &pages[1];
    assert_eq!((c.index, c.format.hres, c.format.vres), (1, 1440, 720));
    assert_eq!(c.format.height_lines, Some(6828));
    assert_eq!(c.planes.keys().copied().collect::<Vec<_>>(), vec![0, 1, 2, 4]);

    let totals = render::ink_totals(&pages);
    let want = [(0, 30_981_288), (1, 16_464_162), (2, 13_842_300), (4, 17_282_982), (63, 373_302_741)];
    assert_eq!(totals.into_iter().collect::<Vec<_>>(), want.to_vec());

    // the embedded preview is a 140 x 168 PNG
    let thumb = items
        .iter()
        .find_map(|it| if let prn::Item::Thumbnail { body, .. } = it { Some(body) } else { None })
        .unwrap();
    let entries = prn::thumbnail_entries(thumb);
    assert_eq!(f2000::png::size(entries[0].data), Some((140, 168)));
}

/// Epson's own nozzle-check file uses bare RLE (mode 1) at 360 dpi. It must parse and
/// write back exactly and decode cleanly. (Its 360-line bands are not Garment Creator's
/// job layout, so it is not rebuilt.)
#[test]
fn nozzle_check_round_trips() {
    let Some(bytes) = read_or_skip(&reference().join("nozzle/H2000_NzlCheck_Wh.prn")) else { return };
    let items = prn::parse(&bytes).unwrap();
    assert_eq!(prn::write(&items), bytes);
    assert!(items.iter().any(|it| matches!(it, prn::Item::Raster(s) if s.mode == 1)));
    let pages = raster::decode_pages(&items).unwrap();
    assert!(pages.iter().all(|p| p.format.hres == 360 && p.format.vres == 360));
    let totals = render::ink_totals(&pages);
    assert!(totals.contains_key(&63) && totals.values().all(|&v| v > 0), "{totals:?}");
}

/// Job files: the matched pair's settings, and every 2019 job parses.
#[test]
fn job_files_parse() {
    let Some(bytes) = read_or_skip(&reference().join("prn/numbatweb-comau-white-lge.gcr")) else { return };
    let g = gcr::parse(&bytes).unwrap();
    assert_eq!(f2000::png::size(&g.artwork_png), Some((4592, 3200)));
    assert_eq!(g.setting("printerCode").as_deref(), Some("\"03R\""));
    assert_eq!(g.setting("inkSet").as_deref(), Some("4"));
    assert_eq!(g.setting("printModeColor").as_deref(), Some("300200"));
    assert_eq!(g.setting("printModeWhite").as_deref(), Some("300220"));
    assert_eq!(g.setting("platenWidth").as_deref(), Some("5040"));
    // saved by 2.1.7 in 2026: 76 settings; the 2019 jobs have 74
    assert_eq!(g.settings().len(), 76);

    let jobs = reference().join("jobs");
    for entry in std::fs::read_dir(&jobs).into_iter().flatten().flatten() {
        let p = entry.path();
        if p.extension().is_some_and(|x| x == "gcr") {
            let g = gcr::parse(&std::fs::read(&p).unwrap()).unwrap();
            assert_eq!(g.setting("printerCode").as_deref(), Some("\"03R\""), "{}", p.display());
            assert_eq!(g.settings().len(), 74, "{}", p.display());
        }
    }
}
