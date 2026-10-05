//! In-engine parity of the rigged-sprite realization: every frame of a
//! character drawn by Bevy's renderer from its baked sheet and from its part
//! flipbook, saved side by side for a diff
//! (`docs/planning/engine/mary-o-part-realization.md`, Validation 2).
//!
//! The renderer repository's offline gate proves that the published draws
//! recompose the baked frame in PIL. This asks the question that gate cannot:
//! does the GAME draw the same picture? It catches what only the GPU path
//! does — texture filtering of a turned part, blending overlapping parts in
//! linear light, half-pixel placement, the facing flip and draw order.
//!
//! Each frame is pinned (row by name, frame index, held), drawn through the
//! game's own frame code (`draw_held_frame`) and the real rigged systems, into
//! an offscreen image with a transparent clear, and read back. The run writes
//! `<out>/<target>/<row>_<frame>_{baked,parts}.png` and an `index.tsv`;
//! `scripts/measure_rigged_parity.py` runs this and does the diff.
//!
//! ```sh
//! cargo run -p ambition_render --example rigged_sprite_parity -- \
//!     --target mary_o_v2 --scale 1 --out target/rig_parity
//! ```

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use bevy::render::gpu_readback::{Readback, ReadbackComplete};
use bevy::sprite::Anchor;

use ambition_persistence::settings::TextureResolutionScale;
use ambition_render::rendering::actors::draw_held_frame;
use ambition_render::rendering::actors::rigged::{
    add_rigged_impostor_material_plugin, bind_rigged_presentations, drive_rigged_presentations, impostor_cell_class,
    RiggedImpostorAtlas, RiggedPresentations, IMPOSTOR_CELL_CLASSES, IMPOSTOR_MARGIN, impostor_margin,
};
use ambition_render::rendering::actors::BoundSpriteQuality;
use ambition_sprite_sheet::character::rigged::{RiggedSpriteAdmission, RiggedSpriteAsset, RiggedSpritePages};
use ambition_sprite_sheet::character::{
    build_character_presentation_with_render_size, try_load_spec_for_character_id, CharacterAnim,
    CharacterAnimator, CharacterSpriteAsset, CharacterSpritePage,
};
use ambition_sprite_sheet::game_assets::GameAssets;

const ASSETS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../ambition_platformer2d_actor_monolith/assets");
/// Transparent pixels around the drawn frame, so nothing touches the edge.
const MARGIN: u32 = 16;

/// The frame every root is pinned to.
#[derive(Resource, Clone, Default)]
struct Pin {
    row: String,
    frame: usize,
    flip: bool,
    /// How far into the frame (0..1): above 0, a tweened clip draws an
    /// in-between.
    phase: f32,
    duration: f32,
}

/// The last readback, filled by the observer.
#[derive(Resource, Clone, Default)]
struct Captured(Arc<Mutex<Option<Vec<u8>>>>);

fn main() {
    let mut targets: Vec<String> = Vec::new();
    let mut out = PathBuf::from("target/rig_parity");
    let mut scale: f32 = 1.0;
    let mut flips = vec![false];
    let mut phase: f32 = 0.0;
    let mut centre_anchored = false;
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--target" => targets.push(it.next().expect("--target ID")),
            "--out" => out = PathBuf::from(it.next().expect("--out DIR")),
            "--scale" => scale = it.next().expect("--scale S").parse().expect("--scale S"),
            "--both-facings" => flips = vec![false, true],
            "--phase" => phase = it.next().expect("--phase T").parse().expect("--phase T"),
            // Build the root as a player with a sheet-authored quad is built:
            // `Anchor::CENTER`, its translation the quad's centre
            // (`character_render_basis`). Without it the root is built as an
            // NPC is: anchored at its feet.
            "--centre-anchored" => centre_anchored = true,
            other => {
                eprintln!("rigged_sprite_parity: unknown argument {other}");
                std::process::exit(2);
            }
        }
    }
    if targets.is_empty() {
        targets = vec!["mary_o_v2".into(), "mary_o_v2_tall".into(), "mary_o_v2_fire".into()];
    }
    let mut index = String::from("target\trow\tframe\tflip\tfeet_x\tfeet_y\troot_x\tframe_x0\tframe_y0\tcell_x0\tcell_y0\tcell_x1\tcell_y1\tbaked\tparts\n");
    for target in &targets {
        let dir = out.join(target);
        std::fs::create_dir_all(&dir).expect("create the output directory");
        let baked = capture_all(target, false, scale, &flips, phase, centre_anchored);
        let parts = capture_all(target, true, scale, &flips, phase, centre_anchored);
        assert_eq!(baked.len(), parts.len(), "`{target}`: the two runs pinned different frames");
        for ((pin, size, (feet, root_x, frame_tl, cell), baked), (_pin, _size, _feet, parts)) in baked.into_iter().zip(parts) {
            let stem = format!("{}_{}{}", pin.row, pin.frame, if pin.flip { "_flip" } else { "" });
            let baked_path = dir.join(format!("{stem}_baked.png"));
            let parts_path = dir.join(format!("{stem}_parts.png"));
            save(&baked_path, size, &baked);
            save(&parts_path, size, &parts);
            index.push_str(&format!(
                "{target}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                pin.row,
                pin.frame,
                pin.flip,
                feet.x,
                feet.y,
                root_x,
                frame_tl.x,
                frame_tl.y,
                cell.min.x,
                cell.min.y,
                cell.max.x,
                cell.max.y,
                baked_path.display(),
                parts_path.display()
            ));
        }
        println!("[rigged_sprite_parity] {target}: wrote {}", dir.display());
    }
    std::fs::write(out.join("index.tsv"), index).expect("write index.tsv");
}

