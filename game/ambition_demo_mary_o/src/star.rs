//! **The pocket quasar** — Mary-O's super-star.
//!
//! Jon: *"Super maryo needs a super-star equivalent? We already have a 'super
//! star' invincibility music track ready to go. We need to use a shader for her
//! invisible mode. … Her super star equivalent prop will be the 'cosmic quasar'
//! or 'pocket quasar' or something like a big bright galaxy."*
//!
//! Every piece of that already existed except the one that connects them. The
//! shader ([`crate::quasar_shader`]) has been waiting on `BodyOffense::invincible`
//! since it landed; the art is the generated `super_mary_o_cosmic_quasar` prop;
//! the music is the authored `invincible_maryo` score. What was missing is that
//! **nothing in the workspace has ever written `BodyOffense::invincible`** — it
//! was a field with no producer and no consumer outside the shader, so the
//! overlay could not have drawn once. This module is its producer.
//!
//! ## Untouchable is a SET, so the star does not have to know about beats
//!
//! Damage gates on `Health::invulnerable`, which is an
//! [`Invulnerability`](ambition_platformer2d::characters::actor::Invulnerability)
//! — a set of REASONS rather than a bool. The star takes `EMPOWERED` while it burns
//! and releases `EMPOWERED` when it ends, and that is the whole of its concern: a
//! transformation beat overlapping it holds `TRANSFORMING` independently, and
//! neither can strip the other by finishing first.
//!
//! This module was written against the bool first, and the difference is worth
//! recording. With one flag it needed a local precedence rule — assert the flag
//! every tick so a beat's restore could not eat it, and yield on expiry only if
//! no beat was running — which is a rule that has to be re-derived every time a
//! third writer appears. With a reason set there is no rule, because there is
//! nothing to coordinate.

use bevy::prelude::*;

use ambition_platformer2d::actors::actor::PrimaryPlayer;
use ambition_platformer2d::actors::features::empowerment::{Empowered, Empowerment};
use ambition_platformer2d::characters::equipment::{EquipmentRow, WornEquipment};
use ambition_platformer2d::engine_core as ae;

/// Row id of the pocket quasar. Unlike the wand and the beacon this row is NOT a
/// power form: it takes no `exclusive_slot`, because becoming briefly untouchable
/// is not a rung on the small→grown→fire ladder and must not displace one.
pub const POCKET_QUASAR_ID: &str = "pocket_quasar";

/// The presentation art id the quasar `WorldItem` carries, bound to the generated
/// prop by the provider through the shared `WorldItemArt` seam.
pub const QUASAR_SPRITE: &str = "super_mary_o_cosmic_quasar";

/// The quasar's half-extent — a round pickup, sized from the generated prop.
pub const QUASAR_HALF: ae::Vec2 = ae::Vec2::new(13.0, 13.0);

/// How long the star lasts. Classic-length: long enough to change how you play a
/// screen, short enough that you feel it end. BLIND until Jon plays it.
pub const STAR_SECONDS: f32 = 10.0;

/// This beat's claim on the encounter layer's priority music tier, mirroring the
/// death and victory sequences.
const STAR_MUSIC_OWNER: &str = "mary_o_star";

/// The pocket quasar as an equipment row: a pure token.
///
/// It carries no modifier, no grant and no armor, because what it does is not
/// expressible as equipment — it is a TIMED body state. The row exists only so
/// the quasar can ride the same "touch it → it's yours" pickup the wand and the
/// beacon ride ([`ambition_platformer2d::actors::items::collect_world_items`]),
/// and [`begin_star_power`] converts it into that state and takes the token back
/// on the very next tick.
pub fn pocket_quasar() -> EquipmentRow {
    EquipmentRow {
        id: POCKET_QUASAR_ID.to_string(),
        modifiers: Vec::new(),
        grants: Vec::new(),
        on_hit: None,
        exclusive_slot: None,
    }
}

/// **The cosmic quasar super state**, composed rather than named: untouchable,
/// and harming what she touches. The engine knows both traits and neither knows
/// about Mary-O.
///
/// Jon: "There should be an elegant way to represent the idea of I'm invincible
/// and I hurt everything I touch and compose those together." This line is that
/// — adding a third trait later is a `.with()`, not a new mode.
pub const COSMIC_QUASAR_SUPER_STATE: Empowerment =
    Empowerment::UNTOUCHABLE.with(Empowerment::HARMS_ON_CONTACT);

/// **Collecting the quasar lights the super state.**
///
/// Reads the worn set rather than a collect message, for the same reason
/// `sync_grown_form` does: the worn set is the one place "what does she have"
/// has an answer. The token is spent in the same breath — the STATE's lifetime
/// is its own timer, never the item's, so nothing holds a reference to a pickup
/// that has already been collected.
///
/// Re-collecting during a star REFRESHES it: two quasars in ten seconds is one
/// longer star as far as the player can tell.
pub fn begin_star_power(
    mut commands: Commands,
    mut players: Query<(Entity, &mut WornEquipment), With<PrimaryPlayer>>,
) {
    for (body, mut worn) in &mut players {
        if worn.unequip(POCKET_QUASAR_ID).is_some() {
            commands
                .entity(body)
                .try_insert(Empowered::new(COSMIC_QUASAR_SUPER_STATE, STAR_SECONDS));
        }
    }
}

