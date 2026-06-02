//! Headless geometry-debug renderer (TODO #200, geometry slice).
//!
//! Renders a room's collision world to a PNG using a pure-Rust pixel
//! buffer — **no GPU / wgpu / windowing**, so it runs anywhere the
//! sandbox compiles (CI, this VM, a remote box). The point is to let an
//! agent or reviewer *see* a room's spatial layout (solids, one-ways,
//! hazards, pogo orbs, spawn) and verify authoring / collision bugs
//! without launching the visible binary or hand-reading LDtk JSON.
//!
//! This is the geometry half of the broader "headless screenshot"
//! verification path: it draws collision/volume *boxes* in world space,
//! not the sprite art. That covers the class of bugs that are about
//! *where things are* (room boundaries, mid-air doors, hurtbox vs body
//! envelope) rather than what they look like.
//!
//! Usage:
//!   cargo run -p ambition_sandbox --example render_room_geometry -- [ROOM_ID] [OUT.png]
//!
//! With no ROOM_ID it lists every room id and exits. Default output is
//! `/tmp/room_<id>.png`.

use ambition_sandbox as sb;
use image::{Rgba, RgbaImage};
use sb::engine_core as ae;

/// Longest edge of the output image, in pixels. Worlds scale down to fit.
const MAX_CANVAS_PX: u32 = 1000;
/// Padding around the world inside the canvas.
const MARGIN_PX: u32 = 16;

fn color_for(kind: &ae::BlockKind) -> Rgba<u8> {
    match kind {
        ae::BlockKind::Solid => Rgba([120, 124, 132, 255]), // gray
        ae::BlockKind::BlinkWall { .. } => Rgba([150, 90, 200, 255]), // purple
        ae::BlockKind::OneWay => Rgba([70, 120, 210, 255]), // blue
        ae::BlockKind::Hazard => Rgba([210, 70, 70, 255]),  // red
        ae::BlockKind::PogoOrb => Rgba([240, 200, 60, 255]), // gold
        ae::BlockKind::Rebound { .. } => Rgba([70, 200, 160, 255]), // teal
    }
}

/// World→image projection: uniform scale, world origin maps to the
/// top-left margin. World is y-down (matches the engine), so no y-flip.
struct Projection {
    scale: f32,
    world_min: ae::Vec2,
}

impl Projection {
    fn new(world_size: ae::Vec2) -> (Self, u32, u32) {
        let usable = (MAX_CANVAS_PX - 2 * MARGIN_PX) as f32;
        let scale = (usable / world_size.x.max(world_size.y).max(1.0)).min(1.0);
        let w = (world_size.x * scale) as u32 + 2 * MARGIN_PX;
        let h = (world_size.y * scale) as u32 + 2 * MARGIN_PX;
        (
            Self {
                scale,
                world_min: ae::Vec2::ZERO,
            },
            w.max(1),
            h.max(1),
        )
    }
    fn px(&self, p: ae::Vec2) -> (i64, i64) {
        let x = MARGIN_PX as f32 + (p.x - self.world_min.x) * self.scale;
        let y = MARGIN_PX as f32 + (p.y - self.world_min.y) * self.scale;
        (x as i64, y as i64)
    }
}

fn fill_rect(img: &mut RgbaImage, min: (i64, i64), max: (i64, i64), color: Rgba<u8>) {
    let (w, h) = (img.width() as i64, img.height() as i64);
    for y in min.1.max(0)..max.1.min(h) {
        for x in min.0.max(0)..max.0.min(w) {
            img.put_pixel(x as u32, y as u32, color);
        }
    }
}

fn stroke_rect(img: &mut RgbaImage, min: (i64, i64), max: (i64, i64), color: Rgba<u8>) {
    let (w, h) = (img.width() as i64, img.height() as i64);
    for x in min.0.max(0)..max.0.min(w) {
        for &y in &[min.1, max.1 - 1] {
            if (0..h).contains(&y) {
                img.put_pixel(x as u32, y as u32, color);
            }
        }
    }
    for y in min.1.max(0)..max.1.min(h) {
        for &x in &[min.0, max.0 - 1] {
            if (0..w).contains(&x) {
                img.put_pixel(x as u32, y as u32, color);
            }
        }
    }
}

/// Draw a filled cross marker centered at a world point.
fn marker(img: &mut RgbaImage, center: (i64, i64), half: i64, color: Rgba<u8>) {
    fill_rect(
        img,
        (center.0 - half, center.1 - 2),
        (center.0 + half, center.1 + 2),
        color,
    );
    fill_rect(
        img,
        (center.0 - 2, center.1 - half),
        (center.0 + 2, center.1 + half),
        color,
    );
}

/// Outline an authored entity's AABB on top of the collision fill, with
/// a small filled corner tick so single-cell entities stay visible.
fn overlay_aabb(img: &mut RgbaImage, proj: &Projection, aabb: ae::Aabb, color: Rgba<u8>) {
    let min = proj.px(aabb.min);
    let max = proj.px(aabb.max);
    stroke_rect(img, (min.0 - 1, min.1 - 1), (max.0 + 1, max.1 + 1), color);
    stroke_rect(img, min, max, color);
}

