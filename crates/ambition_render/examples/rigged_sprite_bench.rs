//! Rig packet 8: the cost of the rigged-sprite realization against the baked
//! sheet, for 1 to 100 actors.
//!
//! Each actor is a root built as the game builds one (the sheet, its animator
//! and its render basis). A stand-in for the animator system ticks each
//! animator and changes its pose every two seconds, as the game's animator
//! does. The baked run stops there. The rigged run also runs the two real
//! systems of the realization, `bind_rigged_presentations` and
//! `drive_rigged_presentations`. So the difference between the two runs is
//! the cost that the rigged path adds.
//!
//! Two modes:
//!
//! * the default mode has no renderer. It measures the ECS side: the entity
//!   count, the visible sprite count and the update time.
//! * `--render` adds Bevy's renderer, drawing into an offscreen image, with
//!   the sheet and the part pages loaded from the published assets. It also
//!   measures the sprites extracted to the render world, the sprite batches
//!   (the draw calls), and the frame time. `--views 2` draws the same actors
//!   with two cameras side by side, as a split screen does. `--tiny` draws
//!   into a target ten times smaller that shows the same actors, to separate
//!   the rasterizer from the rest of the frame.
//!
//! ⚠ The frame time of `--render` includes the rasterizer of this machine. On
//! a software adapter (llvmpipe) that is CPU work that a GPU does not do, so
//! compare the two paths on one machine, not the numbers across machines.
//! Each measured frame of `--render` waits for the GPU to finish the frame, so
//! on a hardware GPU the time includes the GPU work, not only its submission.
//! The first render run prints the adapter (`adapter name=…`), so a report
//! states which rasterizer made it. `scripts/rig_packet8_gpu_bench.py` runs the
//! Packet 8 matrix on a hardware GPU and writes the report.
//!
//! ```sh
//! cargo run -p ambition_render --example rigged_sprite_bench --profile profiling -- --frames 3000
//! cargo run -p ambition_render --example rigged_sprite_bench --profile profiling -- --render --views 2
//! ```

use std::time::Instant;

use bevy::prelude::*;
use bevy::sprite::Anchor;

use ambition_persistence::settings::TextureResolutionScale;
use ambition_render::rendering::actors::rigged::{
    bind_rigged_presentations, drive_rigged_presentations, RiggedPresentations,
};
use ambition_render::rendering::actors::BoundSpriteQuality;
use ambition_sprite_sheet::character::rigged::{RiggedSpriteAdmission, RiggedSpriteAsset, RiggedSpritePages};
use ambition_sprite_sheet::character::{
    build_character_presentation_with_render_size, try_load_spec_for_character_id, CharacterAnim,
    CharacterAnimator, CharacterSpriteAsset, CharacterSpritePage,
};
use ambition_sprite_sheet::game_assets::GameAssets;

/// The poses the stand-in animator steps through, one every `POSE_FRAMES`.
const POSES: [CharacterAnim; 5] = [
    CharacterAnim::Idle,
    CharacterAnim::Run,
    CharacterAnim::Slash,
    CharacterAnim::Jump,
    CharacterAnim::Walk,
];
const POSE_FRAMES: u64 = 120;
const DT: f32 = 1.0 / 60.0;
const RENDER: Vec2 = Vec2::new(103.0, 114.0);
/// The actors stand in a grid of this many columns and `CELLS / COLUMNS` rows,
/// which one view shows whole. Actors past `CELLS` stand on the cells again,
/// so every actor is on screen (and none is culled) at any count.
const COLUMNS: usize = 10;
const CELLS: usize = 100;
const SPACING: Vec2 = Vec2::new(110.0, 120.0);
/// The offscreen target. At `CAMERA_SCALE` one view shows the full grid of
/// 100 actors.
const TARGET: UVec2 = UVec2::new(1280, 720);
const CAMERA_SCALE: f32 = 2.0;
/// `--tiny`: a target this many times smaller, which shows the same grid. The
/// sprites extracted and batched are the same, and almost no pixel is filled,
/// so the difference to the full target is the rasterizer.
const TINY: u32 = 10;
const ASSETS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../ambition_platformer2d_actor_monolith/assets");

struct Row {
    actors: usize,
    rigged: bool,
    entities: usize,
    visible_sprites: usize,
    median_us: f64,
    p95_us: f64,
    /// `(extracted sprites, sprite batches)`, with `--render`.
    render: Option<(usize, usize)>,
}