/// The star's theme, on the same priority-music seam the death and victory
/// beats use — claimed while it burns, released when it ends, so the level theme
/// returns on its own with no restore bookkeeping here.
pub fn play_star_music(
    stars: Query<&Empowered>,
    music: Option<
        ambition_platformer2d::platformer::lifecycle::SessionWorldMut<
            ambition_platformer2d::actors::encounter::EncounterMusicRequest,
        >,
    >,
) {
    let Some(mut music) = music else {
        return;
    };
    if stars.iter().next().is_some() {
        music.claim_priority(STAR_MUSIC_OWNER, crate::provider::MARY_O_STAR_MUSIC_TRACK);
    } else {
        music.release_priority(STAR_MUSIC_OWNER);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::characters::actor::BodyHealth;

    fn app_with_body() -> (App, Entity) {
        let mut app = App::new();
        app.insert_resource(ambition_platformer2d::time::WorldTime {
            scaled_dt: 1.0 / 60.0,
            ..Default::default()
        });
        let body = app
            .world_mut()
            .spawn((
                PrimaryPlayer,
                WornEquipment::default(),
                ambition_platformer2d::characters::actor::BodyHealth::new(
                    ambition_platformer2d::characters::actor::Health::new(1),
                ),
                ambition_platformer2d::engine_core::BodyKinematics {
                    pos: ambition_platformer2d::engine_core::Vec2::ZERO,
                    vel: ambition_platformer2d::engine_core::Vec2::ZERO,
                    size: ambition_platformer2d::engine_core::Vec2::new(30.0, 48.0),
                    facing: 1.0,
                },
            ))
            .id();
        app.add_systems(
            Update,
            (
                begin_star_power,
                ambition_platformer2d::actors::features::empowerment::run_empowerments,
            )
                .chain(),
        );
        (app, body)
    }

    /// **The quasar is the only thing that has ever written `invincible`.** The
    /// shader has read that fact since it landed and no producer existed, so the
    /// overlay could not have drawn once; this is the assertion that it can.
    #[test]
    fn collecting_the_quasar_makes_her_invincible_and_takes_the_token_back() {
        let (mut app, body) = app_with_body();
        assert!(
            !app
                .world()
                .get::<BodyHealth>(body)
                .unwrap()
                .health
                .invulnerable
                .any(),
            "nothing is invincible by default"
        );

        app.world_mut()
            .get_mut::<WornEquipment>(body)
            .unwrap()
            .equip(pocket_quasar());
        app.update();

        assert!(
            app.world()
                .get::<BodyHealth>(body)
                .unwrap()
                .health
                .invulnerable
                .holds(ambition_platformer2d::characters::actor::Invulnerability::EMPOWERED),
            "the star lights the EMPOWERED reason — the one the overlay draws from,\
             and not the one a transformation holds"
        );
        assert!(
            app.world()
                .get::<BodyHealth>(body)
                .unwrap()
                .health
                .invulnerable.any(),
            "and the fact the damage gate reads"
        );
        assert!(
            !app.world().get::<WornEquipment>(body).unwrap().wears(POCKET_QUASAR_ID),
            "the token is spent, not worn — a worn quasar would never expire"
        );
    }

    /// It ends, and it hands the damage gate back.
    #[test]
    fn the_star_burns_out_and_returns_her_to_ordinary_danger() {
        let (mut app, body) = app_with_body();
        app.world_mut()
            .get_mut::<WornEquipment>(body)
            .unwrap()
            .equip(pocket_quasar());
        app.update();

        // Burn the whole duration, plus a tick.
        for _ in 0..((STAR_SECONDS * 60.0) as usize + 2) {
            app.update();
        }

        assert!(
            app.world().get::<Empowered>(body).is_none(),
            "a spent star leaves the body"
        );
        assert!(
            !app.world()
                .get::<BodyHealth>(body)
                .unwrap()
                .health
                .invulnerable.any(),
            "and she is ordinary again"
        );
    }

    /// **Damage actually stops**, through the real gate rather than by reading
    /// the flag back. A test that only asserted the bool would pass even if
    /// `Health::damage` ignored it.
    #[test]
    fn a_burning_star_actually_eats_the_hit() {
        let (mut app, body) = app_with_body();
        app.world_mut()
            .get_mut::<WornEquipment>(body)
            .unwrap()
            .equip(pocket_quasar());
        app.update();

        let mut health = app.world_mut().get_mut::<BodyHealth>(body).unwrap();
        assert!(!health.health.damage(99), "the hit is refused");
        assert!(health.health.alive(), "so she is still standing");
    }
}
