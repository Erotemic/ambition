//! Mary-O's crony — a stompable walker, authored as pure content.
//!
//! The M4 enemy leg (`docs/planning/demos/super-mary-o.md`). It exercises the
//! actors-vs-props taxonomy and the classic head-stomp on the finished engine
//! face, with **zero engine edits**:
//!
//! - **Body + walk + contact damage** come from a demo-owned roster archetype
//!   (`mary_o_crony`, a 1-HP `Wanderer` that paces and reverses at walls). Its
//!   `body_contact_damage`/`attacks_player` default true, so a side touch hurts
//!   Mary-O through the ONE shared body-contact-damage path — no bespoke code.
//! - **The sprite** is the published `ai_slop` sheet, resolved by a demo catalog
//!   row under a unique display name ("Mary-O Crony" — Ambition owns "Ai Slop",
//!   and catalog assembly rejects a duplicate) so it renders standalone AND hosted.
//! - **The stomp** is a demo RULE, not the engine's attack-hitbox pogo (Jon:
//!   Mary-O does not pogo; she *bounces* on enemies to squash them). A player
//!   descending onto a crony's head bounces up and squashes it.
//!
//! Every type it names comes through the `ambition` umbrella — the E9 oracle.

use bevy::prelude::*;

use ambition::actors::actor::{PlayerEntity, PrimaryPlayer};
use ambition::actors::combat::components::ActorFaction;
use ambition::actors::features::FeatureName;
use ambition::actors::features::{SpawnActorKind, SpawnActorRequest};
use ambition::characters::actor::BodyHealth;
use ambition::engine_core as ae;
use ambition::entity_catalog::placements::CharacterBrain;

use crate::{LEVEL_1_1_ROOM_ID, T};

/// The catalog `display_name` the crony renders from, and the name every crony
/// spawn carries so its sprite resolves. Deliberately NOT "Ai Slop": Ambition's
/// hosted catalog already owns that display name, and catalog assembly rejects a
/// duplicate. This demo row points its own name at the same published `ai_slop`
/// sheet, so the crony renders standalone AND hosted without a name clash.
pub const CRONY_DISPLAY_NAME: &str = "Mary-O Crony";

/// The roster brain key the crony archetype is filed under. Namespaced so it
/// never collides with a host provider's roster when Ambition hosts the demo
/// (assembly rejects a duplicate brain key across providers).
pub const CRONY_BRAIN_KEY: &str = "mary_o_crony";

/// Upward speed Mary-O gets off a squashed crony — a lively hop, a touch under a
/// full jump so a stomp reads as a bounce, not a re-jump.
const BOUNCE_SPEED: f32 = 430.0;

/// The shell's display name and brain key. `StandStill` because it is
/// BRAINLESS by design — nothing decides anything for a shell; it is a body the
/// demo pushes and the engine's ordinary physics carries.
pub const SHELL_DISPLAY_NAME: &str = "Mary-O Shell";
pub const SHELL_BRAIN_KEY: &str = "mary_o_shell";

/// How fast a kicked shell travels. Faster than a crony walks, so a kicked
/// shell reliably runs a line of them down instead of trailing behind.
const SHELL_SLIDE_SPEED: f32 = 300.0;

/// How far apart a shell and a crony must be to stop counting as a hit — the
/// same forgiving band the stomp uses, for the same reason.
const SHELL_HIT_BAND: f32 = 4.0;

/// Vertical tolerance (px) for "feet on the crony's head": the band within which
/// a descending player's feet count as landing on top rather than hitting a side.
const STOMP_BAND: f32 = 16.0;

/// Demo-owned hostile roster: ONE archetype, no `combatant` fallback row (that key
/// belongs to the host and a duplicate would fail roster assembly). A 1-HP
/// `Wanderer` walks forward and reverses at walls; `aggro_radius`/`attack_range`
/// are ignored by that template. It carries no `melee`, so its only offense is the
/// default-on body contact.
const CRONY_ROSTER_RON: &str = r#"{
    "mary_o_crony": (
        max_health: 1,
        patrol_speed: 46.0,
        chase_speed: 46.0,
        aggro_radius: 0.0,
        attack_range: 0.0,
        contact_strength: 0.5,
        damage_amount: 1,
        brain_template: Wanderer,
        move_style: Walk,
        respawn: OnRoomReenter,
    ),
    "mary_o_shell": (
        max_health: 1,
        patrol_speed: 0.0,
        chase_speed: 0.0,
        aggro_radius: 0.0,
        attack_range: 0.0,
        contact_strength: 0.0,
        damage_amount: 0,
        brain_template: StandStill,
        move_style: Walk,
        respawn: DeadStaysDead,
    ),
}"#;

