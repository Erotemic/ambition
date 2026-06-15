//! Per-frame Bevy systems that mirror engine actor state into Bevy
//! sprites + animations. Covers the player, enemies, and bosses
//! along with the upgrade-to-spritesheet pass that converts the
//! initial colored rectangles into authored character sprites once
//! the asset is loaded.

use ambition_sandbox::engine_core as ae;
use ambition_sandbox::engine_core::AabbExt;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use super::primitives::{
    feature_color, feature_z, switch_on_color, FeatureVisual, PlayerSpriteBaseline, PlayerVisual,
    PropVisual, SceneEntities,
};
use ambition_sandbox::assets::game_assets::{self, EntitySprite, GameAssets};
use ambition_sandbox::boss_encounter::sprites::{self, BossAnimState, BossAnimator};
use ambition_sandbox::config::{world_to_bevy, WORLD_Z_PLAYER};
use ambition_sandbox::features::{
    BossClusterRef, BreakableFeature, ChestFeature, FeatureId, FeatureViewIndex, FeatureVisualKind,
    Opened,
};
use ambition_sandbox::mechanics::combat::BoundFeatureKind;
use ambition_sandbox::character_sprites::{
    build_character_sprite, feet_anchor_for, CharacterAnimator,
};

pub fn sync_visuals(
    world: Res<ambition_sandbox::GameWorld>,
    entities: Res<SceneEntities>,
    assets: Option<Res<GameAssets>>,
    feature_views: Res<FeatureViewIndex>,
    mut player_query: Query<
        (
            &mut Transform,
            &mut Sprite,
            Option<&PlayerSpriteBaseline>,
            &ambition_sandbox::player::BodyKinematics,
            &ambition_sandbox::player::PlayerBaseSize,
            &ambition_sandbox::player::PlayerCombatState,
            Option<&ambition_sandbox::platformer_runtime::orientation::ActorRoll>,
        ),
        With<PlayerVisual>,
    >,
    mut feature_query: Query<
        (&FeatureVisual, &mut Transform, &mut Sprite, &mut Visibility),
        Without<PlayerVisual>,
    >,
    ecs_chest_states: Query<(&FeatureId, Option<&Opened>), With<ChestFeature>>,
    ecs_breakable_states: Query<(&FeatureId, &BreakableFeature)>,
) {
    if let Ok((mut transform, mut sprite, baseline, body, base_size, player_combat, roll)) =
        player_query.get_mut(entities.player)
    {
        transform.translation = world_to_bevy(&world.0, body.pos, WORLD_Z_PLAYER);
        // Aerial roll (portal somersault / future gravity-room orientation).
        transform.rotation = Quat::from_rotation_z(roll.map_or(0.0, |r| r.angle));
        if sprite.texture_atlas.is_none() && sprite.image == Handle::default() {
            // Colored-rectangle fallback only — stretch to the collision-box
            // size and tint by flash. Textured sprites (atlas OR plain image)
            // keep their authored size and are tinted in the animation system.
            sprite.custom_size = Some(BVec2::new(body.size.x, body.size.y));
            let alpha = if player_combat.flash_timer > 0.0 {
                0.72
            } else {
                1.0
            };
            sprite.color = Color::srgba(0.80, 0.95, 1.0, alpha);
        } else if let Some(baseline) = baseline {
            // HACK(crouch-sprite-row): when the player crouches (or
            // morphs / crawls / slides), the engine shrinks the AABB
            // and slides `pos.y` down to keep feet planted. The
            // textured sprite was sized for the standing pose, so
            // without compensation it floats below the floor by half
            // the height delta. Re-scale the sprite's vertical extent
            // by the same ratio the collision shrunk; the normalized
            // sprite anchor preserves foot alignment automatically.
            // Phase 1 also lets the development menu swap standing body
            // profiles live. Scale the placeholder art against the recorded
            // startup collision so body-profile experiments remain visual.
            // Replace with authored body-profile rows once the generator emits
            // them — see PlayerSpriteBaseline doc.
            let base_y = base_size.base_size.y.max(1.0);
            let stance_ratio_y = (body.size.y / base_y).clamp(0.1, 1.0);
            let scale_x = base_size.base_size.x / baseline.standing_collision.x.max(1.0);
            let scale_y = base_size.base_size.y / baseline.standing_collision.y.max(1.0);
            sprite.custom_size = Some(BVec2::new(
                baseline.standing_render.x * scale_x,
                baseline.standing_render.y * scale_y * stance_ratio_y,
            ));
        }
    }

    for (visual, mut transform, mut sprite, mut visibility) in &mut feature_query {
        let Some(view) = feature_views.get(&visual.id) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        transform.translation = world_to_bevy(&world.0, view.pos, feature_z(view.kind));
        // Surface-walking enemies (PuppySlug) rotate the sprite so
        // its authored "up" axis aligns with the surface normal —
        // the slug crawls along walls / ceilings with its body
        // visibly clinging to them. All other actors stay axis-
        // aligned (rotation_rad = 0).
        transform.rotation = Quat::from_rotation_z(view.rotation_rad);

        // State-aware sprite swap for breakables and chests. Pickups are
        // chosen at spawn time and never change kind. Enemies are animated
        // through the character spritesheet path.
        if let Some(assets) = assets.as_deref() {
            if let Some(target_key) = state_aware_entity_sprite(
                &visual.id,
                view.kind,
                view.switch_on,
                &ecs_chest_states,
                &ecs_breakable_states,
            ) {
                if let Some(handle) = assets.entities.get(target_key) {
                    if sprite.image != *handle {
                        sprite.image = handle.clone();
                    }
                }
            }
        }

        if sprite.texture_atlas.is_none() && sprite.image == Handle::default() {
            // Bare colored rectangle (no entity sprite available, no atlas).
            sprite.custom_size = Some(BVec2::new(view.size.x, view.size.y));
            sprite.color = if matches!(view.kind, FeatureVisualKind::Switch) && view.switch_on {
                switch_on_color()
            } else {
                feature_color(view.kind, view.flash)
            };
        } else if sprite.texture_atlas.is_none() {
            // Textured single-image entity sprite. Keep author size; tint
            // for hit-flash, otherwise white.
            sprite.custom_size = Some(BVec2::new(view.size.x, view.size.y));
            sprite.color = if view.flash {
                Color::srgba(1.0, 0.55, 0.55, 1.0)
            } else {
                Color::WHITE
            };
        }
        *visibility = if view.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn state_aware_entity_sprite(
    id: &str,
    kind: FeatureVisualKind,
    switch_on: bool,
    ecs_chests: &Query<(&FeatureId, Option<&Opened>), With<ChestFeature>>,
    ecs_breakables: &Query<(&FeatureId, &BreakableFeature)>,
) -> Option<EntitySprite> {
    match kind {
        FeatureVisualKind::Breakable => ambition_sandbox::features::ecs_breakable_state(id, ecs_breakables)
            .map(game_assets::breakable_state_sprite),
        FeatureVisualKind::Chest => {
            ambition_sandbox::features::ecs_chest_opened(id, ecs_chests).map(game_assets::chest_state_sprite)
        }
        // Switch shows its on/off button sprite (armed = on, disabled = off)
        // instead of a flat colored block (#57).
        FeatureVisualKind::Switch => Some(if switch_on {
            EntitySprite::SwitchArmed
        } else {
            EntitySprite::SwitchDisabled
        }),
        _ => None,
    }
}

/// Marker recording which `FeatureVisualKind` the current sprite +
/// `CharacterAnimator` were bound for. The upgrade systems read this
/// to detect mid-life kind changes — e.g. when a peaceful NPC turns
/// hostile and `apply_save` migrates the runtime entry from `npcs`
/// to `enemies`. Without this marker, the existing
/// `Without<CharacterAnimator>` filter hid the entity from the enemy
/// upgrade pass and the kernel guide stayed visually a kernel guide
/// after the third strike.

/// Bind enemy/sandbag visuals to the appropriate character sheet
/// once the asset is available — and re-bind when an existing visual
/// changes kind (e.g. NPC → Enemy on hostility flip).
pub fn upgrade_enemy_sprites(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    images: Res<Assets<Image>>,
    feature_views: Res<FeatureViewIndex>,
    features: Query<(Entity, &FeatureVisual, Option<&BoundFeatureKind>)>,
    ecs_actors: Query<ambition_sandbox::features::ActorSpriteData>,
    // Names we've already warned about resolving no sprite, so the warning fires
    // once per offending name instead of every frame the actor is unbound.
    mut warned_sprite_names: Local<std::collections::HashSet<String>>,
) {
    let Some(assets) = assets else {
        return;
    };
    for (entity, visual, bound) in &features {
        let Some(view) = feature_views.get(&visual.id) else {
            continue;
        };
        if !matches!(
            view.kind,
            FeatureVisualKind::Enemy | FeatureVisualKind::TrainingDummy
        ) {
            continue;
        }
        let collision = BVec2::new(view.size.x, view.size.y);
        // Already bound to the correct kind and collision footprint — nothing
        // to do this frame. The collision-size check is still useful for rare
        // intentional runtime size changes, but shark riders should normally
        // keep the same visual/collision scale across mount and dismount.
        if bound.is_some_and(|b| b.matches(view.kind, view.size)) {
            continue;
        }
        // Sprite-override path: an enemy that was spawned by migrating
        // a hostile NPC carries the original LDtk display name so the
        // renderer can keep that NPC's sheet (with its authored slash
        // / hit rows). Only the Kernel Guide migration leaves the
        // override blank, so kernel→goblin keeps its dedicated visual
        // gag while every other faction NPC stays themselves when
        // hostile.
        //
        // Fallback for direct EnemySpawn entities (no NPC migration
        // history): try the enemy's display name against the same
        // NPC sprite registry. Intro raiders resolve to their
        // placeholder sheet this way without authors having to
        // duplicate the registry entry on an
        // enemy-side table.
        let override_name = ambition_sandbox::features::ecs_enemy_sprite_override(&visual.id, &ecs_actors);
        let enemy_name = ambition_sandbox::features::ecs_enemy_name(&visual.id, &ecs_actors);
        // Resolve a *named* sprite first (override label, then the enemy's own
        // name), then fall back to the generic kind sheet.
        let named = override_name
            .as_deref()
            .and_then(|n| assets.characters.npc_asset_for_name(n))
            .or_else(|| {
                enemy_name
                    .as_deref()
                    .and_then(|n| assets.characters.npc_asset_for_name(n))
            });
        let character_asset = match named {
            Some(asset) => Some(asset),
            None => {
                // Falling back to the generic kind sheet is intended for nameless /
                // truly-generic enemies, but a *named* actor that lands here almost
                // always means its `display_name` doesn't match the character
                // catalog — a content/code bug (e.g. a decorated variant like
                // "Puppy Slug (ally)" instead of the catalog "Puppy Slug"), which
                // used to render the goblin default silently. Surface it once per
                // name (a warning, not a panic — a genuinely missing/late asset
                // file is handled gracefully by the `images.get(..).is_none()`
                // guard below, so the game still runs).
                if let Some(missed) = override_name.as_deref().or(enemy_name.as_deref()) {
                    if warned_sprite_names.insert(missed.to_string()) {
                        bevy::log::warn!(
                            target: "ambition::sprites",
                            "actor '{missed}' resolved no registered sprite — using the {:?} \
                             default sheet. If it should have its own sprite, its display_name \
                             doesn't match the character catalog (likely a typo / decorated name).",
                            view.kind,
                        );
                    }
                }
                assets.characters.enemy_asset(view.kind)
            }
        };
        let Some(character_asset) = character_asset else {
            continue;
        };
        // Android loads assets out of the APK asynchronously, and missing or
        // platform-rejected images still have a Handle. Do not replace the
        // colored fallback with an atlas sprite until the texture is actually
        // present in Assets<Image>; otherwise a failed or delayed load renders
        // the NPC/enemy invisible.
        if images.get(&character_asset.texture).is_none() {
            continue;
        }
        let sprite = build_character_sprite(character_asset, collision);
        commands.entity(entity).insert((
            sprite,
            feet_anchor_for(&character_asset.spec, collision),
            CharacterAnimator::new(&character_asset.spec),
            BoundFeatureKind::new(view.kind, collision),
        ));
    }
}

/// Replace the static `EntitySprite::NpcTerminal` placeholder with a
/// faction-specific spritesheet once the asset is loaded. Today the
/// dispatch is keyed off the NPC's authored name (see
/// `CharacterSpriteAssets::npc_asset_for_name`); when LDtk grows a
/// `category` field on `NpcSpawn`, switch this to lookup-by-category
/// so the dispatch survives display-name edits.
///
/// NPCs without a registered sprite (the common case for the existing
/// hub guides etc.) keep the default terminal placeholder — symmetric
/// with `enemy_asset` returning `None` for non-enemy kinds.
pub fn upgrade_npc_sprites(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    images: Res<Assets<Image>>,
    feature_views: Res<FeatureViewIndex>,
    features: Query<(Entity, &FeatureVisual, Option<&BoundFeatureKind>)>,
    ecs_actors: Query<ambition_sandbox::features::ActorSpriteData>,
) {
    let Some(assets) = assets else {
        return;
    };
    for (entity, visual, bound) in &features {
        let Some(view) = feature_views.get(&visual.id) else {
            continue;
        };
        if !matches!(view.kind, FeatureVisualKind::Npc) {
            continue;
        }
        let collision = BVec2::new(view.size.x, view.size.y);
        if bound.is_some_and(|b| b.matches(view.kind, view.size)) {
            continue;
        }
        let Some(name) = ambition_sandbox::features::ecs_npc_name(&visual.id, &ecs_actors) else {
            continue;
        };
        let Some(character_asset) = assets.characters.npc_asset_for_name(&name) else {
            continue;
        };
        // Keep the visible terminal/rectangle fallback until the PNG has
        // actually loaded. This is especially important on Android, where the
        // asset exists inside the APK but individual textures can still fail
        // or arrive later.
        if images.get(&character_asset.texture).is_none() {
            continue;
        }
        let sprite = build_character_sprite(character_asset, collision);
        commands.entity(entity).insert((
            sprite,
            feet_anchor_for(&character_asset.spec, collision),
            CharacterAnimator::new(&character_asset.spec),
            BoundFeatureKind::new(view.kind, collision),
        ));
    }
}

/// Drive the player sprite's animation state, atlas index, and facing flip.
/// Runs every frame; no-op on color-rectangle fallbacks (no `CharacterAnimator`).
///
/// Query items are split into nested tuples because Bevy 0.18's `Query`
/// tuple impl caps at 15 entries and the picker now reads three more
/// clusters (body_mode, env_contact, abilities) to cover crouch /
/// crawl / slide / ladder / swim.
pub fn animate_player(
    world_time: Res<ambition_sandbox::WorldTime>,
    primary_attack: Query<&ambition_sandbox::player::ActivePlayerAttack, ambition_sandbox::player::PrimaryPlayerOnly>,
    entities: Res<SceneEntities>,
    gravity: Option<Res<ambition_sandbox::physics::GravityField>>,
    mut query: Query<
        (
            (
                &mut Sprite,
                &mut CharacterAnimator,
                &ambition_sandbox::player::BodyKinematics,
                &ambition_sandbox::player::PlayerGroundState,
                &ambition_sandbox::player::PlayerWallState,
                &ambition_sandbox::player::PlayerBlinkState,
                &ambition_sandbox::player::PlayerFlightState,
                &ambition_sandbox::player::PlayerDashState,
                &ambition_sandbox::player::PlayerLedgeState,
                &ambition_sandbox::player::PlayerCombatState,
                &ambition_sandbox::player::PlayerAnimState,
                &ambition_sandbox::player::PlayerBlinkCameraState,
            ),
            (
                &ambition_sandbox::player::PlayerBodyModeState,
                &ambition_sandbox::player::PlayerEnvironmentContact,
                &ambition_sandbox::player::PlayerAbilities,
                &ambition_sandbox::player::PlayerDodgeState,
                &ambition_sandbox::player::PlayerShieldState,
                Option<&ambition_sandbox::time::time_control::ProperTimeScale>,
            ),
        ),
        With<PlayerVisual>,
    >,
) {
    let Ok((
        (
            mut sprite,
            mut animator,
            kinematics,
            ground,
            wall,
            blink,
            flight,
            dash,
            ledge,
            player_combat,
            anim_state,
            blink_cam,
        ),
        (body_mode, env_contact, abilities, dodge, shield, scale),
    )) = query.get_mut(entities.player)
    else {
        return;
    };
    let attack_state = primary_attack.iter().next().and_then(|a| a.0.as_ref());
    let anim = ambition_sandbox::character_sprites::pick_player_anim(
        anim_state,
        player_combat,
        blink_cam,
        attack_state,
        kinematics,
        ground,
        wall,
        blink,
        flight,
        dash,
        ledge,
        body_mode,
        env_contact,
        abilities,
        dodge,
        shield,
    );
    animator.request(anim);
    // ADR 0011 — `entity_dt` collapses to `sim_dt` when no
    // ProperTimeScale is set (SP default), so bullet-time /
    // hitstop / pause still slow the animation in lockstep. Step 4
    // wires the player ProperTimeScale path so future MP regimes
    // can boost the player's cognitive rate without slowing the
    // world for other observers.
    let index = animator.tick(world_time.entity_dt(
        ambition_sandbox::time::time_control::ProperTimeScale::or_default(scale),
    ));
    if let Some(atlas) = sprite.texture_atlas.as_mut() {
        atlas.index = index;
    }
    // Gravity-aware facing flip: a ~180° up-gravity roll already mirrors the
    // sprite, so the flip inverts (fixes #33 "move left, face right upside down").
    let player_gravity = gravity
        .as_deref()
        .map_or(ambition_sandbox::engine_core::Vec2::Y, |g| g.dir);
    sprite.flip_x = ambition_sandbox::physics::gravity_aware_flip_x(kinematics.facing, player_gravity);
    // Hit feedback is drawn by the white-silhouette overlay in
    // `presentation::rendering::hit_flash` — a sibling mesh that
    // samples this atlas frame and outputs pure white modulated by
    // `PlayerCombatState::flash_timer`. The source sprite stays at
    // full opacity / untinted; the overlay does the flashing.
    sprite.color = Color::WHITE;
}

/// Drive enemy AND NPC sprite animation, atlas index, and facing flip.
///
/// Enemies and NPCs both render through `CharacterAnimator`; their
/// per-frame state is owned by separate runtime lists, but a feature
/// id only ever appears in one of them at a time. We try the enemy
/// lookup first (most entities in the room) and fall through to the
/// NPC lookup, so a stationary General sheet ticks its 8 idle frames
/// once the animator is attached.
///
/// One system instead of two avoids the borrow conflict on the
/// shared `(&mut Sprite, &mut CharacterAnimator)` query.
pub fn animate_characters(
    world_time: Res<ambition_sandbox::WorldTime>,
    mut query: Query<
        (
            &FeatureVisual,
            &mut Sprite,
            &mut CharacterAnimator,
            Option<&ambition_sandbox::time::time_control::ProperTimeScale>,
        ),
        (
            Without<PlayerVisual>,
            Without<ambition_sandbox::rooms::PortalSprite>,
            Without<PropVisual>,
        ),
    >,
    ecs_actors: Query<ambition_sandbox::features::ActorSpriteData>,
    // Localized gravity, so an enemy/NPC wall-walking or on a flipped-gravity
    // ceiling flips the right way (the same gravity-aware facing the player got).
    gravity: ambition_sandbox::physics::GravityCtx,
) {
    // ADR 0011 — per-entity proper time. SP today: no entity carries
    // ProperTimeScale, so `entity_dt` collapses to `sim_dt` and
    // every actor ticks at the world rate. The seam matters once a
    // boss freezes the world but leaves the player un-frozen, or
    // future MP boosts one player's proper time.
    for (visual, mut sprite, mut animator, scale) in &mut query {
        let dt = world_time.entity_dt(ambition_sandbox::time::time_control::ProperTimeScale::or_default(
            scale,
        ));
        let (anim, facing, pos, hit_flash, attacking) = if let Some(state) =
            ambition_sandbox::features::ecs_enemy_anim_state(&visual.id, &ecs_actors)
        {
            (
                ambition_sandbox::character_sprites::pick_enemy_anim(state),
                state.facing,
                state.pos,
                state.hit_flash,
                state.attack_active || state.attack_windup,
            )
        } else if let Some(state) = ambition_sandbox::features::ecs_npc_anim_state(&visual.id, &ecs_actors) {
            (
                ambition_sandbox::character_sprites::pick_npc_anim(state),
                state.facing,
                state.pos,
                state.hit_flash,
                false,
            )
        } else {
            continue;
        };
        animator.request(anim);
        let index = animator.tick(dt);
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = index;
        }
        sprite.flip_x = ambition_sandbox::physics::gravity_aware_flip_x(facing, gravity.dir_at(pos));
        // Hit feedback is drawn by the white-silhouette overlay in
        // `presentation::rendering::hit_flash`. Keep the warm
        // attack tint on the multiplicative `sprite.color` channel
        // — it's a separate signal (telegraphing the actor's
        // outgoing swing, not its own damage). The hit_flash
        // boolean still feeds into anim selection upstream so the
        // `hit` row plays.
        sprite.color = if attacking {
            Color::srgba(1.0, 0.85, 0.55, 1.0)
        } else {
            Color::WHITE
        };
        let _ = hit_flash;
    }
}

/// Prop kinds whose authored "Idle" row depicts motion (e.g. rolling
/// wheels). These props stay pinned at frame 0 in [`animate_props`]
/// until a `PropMotionState` component lands to gate their tick by
/// real motion. Add a kind here when its sprite's idle frame reads
/// as "this prop is moving" — the cart is the v1 case.
pub const PROP_KINDS_STATIC_UNTIL_MOVING: &[&str] = &["intro_cart"];

/// Tick the idle animation row for every `PropVisual` sprite that
/// owns a `CharacterAnimator`. Props have no ECS actor entity, so
/// the regular `animate_characters` lookup would skip them — without
/// this system the sprite stays pinned to frame 0 forever.
///
/// Filtered with `Without<ambition_sandbox::rooms::PortalSprite>` so the gate
/// ring + gate portal stay owned by the portal-presentation systems
/// (which drive the animator from `GatePortalPhase` instead of a flat
/// Idle row tick).
///
/// Motion-gated props: a kind listed in [`PROP_KINDS_STATIC_UNTIL_MOVING`]
/// stays pinned at frame 0. The intro cart's authored "idle" row is a
/// wheel-rolling cycle that reads as "the cart is moving"; without a
/// real motion source today (no scripted push), looping it makes the
/// cart look like it's drifting in place. Until a `PropMotionState`
/// component lands, hold these kinds at rest.
pub fn animate_props(
    world_time: Res<ambition_sandbox::WorldTime>,
    mut query: Query<
        (
            &mut Sprite,
            &mut CharacterAnimator,
            &PropVisual,
            Option<&ambition_sandbox::time::time_control::ProperTimeScale>,
        ),
        Without<ambition_sandbox::rooms::PortalSprite>,
    >,
) {
    // ADR 0011 — per-entity proper time. Props that need to keep
    // ticking when the world freezes (a clock prop in a frozen
    // boss arena, say) get a non-1.0 ProperTimeScale.
    for (mut sprite, mut animator, prop, scale) in &mut query {
        if PROP_KINDS_STATIC_UNTIL_MOVING.contains(&prop.kind.as_str()) {
            // Force-rest at frame 0 of the Idle row. `request` selects
            // the row; ticking with dt=0 holds the row's current frame
            // and matches the asset's first frame on entry.
            animator.request(ambition_sandbox::character_sprites::CharacterAnim::Idle);
            let index = animator.tick(0.0);
            if let Some(atlas) = sprite.texture_atlas.as_mut() {
                atlas.index = index;
            }
            continue;
        }
        let dt = world_time.entity_dt(ambition_sandbox::time::time_control::ProperTimeScale::or_default(
            scale,
        ));
        animator.request(ambition_sandbox::character_sprites::CharacterAnim::Idle);
        let index = animator.tick(dt);
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = index;
        }
    }
}

