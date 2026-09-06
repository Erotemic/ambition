//! Generate the art this crate claims to own.
//!
//! U1 let a consumer address and describe its own character sheet. The half
//! left open was that nothing here decodes a PNG, because this repo does not
//! commit binary or generated art — so the fixture authored a sheet describing
//! a file that did not exist, and "a consumer's character renders from consumer
//! art" stayed a claim about plumbing.
//!
//! A build script closes it without committing a byte. The image is emitted
//! into this crate's own asset tree, which is exactly the tree
//! `layered_asset_source` resolves `game://` against, so the generated file
//! travels the same path a third party's real art would.
//!
//! ## Why a PNG encoder lives in a fixture
//!
//! Because the fixture's dependency rule is `ambition_platformer2d` + `bevy` and nothing
//! else — that rule is the evidence it exists to produce, and reaching for
//! `image` to draw a rectangle would spend it. A build script may use `std`
//! alone, and a PNG of one uncompressed block is about eighty lines: signature,
//! IHDR, an IDAT holding a zlib stream of STORED deflate blocks, IEND, each
//! with its CRC. No filtering, no compression, no cleverness — the smallest
//! thing that is genuinely a PNG rather than a file named `.png`.

use std::path::Path;

const WIDTH: u32 = 32;
const HEIGHT: u32 = 48;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/sprites/outlander.png");
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).expect("the consumer's own sprite folder");
    }
    std::fs::write(&out, encode_png(&outlander_pixels())).expect("write the generated sprite");
}

/// A recognisable little figure: a lighter head band over a darker body, on
/// transparent background. Deliberately not art — it is a silhouette whose
/// EXTENT is checkable, so a test can assert the sheet's 32×48 frame matches
/// what was drawn rather than trusting the number in two places.
fn outlander_pixels() -> Vec<u8> {
    let mut rgba = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            // A 2px transparent margin all round, so the alpha bbox is a
            // strictly interior rectangle and a bbox derived from the image
            // cannot accidentally equal the frame.
            let inside = x >= 2 && x < WIDTH - 2 && y >= 2 && y < HEIGHT - 2;
            if !inside {
                continue;
            }
            let head = y < HEIGHT / 3;
            let (r, g, b) = if head {
                (0xE8, 0xC8, 0x90)
            } else {
                (0x3C, 0x6E, 0xA8)
            };
            let i = ((y * WIDTH + x) * 4) as usize;
            rgba[i] = r;
            rgba[i + 1] = g;
            rgba[i + 2] = b;
            rgba[i + 3] = 0xFF;
        }
    }
    rgba
}

fn encode_png(rgba: &[u8]) -> Vec<u8> {
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&WIDTH.to_be_bytes());
    ihdr.extend_from_slice(&HEIGHT.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]); // 8-bit, RGBA, no interlace
    write_chunk(&mut png, b"IHDR", &ihdr);

    // Scanlines, each prefixed with filter type 0 ("None").
    let mut raw = Vec::with_capacity(rgba.len() + HEIGHT as usize);
    for row in rgba.chunks_exact((WIDTH * 4) as usize) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    write_chunk(&mut png, b"IDAT", &zlib_stored(&raw));
    write_chunk(&mut png, b"IEND", &[]);
    png
}

/// A zlib stream whose deflate blocks are all STORED — valid, and the only
/// shape that needs no compressor.
fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01]; // deflate, 32K window, no preset dict
    let mut chunks = data.chunks(0xFFFF).peekable();
    while let Some(chunk) = chunks.next() {
        let final_block = u8::from(chunks.peek().is_none());
        out.push(final_block); // BFINAL, BTYPE = 00 (stored)
        let len = chunk.len() as u16;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(chunk);
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for byte in data {
        a = (a + u32::from(*byte)) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

fn write_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(kind);
    crc_input.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}