/// Register the demo's hostile roster fragment (the crony archetype). Shares the
/// Mary-O provider id so its brain key namespaces under this experience.
pub fn register_crony_roster(app: &mut App) {
    use ambition::actors::features::{CharacterRosterAppExt, CharacterRosterFragment};
    app.register_character_roster_fragment(
        CharacterRosterFragment::from_ron(
            crate::provider::MARY_O_EXPERIENCE,
            None::<String>,
            CRONY_ROSTER_RON,
        )
        .expect("Mary-O crony roster fragment should be valid"),
    );
}

/// Tile x-columns (level grid) each crony paces near. Chosen on the open-teach
/// run and the ground stretches after the pit rhythm, so the walker is a hazard
/// on the flats, not stranded over a pit.
const CRONY_TILE_COLUMNS: &[f32] = &[9.0, 16.0, 27.0, 45.0, 63.0];

/// The crony spawn requests for level 1-1, dropped in at the player's standing
/// height so gravity settles each onto the ground beneath its column.
fn crony_spawn_requests(player_spawn: ae::Vec2) -> Vec<SpawnActorRequest> {
    CRONY_TILE_COLUMNS
        .iter()
        .enumerate()
        .map(|(i, col)| SpawnActorRequest {
            id: format!("mary_o_crony_{i}"),
            name: CRONY_DISPLAY_NAME.to_string(),
            pos: ae::Vec2::new(col * T, player_spawn.y),
            half_size: ae::Vec2::new(14.0, 16.0),
            faction: ActorFaction::Enemy,
            grudge_against: None,
            kind: SpawnActorKind::Enemy {
                brain: CharacterBrain::Custom(CRONY_BRAIN_KEY.to_string()),
            },
        })
        .collect()
}

/// Register the walkers as level 1-1's content staging: whenever the level's
/// contents are staged (initial load, every cyclic replay — the cronies
/// `respawn: OnRoomReenter` — and a snapshot restore staging the room), the
/// walkers stage with them. Mirrors the duel-arena content seam: a pure
/// `RoomSpec` → `SpawnActorRequest`s stager, drained by room construction and
/// applied by the engine's request applier.
pub fn register_crony_content_staging(
    registry: &mut ambition::actors::features::RoomContentStagingRegistry,
) {
    registry
        .register(
            LEVEL_1_1_ROOM_ID,
            "ambition_demo_mary_o",
            "crony",
            "crony-staging.v1",
            |spec| crony_spawn_requests(spec.world.spawn),
        )
        .expect("crony staging registration is unique");
}