/// When `DeveloperTools::hide_sprites` is enabled, force every `Sprite`-bearing
/// entity to `Hidden` so only gizmo hitbox outlines remain visible. When the
/// flag flips off, restore every sprite to `Inherited` *exactly once* on the
/// falling edge — we deliberately do NOT keep stomping `Inherited` every
/// frame because that wipes out legitimate `Visibility::Hidden` writes from
/// upstream systems (collected pickups, idle morph-ball sphere, player while
/// in morph-ball mode, etc.) and makes them flicker back to visible.
/// UI uses `Node`/`ImageNode`, not `Sprite`, so HUD/menus are unaffected.
pub fn apply_hide_sprites_override(
    developer_tools: Res<ambition_sandbox::dev::dev_tools::DeveloperTools>,
    mut prev_active: Local<bool>,
    mut sprites: Query<&mut Visibility, With<Sprite>>,
) {
    let active = effective_hide_sprites(&developer_tools);
    if active {
        for mut vis in sprites.iter_mut() {
            if *vis != Visibility::Hidden {
                *vis = Visibility::Hidden;
            }
        }
    } else if *prev_active {
        for mut vis in sprites.iter_mut() {
            if *vis != Visibility::Inherited {
                *vis = Visibility::Inherited;
            }
        }
    }
    *prev_active = active;
}

