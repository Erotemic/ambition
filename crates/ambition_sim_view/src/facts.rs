//! The observation-boundary staging ground (E4): small sim-resolved view
//! resources presentation consumes INSTEAD of querying live sim components.
//!
//! Every resource here is a plain-data snapshot rebuilt once per tick in the
//! sim tail (`SandboxSet::FeatureViewSync`) by a function of sim state — no
//! caching across ticks, no `Entity`/`Handle` borrows — so any observer
//! (render, RL, netcode confirmation, the fighter brain) can read the same
//! facts. This module (with `view_index`/`anim_helpers`/`pose_view`/
//! `camera_snapshot`) is the seed of the `ambition_sim_view` crate; it moves
//! wholesale at the E4 mint.

use bevy::prelude::*;

use ambition_actors::actor::{BodyKinematics, BodyMana, PlayerEntity, PrimaryPlayer};
use ambition_characters::actor::{BodyHealth, BodyWallet};
use ambition_characters::brain::ActorControl;
use ambition_engine_core as ae;
use ambition_platformer_primitives::markers::ControlledSubject;
use ambition_platformer_primitives::schedule::SimScheduleExt;

/// The controlled body's HUD meters, resolved sim-side (E4 slices 5+6+16):
/// health / mana / wallet follow the [`ControlledSubject`] — while
/// possessing, the HUD shows THAT body's meters, never the vacated home
/// avatar's. `present == false` means no controlled body resolved this tick
/// (startup frames) and the HUD holds its last drawn state.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct PlayerHudFacts {
    pub present: bool,
    pub hp_current: i32,
    pub hp_max: i32,
    pub mana_current: f32,
    pub mana_fraction: f32,
    pub balance: i32,
}

pub fn rebuild_player_hud_facts(
    mut facts: ResMut<PlayerHudFacts>,
    controlled: Option<Res<ControlledSubject>>,
    bodies: Query<(&BodyHealth, &BodyMana, Option<&BodyWallet>)>,
    primary: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
) {
    let subject = controlled
        .as_deref()
        .and_then(|subject| subject.0)
        .or_else(|| primary.single().ok());
    let Some((health, mana, wallet)) = subject.and_then(|e| bodies.get(e).ok()) else {
        facts.present = false;
        return;
    };
    *facts = PlayerHudFacts {
        present: true,
        hp_current: health.current(),
        hp_max: health.max(),
        mana_current: mana.meter.current,
        mana_fraction: mana.meter.fraction(),
        balance: wallet.map(|wallet| wallet.balance).unwrap_or(0),
    };
}

/// The controlled body's held item, resolved sim-side: the geometry facts
/// the hand-sprite needs plus the item identity and its brain-resolved aim
/// (so a possessed body's ranged item points where THAT body aims).
#[derive(Resource, Default, Clone, Debug)]
pub struct HeldItemView(pub Option<HeldItemFact>);

#[derive(Clone, Debug, PartialEq)]
pub struct HeldItemFact {
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub facing: f32,
    pub item_id: String,
    pub ranged: bool,
    pub aim: ae::Vec2,
}

pub fn rebuild_held_item_view(
    mut view: ResMut<HeldItemView>,
    controlled: Option<Res<ControlledSubject>>,
    bodies: Query<(
        &BodyKinematics,
        &ambition_actors::features::HeldItem,
        &ActorControl,
    )>,
) {
    view.0 = controlled
        .as_deref()
        .and_then(|subject| subject.0)
        .and_then(|e| bodies.get(e).ok())
        .map(|(kin, held, control)| HeldItemFact {
            pos: kin.pos,
            size: kin.size,
            facing: kin.facing,
            item_id: held.spec.id.clone(),
            ranged: held.spec.ranged.is_some(),
            aim: control.0.aim,
        });
}

/// Every ground item's visual facts (position, box, item id).
#[derive(Resource, Default, Clone, Debug)]
pub struct GroundItemsView(pub Vec<GroundItemFact>);

#[derive(Clone, Debug, PartialEq)]
pub struct GroundItemFact {
    pub pos: ae::Vec2,
    pub half_extent: ae::Vec2,
    pub item_id: String,
}

