//! **Draw the stage, including the thing that kills you.**
//!
//! `cargo run -p ambition_demo_smash_app --bin stage_diagram -- [OUT.png]`
//!
//! Pure-Rust pixels — no GPU, no windowing, no asset tree — so it runs anywhere
//! the crate compiles. It is the geometry half of seeing a room, in the tradition
//! of `ambition_actors`' `render_room_geometry` example, and it exists separately
//! for two reasons that are both findings:
//!
//! 1. **that example is bound to `ambition_actors`' own room list**, so a demo
//!    cannot render its own stage with it at all; and
//! 2. **nothing in the tree draws BLAST MARGINS.** Every existing renderer draws
//!    what a body can stand on. On this stage the interesting geometry is the
//!    line past which a body stops existing, and it is invisible in every view we
//!    have — which is exactly the kind of thing that is wrong for a week because
//!    nobody can look at it.
//!
//! So this draws four things: the world bounds, the platform, the blast envelope,
//! and where a respawn lands. Three of them are unrepresentable in any other view
//! of this room.

use ambition::engine_core::AabbExt;

const WIDTH: u32 = 480;
const HEIGHT: u32 = 320;

fn main() {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/smash_stage.png".to_string());
    let png = render_stage_diagram();
    std::fs::write(&out, png).expect("write the stage diagram");
    println!("[stage_diagram] wrote {out}");
    println!(
        "[stage_diagram] white = world bounds, solid = platform, dashed = blast \
         envelope (past this a body is gone), dot = respawn"
    );
}

/// The diagram, as PNG bytes.
pub fn render_stage_diagram() -> Vec<u8> {
    let room = ambition_demo_smash::smash_stage();
    let world = &room.world;
    let platform = world.blocks[0].aabb;
    let margin = world.side_blast_margin.unwrap_or(world.blast_margin);
    let respawn = ambition_demo_smash::respawn_placement(ambition_demo_smash::stage_centre());

    // Fit the BLAST ENVELOPE, not the world: the envelope is larger, and framing
    // to the world would crop the one boundary this diagram exists to show.
    let span_x = world.size.x + margin * 2.0;
    let span_y = world.size.y + margin * 2.0;
    let scale = (WIDTH as f32 / span_x).min(HEIGHT as f32 / span_y) * 0.9;
    let to_px = |x: f32, y: f32| -> (i32, i32) {
        (
            (((x + margin) * scale) + (WIDTH as f32 - span_x * scale) / 2.0) as i32,
            (((y + margin) * scale) + (HEIGHT as f32 - span_y * scale) / 2.0) as i32,
        )
    };

    let mut pixels = vec![[12u8, 14, 20, 255]; (WIDTH * HEIGHT) as usize];
    let mut put = |x: i32, y: i32, rgba: [u8; 4]| {
        if x >= 0 && y >= 0 && (x as u32) < WIDTH && (y as u32) < HEIGHT {
            pixels[(y as u32 * WIDTH + x as u32) as usize] = rgba;
        }
    };
    let mut rect = |x0: i32, y0: i32, x1: i32, y1: i32, rgba: [u8; 4], dashed: bool| {
        for x in x0..=x1 {
            if !dashed || (x / 4) % 2 == 0 {
                put(x, y0, rgba);
                put(x, y1, rgba);
            }
        }
        for y in y0..=y1 {
            if !dashed || (y / 4) % 2 == 0 {
                put(x0, y, rgba);
                put(x1, y, rgba);
            }
        }
    };

    // The blast envelope: the boundary a body crosses to stop existing.
    let (bx0, by0) = to_px(-margin, -margin);
    let (bx1, by1) = to_px(world.size.x + margin, world.size.y + margin);
    rect(bx0, by0, bx1, by1, [220, 70, 70, 255], true);

    // The world.
    let (wx0, wy0) = to_px(0.0, 0.0);
    let (wx1, wy1) = to_px(world.size.x, world.size.y);
    rect(wx0, wy0, wx1, wy1, [90, 96, 110, 255], false);

    // The platform — filled, because it is the only thing you can stand on.
    let (px0, py0) = to_px(platform.left(), platform.top());
    let (px1, py1) = to_px(platform.right(), platform.bottom());
    for y in py0..=py1 {
        for x in px0..=px1 {
            put(x, y, [210, 214, 226, 255]);
        }
    }

    // The respawn.
    let (rx, ry) = to_px(respawn.x, respawn.y);
    for dy in -2..=2 {
        for dx in -2..=2 {
            if dx * dx + dy * dy <= 4 {
                put(rx + dx, ry + dy, [120, 220, 140, 255]);
            }
        }
    }

    encode_png(&pixels)
}

/// A minimal PNG: signature, IHDR, one STORED-deflate IDAT, IEND.
///
/// Hand-rolled for the same reason the external-consumer fixture hand-rolls one
/// — this crate's dependency rule is `ambition` + `ambition_demo_smash` + `bevy`,
/// and reaching for an encoder to draw five rectangles would spend the rule this
/// crate exists to hold.
fn encode_png(pixels: &[[u8; 4]]) -> Vec<u8> {
    fn crc32(bytes: &[u8]) -> u32 {
        let mut table = [0u32; 256];
        for (n, entry) in table.iter_mut().enumerate() {
            let mut c = n as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            }
            *entry = c;
        }
        let mut c = 0xFFFF_FFFFu32;
        for byte in bytes {
            c = table[((c ^ *byte as u32) & 0xFF) as usize] ^ (c >> 8);
        }
        c ^ 0xFFFF_FFFF
    }
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        let mut crc_input = kind.to_vec();
        crc_input.extend_from_slice(data);
        out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
    }

    let mut raw = Vec::with_capacity(((WIDTH * 4 + 1) * HEIGHT) as usize);
    for y in 0..HEIGHT {
        raw.push(0); // filter: none
        for x in 0..WIDTH {
            raw.extend_from_slice(&pixels[(y * WIDTH + x) as usize]);
        }
    }

    // zlib: header, STORED deflate blocks, adler32.
    let mut z = vec![0x78, 0x01];
    for (index, block) in raw.chunks(65_535).enumerate() {
        let last = (index + 1) * 65_535 >= raw.len();
        z.push(if last { 1 } else { 0 });
        z.extend_from_slice(&(block.len() as u16).to_le_bytes());
        z.extend_from_slice(&(!(block.len() as u16)).to_le_bytes());
        z.extend_from_slice(block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for byte in &raw {
        a = (a + *byte as u32) % 65_521;
        b = (b + a) % 65_521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());

    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&WIDTH.to_be_bytes());
    ihdr.extend_from_slice(&HEIGHT.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]); // 8-bit RGBA
    chunk(&mut png, b"IHDR", &ihdr);
    chunk(&mut png, b"IDAT", &z);
    chunk(&mut png, b"IEND", &[]);
    png
}
