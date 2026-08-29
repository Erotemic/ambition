//! The track-0 exit oracle: cross-feature state survives forced rollback.
//!
//! The scenario runs in `combat_calibration_lab` — the combat-verb calibration room — which authors
//! a patrol enemy, a striker pair, a breakable brick, and the classify-console switch along one
//! floor route. A steering policy walks the route: absorb one enemy hit with a worn armor row,
//! break the brick, land a melee hit, and flip the switch.
//!
//! That last sentence is the whole design, and it is load-bearing because this
//! route DID stop doing two of the four things and stayed green for it. The
//! walker aimed at the breakable brick's centre, walked into a block whose top
//! face stands 32 above the floor, hopped onto it, and swung horizontally over
//! the thing it was breaking — so `brick_broken` and (gated behind it in route
//! order) `switch_flipped` were false on every pass. A steering policy is
//! CONTENT-SHAPED: it can stop reaching a prop because the room changed, with no
//! compile error and no failing assertion unless the assertion exists.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{
    AgentAction, AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};
use ambition_platformer2d::characters::actor::BodyHealth;
use ambition_platformer2d::characters::equipment::{EquipmentRow, OnHit, WornEquipment};
use bevy::prelude::{Entity, With, Without};

const ORACLE_ARMOR_ID: &str = "oracle_armor";
const MAX_FRAMES: usize = 2400;
const MIN_FRAMES: usize = 600;

fn oracle_sim() -> Platformer2dSimHarness {
    Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room("combat_calibration_lab")
            .with_sync_test_rollback_settings(4, 10),
    )
    .expect("Ambition GGRS sync-test harness builds in the calibration lab")
}

/// Dress the player in one armor row so the first enemy hit is an armor spend
/// rather than an HP loss. `WornEquipment` is registered rollback state, so
/// this pre-run mutation is part of frame-0 state like any authored loadout.
fn wear_oracle_armor(sim: &mut Platformer2dSimHarness) {
    let world = sim.world_mut();
    let player = {
        let mut q =
            world.query_filtered::<Entity, With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>();
        q.single(world)
            .expect("the sim boots exactly one primary player")
    };
    let row = EquipmentRow {
        id: ORACLE_ARMOR_ID.to_string(),
        on_hit: Some(OnHit::ConsumeAsArmor { downgrade_to: None }),
        ..Default::default()
    };
    match world.get_mut::<WornEquipment>(player) {
        Some(mut worn) => worn.rows.push(row),
        None => {
            world
                .entity_mut(player)
                .insert(WornEquipment::new(vec![row]));
        }
    }
    // Deep HP so the run cannot die: a player death triggers a sim-side room
    // RESET, and room reconstruction runs through Commands that no rollback
    // can undo — a reset inside the resim window is a guaranteed divergence
    // (observed at frame ~2147 during development: enemy HP snapped back to
    // full mid-brawl, then checksums split). That boundary is a recorded
    // Phase-5 finding, not this oracle's subject; the oracle stays inside the
    // proven envelope.
    if let Some(mut health) = world.get_mut::<BodyHealth>(player) {
        health.health.max = 200;
        health.health.current = 200;
    }
    // Direct world_mut mutations must become the rollback baseline — GGRS's
    // stored history predates them, and a restore would resurrect the
    // pre-setup state (harness contract on `world_mut`; §2).
    sim.rebase_rollback_history()
        .expect("oracle armor setup becomes the rollback baseline");
}

/// Stage the player on the open arena floor as part of the frame-0 baseline.
///
/// The authored spawn corner is capped by a head-height ledge + rebound pad
/// (the room's parkour tutorial) — crossing it is a platforming exercise, and
/// platforming is not this oracle's subject. The oracle's route (spitter,
/// brick, striker, switch) all lives on the arena floor to the right, so the
/// baseline places the player just east of the hazard cycle (x=720; the
/// hazard band spans x 592-688 and eats a body staged inside it), like the
/// armor row:
/// a setup mutation folded into rollback frame zero by the rebase that follows.
fn stage_player_on_arena_floor(sim: &mut Platformer2dSimHarness) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&mut ambition_platformer2d::platformer::body::BodyKinematics, With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>();
    let mut kin = q
        .single_mut(world)
        .expect("the sim boots exactly one primary player");
    kin.pos = ambition_platformer2d::engine_core::Vec2::new(720.0, kin.pos.y);
    kin.vel = ambition_platformer2d::engine_core::Vec2::ZERO;
    sim.rebase_rollback_history()
        .expect("arena-floor staging becomes the rollback baseline");
}

struct OracleEvents {
    melee_landed: bool,
    armor_spent: bool,
    brick_broken: bool,
    switch_flipped: bool,
}

impl OracleEvents {
    fn all(&self) -> bool {
        self.melee_landed && self.armor_spent && self.brick_broken && self.switch_flipped
    }
}

/// The exact props the route is supposed to act on, by authored id.
///
/// Pinned by [`FeatureId`] rather than by `Entity`: bevy_ggrs DESTROYS and recreates
/// rollback entities, so a handle captured at calibration names nothing after the
/// first forced rewind. The authored id is the identity that survives, which is the
/// same reason the localizer projects entity references through `SimId`.
#[derive(Clone, Debug)]
struct OracleTargets {
    brick: String,
    switch: String,
}

/// The authored ids, from `intro.ldtk`'s `combat_calibration_lab` level: the
/// `BreakablePlatform` named `calibration_brick` and the `Switch` whose id is
/// `combat_lab_classify_switch`.
const ORACLE_BRICK_NAME: &str = "calibration_brick";
const ORACLE_SWITCH_ID: &str = "combat_lab_classify_switch";

/// Identify the route's targets and ASSERT they start in the state the route is
/// supposed to change. A calibration that cannot find them, or finds them already
/// done, fails here rather than producing a green run that proved nothing.
fn calibrate_targets(sim: &mut Platformer2dSimHarness) -> OracleTargets {
    let world = sim.world_mut();

    // Matched on the authored NAME and recorded by runtime id: LDtk gives a
    // breakable its `iid` as `FeatureId` (`BreakablePlatform-104919`), which is
    // stable but opaque, while the designer-facing handle is the `name` field on
    // `FeatureName`. Naming the readable one keeps the constant meaningful; keying
    // the observation by `FeatureId` keeps it stable across bevy_ggrs recreating
    // the entity.
    let bricks: Vec<(String, String, bool)> = {
        let mut q = world.query::<(
            &ambition_platformer2d::combat::components::FeatureId,
            &ambition_platformer2d::combat::components::FeatureName,
            &ambition_platformer2d::combat::components::BreakableFeature,
        )>();
        q.iter(world)
            .map(|(id, name, feature)| (id.0.clone(), name.0.clone(), feature.broken()))
            .collect()
    };
    let switches: Vec<(String, bool)> = {
        let mut q = world.query::<(
            &ambition_platformer2d::combat::components::FeatureId,
            &ambition_platformer2d::encounter::switches::SwitchOn,
        )>();
        q.iter(world).map(|(id, on)| (id.0.clone(), on.0)).collect()
    };

    let (brick, _, already_broken) = bricks
        .iter()
        .find(|(_, name, _)| name == ORACLE_BRICK_NAME)
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "the calibration lab must author a breakable named `{ORACLE_BRICK_NAME}` \
                 — the route is aimed at THAT prop, and taking whichever breakable the \
                 query yielded first would silently retarget the oracle. Present: {:?}",
                bricks.iter().map(|(_, name, _)| name).collect::<Vec<_>>()
            )
        });
    assert!(
        !already_broken,
        "`{brick}` is ALREADY broken at calibration, so the brick objective is          satisfied before the route starts and proves nothing"
    );
    let (switch, already_on) = switches
        .iter()
        .find(|(id, _)| id == ORACLE_SWITCH_ID)
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "the calibration lab must author a switch with id \
                 `{ORACLE_SWITCH_ID}`. Present: {:?}",
                switches.iter().map(|(id, _)| id).collect::<Vec<_>>()
            )
        });
    assert!(
        !already_on,
        "`{switch}` is ALREADY on at calibration, so the switch objective is          satisfied before the route starts and proves nothing"
    );
    OracleTargets { brick, switch }
}

