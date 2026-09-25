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
use ambition_platformer2d::combat::components::ActorFaction;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::{PlayerEntity, PrimaryPlayer};

/// The catalog `display_name` the badnik renders from; every LDtk enemy spawn
/// is rebranded to this name in [`crate::sanic_speedway`] so the sprite
/// resolves (the row points at the published `ai_slop` sheet under a
/// demo-owned name — see `SANIC_CATALOG_RON`).
pub const BADNIK_DISPLAY_NAME: &str = "Sanic Badnik";

/// The roster brain key the LDtk `EnemySpawn` entities reference.
pub const BADNIK_BRAIN_KEY: &str = "sanic_badnik";

/// How near Sanic must be for a badnik to keep thinking.
///
/// Derived from Sanic's fastest tuning (`top_speed: 2000.0`): 2.4s of lead.
pub const BADNIK_WAKE_RADIUS: f32 = 4800.0;

/// Badniks stop thinking when Sanic is nowhere near them.
///
/// Dormancy is declared per character, never inherited. An actor with no
/// `DormancyPolicy` is always awake.
pub fn tag_sanic_badniks(
    mut commands: Commands,
    fresh: Query<
        (
            Entity,
            &ambition_platformer2d::combat::actor_tuning::ActorConfig,
        ),
        Without<ambition_platformer2d::actors::features::ecs::dormancy::DormancyPolicy>,
    >,
) {
    for (entity, config) in &fresh {
        if matches!(
            &config.brain,
            ambition_platformer2d::entity_catalog::placements::CharacterBrain::Custom(key)
                if key == BADNIK_BRAIN_KEY
        ) {
            commands.entity(entity).try_insert(
                ambition_platformer2d::actors::features::ecs::dormancy::DormancyPolicy::
                    AwakeNearObservers { radius: BADNIK_WAKE_RADIUS },
            );
        }
    }
}

/// Upward speed off a stomped badnik — a lively bounce, under a full jump.
const BOUNCE_SPEED: f32 = 460.0;

/// Vertical tolerance (px) for "feet on the badnik's head".
const STOMP_BAND: f32 = 16.0;

/// How tall a badnik stands: three fifths of Sanic, the proportion a classic
/// badnik has to the hero. At his full 48 its wide art was 74 units across,
/// over twice his width (Jon: "the badniks are always way too big").
const BADNIK_STANDING_HEIGHT: f32 = crate::SANIC_STANDING_HEIGHT * 0.6;

/// Register the badnik as a character.
///
/// A 1-HP wanderer that paces and reverses at walls, with no melee: its only
/// offense is walking into you.
pub fn register_badnik_character(app: &mut App) {
    use ambition_platformer2d::actors::character_runtime::CharacterDefinitionAppExt;
    use ambition_platformer2d::character::CharacterDefinition;
    use ambition_platformer2d::characters::actor::{CharacterLocomotion, ContactDamage};
    use ambition_platformer2d::characters::brain::{
        BrainProfile, CharacterBrainTemplate, MoveStyleSpec,
    };

    let mut definition = CharacterDefinition::new(
        BADNIK_BRAIN_KEY,
        BADNIK_DISPLAY_NAME,
        crate::provider::SANIC_EXPERIENCE,
    )
    // It wears the published `ai_slop` sheet under its own name — the catalog
    // row says so, and a character states the target rather than the file.
    .with_sheet("ai_slop")
    // Its body is its art at one scale, so the quad sits on its feet (the
    // catalog join sized the box right but published no offset, and the art
    // floated ~3.6 units).
    .with_sprite_authored_body(crate::world_per_pixel_for_height(
        "ai_slop",
        BADNIK_STANDING_HEIGHT,
    ))
    .with_locomotion(CharacterLocomotion {
        run_speed: 60.0,
        move_style: MoveStyleSpec::Walk,
        ..Default::default()
    })
    // It rides the ground Sanic rides. On the axis kernel it collided only with
    // the flat solids under the hills, so it walked THROUGH every hill; on the
    // surface solver it follows the chain. `slope_factor: 0` makes it a walker,
    // not a ball: gravity along a slope neither drags it downhill nor stalls
    // it climbing, so it paces at one speed over any ground.
    .with_motion_model(ae::MotionModelSpec::SurfaceMomentum(ae::MomentumParams {
        ground_accel: 600.0,
        brake: 1800.0,
        friction: 1800.0,
        slope_factor: 0.0,
        top_speed: 60.0,
        air_accel: 0.0,
        jump_speed: 0.0,
        stick_factor: 1.5,
        ..ae::MomentumParams::default()
    }))
    .with_contact_damage(ContactDamage {
        strength: 0.5,
        amount: 1,
    })
    .with_autonomous_profile(BrainProfile {
        template: CharacterBrainTemplate::Wanderer,
        aggro_radius: 0.0,
        attack_range: 0.0,
        // Preserve the shipped patrol effort when constructing the character definition.
        patrol_effort: 1.0,
        // A badnik paces its stretch of ground: it turns at a pit lip or a
        // slope it cannot climb rather than walking off.
        turns_at_ledges: true,
        ..Default::default()
    });
    definition.vitals.max_health = Some(1);
    app.register_character(definition);
}

