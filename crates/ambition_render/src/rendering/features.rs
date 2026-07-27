//! Per-frame discovery system that spawns Bevy `FeatureVisual` entities for
//! dynamically introduced features (encounter mobs, staged duel actors,
//! post-boss NPCs, and reward chests). Static LDtk-derived features are
//! handled by [`super::world::spawn_room_visuals`] at room load.
//!
//! Pure consumer of the sim-built
//! [`ambition_sim_view::DynamicFeatureViews`] rows (E4 slice
//! 9): the sim resolves identity / geometry / placeholder-sprite facts; this
//! pass only spawns the missing visuals.

use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use super::primitives::{feature_color, feature_z, FeatureVisual, RoomVisual};
use ambition_engine_core::config::world_to_bevy;
use ambition_platformer_primitives::lifecycle::{
    ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_sim_view::DynamicFeatureViews;
use ambition_sprite_sheet::game_assets::{entity_sprite_or_color, GameAssets};

/// Spawn `FeatureVisual` entities for dynamically introduced ECS features
/// that don't have one yet. Static LDtk-derived features get their visuals
/// from `spawn_room_visuals` at room load; the sim's `DynamicFeatureViews`
/// carries everything introduced after that point.
///
/// `sync_visuals` reads the matching `FeatureView` and
/// `upgrade_actor_sprites` swaps in the character spritesheet on the
/// same frame; chests pick up their sprite via `state_aware_entity_sprite`.
pub fn spawn_dynamic_feature_visuals(
    mut commands: Commands,
    world: ambition_platformer_primitives::lifecycle::SessionWorldRef<
        ambition_engine_core::RoomGeometry,
    >,
    assets: Option<Res<GameAssets>>,
    active_session: Option<Res<ActiveSessionScope>>,
    existing: Query<&FeatureVisual>,
    dynamic: Res<DynamicFeatureViews>,
) {
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    let known: std::collections::HashSet<&str> = existing.iter().map(|v| v.id.as_str()).collect();
    let assets_ref = assets.as_deref();
    for fact in &dynamic.0 {
        if known.contains(fact.id.as_str()) {
            continue;
        }
        let render = BVec2::new(fact.size.x, fact.size.y);
        let fallback = feature_color(fact.visual_kind, fact.fighting, false);
        // A drop may name an ANIMATED sheet (a spinning ring): bind it exactly
        // as the room-load pass binds an authored pickup's, so a ring that burst
        // out of the player is the same spinning ring as one lying in the level
        // — not a static coin standing in for it.
        let animated = fact
            .prop_sheet
            .as_deref()
            .and_then(|kind| assets_ref.and_then(|a| a.characters.prop_asset_for_kind(kind)));
        let transform = Transform::from_translation(world_to_bevy(
            &world.0,
            fact.pos,
            feature_z(fact.visual_kind),
        ));
        let name = Name::new(format!("{}: {}", fact.family, fact.label));
        let visual = FeatureVisual {
            id: fact.id.clone(),
        };
        match animated {
            Some(asset) => {
                commands.spawn_session_scoped(
                    session_scope,
                    (
                        ambition_sprite_sheet::character::build_character_sprite(asset, render),
                        // A collectible floats: centre-anchored, like the
                        // authored animated pickup.
                        bevy::sprite::Anchor::CENTER,
                        ambition_sprite_sheet::character::CharacterAnimator::new(asset),
                        transform,
                        name,
                        visual,
                        RoomVisual,
                        DynamicFeatureVisual,
                    ),
                );
            }
            None => {
                let sprite = match assets_ref {
                    Some(a) => entity_sprite_or_color(a, fact.sprite_key, render, fallback),
                    None => Sprite::from_color(fallback, render),
                };
                commands.spawn_session_scoped(
                    session_scope,
                    (
                        sprite,
                        transform,
                        name,
                        visual,
                        RoomVisual,
                        DynamicFeatureVisual,
                    ),
                );
            }
        }
    }
}

/// A visual THIS pass spawned, and therefore this pass is responsible for.
///
/// Room-load visuals live until the room does; a dynamic one outlives its sim
/// entity only as an invisible orphan. The marker keeps the cleanup below
/// strictly symmetric with the spawn above — it can only ever despawn something
/// this module created, so it cannot mistake a static visual for a dead one
/// during a frame when the sim's view index hasn't been built yet.
#[derive(Component)]
pub struct DynamicFeatureVisual;

/// Despawn the visual of a dynamic feature the sim has finished with.
///
/// A dropped ring expires. Without this, its sprite lingers for the life of the
/// ROOM — hidden (a `FeatureVisual` with no view is hidden by `sync_visuals`),
/// but accumulating one entity per drop for as long as the player keeps taking
/// hits.
///
/// GONE means gone from BOTH read-models. Falling out of `DynamicFeatureViews`
/// alone does not mean a feature died: that list is a discovery feed with
/// per-family conditions (a mob that turns peaceful drops out of it while very
/// much still standing there). The feature is dead only when the per-frame
/// `FeatureViewIndex` — which every live feature appears in — has also lost it.
pub fn despawn_dead_dynamic_feature_visuals(
    mut commands: Commands,
    dynamic: Res<DynamicFeatureViews>,
    features: Res<ambition_sim_view::FeatureViewIndex>,
    visuals: Query<(Entity, &FeatureVisual), With<DynamicFeatureVisual>>,
) {
    if visuals.is_empty() {
        return;
    }
    let discovered: std::collections::HashSet<&str> =
        dynamic.0.iter().map(|fact| fact.id.as_str()).collect();
    for (entity, visual) in &visuals {
        let id = visual.id.as_str();
        if !discovered.contains(id) && features.get(id).is_none() {
            commands.entity(entity).despawn();
        }
    }
}

/// **Nothing the sim publishes goes undrawn.** (Jon, 2026-07-27)
///
/// Every render family discovers its own population — the authored room pass
/// takes `spec.enemy_spawns`, the dynamic pass takes `EncounterMob` / staged
/// actors / reward chests — and a body that fits none of them is simply never
/// drawn. Not drawn wrong, not drawn as a placeholder: absent. A versus fighter
/// shipped that way, with a body, a published view, a hurtbox, a moveset and no
/// picture, and the reason the placeholder did not cover it is that a
/// placeholder stands in for art that failed to RESOLVE, and nothing had been
/// asked to resolve any.
///
/// So this is the floor: if the sim published a view for it and no family
/// claimed it, it gets a marked rectangle. Deliberately ugly — it is a
/// diagnosis, not a design, and a fighter that looks like a coloured box is a
/// bug somebody will fix, where a fighter that looks like nothing is a bug
/// somebody has to notice first.
///
/// Runs AFTER every family's own spawn, and skips any id that already has a
/// visual, so it can only ever fill a genuine gap.
pub fn draw_unclaimed_feature_views(
    mut commands: Commands,
    world: ambition_platformer_primitives::lifecycle::SessionWorldRef<
        ambition_engine_core::RoomGeometry,
    >,
    active_session: Option<Res<ActiveSessionScope>>,
    views: Res<ambition_sim_view::FeatureViewIndex>,
    existing: Query<&FeatureVisual>,
) {
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    let known: std::collections::HashSet<&str> = existing.iter().map(|v| v.id.as_str()).collect();
    for (id, view) in views.iter() {
        if known.contains(id) {
            continue;
        }
        // Zero-sized views are read-models for things with no body (a trigger
        // volume's state). A rectangle of no size is not a diagnosis.
        if view.size.x <= 0.0 || view.size.y <= 0.0 {
            continue;
        }
        bevy::log::warn!(
            target: "ambition::render",
            "no render family claimed `{id}` ({:?}); drawing the unclaimed-body \
             placeholder. Some spawn path is missing its family marker.",
            view.kind
        );
        commands.spawn_session_scoped(
            session_scope,
            (
                Sprite::from_color(UNCLAIMED_BODY_COLOR, BVec2::new(view.size.x, view.size.y)),
                Transform::from_translation(world_to_bevy(
                    &world.0,
                    view.pos,
                    feature_z(view.kind),
                )),
                Name::new(format!("UNCLAIMED body placeholder: {id}")),
                FeatureVisual { id: id.to_string() },
                RoomVisual,
                DynamicFeatureVisual,
            ),
        );
    }
}

/// Magenta, because nobody ships magenta on purpose.
const UNCLAIMED_BODY_COLOR: Color = Color::srgba(1.0, 0.0, 0.85, 0.85);
