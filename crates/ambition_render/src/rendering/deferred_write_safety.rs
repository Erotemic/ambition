//! Does this pass survive its target being torn down?
//!
//! A presentation pass queues `commands.entity(body).insert(..)`. Between the
//! query that produced `body` and the frame's command flush, another system can
//! despawn it — session teardown on a provider switch, room teardown on a
//! transition, an actor cleanup on death. When that happens Bevy's default error
//! handler PANICS, and the crash names a bundle rather than a lifecycle.
//!
//! This is not hypothetical and it is not rare-but-survivable: it took down the
//! multi-provider acceptance cycle (L23), and it was surfaced by adding a system
//! that spawns nothing to the render chain — because that moved a flush
//! boundary. A hazard a no-op can trip is a hazard.
//!
//! ## Why a harness instead of a rule
//!
//! The obvious response is "use `try_insert` everywhere", and it is wrong.
//!
//! Turning "I reasoned that this is safe" into "I ran it" is the whole point.

use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

/// Run `app` for one frame with every entity matching `Doomed` despawned
/// AFTER `pass` has run but BEFORE the frame's commands flush.
///
/// A pass whose deferred writes do not tolerate a vanished target fails inside
/// Bevy's command error handler, which panics — so callers assert by *not*
/// panicking, and a caller that wants the negative result wraps this in
/// [`std::panic::catch_unwind`].
///
/// `Doomed` is a marker the caller puts on the entities it wants torn down, so
/// a test can aim this at exactly the population a real teardown would take
/// (room-scoped, session-scoped) rather than at everything.
pub fn run_frame_despawning_targets<Doomed: Component, M, P>(
    app: &mut App,
    schedule: impl ScheduleLabel + Clone,
    pass: P,
) where
    P: bevy::ecs::schedule::IntoScheduleConfigs<bevy::ecs::system::ScheduleSystem, M>,
{
    // The teardown is CHAINED BEFORE the pass, and that is the whole fidelity of
    // this harness.
    //
    // Deferred commands apply at the sync point in the order their systems ran.
    // For the pass's `insert` to land on a dead entity, the despawn has to be
    // QUEUED FIRST — while the entity is still live, so the pass's query still
    // yields it. That is exactly the production shape: one system reads an
    // entity another system has already asked to despawn, and the flush honours
    // the despawn first.
    //
    // That is how `upgrade_actor_sprites` passed this harness while holding a plain `insert`.
    // That is the safe case, not the hazard — and it is what made the harness's own meta-test
    // stop failing when the ordering was first corrected. Both command buffers must reach ONE
    // flush.
    app.add_systems(
        schedule,
        (despawn_doomed::<Doomed>, pass).chain_ignore_deferred(),
    );
    app.update();
}

/// Run the same frame with a surviving witness that proves the pass executed.
/// The witness belongs to the same population and names a component the pass must
/// write, preventing an early-out fixture from passing vacuously.
pub fn run_frame_despawning_targets_with_witness<Doomed, Witness, Written, M, P>(
    app: &mut App,
    schedule: impl ScheduleLabel + Clone,
    pass: P,
) where
    Doomed: Component,
    Witness: Component,
    Written: Component,
    P: bevy::ecs::schedule::IntoScheduleConfigs<bevy::ecs::system::ScheduleSystem, M>,
{
    run_frame_despawning_targets::<Doomed, M, P>(app, schedule, pass);
    let world = app.world_mut();
    let mut witnesses = world.query_filtered::<(), (With<Witness>, With<Written>)>();
    assert!(
        witnesses.iter(world).next().is_some(),
        "the pass wrote nothing to the surviving witness, so it took an early-out \
         and never reached the deferred write this probe exists to exercise. The \
         fixture is missing a precondition — a green result here would report a \
         hazard as absent when it was never tried."
    );
}

