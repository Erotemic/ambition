//! Static world-visual spawning: blocks, water/climbable regions,
//! grid lines, loading-zone overlays, and authored `RoomObject`s.
//! `spawn_room_visuals` is the entry point called once per room
//! load.

use ambition_engine_core as ae;
use ambition_engine_core::AabbExt;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use super::nameplates::DoorNameplateSource;
use super::primitives::{
    block_color, feature_color, feature_z, spawn_world_label, FeatureVisual, LockWallVisual,
    PropVisual, RoomVisual,
};
use ambition_gameplay_core::assets::game_assets::{
    self, entity_sprite, entity_sprite_or_color, GameAssets,
};
use ambition_gameplay_core::character_sprites::{
    build_character_sprite, feet_anchor_for, CharacterAnimator,
};
use ambition_engine_core::config::{world_to_bevy, GRID_STEP, WORLD_Z_BLOCK, WORLD_Z_PLAYER};
use ambition_gameplay_core::features::FeatureVisualKind;
use ambition_gameplay_core::rooms::{LoadingZone, LoadingZoneActivation, PropSpec};
use ambition_gameplay_core::world::physics;

/// Presentation consumer of [`ambition_gameplay_core::session::RespawnRoomVisualsRequested`].
///
/// The sim (sandbox reset) emits the request after flipping the active room; this
/// reads the active room from [`RoomSet`] and rebuilds its static visuals +
/// parallax. Keeping the spawn on the render side means the sim never imports the
/// render layer, and a headless build (no presentation plugins) simply never runs
/// this system — correct, since it needs no visuals.
pub fn respawn_room_visuals_on_request(
    mut requests: MessageReader<ambition_gameplay_core::session::RespawnRoomVisualsRequested>,
    mut commands: Commands,
    room_set: Res<ambition_gameplay_core::rooms::RoomSet>,
    physics_settings: Res<physics::PhysicsSandboxSettings>,
    assets: Option<Res<GameAssets>>,
    quality: Option<Res<crate::quality::ResolvedVisualQuality>>,
) {
    if requests.is_empty() {
        return;
    }
    requests.clear();
    let spec = room_set.active_spec();
    super::spawn_parallax_layers(
        &mut commands,
        &spec.world,
        &spec.metadata,
        assets.as_deref(),
        quality.as_deref().map(|q| &q.budget.parallax),
    );
    spawn_room_visuals(&mut commands, spec, *physics_settings, assets.as_deref());
}

pub fn spawn_room_visuals(
    commands: &mut Commands,
    spec: &ambition_gameplay_core::rooms::RoomSpec,
    physics_settings: physics::PhysicsSandboxSettings,
    assets: Option<&GameAssets>,
) {
    let world = &spec.world;
    spawn_grid(commands, world);
    for block in &world.blocks {
        spawn_block(commands, world, block, physics_settings, assets);
    }
    for region in &world.water_regions {
        spawn_water_region(commands, world, region);
    }
    for region in &world.climbable_regions {
        spawn_climbable_region(commands, world, region);
    }
    for zone in &spec.loading_zones {
        spawn_loading_zone(commands, world, zone, assets);
    }
    // Per-family authored visuals. Each family carries an Authored<T>
    // payload; spawn_authored_visual builds the sprite + label.
    for hazard in &spec.hazards {
        spawn_authored_hazard(commands, world, hazard, assets);
    }
    for pickup in &spec.pickups {
        spawn_authored_basic(
            commands,
            world,
            &pickup.id,
            &pickup.name,
            pickup.aabb,
            FeatureVisualKind::Pickup,
            game_assets::entity_sprite_for_pickup(&pickup.payload),
            assets,
        );
    }
    for chest in &spec.chests {
        spawn_authored_chest(commands, world, chest, assets);
    }
    for breakable in &spec.breakables {
        spawn_authored_basic(
            commands,
            world,
            &breakable.id,
            &breakable.name,
            breakable.aabb,
            FeatureVisualKind::Breakable,
            game_assets::entity_sprite_for_breakable(&breakable.payload),
            assets,
        );
    }
    for enemy in &spec.enemy_spawns {
        let kind = ambition_gameplay_core::features::enemy_visual_kind(&enemy.payload);
        // Composite "X on Shark" spawns become a mount entity + a
        // rider entity on the sim side (see
        // `content::features::ecs::mount`). Fan the same way here
        // so each side has its own FeatureVisual entity — without
        // this both sims point at ids that don't match any room
        // visual, and `sync_visuals` hides them.
        if let Some(plan) = ambition_gameplay_core::features::composite_visual_plan(&enemy.payload)
        {
            spawn_composite_visuals(commands, world, enemy, kind, assets, &plan);
            continue;
        }
        spawn_authored_basic(
            commands,
            world,
            &enemy.id,
            &enemy.name,
            enemy.aabb,
            kind,
            game_assets::entity_sprite_for_enemy(&enemy.payload),
            assets,
        );
    }
    for boss in &spec.boss_spawns {
        spawn_authored_basic(
            commands,
            world,
            &boss.id,
            &boss.name,
            boss.aabb,
            FeatureVisualKind::Boss,
            game_assets::entity_sprite_for_boss(&boss.payload),
            assets,
        );
    }
    for interactable in &spec.interactables {
        spawn_authored_interactable(commands, world, interactable, assets);
    }
    for label in &spec.debug_labels {
        spawn_world_label(
            commands,
            world,
            label.payload.position,
            &label.payload.text,
            14.0,
        );
    }
    for prop in &spec.props {
        spawn_room_prop(commands, world, prop, assets);
    }
}