fn save(path: &std::path::Path, size: UVec2, pixels: &[u8]) {
    image::RgbaImage::from_raw(size.x, size.y, pixels.to_vec())
        .expect("a full frame")
        .save(path)
        .expect("save the frame");
}

/// Every row and frame of `target`, drawn by one path: `(pin, size, (feet
/// pixel, root x, frame top left, cell), RGBA)`. The feet pixel is where the body's feet land
/// in the image, +y down, so a reader can place the published frame there; the
/// root x is the column a facing flip mirrors about (the root's origin: the
/// feet for an NPC, the quad's centre for a centre-anchored player); the
/// frame's top left is where the frame landed in the image; the cell
/// is the image rectangle (+y down) the part road can draw in at all — the
/// body's impostor cell, mirrored with it.
fn capture_all(
    target: &str,
    rigged: bool,
    scale: f32,
    flips: &[bool],
    phase: f32,
    centre_anchored: bool,
) -> Vec<(Pin, UVec2, (Vec2, f32, Vec2, Rect), Vec<u8>)> {
    let spec = try_load_spec_for_character_id(target).expect("a baked sheet: run scripts/regen/sprites.sh");
    let frame = Vec2::new(spec.frame_width as f32, spec.frame_height as f32);
    let render = frame * scale;
    let size = UVec2::new(render.x.ceil() as u32 + 2 * MARGIN, render.y.ceil() as u32 + 2 * MARGIN);
    let (mut app, image) = renderer(size);
    let captured = Captured::default();
    app.init_resource::<RiggedPresentations>()
        .init_resource::<RiggedImpostorAtlas>()
        .insert_resource(RiggedSpriteAdmission { admit: rigged })
        .insert_resource(Pin::default())
        .insert_resource(captured.clone())
        .add_systems(Update, (pin, bind_rigged_presentations, drive_rigged_presentations).chain());
    let asset = {
        let world = app.world_mut();
        let server = world.resource::<AssetServer>().clone();
        let mut layouts = world.resource_mut::<Assets<TextureAtlasLayout>>();
        sheet(target, rigged, &server, &mut layouts)
    };
    wait_for_pages(&mut app, &asset);
    let mut assets = GameAssets::default();
    assets.characters.declare("parity");
    assets.characters.publish("parity", asset.clone());
    app.insert_resource(assets);
    let feet = Vec2::new(asset.spec.feet_anchor_x, asset.spec.feet_anchor_y);
    let built_at = if centre_anchored { Anchor::CENTER } else { Anchor(feet) };
    let (sprite, anchor, animator) = build_character_presentation_with_render_size(&asset, render, built_at);
    // The frame lands on whole pixels, as it does wherever it is drawn 1:1, so
    // neither path is resampled by a sub-pixel root. `feet_pixel` is where the
    // feet land in the image (+y down), for the reader's oracle.
    // ⛔ The FRAME lands on whole pixels, not the feet: a sheet whose anchor is
    // not on its pixel grid (paradox_barber's feet are half a pixel off it)
    // put every frame half a pixel off when the feet were rounded, and the GPU
    // resampled the whole frame. `tl` is the frame's top left in the image
    // (+y down); the root stands where the anchor puts it from there.
    let half = size.as_vec2() * 0.5;
    let tl = ((size.as_vec2() - render) * 0.5).floor();
    let feet_in_frame = Vec2::new((feet.x + 0.5) * render.x, (0.5 - feet.y) * render.y);
    let at = if centre_anchored {
        // The root is the frame's centre.
        Vec2::new(tl.x + render.x * 0.5 - half.x, half.y - tl.y - render.y * 0.5)
    } else {
        // The root is the frame's feet.
        Vec2::new(tl.x + feet_in_frame.x - half.x, half.y - tl.y - feet_in_frame.y)
    };
    let feet_pixel = tl + feet_in_frame;
    // ⛔ The part road draws a body into its impostor CELL (the frame with
    // `impostor_margin` on its top and left, the class's size), and the root's
    // quad shows that cell mirrored about the root when the body faces left. A
    // part reaching past the cell is cut there. The image is the frame with a
    // margin on every side, so facing right the cell covers it from the top
    // left, but mirrored about feet that are off the frame's centre it does not:
    // the oni leader's banner, past the cell, read as 1153 wrong pixels of a
    // correct draw (2026-10-03). The reader clips its oracle to this.
    // A frame no cell fits is not drawn from parts at all: nothing to clip.
    // The target's flipbook's margin, in both captures: the baked one's sheet
    // carries no flipbook, and a margin of 0 clipped the oracle to the frame.
    let margin = RiggedSpriteAsset::baked(target).map_or(IMPOSTOR_MARGIN, |flipbook| impostor_margin(&flipbook));
    let cell_px = impostor_cell_class(frame, margin).map_or(1.0e6, |class| IMPOSTOR_CELL_CLASSES[class].0);
    let cell_min = tl - Vec2::splat(margin * scale);
    let cell = Rect::from_corners(cell_min, cell_min + Vec2::splat(cell_px * scale));
    app.world_mut().spawn((
        sprite,
        anchor,
        animator,
        Transform::from_xyz(at.x, at.y, 0.0),
        Visibility::Inherited,
        BoundSpriteQuality {
            scale: TextureResolutionScale::Full,
        },
    ));
    let rows: Vec<(String, usize, f32)> = ambition_sprite_sheet::character::sheets::record_for_sheet_key(target)
        .expect("a baked sheet record")
        .rows
        .iter()
        .map(|row| (row.animation.clone(), row.frame_count as usize, row.duration_secs))
        .collect();
    let mut out = Vec::new();
    for &flip in flips {
        for (row, count, duration) in &rows {
            for frame in 0..*count {
                let pinned = Pin {
                    row: row.clone(),
                    frame,
                    flip,
                    phase,
                    duration: *duration,
                };
                *app.world_mut().resource_mut::<Pin>() = pinned.clone();
                // Two updates: the pin and the drive land, then the frame draws.
                app.update();
                app.update();
                let root_x = size.x as f32 * 0.5 + at.x;
                let cell = if flip {
                    Rect::new(2.0 * root_x - cell.max.x, cell.min.y, 2.0 * root_x - cell.min.x, cell.max.y)
                } else {
                    cell
                };
                out.push((pinned, size, (feet_pixel, root_x, tl, cell), readback(&mut app, &image, &captured, size)));
            }
        }
    }
    out
}