fn despawn_doomed<Doomed: Component>(mut commands: Commands, doomed: Query<Entity, With<Doomed>>) {
    for entity in &doomed {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Component)]
    struct Doomed;

    #[derive(Component)]
    struct Decoration;

    /// The harness catches an intolerant deferred write.
    ///
    /// A meta-test, and worth the exception: this harness exists to produce
    /// evidence, so a harness that cannot fail would launder guesses into
    /// results. It asserts the shape it is built to detect, using a pass written
    /// to be wrong.
    #[test]
    fn an_intolerant_insert_on_a_torn_down_target_is_caught() {
        fn intolerant(mut commands: Commands, targets: Query<Entity, With<Doomed>>) {
            for entity in &targets {
                commands.entity(entity).insert(Decoration);
            }
        }

        let caught = std::panic::catch_unwind(|| {
            let mut app = App::new();
            app.world_mut().spawn(Doomed);
            run_frame_despawning_targets::<Doomed, _, _>(&mut app, Update, intolerant);
        });
        assert!(
            caught.is_err(),
            "the harness did not notice an `insert` landing on a despawned \
             target, so every result it produces is worthless"
        );
    }

    /// And passes a tolerant one, so the harness is not simply failing at
    /// everything.
    #[test]
    fn a_tolerant_insert_survives_the_same_teardown() {
        fn tolerant(mut commands: Commands, targets: Query<Entity, With<Doomed>>) {
            for entity in &targets {
                commands.entity(entity).try_insert(Decoration);
            }
        }

        let mut app = App::new();
        app.world_mut().spawn(Doomed);
        run_frame_despawning_targets::<Doomed, _, _>(&mut app, Update, tolerant);
    }
}

/// The harness pointed at a REAL pass.
///
/// `apply_placeholder_sprites_override` targets sprite entities, and sprite
/// entities are exactly what `despawn_dead_dynamic_feature_visuals` retires when
/// a feature's view disappears — so its deferred `SpriteOriginalState` write can
/// land on an entity that no longer exists.
///
/// This is the difference between the queue row's "reasoned" and "reproduced":
/// it runs the shipped system against a real teardown rather than arguing about
/// whether one is possible.
#[cfg(test)]
mod production_passes {
    use super::*;
    use crate::rendering::actors::apply_placeholder_sprites_override;

    #[derive(Component)]
    struct Doomed;

    #[derive(Component)]
    struct Witness;

    /// Portal sprite marking targets `PropVisual` entities, which room teardown
    /// despawns with the room.
    #[test]
    fn portal_sprite_marking_survives_its_targets_being_retired() {
        use crate::rendering::gate_portal_visuals::sync_portal_sprite_visibility;
        use crate::rendering::primitives::PropVisual;

        let mut app = App::new();
        // A REGISTERED portal whose sprite name matches the prop below. Without
        // this the pass's outer loop is over an empty map, it never reaches the
        // insert, and the probe passes while proving nothing — which is exactly
        // what it did on the first run.
        let mut registry = ambition_platformer2d_world::rooms::GatePortalRegistry::default();
        registry.register("zone", "switch", "portal", "ring");
        app.insert_resource(registry);
        app.init_resource::<ambition_platformer2d_world::rooms::GatePortalPhases>();
        app.world_mut().spawn((
            PropVisual {
                id: "p".into(),
                kind: "portal".into(),
                // The pass matches on NAME, so this has to be a name it acts on
                // — otherwise the loop skips and the test proves nothing.
                name: "portal".into(),
                size: Vec2::splat(16.0),
                draw: Default::default(),
                flip_y: false,
            },
            Visibility::default(),
            Doomed,
        ));
        app.world_mut().spawn((
            PropVisual {
                id: "w".into(),
                kind: "portal".into(),
                name: "portal".into(),
                size: Vec2::splat(16.0),
                draw: Default::default(),
                flip_y: false,
            },
            Visibility::default(),
            Witness,
        ));
        run_frame_despawning_targets_with_witness::<
            Doomed,
            Witness,
            crate::rendering::primitives::PortalSprite,
            _,
            _,
        >(&mut app, Update, sync_portal_sprite_visibility);
    }