/// Spawn the visual entity for one [`PropSpec`]. Falls back to a
/// colored rectangle when the prop's `kind` is unknown or its asset
/// hasn't loaded yet.
///
/// Always inserts:
/// - `RoomVisual` so the room-swap path despawns the prop with the
///   rest of the room's presentation.
/// - `PropVisual { id, kind, size }` so the generic prop-anim tick can
///   find it and so debug overlays can label it.
/// - `ambition_gameplay_core::features::FeatureName(prop.name)` so per-name systems
///   (portal visibility / ring rotation / portal anim) keep finding
///   the gate ring + gate portal entities after the migration from
///   NpcSpawn-as-prop.
pub fn spawn_room_prop(
    commands: &mut Commands,
    world: &ae::World,
    prop: &PropSpec,
    assets: Option<&GameAssets>,
) {
    let kind = FeatureVisualKind::Npc;
    let z = feature_z(kind);
    let translation = world_to_bevy(world, prop.pos, z);
    let collision = BVec2::new(prop.size.x, prop.size.y);

    let mut entity = commands.spawn((
        Transform::from_translation(translation),
        Name::new(format!("Prop: {}", prop.name)),
        RoomVisual,
        PropVisual {
            id: prop.id.clone(),
            kind: prop.kind.clone(),
            size: BVec2::new(prop.size.x, prop.size.y),
        },
        ambition_gameplay_core::features::FeatureName::new(prop.name.clone()),
    ));

    if let Some(asset) = assets.and_then(|a| a.characters.prop_asset_for_kind(&prop.kind)) {
        let sprite = build_character_sprite(asset, collision);
        entity.insert((
            sprite,
            feet_anchor_for(&asset.spec, collision),
            CharacterAnimator::new(asset),
        ));
    } else {
        // Fallback: a translucent placeholder rectangle so authors
        // see a visible marker for unregistered prop kinds. Same
        // pattern as other "asset missing" fallbacks in the renderer.
        entity.insert(Sprite::from_color(
            Color::srgba(0.55, 0.45, 0.85, 0.55),
            collision,
        ));
    }
}