fn main() {
    let mut target = "pirate_admiral".to_string();
    let mut frames: usize = 3000;
    let mut counts = vec![1usize, 10, 50, 100];
    let mut views: Option<usize> = None;
    let mut tiny = false;
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--target" => target = it.next().expect("--target ID"),
            "--frames" => frames = it.next().expect("--frames N").parse().expect("--frames N"),
            "--render" => views = Some(views.unwrap_or(1)),
            "--tiny" => tiny = true,
            "--views" => views = Some(it.next().expect("--views 1|2").parse().expect("--views 1|2")),
            "--actors" => {
                counts = it
                    .next()
                    .expect("--actors N,N,..")
                    .split(',')
                    .map(|n| n.parse().expect("--actors N,N,.."))
                    .collect()
            }
            other => {
                eprintln!("rigged_sprite_bench: unknown argument {other}");
                std::process::exit(2);
            }
        }
    }
    let flipbook = RiggedSpriteAsset::baked(&target)
        .unwrap_or_else(|| panic!("`{target}` publishes no part flipbook: run scripts/regen/sprites.sh"));
    println!(
        "[rigged_sprite_bench] target={target} max_draws={} parts={} frames={frames} views={views:?} tiny={tiny}",
        flipbook.max_draws(),
        flipbook.parts.len()
    );
    for &actors in &counts {
        let baked = run(&target, actors, false, frames, views, tiny);
        let rigged = run(&target, actors, true, frames, views, tiny);
        for row in [&baked, &rigged] {
            let render = row.render.map_or(String::new(), |(extracted, batches)| {
                format!(" extracted_sprites={extracted} sprite_batches={batches}")
            });
            println!(
                "[rigged_sprite_bench] actors={} path={} entities={} visible_sprites={}{render} update_us median={:.1} p95={:.1}",
                row.actors,
                if row.rigged { "rigged" } else { "baked" },
                row.entities,
                row.visible_sprites,
                row.median_us,
                row.p95_us,
            );
        }
        println!(
            "[rigged_sprite_bench] actors={actors} rigged_minus_baked_us median={:.1}",
            rigged.median_us - baked.median_us
        );
    }
}

/// The sheet of `target`: with default handles and no pages when nothing
/// draws, or with its pages loaded from the published assets.
fn sheet(
    target: &str,
    rigged: bool,
    loader: Option<(&AssetServer, &mut Assets<TextureAtlasLayout>)>,
) -> CharacterSpriteAsset {
    let spec = try_load_spec_for_character_id(target).expect("a baked sheet: run scripts/regen/sprites.sh");
    let flipbook = rigged.then(|| RiggedSpriteAsset::baked(target).expect("a published flipbook"));
    let (texture, layout, pages, part_pages) = match loader {
        None => (
            Handle::default(),
            Handle::default(),
            Vec::new(),
            flipbook.as_ref().map_or(Vec::new(), |flipbook| vec![Handle::default(); flipbook.pages.len()]),
        ),
        Some((server, layouts)) => {
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
                flipbook.pages.iter().map(|page| server.load(format!("sprites/{page}"))).collect()
            });
            (pages[0].texture.clone(), pages[0].layout.clone(), pages, part_pages)
        }
    };
    CharacterSpriteAsset {
        texture,
        layout,
        spec,
        pages,
        requested_tier: TextureResolutionScale::Full,
        resolved_tier: TextureResolutionScale::Full,
        rigged: flipbook.map(|flipbook| RiggedSpritePages {
            flipbook: std::sync::Arc::new(flipbook),
            pages: part_pages,
        }),
    }
}

/// The stand-in for the game's animator system: the same animator calls, and
/// the atlas index written on the root sprite.
fn animate(mut roots: Query<(&mut CharacterAnimator, &mut Sprite)>, mut tick: Local<u64>) {
    *tick += 1;
    for (index, (mut animator, mut sprite)) in roots.iter_mut().enumerate() {
        // Each actor changes pose on its own tick, so the poses are mixed.
        let phase = (*tick + index as u64 * 7) / POSE_FRAMES;
        animator.request(POSES[phase as usize % POSES.len()]);
        let atlas_index = animator.tick(DT);
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = atlas_index;
        }
    }
}

/// Bevy's renderer with no window, and `views` cameras that draw into one
/// offscreen image, side by side.
fn add_renderer(app: &mut App, views: usize, tiny: bool) {
    let (size, scale) = if tiny {
        (TARGET / TINY, CAMERA_SCALE * TINY as f32)
    } else {
        (TARGET, CAMERA_SCALE)
    };
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
            // No window and no display: `update` is called by hand.
            .disable::<bevy::winit::WinitPlugin>(),
    );
    // This crate's Bevy features include no pipelined rendering, so the render
    // world runs inside `update` and the frame time includes it.
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
    let width = size.x / views as u32;
    for view in 0..views {
        app.world_mut().spawn((
            Camera2d,
            Camera {
                order: view as isize,
                viewport: Some(bevy::camera::Viewport {
                    physical_position: UVec2::new(view as u32 * width, 0),
                    physical_size: UVec2::new(width, size.y),
                    ..default()
                }),
                ..default()
            },
            bevy::camera::RenderTarget::Image(bevy::camera::ImageRenderTarget::from(image.clone())),
            Projection::Orthographic(OrthographicProjection {
                scale,
                ..OrthographicProjection::default_2d()
            }),
            Transform::from_xyz(0.0, 0.0, 100.0),
        ));
    }
}

