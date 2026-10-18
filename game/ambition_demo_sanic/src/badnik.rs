//! The speedway badnik: a stompable, roll-through-able walker, pure content.
//!
//! The Mary-O crony pattern applied to Sanic's verbs, with no engine edits:
//!
//! - Placement comes from the demo's LDtk file: `EnemySpawn` entities with
//!   `brain: "sanic_badnik"` lower into `RoomSpec::enemy_spawns`, and the
//!   engine's room staging spawns them.
//! - Body, walk, and contact damage come from the generic combatant. There is
//!   no `sanic_badnik` roster archetype, and `spec_for_brain` returns
//!   `combatant` for an unknown key. A true 1-HP wanderer needs a
//!   `sanic_badnik` character definition, because health and movement are
//!   character facts.
//! - The defeat is Sanic's, not Mary-O's: a descending bounce on the head
//!   (classic stomp), or any overlap while rolling (ball dash or crouch-roll).
//!   Both despawn the badnik the same frame, so the shared contact-damage pass
//!   never bills the attacker.
//!
//! Every type it names comes through the `ambition_platformer2d` umbrella (the
//! E9 oracle).

use bevy::prelude::*;

use ambition_platformer2d::characters::actor::BodyHealth;
use ambition_platformer2d::actor::ActorFaction;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::{PlayerEntity, PrimaryPlayer};

/// The roster brain key the LDtk `EnemySpawn` entities reference.
pub const BADNIK_BRAIN_KEY: &str = "sanic_badnik";



