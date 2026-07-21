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
            // Neutralize before the contact pass runs, then remove the body.
            health.health.current = 0;
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_crony_roster_fragment_parses() {
        // The archetype RON must be a valid roster fragment — the standalone demo
        // has no other roster, so a malformed row would leave the crony as the
        // inert engine fallback (no walk, no contact).
        let mut app = App::new();
        register_crony_roster(&mut app);
        assert!(app
            .world()
            .contains_resource::<ambition::actors::features::CharacterRoster>());
    }

    fn kin(pos: ae::Vec2, vel: ae::Vec2) -> ae::BodyKinematics {
        ae::BodyKinematics {
            pos,
            vel,
            size: ae::Vec2::new(28.0, 32.0),
            facing: 1.0,
        }
    }

    fn spawn_pair(app: &mut App, player_vel: ae::Vec2) -> (Entity, Entity) {
        use ambition::characters::actor::Health;
        // Crony at the origin; its head (min.y) sits at y = -16.
        let crony = app
            .world_mut()
            .spawn((
                kin(ae::Vec2::ZERO, ae::Vec2::ZERO),
                BodyHealth::new(Health::new(1)),
            ))
            .id();
        // Player directly above, feet (max.y) exactly on the crony's head.
        let player = app
            .world_mut()
            .spawn((
                PrimaryPlayer,
                PlayerEntity,
                kin(ae::Vec2::new(0.0, -32.0), player_vel),
            ))
            .id();
        (crony, player)
    }

    #[test]
    fn a_descending_player_bounces_off_and_squashes_a_crony() {
        let mut app = App::new();
        app.add_message::<ambition::vfx::VfxMessage>();
        app.add_message::<ambition::sfx::OwnedSfxMessage>();
        app.add_systems(Update, bounce_squash_cronies);
        // Falling onto the head (screen gravity: +y is down, so vel.y > 0 falls).
        let (crony, player) = spawn_pair(&mut app, ae::Vec2::new(0.0, 240.0));
        app.update();

        assert!(
            app.world().get_entity(crony).is_err(),
            "a stomped crony is squashed (despawned)"
        );
        let vel = app.world().get::<ae::BodyKinematics>(player).unwrap().vel;
        assert!(
            vel.y < 0.0,
            "the stomp bounces the player back UP (screen gravity: up is -y), got {vel:?}"
        );
        // The squash leaves a visible mark: a dust burst through the engine seam.
        let bursts = app
            .world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ambition::vfx::VfxMessage>>()
            .drain()
            .filter(|m| matches!(m, ambition::vfx::VfxMessage::Burst { .. }))
            .count();
        assert_eq!(bursts, 1, "a squash pops exactly one dust burst");
    }

    #[test]
    fn a_rising_player_does_not_squash_a_crony() {
        let mut app = App::new();
        app.add_message::<ambition::vfx::VfxMessage>();
        app.add_message::<ambition::sfx::OwnedSfxMessage>();
        app.add_systems(Update, bounce_squash_cronies);
        // Overlapping the crony's head band but moving UP — a side/undercut hit,
        // which the engine's contact-damage pass owns, not a stomp.
        let (crony, _player) = spawn_pair(&mut app, ae::Vec2::new(0.0, -200.0));
        app.update();
        assert!(
            app.world().get_entity(crony).is_ok(),
            "only a DESCENDING player stomps; a rising one must not squash"
        );
    }

    #[test]
    fn cronies_spawn_on_the_flats_named_for_the_ai_slop_sheet() {
        let spawn = ae::Vec2::new(2.0 * T, 400.0);
        let reqs = crony_spawn_requests(spawn);
        assert_eq!(reqs.len(), CRONY_TILE_COLUMNS.len());
        for req in &reqs {
            assert_eq!(
                req.name, CRONY_DISPLAY_NAME,
                "every crony must carry the display name the ai_slop sheet resolves from"
            );
            assert!(
                matches!(&req.kind, SpawnActorKind::Enemy { brain }
                    if matches!(brain, CharacterBrain::Custom(k) if k == CRONY_BRAIN_KEY)),
                "cronies spawn on the demo's own roster archetype"
            );
            assert_eq!(
                req.pos.y, spawn.y,
                "dropped in at standing height to settle"
            );
        }
    }
}
