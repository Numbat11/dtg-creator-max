# DTG Creator Max

Open-source, native Linux software for printing T-shirts on the **Epson SureColor SC-F2000**
direct-to-garment printer.

Epson's own software for the SC-F2000, Garment Creator, runs only on Windows and macOS, and
the printer itself is discontinued. DTG Creator Max aims to give its owners a native Linux
app instead: open your artwork, place it on the platen, choose the garment, and print.
No virtual machines, no Wine.

## Status

| Part | State |
|---|---|
| Print-file reader and writer ([`crates/f2000`](crates/f2000)) | **Working.** Reads, renders and writes SC-F2000 print files. Real Garment Creator jobs rebuild byte for byte. |
| Garment Creator job files (`.gcr`) | **Working:** artwork and settings. |
| Artwork to dots: colour conversion, white underbase, halftoning | Not started. The next big piece. |
| Sending to the printer over USB or network | Not started. |
| Desktop app | Not started. |

The print file format is documented in [`docs/print-file-format.md`](docs/print-file-format.md).

## Build and test

You need Rust (stable, via [rustup](https://rustup.rs)).

```bash
cargo build --release
cargo test
```

The `f2000` command-line tool can inspect, preview and check print files:

```bash
cargo run --release -p f2000 -- info job.prn
cargo run --release -p f2000 -- render job.prn previews/
```

## Ground rules

- **No Epson software or data in this repository:** no installers, libraries, colour
  profiles, halftone tables or print files made by Epson's software. Everything here is
  written from scratch.
- **No customers' artwork.** Tests that need real print files read them from a private
  folder outside the repository (see [`crates/f2000/tests/golden.rs`](crates/f2000/tests/golden.rs)).
- Format knowledge comes from studying files that Epson's software produced on the
  contributors' own machines, for interoperability with printers they own.

## Licence

DTG Creator Max is free software: you can redistribute it and/or modify it under the
terms of the GNU General Public License as published by the Free Software Foundation,
either version 3 of the License, or (at your option) any later version. See
[`LICENSE`](LICENSE).

Anyone may use it, including print shops printing commercially. If you distribute a
modified version, you must share your changes under the same licence. For other licence
terms, such as building it into a closed-source product, contact Numbat Web.

Copyright (C) 2026 Numbat Web. Contributions are welcome; see
[`CONTRIBUTING.md`](CONTRIBUTING.md).

## Trademarks

Epson, SureColor and Garment Creator are trademarks of Seiko Epson Corporation. DTG Creator
Max is an independent project. It is not made, endorsed or supported by Epson.