/// Render a single `WaterRegion` as a tinted overlay quad. Source-
/// agnostic: any region — IntGrid `Water` or entity `WaterVolume` —
/// uses the same path. Two layers per kind:
///
/// - **Body**: a tinted rect spanning the whole region. Clear sits
///   *behind* the player so the player is visible while submerged;
///   Murky sits *in front of* the player so it actually hides what
///   is underneath.
/// - **Surface strip**: a brighter band along the top edge so the
///   water surface reads at a glance even with a flat tint.
fn spawn_water_region(commands: &mut Commands, world: &ae::World, region: &ae::WaterRegion) {
    let size = region.aabb.half_size() * 2.0;
    let render = BVec2::new(size.x, size.y);
    let (body_color, body_z) = match region.kind {
        // Cool blue, mostly transparent. Z just above blocks so the
        // floor tint shows through; player draws on top normally.
        ae::WaterKind::Clear => (Color::srgba(0.24, 0.72, 0.88, 0.32), WORLD_Z_BLOCK + 5.0),
        // Dark teal, near-opaque. Z above the player so anything
        // beneath the surface is genuinely hidden.
        ae::WaterKind::Murky => (Color::srgba(0.10, 0.20, 0.18, 0.88), WORLD_Z_PLAYER + 5.0),
    };
    commands.spawn((
        Sprite::from_color(body_color, render),
        Transform::from_translation(world_to_bevy(world, region.aabb.center(), body_z)),
        Name::new(format!("Water body ({:?})", region.kind)),
        RoomVisual,
    ));

    // Surface strip: a brighter band 4px tall at the very top of the
    // region. The strip always renders above the body and the
    // player so the surface reads cleanly even through Murky.
    let strip_color = match region.kind {
        ae::WaterKind::Clear => Color::srgba(0.82, 0.95, 1.0, 0.85),
        ae::WaterKind::Murky => Color::srgba(0.55, 0.78, 0.62, 0.95),
    };
    let strip_h = 4.0;
    let strip_size = BVec2::new(size.x, strip_h);
    let strip_center = ae::Vec2::new(region.aabb.center().x, region.aabb.top() + strip_h * 0.5);
    commands.spawn((
        Sprite::from_color(strip_color, strip_size),
        Transform::from_translation(world_to_bevy(world, strip_center, WORLD_Z_PLAYER + 6.0)),
        Name::new(format!("Water surface ({:?})", region.kind)),
        RoomVisual,
    ));
}

/// Render a single `ClimbableRegion` as a tinted overlay quad +
/// "rung" stripes for visual rhythm. Mirror of `spawn_water_region`'s
/// shape; placeholder until proper ladder/vine/wall sprite art lands.
/// All three kinds share the same overlay shape but with kind-specific
/// tint so the player can tell at a glance what they're touching.
fn spawn_climbable_region(
    commands: &mut Commands,
    world: &ae::World,
    region: &ae::ClimbableRegion,
) {
    let size = region.aabb.half_size() * 2.0;
    let render = BVec2::new(size.x, size.y);
    // Sit above blocks but below the player so the ladder reads as
    // background scenery the player climbs in front of.
    let body_z = WORLD_Z_BLOCK + 4.0;
    let (body_color, rung_color) = match region.kind {
        // Brown ladder with darker rung accents.
        ae::ClimbableKind::Ladder => (
            Color::srgba(0.76, 0.52, 0.28, 0.90),
            Color::srgba(0.45, 0.30, 0.15, 1.0),
        ),
        // Green vine with yellow-green leaf accents.
        ae::ClimbableKind::Vine => (
            Color::srgba(0.37, 0.64, 0.32, 0.85),
            Color::srgba(0.65, 0.85, 0.40, 1.0),
        ),
        // Tan/sand climbable wall, no rung accents.
        ae::ClimbableKind::Wall => (
            Color::srgba(0.61, 0.48, 0.29, 0.80),
            Color::srgba(0.45, 0.35, 0.20, 0.0), // alpha=0 = no rungs
        ),
    };
    commands.spawn((
        Sprite::from_color(body_color, render),
        Transform::from_translation(world_to_bevy(world, region.aabb.center(), body_z)),
        Name::new(format!("Climbable body ({:?})", region.kind)),
        RoomVisual,
    ));

    // Add rung stripes spaced every 16 px on the y axis. Skipped for
    // Wall (rung_color alpha=0). Quick visual rhythm so a tall ladder
    // doesn't look like a flat colored block.
    if rung_color.alpha() > 0.0 {
        let rung_h = 3.0;
        let rung_size = BVec2::new(size.x, rung_h);
        let mut y = region.aabb.top() + 8.0;
        while y < region.aabb.bottom() - 4.0 {
            let center = ae::Vec2::new(region.aabb.center().x, y);
            commands.spawn((
                Sprite::from_color(rung_color, rung_size),
                Transform::from_translation(world_to_bevy(world, center, body_z + 0.5)),
                Name::new(format!("Climbable rung ({:?})", region.kind)),
                RoomVisual,
            ));
            y += 16.0;
        }
    }
}