/// Read every oracle observation from live world state.
fn observe(
    sim: &mut Platformer2dSimHarness,
    targets: &OracleTargets,
    enemy_health_baseline: i32,
    events: &mut OracleEvents,
) {
    let world = sim.world_mut();

    let enemy_health: i32 = {
        let mut q = world
            .query_filtered::<&BodyHealth, Without<ambition_platformer2d::platformer::markers::PrimaryPlayer>>();
        q.iter(world).map(|body| body.health.current).sum()
    };
    if enemy_health < enemy_health_baseline {
        events.melee_landed = true;
    }

    {
        let mut q = world
            .query_filtered::<&WornEquipment, With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>();
        if let Ok(worn) = q.single(world) {
            if !worn.wears(ORACLE_ARMOR_ID) {
                events.armor_spent = true;
            }
        }
    }

    // THE brick and THE switch the route is aimed at, by authored id — not "any
    // breakable" and "any switch", which a second prop in a different initial state
    // would satisfy for free.
    {
        let mut q = world.query::<(
            &ambition_platformer2d::combat::components::FeatureId,
            &ambition_platformer2d::combat::components::BreakableFeature,
        )>();
        if q.iter(world)
            .any(|(id, feature)| id.0 == targets.brick && feature.broken())
        {
            events.brick_broken = true;
        }
    }

    {
        let mut q = world.query::<(
            &ambition_platformer2d::combat::components::FeatureId,
            &ambition_platformer2d::encounter::switches::SwitchOn,
        )>();
        if q.iter(world).any(|(id, on)| id.0 == targets.switch && on.0) {
            events.switch_flipped = true;
        }
    }
}

/// Centers of the living enemies, in sim space.
///
/// Split out of `target_positions` because the probes below run a policy that
/// only chases enemies: building and iterating the brick and switch queries for
/// values they discard costs two fresh `QueryState`s on every simulated frame,
/// and these loops run 600-2400 frames.
fn enemy_positions(sim: &mut Platformer2dSimHarness) -> Vec<(f32, f32)> {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<(
        &ambition_platformer2d::platformer::body::BodyKinematics,
        &BodyHealth,
    ), Without<ambition_platformer2d::platformer::markers::PrimaryPlayer>>();
    q.iter(world)
        .filter(|(_, health)| health.health.current > 0)
        .map(|(kin, _)| {
            use bevy::math::bounding::BoundingVolume;
            let center = kin.aabb().center();
            (center.x, center.y)
        })
        .collect()
}

/// A prop's box, in sim space: center plus half-width.
///
/// The half-width is not decoration. The brick is a 48x48 block whose top face
/// stands 32 above the floor, so a policy that steers at its CENTER walks into it,
/// climbs it (the route's periodic hop is enough), and then swings horizontally
/// over the thing it is trying to break. That is exactly what this route did for
/// its whole existence — see `brick_standoff`.
#[derive(Clone, Copy, Debug)]
struct PropBox {
    x: f32,
    half_w: f32,
}

// no `y`. It was recorded and never read, and the compiler said so. The
// standoff this feeds is a HORIZONTAL question — which side to approach from and
// where to stop — so a centre height is a fact the oracle does not use. Carrying
// it anyway is how a reader concludes the approach considers height when it does
// not. If the oracle ever needs to reach a brick above or below the player, the
// field comes back WITH the code that reads it.

/// Where to STAND to hit the brick, rather than where the brick is.
///
/// Approach from whichever side the player is already on, and stop clear of the
/// block's face by its half-width plus the swing's reach. The strike volume is
/// offset forward of the body, so standing flush against the face puts the volume
/// PAST the block — and standing on top of it puts the volume above it.
fn brick_standoff(brick: PropBox, px: f32) -> f32 {
    const STANDOFF: f32 = 26.0;
    if px <= brick.x {
        brick.x - brick.half_w - STANDOFF
    } else {
        brick.x + brick.half_w + STANDOFF
    }
}

/// Positions of the actionable things, in sim space, queried live so the
/// policy needs no knowledge of the room's coordinate frame.
fn target_positions(
    sim: &mut Platformer2dSimHarness,
    targets: &OracleTargets,
) -> (Vec<(f32, f32)>, Option<PropBox>, Option<(f32, f32)>) {
    let enemies = enemy_positions(sim);
    let world = sim.world_mut();

    // The SAME props `observe` watches. Steering at one brick while asserting on
    // another is how a route can walk past its objective and still report it done.
    let brick = {
        let mut q = world.query::<(
            &ambition_platformer2d::combat::components::FeatureId,
            &ambition_platformer2d::combat::components::BreakableFeature,
            &ambition_platformer2d::engine_core::geometry::CenteredAabb,
        )>();
        q.iter(world)
            .find(|(id, feature, _)| id.0 == targets.brick && !feature.broken())
            .map(|(_, _, aabb)| PropBox {
                x: aabb.center.x,
                half_w: aabb.size().x / 2.0,
            })
    };

    let switch = {
        let mut q = world.query::<(
            &ambition_platformer2d::combat::components::FeatureId,
            &ambition_platformer2d::encounter::switches::SwitchFeature,
            &ambition_platformer2d::engine_core::geometry::CenteredAabb,
        )>();
        q.iter(world)
            .find(|(id, _, _)| id.0 == targets.switch)
            .map(|(_, _, aabb)| (aabb.center.x, aabb.center.y))
    };

    (enemies, brick, switch)
}

/// Every state-bearing rollback registration must have a localization probe.
/// The test compares the registry directly with the probe vocabulary so adding
/// registered state without a probe fails immediately.
#[test]
fn every_state_bearing_rollback_registration_owns_a_localization_probe() {
    let sim = oracle_sim();
    let registry = sim
        .world()
        .resource::<ambition_platformer2d::rollback::RollbackRegistry>();
    let probed = sim
        .world()
        .resource::<ambition_platformer2d::rollback::RollbackChecksumProbes>()
        .type_names();

    let mut state_bearing = 0usize;
    let mut missing: Vec<String> = Vec::new();
    for descriptor in registry.descriptors() {
        if !descriptor.kind.carries_state() {
            continue;
        }
        state_bearing += 1;
        if !probed.contains(descriptor.type_name.as_str()) {
            missing.push(format!(
                "  {} [{}] {}",
                descriptor.name,
                descriptor.kind.canonical_name(),
                descriptor.type_name
            ));
        }
    }
    // Vacuity guard: a composition that registered nothing would pass the coverage
    // check trivially, and this test would then be asserting that zero equals zero
    // for the rest of the project's life.
    assert!(
        state_bearing > 50,
        "only {state_bearing} state-bearing registrations were found; this sim is \
         supposed to compose the whole game, so the comparison below would be \
         vacuous"
    );
    missing.sort();
    assert!(
        missing.is_empty(),
        "{} of {state_bearing} state-bearing rollback registrations have NO \
         localization probe, so the localizer is blind to them and cannot support \
         any statement about what did or did not survive a restore:\n{}",
        missing.len(),
        missing.join("\n")
    );
    println!(
        "[probe coverage] {state_bearing} state-bearing registrations, {} probes",
        probed.len()
    );
}