fn effective_hide_sprites(developer_tools: &ambition_sandbox::dev::dev_tools::DeveloperTools) -> bool {
    // Placeholder art is a visible debug-art mode. If an old persisted or
    // inspector-mutated state leaves both booleans true, keep placeholders
    // visible instead of letting hide mode erase them.
    developer_tools.hide_sprites && !developer_tools.placeholder_sprites
}

// =================================================================
// Gradient Sentinel — HazardColumn vertical-column visual
// =================================================================
//
// The new HazardColumn boss attack profile is a tall vertical
// hazard column at the boss x. `volumes_for_profile` already
// returns the right AABB for damage; this system layers a visible
// rectangle so the player can read the column shape during
// telegraph (yellow pulsing) and strike (red solid). Without it
// the player only sees the boss's sprite tint and can't tell where
// the column is in world space.
//
// Pattern: a `GradientLaneVisual` marker component holds the owner
// boss entity. `manage_gradient_lane_visual` spawns one when the
// boss enters HazardColumn telegraph/active and despawns it when
// the boss leaves the profile. Per-frame, it also updates the
// visual's transform + color based on the live state.

/// Marker for the HazardColumn column visual entity. Carries the
/// owner boss entity so the manager system can find / remove the
/// matching visual.
#[derive(Component, Clone, Copy, Debug)]
pub struct GradientLaneVisual {
    pub owner: Entity,
}

