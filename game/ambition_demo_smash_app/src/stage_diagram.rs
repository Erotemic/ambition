//! Draw the stage, including the thing that kills you.
//!
//! Pure-Rust pixels (no GPU, windowing, or asset tree), so it runs wherever
//! the crate compiles. The `render_room_geometry` example of
//! `ambition_platformer2d_actor_monolith` is bound to that crate's room list,
//! and no other renderer draws blast margins.
//!
//! It draws the world bounds, the platform, the blast envelope, and where a
//! respawn lands.
//!
//! It lives in the lib, not in a bin, because `match_diagram` uses the same
//! drawing.

use ambition_platformer2d::engine_core::AabbExt;

const WIDTH: u32 = 480;
const HEIGHT: u32 = 320;

/// One fighter, as the diagram draws it.
pub struct DrawnFighter {
    pub aabb: ambition_platformer2d::engine_core::Aabb,
    /// Damage percent — `1.88` is 188%, and the bar is allowed past full for
    /// exactly that reason.
    pub percent: f32,
    pub stocks: u32,
}

/// The diagram, as PNG bytes.
pub fn render_stage_diagram() -> Vec<u8> {
    render_match_diagram(&[])
}

/// The stage with fighters on it.
pub fn render_match_diagram(fighters: &[DrawnFighter]) -> Vec<u8> {
    let room = ambition_demo_smash::smash_stage();
    let world = &room.world;
    let platform = world.blocks[0].aabb;
    let side_margin = world.edges.side.unwrap_or(world.edges.fall);
    let ceiling_margin = world.edges.rise.unwrap_or(world.edges.fall);
    let fall_margin = world.edges.fall;
    let respawn = ambition_demo_smash::respawn_placement(ambition_demo_smash::stage_centre(), 0);

    // Fit the BLAST ENVELOPE, not the world: the envelope is larger, and framing
    // to the world would crop the one boundary this diagram exists to show.
    // Each axis uses its own authored margin; fighter stages need not be
    // symmetric vertically or share their horizontal knockout distance.
    let span_x = world.size.x + side_margin * 2.0;
    let span_y = world.size.y + ceiling_margin + fall_margin;
    let scale = (WIDTH as f32 / span_x).min(HEIGHT as f32 / span_y) * 0.9;
    let to_px = |x: f32, y: f32| -> (i32, i32) {
        (
            (((x + side_margin) * scale) + (WIDTH as f32 - span_x * scale) / 2.0) as i32,
            (((y + ceiling_margin) * scale) + (HEIGHT as f32 - span_y * scale) / 2.0) as i32,
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
    let (bx0, by0) = to_px(-side_margin, -ceiling_margin);
    let (bx1, by1) = to_px(world.size.x + side_margin, world.size.y + fall_margin);
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

    // The fighters, each with a percent bar over its head. The bar may run
    // past full: the meter is unbounded, and clamping would hide that.
    for (index, fighter) in fighters.iter().enumerate() {
        let tint = if index % 2 == 0 {
            [110u8, 170, 240, 255]
        } else {
            [240u8, 150, 110, 255]
        };
        let (fx0, fy0) = to_px(fighter.aabb.left(), fighter.aabb.top());
        let (fx1, fy1) = to_px(fighter.aabb.right(), fighter.aabb.bottom());
        for y in fy0..=fy1 {
            for x in fx0..=fx1 {
                put(x, y, tint);
            }
        }
        // The percent bar, above the body. Length is percent-relative, so 188%
        // is visibly longer than the body is wide.
        let bar_len = ((fx1 - fx0).max(6) as f32 * fighter.percent.max(0.0)) as i32;
        for step in 0..bar_len {
            put(fx0 + step, fy0 - 4, [230, 90, 90, 255]);
            put(fx0 + step, fy0 - 5, [230, 90, 90, 255]);
        }
        // Stocks, as ticks under the body.
        for stock in 0..fighter.stocks as i32 {
            for dx in 0..3 {
                put(fx0 + stock * 5 + dx, fy1 + 4, [120, 220, 140, 255]);
            }
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
/// Hand-rolled: this crate depends only on `ambition_platformer2d`,
/// `ambition_demo_smash`, and `bevy`, and an encoder dependency to draw a few
/// rectangles would break that rule.
fn encode_png(pixels: &[[u8; 4]]) -> Vec<u8> {
    fn crc32(bytes: &[u8]) -> u32 {
        let mut table = [0u32; 256];
        for (n, entry) in table.iter_mut().enumerate() {
            let mut c = n as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 {
                    0xEDB8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
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