/// Every presence-only rollback probe is enumerated with a reason.
///
/// Presence counts cannot detect value corruption, so new weak probes must be
/// explicitly justified. Registrations that gain value probes must also leave this
/// list, keeping the weakness inventory synchronized with probe strength.
#[test]
fn every_presence_only_probe_is_named_with_its_reason() {
    // Require exact Rust type names plus a reason. Substring allowlists can
    // accidentally exempt future types that merely share a name fragment.
    //
    // DERIVED registrations are not listed here. Their justification is the rebuild
    // promise they already declare at the registration site — `declare_rollback_derived_*`
    // takes a `reason` that lands in the descriptor's `detail` — and copying it into
    // a test list would create a second copy to drift. The rule below checks that
    // promise is actually written down instead.
    const PRESENCE_ONLY: &[(&str, &str)] = &[
        (
            "ambition_boss_encounter::encounter_entity::EncounterDef",
            "authored encounter definition; immutable at runtime",
        ),
        (
            "ambition_platformer2d_actor_monolith::character_runtime::hurtbox::AuthoredHurtboxes",
            "authored hurtbox document; immutable at runtime",
        ),
        (
            "ambition_sprite_sheet::character::sheets::SpritePosedBody",
            "authored per-pose body table; immutable at runtime",
        ),
                (
            "ambition_encounter::switches::SwitchFeature",
            "authored switch payload; the mutable half is SwitchOn, value-probed",
        ),
        (
            "ambition_combat::actor_tuning::ActorConfig",
            "MUTATED AT RUNTIME (checked 2026-08-29): `apply_catalog_mode` writes\
             `brain_profile`, `brain` and `sprite_override_npc_name` on a controller\
             change — wants a value projection",
        ),
        (
            "ambition_platformer2d_shared_tangle::body::SpawnBaseline",
            "the authored body a reset hands back; written by the SEED before the entity exists",
        ),
        (
            "ambition_characters::actor::limb::LimbIntents",
            "republished every tick by the limb router",
        ),
        (
            "ambition_characters::actor::limb::LimbRouteState",
            "republished every tick by the limb router",
        ),
        (
            "ambition_boss_encounter::clusters::BossConfig",
            "authored boss definition; nothing writes it after spawn",
        ),
        (
            "ambition_mount::CanPilot",
            "authored capability payload; immutable at runtime",
        ),
        (
            // ⛔ THE PATH MOVED 2026-08-26 and this list is keyed by the PATH.
            // It is one of the ledgers a moved registered type touches; the
            // others are the registration turbofish and the schema baseline
            // (whose STABLE NAME, `mount.mass`, deliberately did not change).
            // `Mass` left `features::ecs::mount` because two domains share it;
            // the rest of that module then left the monolith entirely for
            // `crates/ambition_mount`, which is why the rows below say
            // `ambition_mount::` and not `features::ecs::mount::`.
            "ambition_platformer2d_shared_tangle::body::Mass",
            "authored mass; immutable at runtime",
        ),
        (
            "ambition_mount::Mountable",
            "authored capability payload; immutable at runtime",
        ),
        (
            "ambition_mount::MountedBrainCache",
            "a cached Brain + ActionSet; holds no entity handle (checked 2026-07-27)",
        ),
        (
            "ambition_mount::MountedSize",
            "authored size; immutable at runtime",
        ),
        (
            "ambition_platformer2d_actor_monolith::features::ecs::pickups::PickupArt",
            "authored art id; immutable at runtime",
        ),
        (
            "ambition_boss_encounter::clusters::BossOverrides",
            "authored spawn overrides; immutable at runtime",
        ),
        (
            "ambition_platformer2d_actor_monolith::gravity::lifecycle::GravityFlipSwitch",
            "authored switch payload; immutable at runtime",
        ),
        (
            "ambition_platformer2d_actor_monolith::items::pickup::GroundItem",
            "authored item spec; immutable while it lies on the ground",
        ),
        (
            "ambition_platformer2d_actor_monolith::items::pickup::StashedActionSet",
            "authored action set held across a possession",
        ),
        (
            "ambition_characters::actor::body::BodyAnimFacts",
            "republished every tick from motion by the body animator",
        ),
        (
            "ambition_characters::actor::pose::ActorFaction",
            "authored faction; changed only by possession, which respawns the body",
        ),
        (
            "ambition_characters::control::PlayerSlot",
            "authored slot index; immutable for the session",
        ),
        (
            "ambition_characters::brain::action_set::ActionSet",
            "MUTATED AT RUNTIME (checked 2026-08-29): `apply_catalog_mode` overwrites\
             the whole value for a peaceful body — wants a value projection",
        ),
        (
            "ambition_characters::brain::action_set::IdentityKit",
            "authored kit; immutable at runtime",
        ),
        (
            "ambition_characters::brain::boss_pattern::BossCapability",
            "authored capability set; immutable at runtime",
        ),
        (
            "ambition_characters::equipment::WornEquipment",
            "equipment rows; wants a canonical projection (G2b)",
        ),
        (
            "ambition_combat::components::CombatCapabilities",
            "MUTATED AT RUNTIME (checked 2026-08-29): `apply_catalog_mode` overwrites\
             the whole value for a peaceful body — wants a value projection",
        ),
        (
            "ambition_combat::components::CombatTuning",
            "authored tuning; immutable at runtime",
        ),
        (
            "ambition_combat::components::actors::ActorIdentity",
            "authored identity; immutable at runtime",
        ),
        (
            "ambition_combat::components::actors::ActorInteraction",
            "authored interaction payload; immutable at runtime",
        ),
        (
            "ambition_combat::components::actors::ActorRenderSize",
            "authored render size; immutable at runtime",
        ),
        (
            "ambition_combat::components::actors::ActorSpriteOffset",
            "authored sprite offset; immutable at runtime",
        ),
        (
            "ambition_combat::components::actors::BossDeathAnimation",
            "authored animation spec; immutable at runtime",
        ),
        (
            "ambition_combat::components::actors::CombatKit",
            "authored kit; immutable at runtime",
        ),
        (
            "ambition_combat::components::features::BreakableFeature",
            "authored breakable payload; the mutable half is its broken flag",
        ),
        (
            "ambition_combat::components::features::ChestFeature",
            "authored chest payload; the mutable half is the Opened marker",
        ),
        (
            "ambition_combat::components::features::DamageableVolumes",
            "republished every tick by refresh_body_damageable_volumes",
        ),
        (
            "ambition_combat::components::features::FeatureId",
            "authored stable id; immutable at runtime",
        ),
        (
            "ambition_combat::components::features::FeatureName",
            "authored name; immutable at runtime",
        ),
        (
            "ambition_combat::components::features::PickupFeature",
            "authored pickup payload; the mutable half is the Collected marker",
        ),
        (
            "ambition_combat::components::features::PogoPolicy",
            "authored policy; immutable at runtime",
        ),
        (
            "ambition_combat::components::features::PogoTargetVolumes",
            "republished every tick by the pogo-target publisher",
        ),
        (
            "ambition_combat::hazard_runtime::HazardFeature",
            "authored hazard payload; immutable at runtime",
        ),
        (
            "ambition_combat::held_items::HeldItem",
            "item spec, replaced wholesale on pickup rather than mutated in place",
        ),
        (
            "ambition_combat::moveset::ActorMoveset",
            "authored moveset; the mutable half is MovePlayback, canonical",
        ),
        (
            "ambition_combat::targeting::FactionRelations",
            "authored relation matrix; immutable at runtime",
        ),
        (
            "ambition_combat::targeting::FriendlyFire",
            "authored policy; immutable at runtime",
        ),
        (
            "ambition_content::bosses::cut_rope::CutRopeHeavyObjectCycle",
            "MUTATED AT RUNTIME (checked 2026-08-29):\
             `reset_cut_rope_boss_arena_on_room_reset` calls `advance()` — wants a\
             value projection",
        ),
        (
            "ambition_encounter::entity::Encounter",
            "an encounter id STRING; already a stable identity",
        ),
        (
            "ambition_encounter::music::EncounterMusicRequest",
            "authored music request; immutable at runtime",
        ),
        (
            "ambition_encounter::objective::EncounterObjective",
            "authored objective; immutable at runtime",
        ),
                (
            "ambition_encounter::staging::EncounterCameraZoom",
            "authored staging camera; immutable at runtime",
        ),
        (
            "ambition_encounter::staging::EncounterLockWall",
            "authored staging geometry; immutable at runtime",
        ),
        (
            "ambition_encounter::staging::EncounterTrack",
            "authored track reference; immutable at runtime",
        ),
        (
            "ambition_platformer2d_core::body_clusters::AbilityBase",
            "refreshed every tick from the ability set",
        ),
        (
            "ambition_platformer2d_core::world::RoomGeometry",
            "authored room geometry; immutable while the room is loaded",
        ),
        (
            "ambition_items::OwnedItems",
            "inventory set; wants a canonical projection (G2b)",
        ),
        (
            "ambition_platformer2d_actor_monolith::session::durable_horizon::SaveRestored",
            "the save-applied latch, set in literal `Update` and so NOT in step \
             with the sim ticks a checksum covers — projecting the bool reddened \
             `the_calibration_lab_is_checksum_stable_at_rest` and most of this \
             file. It must REWIND (or a rewind past the restore keeps the record \
             of applying a save it just undid, and the write-back puts the \
             starter inventory over it) and must NOT be checksummed",
        ),
        (
            "ambition_portal2d::eviction::PortalFrameHistory",
            "channel -> aperture geometry; holds no entity handle (checked 2026-07-27)",
        ),
        (
            "ambition_portal2d::gun::PortalGun",
            "an active flag and the next channel colour; holds no entity handle",
        ),
        (
            "ambition_portal2d::gun_pickup::PortalGunPickup",
            "position, half-extent and an arm timer; holds no entity handle",
        ),
        (
            "ambition_portal2d::gun_projectile::PortalShot",
            "channel plus shot kinematics; holds no entity handle",
        ),
        (
            "ambition_portal2d::transit::PortalEmission",
            "an exit normal and a protection timer; holds no entity handle",
        ),
        (
            "ambition_portal2d::transit::PortalPolicy",
            "authored policy; immutable at runtime",
        ),
        (
            "ambition_portal2d::transit::PortalTransit",
            "the straddled CHANNEL plus a crossed flag; a channel is a stable identity, not a handle",
        ),
        (
            "ambition_portal2d::types::PlacedPortal",
            "hosted on a `GeoFaceRef` (a stable `GeoId` + face), which is the stable identity G2b asks for — deliberately never an entity handle",
        ),
        (
            "ambition_platformer2d_runtime::input_stream::InputStreamRecorder",
            "the recorded input stream itself; grows every frame by design",
        ),
        (
            "ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState",
            "presentation camera state, republished from the blink clock",
        ),
        (
            "ambition_platformer2d_world::rooms::metadata::ActiveRoomMetadata",
            "authored room metadata; replaced on room load",
        ),
        (
            "ambition_platformer2d_world::rooms::metadata::RoomMusicRequest",
            "authored music request; immutable at runtime",
        ),
        (
            "ambition_sprite_sheet::character::anim::ActorAnimOverride",
            "republished from the move clock by the moveset animator",
        ),
        (
            "bevy_ecs::name::Name",
            "authored debug name; immutable at runtime",
        ),
        (
            "bevy_transform::components::transform::Transform",
            "presentation transform, republished from BodyKinematics every frame",
        ),
    ];

    let sim = oracle_sim();
    let registry = sim
        .world()
        .resource::<ambition_platformer2d::rollback::RollbackRegistry>();
    let probes = sim
        .world()
        .resource::<ambition_platformer2d::rollback::RollbackChecksumProbes>();
    let presence_only = probes.presence_only_type_names();
    let derived: std::collections::BTreeSet<&str> = probes
        .probes()
        .filter(|probe| probe.is_derived())
        .map(|probe| probe.type_name)
        .collect();

    // EXACT full type names, both directions. An allowlist that approximately matches is an
    // allowlist that grows without anyone deciding.
    //
    // Derived probes are listed individually too, rather than skipped as a class.
    // "It is derived" is a real reason and it is also the reason `ProjectileOwner`'s
    // broken derived promise went unnoticed for a day — so each one says which
    // system is supposed to rebuild it.
    let listed: std::collections::BTreeMap<&str, &str> = PRESENCE_ONLY.iter().copied().collect();
    assert_eq!(
        listed.len(),
        PRESENCE_ONLY.len(),
        "the presence-only list has a duplicate entry"
    );
    assert!(
        listed.values().all(|reason| reason.len() > 12),
        "every entry needs a real reason, not a placeholder"
    );

    // A derived presence-only probe is justified by the rebuild promise its
    // declaration makes. Empty or perfunctory is a declaration nobody wrote.
    let declared_reason: std::collections::BTreeMap<&str, &str> = registry
        .descriptors()
        .filter(|d| d.kind == ambition_platformer2d::rollback::RollbackEntryKind::Derived)
        .map(|d| (d.type_name.as_str(), d.detail.as_str()))
        .collect();
    let mut unpromised: Vec<&str> = derived
        .iter()
        .filter(|type_name| presence_only.contains(*type_name))
        .filter(|type_name| {
            declared_reason
                .get(*type_name)
                .is_none_or(|reason| reason.len() < 20)
        })
        .copied()
        .collect();
    unpromised.sort();
    assert!(
        unpromised.is_empty(),
        "{} derived registration(s) have a presence-only probe and no substantive \
         rebuild promise on the declaration. Presence can see that nobody rebuilt \
         the component — the failure `ProjectileOwner` actually shipped — and cannot \
         see one rebuilt WRONG, so the declaration has to say which system owes the \
         rebuild:\n  {}",
        unpromised.len(),
        unpromised.join("\n  ")
    );

    let mut unlisted: Vec<&str> = presence_only
        .iter()
        .filter(|type_name| !derived.contains(*type_name))
        .filter(|type_name| !listed.contains_key(*type_name))
        .copied()
        .collect();
    unlisted.sort();
    assert!(
        unlisted.is_empty(),
        "{} registration(s) carry a PRESENCE-ONLY localization probe and are not named \
         in this test's list. A presence probe satisfies the coverage test above while \
         seeing nothing of the value, so each needs either a value projection \
         (`rollback_component_clone_entity_ref` for a handle, \
         `rollback_component_clone_probed` for anything else, \
         `declare_rollback_derived_component_state` for derived state) or an entry \
         here saying why it cannot have one:\n  {}",
        unlisted.len(),
        unlisted.join("\n  ")
    );

    // And the reverse: an entry that no longer describes a presence-only probe —
    // the type gained a value projection, or is no longer registered at all — must
    // be dropped, or the list drifts into a description of a world that has moved on.
    let mut stale: Vec<&str> = listed
        .keys()
        .filter(|name| !presence_only.contains(*name) || derived.contains(*name))
        .copied()
        .collect();
    stale.sort();
    assert!(
        stale.is_empty(),
        "these entries no longer describe a presence-only probe:\n  {}",
        stale.join("\n  ")
    );

    // This list enforced that a sentence EXISTS, never that it is true, and nine
    // of its thirteen entity-handle claims were false: `PortalShot` carries shot
    // kinematics, `PortalGun` a flag and a colour, `SwitchActivationQueue` three
    // strings, `PlacedPortal` a `GeoFaceRef` whose `GeoId` is deliberately the
    // stable identity G2b asks for. The row would have sent somebody to add
    // stable-identity projections to types that hold nothing else.
    //
    // A type holds remappable handles exactly when it has an entity-mapping
    // registration, and that is a fact in the registry. So the two must agree in
    // both directions.
    let entity_mapped: std::collections::BTreeSet<&str> = registry
        .descriptors()
        .filter(|d| {
            matches!(
                d.kind,
                ambition_platformer2d::rollback::RollbackEntryKind::EntityMapping
                    | ambition_platformer2d::rollback::RollbackEntryKind::ResourceEntityMapping
            )
        })
        .map(|d| d.type_name.as_str())
        .collect();
    assert!(
        entity_mapped.len() > 5,
        "only {} entity-mapping registrations found, so the cross-check below \
         would be vacuous",
        entity_mapped.len()
    );

    // NO entity-mapped type may be presence-only.
    //
    // A type holds remappable handles exactly when something registers an entity
    // mapping for it, and every one of those now has a projection through the
    // targets' stable sim identities. So the rule is the invariant itself: a
    // remap that lands on the wrong body must be OBSERVABLE, and a new
    // entity-mapped registration fails here until it is.
    let mut unobservable: Vec<&str> = entity_mapped
        .iter()
        .filter(|name| presence_only.contains(*name))
        .copied()
        .collect();
    unobservable.sort();
    assert!(
        unobservable.is_empty(),
        "{} entity-mapped registration(s) carry only a PRESENCE probe, so a \
         restore can preserve the COUNT while attaching the wrong limb to a \
         slot, the wrong rider to a mount, or the wrong entity to an id — and \
         the localizer reports no difference:\n  {}\n\n\
         Give each a projection through its targets' stable identities \
         (`rollback_component_clone_entity_ref` / `_entity_set`, or \
         `rollback_resource_clone_entity_set` for a resource).",
        unobservable.len(),
        unobservable.join("\n  ")
    );

    let (complete, value, presence) = probes.strength_tally();
    println!(
        "[probe strength] {} probes: {value} value, {complete} complete (zero-sized, \
         presence IS the value), {presence} presence-only ({} of those derived)",
        probes.len(),
        presence_only
            .iter()
            .filter(|name| derived.contains(*name))
            .count()
    );
}

