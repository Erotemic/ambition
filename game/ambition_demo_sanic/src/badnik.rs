//! The speedway badnik — a stompable, roll-through-able walker, pure content.
//!
//! The Mary-O crony pattern applied to Sanic's verbs, with **zero engine
//! edits**:
//!
//! - **Placement** comes from the demo's LDtk file: `EnemySpawn` entities
//!   carrying `brain: "sanic_badnik"` lower into `RoomSpec::enemy_spawns` and
//!   the engine's room staging spawns them — no demo staging system at all.
//! - **Body + walk + contact damage** come from a demo-owned roster archetype
//!   (`sanic_badnik`, a 1-HP `Wanderer` that paces and reverses at walls). Its
//!   body contact hurts a Sanic that runs into it un-rolled.
//! - **The defeat** is Sanic's, not Mary-O's: a descending bounce on the head
//!   (classic stomp, with the bounce) OR any overlap while ROLLING (the ball
//!   dash / crouch-roll is the weapon — rolling through a badnik at speed is
//!   the Sonic fantasy). Both despawn the badnik the same frame so the shared
//!   contact-damage pass never bills the attacker.
//!
//! Every type it names comes through the `ambition` umbrella — the E9 oracle.

use bevy::prelude::*;

use ambition::actors::actor::{PlayerEntity, PrimaryPlayer};
use ambition::actors::combat::components::ActorFaction;
use ambition::characters::actor::BodyHealth;
use ambition::engine_core as ae;

/// The catalog `display_name` the badnik renders from; every LDtk enemy spawn
/// is rebranded to this name in [`crate::sanic_speedway`] so the sprite
/// resolves (the row points at the published `ai_slop` sheet under a
/// demo-owned name — see `SANIC_CATALOG_RON`).
pub const BADNIK_DISPLAY_NAME: &str = "Sanic Badnik";

/// The roster brain key the LDtk `EnemySpawn` entities reference.
pub const BADNIK_BRAIN_KEY: &str = "sanic_badnik";

/// Upward speed off a stomped badnik — a lively bounce, under a full jump.
const BOUNCE_SPEED: f32 = 460.0;

/// Vertical tolerance (px) for "feet on the badnik's head".
const STOMP_BAND: f32 = 16.0;

/// Demo-owned hostile roster: ONE archetype. A 1-HP `Wanderer` paces and
/// reverses at walls; it carries no melee, so its only offense is the
/// default-on body contact.
const BADNIK_ROSTER_RON: &str = r#"{
    "sanic_badnik": (
        max_health: 1,
        patrol_speed: 60.0,
        chase_speed: 60.0,
        aggro_radius: 0.0,
        attack_range: 0.0,
        contact_strength: 0.5,
        damage_amount: 1,
        brain_template: Wanderer,
        move_style: Walk,
        respawn: OnRoomReenter,
    ),
}"#;

/// Register the demo's hostile roster fragment (the badnik archetype), keyed
/// under the Sanic experience so the brain key namespaces per provider.
pub fn register_badnik_roster(app: &mut App) {
    use ambition::actors::features::{CharacterRosterAppExt, CharacterRosterFragment};
    app.register_character_roster_fragment(
        CharacterRosterFragment::from_ron(
            crate::provider::SANIC_EXPERIENCE,
            None::<String>,
            BADNIK_ROSTER_RON,
        )
        .expect("Sanic badnik roster fragment should be valid"),
    );
}