    /// The parallax root is a room presentation entity. LDtk hot reload and
    /// ordinary room replacement may retire it in the same Update in which the
    /// per-view mirror pass decides it needs a `PresentedForView` key. The
    /// deferred write must tolerate that lifecycle race; a surviving root in the
    /// same fixture proves the mirror actually reached the write path.
    #[test]
    fn parallax_view_claim_survives_room_root_retirement() {
        use crate::rendering::parallax::{
            mirror_parallax_layers_per_view, ParallaxLayerVisual,
        };
        use ambition_sim_view::{LocalView, LocalViewId, PresentedForView};

        let mut app = App::new();
        let view = app.world_mut().spawn((LocalView, LocalViewId(0))).id();

        let layer = ParallaxLayerVisual {
            factor: Vec2::ONE,
            z: -18.0,
            panel_scale: 1.0,
            travel: Vec2::ZERO,
            world_size: Vec2::new(2000.0, 480.0),
        };
        app.world_mut().spawn((Sprite::default(), layer, Doomed));
        app.world_mut().spawn((Sprite::default(), layer, Witness));

        run_frame_despawning_targets_with_witness::<
            Doomed,
            Witness,
            PresentedForView,
            _,
            _,
        >(&mut app, Update, mirror_parallax_layers_per_view);

        let claimed = {
            let world = app.world_mut();
            let mut keyed = world.query_filtered::<&PresentedForView, With<Witness>>();
            keyed
                .single(world)
                .expect("the surviving root was claimed")
                .0
        };
        assert_eq!(
            claimed, view,
            "the witness proves the mirror reached the deferred ownership write"
        );
    }

    #[test]
    fn the_placeholder_sprite_override_survives_its_targets_being_retired() {
        let mut app = App::new();
        app.insert_resource(ambition_dev_tools::dev_tools::DeveloperTools {
            // The branch that writes: without this the pass takes its early-out
            // and the test would pass without exercising anything.
            placeholder_sprites: true,
            ..Default::default()
        });
        app.init_resource::<ambition_sim_view::FeatureViewIndex>();
        app.init_resource::<ambition_projectiles::ProjectileVisualCatalog>();
        // A sprite entity that a teardown is about to take, and one that
        // survives to prove the pass reached its write.
        app.world_mut().spawn((Sprite::default(), Doomed));
        app.world_mut().spawn((Sprite::default(), Witness));

        run_frame_despawning_targets_with_witness::<
            Doomed,
            Witness,
            crate::rendering::actors::SpriteOriginalState,
            _,
            _,
        >(&mut app, Update, apply_placeholder_sprites_override);
    }
}

/// Exercise the boss visual insert with all prerequisites satisfied. The fixture
/// resolves the boss sheet, loads page 0, and populates both read models so the
/// pass cannot succeed through an early-out.
#[cfg(test)]
mod boss_pass {
    use super::*;
    use crate::rendering::primitives::FeatureVisual;

    #[derive(Component)]
    struct Doomed;

    /// Survives the frame and proves the pass reached its write.
    #[derive(Component)]
    struct Witness;

    const BOSS_ID: &str = "probe_boss";
    const WITNESS_ID: &str = "probe_boss_witness";

    pub(super) fn a_feature_view() -> ambition_sim_view::FeatureView {
        ambition_sim_view::FeatureView {
            pos: ambition_platformer2d_core::Vec2::new(64.0, 64.0),
            size: ambition_platformer2d_core::Vec2::new(96.0, 128.0),
            kind: ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind::Actor,
            visible: true,
            submerged: false,
            wire_anchor: None,
            flash: false,
            breakable_state: None,
            chest_opened: false,
            fighting: true,
            switch_on: false,
            rotation_rad: 0.0,
            alive: true,
            hit_flash_secs: 0.0,
            parry_flash_secs: 0.0,
            hp_current: 40,
            hp_max: 40,
            training_dummy: false,
            hit_strength: 0.0,
            unhittable: false,
            defense_cues: ambition_sim_view::DefenseCueCauses::NONE,
            sprite_offset: None,
        }
    }

    #[test]
    fn the_boss_sprite_upgrade_survives_its_target_being_retired() {
        use ambition_sprite_sheet::boss::{BossSpriteAsset, BossSpritePage, BOSS_SHEET};

        let mut app = App::new();
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<Image>();
        app.init_asset::<bevy::image::TextureAtlasLayout>();

        // A REAL page-0 texture in `Assets<Image>`. The pass skips any boss
        // whose page-0 image has not finished loading, so without this the
        // insert below is never reached and the probe proves nothing.
        let texture = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::default());
        let layout = app
            .world_mut()
            .resource_mut::<Assets<bevy::image::TextureAtlasLayout>>()
            .add(bevy::image::TextureAtlasLayout::new_empty(
                bevy::math::UVec2::splat(128),
            ));
        let spec = BOSS_SHEET.clone();
        let record = spec.synth_record("probe_boss_spritesheet.png");
        let mut assets = ambition_sprite_sheet::game_assets::GameAssets::default();
        // The GENERIC sheet, which is the fallback arm every boss without a
        // dedicated sheet takes — so this probe covers the common path.
        assets.boss = Some(BossSpriteAsset {
            pages: vec![BossSpritePage { texture, layout }],
            record,
            spec,
        });
        app.insert_resource(assets);