const GRADIENT_LANE_TELEGRAPH_COLOR: Color = Color::srgba(1.0, 0.85, 0.20, 0.45);
const GRADIENT_LANE_STRIKE_COLOR: Color = Color::srgba(1.0, 0.32, 0.20, 0.75);
/// Z layer for the lane visual. Sits behind feature sprites
/// (`feature_z(Boss) = 11.0`) but in front of background tiles so
/// the column reads as a foreground hazard.
const GRADIENT_LANE_VISUAL_Z: f32 = 10.5;

/// Spawn/update/despawn a vertical column visual for every boss
/// currently telegraphing or striking `HazardColumn`. The column
/// re-uses the volume AABB computed by `volumes_for_profile` so
/// the visible rectangle always matches the damage geometry.
pub fn manage_gradient_lane_visual(
    mut commands: Commands,
    world: Res<ambition_sandbox::GameWorld>,
    bosses: Query<(Entity, BossClusterRef, &ambition_sandbox::brain::BossAttackState)>,
    mut visuals: Query<(Entity, &GradientLaneVisual, &mut Transform, &mut Sprite)>,
) {
    use ambition_sandbox::brain::BossAttackProfile;
    let mut active: std::collections::HashMap<Entity, (bool, ae::Vec2, BVec2)> =
        std::collections::HashMap::new();
    for (entity, item, attack_state) in &bosses {
        let boss = item.as_boss_ref();
        if !boss.status.alive {
            continue;
        }
        let in_telegraph = matches!(
            attack_state.telegraph_profile,
            Some(BossAttackProfile::HazardColumn)
        );
        let in_strike = matches!(
            attack_state.active_profile,
            Some(BossAttackProfile::HazardColumn)
        );
        if !in_telegraph && !in_strike {
            continue;
        }
        // Use the same volume math as damage so the visual and the
        // hitbox are exactly coincident.
        let mut volumes = ambition_sandbox::features::volumes_for_profile(
            &BossAttackProfile::HazardColumn,
            boss.kin.pos,
            boss.combat_size(),
            &boss.config.behavior,
        );
        let Some(volume) = volumes.pop() else {
            continue;
        };
        let center = volume.center();
        let size = volume.half_size() * 2.0;
        active.insert(entity, (in_strike, center, BVec2::new(size.x, size.y)));
    }

    // Update existing visuals + remove stale ones.
    for (visual_entity, visual, mut transform, mut sprite) in &mut visuals {
        if let Some((in_strike, center, size)) = active.remove(&visual.owner) {
            transform.translation = world_to_bevy(&world.0, center, GRADIENT_LANE_VISUAL_Z);
            sprite.custom_size = Some(size);
            sprite.color = if in_strike {
                GRADIENT_LANE_STRIKE_COLOR
            } else {
                GRADIENT_LANE_TELEGRAPH_COLOR
            };
        } else {
            // Owner stopped telegraphing/striking HazardColumn — despawn.
            commands.entity(visual_entity).despawn();
        }
    }

    // Spawn visuals for bosses that newly entered HazardColumn.
    for (owner, (in_strike, center, size)) in active {
        let color = if in_strike {
            GRADIENT_LANE_STRIKE_COLOR
        } else {
            GRADIENT_LANE_TELEGRAPH_COLOR
        };
        commands.spawn((
            Sprite {
                color,
                custom_size: Some(size),
                ..default()
            },
            Transform::from_translation(world_to_bevy(&world.0, center, GRADIENT_LANE_VISUAL_Z)),
            super::primitives::RoomVisual,
            GradientLaneVisual { owner },
            Name::new("Gradient Lane visual"),
        ));
    }
}

