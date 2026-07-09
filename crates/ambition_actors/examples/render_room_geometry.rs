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
//!   cargo run -p ambition_actors --example render_room_geometry -- [ROOM_ID] [OUT.png]
//!   cargo run -p ambition_actors --example render_room_geometry -- capture <ROOM_ID> <X,Y> [OUT.png] [WIDTHxHEIGHT]
//!
//! With no ROOM_ID it lists every room id and exits. Default output is
//! `/tmp/room_<id>.png`.

use ambition_actors as sb;
use ambition_engine_core::{self as ae, AabbExt};

/// Interactable footprints, read from the single `placements` channel
/// (fable audit F9.2 — interactables no longer have a typed `RoomSpec` list).
fn interactable_aabbs(room: &sb::rooms::RoomSpec) -> Vec<ae::Aabb> {
    room.placements
        .iter()
        .filter(|r| {
            matches!(
                r.schema,
                ambition_entity_catalog::placements::PlacementSchema::Interactable(_)
            )
        })
        .map(|r| r.aabb)
        .collect()
}
use ambition_sim_view::camera_snapshot::{
    resolve_follow_camera_snapshot, CameraFocus2d, CameraSnapshot2d, CameraSnapshotResolveInput,
    CameraSnapshotResolveMode,
};
use image::{Rgba, RgbaImage};
use sb::persistence::settings::video::CameraFramingPreset;
use sb::persistence::settings::CameraAspectPolicy;

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
    image_offset: ae::Vec2,
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
                image_offset: ae::Vec2::splat(MARGIN_PX as f32),
            },
            w.max(1),
            h.max(1),
        )
    }

    fn from_snapshot(snapshot: &CameraSnapshot2d, image_size: (u32, u32)) -> (Self, u32, u32) {
        let (w, h) = (image_size.0.max(1), image_size.1.max(1));
        let scale_x = w as f32 / snapshot.visible_view.x.max(1.0);
        let scale_y = h as f32 / snapshot.visible_view.y.max(1.0);
        let scale = scale_x.min(scale_y);
        let draw_size = snapshot.visible_view * scale;
        let image_offset = ae::Vec2::new(
            ((w as f32 - draw_size.x) * 0.5).max(0.0),
            ((h as f32 - draw_size.y) * 0.5).max(0.0),
        );
        (
            Self {
                scale,
                world_min: snapshot.center_world - snapshot.visible_view * 0.5,
                image_offset,
            },
            w,
            h,
        )
    }

    fn px(&self, p: ae::Vec2) -> (i64, i64) {
        let x = self.image_offset.x + (p.x - self.world_min.x) * self.scale;
        let y = self.image_offset.y + (p.y - self.world_min.y) * self.scale;
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

/// Draw a 1px line between two image-space points (Bresenham). Used
/// for kinematic-path routes so an author can see where a platform or
/// patrol travels, not just where it starts.
fn draw_line(img: &mut RgbaImage, a: (i64, i64), b: (i64, i64), color: Rgba<u8>) {
    let (w, h) = (img.width() as i64, img.height() as i64);
    let (mut x0, mut y0) = a;
    let (x1, y1) = b;
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        if (0..w).contains(&x0) && (0..h).contains(&y0) {
            img.put_pixel(x0 as u32, y0 as u32, color);
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

/// Draw a kinematic path: connect its waypoints with line segments and
/// drop a small filled dot on each, so platform/patrol routes read at a
/// glance.
fn overlay_path(img: &mut RgbaImage, proj: &Projection, points: &[ae::Vec2], color: Rgba<u8>) {
    for pair in points.windows(2) {
        draw_line(img, proj.px(pair[0]), proj.px(pair[1]), color);
    }
    for &p in points {
        let (x, y) = proj.px(p);
        fill_rect(img, (x - 2, y - 2), (x + 2, y + 2), color);
    }
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
    render_room_projected(room, &proj, w, h, None)
}

fn render_room_snapshot(
    room: &sb::rooms::RoomSpec,
    snapshot: &CameraSnapshot2d,
    image_size: (u32, u32),
) -> RgbaImage {
    let (proj, w, h) = Projection::from_snapshot(snapshot, image_size);
    render_room_projected(room, &proj, w, h, Some(snapshot.target_world))
}

fn render_room_projected(
    room: &sb::rooms::RoomSpec,
    proj: &Projection,
    w: u32,
    h: u32,
    focus_marker: Option<ae::Vec2>,
) -> RgbaImage {
    let world = &room.world;
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

    // Camera zones (thin dim-violet outline) — drawn first as
    // background context so gameplay overlays sit on top.
    for cz in &room.camera_zones {
        overlay_aabb(&mut img, proj, cz.aabb, Rgba([120, 90, 160, 180]));
    }

    // Kinematic paths (platform/patrol/camera-rail routes): bright
    // green polyline + waypoint dots, plus the authored path AABB.
    for kp in &room.kinematic_paths {
        overlay_path(&mut img, proj, &kp.path.points, Rgba([90, 230, 120, 255]));
        overlay_aabb(&mut img, proj, kp.aabb, Rgba([90, 230, 120, 160]));
    }

    // Moving platforms: filled tan (they're solid riding surfaces) at
    // their authored start AABB, with a dark edge.
    for mp in &room.moving_platforms {
        let aabb = mp.aabb();
        let (min, max) = (proj.px(aabb.min), proj.px(aabb.max));
        fill_rect(&mut img, min, max, Rgba([200, 170, 110, 255]));
        stroke_rect(&mut img, min, max, Rgba([40, 30, 15, 255]));
    }

    // Authored entity families (outlined, drawn over the collision so
    // both stay legible). Colors echo the in-game debug overlay.
    for e in &room.enemy_spawns {
        overlay_aabb(&mut img, proj, e.aabb, Rgba([235, 70, 70, 255])); // red
    }
    for b in &room.boss_spawns {
        // Orange: the authored spawn / collision envelope.
        overlay_aabb(&mut img, proj, b.aabb, Rgba([255, 140, 30, 255]));
        // Bright cyan: the actual rest-pose damageable hurtbox(es) the
        // player must hit — derived from the boss's sprite metrics, so a
        // boss whose hurtbox is a small head inside a big body envelope
        // reads correctly.
        for hb in sb::features::boss_spawn_hurtboxes(&b.id, &b.name, b.aabb, b.payload.clone()) {
            overlay_aabb(&mut img, proj, hb, Rgba([60, 240, 255, 255]));
        }
    }
    for it_aabb in interactable_aabbs(room) {
        overlay_aabb(&mut img, proj, it_aabb, Rgba([70, 230, 120, 255])); // green (NPC/switch)
    }
    for p in &room.pickups {
        overlay_aabb(&mut img, proj, p.aabb, Rgba([90, 210, 230, 255])); // cyan
    }
    for c in &room.chests {
        overlay_aabb(&mut img, proj, c.aabb, Rgba([240, 205, 70, 255])); // gold
    }
    for br in &room.breakables {
        overlay_aabb(&mut img, proj, br.aabb, Rgba([150, 190, 240, 255])); // light blue
    }
    for hz in &room.hazards {
        overlay_aabb(&mut img, proj, hz.aabb, Rgba([235, 80, 220, 255])); // magenta
    }
    for lz in &room.loading_zones {
        overlay_aabb(&mut img, proj, lz.aabb, Rgba([230, 230, 235, 255])); // white (door/exit)
    }

    // Spawn point (green cross) on top of everything.
    marker(&mut img, proj.px(world.spawn), 8, Rgba([60, 230, 90, 255]));
    // Optional focus marker (blue cross) for snapshot captures.
    if let Some(focus) = focus_marker {
        marker(&mut img, proj.px(focus), 6, Rgba([80, 170, 255, 255]));
    }

    img
}

fn parse_vec2(text: &str) -> Option<ae::Vec2> {
    let (x, y) = text.split_once(',')?;
    Some(ae::Vec2::new(x.parse().ok()?, y.parse().ok()?))
}

fn parse_image_size(text: &str) -> Option<(u32, u32)> {
    let (w, h) = text.split_once('x').or_else(|| text.split_once('X'))?;
    Some((w.parse().ok()?, h.parse().ok()?))
}

fn resolve_headless_snapshot(
    room: &sb::rooms::RoomSpec,
    focus_world: ae::Vec2,
    image_size: (u32, u32),
) -> CameraSnapshot2d {
    let body = ae::default_player_body_size();
    resolve_follow_camera_snapshot(
        CameraSnapshotResolveInput {
            world: &room.world,
            camera_zones: &room.camera_zones,
            focus: CameraFocus2d {
                center_world: focus_world,
                size: body,
                base_size: body,
                facing: 1.0,
            },
            base_view: ae::Vec2::new(800.0, 450.0),
            viewport_px: ae::Vec2::new(image_size.0 as f32, image_size.1 as f32),
            aspect_policy: CameraAspectPolicy::FitDesign,
            framing: CameraFramingPreset::Combat,
            overview_scale: 1.0,
            encounter_scale: 1.0,
            overview_camera: false,
            snap_camera: true,
            blink: None,
            dt: 0.0,
            mode: CameraSnapshotResolveMode::Instant,
            extra_clamp_center_world: None,
            ease_tuning: ambition_platformer_primitives::camera_ease::CameraEaseTuning::default(),
        },
        None,
    )
}

/// Scan every room's lowered `RoomSpec` for spatial anomalies that
/// would be runtime bugs: an authored entity whose center sits outside
/// the room bounds (it would fall/float forever), or a player spawn
/// embedded inside a Solid collision block (the player loads stuck).
/// These are projection-level checks the LDtk validator does not run.
fn run_anomaly_report(room_set: &sb::rooms::RoomSet) {
    let mut total = 0usize;
    for room in &room_set.rooms {
        let world = &room.world;
        let mut issues: Vec<String> = Vec::new();

        // (1) authored entity centers outside the room.
        let mut families: Vec<(&str, ae::Aabb)> = Vec::new();
        families.extend(room.enemy_spawns.iter().map(|e| ("enemy", e.aabb)));
        families.extend(room.boss_spawns.iter().map(|b| ("boss", b.aabb)));
        families.extend(
            interactable_aabbs(room)
                .into_iter()
                .map(|a| ("interactable", a)),
        );
        families.extend(room.pickups.iter().map(|p| ("pickup", p.aabb)));
        families.extend(room.chests.iter().map(|c| ("chest", c.aabb)));
        families.extend(room.breakables.iter().map(|b| ("breakable", b.aabb)));
        families.extend(room.hazards.iter().map(|h| ("hazard", h.aabb)));
        families.extend(room.loading_zones.iter().map(|z| ("loading_zone", z.aabb)));
        for (label, aabb) in families {
            let c = aabb.center();
            if c.x < 0.0 || c.y < 0.0 || c.x > world.size.x || c.y > world.size.y {
                issues.push(format!(
                    "{label} center {:?} outside room bounds {:?}",
                    (c.x, c.y),
                    (world.size.x, world.size.y)
                ));
            }
        }

        // (2) spawn + small open-space entities embedded in a Solid block.
        let point_in_solid = |p: ae::Vec2| {
            world.blocks.iter().any(|block| {
                matches!(block.kind, ae::BlockKind::Solid)
                    && p.x >= block.aabb.min.x
                    && p.x <= block.aabb.max.x
                    && p.y >= block.aabb.min.y
                    && p.y <= block.aabb.max.y
            })
        };
        if point_in_solid(world.spawn) {
            issues.push(format!(
                "spawn {:?} is inside a Solid block",
                (world.spawn.x, world.spawn.y)
            ));
        }
        let mut embeddable: Vec<(&str, ae::Aabb)> = Vec::new();
        embeddable.extend(room.enemy_spawns.iter().map(|e| ("enemy", e.aabb)));
        embeddable.extend(room.pickups.iter().map(|p| ("pickup", p.aabb)));
        embeddable.extend(room.chests.iter().map(|c| ("chest", c.aabb)));
        embeddable.extend(room.breakables.iter().map(|b| ("breakable", b.aabb)));
        for (label, aabb) in embeddable {
            if point_in_solid(aabb.center()) {
                issues.push(format!(
                    "{label} center {:?} is inside a Solid block",
                    (aabb.center().x, aabb.center().y)
                ));
            }
        }

        if !issues.is_empty() {
            println!("ANOMALY {}:", room.id);
            for issue in &issues {
                println!("  - {issue}");
            }
            total += issues.len();
        }
    }
    if total == 0 {
        println!(
            "no spatial anomalies across {} rooms (entity-out-of-bounds + spawn-in-solid)",
            room_set.rooms.len()
        );
    } else {
        println!("\n{total} anomalies across {} rooms", room_set.rooms.len());
    }
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();

    let project =
        sb::ldtk_world::LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
    let report = project.validate();
    if !report.is_ok() {
        eprintln!("warning: LDtk validation reported issues; rendering anyway");
    }
    let room_set = project.to_room_set().expect("room_set should build");

    // `report` mode: scan every room's RUNTIME projection for spatial
    // anomalies the LDtk validator can't see (it validates LDtk-level
    // data, not the lowered `RoomSpec`). Pure text, no PNGs.
    if args.first().map(String::as_str) == Some("report") {
        run_anomaly_report(&room_set);
        return;
    }

    // `all` mode: render every room into a directory so an agent can
    // review the whole map at once.
    if args.first().map(String::as_str) == Some("all") {
        let dir = args
            .get(1)
            .cloned()
            .unwrap_or_else(|| "/tmp/rooms".to_string());
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

    // `capture` / `snapshot` mode: resolve CameraSnapshot2d for an arbitrary
    // follow point, then render the room through that snapshot. This is the
    // no-GPU version of "what would the scene look like if the camera followed
    // this point?" and is the same camera-policy seam future portal capture
    // requests can consume.
    if matches!(
        args.first().map(String::as_str),
        Some("capture" | "snapshot")
    ) {
        args.remove(0);
        let Some(room_id) = args.first().cloned() else {
            eprintln!(
                "Usage: render_room_geometry capture <ROOM_ID> <X,Y> [OUT.png] [WIDTHxHEIGHT]"
            );
            std::process::exit(2);
        };
        let Some(focus_text) = args.get(1).cloned() else {
            eprintln!(
                "Usage: render_room_geometry capture <ROOM_ID> <X,Y> [OUT.png] [WIDTHxHEIGHT]"
            );
            std::process::exit(2);
        };
        let Some(focus) = parse_vec2(&focus_text) else {
            eprintln!("focus must be X,Y world coordinates, got '{focus_text}'");
            std::process::exit(2);
        };
        let image_size = args
            .get(3)
            .and_then(|text| parse_image_size(text))
            .unwrap_or((1280, 720));
        let Some(room) = room_set.rooms.iter().find(|r| r.id == room_id) else {
            eprintln!("room '{room_id}' not found. Known rooms:");
            for r in &room_set.rooms {
                eprintln!("  {}", r.id);
            }
            std::process::exit(1);
        };
        let snapshot = resolve_headless_snapshot(room, focus, image_size);
        let img = render_room_snapshot(room, &snapshot, image_size);
        let out = args
            .get(2)
            .cloned()
            .unwrap_or_else(|| format!("/tmp/room_{room_id}_capture.png"));
        img.save(&out).expect("PNG save should succeed");
        println!(
            "captured '{room_id}' focus=({:.1},{:.1}) center=({:.1},{:.1}) visible=({:.1},{:.1}) -> {out} [{}x{} px]",
            focus.x,
            focus.y,
            snapshot.center_world.x,
            snapshot.center_world.y,
            snapshot.visible_view.x,
            snapshot.visible_view.y,
            img.width(),
            img.height(),
        );
        return;
    }

    let room_id = args.first().cloned();
    let out_path = args.get(1).cloned();

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
        println!("       render_room_geometry capture <ROOM_ID> <X,Y> [OUT.png] [WIDTHxHEIGHT]");
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
        "  collision: {} blocks | enemies: {} | bosses: {} | interactables: {} | pickups: {} | chests: {} | breakables: {} | hazards: {} | doors: {} | platforms: {} | paths: {} | camera-zones: {}",
        room.world.blocks.len(),
        room.enemy_spawns.len(),
        room.boss_spawns.len(),
        interactable_aabbs(room).len(),
        room.pickups.len(),
        room.chests.len(),
        room.breakables.len(),
        room.hazards.len(),
        room.loading_zones.len(),
        room.moving_platforms.len(),
        room.kinematic_paths.len(),
        room.camera_zones.len(),
    );
    println!(
        "legend: FILLED collision (gray=Solid blue=OneWay red=Hazard gold=PogoOrb) tan=moving-platform | OUTLINES red=enemy orange=boss green=NPC/switch cyan=pickup gold=chest lightblue=breakable magenta=hazard-vol white=door violet=camera-zone | green-line=kinematic-path | green-cross=spawn blue-cross=snapshot-focus"
    );
}