/// The defeat rule. A player descending onto a badnik's head bounces and
/// squashes it. A rolling player squashes it on any overlap and keeps its
/// speed. A super player squashes it on any overlap. A side touch while not
/// rolling and not super lands as normal contact damage.
///
/// Ordered before the shared body-contact-damage pass: the squash zeroes the
/// badnik's health this frame, so the contact pass skips a dead attacker; the
/// body is then despawned. Direct despawn (not the deferred actor-death
/// pipeline), as for the Mary-O crony: the shared path runs a stage late and
/// would hurt the stomper first, and a badnik has no drops or score. The pop
/// is a dust burst through the engine's vfx seam.
pub fn defeat_badniks(
    mut commands: Commands,
    mut vfx: MessageWriter<ambition_platformer2d::vfx::VfxMessage>,
    mut sfx: ambition_platformer2d::sfx::BodySfxWriter,
    mut players: Query<
        (
            &mut ae::BodyKinematics,
            Option<&crate::ball_dash::Rolling>,
            // What is true of this body, not who it is. Optional so the thin
            // test harnesses need not dress the body.
            Option<&ambition_platformer2d::actors::features::empowerment::Empowered>,
        ),
        With<PrimaryPlayer>,
    >,
    mut badniks: Query<
        (Entity, &ae::BodyKinematics, &ActorFaction, &mut BodyHealth),
        (Without<PrimaryPlayer>, Without<PlayerEntity>),
    >,
) {
    let Ok((mut player, rolling, empowered)) = players.single_mut() else {
        return;
    };
    // Squashing on touch is a trait the body holds, not a name it wears.
    // Rolling joins the kill condition but not the bounce: a super stomp
    // still bounces.
    let harms_on_contact = empowered.is_some_and(|e| {
        e.traits.holds(
            ambition_platformer2d::actors::features::empowerment::Empowerment::HARMS_ON_CONTACT,
        )
    });
    let rolling = rolling.is_some();
    let lethal_touch = rolling || harms_on_contact;
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
        vfx.write(ambition_platformer2d::vfx::VfxMessage::Burst {
            pos: badnik_kin.pos,
            count: 12,
            speed: 150.0,
            color: [0.85, 0.62, 0.35, 1.0],
            kind: ambition_platformer2d::vfx::ParticleKind::Dust,
        });
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
        app.add_message::<ambition_platformer2d::vfx::VfxMessage>();
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

    /// A body that harms on contact squashes on any touch — un-rolled, not
    /// falling, a plain walk-into.
    #[test]
    fn a_body_that_harms_on_contact_squashes_a_badnik_on_any_touch() {
        let mut app = defeat_app();
        let badnik = spawn_badnik(&mut app, ae::Vec2::new(10.0, 0.0));
        app.world_mut().spawn((
            PrimaryPlayer,
            ambition_platformer2d::actors::features::empowerment::Empowered::held(
                crate::SUPER_SANIC_SUPER_STATE,
            ),
            kin(ae::Vec2::ZERO, ae::Vec2::new(120.0, 0.0)),
        ));
        app.update();
        assert!(
            app.world().get_entity(badnik).is_err(),
            "a body whose contact harms destroys a badnik it walks into"
        );
    }

    /// The super identity alone does not: the empowerment the form grants
    /// does the work, so a body with the name and without the trait is
    /// ordinary.
    #[test]
    fn the_super_identity_alone_is_not_what_squashes() {
        let mut app = defeat_app();
        let badnik = spawn_badnik(&mut app, ae::Vec2::new(10.0, 0.0));
        app.world_mut().spawn((
            PrimaryPlayer,
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
            ambition_platformer2d::characters::actor::WornCharacter::new(crate::SANIC_CHARACTER_ID),
            kin(ae::Vec2::ZERO, ae::Vec2::new(120.0, 0.0)),
        ));
        app.update();
        assert!(
            app.world().get_entity(badnik).is_ok(),
            "the base form's side touch leaves the badnik alive"
        );
    }
}