/// **The head-stomp.** A player descending onto a crony's head bounces up and
/// squashes it — the classic contact stomp, NOT the engine's attack-hitbox pogo.
///
/// Ordered BEFORE the shared body-contact-damage pass so a stomp never also hurts
/// the stomper: on a squash the crony's health is zeroed THIS frame (a component
/// write, immediately visible), so the contact pass sees a not-alive attacker and
/// skips it; the body is then despawned. A SIDE touch (no head overlap) is left
/// untouched here and lands as normal contact damage on Mary-O.
///
/// **Why this despawns directly instead of routing through the shared actor-death
/// path** (`HitEvent` → `apply_actor_hit` → drops/score/debris): that path is
/// DEFERRED — a hit event emitted here is consumed a stage later, so the crony
/// would still be alive-and-hostile when `apply_actor_contact_damage` runs THIS
/// frame and would hurt the stomper (the exact bug the same-frame neutralize
/// avoids). And a crony has no score value and no drop table, so there is nothing
/// for the shared path to carry. The one thing a silent despawn would drop is the
/// visible pop, so we emit a dust [`VfxMessage::Burst`] at the corpse through the
/// engine's own vfx seam — a squash reads as a squash without adopting a death
/// pipeline whose ordering is wrong for a contact stomp.
///
/// Mary-O runs under screen gravity (down = +y), so "descending" is `vel.y > 0`,
/// her feet are the `+y` (max) edge, and a crony's head is its `-y` (min) edge.
pub fn bounce_squash_cronies(
    mut commands: Commands,
    mut spawns: MessageWriter<SpawnActorRequest>,
    mut vfx: MessageWriter<ambition::vfx::VfxMessage>,
    mut sfx: ambition::sfx::SfxWriter,
    mut players: Query<&mut ae::BodyKinematics, With<PrimaryPlayer>>,
    mut cronies: Query<
        (Entity, &ae::BodyKinematics, &mut BodyHealth),
        (Without<PrimaryPlayer>, Without<PlayerEntity>),
    >,
) {
    let Ok(mut player) = players.single_mut() else {
        return;
    };
    // Only a falling player can stomp; a rising / level player that overlaps a
    // crony is taking a side hit, which the contact pass owns.
    if player.vel.y <= 0.0 {
        return;
    }
    let p = player.aabb();
    for (entity, crony_kin, mut health) in &mut cronies {
        let g = crony_kin.aabb();
        let overlap_x = p.min.x < g.max.x && p.max.x > g.min.x;
        let feet = p.max.y;
        let on_head = feet >= g.min.y - STOMP_BAND && feet <= g.min.y + STOMP_BAND;
        if overlap_x && on_head {
            ae::movement::set_jump_velocity(&mut player.vel, ae::DEFAULT_GRAVITY_DIR, BOUNCE_SPEED);
            // The squash pops a low, tan dust burst — the engine's shared particle
            // seam, so the crony leaves a mark instead of blinking out.
            vfx.write(ambition::vfx::VfxMessage::Burst {
                pos: crony_kin.pos,
                count: 12,
                speed: 130.0,
                color: [0.80, 0.68, 0.48, 1.0],
                kind: ambition::vfx::ParticleKind::Dust,
            });
            // ...and the stomp thuds. PLACEHOLDER TIMBRE, same arrangement as the
            // brick: the engine's existing `Pogo` cue (the shared "you bounced off
            // something" verb, which is exactly what a stomp is) voiced by the
            // provider's own spec, so the demo names a cue and never a sound.
            sfx.write(ambition::sfx::SfxMessage::Pogo { pos: crony_kin.pos });
            // Neutralize before the contact pass runs, then remove the body...
            health.health.current = 0;
            commands.entity(entity).despawn();
            // ...and leave its SHELL behind. This is the whole reason a stomp is
            // interesting: the crony stops being a threat and becomes a thing
            // you can pick a fight with.
            spawns.write(SpawnActorRequest {
                id: format!("mary_o_shell_{}", entity.index()),
                name: SHELL_DISPLAY_NAME.to_string(),
                pos: crony_kin.pos,
                half_size: ae::Vec2::new(14.0, 12.0),
                faction: ActorFaction::Enemy,
                grudge_against: None,
                kind: SpawnActorKind::Enemy {
                    brain: CharacterBrain::Custom(SHELL_BRAIN_KEY.to_string()),
                },
            });
        }
    }
}

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// The shell — a brainless prop, and the demo's one emergent mechanic.
// ---------------------------------------------------------------------------

/// A stomped crony's shell. `Resting` until something kicks it; `Sliding` until
/// something stops it.
///
/// BRAINLESS is literal: the archetype's brain is `StandStill`, so nothing ever
/// decides anything for a shell. Its whole behaviour is three rules below, and
/// everything else — gravity, ground contact, walls — is the ordinary body
/// physics every actor already gets.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub enum MaryOShell {
    Resting,
    /// Sliding along the ground. `-1.0` is leftward, `1.0` rightward.
    Sliding(f32),
}

/// Tag freshly spawned shells: the engine spawns the body from the request, and
/// the demo recognises its own.
///
/// Matches `FeatureName`, which carries the AUTHORED name verbatim — not `Name`,
/// which the spawner decorates into `"Feature actor enemy: {name}"`. Matching
/// `Name` is what silently broke this the first time: the tag never fired, so
/// shells spawned inert and the whole mechanic did nothing.
pub fn tag_mary_o_shells(
    mut commands: Commands,
    fresh: Query<(Entity, &FeatureName), Without<MaryOShell>>,
) {
    for (entity, name) in &fresh {
        if name.0 == SHELL_DISPLAY_NAME {
            commands.entity(entity).try_insert(MaryOShell::Resting);
        }
    }
}