pub fn rebuild_ground_items_view(
    mut view: ResMut<GroundItemsView>,
    grounds: Query<&ambition_actors::items::pickup::GroundItem>,
) {
    view.0.clear();
    view.0.extend(grounds.iter().map(|ground| GroundItemFact {
        pos: ground.pos,
        half_extent: ground.half_extent,
        item_id: ground.spec.id.clone(),
    }));
}

/// Every walk-into world item's visual facts (position, box, the row it grants —
/// so the renderer can pick an icon/tint per pickup).
#[derive(Resource, Default, Clone, Debug)]
pub struct WorldItemsView(pub Vec<WorldItemFact>);

#[derive(Clone, Debug, PartialEq)]
pub struct WorldItemFact {
    pub pos: ae::Vec2,
    pub half_extent: ae::Vec2,
    /// The equipment row id the item grants (e.g. `"grow_cap"`), used only to
    /// choose the visual. An empty string if the payload has no id.
    pub row_id: String,
}

pub fn rebuild_world_items_view(
    mut view: ResMut<WorldItemsView>,
    items: Query<&ambition_actors::items::world_item::WorldItem>,
) {
    use ambition_actors::items::world_item::WorldItemPayload;
    view.0.clear();
    view.0.extend(items.iter().map(|item| WorldItemFact {
        pos: item.pos,
        half_extent: item.half_extent,
        row_id: match &item.payload {
            WorldItemPayload::Equip(row) => row.id.clone(),
        },
    }));
}

/// Every in-flight held shot (gun-sword laser / fireball).
#[derive(Resource, Default, Clone, Debug)]
pub struct HeldShotsView(pub Vec<HeldShotFact>);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeldShotFact {
    pub pos: ae::Vec2,
    pub vel: ae::Vec2,
    /// Radial fireball (draws the glowing sphere) vs a velocity-aligned
    /// spinning blade.
    pub fireball: bool,
}

pub fn rebuild_held_shots_view(
    mut view: ResMut<HeldShotsView>,
    projectiles: Query<(
        &BodyKinematics,
        &ambition_actors::items::pickup::HeldProjectile,
    )>,
) {
    view.0.clear();
    view.0
        .extend(projectiles.iter().map(|(kin, proj)| HeldShotFact {
            pos: kin.pos,
            vel: kin.vel,
            fireball: proj.explode_half > 0.0,
        }));
}

/// Every player's dropped recall-mark position.
#[derive(Resource, Default, Clone, Debug)]
pub struct MarkBeaconsView(pub Vec<ae::Vec2>);

pub fn rebuild_mark_beacons_view(
    mut view: ResMut<MarkBeaconsView>,
    marks: Query<&ambition_actors::abilities::traversal::mark_recall::PlayerMark>,
) {
    view.0.clear();
    view.0.extend(marks.iter().filter_map(|mark| mark.pos));
}

/// Every gravity-flip switch's geometry.
#[derive(Resource, Default, Clone, Debug)]
pub struct GravitySwitchesView(pub Vec<GravitySwitchFact>);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GravitySwitchFact {
    pub pos: ae::Vec2,
    pub half_extent: ae::Vec2,
}

pub fn rebuild_gravity_switches_view(
    mut view: ResMut<GravitySwitchesView>,
    switches: Query<&ambition_actors::gravity::GravityFlipSwitch>,
) {
    view.0.clear();
    view.0.extend(switches.iter().map(|sw| GravitySwitchFact {
        pos: sw.pos,
        half_extent: sw.half_extent,
    }));
}

/// Every heal shrine's geometry.
#[derive(Resource, Default, Clone, Debug)]
pub struct ShrinesView(pub Vec<ShrineFact>);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShrineFact {
    pub pos: ae::Vec2,
    pub half_extent: ae::Vec2,
}

pub fn rebuild_shrines_view(
    mut view: ResMut<ShrinesView>,
    shrines: Query<&ambition_actors::shrine::HealShrine>,
) {
    view.0.clear();
    view.0.extend(shrines.iter().map(|shrine| ShrineFact {
        pos: shrine.pos,
        half_extent: shrine.half_extent,
    }));
}