/// Sharpest probe: no armor, no attacks — stand in the striker's path and take
/// repeated hits. Isolates the victim-side damage path under rollback: every
/// hit crosses the staging FIFO, the striker's swing runs its strike volume
/// through GGRS despawn/respawn, and the post-hit clock ramp rewinds. This
/// caught (in order) the unregistered `Collected` latch, the in-flight
/// victim-hit loss (`PendingPlayerHitEvents`), and the strike-volume family
/// living outside the rollback envelope.
#[test]
fn a_player_taking_hp_damage_survives_rollback() {
    let mut sim = oracle_sim();
    let mut last_hp = i32::MAX;
    for frame in 0..600 {
        let enemies = enemy_positions(&mut sim);
        let obs = sim.observation();
        let (px, _) = obs.player_pos;
        if obs.hp != last_hp {
            eprintln!("[hit] frame {frame}: player_hp={} px={px:.1}", obs.hp);
            last_hp = obs.hp;
        }
        let nearest = enemies
            .iter()
            .copied()
            .map(|(x, y)| (x, y, (x - px).abs()))
            .min_by(|a, b| a.2.total_cmp(&b.2));
        let action = match nearest {
            Some((x, _, d)) if d > 10.0 => AgentAction::move_x((x - px).signum()),
            _ => AgentAction::default(),
        };
        sim.step(action);
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("frame {frame}: {error}"));
    }
}