pub fn spawn_grid(commands: &mut Commands, world: &ae::World) {
    let grid_color = Color::srgba(0.12, 0.15, 0.22, 0.28);
    let mut x = 0.0;
    while x <= world.size.x {
        let center = ae::Vec2::new(x, world.size.y * 0.5);
        commands.spawn((
            Sprite::from_color(grid_color, BVec2::new(1.0, world.size.y)),
            Transform::from_translation(world_to_bevy(world, center, -20.0)),
            RoomVisual,
        ));
        x += GRID_STEP;
    }
    let mut y = 0.0;
    while y <= world.size.y {
        let center = ae::Vec2::new(world.size.x * 0.5, y);
        commands.spawn((
            Sprite::from_color(grid_color, BVec2::new(world.size.x, 1.0)),
            Transform::from_translation(world_to_bevy(world, center, -20.0)),
            RoomVisual,
        ));
        y += GRID_STEP;
    }
}

/// Pick a `Tiled` stretch value that keeps the slice count under
/// `MAX_TILES_PER_AXIS²`. Tiles are sized at `source × stretch`, so
/// raising the stretch reduces tile count proportionally. Returns 1.0
/// (native size) when the block fits inside the cap.
fn tiled_block_stretch(render: BVec2, source_px: f32) -> f32 {
    const MAX_TILES_PER_AXIS: f32 = 32.0;
    let source = source_px.max(1.0);
    let tiles_x = (render.x / source).max(1.0);
    let tiles_y = (render.y / source).max(1.0);
    let needed = (tiles_x.max(tiles_y) / MAX_TILES_PER_AXIS).max(1.0);
    needed.ceil()
}

/// Marker for already-spawned single-image entity sprites whose `Handle<Image>`
/// should be rebound when `GameAssets` is rebuilt for a confirmed quality
/// change. The marker is intentionally handle-only: it preserves the current
/// sprite size, image mode, atlas-free shape, tint, visibility, and entity
/// identity, avoiding the despawn/respawn bugs from earlier live-refresh attempts.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundEntitySprite {
    key: game_assets::EntitySprite,
}

impl BoundEntitySprite {
    fn new(key: game_assets::EntitySprite) -> Self {
        Self { key }
    }
}

pub fn refresh_entity_sprite_handles_on_game_assets_change(
    assets: Option<Res<GameAssets>>,
    mut sprites: Query<
        (&BoundEntitySprite, &mut Sprite),
        (
            Without<CharacterAnimator>,
            Without<ambition_gameplay_core::boss_encounter::sprites::BossAnimator>,
        ),
    >,
) {
    let Some(assets) = assets else {
        return;
    };
    if !assets.is_changed() {
        return;
    }
    for (bound, mut sprite) in &mut sprites {
        if let Some(handle) = assets.entities.get(bound.key) {
            if sprite.image != *handle {
                sprite.image = handle.clone();
            }
        }
    }
}

pub fn spawn_block(
    commands: &mut Commands,
    world: &ae::World,
    block: &ae::Block,
    physics_settings: physics::PhysicsSandboxSettings,
    assets: Option<&GameAssets>,
) {
    let size = block.aabb.half_size() * 2.0;
    let render = BVec2::new(size.x, size.y);
    // IntGrid-derived blocks (named "ldtk *" by `int_grid_value_to_block`)
    // can be arbitrary aspect ratios (1904×32 floors, 48×240 pillars, …).
    // Stretching the single 128-px entity-art textures across those
    // smears the texture's internal structure into a false repeat.
    // Solution: tiled 32×32 textures (one per BlockKind) repeated via
    // `Sprite::image_mode = Tiled` so the texture renders at native
    // pixel scale and TILES to fill `custom_size` — exactly what a
    // long stone floor or tall pillar wants.
    //
    // Falls back to a colored quad when the tile asset is missing
    // (no-asset mode, missing file). Authored entity-derived blocks
    // (e.g. authored Solid rectangles outside the IntGrid layer) keep
    // the entity-art path because their footprints match the texture
    // aspect ratio.
    let is_intgrid_block = block.name.starts_with("ldtk ");
    let sprite_key = if is_intgrid_block {
        game_assets::block_tile_sprite(block.kind)
    } else {
        game_assets::block_sprite(block.kind)
    };
    let sprite = if is_intgrid_block {
        let tile_handle = assets
            .and_then(|a| sprite_key.and_then(|key| a.entities.get(key)))
            .cloned();
        match tile_handle {
            Some(image) => Sprite {
                image,
                custom_size: Some(render),
                image_mode: bevy::sprite::SpriteImageMode::Tiled {
                    tile_x: true,
                    tile_y: true,
                    // Clamp the slice count for very large IntGrid
                    // surfaces. With a 32px source tile and 1.0 stretch,
                    // a single 3072×3328 floor would emit ~9984 slices
                    // and trigger a bevy_sprite performance warning.
                    // Scaling the tile up keeps the visual tiling but
                    // bounds the per-block slice count.
                    stretch_value: tiled_block_stretch(render, 32.0),
                },
                ..Default::default()
            },
            None => Sprite::from_color(block_color(block.kind), render),
        }
    } else {
        match assets {
            Some(a) => entity_sprite_or_color(a, sprite_key, render, block_color(block.kind)),
            None => Sprite::from_color(block_color(block.kind), render),
        }
    };
    let mut entity = commands.spawn((
        sprite,
        Transform::from_translation(world_to_bevy(world, block.aabb.center(), WORLD_Z_BLOCK)),
        Name::new(format!("Block: {}", block.name)),
        RoomVisual,
    ));
    if let Some(key) = sprite_key {
        entity.insert(BoundEntitySprite::new(key));
    }
    physics::spawn_static_collider_for_block(commands, world, block, physics_settings);
}