        // Both read-models must carry the id: the boss identity is the GATE, and
        // the geometry view supplies the render size.
        let identity = || ambition_sim_view::BossRenderView {
            name: "Probe Boss".to_string(),
            behavior_id: "probe_boss".to_string(),
        };
        app.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([
            (BOSS_ID.to_string(), a_feature_view()),
            (WITNESS_ID.to_string(), a_feature_view()),
        ]));
        app.insert_resource(ambition_sim_view::BossRenderIndex::from_rows([
            (BOSS_ID.to_string(), identity()),
            (WITNESS_ID.to_string(), identity()),
        ]));

        // A boss visual with neither animator — the exact population the pass
        // upgrades — that a teardown is about to take.
        app.world_mut().spawn((
            FeatureVisual {
                id: BOSS_ID.to_string(),
            },
            Doomed,
        ));
        app.world_mut().spawn((
            FeatureVisual {
                id: WITNESS_ID.to_string(),
            },
            Witness,
        ));

        run_frame_despawning_targets_with_witness::<
            Doomed,
            Witness,
            ambition_sprite_sheet::boss::BossAnimator,
            _,
            _,
        >(
            &mut app,
            Update,
            crate::rendering::actors::upgrade_boss_sprites,
        );
    }
}

/// The character-sprite passes — the "also worth doing" half of L24.
///
/// `upgrade_actor_sprites` is the boss pass's twin over the ordinary actor
/// population, and its targets are the same `FeatureVisual` entities
/// `despawn_dead_dynamic_feature_visuals` retires. The two player passes target
/// `PlayerVisual`, which session teardown takes.
#[cfg(test)]
mod character_sprite_passes {
    use super::*;
    use crate::rendering::primitives::{FeatureVisual, PlayerVisual};

    #[derive(Component)]
    struct Doomed;

    const ACTOR_ID: &str = "probe_actor";
    const WITNESS_ID: &str = "probe_actor_witness";
    const ACTOR_NAME: &str = "Probe Actor";

    /// Survives the frame, and proves the pass reached its write. See
    /// [`run_frame_despawning_targets_with_witness`].
    #[derive(Component)]
    struct Witness;

