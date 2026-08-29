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
    // A DIAGNOSTIC placeholder does not count as a claim.
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

/// Publish the set of feature views not yet claimed by a renderer and draw a
/// marked fallback rectangle for them. Family-specific renderers run first. A
/// fallback is not itself a claim and is retired as soon as a real visual
/// appears. Publishing remains valid without a session world; only fallback
/// drawing requires world geometry.
pub fn draw_unclaimed_feature_views(
    mut commands: Commands,
    //  `Option`, and that is load-bearing. `SessionWorldRef` is a `Single`,
    // so an app with no session world SKIPS the whole system — including the
    // census below, which the cover then reads a frame (or a hundred) stale.
    // Publishing is unconditional; only the DRAWING needs a world to place a
    // rectangle in.
    world: Option<
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
            ambition_platformer2d_core::RoomGeometry,
        >,
    >,
    active_session: Option<Res<ActiveSessionScope>>,
    views: Res<ambition_sim_view::FeatureViewIndex>,
    existing: Query<(Entity, &FeatureVisual, Has<UnclaimedBodyPlaceholder>)>,
    mut unsettled: ResMut<UnclaimedFeatureViews>,
    // How many consecutive frames each id has been unclaimed. The stand-in's
    // grace period; see `UNCLAIMED_STAND_IN_GRACE_FRAMES`.
    mut unclaimed_streak: Local<std::collections::HashMap<String, u32>>,
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

    // ── The CENSUS: the cover's question, answered every frame this runs ─────
    //
    //  `stand_ins` is deliberately NOT subtracted. A stand-in is not art; a view wearing
    // one is still a view nothing drew.
    let mut unclaimed_now: Vec<(&str, &ambition_sim_view::FeatureView)> = views
        .iter()
        .filter(|(id, view)| {
            // Zero-sized views are read-models for things with no body (a
            // trigger volume's state). Nothing is waiting to draw them and a
            // rectangle of no size is not a diagnosis.
            !known.contains(id) && view.size.x > 0.0 && view.size.y > 0.0
        })
        .collect();
    // `FeatureViewIndex::iter` is hash-ordered. Presentation-only, so the order
    // cannot enter a trajectory — but a REPORT that names these ids should read
    // the same twice, and so should the spawn order below.
    unclaimed_now.sort_unstable_by(|(left, _), (right, _)| left.cmp(right));
    unsettled.ids.clear();
    unsettled
        .ids
        .extend(unclaimed_now.iter().map(|(id, _)| id.to_string()));

    // An id that is claimed (or gone) starts its grace over. Without this the
    // map grows one entry per feature per room, forever.
    unclaimed_streak.retain(|id, _| unsettled.ids.binary_search(id).is_ok());

    let Some(world) = world else {
        return;
    };
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    let already_standing: std::collections::HashSet<&str> =
        stand_ins.iter().map(|(_, id)| *id).collect();
    for (id, view) in unclaimed_now {
        if already_standing.contains(id) {
            continue;
        }
        // ── The DIAGNOSIS, and it is allowed to be late ──────────────────────
        //
        // Standing in immediately made this instrument 100% false positives on the happy path —
        // 190 warnings against 0 cover give-ups in one healthy 290 s session — and put a
        // magenta box on screen for anything the cover was not over.
        //
        // `a_permanently_unclaimed_view_still_gets_its_stand_in` is the poison that keeps it
        // that way.
        let streak = unclaimed_streak.entry(id.to_string()).or_insert(0);
        *streak = streak.saturating_add(1);
        if *streak < UNCLAIMED_STAND_IN_GRACE_FRAMES {
            continue;
        }
        bevy::log::warn!(
            target: "ambition_platformer2d::render",
            "no render family claimed `{id}` ({:?}) for {} consecutive frames; \
             drawing the unclaimed-body placeholder. Some spawn path is missing \
             its family marker.",
            view.kind,
            *streak,
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

/// Presentation is dormant, so nothing is waiting on it.
///
/// So the dormant answer is published as explicitly as the live one. Same
/// statement, inverse condition, registered beside its twin.
pub fn forget_unclaimed_feature_views_while_dormant(mut unsettled: ResMut<UnclaimedFeatureViews>) {
    if !unsettled.ids.is_empty() {
        unsettled.ids.clear();
    }
}

/// This visual is the FLOOR's stand-in, not a render family's picture of the
/// feature. See [`draw_unclaimed_feature_views`].
#[derive(Component)]
pub struct UnclaimedBodyPlaceholder;

/// Which published feature views nothing has drawn.
///
/// Republished every frame by [`draw_unclaimed_feature_views`]: every
/// [`ambition_sim_view::FeatureViewIndex`] row with a body (`size > 0`) that no
/// render family has claimed with a real [`FeatureVisual`].
///
/// ##  This is NOT "how many magenta boxes are on screen"
///
/// | role | question | wants to fire |
/// |---|---|---|
/// | the stand-in ([`UnclaimedBodyPlaceholder`]) | *did somebody forget a family marker?* | late — only once a view has stayed unclaimed long enough to be a real orphan |
/// | this resource | *is the new room finished drawing?* | immediately — the instant a view is unclaimed, so the cover keeps waiting |
///
///  a view wearing a stand-in is still counted here. A stand-in is not art.
/// That subtraction is the one place the two counts differed.
///
/// ORDERING. A reader must be ordered AFTER the publisher in the SAME schedule.
#[derive(Resource, Default, Debug)]
pub struct UnclaimedFeatureViews {
    /// Sorted, so a report that names them reads the same twice.
    ids: Vec<String>,
}

impl UnclaimedFeatureViews {
    /// How many published views are still undrawn.
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Is every published view drawn by some render family?
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// The undrawn ids, sorted — for a report that has to say WHICH.
    pub fn ids(&self) -> impl ExactSizeIterator<Item = &str> {
        self.ids.iter().map(String::as_str)
    }
}

/// Consecutive frames a view must stay unclaimed before the floor draws its
/// magenta stand-in and warns about it.
///
/// Small on purpose.
///
/// What a measurement would change: a long tail (tens of frames) means raising this, and costs
/// nothing now that the cover no longer depends on it.
const UNCLAIMED_STAND_IN_GRACE_FRAMES: u32 = 5;

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
        //  not optional, and its absence is silent. `ResMut<..>` of a
        // missing resource fails param validation, which SKIPS the system — so a
        // fixture that forgot this would exercise nothing and pass.
        app.init_resource::<UnclaimedFeatureViews>();
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
            submerged: false,
            flash: false,
            breakable_state: None,
            chest_opened: false,
            fighting: false,
            switch_on: false,
            rotation_rad: 0.0,
            alive: true,
            hit_flash_secs: 0.0,
            parry_flash_secs: 0.0,
            hp_current: 1,
            hp_max: 1,
            training_dummy: false,
            hit_strength: 0.0,
            unhittable: false,
            defense_cues: ambition_sim_view::DefenseCueCauses::NONE,
            sprite_offset: None,
        }
    }

    /// A diagnosis must not outlive the bug it diagnosed.
    ///
    /// The floor spawns a `FeatureVisual`, and the family spawner skips any id that already has
    /// one — so a stand-in drawn on a frame where a family was not yet ready would make that
    /// family unreachable for the rest of the feature's life.
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

    /// An authored feature's placeholder must retire when its real visual arrives;
    /// otherwise the placeholder keeps the room-transition cover active.
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

    /// The poison, and it is the whole limit on the grace period. The floor must still DRAW
    /// a stand-in for a view nothing will ever claim.
    #[test]
    fn a_permanently_unclaimed_view_still_gets_its_stand_in() {
        let mut app = app_with_a_room();
        app.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([(
            "nobody_drew_me".to_string(),
            a_view(),
        )]));

        app.add_systems(Update, draw_unclaimed_feature_views);
        for _ in 0..UNCLAIMED_STAND_IN_GRACE_FRAMES {
            app.update();
        }

        let world = app.world_mut();
        let mut placeholders = world.query::<(&FeatureVisual, &UnclaimedBodyPlaceholder)>();
        assert_eq!(
            placeholders
                .iter(world)
                .filter(|(v, _)| v.id == "nobody_drew_me")
                .count(),
            1,
            "a view no family will ever claim is still a bug, and still gets its \
             marked box. The grace period is a delay on the DIAGNOSIS, never an \
             amnesty for it."
        );
    }

    ///  THE SPLIT, stated in one assertion.
    ///
    /// On the very first frame a view is unclaimed, the cover must already know
    /// the room is not drawn — and no magenta box may exist yet, because a
    /// one-flush ordering gap is not a diagnosis.
    #[test]
    fn a_view_is_censused_as_unsettled_before_any_stand_in_is_drawn() {
        let mut app = app_with_a_room();
        app.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([(
            "not_drawn_yet".to_string(),
            a_view(),
        )]));

        app.add_systems(Update, draw_unclaimed_feature_views);
        app.update();

        assert_eq!(
            app.world()
                .resource::<UnclaimedFeatureViews>()
                .ids()
                .collect::<Vec<_>>(),
            vec!["not_drawn_yet"],
            "the census must report an undrawn view on the FIRST frame it is \
             undrawn. This is what the room-transition cover waits on; anything \
             later and it retires over art that has not arrived."
        );

        let world = app.world_mut();
        let mut placeholders = world.query_filtered::<(), With<UnclaimedBodyPlaceholder>>();
        assert_eq!(
            placeholders.iter(world).count(),
            0,
            "…and NO magenta box yet. One frame unclaimed is a `Commands` flush, \
             not a missing family marker — 190 warnings against 0 cover give-ups \
             in one healthy session is what drawing it immediately produced."
        );
    }

    /// A view published now and claimed two flushes later leaves the room
    /// unsettled on the frames in between.
    ///
    /// The guard the conflation makes impossible: once the stand-in has a grace
    /// period, a placeholder-based count cannot express this at all — it reads
    /// zero on frames 1 and 2 and would retire the cover into the gap.
    #[test]
    fn a_view_claimed_two_flushes_later_is_unsettled_until_it_is_drawn() {
        let mut app = app_with_a_room();
        app.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([(
            "late_family".to_string(),
            a_view(),
        )]));
        app.add_systems(Update, draw_unclaimed_feature_views);

        let mut census: Vec<usize> = Vec::new();
        app.update();
        census.push(app.world().resource::<UnclaimedFeatureViews>().len());
        app.update();
        census.push(app.world().resource::<UnclaimedFeatureViews>().len());

        // The family finally draws it.
        app.world_mut().spawn((
            FeatureVisual {
                id: "late_family".into(),
            },
            RoomVisual,
        ));
        app.update();
        census.push(app.world().resource::<UnclaimedFeatureViews>().len());

        assert_eq!(
            census,
            vec![1, 1, 0],
            "the census must stay non-zero for every frame the view is undrawn \
             and drop to zero on the frame it is drawn. A zero on frame 1 or 2 is \
             a cover retiring into the gap it exists to hide."
        );
    }

    /// A stand-in is not art. A view wearing one is still a view no render
    /// family drew, so it must still hold the cover. Subtracting the stand-ins —
    /// which the draw loop legitimately does, so it does not spawn a second box
    /// — is exactly the mistake that made a magenta rectangle look like a
    /// finished room.
    #[test]
    fn a_view_wearing_a_stand_in_is_still_unsettled() {
        let mut app = app_with_a_room();
        app.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([(
            "wearing_magenta".to_string(),
            a_view(),
        )]));
        // The floor already stood in for it on an earlier frame.
        app.world_mut().spawn((
            FeatureVisual {
                id: "wearing_magenta".into(),
            },
            DynamicFeatureVisual,
            UnclaimedBodyPlaceholder,
        ));

        app.add_systems(Update, draw_unclaimed_feature_views);
        app.update();

        assert_eq!(
            app.world()
                .resource::<UnclaimedFeatureViews>()
                .ids()
                .collect::<Vec<_>>(),
            vec!["wearing_magenta"],
            "a view standing under the diagnostic rectangle is UNDRAWN. If this \
             ever reports empty, the room-transition cover retires over a magenta \
             box — which is the flash, exactly."
        );

        let world = app.world_mut();
        let mut placeholders = world.query_filtered::<(), With<UnclaimedBodyPlaceholder>>();
        assert_eq!(
            placeholders.iter(world).count(),
            1,
            "…and it must not be given a SECOND box. The draw loop skips ids that \
             already have one; only the census ignores that."
        );
    }

    /// Presentation is dormant, so the census must say so.
    ///
    /// The publisher is session-gated. A `Resource` it stops writing keeps its
    /// last value — unlike the session-scoped entities it replaced, which the
    /// lifecycle sweeps — and a stale non-zero census is a transition cover
    /// holding a black screen to its full give-up deadline.
    #[test]
    fn a_dormant_presentation_publishes_an_empty_census() {
        let mut app = App::new();
        app.init_resource::<UnclaimedFeatureViews>();
        // What the last live frame left behind.
        app.world_mut()
            .resource_mut::<UnclaimedFeatureViews>()
            .ids
            .push("left_over_from_the_last_session".to_string());
        assert_eq!(
            app.world().resource::<UnclaimedFeatureViews>().len(),
            1,
            "the fixture has to actually leave something in the census, or the \
             clear below proves nothing"
        );

        app.add_systems(Update, forget_unclaimed_feature_views_while_dormant);
        app.update();
        assert!(
            app.world().resource::<UnclaimedFeatureViews>().is_empty(),
            "a dormant presentation is not waiting for anything to be drawn"
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