/// Width-to-height aspect of the authored `door_zone.png` (published with
/// `ground = true`, so its bottom edge is the door's feet). A door preserves
/// this aspect instead of stretching to fill the trigger box — which can be
/// any size — so it never looks squashed. Keep in sync with the `door_zone`
/// drawer in the sprite renderer.
const DOOR_SPRITE_ASPECT: f32 = 0.56;

pub fn spawn_loading_zone(
    commands: &mut Commands,
    world: &ae::World,
    zone: &LoadingZone,
    assets: Option<&GameAssets>,
) {
    let size = zone.aabb.half_size() * 2.0;
    let fallback_color = match zone.activation {
        LoadingZoneActivation::EdgeExit => Color::srgba(0.20, 0.95, 1.0, 0.22),
        LoadingZoneActivation::Door => Color::srgba(1.0, 0.72, 0.18, 0.46),
        // Walk-through portal: green tint to distinguish from edge
        // exits while still reading as "step in and go."
        LoadingZoneActivation::Walk => Color::srgba(0.40, 1.00, 0.55, 0.30),
    };
    // A `Door` is a standing prop: render it like a character. Its feet (the
    // sprite's bottom edge) plant on the bottom (floor) face of the trigger
    // box via a bottom-centre anchor, and it keeps its authored aspect rather
    // than stretching to the box. Edge-exit / walk zones stay box-filling
    // tints, anchored at the box centre. This is why doors stand on the
    // ground without any per-placement nudging — the box is authored flush
    // to the floor, and the feet anchor does the rest.
    let grounded = matches!(zone.activation, LoadingZoneActivation::Door);
    let (render, sprite_pos, anchor) = if grounded {
        let height = size.y;
        let width = height * DOOR_SPRITE_ASPECT;
        // Bottom-centre of the box in world space (y-down → +half_y is the
        // floor edge).
        let foot = zone.aabb.center() + ae::Vec2::new(0.0, zone.aabb.half_size().y);
        (BVec2::new(width, height), foot, Anchor::BOTTOM_CENTER)
    } else {
        (
            BVec2::new(size.x, size.y),
            zone.aabb.center(),
            Anchor::CENTER,
        )
    };
    let sprite_key = game_assets::loading_zone_sprite(zone.activation);
    let sprite = match assets {
        Some(a) => entity_sprite(a, sprite_key, render, fallback_color),
        None => Sprite::from_color(fallback_color, render),
    };
    let mut visual = commands.spawn((
        sprite,
        anchor,
        Transform::from_translation(world_to_bevy(world, sprite_pos, WORLD_Z_BLOCK + 6.0)),
        Name::new(format!("Loading zone: {}", zone.name)),
        // Marker carrying the zone id so portal-aware systems can
        // hide the debug door visual for portal-mode LoadingZones
        // (the portal sprite IS the door visual; the DoorZone box
        // behind it reads as a second door).
        crate::rendering::primitives::LoadingZoneVisual {
            id: zone.id.clone(),
        },
        RoomVisual,
        BoundEntitySprite::new(sprite_key),
    ));
    if matches!(zone.activation, LoadingZoneActivation::Door) {
        visual.insert(DoorNameplateSource::new(
            zone.id.clone(),
            zone.name.clone(),
            zone.aabb,
        ));
    } else {
        let label_pos = zone.aabb.center() + ae::Vec2::new(0.0, -zone.aabb.half_size().y - 18.0);
        spawn_world_label(commands, world, label_pos, &zone.name, 13.0);
    }
}