/// **Kick a resting shell; stop a sliding one.**
///
/// Walking into a shell at rest launches it AWAY from you — the direction is the
/// side you touched it from, so you decide which way it goes by which side you
/// approach. Walking into one already sliding stops it dead, which is what makes
/// a shell a tool rather than a thing you set loose and lose.
pub fn kick_mary_o_shells(
    mut sfx: ambition::sfx::SfxWriter,
    players: Query<&ae::BodyKinematics, With<PrimaryPlayer>>,
    mut shells: Query<(&mut ae::BodyKinematics, &mut MaryOShell), Without<PrimaryPlayer>>,
) {
    let Ok(player) = players.single() else {
        return;
    };
    let p = player.aabb();
    for (mut shell_kin, mut shell) in &mut shells {
        let s = shell_kin.aabb();
        let touching =
            p.min.x < s.max.x && p.max.x > s.min.x && p.min.y < s.max.y && p.max.y > s.min.y;
        if !touching {
            continue;
        }
        *shell = match *shell {
            MaryOShell::Resting => {
                // Away from the player: if she is to its left, it goes right.
                let dir = if player.pos.x <= shell_kin.pos.x {
                    1.0
                } else {
                    -1.0
                };
                sfx.write(ambition::sfx::SfxMessage::Pogo { pos: shell_kin.pos });
                // The kick IMPARTS the velocity. Leaving that to the driver
                // would have it see a stationary shell that is nominally
                // sliding — indistinguishable from one that just hit a wall —
                // and reverse it on the frame it was kicked.
                shell_kin.vel.x = dir * SHELL_SLIDE_SPEED;
                MaryOShell::Sliding(dir)
            }
            MaryOShell::Sliding(_) => {
                shell_kin.vel.x = 0.0;
                MaryOShell::Resting
            }
        };
    }
}

/// **A sliding shell runs cronies down, and turns around at walls.**
///
/// This is the payoff: one stomp becomes a weapon that clears the rest of the
/// line. The shell keeps its own horizontal velocity each tick (the body has no
/// brain to set one) and flips direction when the world stops it, so it ricochets
/// down a corridor instead of parking against the first wall.
pub fn drive_mary_o_shells(
    mut commands: Commands,
    mut vfx: MessageWriter<ambition::vfx::VfxMessage>,
    mut sfx: ambition::sfx::SfxWriter,
    mut shells: Query<(&mut ae::BodyKinematics, &mut MaryOShell), Without<PrimaryPlayer>>,
    mut cronies: Query<
        (Entity, &ae::BodyKinematics, &mut BodyHealth, &FeatureName),
        (
            Without<PrimaryPlayer>,
            Without<PlayerEntity>,
            Without<MaryOShell>,
        ),
    >,
) {
    for (mut shell_kin, mut shell) in &mut shells {
        let MaryOShell::Sliding(dir) = *shell else {
            // A resting shell holds still rather than drifting on leftover
            // velocity from the stomp that made it.
            shell_kin.vel.x = 0.0;
            continue;
        };

        // The body kernel zeroes horizontal velocity on a wall contact, so a
        // shell that has stopped moving while still commanded to slide has hit
        // something: turn it around.
        let blocked = shell_kin.vel.x.abs() < SHELL_SLIDE_SPEED * 0.25;
        let dir = if blocked { -dir } else { dir };
        *shell = MaryOShell::Sliding(dir);
        shell_kin.vel.x = dir * SHELL_SLIDE_SPEED;

        let s = shell_kin.aabb();
        for (entity, crony_kin, mut health, name) in &mut cronies {
            // Same story as the tag above: `FeatureName` is the authored name;
            // `Name` is decorated. Filtering on `Name` here meant a sliding
            // shell matched NOTHING and ran straight through every crony.
            if name.0 != CRONY_DISPLAY_NAME {
                continue;
            }
            let g = crony_kin.aabb();
            let hit = s.min.x < g.max.x + SHELL_HIT_BAND
                && s.max.x > g.min.x - SHELL_HIT_BAND
                && s.min.y < g.max.y
                && s.max.y > g.min.y;
            if !hit {
                continue;
            }
            vfx.write(ambition::vfx::VfxMessage::Burst {
                pos: crony_kin.pos,
                count: 12,
                speed: 130.0,
                color: [0.80, 0.68, 0.48, 1.0],
                kind: ambition::vfx::ParticleKind::Dust,
            });
            sfx.write(ambition::sfx::SfxMessage::Pogo { pos: crony_kin.pos });
            health.health.current = 0;
            commands.entity(entity).despawn();
        }
    }
}