/// Minimal repro probe: kill the patrol enemy, then stand still through its
/// in-place revive and re-aggro. Isolates the death → respawn-timer → revive →
/// re-engage cycle that the full oracle exposed.
#[test]
fn enemy_death_and_inplace_revive_survive_rollback() {
    let mut sim = oracle_sim();
    wear_oracle_armor(&mut sim);
    let mut phase = "approach";
    let mut last_hp = i32::MAX;
    for frame in 0..900 {
        let enemies = enemy_positions(&mut sim);
        let obs = sim.observation();
        let (px, _) = obs.player_pos;
        let nearest = enemies
            .iter()
            .copied()
            .map(|(x, y)| (x, y, (x - px).abs()))
            .min_by(|a, b| a.2.total_cmp(&b.2));
        let (hp, count) = {
            let world = sim.world_mut();
            let mut q = world.query_filtered::<&BodyHealth, Without<ambition_platformer2d::platformer::markers::PrimaryPlayer>>();
            // One pass: this runs every frame for 900 frames, and the two
            // values only feed the change-triggered log line below.
            q.iter(world)
                .fold((0, 0), |(hp, count), b| (hp + b.health.current, count + 1))
        };
        if hp != last_hp {
            eprintln!(
                "[repro] frame {frame}: phase={phase} enemy_hp={hp} enemies={count} px={px:.1}"
            );
            last_hp = hp;
        }
        let action = match (phase, nearest) {
            ("approach", Some((x, _, d))) => {
                if d < 60.0 {
                    phase = "kill";
                }
                AgentAction::move_x((x - px).signum())
            }
            ("kill", Some((x, _, d))) => AgentAction {
                move_x: if d < 30.0 { 0.0 } else { (x - px).signum() },
                attack: frame % 6 == 2,
                ..AgentAction::default()
            },
            _ => AgentAction::default(),
        };
        sim.step(action);
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("frame {frame} (phase {phase}): {error}"));
    }
}

/// Narrowing probe: the lab must be checksum-stable with NO player input at
/// all — only the enemy brains, patrol paths, and feature timers running. A
/// divergence here isolates the fault to the room's autonomous population
/// before the full oracle's combat even starts.
///
/// During development this test carried a five-variant despawn matrix (no_enemies / no_brick /
/// no_switch / no_pickups) plus a print-only pickup census — the bisection tools that cornered
/// the `Collected` latch. Resurrect the matrix from git history if this ever goes red again.
#[test]
fn the_calibration_lab_is_checksum_stable_at_rest() {
    let mut sim = oracle_sim();
    for frame in 0..48 {
        sim.step(AgentAction::default());
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("frame {frame}: {error}"));
    }
}

/// Walk the calibration lab's combat route under a forced rollback window,
/// returning the divergence report instead of panicking so a caller can sweep
/// several worlds and compare which ones diverge.
///
/// `frames_run` and the observed events come back on both paths: a divergence
/// still wants to say what the route had achieved when it hit.
///
/// The fourth return is a per-frame union of unaccounted components. A
/// one-shot sweep at failure time cannot see TRANSIENT sim entities — an attack's
/// hit volume, a projectile, a debris chunk — because they live for a handful of
/// frames and are gone by the time anyone samples. Those are exactly the entities
/// a rewind has to reproduce, so the census walks every frame and unions.
/// The first harness frame on which the simulation advanced and the GGRS
/// session did not, with everything known about the session at that moment.
#[derive(Debug)]
#[allow(dead_code)]
struct GgrsStall {
    frame: usize,
    stats: Option<ambition_platformer2d::rollback::RollbackExecutionStats>,
    session_active: bool,
}

/// What one walk of the route observed. A struct rather than a tuple because
/// the fifth member (the stall) was the one that mattered and a 5-tuple is
/// where a reader stops counting.
struct RouteWalk {
    health: Result<(), String>,
    events: OracleEvents,
    frames_run: usize,
    census: std::collections::BTreeMap<String, usize>,
    stalled_at: Option<GgrsStall>,
}

fn walk_the_combat_route(sim: &mut Platformer2dSimHarness) -> RouteWalk {
    // The props the route must change, with their initial states CHECKED. Anything
    // this cannot find, or finds already done, fails here — before a run that would
    // otherwise report those objectives satisfied by the room's authoring.
    let targets = calibrate_targets(&mut *sim);
    // It belongs on THIS path: the main oracle must never report a melee observation over a
    // room with nothing alive in it. The sweep deliberately removes populations, so it takes
    // the other entry point and owns that risk explicitly (its variant list removes props, not
    // enemies).
    {
        let world = sim.world_mut();
        let mut q = world
            .query_filtered::<&BodyHealth, Without<ambition_platformer2d::platformer::markers::PrimaryPlayer>>();
        let total: i32 = q.iter(world).map(|body| body.health.current).sum();
        assert!(
            total > 0,
            "the calibration lab booted with no live enemies — the melee-hit \
             observation would be vacuous"
        );
    }
    walk_the_combat_route_with(sim, targets)
}

/// Walk the route using targets identified while the intact fixture was
/// calibrated. Population-isolation runs may despawn a target afterward, so
/// target lookups in this path must tolerate absence.
fn walk_the_combat_route_with(
    sim: &mut Platformer2dSimHarness,
    targets: OracleTargets,
) -> RouteWalk {
    let mut census: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let enemy_health_baseline: i32 = {
        let world = sim.world_mut();
        let mut q = world
            .query_filtered::<&BodyHealth, Without<ambition_platformer2d::platformer::markers::PrimaryPlayer>>();
        q.iter(world).map(|body| body.health.current).sum()
    };

    let mut events = OracleEvents {
        melee_landed: false,
        armor_spent: false,
        brick_broken: false,
        switch_flipped: false,
    };

    let mut frames_run = 0usize;
    let mut stalled_at: Option<GgrsStall> = None;
    for frame in 0..MAX_FRAMES {
        let (enemies, brick, switch) = target_positions(&mut *sim, &targets);
        let player = sim.observation();
        let (px, _py) = player.player_pos;

        // The next objective, in route order: take the armor hit from the
        // nearest enemy first, then the brick, then any remaining melee proof,
        // then the switch. The brick outranks enemies once armor is spent
        // because the lab's enemies revive in place — "nearest melee target"
        // forever re-selects the respawned neighbor and the walk never leaves
        // the spawn corner.
        let nearest_enemy = enemies
            .iter()
            .copied()
            .map(|(x, y)| (x, y, (x - px).abs()))
            .min_by(|a, b| a.2.total_cmp(&b.2));
        // Is the brick the current objective? Kept as its own flag because two
        // parts of the action below depend on it, and re-deriving the condition in
        // both is how they drift apart.
        let breaking_brick = events.armor_spent && !events.brick_broken && brick.is_some();
        let target_x = if events.switch_flipped {
            px
        } else if !events.armor_spent {
            nearest_enemy.map(|(x, _, _)| x).unwrap_or(px)
        } else if breaking_brick {
            brick.map(|b| brick_standoff(b, px)).unwrap_or(px)
        } else if !events.melee_landed {
            nearest_enemy.map(|(x, _, _)| x).unwrap_or(px)
        } else if let Some((x, _)) = switch {
            x
        } else {
            px
        };

        let dx = target_x - px;
        let near = dx.abs() < 70.0;
        // Until the armor row is spent, walk INTO the target without swinging —
        // the point is to TAKE a hit, and a policy that kills everything first
        // never exercises the equipment path.
        let brawling = events.armor_spent;
        let action = AgentAction {
            move_x: if dx.abs() < 8.0 { 0.0 } else { dx.signum() },
            // Melee in reach; the moveset faces along move_x.
            attack: brawling && near && frame % 6 == 2,
            // Interact pulses flip the switch once the player stands in its
            // region; harmless elsewhere (single-press Up never triggers).
            interact: near && frame % 10 == 5,
            // An occasional hop un-sticks the walk against bodies and debris —
            // but NEVER while the brick is the objective. The brick's top face
            // stands 32 above the floor, well within one hop, and a walker that
            // lands on top of it is a walker whose forward strike sweeps the air
            // above the block forever. This route spent its entire existence up
            // there (A16): standing at x=900 on a brick centred at (904, 728),
            // swinging, and reporting no break.
            jump: !breaking_brick && frame % 90 == 40,
            jump_held: !breaking_brick && frame % 90 >= 40 && frame % 90 < 48,
            ..AgentAction::default()
        };

        let advances_before = sim
            .rollback_execution_stats()
            .map(|stats| stats.advance_runs)
            .unwrap_or(0);
        sim.step(action);
        // Did the step actually DRIVE the rollback session?
        //
        // A harness step advances `SimTick` whether or not GGRS is running, so a
        // session that stops being driven is invisible: the route keeps walking,
        // the checksums keep agreeing (with nothing), and the only thing that
        // notices is the `advance_runs > frames_run` assert 600 frames later —
        // which reports a ratio and cannot say when or why. That is exactly how
        // AC18 was found and exactly why its mechanism went unestablished.
        //
        // Record the FIRST frame where the sim advanced and GGRS did not. It is
        // not fatal here — the run continues so the events and the census still
        // report — but the final assertion can now name the frame.
        if stalled_at.is_none() && sim.rollback_enabled() {
            let advances_after = sim
                .rollback_execution_stats()
                .map(|stats| stats.advance_runs)
                .unwrap_or(0);
            if advances_after == advances_before {
                stalled_at = Some(GgrsStall {
                    frame,
                    stats: sim.rollback_execution_stats(),
                    session_active: sim.rollback_status().is_some(),
                });
            }
        }
        for (name, count) in crate::rollback_coverage::unaccounted_components(sim) {
            let seen = census.entry(name).or_default();
            *seen = (*seen).max(count);
        }
        if let Err(error) = sim.rollback_health() {
            let late = crate::rollback_coverage::unaccounted_components(sim);
            let report = format!(
                "frame {frame}: resimulation diverged: {error} \
                 (events at failure: melee={} armor={} brick={} switch={}, px={px:.1}, target_x={target_x:.1})\n\
                 unaccounted components at failure (candidates inserted mid-run): {late:?}",
                events.melee_landed, events.armor_spent, events.brick_broken, events.switch_flipped
            );
            return RouteWalk {
                health: Err(report),
                events,
                frames_run: frame + 1,
                census,
                stalled_at,
            };
        }
        let before = (
            events.melee_landed,
            events.armor_spent,
            events.brick_broken,
            events.switch_flipped,
        );
        observe(sim, &targets, enemy_health_baseline, &mut events);
        let after = (
            events.melee_landed,
            events.armor_spent,
            events.brick_broken,
            events.switch_flipped,
        );
        if before != after {
            eprintln!(
                "[oracle] frame {frame}: events now melee={} armor={} brick={} switch={}",
                after.0, after.1, after.2, after.3
            );
        }
        frames_run = frame + 1;
        if events.all() && frames_run >= MIN_FRAMES {
            break;
        }
    }
    RouteWalk {
        health: Ok(()),
        events,
        frames_run,
        census,
        stalled_at,
    }
}