    /// A real baked sheet, resolved through the same loader production uses.
    ///
    /// Hand-rolling a `CharacterSheetSpec` would be a fixture agreeing with
    /// itself; this asks the shipped record table for one, so a probe cannot
    /// pass against geometry no sheet has.
    fn a_published_sheet(
        app: &mut App,
    ) -> Option<ambition_sprite_sheet::character::CharacterSpriteAsset> {
        use ambition_sprite_sheet::character::sheets::{try_load_spec_for_target, SheetTuning};

        let spec = try_load_spec_for_target("robot", &SheetTuning::new(1.0, 1))?;
        let texture = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::default());
        let layout = app
            .world_mut()
            .resource_mut::<Assets<bevy::image::TextureAtlasLayout>>()
            .add(bevy::image::TextureAtlasLayout::new_empty(
                bevy::math::UVec2::splat(128),
            ));
        Some(ambition_sprite_sheet::character::CharacterSpriteAsset {
            texture: texture.clone(),
            layout: layout.clone(),
            spec,
            pages: vec![ambition_sprite_sheet::character::CharacterSpritePage { texture, layout }],
            requested_tier: ambition_persistence::settings::TextureResolutionScale::Full,
            resolved_tier: ambition_persistence::settings::TextureResolutionScale::Full,
        })
    }

    fn asset_app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<Image>();
        app.init_asset::<bevy::image::TextureAtlasLayout>();
        app
    }

    #[test]
    fn the_actor_sprite_upgrade_survives_its_target_being_retired() {
        let mut app = asset_app();
        let Some(sheet) = a_published_sheet(&mut app) else {
            // The baked record table is populated by `build.rs` from
            // `assets/sprites`. A checkout without it cannot run this probe, and
            // a probe that silently "passes" there would be the vacuous kind.
            eprintln!(
                "[deferred-write] SKIPPED: no baked `robot` sheet record, so this fixture \
                 cannot reach the insert it exists to exercise"
            );
            return;
        };
        let mut assets = ambition_sprite_sheet::game_assets::GameAssets::default();
        assets.characters.publish(ACTOR_NAME, sheet);
        app.insert_resource(assets);

        let identity = || ambition_sim_view::ActorRenderView {
            sprite_character_id: None,
            name: ACTOR_NAME.to_string(),
            sprite_override_name: None,
            is_sandbag: false,
            render_size: None,
            dream_seed: None,
        };
        app.insert_resource(ambition_sim_view::FeatureViewIndex::from_rows([
            (ACTOR_ID.to_string(), super::boss_pass::a_feature_view()),
            (WITNESS_ID.to_string(), super::boss_pass::a_feature_view()),
        ]));
        // The identity read-model: without a row the pass skips a frame, and the
        // probe would prove nothing.
        app.insert_resource(ambition_sim_view::ActorRenderIndex::from_rows([
            (ACTOR_ID.to_string(), identity()),
            (WITNESS_ID.to_string(), identity()),
        ]));
        // Empty: a boss id would make the actor path YIELD rather than bind.
        app.insert_resource(ambition_sim_view::BossRenderIndex::default());

        app.world_mut().spawn((
            FeatureVisual {
                id: ACTOR_ID.to_string(),
            },
            Doomed,
        ));
        app.world_mut().spawn((
            FeatureVisual {
                id: WITNESS_ID.to_string(),
            },
            Witness,
        ));

        run_frame_despawning_targets_with_witness::<
            Doomed,
            Witness,
            ambition_sprite_sheet::character::CharacterAnimator,
            _,
            _,
        >(
            &mut app,
            Update,
            crate::rendering::actors::upgrade_actor_sprites,
        );
    }

    /// The quality-change rebind, the last plain `insert` in the render layer.
    ///
    /// Same `PlayerVisual` target as the safety net below, reached on a very
    /// different frame: a confirmed quality-profile switch rebuilds `GameAssets`,
    /// and a provider switch in the same frame despawns the session scope.
    #[test]
    fn the_player_sprite_quality_rebind_survives_its_target_being_retired() {
        let mut app = asset_app();
        let Some(sheet) = a_published_sheet(&mut app) else {
            eprintln!(
                "[deferred-write] SKIPPED: no baked `robot` sheet record, so this fixture \
                 cannot reach the insert it exists to exercise"
            );
            return;
        };
        let mut assets = ambition_sprite_sheet::game_assets::GameAssets::default();
        // `"player_robot_v3"` is the id this pass falls back to for a visual with no
        // `PlayerSpriteCharacter` marker — publishing under any other name makes
        // the pass skip and the probe vacuous.
        assets.characters.publish("player_robot_v3", sheet);
        app.insert_resource(assets);

        app.world_mut().spawn((
            PlayerVisual,
            ambition_sim_view::BodyPoseView::default(),
            Doomed,
        ));
        app.world_mut().spawn((
            PlayerVisual,
            ambition_sim_view::BodyPoseView::default(),
            Witness,
        ));

        run_frame_despawning_targets_with_witness::<
            Doomed,
            Witness,
            crate::rendering::PlayerSpriteBaseline,
            _,
            _,
        >(
            &mut app,
            Update,
            crate::rendering::actors::refresh_player_sprites_for_resident_quality,
        );
    }

    /// The bare-player safety net: a `PlayerVisual` with no worn identity and no
    /// sprite, which session teardown can take before the flush.
    #[test]
    fn the_bare_player_sprite_fallback_survives_its_target_being_retired() {
        let mut app = App::new();
        app.world_mut().spawn((PlayerVisual, Doomed));
        app.world_mut().spawn((PlayerVisual, Witness));
        run_frame_despawning_targets_with_witness::<Doomed, Witness, Sprite, _, _>(
            &mut app,
            Update,
            crate::rendering::actors::ensure_player_visual_sprite,
        );
    }
}