/// Common spawn body for an authored entity with a sprite and no
/// label. Hazards, pickups, breakables, enemies, bosses all funnel
/// through here — they differ only in `kind` + which `EntitySprite`
/// the asset bank resolves to.
fn spawn_authored_basic(
    commands: &mut Commands,
    world: &ae::World,
    id: &str,
    name: &str,
    aabb: ae::Aabb,
    kind: FeatureVisualKind,
    entity_key: Option<game_assets::EntitySprite>,
    assets: Option<&GameAssets>,
) {
    let size = aabb.half_size() * 2.0;
    let render = BVec2::new(size.x, size.y);
    let sprite = match assets {
        Some(a) => entity_sprite_or_color(a, entity_key, render, feature_color(kind, false)),
        None => Sprite::from_color(feature_color(kind, false), render),
    };
    let mut entity = commands.spawn((
        sprite,
        Transform::from_translation(world_to_bevy(world, aabb.center(), feature_z(kind))),
        Name::new(format!("Room entity: {}", name)),
        FeatureVisual { id: id.to_string() },
        ambition_gameplay_core::features::FeatureName::new(name.to_string()),
        RoomVisual,
    ));
    if let Some(key) = entity_key {
        entity.insert(BoundEntitySprite::new(key));
    }
}

/// Spawn the mount + rider FeatureVisual entities for an authored
/// composite "X on Shark" enemy. Mirrors the sim-side fan out in
/// `content::features::ecs::spawn::spawn_composite_mount_rider`: the
/// mount keeps the AUTHORED id (so existing same-room reset paths
/// and id-keyed save flags still target the mount entry), and the
/// rider takes the `<id>:rider` suffix.
///
/// The actual positions / sizes get rewritten by `sync_visuals`
/// each frame off the FeatureViewIndex (which the sim entities
/// populate); the spawn just needs a placeholder Sprite + Transform
/// big enough for `upgrade_enemy_sprites` to replace with a textured
/// CharacterSprite once the asset bank loads.
fn spawn_composite_visuals(
    commands: &mut Commands,
    world: &ae::World,
    enemy: &ambition_gameplay_core::rooms::Authored<ambition_characters::actor::CharacterBrain>,
    kind: FeatureVisualKind,
    assets: Option<&GameAssets>,
    plan: &ambition_gameplay_core::features::CompositeVisualPlan,
) {
    // Mount: keep authored id; reuse the authored aabb (the mount's
    // default_size is what the sim-side composite spawn uses too).
    spawn_authored_basic(
        commands,
        world,
        &enemy.id,
        &plan.mount_name,
        enemy.aabb,
        kind,
        game_assets::entity_sprite_for_enemy(&plan.mount_brain),
        assets,
    );
    // Rider: ":rider" suffix; display name parsed from the authored
    // spawn name when it carries the " on Shark" composite suffix.
    let rider_name = if plan.rider_name_from_spawn {
        enemy
            .name
            .strip_suffix(" on Shark")
            .map(str::to_owned)
            .unwrap_or_else(|| plan.rider_fallback_name.clone())
    } else {
        plan.rider_fallback_name.clone()
    };
    // Rider renders at HALF its standalone size while mounted so it
    // fits visually on the mount — mirrors the sim-side `MountedSize`;
    // `sync_visuals` re-syncs from the FeatureViewIndex next frame,
    // but the initial visual must be sized right to avoid a tick-1 pop.
    let rider_size = plan.rider_standalone_size * 0.5;
    let rider_offset_y = -(plan.mount_size.y * 0.5) - (rider_size.y * 0.5) + 8.0;
    let rider_pos = enemy.aabb.center() + ae::Vec2::new(0.0, rider_offset_y);
    let rider_aabb = ae::Aabb::new(rider_pos, rider_size * 0.5);
    spawn_authored_basic(
        commands,
        world,
        &format!("{}:rider", enemy.id),
        &rider_name,
        rider_aabb,
        kind,
        game_assets::entity_sprite_for_enemy(&plan.rider_brain),
        assets,
    );
}