/// Exercise melee, armor, a breakable, and a switch through forced rollback.
/// All four effects must occur while the replay remains checksum-identical.
#[test]
fn combat_equipment_switch_and_breakable_survive_forced_rollback_identically() {
    let mut sim = oracle_sim();
    wear_oracle_armor(&mut sim);
    stage_player_on_arena_floor(&mut sim);

    // The session count AFTER staging, because staging deliberately rebases:
    // both setup helpers fold a `world_mut` mutation into frame zero, and each
    // fold installs a session. Counting from here is what makes the pin below a
    // statement about the WALK rather than about how the baseline was built.
    let sessions_at_the_start = sim
        .rollback_execution_stats()
        .expect("GGRS instrumentation is installed")
        .sessions_installed;

    let RouteWalk {
        health,
        events,
        frames_run,
        census,
        stalled_at,
    } = walk_the_combat_route(&mut sim);
    assert!(
        census.is_empty(),
        "state lived on a simulated entity at some point during the route that \
         GGRS will not rewind. These were invisible to the one-shot sweep in \
         `rollback_coverage` because they are TRANSIENT — spawned and despawned \
         inside the route — which is why this census samples every frame:\n{census:#?}"
    );
    health.unwrap_or_else(|report| panic!("{report}"));

    assert!(
        events.melee_landed,
        "no melee hit landed in {frames_run} frames — the oracle never \
         exercised combat state, so its checksum agreement proves nothing"
    );
    assert!(
        events.armor_spent,
        "the armor row was never consumed in {frames_run} frames — the oracle \
         never exercised equipment state"
    );
    // The route must physically exercise both targets. Stop clear of the brick
    // face and suppress hopping while it is the objective so horizontal attacks
    // can connect before proceeding to the switch.
    assert!(
        events.brick_broken,
        "the brick was never broken in {frames_run} frames — Track 0's exit \
         criterion names it explicitly, and breakable state is registered rollback \
         state that nothing else in this suite exercises inside a rewind window"
    );
    assert!(
        events.switch_flipped,
        "the switch was never flipped in {frames_run} frames — the walker either \
         never reached x≈1132 or its interact pulses did not land"
    );

    // A step that advanced the sim and not GGRS.
    assert!(
        stalled_at.is_none(),
        "the simulation advanced while the GGRS session did not: {:?}\n\
         Every frame after this one ran with nothing rewinding it, so the \
         checksum agreement above covers only the frames before it.",
        stalled_at
    );

    let stats = sim
        .rollback_execution_stats()
        .expect("GGRS instrumentation is installed");
    // LIFETIME, not per-session, and that distinction is the whole of AC18.
    //
    // This route can produce a confirmed Track-B lifecycle commit, which rebases
    // the GGRS session — atomically, deliberately, correctly. A rebase installs
    // a new session, and a new session's frame numbering (and therefore these
    // per-session counters) starts at zero.
    //
    // The content coupling AC18 filed was real, but not where it looked: the
    // authored enemy set decides whether a lifecycle op commits inside the
    // window, and the assertion was only valid for runs where none did.
    assert!(
        stats.lifetime_load_runs > 0,
        "no LoadWorld request was ever issued, so nothing was rewound and the \
         checksum agreement above is agreement with itself: {stats:?}"
    );
    assert!(
        stats.lifetime_advance_runs > frames_run as u64,
        "resimulation must execute more GGRS frames than the {frames_run} \
         harness steps, or the same frames were never replayed: {stats:?}"
    );

    // This oracle proves checksum identity over the no-rebase walk. Crossed-rebase
    // behavior is covered by `rollback_room_transition`. Session count is reported
    // with a small ceiling rather than pinned exactly because feel tuning may shift a
    // lifecycle commit without changing this oracle's invariant.
    let further = stats.sessions_installed as i64 - sessions_at_the_start as i64;
    println!(
        "[oracle-coverage] the walk installed {further} further session(s) on top \
         of the {sessions_at_the_start} the setup built. Reported, not pinned — \
         see the note above. Full stats: {stats:?}"
    );
    assert!(
        (0..=2).contains(&further),
        "this oracle's coverage RAN AWAY: {further} further sessions on top of \
         {sessions_at_the_start}. A drift of one is feel tuning changing how long \
         the route takes; this is a route that has stopped being the route. \
         Full stats: {stats:?}"
    );
}

/// Which population does the divergence need? — the localizer, opt-in.
///
/// When the oracle above goes red it names a frame and nothing else: a GGRS sync-test reports
/// one aggregate checksum, so "frames [149, 150, 151] differ" is the whole story it can tell. A
/// variant that goes green names the class the divergence needs, which is the question the
/// aggregate checksum cannot answer and the one a fix has to start from.
///
/// `#[ignore]` because it boots five sims and re-walks a ~150-frame route in
/// each. It is a bisection tool, not a standing guard — the oracle is the guard.
/// Run it with `./run_tests.sh --heavy -k which_population`.
#[test]
#[ignore = "diagnostic bisection: five sim boots; run when the oracle above is red"]
fn which_population_does_the_rollback_divergence_need() {
    // No `no_enemies` variant: the route SPENDS ARMOR by taking an enemy hit, so
    // a world without enemies cannot walk it at all (the helper's own vacuity
    // guard says so). The removable classes are the ones the route passes but
    // does not depend on.
    let mut findings: Vec<String> = Vec::new();
    for variant in ["intact", "no_brick", "no_switch", "no_pickups"] {
        let mut sim = oracle_sim();
        wear_oracle_armor(&mut sim);
        stage_player_on_arena_floor(&mut sim);
        // calibrate the INTACT world, before anything is removed. See
        // `walk_the_combat_route_with`.
        let targets = calibrate_targets(&mut sim);
        {
            let world = sim.world_mut();
            let doomed: Vec<Entity> = match variant {
                // NOT the player: the route needs a body to drive.
                "no_enemies" => {
                    let mut q = world.query_filtered::<Entity, (
                        With<BodyHealth>,
                        Without<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
                    )>();
                    q.iter(world).collect()
                }
                "no_brick" => {
                    let mut q = world
                        .query_filtered::<Entity, With<ambition_platformer2d::combat::components::BreakableFeature>>();
                    q.iter(world).collect()
                }
                "no_switch" => {
                    let mut q = world
                        .query_filtered::<Entity, With<ambition_platformer2d::encounter::switches::SwitchFeature>>(
                        );
                    q.iter(world).collect()
                }
                "no_pickups" => {
                    let mut q = world
                        .query_filtered::<Entity, With<ambition_platformer2d::combat::components::PickupFeature>>();
                    q.iter(world).collect()
                }
                _ => Vec::new(),
            };
            for entity in doomed {
                world.despawn(entity);
            }
        }
        // The despawns are setup, not gameplay: they become frame-0 state, or
        // GGRS would rewind INTO a world that still had them.
        sim.rebase_rollback_history()
            .expect("variant despawn setup becomes the rollback baseline");

        let RouteWalk {
            health,
            events,
            frames_run,
            census,
            stalled_at,
        } = walk_the_combat_route_with(&mut sim, targets);
        if !census.is_empty() {
            findings.push(format!("  {variant:<12} TRANSIENT UNACCOUNTED: {census:?}"));
        }
        if let Some(stall) = stalled_at {
            findings.push(format!("  {variant:<12} GGRS STALLED: {stall:?}"));
        }
        match health {
            Ok(()) => findings.push(format!(
                "  {variant:<12} CLEAN over {frames_run} frames \
                 (melee={} armor={} brick={} switch={})",
                events.melee_landed, events.armor_spent, events.brick_broken, events.switch_flipped
            )),
            Err(report) => findings.push(format!("  {variant:<12} DIVERGED — {report}")),
        }
    }
    // THIS PANICKED UNCONDITIONALLY, and it made `--heavy` structurally
    // red. `run_tests.py`'s heavy pass runs `--include-ignored`, so the ONLY
    // mode that executes ignored tests contained a guaranteed failure — which
    // turns its output into noise, which is precisely why three rotted
    // `#[ignore]`s sat unnoticed for weeks behind it. A suite mode nobody can
    // read is a suite mode nobody reads.
    //
    // The panic was there to force output past libtest's capture. Its siblings
    // (`list_what_every_waiver_actually_covers`, `probe_what_a_rollback_frame_costs`,
    // `list_what_each_character_derives_for_its_body`) print and PASS, and are
    // read with `--nocapture` — which is how you run a diagnostic on purpose
    // anyway. This one is now the same shape as the rest.
    println!(
        "rollback divergence population sweep (run with --nocapture; this test \
         reports and passes):\n{}",
        findings.join("\n")
    );
}