/// **The defeat rule.** A player descending onto a badnik's head bounces up
/// and squashes it; a ROLLING player squashes it on any overlap and keeps its
/// speed (rolling through a line of badniks is the point of rolling); a SUPER
/// player squashes it on any overlap, full stop (the classic invincible-form
/// contract — walking through badniks is the super fantasy). A side touch
/// while un-rolled and un-super is left alone and lands as normal contact
/// damage.
///
/// Ordered BEFORE the shared body-contact-damage pass: the squash zeroes the
/// badnik's health THIS frame (a component write, immediately visible), so the
/// contact pass sees a not-alive attacker and skips it; the body is then
/// despawned. Direct despawn (not the deferred actor-death pipeline) for the
/// same reasons as the Mary-O crony: the shared path is a stage late and would
/// hurt the stomper first, and a badnik carries no drops/score. The visible
/// pop comes from a dust burst through the engine's own vfx seam.
pub fn defeat_badniks(
    mut commands: Commands,
    mut vfx: MessageWriter<ambition::vfx::VfxMessage>,
    mut sfx: ambition::sfx::SfxWriter,
    mut players: Query<
        (
            &mut ae::BodyKinematics,
            Option<&crate::ball_dash::Rolling>,
            // Optional so the thin test harnesses need not dress the body; a
            // real player always wears an identity.
            Option<&ambition::characters::actor::WornCharacter>,
        ),
        With<PrimaryPlayer>,
    >,
    mut badniks: Query<
        (Entity, &ae::BodyKinematics, &ActorFaction, &mut BodyHealth),
        (Without<PrimaryPlayer>, Without<PlayerEntity>),
    >,
) {
    let Ok((mut player, rolling, worn)) = players.single_mut() else {
        return;
    };
    // The SUPER form squashes on touch — derived from the worn identity, the
    // same read `sync_super_form_traits` keys invincibility on. It joins
    // rolling for the kill condition but not for the bounce: a super stomp
    // still bounces like any stomp.
    let is_super = worn.is_some_and(|w| w.id() == crate::SUPER_SANIC_CHARACTER_ID);
    let rolling = rolling.is_some();
    let lethal_touch = rolling || is_super;
    // Screen gravity is +y: "descending" is vel.y > 0, feet are the max-y edge.
    let falling = player.vel.y > 0.0;
    if !lethal_touch && !falling {
        return;
    }
    let p = player.aabb();
    for (entity, badnik_kin, faction, mut health) in &mut badniks {
        if !matches!(faction, ActorFaction::Enemy) {
            continue;
        }
        let g = badnik_kin.aabb();
        let overlap_x = p.min.x < g.max.x && p.max.x > g.min.x;
        let overlap_y = p.min.y < g.max.y && p.max.y > g.min.y;
        let feet = p.max.y;
        let stomp =
            falling && overlap_x && feet >= g.min.y - STOMP_BAND && feet <= g.min.y + STOMP_BAND;
        let roll = lethal_touch && overlap_x && overlap_y;
        if !stomp && !roll {
            continue;
        }
        if stomp && !rolling {
            ae::movement::set_jump_velocity(&mut player.vel, ae::DEFAULT_GRAVITY_DIR, BOUNCE_SPEED);
        }
        vfx.write(ambition::vfx::VfxMessage::Burst {
            pos: badnik_kin.pos,
            count: 12,
            speed: 150.0,
            color: [0.85, 0.62, 0.35, 1.0],
            kind: ambition::vfx::ParticleKind::Dust,
        });
        sfx.write(ambition::sfx::SfxMessage::Play {
            id: ambition::sfx::SfxId::from_static(crate::SFX_BADNIK),
            pos: badnik_kin.pos,
        });
        // Neutralize before the contact pass runs, then remove the body.
        health.health.current = 0;
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_badnik_roster_fragment_parses() {
        register_badnik_roster(&mut App::new());
    }

    fn kin(pos: ae::Vec2, vel: ae::Vec2) -> ae::BodyKinematics {
        let mut kin = ae::BodyKinematics::default();
        kin.pos = pos;
        kin.vel = vel;
        kin.size = ae::Vec2::new(28.0, 32.0);
        kin
    }

    fn defeat_app() -> App {
        let mut app = App::new();
        app.add_message::<ambition::vfx::VfxMessage>();
        app.add_message::<ambition::sfx::OwnedSfxMessage>();
        app.add_systems(Update, defeat_badniks);
        app
    }

    fn spawn_badnik(app: &mut App, pos: ae::Vec2) -> Entity {
        use ambition::characters::actor::Health;
        app.world_mut()
            .spawn((
                kin(pos, ae::Vec2::ZERO),
                ActorFaction::Enemy,
                BodyHealth::new(Health::new(1)),
            ))
            .id()
    }

    #[test]
    fn a_descending_player_bounces_off_and_squashes_a_badnik() {
        let mut app = defeat_app();
        let badnik = spawn_badnik(&mut app, ae::Vec2::new(0.0, 32.0));
        app.world_mut().spawn((
            PrimaryPlayer,
            kin(ae::Vec2::ZERO, ae::Vec2::new(0.0, 240.0)),
        ));
        app.update();
        assert!(
            app.world().get_entity(badnik).is_err(),
            "a stomped badnik is squashed (despawned)"
        );
    }

    #[test]
    fn a_rolling_player_squashes_a_badnik_on_overlap() {
        let mut app = defeat_app();
        let badnik = spawn_badnik(&mut app, ae::Vec2::new(10.0, 0.0));
        app.world_mut().spawn((
            PrimaryPlayer,
            crate::ball_dash::Rolling {
                restore_size: ae::Vec2::new(28.0, 32.0),
            },
            kin(ae::Vec2::ZERO, ae::Vec2::new(600.0, 0.0)),
        ));
        app.update();
        assert!(
            app.world().get_entity(badnik).is_err(),
            "a rolling player squashes a badnik it overlaps"
        );
    }

    #[test]
    fn a_rising_unrolled_player_does_not_squash() {
        let mut app = defeat_app();
        let badnik = spawn_badnik(&mut app, ae::Vec2::new(10.0, 0.0));
        app.world_mut().spawn((
            PrimaryPlayer,
            kin(ae::Vec2::ZERO, ae::Vec2::new(0.0, -200.0)),
        ));
        app.update();
        assert!(
            app.world().get_entity(badnik).is_ok(),
            "a rising, un-rolled player leaves the badnik to the contact pass"
        );
    }

    #[test]
    fn a_super_player_squashes_a_badnik_on_any_touch() {
        // Un-rolled, not falling — a plain walk-into. The worn SUPER identity
        // alone makes the touch lethal to the badnik instead of the player.
        let mut app = defeat_app();
        let badnik = spawn_badnik(&mut app, ae::Vec2::new(10.0, 0.0));
        app.world_mut().spawn((
            PrimaryPlayer,
            ambition::characters::actor::WornCharacter::new(crate::SUPER_SANIC_CHARACTER_ID),
            kin(ae::Vec2::ZERO, ae::Vec2::new(120.0, 0.0)),
        ));
        app.update();
        assert!(
            app.world().get_entity(badnik).is_err(),
            "a super player destroys a badnik on contact"
        );
    }

    #[test]
    fn the_base_form_walking_into_a_badnik_does_not_squash() {
        // The same walk-into WITHOUT the super identity: the badnik survives
        // (and the shared contact pass bills the player instead).
        let mut app = defeat_app();
        let badnik = spawn_badnik(&mut app, ae::Vec2::new(10.0, 0.0));
        app.world_mut().spawn((
            PrimaryPlayer,
            ambition::characters::actor::WornCharacter::new(crate::SANIC_CHARACTER_ID),
            kin(ae::Vec2::ZERO, ae::Vec2::new(120.0, 0.0)),
        ));
        app.update();
        assert!(
            app.world().get_entity(badnik).is_ok(),
            "the base form's side touch leaves the badnik alive"
        );
    }
}