fn render_room(room: &sb::rooms::RoomSpec) -> RgbaImage {
    let world = &room.world;
    let (proj, w, h) = Projection::new(world.size);
    let mut img = RgbaImage::from_pixel(w, h, Rgba([24, 26, 30, 255]));

    // World bounds outline.
    let (bmin, bmax) = (proj.px(ae::Vec2::ZERO), proj.px(world.size));
    stroke_rect(&mut img, bmin, bmax, Rgba([90, 94, 100, 255]));

    // Collision blocks (filled).
    for block in &world.blocks {
        let min = proj.px(block.aabb.min);
        let max = proj.px(block.aabb.max);
        fill_rect(&mut img, min, max, color_for(&block.kind));
        stroke_rect(&mut img, min, max, Rgba([10, 10, 12, 255]));
    }

    // Authored entity families (outlined, drawn over the collision so
    // both stay legible). Colors echo the in-game debug overlay.
    for e in &room.enemy_spawns {
        overlay_aabb(&mut img, &proj, e.aabb, Rgba([235, 70, 70, 255])); // red
    }
    for b in &room.boss_spawns {
        // Thicker: double outline already; brighten so it reads as the
        // room's headline threat.
        overlay_aabb(&mut img, &proj, b.aabb, Rgba([255, 140, 30, 255])); // orange
    }
    for it in &room.interactables {
        overlay_aabb(&mut img, &proj, it.aabb, Rgba([70, 230, 120, 255])); // green (NPC/switch)
    }
    for p in &room.pickups {
        overlay_aabb(&mut img, &proj, p.aabb, Rgba([90, 210, 230, 255])); // cyan
    }
    for c in &room.chests {
        overlay_aabb(&mut img, &proj, c.aabb, Rgba([240, 205, 70, 255])); // gold
    }
    for br in &room.breakables {
        overlay_aabb(&mut img, &proj, br.aabb, Rgba([150, 190, 240, 255])); // light blue
    }
    for hz in &room.hazards {
        overlay_aabb(&mut img, &proj, hz.aabb, Rgba([235, 80, 220, 255])); // magenta
    }
    for lz in &room.loading_zones {
        overlay_aabb(&mut img, &proj, lz.aabb, Rgba([230, 230, 235, 255])); // white (door/exit)
    }

    // Spawn point (green cross) on top of everything.
    marker(&mut img, proj.px(world.spawn), 8, Rgba([60, 230, 90, 255]));

    img
}

fn main() {
    let mut args = std::env::args().skip(1);
    let room_id = args.next();
    let out_path = args.next();

    let project =
        sb::ldtk_world::LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
    let report = project.validate();
    if !report.is_ok() {
        eprintln!("warning: LDtk validation reported issues; rendering anyway");
    }
    let room_set = project.to_room_set().expect("room_set should build");

    // `all` mode: render every room into a directory so an agent can
    // review the whole map at once.
    if room_id.as_deref() == Some("all") {
        let dir = out_path.unwrap_or_else(|| "/tmp/rooms".to_string());
        std::fs::create_dir_all(&dir).expect("create output dir");
        for room in &room_set.rooms {
            let img = render_room(room);
            let out = format!("{dir}/room_{}.png", room.id);
            img.save(&out).expect("PNG save should succeed");
        }
        println!(
            "rendered {} rooms -> {dir}/room_<id>.png",
            room_set.rooms.len()
        );
        return;
    }

    let Some(room_id) = room_id else {
        println!("Available rooms ({}):", room_set.rooms.len());
        for r in &room_set.rooms {
            println!(
                "  {:<28} size={:?} blocks={}",
                r.id,
                (r.world.size.x, r.world.size.y),
                r.world.blocks.len()
            );
        }
        println!("\nUsage: render_room_geometry <ROOM_ID | all> [OUT.png | OUT_DIR]");
        return;
    };

    let Some(room) = room_set.rooms.iter().find(|r| r.id == room_id) else {
        eprintln!("room '{room_id}' not found. Known rooms:");
        for r in &room_set.rooms {
            eprintln!("  {}", r.id);
        }
        std::process::exit(1);
    };

    let img = render_room(room);
    let out = out_path.unwrap_or_else(|| format!("/tmp/room_{room_id}.png"));
    img.save(&out).expect("PNG save should succeed");
    println!(
        "rendered '{room_id}' ({}x{} world) -> {out}  [{}x{} px]",
        room.world.size.x,
        room.world.size.y,
        img.width(),
        img.height(),
    );
    println!(
        "  collision: {} blocks | enemies: {} | bosses: {} | interactables: {} | pickups: {} | chests: {} | breakables: {} | hazards: {} | doors: {}",
        room.world.blocks.len(),
        room.enemy_spawns.len(),
        room.boss_spawns.len(),
        room.interactables.len(),
        room.pickups.len(),
        room.chests.len(),
        room.breakables.len(),
        room.hazards.len(),
        room.loading_zones.len(),
    );
    println!(
        "legend: FILLED collision (gray=Solid blue=OneWay red=Hazard gold=PogoOrb) | OUTLINES red=enemy orange=boss green=NPC/switch cyan=pickup gold=chest lightblue=breakable magenta=hazard-vol white=door | green-cross=spawn"
    );
}