/// Cached pre-placeholder sprite state so toggling `placeholder_sprites`
/// off can restore the textured rendering. Stored per-entity the first
/// time we collapse the sprite to a colored rectangle.
#[derive(Component, Clone)]
pub struct SpriteOriginalState {
    pub image: Handle<Image>,
    pub atlas: Option<bevy::image::TextureAtlas>,
    pub color: Color,
    pub custom_size: Option<BVec2>,
    pub image_mode: bevy::sprite::SpriteImageMode,
}

/// When `DeveloperTools::placeholder_sprites` is enabled, replace every
/// textured sprite with a colored rectangle of the collision/debug size —
/// the "placeholder art era" look. When the flag flips back off, restore
/// the original texture, atlas, tint, sizing, and image mode.
///
/// The placeholder color is derived from a per-entity discriminator
/// (`FeatureVisual` / `PlayerVisual` / boss / projectile markers) so
/// similar entities visually group. Anything without a known marker
/// falls back to the existing sprite color (kept as-is).
pub fn apply_placeholder_sprites_override(
    mut commands: Commands,
    developer_tools: Res<ambition_sandbox::dev::dev_tools::DeveloperTools>,
    feature_views: Res<FeatureViewIndex>,
    mut sprites: Query<(
        Entity,
        &mut Sprite,
        Option<&SpriteOriginalState>,
        Option<&FeatureVisual>,
        Option<&PlayerVisual>,
        Option<&ambition_sandbox::player::BodyKinematics>,
        Option<&ambition_sandbox::projectile::PlayerProjectileVisual>,
        Option<&ambition_sandbox::enemy_projectile::EnemyProjectileVisual>,
    )>,
) {
    if developer_tools.placeholder_sprites {
        for (entity, mut sprite, original, feature, player, player_body, p_proj, e_proj) in
            &mut sprites
        {
            // Record original state once so we can restore on toggle-off.
            if original.is_none() {
                commands.entity(entity).insert(SpriteOriginalState {
                    image: sprite.image.clone(),
                    atlas: sprite.texture_atlas.clone(),
                    color: sprite.color,
                    custom_size: sprite.custom_size,
                    image_mode: sprite.image_mode.clone(),
                });
            }
            let feature_view = feature.and_then(|fv| feature_views.get(&fv.id));
            let placeholder_color = pick_placeholder_color(
                feature_view.map(|v| v.kind),
                player.is_some(),
                p_proj.is_some(),
                e_proj.is_some(),
            );
            // Drop the texture and atlas so the sprite renders as a flat
            // rectangle. Size feature placeholders to their gameplay AABB
            // rather than their authored render bounds so placeholder mode
            // doubles as a collision-readability mode.
            if sprite.image != Handle::default() {
                sprite.image = Handle::default();
            }
            if sprite.texture_atlas.is_some() {
                sprite.texture_atlas = None;
            }
            sprite.image_mode = bevy::sprite::SpriteImageMode::Auto;
            if let Some(view) = feature_view {
                sprite.custom_size = Some(BVec2::new(view.size.x, view.size.y));
            } else if let Some(body) = player_body {
                sprite.custom_size = Some(BVec2::new(body.size.x, body.size.y));
            }
            sprite.color = placeholder_color;
        }
    } else {
        // Restore any cached originals.
        for (entity, mut sprite, original, _, _, _, _, _) in &mut sprites {
            if let Some(orig) = original {
                if sprite.image != orig.image {
                    sprite.image = orig.image.clone();
                }
                if sprite.texture_atlas != orig.atlas {
                    sprite.texture_atlas = orig.atlas.clone();
                }
                sprite.color = orig.color;
                sprite.custom_size = orig.custom_size;
                sprite.image_mode = orig.image_mode.clone();
                commands.entity(entity).remove::<SpriteOriginalState>();
            }
        }
    }
}

fn pick_placeholder_color(
    feature_kind: Option<FeatureVisualKind>,
    is_player: bool,
    is_player_projectile: bool,
    is_enemy_projectile: bool,
) -> Color {
    if is_player {
        return Color::srgba(0.55, 0.85, 1.00, 1.0);
    }
    if is_player_projectile {
        return Color::srgba(1.00, 0.74, 0.30, 1.0);
    }
    if is_enemy_projectile {
        return Color::srgba(1.00, 0.32, 0.32, 1.0);
    }
    match feature_kind {
        Some(kind) => feature_color(kind, false),
        None => Color::srgba(0.70, 0.70, 0.72, 1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::effective_hide_sprites;
    use ambition_sandbox::dev::dev_tools::{DebugArtMode, DeveloperTools};

    #[test]
    fn placeholder_art_wins_over_stale_hide_flag() {
        let mut tools = DeveloperTools::default();
        tools.apply_debug_art_mode(DebugArtMode::Hidden);
        assert!(effective_hide_sprites(&tools));

        tools.hide_sprites = true;
        tools.placeholder_sprites = true;
        assert!(!effective_hide_sprites(&tools));
    }
}

mod boss;
pub use boss::*;