fn spawn_authored_hazard(
    commands: &mut Commands,
    world: &ae::World,
    authored: &ambition_gameplay_core::rooms::Authored<
        ambition_gameplay_core::combat::DamageVolume,
    >,
    assets: Option<&GameAssets>,
) {
    spawn_authored_basic(
        commands,
        world,
        &authored.id,
        &authored.name,
        authored.aabb,
        FeatureVisualKind::Hazard,
        game_assets::entity_sprite_for_hazard(&authored.payload),
        assets,
    );
}

fn spawn_authored_chest(
    commands: &mut Commands,
    world: &ae::World,
    authored: &ambition_gameplay_core::rooms::Authored<ambition_interaction::Chest>,
    assets: Option<&GameAssets>,
) {
    spawn_authored_basic(
        commands,
        world,
        &authored.id,
        &authored.name,
        authored.aabb,
        FeatureVisualKind::Chest,
        game_assets::entity_sprite_for_chest(&authored.payload),
        assets,
    );
    // Chest label (mirrors the pre-migration behavior).
    let half_h = authored.aabb.half_size().y;
    spawn_world_label(
        commands,
        world,
        authored.aabb.center() + ae::Vec2::new(0.0, -half_h - 22.0),
        &authored.name,
        14.0,
    );
}

fn spawn_authored_interactable(
    commands: &mut Commands,
    world: &ae::World,
    authored: &ambition_gameplay_core::rooms::Authored<ambition_interaction::Interactable>,
    assets: Option<&GameAssets>,
) {
    let interactable = &authored.payload;
    let kind = if matches!(
        interactable.kind,
        ambition_interaction::InteractionKind::Npc { .. }
    ) {
        FeatureVisualKind::Npc
    } else if matches!(&interactable.kind, ambition_interaction::InteractionKind::Custom(s) if s.starts_with("switch:"))
    {
        FeatureVisualKind::Switch
    } else {
        return;
    };
    spawn_authored_basic(
        commands,
        world,
        &authored.id,
        &authored.name,
        authored.aabb,
        kind,
        game_assets::entity_sprite_for_interactable(interactable),
        assets,
    );
    // NPC labels are rendered by the presentation nameplate system. Keep this
    // spawn path to sprites/features only so authored map labels, chest labels,
    // and non-door loading-zone labels remain independent presentation surfaces.
}

/// Block-name prefixes whose presence in `world.blocks` should be
/// reflected as a `LockWallVisual` Bevy entity. The encounter system
/// writes `lockwall:<id>` blocks; the intro-v1 flag-gated lock-wall
/// system writes `intro_lock:<id>` blocks. Both are surfaced with the
/// same `LockWallTile` sprite so a Task 08 conditional gate reads the
/// same way as an encounter-driven slam.
const LOCK_WALL_BLOCK_PREFIXES: &[&str] = &["lockwall:", "intro_lock:"];

