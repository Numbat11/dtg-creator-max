// SPDX-License-Identifier: GPL-3.0-or-later
//! `f2000`: inspect, render, verify and rebuild Epson SC-F2000 print files.

use f2000::prn::{self, Item};
use f2000::{gcr, png, raster, render};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

const USAGE: &str = "\
f2000: Epson SC-F2000 print files (.prn) and Garment Creator jobs (.gcr)

usage:
  f2000 info <file.prn>               summarise the header, settings and pages
  f2000 render <file.prn> <out-dir>   write white.png, colour.png, on-black.png, channels.txt
  f2000 verify <file.prn>...          check this crate reproduces each file byte for byte
  f2000 rebuild <in.prn> <out.prn>    decode to dot planes and write the job again
  f2000 gcr <file.gcr> [<out-dir>]    show a job's settings; optionally save artwork.png
                                      and settings.json
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["info", file] => info(Path::new(file)),
        ["render", file, out] => render_cmd(Path::new(file), Path::new(out)),
        ["verify", files @ ..] if !files.is_empty() => verify_cmd(files),
        ["rebuild", input, output] => rebuild_cmd(Path::new(input), Path::new(output)),
        ["gcr", file] => gcr_cmd(Path::new(file), None),
        ["gcr", file, out] => gcr_cmd(Path::new(file), Some(Path::new(out))),
        _ => {
            eprint!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

type Res = Result<bool, Box<dyn std::error::Error>>;

fn printable(data: &[u8]) -> String {
    let text = data.iter().all(|&b| (0x20..0x7F).contains(&b)) && !data.is_empty();
    let s = if text { String::from_utf8_lossy(data).into_owned() } else { hex(data) };
    if s.len() > 72 { format!("{}...", &s[..72]) } else { s }
}

fn hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ")
}

fn info(path: &Path) -> Res {
    let bytes = std::fs::read(path)?;
    let items = prn::parse(&bytes)?;
    println!("{} ({} bytes, {} items)", path.display(), bytes.len(), items.len());
    let mut remote_blocks = 0;
    for item in &items {
        match item {
            Item::LeadIn(n) => println!("lead-in: {n} bytes of 0x11"),
            Item::Ejl(line) => println!("EJL: {}", String::from_utf8_lossy(line)),
            Item::Remote { cmds, .. } => {
                remote_blocks += 1;
                println!("remote block {remote_blocks}:");
                for c in cmds {
                    println!("  {} [{}] {}", c.code_str(), c.data.len(), printable(&c.data));
                }
            }
            Item::Thumbnail { body, .. } => {
                for e in prn::thumbnail_entries(body) {
                    let what = match png::size(e.data) {
                        Some((w, h)) => format!("PNG {w}x{h}"),
                        None => printable(e.data),
                    };
                    println!("thumbnail entry {}: {} bytes, {what}", e.id, e.data.len());
                }
            }
            _ => {}
        }
    }
    let pages = raster::decode_pages(&items)?;
    let totals = render::ink_totals(&pages);
    for page in &pages {
        let f = page.format;
        let h = page.height();
        let w_px = f.width_px.unwrap_or(0);
        println!(
            "page {}: {}x{} dpi, {} x {} px ({:.2} x {:.2} in), {}-bit dots",
            page.index,
            f.hres,
            f.vres,
            w_px,
            h,
            w_px as f64 / f.hres as f64,
            h as f64 / f.vres as f64,
            page.bits
        );
        let strips = |ch: u8| {
            let mut n = 0;
            let mut current = 0usize;
            for it in &items {
                match it {
                    Item::Ctl(prn::FF) => current += 1,
                    Item::Raster(s) if current == page.index && s.channel == ch => n += 1,
                    _ => {}
                }
            }
            n
        };
        for &ch in page.planes.keys() {
            println!(
                "  channel {ch:>2} {:<8} {:>5} strips, ink units {}",
                render::channel_name(ch),
                strips(ch),
                if pages.iter().filter(|p| p.planes.contains_key(&ch)).count() == 1 {
                    totals[&ch].to_string()
                } else {
                    "(shared across pages)".into()
                }
            );
        }
    }
    Ok(true)
}

fn render_cmd(path: &Path, out: &Path) -> Res {
    let t = Instant::now();
    let items = prn::parse(&std::fs::read(path)?)?;
    let pages = raster::decode_pages(&items)?;
    let p = render::preview(&pages);
    std::fs::create_dir_all(out)?;
    png::write_rgb(&out.join("white.png"), p.width, p.height, &p.white)?;
    png::write_rgb(&out.join("colour.png"), p.width, p.height, &p.colour)?;
    png::write_rgb(&out.join("on-black.png"), p.width, p.height, &p.on_black)?;
    let mut txt = format!("source: {}\npreview: {}x{} px at {} dpi\n", path.display(), p.width, p.height, 1440 / render::BLOCK_PX);
    for (ch, ink) in render::ink_totals(&pages) {
        txt += &format!("channel {ch}: {}, ink units {ink}\n", render::channel_name(ch));
    }
    std::fs::write(out.join("channels.txt"), &txt)?;
    print!("{txt}");
    println!("wrote {} in {:.1}s", out.display(), t.elapsed().as_secs_f64());
    Ok(true)
}

fn verify_cmd(files: &[&str]) -> Res {
    let mut all = true;
    for file in files {
        let t = Instant::now();
        let bytes = std::fs::read(file)?;
        let v = f2000::verify(&bytes)?;
        let says = |d: Option<usize>| match d {
            None => "identical".to_string(),
            Some(at) => format!("DIFFERENT from offset {at}"),
        };
        println!("{file}");
        println!("  parse + write:              {} ({} bytes)", says(v.container_diff), v.file_len);
        println!("  strips re-compressed:       {}/{} identical", v.strips_identical, v.strips);
        println!("  rebuilt from dot planes:    {} ({} bytes)", says(v.rebuild_diff), v.rebuild_len);
        println!("  ({:.1}s)", t.elapsed().as_secs_f64());
        all &= v.all_identical();
    }
    Ok(all)
}

fn rebuild_cmd(input: &Path, output: &Path) -> Res {
    let bytes = std::fs::read(input)?;
    let items = prn::parse(&bytes)?;
    let pages = raster::decode_pages(&items)?;
    let out = prn::write(&raster::rebuild(&items, &pages)?);
    std::fs::write(output, &out)?;
    match f2000::first_diff(&out, &bytes) {
        None => println!("wrote {} ({} bytes): identical to the input", output.display(), out.len()),
        Some(at) => println!("wrote {} ({} bytes): differs from the input from offset {at}", output.display(), out.len()),
    }
    Ok(true)
}

fn gcr_cmd(path: &Path, out: Option<&Path>) -> Res {
    let g = gcr::parse(&std::fs::read(path)?)?;
    let size = png::size(&g.artwork_png).map_or("not a PNG".into(), |(w, h)| format!("PNG {w}x{h}"));
    println!("{}: version {}, {} entries, artwork {} bytes ({size})", path.display(), g.version, g.entries, g.artwork_png.len());
    for (k, v) in g.settings() {
        println!("  {k} = {v}");
    }
    if let Some(out) = out {
        std::fs::create_dir_all(out)?;
        let (a, s): (PathBuf, PathBuf) = (out.join("artwork.png"), out.join("settings.json"));
        std::fs::write(&a, &g.artwork_png)?;
        std::fs::write(&s, &g.settings_json)?;
        println!("wrote {} and {}", a.display(), s.display());
    }
    Ok(true)
}
