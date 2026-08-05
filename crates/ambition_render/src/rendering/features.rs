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
use ambition_platformer2d_core::config::world_to_bevy;
use ambition_platformer2d_shared_tangle::lifecycle::{
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
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    assets: Option<Res<GameAssets>>,
    active_session: Option<Res<ActiveSessionScope>>,
    existing: Query<(Entity, &FeatureVisual, Has<UnclaimedBodyPlaceholder>)>,
    dynamic: Res<DynamicFeatureViews>,
) {
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    // A DIAGNOSTIC placeholder does not count as a claim. If the floor drew a
    // magenta rectangle for an id on an earlier frame — because the sim
    // published the view before this family's own preconditions were ready —
    // treating that as "already drawn" would make the diagnosis permanent and
    // the real sprite unreachable for the lifetime of the feature.
    let mut known: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut placeholders: std::collections::HashMap<&str, Entity> =
        std::collections::HashMap::new();
    for (entity, visual, is_placeholder) in &existing {
        if is_placeholder {
            placeholders.insert(visual.id.as_str(), entity);
        } else {
            known.insert(visual.id.as_str());
        }
    }
    let assets_ref = assets.as_deref();
    for fact in &dynamic.0 {
        if known.contains(fact.id.as_str()) {
            continue;
        }
        // The real thing is arriving: retire the stand-in in the same flush, so
        // the frame never shows both.
        if let Some(placeholder) = placeholders.get(fact.id.as_str()) {
            commands.entity(*placeholder).try_despawn();
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
///
/// A stand-in it draws is MARKED ([`UnclaimedBodyPlaceholder`]) and is not a
/// claim on the id: `spawn_dynamic_feature_visuals` ignores it when deciding
/// what still needs drawing and retires it when the real visual arrives. Without
/// that, a placeholder drawn on a frame where a family was not yet ready would
/// lock the feature into magenta forever — the diagnosis outliving the bug it
/// diagnosed (GPT 5.6, 2026-07-27).
///
/// ⛔ **and that retirement covered only HALF the population.**
/// `spawn_dynamic_feature_visuals` walks `DynamicFeatureViews` — the
/// dynamically-discovered features — while stand-ins are spawned here from
/// `FeatureViewIndex`, which holds *every* feature. A stand-in drawn for an
/// AUTHORED brick or coin appeared in the first list and never the second, so
/// neither retirement path could reach it and it lived until the room unloaded.
///
/// That is what put Jon's screen black across a level change (2026-08-05): the
/// room-transition cover holds until no `UnclaimedBodyPlaceholder` remains, and
/// on the frame after a commit the destination room's authored features are
/// already in the view index while `respawn_room_visuals_on_request` — which is
/// in `Update`, with no ordering against this chain's simulation phase — has not
/// drawn them yet. Every authored feature got a permanent stand-in, and the
/// cover sat out its full 8-second give-up deadline. The screen was black
/// because the diagnosis outlived the bug, exactly as the paragraph above warns,
/// for the population that paragraph did not cover.
///
/// So retirement lives HERE now, beside the spawn, keyed on the same list. One
/// system owns both halves of "is this id claimed", which is the only way the
/// two can not disagree.
///
/// ⚠ the ordering hole is real and is NOT fixed here: an `.after()` from a
/// simulation-phase set to an `Update` system is silently vacuous in Bevy, so
/// pointing this chain at the room spawner would look like a fix and do nothing.
/// The cost of the race is now one frame of magenta instead of eight seconds of
/// black, and the schedule question is a ledger row.
pub fn draw_unclaimed_feature_views(
    mut commands: Commands,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    active_session: Option<Res<ActiveSessionScope>>,
    views: Res<ambition_sim_view::FeatureViewIndex>,
    existing: Query<(Entity, &FeatureVisual, Has<UnclaimedBodyPlaceholder>)>,
) {
    // Split the drawn ids into the REAL visuals and this system's own stand-ins,
    // in one pass, because "claimed" and "standing in for a claim" are different
    // answers and the old single `known` set could not tell them apart.
    let mut known: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut stand_ins: Vec<(Entity, &str)> = Vec::new();
    for (entity, visual, is_stand_in) in &existing {
        if is_stand_in {
            stand_ins.push((entity, visual.id.as_str()));
        } else {
            known.insert(visual.id.as_str());
        }
    }
    // The real thing arrived: retire the stand-in. Before the session-scope
    // guard below, because despawning needs no scope to spawn into — and a
    // placeholder that outlives its session's scope resolution is exactly the
    // one that would hold a cover open.
    for (entity, id) in &stand_ins {
        if known.contains(id) {
            commands.entity(*entity).try_despawn();
        }
    }

    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    let already_standing: std::collections::HashSet<&str> =
        stand_ins.iter().map(|(_, id)| *id).collect();
    for (id, view) in views.iter() {
        if known.contains(id) || already_standing.contains(id) {
            continue;
        }
        // Zero-sized views are read-models for things with no body (a trigger
        // volume's state). A rectangle of no size is not a diagnosis.
        if view.size.x <= 0.0 || view.size.y <= 0.0 {
            continue;
        }
        bevy::log::warn!(
            target: "ambition_platformer2d::render",
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
                UnclaimedBodyPlaceholder,
            ),
        );
    }
}

/// This visual is the FLOOR's stand-in, not a render family's picture of the
/// feature. See [`draw_unclaimed_feature_views`].
#[derive(Component)]
pub struct UnclaimedBodyPlaceholder;

/// Magenta, because nobody ships magenta on purpose.
const UNCLAIMED_BODY_COLOR: Color = Color::srgba(1.0, 0.0, 0.85, 0.85);

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::lifecycle::SessionRoot;
    use ambition_sim_view::DynamicFeatureFact;

    fn app_with_a_room() -> App {
        let mut app = App::new();
        app.init_resource::<DynamicFeatureViews>();
        // A real session, so the spawn path takes its scoped arm rather than the
        // unscoped fixture arm — the placeholder swap has to work where it ships.
        let mut active = ActiveSessionScope::default();
        let scope = active.begin();
        app.insert_resource(active);
        app.world_mut().spawn((
            SessionRoot(scope),
            ambition_platformer2d_core::RoomGeometry(ambition_platformer2d_core::World::new(
                "probe",
                ambition_platformer2d_core::Vec2::new(320.0, 180.0),
                ambition_platformer2d_core::Vec2::new(40.0, 40.0),
                Vec::new(),
            )),
        ));
        app
    }

    fn a_fact(id: &str) -> DynamicFeatureFact {
        DynamicFeatureFact {
            id: id.to_string(),
            label: id.to_string(),
            family: "probe",
            pos: ambition_platformer2d_core::Vec2::new(10.0, 10.0),
            size: ambition_platformer2d_core::Vec2::new(16.0, 24.0),
            visual_kind:
                ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind::Actor,
            fighting: false,
            sprite_key: None,
            prop_sheet: None,
        }
    }

    fn a_view() -> ambition_sim_view::FeatureView {
        ambition_sim_view::FeatureView {
            pos: ambition_platformer2d_core::Vec2::new(10.0, 10.0),
            size: ambition_platformer2d_core::Vec2::new(16.0, 24.0),
            kind: ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind::Breakable,
            visible: true,
            flash: false,
            breakable_state: None,
            chest_opened: false,
            fighting: false,
            switch_on: false,
            rotation_rad: 0.0,
            alive: true,
            hit_flash_secs: 0.0,
            hp_current: 1,
            hp_max: 1,
            training_dummy: false,
            sprite_offset: None,
        }
    }

    /// **A diagnosis must not outlive the bug it diagnosed.**
    ///
    /// The floor spawns a `FeatureVisual`, and the family spawner skips any id
    /// that already has one — so a stand-in drawn on a frame where a family was
    /// not yet ready would make that family unreachable for the rest of the
    /// feature's life. The magenta box would then be permanent and would look
    /// exactly like the render bug it exists to report.
    #[test]
    fn the_real_visual_replaces_the_unclaimed_stand_in() {
        let mut app = app_with_a_room();
        // The floor got there first.
        app.world_mut().spawn((
            FeatureVisual {
                id: "late_arrival".into(),
            },
            DynamicFeatureVisual,
            UnclaimedBodyPlaceholder,
        ));
        app.world_mut()
            .resource_mut::<DynamicFeatureViews>()
            .0
            .push(a_fact("late_arrival"));

        app.add_systems(Update, spawn_dynamic_feature_visuals);
        app.update();

        let world = app.world_mut();
        let mut visuals = world.query::<(&FeatureVisual, Has<UnclaimedBodyPlaceholder>)>();
        let rows: Vec<bool> = visuals
            .iter(world)
            .filter(|(v, _)| v.id == "late_arrival")
            .map(|(_, placeholder)| placeholder)
            .collect();
        assert_eq!(
            rows,
            vec![false],
            "expected exactly one visual for the id and for it to be the REAL \
             one; got {rows:?} (true = the magenta stand-in). A stand-in that \
             survives the family's own spawn is a permanent misdiagnosis, and \
             two visuals for one id is a double draw."
        );
    }

    /// **The stand-in for an AUTHORED feature was unreachable by every cleanup
    /// path, and it held the room-transition cover for eight seconds.**
    ///
    /// Jon, 2026-08-05: *"When we transition from world 1-1 to 1-2 or back there
    /// is a long time where there is just a black screen even though music
    /// plays."* The black is the transition cover, and it is retired only once
    /// no `UnclaimedBodyPlaceholder` remains — otherwise it holds to an
    /// 8-second give-up deadline.
    ///
    /// A placeholder is SPAWNED from `FeatureViewIndex`, which holds every
    /// feature. It was RETIRED only through `DynamicFeatureViews`, which holds
    /// the dynamically-discovered ones. An authored coin or brick is in the
    /// first and never the second, so a stand-in drawn for one on the frame
    /// after a room commit — before `respawn_room_visuals_on_request` has run,
    /// which is in `Update` with no ordering against this chain's sim phase —
    /// could not be despawned by anything short of unloading the room.
    ///
    /// `the_real_visual_replaces_the_unclaimed_stand_in` above asserts exactly
    /// this property for the DYNAMIC population and passed throughout.
    #[test]
    fn the_stand_in_for_an_authored_feature_is_retired_when_its_visual_arrives() {
        let mut app = app_with_a_room();
        // An authored feature: in the view index, never in the dynamic feed.
        app.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([(
            "authored_brick".to_string(),
            a_view(),
        )]));
        // The floor drew its stand-in on a frame the room's visuals were not up.
        app.world_mut().spawn((
            FeatureVisual {
                id: "authored_brick".into(),
            },
            DynamicFeatureVisual,
            UnclaimedBodyPlaceholder,
        ));
        // ...and now the room-load spawner has caught up and drawn the real one.
        app.world_mut().spawn((
            FeatureVisual {
                id: "authored_brick".into(),
            },
            RoomVisual,
        ));

        app.add_systems(Update, draw_unclaimed_feature_views);
        app.update();

        let world = app.world_mut();
        let mut placeholders = world.query::<(&FeatureVisual, &UnclaimedBodyPlaceholder)>();
        assert_eq!(
            placeholders
                .iter(world)
                .filter(|(v, _)| v.id == "authored_brick")
                .count(),
            0,
            "the real visual has arrived, so the stand-in must go. While it \
             stays, the room-transition cover holds the screen black to its \
             8-second deadline."
        );
    }

    /// The poison: the floor must still DRAW a stand-in for a view nothing has
    /// claimed. A retirement rule that also stopped it spawning would pass the
    /// test above by deleting the diagnosis instead of resolving it.
    #[test]
    fn a_view_with_no_visual_at_all_still_gets_its_stand_in() {
        let mut app = app_with_a_room();
        app.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([(
            "nobody_drew_me".to_string(),
            a_view(),
        )]));

        app.add_systems(Update, draw_unclaimed_feature_views);
        app.update();

        let world = app.world_mut();
        let mut placeholders = world.query::<(&FeatureVisual, &UnclaimedBodyPlaceholder)>();
        assert_eq!(
            placeholders
                .iter(world)
                .filter(|(v, _)| v.id == "nobody_drew_me")
                .count(),
            1,
            "an unclaimed view is still a bug, and still gets its marked box"
        );
    }

    /// And the ordinary case still holds: a family that has already drawn an id
    /// is not asked to draw it twice.
    #[test]
    fn a_real_visual_is_not_respawned() {
        let mut app = app_with_a_room();
        app.world_mut().spawn((
            FeatureVisual {
                id: "already_drawn".into(),
            },
            DynamicFeatureVisual,
        ));
        app.world_mut()
            .resource_mut::<DynamicFeatureViews>()
            .0
            .push(a_fact("already_drawn"));

        app.add_systems(Update, spawn_dynamic_feature_visuals);
        app.update();

        let world = app.world_mut();
        let mut visuals = world.query::<&FeatureVisual>();
        assert_eq!(
            visuals
                .iter(world)
                .filter(|v| v.id == "already_drawn")
                .count(),
            1
        );
    }
}