fn run(target: &str, actors: usize, rigged: bool, frames: usize, views: Option<usize>, tiny: bool) -> Row {
    let mut app = App::new();
    if let Some(views) = views {
        add_renderer(&mut app, views, tiny);
    }
    app.init_resource::<RiggedPresentations>()
        .insert_resource(RiggedSpriteAdmission { admit: rigged })
        .add_systems(
            Update,
            (animate, bind_rigged_presentations, drive_rigged_presentations).chain(),
        );
    let asset = if views.is_some() {
        let world = app.world_mut();
        let server = world.resource::<AssetServer>().clone();
        let mut layouts = world.resource_mut::<Assets<TextureAtlasLayout>>();
        sheet(target, rigged, Some((&server, &mut layouts)))
    } else {
        // No renderer: the pages are images made present by hand, so the
        // binder (which waits for every page) binds at once.
        let mut asset = sheet(target, rigged, None);
        let mut images = Assets::<Image>::default();
        if let Some(pages) = asset.rigged.as_mut() {
            for page in &mut pages.pages {
                *page = images.reserve_handle();
                images.insert(page.id(), Image::default()).unwrap();
            }
        }
        app.insert_resource(images);
        asset
    };
    if views.is_some() {
        wait_for_pages(&mut app, &asset);
    }
    let mut assets = GameAssets::default();
    assets.characters.declare("bench", "Bench");
    assets.characters.publish("bench", asset.clone());
    app.insert_resource(assets);
    let feet = Vec2::new(asset.spec.feet_anchor_x, asset.spec.feet_anchor_y);
    let rows = actors.min(CELLS).div_ceil(COLUMNS);
    for index in 0..actors {
        let (sprite, anchor, animator) = build_character_presentation_with_render_size(&asset, RENDER, Anchor(feet));
        let cell = index % CELLS;
        let cell = Vec2::new((cell % COLUMNS) as f32, (cell / COLUMNS) as f32);
        let centre = Vec2::new(COLUMNS.min(actors) as f32 - 1.0, rows as f32 - 1.0) * 0.5;
        let at = (cell - centre) * SPACING;
        app.world_mut().spawn((
            sprite,
            anchor,
            animator,
            Transform::from_xyz(at.x, at.y, index as f32 * 0.01),
            Visibility::Inherited,
            BoundSpriteQuality {
                scale: TextureResolutionScale::Full,
            },
        ));
    }
    // Warm up: bind, and let every pose come round once.
    for _ in 0..(POSE_FRAMES as usize * POSES.len()) {
        app.update();
    }
    if views.is_some() {
        print_adapter_once(&app);
    }
    let mut samples: Vec<f64> = Vec::with_capacity(frames);
    for _ in 0..frames {
        let start = Instant::now();
        app.update();
        if views.is_some() {
            wait_for_gpu(&app);
        }
        samples.push(start.elapsed().as_secs_f64() * 1.0e6);
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let render = views.map(|_| {
        let render = app.sub_app(bevy::render::RenderApp).world();
        (
            render.resource::<bevy::sprite_render::ExtractedSprites>().sprites.len(),
            render.resource::<bevy::sprite_render::SpriteBatches>().len(),
        )
    });
    let world = app.world_mut();
    let entities = world.entities().count_spawned() as usize;
    let mut sprites = world.query_filtered::<&Visibility, With<Sprite>>();
    let visible_sprites = sprites
        .iter(world)
        .filter(|visibility| **visibility != Visibility::Hidden)
        .count();
    Row {
        actors,
        rigged,
        entities,
        visible_sprites,
        median_us: samples[samples.len() / 2],
        p95_us: samples[samples.len() * 95 / 100],
        render,
    }
}

/// Print the render adapter, once per process.
fn print_adapter_once(app: &App) {
    static PRINTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if PRINTED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    let info = &app
        .sub_app(bevy::render::RenderApp)
        .world()
        .resource::<bevy::render::renderer::RenderAdapterInfo>()
        .0;
    println!(
        "[rigged_sprite_bench] adapter name={} backend={:?} device_type={:?}",
        info.name, info.backend, info.device_type
    );
}

/// Block until the GPU has finished every submitted frame. Without it, a
/// hardware GPU's work overlaps the next `update` and the frame time measures
/// only the CPU that submits it.
fn wait_for_gpu(app: &App) {
    app.sub_app(bevy::render::RenderApp)
        .world()
        .resource::<bevy::render::renderer::RenderDevice>()
        .poll(bevy::render::render_resource::PollType::wait_indefinitely())
        .expect("the GPU finishes the frame");
}

/// Run frames until every sheet page and part page has loaded. A bench that
/// draws unloaded pages draws nothing and reports a small number.
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