/// Tick the shrine activation pulse SIM-side (it used to be decremented by
/// the render animator — a render-owned sim write, the exact back-edge E4
/// kills). Uses scaled time, so bullet-time slows the pulse with the world.
pub fn tick_shrine_activation_pulse(
    world_time: Res<ambition_time::WorldTime>,
    mut activation: ResMut<ambition_actors::shrine::ShrineActivationPulse>,
) {
    if activation.remaining > 0.0 {
        activation.remaining = (activation.remaining - world_time.scaled_dt).max(0.0);
    }
}

/// Every wielded gun-sword's presentation facts: where the hand is, where
/// it aims, and the wielder's height (the sprite scales off it). Resolved
/// sim-side from the rider actors + the primary player's position.
#[derive(Resource, Default, Clone, Debug)]
pub struct WieldedGunSwordsView(pub Vec<GunSwordFact>);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GunSwordFact {
    pub hand_world: ae::Vec2,
    pub aim_world: ae::Vec2,
    pub rider_height: f32,
}

#[allow(clippy::type_complexity)]
pub fn rebuild_wielded_gun_swords_view(
    mut view: ResMut<WieldedGunSwordsView>,
    rider_actors: Query<(
        &ambition_actors::features::ActorDisposition,
        &ambition_actors::features::HeldItem,
        Option<&BodyKinematics>,
        Option<&BodyHealth>,
    )>,
    player_q: Query<&BodyKinematics, (With<PlayerEntity>, With<PrimaryPlayer>)>,
) {
    view.0.clear();
    let Ok(player) = player_q.single() else {
        return;
    };
    for (disposition, held_item, kin, health) in &rider_actors {
        if held_item.id() != "gun_sword" || disposition.is_peaceful() {
            continue;
        }
        let (Some(kin), Some(health)) = (kin, health) else {
            continue;
        };
        if !health.alive() {
            continue;
        }
        let rider_height = kin.size.y;
        view.0.push(GunSwordFact {
            hand_world: ambition_actors::features::rider_hand_world_pos(
                kin.pos,
                kin.facing,
                rider_height,
            ),
            aim_world: player.pos,
            rider_height,
        });
    }
}

/// Per-projectile presentation pose (E4 slice 13): the art-selection kind
/// plus the frame's kinematic facts, written on the projectile entity
/// sim-side. Render queries ONLY this component — never the live
/// `BodyKinematics`. Removed when a pooled projectile stops being live.
#[derive(Component, Clone, Copy, Debug)]
pub struct ProjectileView {
    pub kind: ambition_projectiles::ProjectileVisualKind,
    pub pos: ae::Vec2,
    pub vel: ae::Vec2,
    pub size: ae::Vec2,
}

#[allow(clippy::type_complexity)]
pub fn rebuild_projectile_views(
    mut commands: Commands,
    mut live: Query<
        (
            Entity,
            &BodyKinematics,
            &ambition_projectiles::ProjectileVisualKind,
            Option<&mut ProjectileView>,
        ),
        With<ambition_projectiles::LiveProjectile>,
    >,
    // Pooled projectiles: a reused entity that is no longer live must drop
    // its view so render despawns the visual instead of drawing a corpse.
    stale: Query<
        Entity,
        (
            With<ProjectileView>,
            Without<ambition_projectiles::LiveProjectile>,
        ),
    >,
) {
    for (entity, kin, kind, view) in &mut live {
        let next = ProjectileView {
            kind: *kind,
            pos: kin.pos,
            vel: kin.vel,
            size: kin.size,
        };
        match view {
            Some(mut view) => *view = next,
            None => {
                commands.entity(entity).insert(next);
            }
        }
    }
    for entity in &stale {
        commands.entity(entity).remove::<ProjectileView>();
    }
}