/// The defeat rule. A player descending onto a badnik's head bounces and
/// squashes it. A rolling player squashes it on any overlap and keeps its
/// speed. A super player squashes it on any overlap. A side touch while not
/// rolling and not super lands as normal contact damage.
///
/// ⛔ EACH PLAYER OF THE BADNIK'S OWN LIVE ROOM. It read one global
/// `PrimaryPlayer` with `single_mut()` and no room: a second seat could not
/// squash a badnik, and the primary player squashed a badnik of another live
/// room at the same place. The players are taken in the order of their
/// simulation identity, and the first one whose touch is lethal is the one
/// that squashes it.
///
/// Ordered before the shared body-contact-damage pass: the squash zeroes the
/// badnik's health this frame, so the contact pass skips a dead attacker; the
/// body is then despawned. Direct despawn (not the deferred actor-death
/// pipeline), as for the Mary-O crony: the shared path runs a stage late and
/// would hurt the stomper first, and a badnik has no drops or score. The pop
/// is a dust burst through the engine's vfx seam.
#[allow(clippy::type_complexity)]
pub fn defeat_badniks(
    mut commands: Commands,
    mut vfx: ambition_platformer2d::vfx::VfxWriter,
    // The burst is drawn in the badnik's own live room.
    rooms: ambition_platformer2d::platformer::lifecycle::LiveRooms,
    mut sfx: ambition_platformer2d::sfx::BodySfxWriter,
    mut players: Query<
        (
            Entity,
            &mut ae::BodyKinematics,
            Option<&crate::ball_dash::Rolling>,
            // What is true of this body: its grant and the character it
            // wears. Optional so the thin test harnesses need not dress the body.
            Option<&ambition_platformer2d::actors::features::empowerment::Empowered>,
            Option<&ambition_platformer2d::characters::actor::WornCharacter>,
            Option<&ambition_platformer2d::platformer::sim_id::SimId>,
        ),
        With<PlayerEntity>,
    >,
    cast: ambition_platformer2d::characters::prepared::SessionCast,
    mut badniks: Query<
        (Entity, &ae::BodyKinematics, &ActorFaction, &mut BodyHealth),
        (Without<PrimaryPlayer>, Without<PlayerEntity>),
    >,
) {
    // Each player: its room, its box and velocity at the start of the pass,
    // whether it rolls, and whether any touch of it is lethal. Squashing on
    // touch is a trait the body holds, not a name it wears.
    let everyone = ambition_platformer2d::platformer::sim_selection::in_deterministic_order(
        players.iter().map(|(player, kin, rolling, grant, worn, id)| {
            let harms_on_contact =
                ambition_platformer2d::actors::features::empowerment::empowerment_of(grant, worn, cast.get())
                    .holds(ambition_platformer2d::actors::features::empowerment::Empowerment::HARMS_ON_CONTACT);
            (player, rooms.of(player), kin.aabb(), kin.vel, rolling.is_some(), harms_on_contact, id.cloned())
        }),
        |_| 0.0,
        |(.., id)| id.as_ref(),
    );
    for (entity, badnik_kin, faction, mut health) in &mut badniks {
        if !matches!(faction, ActorFaction::Enemy) {
            continue;
        }
        let room = rooms.of(entity);
        let g = badnik_kin.aabb();
        // Rolling joins the kill condition but not the bounce: a super stomp
        // still bounces. Screen gravity is +y: "descending" is vel.y > 0, feet
        // are the max-y edge.
        let squash = everyone.iter().find_map(|(player, player_room, p, vel, rolling, harms, _)| {
            if *player_room != room {
                return None;
            }
            let falling = vel.y > 0.0;
            let overlap_x = p.min.x < g.max.x && p.max.x > g.min.x;
            let overlap_y = p.min.y < g.max.y && p.max.y > g.min.y;
            let feet = p.max.y;
            let stomp = falling && overlap_x && crate::rules::feet_on_top(feet, g.min.y);
            let roll = (*rolling || *harms) && overlap_x && overlap_y;
            (stomp || roll).then_some((*player, stomp && !*rolling))
        });
        let Some((player, bounce)) = squash else {
            continue;
        };
        if bounce {
            if let Ok((_, mut kin, ..)) = players.get_mut(player) {
                ae::movement::set_jump_velocity(&mut kin.vel, ae::DEFAULT_GRAVITY_DIR, crate::rules::sanic_rules().badnik.bounce_speed);
            }
        }
        for squash in &crate::rules::sanic_rules().effects.badnik_squash {
            vfx.for_room(room).write(squash.at(badnik_kin.pos));
        }
        // H2: the badnik is a body, and this is it popping. Its own voice.
        sfx.write_for(
            entity,
            ambition_platformer2d::sfx::SfxMessage::Play {
                id: ambition_platformer2d::sfx::SfxId::from_static(crate::SFX_BADNIK),
                pos: badnik_kin.pos,
            },
        );
        // Neutralize before the contact pass runs, then remove the body.
        health.health.current = 0;
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kin(pos: ae::Vec2, vel: ae::Vec2) -> ae::BodyKinematics {
        let mut kin = ae::BodyKinematics::default();
        kin.pos = pos;
        kin.vel = vel;
        kin.size = ae::Vec2::new(28.0, 32.0);
        kin
    }

    fn defeat_app() -> App {
        let mut app = App::new();
        app.add_message::<ambition_platformer2d::vfx::VfxInRoom>();
        app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
        app.add_systems(Update, defeat_badniks);
        app
    }

    fn spawn_badnik(app: &mut App, pos: ae::Vec2) -> Entity {
        use ambition_platformer2d::characters::actor::Health;
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
            PlayerEntity,
            kin(ae::Vec2::ZERO, ae::Vec2::new(0.0, 240.0)),
        ));
        app.update();
        assert!(
            app.world().get_entity(badnik).is_err(),
            "a stomped badnik is squashed (despawned)"
        );
    }

    /// A squashed badnik plays the bursts its rules file states
    /// (`effects.badnik_squash`), in their order, where it was.
    #[test]
    fn a_squash_plays_the_bursts_its_rules_state() {
        let mut app = defeat_app();
        let at = ae::Vec2::new(0.0, 32.0);
        spawn_badnik(&mut app, at);
        app.world_mut().spawn((PrimaryPlayer, PlayerEntity, kin(ae::Vec2::ZERO, ae::Vec2::new(0.0, 240.0))));
        app.update();
        let played: Vec<_> = app
            .world()
            .resource::<Messages<ambition_platformer2d::vfx::VfxInRoom>>()
            .iter_current_update_messages()
            .filter_map(|message| crate::rules::burst_shape(&message.vfx))
            .collect();
        let stated: Vec<_> = crate::rules::sanic_rules()
            .effects
            .badnik_squash
            .iter()
            .filter_map(|row| crate::rules::burst_shape(&row.at(at)))
            .collect();
        assert!(!stated.is_empty(), "the rules file states a squash");
        assert_eq!(played, stated, "the squash is not the file's bursts");
    }

    #[test]
    fn a_rolling_player_squashes_a_badnik_on_overlap() {
        let mut app = defeat_app();
        let badnik = spawn_badnik(&mut app, ae::Vec2::new(10.0, 0.0));
        app.world_mut().spawn((
            PrimaryPlayer,
            PlayerEntity,
            crate::ball_dash::Rolling,
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
            PlayerEntity,
            kin(ae::Vec2::ZERO, ae::Vec2::new(0.0, -200.0)),
        ));
        app.update();
        assert!(
            app.world().get_entity(badnik).is_ok(),
            "a rising, un-rolled player leaves the badnik to the contact pass"
        );
    }

    /// A body that harms on contact squashes on any touch — un-rolled, not
    /// falling, a plain walk-into.
    #[test]
    fn a_body_that_harms_on_contact_squashes_a_badnik_on_any_touch() {
        let mut app = defeat_app();
        let badnik = spawn_badnik(&mut app, ae::Vec2::new(10.0, 0.0));
        app.world_mut().spawn((
            PrimaryPlayer,
            PlayerEntity,
            ambition_platformer2d::actors::features::empowerment::Empowered::held(
                ambition_platformer2d::actors::features::empowerment::Empowerment::HARMS_ON_CONTACT,
            ),
            kin(ae::Vec2::ZERO, ae::Vec2::new(120.0, 0.0)),
        ));
        app.update();
        assert!(
            app.world().get_entity(badnik).is_err(),
            "a body whose contact harms destroys a badnik it walks into"
        );
    }

    /// The name alone does not: the traits come from the worn character's
    /// prepared row, so a body wearing an id no prepared cast describes is
    /// ordinary.
    #[test]
    fn the_super_identity_alone_is_not_what_squashes() {
        let mut app = defeat_app();
        let badnik = spawn_badnik(&mut app, ae::Vec2::new(10.0, 0.0));
        app.world_mut().spawn((
            PrimaryPlayer,
            PlayerEntity,
            ambition_platformer2d::characters::actor::WornCharacter::new(
                crate::SUPER_SANIC_CHARACTER_ID,
            ),
            kin(ae::Vec2::ZERO, ae::Vec2::new(120.0, 0.0)),
        ));
        app.update();
        assert!(
            app.world().get_entity(badnik).is_ok(),
            "the worn name is not the authority; the trait the form grants is"
        );
    }

    #[test]
    fn the_base_form_walking_into_a_badnik_does_not_squash() {
        // The same walk-into without the super identity: the badnik survives
        // (and the shared contact pass bills the player instead).
        let mut app = defeat_app();
        let badnik = spawn_badnik(&mut app, ae::Vec2::new(10.0, 0.0));
        app.world_mut().spawn((
            PrimaryPlayer,
            PlayerEntity,
            ambition_platformer2d::characters::actor::WornCharacter::new(crate::SANIC_CHARACTER_ID),
            kin(ae::Vec2::ZERO, ae::Vec2::new(120.0, 0.0)),
        ));
        app.update();
        assert!(
            app.world().get_entity(badnik).is_ok(),
            "the base form's side touch leaves the badnik alive"
        );
    }

    /// ⛔ A SECOND SEAT SQUASHES A BADNIK OF ITS OWN ROOM, AND A PLAYER OF
    /// ANOTHER LIVE ROOM AT THE SAME PLACE DOES NOT. The rule read one global
    /// `PrimaryPlayer` and no room. The control is the primary player, in the
    /// badnik's room, which squashes it.
    #[test]
    fn each_player_of_the_badniks_room_squashes_it_and_no_other() {
        use ambition_platformer2d::platformer::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot};
        let rooms = [LiveRoomInstance::ACTIVATION, LiveRoomInstance::ACTIVATION.next()];
        // (the primary player, the player's room): squashed?
        for (primary, room, squashed) in [(true, 0, true), (false, 0, true), (true, 1, false), (false, 1, false)] {
            let mut app = defeat_app();
            for room in rooms {
                app.world_mut().spawn((RoomInstanceRoot, room));
            }
            let badnik = spawn_badnik(&mut app, ae::Vec2::new(0.0, 32.0));
            app.world_mut().entity_mut(badnik).insert(InRoomInstance(rooms[0]));
            let player = app
                .world_mut()
                .spawn((PlayerEntity, InRoomInstance(rooms[room]), kin(ae::Vec2::ZERO, ae::Vec2::new(0.0, 240.0))))
                .id();
            if primary {
                app.world_mut().entity_mut(player).insert(PrimaryPlayer);
            }
            app.update();
            assert_eq!(
                app.world().get_entity(badnik).is_err(),
                squashed,
                "primary {primary}, in room {room}: squashed {squashed}"
            );
            let vel = app.world().get::<ae::BodyKinematics>(player).unwrap().vel;
            assert_eq!(vel.y < 0.0, squashed, "primary {primary}, in room {room}: the stomper bounces, and only it");
        }
    }
}