/// Pin every root to the requested row and frame, and draw it the game's way.
fn pin(pin: Res<Pin>, mut roots: Query<(&mut Sprite, &mut CharacterAnimator, &mut Anchor)>) {
    for (mut sprite, mut animator, mut anchor) in &mut roots {
        animator.request_clip([pin.row.as_str()], CharacterAnim::Idle);
        animator.frame = pin.frame;
        // Held at the frame, or `phase` of the way into it. The frame is drawn
        // with no time passing either way, so it never advances.
        animator.elapsed = pin.phase * pin.duration;
        animator.clip_held = pin.phase <= 0.0;
        let flip = animator.face(pin.flip);
        draw_held_frame(&mut sprite, &mut animator, &mut anchor, flip);
        sprite.color = Color::WHITE;
    }
}

fn readback(app: &mut App, image: &Handle<Image>, captured: &Captured, size: UVec2) -> Vec<u8> {
    *captured.0.lock().unwrap() = None;
    let slot = captured.0.clone();
    app.world_mut()
        .spawn(Readback::texture(image.clone()))
        .observe(move |event: On<ReadbackComplete>, mut commands: Commands| {
            *slot.lock().unwrap() = Some(event.data.clone());
            commands.entity(event.entity).despawn();
        });
    for _ in 0..30 {
        app.update();
        if let Some(data) = captured.0.lock().unwrap().take() {
            let row = size.x as usize * 4;
            let padded = row.div_ceil(256) * 256;
            let mut pixels = vec![0u8; row * size.y as usize];
            for y in 0..size.y as usize {
                pixels[y * row..(y + 1) * row].copy_from_slice(&data[y * padded..y * padded + row]);
            }
            return pixels;
        }
    }
    panic!("the readback did not complete in 30 frames");
}