/// One dynamically-introduced feature's spawn facts (E4 slice 9): encounter
/// mobs, staged duel actors, post-boss NPCs, and reward chests appear after
/// room load, so render discovers them from THIS list instead of declaring
/// the marker/config queries itself.
#[derive(Clone, Debug)]
pub struct DynamicFeatureFact {
    pub id: String,
    /// Display label for the visual's debug `Name`.
    pub label: String,
    /// Family label ("Encounter mob" / "Staged actor" / "Post-boss NPC" /
    /// "Reward chest") — presentation naming only.
    pub family: &'static str,
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub visual_kind: ambition_combat::FeatureVisualKind,
    pub fighting: bool,
    /// The placeholder entity-sprite the spawn resolves to (from the actor's
    /// brain / the NPC's interactable / the chest payload).
    pub sprite_key: Option<ambition_sprite_sheet::game_assets::EntitySprite>,
}

#[derive(Resource, Default, Clone, Debug)]
pub struct DynamicFeatureViews(pub Vec<DynamicFeatureFact>);

#[allow(clippy::type_complexity)]
pub fn rebuild_dynamic_feature_views(
    mut view: ResMut<DynamicFeatureViews>,
    ecs_mobs: Query<
        (
            &ambition_actors::features::FeatureId,
            &ambition_actors::features::CenteredAabb,
            &ambition_actors::features::ActorDisposition,
            Option<&ambition_actors::features::ActorConfig>,
        ),
        With<ambition_actors::features::EncounterMob>,
    >,
    staged_actors: Query<
        (
            &ambition_actors::features::FeatureId,
            &ambition_actors::features::CenteredAabb,
            &ambition_actors::features::ActorDisposition,
            Option<&ambition_actors::features::ActorConfig>,
        ),
        With<ambition_actors::features::RuntimeStagedActor>,
    >,
    post_boss_npcs: Query<
        (
            &ambition_actors::features::FeatureId,
            &ambition_actors::features::FeatureName,
            &ambition_actors::features::CenteredAabb,
            &ambition_actors::features::ActorDisposition,
            Option<&ambition_actors::features::ActorConfig>,
            Option<&ambition_actors::features::ActorInteraction>,
        ),
        With<ambition_actors::features::PostBossNpc>,
    >,
    ecs_reward_chests: Query<
        (
            &ambition_actors::features::FeatureId,
            &ambition_actors::features::CenteredAabb,
            &ambition_actors::features::ChestFeature,
        ),
        bevy::prelude::Or<(
            With<ambition_actors::features::EncounterRewardChest>,
            With<ambition_actors::features::BossRewardChest>,
        )>,
    >,
) {
    use ambition_combat::FeatureVisualKind;
    use ambition_sprite_sheet::game_assets;
    view.0.clear();
    for (id, aabb, disposition, config) in &ecs_mobs {
        // Encounter mobs are hostile by construction; skip any peaceful one.
        let (false, Some(config)) = (disposition.is_peaceful(), config) else {
            continue;
        };
        view.0.push(DynamicFeatureFact {
            id: id.as_str().to_string(),
            label: config.name.clone(),
            family: "Encounter mob",
            pos: aabb.center,
            size: aabb.size(),
            visual_kind: FeatureVisualKind::Actor,
            fighting: true,
            sprite_key: game_assets::entity_sprite_for_enemy(&config.brain),
        });
    }
    for (id, aabb, disposition, config) in &staged_actors {
        let (false, Some(config)) = (disposition.is_peaceful(), config) else {
            continue;
        };
        view.0.push(DynamicFeatureFact {
            id: id.as_str().to_string(),
            label: config.name.clone(),
            family: "Staged actor",
            pos: aabb.center,
            size: aabb.size(),
            visual_kind: FeatureVisualKind::Actor,
            fighting: true,
            sprite_key: game_assets::entity_sprite_for_enemy(&config.brain),
        });
    }
    for (id, name, aabb, disposition, config, interaction) in &post_boss_npcs {
        let fighting = !disposition.is_peaceful();
        // A peaceful post-boss NPC resolves its sprite from the dialogue
        // interactable; a hostile one (provoked) from its archetype brain.
        let sprite_key = if disposition.is_peaceful() {
            match interaction {
                Some(i) => game_assets::entity_sprite_for_runtime_interactable(&i.interactable),
                None => continue,
            }
        } else {
            match config {
                Some(c) => game_assets::entity_sprite_for_enemy(&c.brain),
                None => continue,
            }
        };
        view.0.push(DynamicFeatureFact {
            id: id.as_str().to_string(),
            label: name.0.clone(),
            family: "Post-boss NPC",
            pos: aabb.center,
            size: aabb.size(),
            visual_kind: FeatureVisualKind::Actor,
            fighting,
            sprite_key,
        });
    }
    for (id, aabb, chest) in &ecs_reward_chests {
        view.0.push(DynamicFeatureFact {
            id: id.as_str().to_string(),
            label: id.as_str().to_string(),
            family: "Reward chest",
            pos: aabb.center,
            size: aabb.size(),
            visual_kind: FeatureVisualKind::Chest,
            fighting: false,
            sprite_key: game_assets::entity_sprite_for_runtime_chest(&chest.chest),
        });
    }
}