fn is_lock_wall_block(name: &str) -> bool {
    LOCK_WALL_BLOCK_PREFIXES
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

/// Reconcile `LockWallVisual` Bevy entities against the encounter-
/// driven `lockwall:*` and intro flag-gated `intro_lock:*` gate solids
/// the gates contribute to the per-frame collision overlay. Spawn a
/// sprite for any new lock wall, despawn entities whose backing block
/// has been removed (encounter cleared / failed, or flag unlocked).
///
/// The walls live in [`FeatureEcsWorldOverlay::gate_solids`] (derived
/// each frame), NOT the authored `RoomGeometry` base — so this reads the
/// overlay for the block set and the base only for the world→screen
/// coordinate frame. Without this system the lock wall has collision
/// (the overlay folds it into every collision read-path) but no rendered
/// tile — the player bumps into an invisible barrier. The dedicated
/// `LockWallTile` asset keeps the visual distinct from regular solid
/// walls so the "this just slammed shut" beat reads at a glance.
pub fn sync_lock_wall_visuals(
    mut commands: Commands,
    world: Res<ambition_gameplay_core::RoomGeometry>,
    overlay: Res<ambition_gameplay_core::features::FeatureEcsWorldOverlay>,
    assets: Option<Res<GameAssets>>,
    existing: Query<(Entity, &LockWallVisual)>,
) {
    use bevy::math::Vec2 as BVec2;

    // Index existing visuals by their backing block name so we can
    // diff against the world snapshot in linear time.
    let mut existing_by_name: std::collections::HashMap<String, Entity> =
        std::collections::HashMap::new();
    for (entity, visual) in &existing {
        existing_by_name.insert(visual.block_name.clone(), entity);
    }

    // Pass 1: spawn a visual for any lock-wall block (encounter or
    // intro flag-gated) that doesn't have one yet. Mark consumed
    // names so the despawn pass below leaves them alone.
    let mut consumed: std::collections::HashSet<String> = std::collections::HashSet::new();
    for block in &overlay.gate_solids {
        if !is_lock_wall_block(&block.name) {
            continue;
        }
        if existing_by_name.contains_key(&block.name) {
            consumed.insert(block.name.clone());
            continue;
        }
        let size = block.aabb.half_size() * 2.0;
        let render = BVec2::new(size.x, size.y);
        // Bright purple fallback when no asset is loaded — distinct
        // from the standard solid-block fallback so a missing tile
        // is obvious in playtest.
        let fallback = Color::srgba(0.65, 0.20, 0.85, 0.92);
        let sprite = match assets.as_deref() {
            Some(a) => entity_sprite_or_color(
                a,
                Some(game_assets::EntitySprite::LockWallTile),
                render,
                fallback,
            ),
            None => Sprite::from_color(fallback, render),
        };
        commands.spawn((
            sprite,
            Transform::from_translation(world_to_bevy(
                &world.0,
                block.aabb.center(),
                // Sit just above the regular block layer so a lock
                // wall reads on top of any floor/wall art it overlaps.
                WORLD_Z_BLOCK + 4.0,
            )),
            Name::new(format!("LockWall: {}", block.name)),
            LockWallVisual {
                block_name: block.name.clone(),
            },
            BoundEntitySprite::new(game_assets::EntitySprite::LockWallTile),
            RoomVisual,
        ));
        consumed.insert(block.name.clone());
    }

    // Pass 2: despawn visuals whose gate solid disappeared (encounter
    // cleared / failed, or flag unlocked → the contributor stopped
    // deriving the block this frame).
    for (name, entity) in &existing_by_name {
        if !consumed.contains(name) {
            commands.entity(*entity).despawn();
        }
    }
}

#[cfg(test)]
mod lock_wall_visual_tests {
    use super::*;
    use ambition_gameplay_core::features::FeatureEcsWorldOverlay;
    use ambition_gameplay_core::RoomGeometry;

    fn room() -> RoomGeometry {
        RoomGeometry(ae::World::new(
            "test",
            ae::Vec2::new(800.0, 600.0),
            ae::Vec2::new(50.0, 50.0),
            Vec::new(),
        ))
    }

    fn gate_wall() -> ae::Block {
        ae::Block::solid(
            "lockwall:goblin_encounter",
            ae::Vec2::new(300.0, 300.0),
            ae::Vec2::new(16.0, 100.0),
        )
    }

    fn lock_wall_names(app: &mut App) -> Vec<String> {
        let mut q = app.world_mut().query::<&LockWallVisual>();
        let mut names: Vec<String> = q.iter(app.world()).map(|v| v.block_name.clone()).collect();
        names.sort();
        names
    }

    /// The reconcile reads the overlay's `gate_solids` (NOT the authored base):
    /// a gate solid spawns a `LockWallVisual`, and dropping it from the overlay
    /// despawns the visual. This is what keeps lock walls visible after the
    /// move off the base — the render contract the base→overlay conversion must
    /// preserve.
    #[test]
    fn lock_wall_visual_tracks_overlay_gate_solids() {
        let mut app = App::new();
        app.insert_resource(room());
        app.insert_resource(FeatureEcsWorldOverlay {
            gate_solids: vec![gate_wall()],
            ..Default::default()
        });
        app.add_systems(Update, sync_lock_wall_visuals);

        app.update();
        assert_eq!(
            lock_wall_names(&mut app),
            vec!["lockwall:goblin_encounter".to_string()],
            "a gate solid spawns its LockWallVisual"
        );

        // Encounter cleared / flag unlocked → the contributor stops deriving the
        // wall, so the overlay no longer carries it and the visual despawns.
        app.world_mut()
            .resource_mut::<FeatureEcsWorldOverlay>()
            .gate_solids
            .clear();
        app.update();
        assert!(
            lock_wall_names(&mut app).is_empty(),
            "dropping the gate solid despawns the LockWallVisual"
        );
    }
}