/// Bevy's renderer with no window, one camera drawing 1:1 into an offscreen
/// image with a transparent clear.
fn renderer(size: UVec2) -> (App, Handle<Image>) {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: bevy::window::ExitCondition::DontExit,
                ..default()
            })
            .set(AssetPlugin {
                file_path: ASSETS.to_string(),
                ..default()
            })
            .disable::<bevy::winit::WinitPlugin>(),
    );
    add_rigged_impostor_material_plugin(&mut app);
    app.finish();
    app.cleanup();
    let image = {
        let mut target = Image::new_target_texture(
            size.x,
            size.y,
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            None,
        );
        target.texture_descriptor.usage |= bevy::render::render_resource::TextureUsages::COPY_SRC;
        app.world_mut().resource_mut::<Assets<Image>>().add(target)
    };
    // The camera blends as the game's world cameras do (`WORLD_COMPOSITING`):
    // the harness measures what the player sees. `AMBITION_PARITY_COMPOSITING=srgb`
    // blends this offscreen target in the art's gamma space instead (an
    // experiment; a window cannot, see `WORLD_COMPOSITING`).
    let space = match std::env::var("AMBITION_PARITY_COMPOSITING").as_deref() {
        Err(_) => ambition_render::rendering::WORLD_COMPOSITING,
        Ok("srgb") => ambition_render::rendering::ART_COMPOSITING,
        Ok(other) => panic!("AMBITION_PARITY_COMPOSITING={other:?} is not `srgb`"),
    };
    app.world_mut().spawn((
        Camera2d,
        space,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::NONE),
            ..default()
        },
        bevy::render::view::Msaa::Off,
        bevy::camera::RenderTarget::Image(bevy::camera::ImageRenderTarget::from(image.clone())),
        Transform::from_xyz(0.0, 0.0, 100.0),
    ));
    (app, image)
}

/// The sheet of `target` with its pages (and part pages when `rigged`) loaded
/// from the published assets.
fn sheet(
    target: &str,
    rigged: bool,
    server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
) -> CharacterSpriteAsset {
    let spec = try_load_spec_for_character_id(target).expect("a baked sheet: run scripts/regen/sprites.sh");
    let flipbook = rigged.then(|| RiggedSpriteAsset::baked(target).expect("a published flipbook"));
    let pages: Vec<CharacterSpritePage> = (0..spec.page_count().max(1))
        .map(|page| {
            let file = if page == 0 {
                format!("{target}_spritesheet.png")
            } else {
                spec.page_images[page as usize].clone()
            };
            CharacterSpritePage {
                texture: server.load(format!("sprites/{file}")),
                layout: layouts.add(spec.build_atlas_for_page(page)),
            }
        })
        .collect();
    let part_pages = flipbook.as_ref().map_or(Vec::new(), |flipbook| {
        flipbook.pages.iter().map(|page| ambition_sprite_sheet::game_assets::load_sheet_image(server, "character-parts", format!("sprites/{page}"))).collect()
    });
    CharacterSpriteAsset {
        texture: pages[0].texture.clone(),
        layout: pages[0].layout.clone(),
        spec,
        pages,
        requested_tier: TextureResolutionScale::Full,
        resolved_tier: TextureResolutionScale::Full,
        rigged: flipbook.map(|flipbook| RiggedSpritePages {
            flipbook: Arc::new(flipbook),
            pages: part_pages,
        }),
    }
}

fn wait_for_pages(app: &mut App, asset: &CharacterSpriteAsset) {
    let mut handles: Vec<Handle<Image>> = asset.pages.iter().map(|page| page.texture.clone()).collect();
    if let Some(rigged) = &asset.rigged {
        handles.extend(rigged.pages.iter().cloned());
    }
    for _ in 0..600 {
        app.update();
        let server = app.world().resource::<AssetServer>();
        if handles.iter().all(|handle| server.is_loaded_with_dependencies(handle.id())) {
            return;
        }
    }
    panic!("the sheet or part pages did not load in 600 frames");
}