/// The live blink-destination preview, resolved sim-side (E4 slice 18): the
/// SAME destination resolution the actual blink uses (precision aim via
/// `blink_destination_to_point_clusters`, quick-tap along input/facing, both
/// against the moving-platform-composed world), so the preview can never
/// disagree with the eventual teleport endpoint. Render draws the ember
/// ring; it computes nothing.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct BlinkPreviewFact {
    /// Ring visible this tick (blink held / aiming, ability owned, gameplay
    /// allowed).
    pub active: bool,
    /// Predicted landing point.
    pub target: ae::Vec2,
    /// Precision (steered) aim vs quick-tap — picks the ember palette.
    pub precision: bool,
    /// The blinking body's smaller AABB extent — ring radius + ember size
    /// scale off it.
    pub body_min_extent: f32,
}

/// Rebuild [`BlinkPreviewFact`] each tick. Mirrors the destination
/// resolution used by the engine and the `show_blink_preview` debug overlay.
/// The blink button shares ground with menu input, so this honours the same
/// gameplay-only gate as `draw_player_debug` — paused / dialog states don't
/// light up the ring.
#[cfg(feature = "input")]
#[allow(clippy::type_complexity)]
pub fn rebuild_blink_preview_fact(
    mut fact: ResMut<BlinkPreviewFact>,
    world: ambition_platformer_primitives::lifecycle::SessionWorldRef<
        ambition_engine_core::RoomGeometry,
    >,
    platform_set: Res<ambition_world::collision::MovingPlatformSet>,
    mode: Res<bevy::prelude::State<ambition_platformer_primitives::schedule::GameMode>>,
    scene: Res<ambition_platformer_primitives::lifecycle::SceneEntities>,
    action_query: Query<
        &leafwing_input_manager::prelude::ActionState<ambition_input::SandboxAction>,
        With<ambition_platformer_primitives::lifecycle::PlayerVisual>,
    >,
    // The blink reticle previews from the CONTROLLED SUBJECT (the body
    // carrying `Brain::Player(PRIMARY)`) — the body you are driving — so it
    // follows a possessed body instead of hovering at the vacated home
    // avatar. Both player and actor bodies carry these blink clusters.
    controlled: Res<ControlledSubject>,
    player_q: Query<(
        &BodyKinematics,
        &ambition_engine_core::BodyAbilities,
        &ambition_engine_core::BodyMotionFacts,
    )>,
) {
    use ambition_engine_core as ae;
    use ambition_input::read_gameplay_control_frame;

    fact.active = false;
    let Ok((kin, abilities, motion_facts)) =
        controlled.0.and_then(|e| player_q.get(e).ok()).ok_or(())
    else {
        return;
    };
    let actions = if mode.get().allows_gameplay() {
        action_query.get(scene.player).ok()
    } else {
        None
    };
    let controls = actions.map(read_gameplay_control_frame).unwrap_or_default();

    if !(abilities.abilities.blink && (controls.blink_held || motion_facts.blink_aiming)) {
        return;
    }

    // Match the debug overlay's destination resolution exactly. The
    // moving-platform-aware temporary world is what the actual blink
    // resolves against, so the preview must use it too.
    let blink_world =
        ambition_actors::world::platforms::world_with_moving_platforms(&world.0, &platform_set.0);
    let target = if motion_facts.blink_aiming {
        ae::blink_destination_to_point_clusters(
            &blink_world,
            kin,
            abilities,
            kin.pos + motion_facts.blink_aim_offset,
        )
    } else {
        let aim = ae::Vec2::new(controls.axis_x, controls.axis_y)
            .normalize_or(ae::Vec2::new(kin.facing, 0.0));
        ae::blink_destination_clusters(&blink_world, kin, abilities, aim, ae::BLINK_DISTANCE)
    };

    *fact = BlinkPreviewFact {
        active: true,
        target,
        precision: motion_facts.blink_aiming,
        body_min_extent: kin.size.min_element(),
    };
}

