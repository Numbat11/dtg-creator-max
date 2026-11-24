# SC-F2000 print file format

What an Epson SureColor SC-F2000 (White Edition) receives when Epson Garment Creator 2.1.7
prints or saves a print file (`.prn`). It was worked out from print files the author's own
Garment Creator saved, for interoperability. Every rule marked *confirmed* is checked by
this repository's golden tests: they decode real print files and write them again byte for
byte.

Meanings marked *(likely)* are inferences still to be confirmed.

## Overview

A print file is Epson ESC/P2 with Epson's "remote mode" commands. A job on a dark garment
has two pages, printed one after the other on the same garment:

1. **The white underbase**, on the white ink channel, at 1440 × 1440 dpi.
2. **The colour**, on the cyan, magenta, yellow and black channels, at 1440 dpi across and
   720 dpi down.

## Layout

1. **Lead-in:** 16,384 bytes of `0x11`.
2. **EJL preamble:** `00 00 00 1B 01 "@EJL 1284.4" 0A "@EJL" 0A`.
3. **`ESC @`, then a remote-mode block** `ESC ( R 08 00 00 "REMOTE1"`. Each remote command
   is a two-letter code, a u16 LE length and data; the block ends with `ESC 00 00 00`.
   - `JS`: job start plus job name.
   - `JX`: `00 00 02 08 …`.
   - `JH`: the job name again.
   - `JU`: the application name.
   - `HD` ×3: host details (application, operating system, application id).
   - `MI 00 03 01 01`, `DN 00 01 00 00 00`.
4. **Thumbnail:** `ESC ( R 0A 00 00 "THUMBNAIL"`, a u32 LE total length, then entries of
   u32 id, u32 length and data.
   - Entry 1 is a 140 × 168 PNG preview of the design.
   - Entry 2 is a u32 followed by another 140-pixel-wide PNG *(likely a second preview)*.
5. **Each page** starts with `ESC @`, two remote blocks and `ESC @`:
   - `EX … 38 0P 00 00`, where P is the page number counting from 1.
   - `DR 00 40 00 00`, `DR 00 01 00 00`, `IK 00 61` (white page) or `IK 00 01` (colour
     page), and a 4,775-byte `EX` block that is the same in every job seen so far *(likely
     calibration or ink curves)*.
6. **Page setup** (ESC/P2):

   | Command | Page 1: white | Page 2: colour |
   |---|---|---|
   | `ESC ( G` | `01` graphics mode | same |
   | `ESC U` | `00` | same |
   | `ESC ( K` | `00 00` | same |
   | `ESC ( i` | `01 F0 00` microweave, done in the printer | same |
   | `ESC ( e` dot size | `00 21` | `00 11` |
   | `ESC ( E` | `00 00` | same |
   | `ESC ( U` units | `08 02 02 40 0B`: base 2880, page 1/360 in, vertical 1/1440, horizontal 1/1440 | `08 04 02 40 0B`: vertical 1/720 |
   | `ESC ( C` | page length, u32, in 1/360 in (the design's height) | same |
   | `ESC ( c`, `ESC ( S` | page format; `( S` is width and length as u32, width 5040 (14 in) on the standard platen | same |
   | `ESC ( D` raster | `40 38 0A 0A`: 1440 × 1440 dpi | `40 38 14 0A`: 1440 across × 720 down |

7. **Raster** (see below), then `FF` to end the page.
8. **Trailer** after the last page: `ESC @`, a remote block with `EX … 38 00 00 00`, a
   remote block with `LD` and `JE 00`, and `ESC @`.

## Raster

- `ESC ( v` (int32) moves down, in the page's vertical units.
- `ESC ( $` (u32) sets the absolute horizontal position, in 1/1440 in.
- Each strip is `ESC i <channel> <mode> <bits> <bytes per line u16> <lines u16>`, then its
  data, then `CR`.
  - Bits is 2: four pixels per byte, leftmost in the top bits. Each pixel is a dot size from
    0 (none) to 3 (large).
  - **Mode 5:** `00`, a u32 LE length, then that many bytes of run-length coded data that
    expand to exactly bytes per line × lines.
  - **Mode 4:** the same framing, with raw data.
  - **Mode 1:** run-length data with no framing (Epson's nozzle-check file only).
- **Run-length coding:** a count byte `n < 0x80` is followed by `n + 1` literal bytes. A
  count byte `n ≥ 0x80` is followed by one byte that repeats `257 − n` times, so `0x80`
  means 129 repeats, unlike PackBits.
- **Channels** *(confirmed by rendering real jobs)*: 63 white, 0 black, 1 magenta, 2 cyan,
  4 yellow.
- Rows arrive in plain top-to-bottom order; the printer does the weaving.

## How Garment Creator writes the raster *(confirmed)*

Decoding three real jobs to dots and writing them again with these rules reproduces each
file byte for byte (8,115 strips):

- **Bands:** 16 lines each, on a fixed grid from the top of the page. Empty bands are
  skipped. The last band stops at the page length from `ESC ( C`.
- **Channel order within a band:** cyan (2), magenta (1), yellow (4), black (0). White
  (63) is on its own page. A channel with no dots in the band is left out.
- **Commands:** `ESC ( v` with the distance from the previous band, before each band's first
  strip only. Then for each strip: `ESC ( $` with the absolute x, the `ESC i` strip, and
  `CR`.
- **Crop:**
  - The strip starts at the first byte column with dots, rounded down to a multiple of 16
    pixels (4 bytes).
  - It runs to the last byte column with dots, with the width rounded up to an even number
    of bytes.
  - Exception: a strip whose dots all sit in its first byte stays 1 byte wide.
- **Compression:** each line is coded on its own, greedily from the left. Runs of 3 or more
  identical bytes become repeat tokens of at most 128 bytes. Everything else is literal, at
  most 127 bytes per token. Runs of 2 stay literal.
- **Mode:** 5 unless the run-length data would be larger than the raw data, then 4.

## Job files (.gcr)

Garment Creator's job file:
- **Header:** five u32 LE values: 1, 0, 2, the length of everything after the fourth value,
  and the PNG length.
- **Artwork:** the full-resolution PNG.
- **Settings:** URL-encoded JSON running to the end of the file. Version 2.1.7 writes 76
  settings; files from 2019 have 74.
- **Settings that matter for printing:**
  - Printer and inks: `printerCode` (`03R` is the SC-F2000 White Edition) and `inkSet`.
  - Print modes: `printModeColor` and `printModeWhite`.
  - White: `underWhite`, `designWhite`, `underShapeWidth`, `whiteLayerNum` and
    `colorLayerNum`.
  - Platen and placement: `platenType`, `platenWidth` and `platenHeight` (5040 × 5760 at
    360 dpi for the standard 14 × 16 in platen), `imagePosX/Y`, `scaleX/Y`,
    `resizeWidth/Height`.
  - Colour management: `inputProfile`, `cmykDefaultOutputProfile` and `renderingIntent`.

## Epson's nozzle-check file

Epson's own nozzle-check print file differs from jobs: 360 dpi, microweave off, mode-1
compression in 360-line bands, no thumbnail, no 4,775-byte block, and four white channel
numbers (63, 127, 191, 254).

## Still unknown

- What the remote settings (`JX`, `MI`, `DN`, `DR`, `IK` and the 4,775-byte `EX`) mean, and
  how they change with print mode, ink set and platen.
- Where white goes in the channel order if white and colour ever share a page.
- Whether an exact tie between raw and compressed size picks mode 4 or 5.
