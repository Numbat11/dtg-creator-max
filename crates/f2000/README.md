# f2000

The print-file library behind DTG Creator Max. It reads, renders and writes print files for
the Epson SureColor SC-F2000, and reads Garment Creator job files (`.gcr`).

- [`prn`](src/prn.rs): lossless parse and write of the print-file command stream.
- [`rle`](src/rle.rs): Epson's run-length coding, matching Garment Creator's encoder
  exactly.
- [`raster`](src/raster.rs): dot planes per page and ink channel; decode a job to planes and
  write planes back as a job.
- [`render`](src/render.rs): preview images and ink totals.
- [`gcr`](src/gcr.rs): job files (artwork plus settings).

The format is described in [`docs/print-file-format.md`](../../docs/print-file-format.md).

## Command-line tool

```bash
cargo run --release -p f2000 -- info job.prn            # header, settings, pages, ink per channel
cargo run --release -p f2000 -- render job.prn out/     # white.png, colour.png, on-black.png
cargo run --release -p f2000 -- verify job.prn ...      # byte-for-byte checks
cargo run --release -p f2000 -- rebuild in.prn out.prn  # decode to dots and write it again
cargo run --release -p f2000 -- gcr job.gcr [out/]      # job settings; optionally the artwork
```

## Tests

`cargo test` runs the unit tests and, if a reference set is present, the golden tests in
[`tests/golden.rs`](tests/golden.rs). See that file for the expected layout. The golden tests
check that files saved by Garment Creator come back byte for byte after parsing, after
re-compression, and after decoding to dots and re-encoding.