/// Which COMPONENT does the divergence live in? — per-component localization.
///
/// The sibling localizer above answers "which entity class", by bisection over
/// five sim boots. This answers the sharper question directly, in one run: for
/// every registered rollback component, census its checksum projection when GGRS
/// saves a frame, and census it again when GGRS loads that same frame. A component
/// whose census changed did not survive its own snapshot, and it is named.
///
/// This is the tool the triage doc ends by asking for. Two things it deliberately
/// does NOT do:
///
/// * it does not compare two independent runs — that reproduces the aggregate
///   checksum's blindness with more machinery;
/// * it does not fold per-entity checksums in iteration order. bevy_ggrs destroys
///   and recreates rollback entities, so ids and archetype order both change across
///   a load; an order-dependent fold would report every component as diverging.
///   XOR plus a count is invariant under reordering and still catches a changed
///   value, a lost carrier, or a gained one.
///
/// `#[ignore]` for cost, like its sibling: it censuses every registered type on
/// every save and every load. Run it with
/// `./run_tests.sh --heavy -k which_component`.
#[test]
#[ignore = "diagnostic: per-component restore census on every save/load; run when the oracle is red"]
fn which_component_does_the_rollback_divergence_live_in() {
    let mut sim = oracle_sim();
    sim.world_mut()
        .insert_resource(ambition_platformer2d::rollback::RollbackRestoreAudit::enabled());
    wear_oracle_armor(&mut sim);
    stage_player_on_arena_floor(&mut sim);

    let probes = sim
        .world()
        .resource::<ambition_platformer2d::rollback::RollbackChecksumProbes>()
        .len();
    assert!(
        probes > 0,
        "no localization probes were registered, so this test can only ever \
         report success — the probe registration is coupled to the checksum \
         registration precisely so that cannot happen silently"
    );

    let _ = walk_the_combat_route(&mut sim);

    let audit = sim
        .world()
        .resource::<ambition_platformer2d::rollback::RollbackRestoreAudit>();
    // Vacuity guard FIRST. A localizer that reports "nothing diverged" while never
    // comparing anything launders an absence of evidence into evidence of absence,
    // which is the single most useless thing a diagnostic can do.
    assert!(
        audit.comparisons > 0 && audit.resimulations > 0,
        "the audit compared nothing, so its verdict is meaningless: {}",
        audit.coverage()
    );
    assert!(
        audit.divergences.is_empty(),
        "{} registered component(s) did not survive their own snapshot across \
         {probes} probed types. THIS IS THE ANSWER the aggregate checksum could \
         not give:\n{}",
        audit.diverging_types().len(),
        audit.report()
    );
    // Report coverage on success too: the useful negative result is "N frames were
    // compared and every registered component came back identical", not "no
    // assertion fired".
    println!("[localizer] {}", audit.coverage());
}