/// Registers the observation-boundary view resources + their rebuilds in the
/// sim tail. Owned here (anti-god rule 5): the plugin that rebuilds a view
/// initializes it; presentation only reads.
pub struct SimViewPlugin;

impl Plugin for SimViewPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.init_resource::<PlayerHudFacts>()
            .init_resource::<HeldItemView>()
            .init_resource::<GroundItemsView>()
            .init_resource::<WorldItemsView>()
            .init_resource::<HeldShotsView>()
            .init_resource::<MarkBeaconsView>()
            .init_resource::<GravitySwitchesView>()
            .init_resource::<ShrinesView>()
            .init_resource::<WieldedGunSwordsView>()
            .init_resource::<DynamicFeatureViews>()
            .init_resource::<BlinkPreviewFact>();
        // The blink-preview resolve reads device actions, so it exists only
        // with the input layer; the FACT resource above is unconditional so
        // consumers read an inert default headless.
        #[cfg(feature = "input")]
        app.add_systems(
            sim,
            rebuild_blink_preview_fact
                .in_set(ambition_platformer_primitives::schedule::SandboxSet::FeatureViewSync),
        );
        app.add_systems(
            sim,
            (
                rebuild_player_hud_facts,
                rebuild_held_item_view,
                rebuild_ground_items_view,
                rebuild_world_items_view,
                rebuild_held_shots_view,
                rebuild_mark_beacons_view,
                rebuild_gravity_switches_view,
                rebuild_shrines_view,
                tick_shrine_activation_pulse,
                rebuild_wielded_gun_swords_view,
                rebuild_projectile_views,
                rebuild_dynamic_feature_views,
            )
                .in_set(ambition_platformer_primitives::schedule::SandboxSet::FeatureViewSync),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hud_facts_track_the_controlled_body() {
        use ambition_characters::actor::Health;
        let mut app = App::new();
        app.init_resource::<PlayerHudFacts>();
        app.add_systems(Update, rebuild_player_hud_facts);

        // Home avatar with a fat purse; a driven actor with its own economy.
        app.world_mut().spawn((
            PlayerEntity,
            PrimaryPlayer,
            BodyHealth::new(Health::new(20)),
            BodyMana::default(),
            BodyWallet { balance: 42 },
        ));
        let mut actor_hp = BodyHealth::new(Health::new(10));
        actor_hp.damage(7);
        let actor = app
            .world_mut()
            .spawn((actor_hp, BodyMana::default(), BodyWallet { balance: 7 }))
            .id();
        app.world_mut()
            .insert_resource(ControlledSubject(Some(actor)));
        app.update();

        let facts = *app.world().resource::<PlayerHudFacts>();
        assert!(facts.present);
        assert_eq!(
            (facts.hp_current, facts.hp_max),
            (3, 10),
            "HUD facts must snapshot the POSSESSED body's health"
        );
        assert_eq!(facts.balance, 7, "money is a body stat");
    }

    #[test]
    fn shrine_pulse_ticks_down_sim_side() {
        let mut app = App::new();
        app.insert_resource(ambition_time::WorldTime {
            raw_dt: 0.1,
            scaled_dt: 0.1,
        });
        app.insert_resource(ambition_actors::shrine::ShrineActivationPulse { remaining: 0.25 });
        app.add_systems(Update, tick_shrine_activation_pulse);
        app.update();
        let remaining = app
            .world()
            .resource::<ambition_actors::shrine::ShrineActivationPulse>()
            .remaining;
        assert!((remaining - 0.15).abs() < 1e-6);
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(
            app.world()
                .resource::<ambition_actors::shrine::ShrineActivationPulse>()
                .remaining,
            0.0,
            "pulse clamps at zero"
        );
    }
}
