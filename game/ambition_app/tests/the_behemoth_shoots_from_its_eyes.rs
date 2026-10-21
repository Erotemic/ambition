//! The Smirking Behemoth's eye beam, in its own arena.
//!
//! The road, with nothing injected: the boss pattern telegraphs and strikes
//! `Special("eye_beam")` -> the boss domain's trigger adapter -> the host runs
//! `ambition_content_modules::eye_beam` -> the projectile domain's request
//! adapter -> bolts in flight, from the eye, toward the player.
//!
//! WHAT WAS WRONG (2026-10-13, Jon: "the eyes just glow and nothing happens").
//! The simulation did fire. Three things made that invisible. The boss did
//! not attack while it walked, so it shot only at a player that touched it,
//! across its own body. The shots left from the top of the body, not the eye.
//! And they had no look, so the game drew the engine's plain orange boxes.

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_platformer2d::boss_encounter::BossConfig;
use ambition_platformer2d::combat::events::{HitEvent, HitSource, HitTarget};
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use ambition_platformer2d::projectiles::spawn_request::{ProjectilePresentation, ProjectileSpawnRequest};
use ambition_platformer2d::projectiles::ProjectileVisualCatalog;
use bevy::ecs::message::Messages;
use bevy::prelude::{Entity, With};

const CUT_ROPE_ROOM: &str = "you_have_to_cut_the_rope";
/// The two places the player stands, on the open floor at each end of the
/// arena. The left one is out from behind the pillar (its right face is
/// x = 272). The map puts the boss at x = 888.
const PLAYER_PLACES: [f32; 2] = [300.0, 1160.0];
/// The player keeps away: when the boss is this near, the player goes to the
/// place that is farther from it.
const KEEP_AWAY: f32 = 80.0;
/// Long enough for several strikes. Measured: the boss needs 137 draws of its
/// idle chance before its first telegraph, so the first strike is at tick 326.
const TICKS: usize = 1500;

/// One strike, as the test saw it.
struct Strike {
    /// The space between the boss's body and the player's, along x.
    gap: f32,
    /// Where each bolt started, from the centre of the boss's body, with x
    /// along the way the boss faces.
    muzzles: Vec<(f32, f32)>,
    /// The look each bolt asked for.
    looks: Vec<String>,
}

fn arena() -> (Platformer2dSimHarness, Entity, Entity) {
    let opts = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        // Required: the tolerant road falls back to the authored start room
        // and would run this test in the wrong room, green.
        .with_required_start_room(CUT_ROPE_ROOM);
    let mut sim = Platformer2dSimHarness::new_with_options(opts).expect("the cut-rope room builds headlessly");
    let world = sim.world_mut();
    let boss = {
        let mut q = world.query_filtered::<Entity, With<BossConfig>>();
        q.iter(world).next().expect("the arena has its boss")
    };
    let player = {
        let mut q = world.query_filtered::<(Entity, &mut BodyKinematics), PrimaryPlayerOnly>();
        let (player, mut kin) = q.single_mut(world).expect("primary player exists");
        kin.pos.x = PLAYER_PLACES[0];
        player
    };
    (sim, boss, player)
}