/// A14: every gameplay message channel is either rewound or named.
///
/// A `MessageReader<T>` reads through a `Local<MessageCursor<T>>`. `Local` is system
/// state, GGRS does not rewind it, and so any gameplay fact derived from `.read()`
/// is non-deterministic across a rollback BY CONSTRUCTION: after a load the reader
/// resumes from wherever the abandoned future left its cursor, and either re-reads
/// messages it already consumed or skips ones it has not.
///
/// So: enumerate. Every `Messages<T>` resource the composed sim carries is compared
/// against the registered clears, and anything unregistered must appear below with a
/// reason. Same discipline as the probe-strength guard — a hazard that has to be
/// written down is a different thing from one that has to be remembered.
#[test]
fn every_gameplay_message_channel_is_rewound_on_rollback_or_named() {
    // (exact message type name, why an un-rewound cursor is harmless for it)
    //
    // Every entry says the same kind of thing — the channel is read OUTSIDE the
    // rollback schedule — and that is a stated premise, not something this test can
    // verify: Bevy does not expose which schedule a `MessageReader` param belongs
    // to. What the test does enforce is that the list stays short and deliberate.
    // A new gameplay channel lands here as a failure, and its author has to decide
    // rather than inherit silence.
    const NOT_REWOUND: &[(&str, &str)] = &[
        // sixteen are answered STRUCTURALLY rather than one at a time. The
        // checker asks whether a stale reader cursor can change the simulation. A
        // sim crate that cannot NAME the message type cannot hold a cursor into
        // its channel, and the dependency graph settles that without reading a
        // single system: nothing under `ambition_platformer2d_*` depends on
        // `ambition_game_shell` or `ambition_load_presentation`. The two that the
        // graph does not settle — `ambition_audio` and `ambition_shared_tangle`,
        // both real sim dependencies — were checked by hand and say so below.
        (
            "ambition_audio::selection::AudioContextChanged",
            "the shell audio owner changed. Its ONE reader is          `reset_audio_request_state_on_context_change`, registered in literal `Update`          (verified 2026-08-06) — the only channel in this K2b batch that needed          checking rather than a dependency-graph argument, because the sim does depend          on `ambition_audio`",
        ),
        (
            "ambition_game_shell::abandon::ShellAbandonRequested",
            "the pause menu's contributed row was picked. Same STRUCTURAL argument as the          `ambition_game_shell` family below, and it holds for the same reason: no simulation          crate depends on `ambition_game_shell`, so the sim cannot name this type and no          cursor of its can exist inside the GGRS schedule. Its one reader is Smash's          `abandon_the_match_when_the_shell_asks`, in literal `Update`, which translates it          into `MatchAbandonRequest`, a match-scoped latch the sim reads — deliberately NOT a          rewound channel, because an ask made outside the simulation cannot be re-made by a          resimulation",
        ),
        (
            "ambition_game_shell::launcher::ShellLauncherCommand",
            "the SHELL's own lifecycle channel. ⭐ STRUCTURAL, not a judgement call: no          simulation crate depends on `ambition_game_shell` at all (checked in Cargo.toml,          2026-08-06), so the sim cannot name this type, cannot construct a reader for it,          and no cursor of its can exist inside the GGRS schedule. It arrived on this list          with K2b edit 2, when the harness stopped composing the simulation plugin alone          and started composing the host a player runs",
        ),
        (
            "ambition_game_shell::router::ShellCommand",
            "the SHELL's own lifecycle channel. ⭐ STRUCTURAL, not a judgement call: no          simulation crate depends on `ambition_game_shell` at all (checked in Cargo.toml,          2026-08-06), so the sim cannot name this type, cannot construct a reader for it,          and no cursor of its can exist inside the GGRS schedule. It arrived on this list          with K2b edit 2, when the harness stopped composing the simulation plugin alone          and started composing the host a player runs",
        ),
        (
            "ambition_game_shell::router::ShellEvent",
            "the SHELL's own lifecycle channel. ⭐ STRUCTURAL, not a judgement call: no          simulation crate depends on `ambition_game_shell` at all (checked in Cargo.toml,          2026-08-06), so the sim cannot name this type, cannot construct a reader for it,          and no cursor of its can exist inside the GGRS schedule. It arrived on this list          with K2b edit 2, when the harness stopped composing the simulation plugin alone          and started composing the host a player runs",
        ),
        (
            "ambition_game_shell::sequence::ShellSequenceCommand",
            "the SHELL's own lifecycle channel. ⭐ STRUCTURAL, not a judgement call: no          simulation crate depends on `ambition_game_shell` at all (checked in Cargo.toml,          2026-08-06), so the sim cannot name this type, cannot construct a reader for it,          and no cursor of its can exist inside the GGRS schedule. It arrived on this list          with K2b edit 2, when the harness stopped composing the simulation plugin alone          and started composing the host a player runs",
        ),
        (
            "ambition_game_shell::session::GameplaySessionEvent",
            "the SHELL's own lifecycle channel. ⭐ STRUCTURAL, not a judgement call: no          simulation crate depends on `ambition_game_shell` at all (checked in Cargo.toml,          2026-08-06), so the sim cannot name this type, cannot construct a reader for it,          and no cursor of its can exist inside the GGRS schedule. It arrived on this list          with K2b edit 2, when the harness stopped composing the simulation plugin alone          and started composing the host a player runs",
        ),
        (
            "ambition_load_presentation::model::LoadActivitySignal",
            "the loading PRESENTATION channel, and the same structural argument as the          `ambition_game_shell` family above: no simulation crate depends on          `ambition_load_presentation` (checked 2026-08-06), so no sim reader can exist",
        ),
        (
            "ambition_load_presentation::model::LoadPresentationAction",
            "the loading PRESENTATION channel, and the same structural argument as the          `ambition_game_shell` family above: no simulation crate depends on          `ambition_load_presentation` (checked 2026-08-06), so no sim reader can exist",
        ),
        (
            "ambition_load_presentation::model::LoadPresentationCommand",
            "the loading PRESENTATION channel, and the same structural argument as the          `ambition_game_shell` family above: no simulation crate depends on          `ambition_load_presentation` (checked 2026-08-06), so no sim reader can exist",
        ),
        (
            "ambition_load_presentation::model::LoadPresentationEvent",
            "the loading PRESENTATION channel, and the same structural argument as the          `ambition_game_shell` family above: no simulation crate depends on          `ambition_load_presentation` (checked 2026-08-06), so no sim reader can exist",
        ),
        (
            "ambition_menu::MenuActionActivated<ambition_game_shell::basic_presentation::BasicLauncherAction>",
            "a menu channel whose payload type lives in `ambition_game_shell`. The sim does          depend on `ambition_menu` — but not on the crate that owns this generic          argument, so it cannot name THIS instantiation and cannot hold a cursor into it",
        ),
        (
            "ambition_menu::MenuActionActivated<ambition_game_shell::basic_presentation::ShellCardAction>",
            "a menu channel whose payload type lives in `ambition_game_shell`. The sim does          depend on `ambition_menu` — but not on the crate that owns this generic          argument, so it cannot name THIS instantiation and cannot hold a cursor into it",
        ),
        (
            "ambition_menu::MenuActionActivated<ambition_game_shell::pause_menu::PauseEntry>",
            "a menu channel whose payload type lives in `ambition_game_shell`. The sim does          depend on `ambition_menu` — but not on the crate that owns this generic          argument, so it cannot name THIS instantiation and cannot hold a cursor into it",
        ),
        (
            "ambition_menu::MenuActionPreviewed<ambition_game_shell::basic_presentation::BasicLauncherAction>",
            "a menu channel whose payload type lives in `ambition_game_shell`. The sim does          depend on `ambition_menu` — but not on the crate that owns this generic          argument, so it cannot name THIS instantiation and cannot hold a cursor into it",
        ),
        (
            "ambition_menu::MenuActionPreviewed<ambition_game_shell::basic_presentation::ShellCardAction>",
            "a menu channel whose payload type lives in `ambition_game_shell`. The sim does          depend on `ambition_menu` — but not on the crate that owns this generic          argument, so it cannot name THIS instantiation and cannot hold a cursor into it",
        ),
        (
            "ambition_menu::MenuActionPreviewed<ambition_game_shell::pause_menu::PauseEntry>",
            "a menu channel whose payload type lives in `ambition_game_shell`. The sim does          depend on `ambition_menu` — but not on the crate that owns this generic          argument, so it cannot name THIS instantiation and cannot hold a cursor into it",
        ),
        (
            "ambition_platformer2d_shared_tangle::lifecycle::session::SessionScopeRetired",
            "a session scope ENDED. Unlike the rest of this batch it lives in a sim crate, so          it got the individual check: its sole writer is          `translate_shell_session_lifecycle`, registered in literal `Update` (verified          2026-08-06) — the same system and schedule that make the `ActiveSessionScope`          waiver hold, so the two go stale together",
        ),

        (
            "ambition_load::plugin::LoadCommand",
            "asset-loading orchestration; `apply_load_commands` runs in Update, not              the GGRS schedule",
        ),
        (
            "ambition_load::plugin::LoadEvent",
            "the same coordinator's outbound channel, read by the loading UI",
        ),
        (
            "ambition_platformer2d_shared_tangle::developer_hotkeys::DeveloperAction",
            "developer hotkeys: shell, trace and debug-viz consumers, none in the sim",
        ),
        (
            "bevy_asset::event::AssetEvent<ambition_platformer2d_actor_monolith::session::data::Platformer2dGameplayDefaults>",
            "bevy's asset lifecycle, delivered on the frame clock",
        ),
        (
            "bevy_asset::event::AssetLoadFailedEvent<ambition_platformer2d_actor_monolith::session::data::Platformer2dGameplayDefaults>",
            "bevy's asset lifecycle, delivered on the frame clock",
        ),
        (
            "bevy_state::state::transitions::StateTransitionEvent<ambition_platformer2d_shared_tangle::schedule::GameMode>",
            "bevy's state machinery; a mode transition is a frame-level fact the sim              never reads",
        ),
        (
            "ambition_platformer2d_shared_tangle::block_nudge::BlockStruck",
            "the flinch is PRESENTATION and has exactly one reader: \
             `flinch_struck_blocks`, registered in `Update` in the render plugin, \
             which writes only a `Transform` on a `BlockVisual` and a `BlockFlinch` \
             that nothing outside presentation reads, on the WALL clock. A stale \
             cursor can make a block flinch twice or not at all; it cannot reach \
             the simulation, because the block's geometry is authoritative and \
             static by design (see `block_nudge`'s module doc: moving the box \
             would lift a body standing on it and give a rollback an animation to \
             rewind). ⚠ this argument is UNCONDITIONAL rather than \
             host-conditional: under a `RenderFrame` host `Update` would be the sim \
             schedule, but that host has no GGRS at all, so there is no rollback \
             for a cursor to be stale across",
        ),
    ];

    let mut sim = oracle_sim();
    let registered: std::collections::BTreeSet<String> = sim
        .world()
        .resource::<ambition_platformer2d::rollback::RollbackRegistry>()
        .descriptors()
        .filter(|d| d.kind == ambition_platformer2d::rollback::RollbackEntryKind::MessageClear)
        .map(|d| d.type_name.clone())
        .collect();

    // Every live message channel, by the payload type inside `Messages<T>`.
    let world = sim.world_mut();
    let mut channels: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for (info, _) in world.iter_resources() {
        let name = info.name().to_string();
        let Some(inner) = name
            .strip_prefix("bevy_ecs::message::messages::Messages<")
            .and_then(|rest| rest.strip_suffix('>'))
        else {
            continue;
        };
        // Only OUR channels: bevy's own (window events, asset events) are not
        // gameplay facts and are not read by the sim schedule.
        if inner.contains("ambition") {
            channels.insert(inner.to_string());
        }
    }
    assert!(
        channels.len() > 25,
        "only {} ambition message channels found — the scan is not seeing the \
         composed sim's channels and the comparison below would be vacuous",
        channels.len()
    );

    let named: std::collections::BTreeSet<&str> =
        NOT_REWOUND.iter().map(|(name, _)| *name).collect();
    let mut unhandled: Vec<&str> = channels
        .iter()
        .filter(|name| !registered.contains(*name))
        .filter(|name| !named.contains(name.as_str()))
        .map(String::as_str)
        .collect();
    unhandled.sort();
    // And the reverse, so the list cannot outlive what it describes: an entry for a
    // channel that has since been registered, or that no longer exists, is removed.
    let mut stale: Vec<&str> = named
        .iter()
        .filter(|name| registered.contains(**name) || !channels.contains(**name))
        .copied()
        .collect();
    stale.sort();
    assert!(
        stale.is_empty(),
        "these entries no longer describe an un-rewound channel:\n  {}",
        stale.join("\n  ")
    );
    assert!(
        unhandled.is_empty(),
        "{} message channel(s) are neither cleared on rollback nor named here. A \
         reader's cursor is `Local` state GGRS never rewinds, so after a load it \
         resumes wherever an abandoned future left it — re-reading consumed \
         messages or skipping unread ones. Either register \
         `clear_message_on_rollback::<T>` or add an entry saying why a stale cursor \
         cannot change the simulation:\n  {}",
        unhandled.len(),
        unhandled.join("\n  ")
    );
    println!(
        "[message channels] {} ambition channels, {} cleared on rollback",
        channels.len(),
        registered.len()
    );
}
