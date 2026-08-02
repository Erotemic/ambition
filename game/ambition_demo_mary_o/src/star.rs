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
use ambition_platformer2d::characters::actor::BodyHealth;
use ambition_platformer2d::characters::equipment::{EquipmentRow, WornEquipment};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::BodyOffense;

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

/// A star currently burning on this body.
///
/// Registered snapshot state in spirit: it gates whether hits land, and anything
/// that can cause a hit to be IGNORED is simulation state. (Rollback registration
/// for the demo's own components is a separate pass; noted rather than assumed.)
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct StarPower {
    pub remaining: f32,
}

impl StarPower {
    fn fresh() -> Self {
        Self {
            remaining: STAR_SECONDS,
        }
    }
}

/// **Collecting the quasar lights the star.**
///
/// Reads the worn set rather than a collect message, for the same reason
/// `sync_grown_form` does: the worn set is the one place "what does she have"
/// has an answer, so there is no second flag to drift. The token is removed in
/// the same breath — a quasar that stayed worn would re-light the star every
/// tick and never expire.
///
/// Re-collecting during a star REFRESHES it rather than stacking: two stars in
/// ten seconds is one longer star as far as the player can tell.
pub fn begin_star_power(
    mut commands: Commands,
    mut players: Query<(Entity, &mut WornEquipment), With<PrimaryPlayer>>,
) {
    for (body, mut worn) in &mut players {
        if worn.unequip(POCKET_QUASAR_ID).is_some() {
            commands.entity(body).try_insert(StarPower::fresh());
        }
    }
}

/// Hold the star: untouchable, and visibly a quasar, until it burns out.
///
/// It writes its own reason each tick and nothing else's, so a transformation
/// running through the middle of a star is simply not this system's problem.
pub fn run_star_power(
    time: Res<ambition_platformer2d::time::WorldTime>,
    mut commands: Commands,
    mut bodies: Query<(Entity, &mut StarPower, &mut BodyHealth, &mut BodyOffense)>,
) {
    let dt = time.scaled_dt;
    for (entity, mut star, mut health, mut offense) in &mut bodies {
        star.remaining -= dt;
        let burning = star.remaining > 0.0;
        health
            .health
            .invulnerable
            .set(ambition_platformer2d::characters::actor::Invulnerability::EMPOWERED, burning);
        offense.invincible = burning;
        if !burning {
            commands.entity(entity).remove::<StarPower>();
        }
    }
}

/// The star's theme, on the same priority-music seam the death and victory
/// beats use — claimed while it burns, released when it ends, so the level theme
/// returns on its own with no restore bookkeeping here.
pub fn play_star_music(
    stars: Query<&StarPower>,
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
                BodyHealth::new(ambition_platformer2d::characters::actor::Health::new(1)),
                BodyOffense::default(),
            ))
            .id();
        app.add_systems(Update, (begin_star_power, run_star_power).chain());
        (app, body)
    }

    /// **The quasar is the only thing that has ever written `invincible`.** The
    /// shader has read that fact since it landed and no producer existed, so the
    /// overlay could not have drawn once; this is the assertion that it can.
    #[test]
    fn collecting_the_quasar_makes_her_invincible_and_takes_the_token_back() {
        let (mut app, body) = app_with_body();
        assert!(
            !app.world().get::<BodyOffense>(body).unwrap().invincible,
            "nothing is invincible by default"
        );

        app.world_mut()
            .get_mut::<WornEquipment>(body)
            .unwrap()
            .equip(pocket_quasar());
        app.update();

        assert!(
            app.world().get::<BodyOffense>(body).unwrap().invincible,
            "the star lights the fact the shader draws from"
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
            app.world().get::<StarPower>(body).is_none(),
            "a spent star leaves the body"
        );
        assert!(!app.world().get::<BodyOffense>(body).unwrap().invincible);
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