/// Run the fight with a player that keeps away from the boss and does not
/// dodge. The strikes the boss made, and how many of its bolts hit the player.
fn fight(sim: &mut Platformer2dSimHarness, boss: Entity, player: Entity) -> (Vec<Strike>, usize) {
    let mut requests = sim.world().resource::<Messages<ProjectileSpawnRequest>>().get_cursor();
    let mut hits = sim.world().resource::<Messages<HitEvent>>().get_cursor();
    let (mut strikes, mut landed) = (Vec::new(), 0);
    for _ in 0..TICKS {
        sim.step(AgentAction::default());
        let made: Vec<(bevy::math::Vec2, String)> = requests
            .read(sim.world().resource::<Messages<ProjectileSpawnRequest>>())
            .filter(|request| request.owner == boss)
            .map(|request| {
                let look = match &request.presentation {
                    ProjectilePresentation::OpenVisual(id) => id.clone(),
                    ProjectilePresentation::NamedKind(kind) => kind.visual_id().to_string(),
                    ProjectilePresentation::NamedCharged(kind, tier) => kind.visual_id_at(*tier).to_string(),
                };
                (request.projectile.body.kin.pos, look)
            })
            .collect();
        landed += hits
            .read(sim.world().resource::<Messages<HitEvent>>())
            .filter(|hit| {
                hit.attacker == Some(boss)
                    && hit.source == HitSource::Projectile
                    && hit.target == HitTarget::Body(player)
            })
            .count();
        let world = sim.world_mut();
        let boss_kin = world.get::<BodyKinematics>(boss).expect("the boss has a body").clone();
        let mut player_kin = world.get_mut::<BodyKinematics>(player).expect("the player has a body");
        let gap = (boss_kin.pos.x - player_kin.pos.x).abs() - (boss_kin.size.x + player_kin.size.x) * 0.5;
        if made.is_empty() {
            if gap < KEEP_AWAY {
                let far = PLAYER_PLACES
                    .into_iter()
                    .max_by(|a, b| (a - boss_kin.pos.x).abs().total_cmp(&(b - boss_kin.pos.x).abs()))
                    .expect("there are two places");
                player_kin.pos.x = far;
                player_kin.vel = bevy::math::Vec2::ZERO;
            }
            continue;
        }
        strikes.push(Strike {
            gap,
            muzzles: made
                .iter()
                .map(|(pos, _)| ((pos.x - boss_kin.pos.x) * boss_kin.facing.signum(), pos.y - boss_kin.pos.y))
                .collect(),
            looks: made.into_iter().map(|(_, look)| look).collect(),
        });
    }
    (strikes, landed)
}

/// The boss shoots at a player that keeps away from it, and the bolts get
/// there. A boss that shoots only once it has arrived never shoots this
/// player at all.
#[test]
fn the_behemoth_shoots_a_player_that_keeps_away() {
    let (mut sim, boss, player) = arena();
    let (strikes, landed) = fight(&mut sim, boss, player);
    let gaps: Vec<f32> = strikes.iter().map(|strike| strike.gap).collect();
    assert!(
        gaps.iter().any(|gap| *gap > KEEP_AWAY),
        "in {TICKS} ticks the boss fired no beam at a player it had not reached; the space between \
         them at each strike: {gaps:?}"
    );
    assert!(landed > 0, "no bolt of {} strikes reached a player that does not dodge", strikes.len());
}

/// The first bolt of a strike starts at the eye: on the side of the body the
/// boss faces, above its middle. (The others are ahead of it, on the line.)
#[test]
fn the_beam_leaves_from_the_eye() {
    let (mut sim, boss, player) = arena();
    let (strikes, _) = fight(&mut sim, boss, player);
    assert!(!strikes.is_empty(), "premise: the boss shot");
    for strike in &strikes {
        let (forward, up) = strike.muzzles[0];
        assert!(
            forward > 20.0 && up < -20.0,
            "a beam started {forward} px ahead of the middle of the body and {up} px under it; the \
             eye is ahead and above"
        );
    }
}

/// Each bolt asks for a look the game has. With no look, or one the catalog
/// does not have, the game draws the engine's plain orange box.
#[test]
fn the_bolts_of_the_beam_have_a_look() {
    let (mut sim, boss, player) = arena();
    let (strikes, _) = fight(&mut sim, boss, player);
    assert!(!strikes.is_empty(), "premise: the boss shot");
    let catalog = sim.world().resource::<ProjectileVisualCatalog>();
    for look in strikes.iter().flat_map(|strike| &strike.looks) {
        assert!(catalog.get(look).is_some(), "a bolt asked for the look {look:?}, which the game does not have");
    }
}
