//! Mary-O's pocket-quasar super state.
//!
//! The effect claims its own invulnerability reason while active, so overlapping
//! transformation or other invulnerability claims compose independently.

use bevy::prelude::*;

use ambition_platformer2d::combat::empowerment::Empowered;
use ambition_platformer2d::character::Empowerment;
use ambition_platformer2d::characters::equipment::{EquipmentRow, WornEquipment};
use ambition_platformer2d::platformer::markers::PlayerEntity;
// The tests seat the first player.
#[cfg(test)]
use ambition_platformer2d::platformer::markers::PrimaryPlayer;

/// Row id of the pocket quasar. Unlike the wand and the beacon this row is NOT a
/// power form: it takes no `exclusive_slot`, because becoming briefly untouchable
/// is not a rung on the small→grown→fire ladder and must not displace one.
pub const POCKET_QUASAR_ID: &str = "pocket_quasar";

/// The presentation art id the quasar `WorldItem` carries, bound to the generated
/// prop by the provider through the shared `WorldItemArt` seam.
pub const QUASAR_SPRITE: &str = "super_mary_o_cosmic_quasar";

/// This beat's claim on the encounter layer's priority music tier, mirroring the
/// death and victory sequences.
const STAR_MUSIC_OWNER: &str = "mary_o_star";

/// The pocket quasar as an equipment row: a pure token.
///
/// It carries no modifier, no grant and no armor, because what it does is not
/// expressible as equipment — it is a TIMED body state. The row exists only so
/// the quasar can ride the same "touch it → it's yours" pickup the wand and the
/// beacon ride ([`ambition_platformer2d::world_items::collect_world_items`]),
/// and [`begin_star_power`] converts it into that state and takes the token back
/// on the very next tick.
pub fn pocket_quasar() -> EquipmentRow {
    EquipmentRow {
        id: POCKET_QUASAR_ID.to_string(),
        modifiers: Vec::new(),
        grants: Vec::new(),
        on_hit: None,
        exclusive_slot: None,
        withholds: Vec::new(),
        lends: Vec::new(),
        rebinds: Default::default(),
    }
}

/// Cosmic-quasar traits: untouchable and harmful on contact.
pub const COSMIC_QUASAR_SUPER_STATE: Empowerment =
    Empowerment::UNTOUCHABLE.with(Empowerment::HARMS_ON_CONTACT);

/// A pocket quasar burning on a body: ONE application with one interval,
/// which supplies several domains (effects 6b). Defense and contact read its
/// empowerment grant ([`QUASAR_GRANT`]), presentation and the star theme read
/// this component, and none of them keeps its own clock.
///
/// The interval is the BODY's once admitted: the token is spent the tick it
/// is collected, so nothing about the pickup can end the star. Course
/// completion and death retire the quasar, and only the quasar: an
/// empowerment some other granter owns is not theirs to take
/// ([`retire_quasar`]).
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PocketQuasar {
    /// Seconds of star left.
    pub remaining: f32,
}

/// The empowerment a burning quasar grants, held for exactly as long as the
/// quasar burns. HELD, not timed: the quasar's interval is the only clock, so
/// the grant cannot outlive or undercut it.
pub const QUASAR_GRANT: Empowered = Empowered { remaining: None, traits: COSMIC_QUASAR_SUPER_STATE };

/// End `body`'s quasar: the application, and its grant if the grant is still
/// the quasar's.
pub(crate) fn retire_quasar(commands: &mut Commands, body: Entity, grant: Option<&Empowered>) {
    let mut body = commands.entity(body);
    body.remove::<PocketQuasar>();
    if grant == Some(&QUASAR_GRANT) {
        body.remove::<Empowered>();
    }
}

/// Collecting the quasar lights the super state.
///
/// Reads the worn set rather than a collect message, for the same reason
/// `sync_grown_form` does: the worn set is the one place "what does she have"
/// has an answer. The token is spent in the same breath — the STATE's lifetime
/// is its own interval, never the item's, so nothing holds a reference to a
/// pickup that has already been collected.
///
/// Re-collecting during a star REFRESHES it: two quasars in ten seconds is one
/// longer star as far as the player can tell. A body already empowered by
/// another granter keeps that grant: the quasar still burns (its theme and
/// its look) without overwriting what it does not own.
pub fn begin_star_power(
    mut commands: Commands,
    mut players: Query<(Entity, &mut WornEquipment, Option<&Empowered>), With<PlayerEntity>>,
) {
    for (body, mut worn, grant) in &mut players {
        // Read first: `unequip` takes `&mut`, which marks `WornEquipment`
        // changed on every tick, and each change re-folds the repertoire.
        if worn.wears(POCKET_QUASAR_ID) && worn.unequip(POCKET_QUASAR_ID).is_some() {
            let mut body = commands.entity(body);
            body.try_insert(PocketQuasar { remaining: crate::rules::mary_o_rules().star_seconds });
            if grant.is_none_or(|grant| *grant == QUASAR_GRANT) {
                body.try_insert(QUASAR_GRANT);
            }
        }
    }
}

/// The quasar's one clock. It runs at the body's room pace, as every
/// empowerment clock does, and the quasar burns out at zero.
pub fn burn_pocket_quasars(
    mut commands: Commands,
    time: Res<ambition_platformer2d::time::WorldTime>,
    freeze: ambition_platformer2d::platformer::lifecycle::RoomFreeze,
    mut quasars: Query<(Entity, &mut PocketQuasar, Option<&Empowered>)>,
) {
    let dt = time.sim_dt();
    for (body, mut quasar, grant) in &mut quasars {
        quasar.remaining -= dt * freeze.scale_of(body);
        if quasar.remaining <= 0.0 {
            retire_quasar(&mut commands, body, grant);
        }
    }
}

/// **THE QUASAR ENDS WHEN THE COURSE DOES.**
///
/// Reaching the pole is the end of the attempt, and an invincibility that
/// outlives the attempt is a status the player cannot get rid of — it followed
/// her through the victory walk and, because it is what `play_star_music`
/// reads, it kept the star theme playing over the victory fanfare.
///
/// ⛔⛤ **THIS IS NOT A MUSIC FIX, AND WRITING ONE WOULD HAVE BEEN THE BUG.**
/// Two live claimants in one room resolve by when each claim began (the
/// later one plays, `EncounterMusicRequest::claim_priority`). Making victory
/// win by ordering it after `play_star_music`, or by when it began, would
/// encode a semantic rank ("victory outranks a super state") somewhere that
/// does not name it.
///
/// Ending the QUASAR instead leaves exactly one claimant, so the music follows
/// from the gameplay fact rather than from a race. It also answers the half a
/// music fix could not: her *status* is reset too, which is what the player
/// actually asked for.
///
/// Each player of the live room whose flag sequence runs: the course of that
/// room is the one that ends.
pub fn end_star_power_at_victory(
    mut commands: Commands,
    rooms: ambition_platformer2d::platformer::lifecycle::LiveRooms,
    sequences: Query<(Entity, &crate::flag::FlagSequence)>,
    quasars: Query<(Entity, Option<&Empowered>), (With<PocketQuasar>, With<PlayerEntity>)>,
) {
    let won: Vec<_> = sequences
        .iter()
        .filter(|(_, sequence)| sequence.active())
        .map(|(entity, _)| rooms.of(entity))
        .collect();
    if won.is_empty() {
        return;
    }
    for (body, grant) in &quasars {
        if won.contains(&rooms.of(body)) {
            retire_quasar(&mut commands, body, grant);
        }
    }
}

/// Run condition of [`play_star_music`]: a quasar burns, or a star claim
/// stands that may be owed a release. The quasar, not any empowerment: a
/// body empowered by something else is not playing the star.
pub fn star_music_is_owed(
    stars: Query<(), With<PocketQuasar>>,
    music: Option<
        ambition_platformer2d::platformer::lifecycle::SessionWorldRef<
            ambition_platformer2d::encounter::EncounterMusicRequest,
        >,
    >,
) -> bool {
    !stars.is_empty() || music.is_some_and(|music| music.holds_claims_of_kind(STAR_MUSIC_OWNER))
}

/// The star's theme, on the same priority-music seam the death and victory
/// beats use — claimed while it burns, released when it ends, so the level theme
/// returns on its own with no restore bookkeeping here.
///
/// The claim is in the live room of each body with a burning quasar.
pub fn play_star_music(
    stars: Query<Entity, With<PocketQuasar>>,
    live: ambition_platformer2d::platformer::lifecycle::LiveRooms,
    music: Option<
        ambition_platformer2d::platformer::lifecycle::SessionWorldMut<
            ambition_platformer2d::encounter::EncounterMusicRequest,
        >,
    >,
    // The tick, which dates a music claim.
    sim_tick: Option<Res<ambition_platformer2d::sim::SimTick>>,
) {
    let Some(mut music) = music else {
        return;
    };
    let now = sim_tick.as_ref().map_or(0, |tick| tick.0);
    let rooms: Vec<_> = stars.iter().map(|body| live.of(body)).collect();
    music.release_priority_where(STAR_MUSIC_OWNER, |_, room| !rooms.contains(&room));
    for room in rooms {
        music.claim_priority(room, STAR_MUSIC_OWNER, crate::provider::MARY_O_STAR_MUSIC_TRACK, now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::characters::actor::BodyHealth;

    fn app_with_body() -> (App, Entity) {
        let mut app = App::new();
        app.insert_resource(ambition_platformer2d::time::WorldTime::new(0.0, 1.0 / 60.0));
        let body = app
            .world_mut()
            .spawn((
                PrimaryPlayer,
                PlayerEntity,
                WornEquipment::default(),
                ambition_platformer2d::characters::actor::BodyHealth::new(
                    ambition_platformer2d::characters::actor::Health::new(1),
                ),
                ambition_platformer2d::engine_core::BodyKinematics { material: Default::default(),
                    pos: ambition_platformer2d::engine_core::Vec2::ZERO,
                    vel: ambition_platformer2d::engine_core::Vec2::ZERO,
                    canonical_size: ambition_platformer2d::engine_core::Vec2::new(30.0, 48.0),
                    facing: 1.0,
                },
            ))
            .id();
        // The grant, and the engine's clock ordered behind it — the same two statements the
        // real composition makes. The clock itself is no longer this harness's to schedule:
        // `EmpowermentLifecyclePlugin` installs it in `EmpowermentExpiry`, which is the whole
        // point of.
        app.add_systems(Update, (begin_star_power, burn_pocket_quasars).chain());
        app.add_plugins(
            ambition_platformer2d::combat::empowerment::EmpowermentLifecyclePlugin,
        );
        app.configure_sets(
            Update,
            ambition_platformer2d::combat::empowerment::EmpowermentExpiry
                .after(begin_star_power),
        );
        (app, body)
    }

    /// The quasar is the only thing that has ever written `invincible`. The
    /// shader has read that fact since it landed and no producer existed, so the
    /// overlay could not have drawn once; this is the assertion that it can.
    #[test]
    fn collecting_the_quasar_makes_her_invincible_and_takes_the_token_back() {
        let (mut app, body) = app_with_body();
        assert!(
            !app.world()
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
                .invulnerable
                .any(),
            "and the fact the damage gate reads"
        );
        assert!(
            !app.world()
                .get::<WornEquipment>(body)
                .unwrap()
                .wears(POCKET_QUASAR_ID),
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
        for _ in 0..((crate::rules::mary_o_rules().star_seconds * 60.0) as usize + 2) {
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
                .invulnerable
                .any(),
            "and she is ordinary again"
        );
    }

    /// Damage actually stops, through the real gate rather than by reading
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

#[cfg(test)]
mod victory_tests {
    use super::*;
    use crate::flag::{FlagPhase, FlagSequence};

    /// **VICTORY ENDS THE QUASAR, SO IT ALSO ENDS THE QUASAR'S MUSIC.**
    ///
    /// Reported 2026-09-21: reaching the flag left her quasar status running and
    /// the star theme playing over the victory fanfare.
    ///
    /// ⛔ THE CONTROL IS THE POINT. With two claimants in one room the later
    /// claim plays, and victory's claim begins after the star's, so a test
    /// that only checked "victory's track is playing" would pass on the broken
    /// build. This asserts the GAMEPLAY fact —
    /// the super-state is gone — which is what removes the second claimant
    /// entirely, and separately that an idle flag leaves a burning quasar alone.
    #[test]
    fn reaching_the_flag_ends_her_quasar() {
        for (phase, still_empowered, label) in [
            (FlagPhase::Idle, true, "an idle flag must not touch her quasar"),
            (
                FlagPhase::Sliding { score: 400 },
                false,
                "sliding down the pole must end it",
            ),
            (
                FlagPhase::Tallied { score: 400 },
                false,
                "and it stays ended through the tally",
            ),
        ] {
            let mut app = App::new();
            let body = app
                .world_mut()
                .spawn((PrimaryPlayer, PlayerEntity, PocketQuasar { remaining: 10.0 }, QUASAR_GRANT))
                .id();
            app.world_mut().spawn(FlagSequence {
                phase,
                ..Default::default()
            });
            app.add_systems(Update, end_star_power_at_victory);
            app.update();

            assert_eq!(
                app.world().get::<Empowered>(body).is_some(),
                still_empowered,
                "{label}"
            );
        }
    }

    /// A flag ends the quasar of each player of its own live room, and of no
    /// player of another: the course that ends is the course of that room.
    #[test]
    fn a_flag_ends_the_quasar_of_its_own_room_only() {
        use ambition_platformer2d::platformer::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot};
        let mut app = App::new();
        let (won, other) = (LiveRoomInstance::ACTIVATION, LiveRoomInstance::ACTIVATION.next());
        for room in [won, other] {
            app.world_mut().spawn((RoomInstanceRoot, room));
        }
        let player = |app: &mut App, room| {
            app.world_mut()
                .spawn((PlayerEntity, InRoomInstance(room), PocketQuasar { remaining: 10.0 }, QUASAR_GRANT))
                .id()
        };
        let first = player(&mut app, won);
        let second = player(&mut app, won);
        let elsewhere = player(&mut app, other);
        app.world_mut().spawn((
            FlagSequence {
                phase: FlagPhase::Sliding { score: 400 },
                ..Default::default()
            },
            InRoomInstance(won),
        ));
        app.add_systems(Update, end_star_power_at_victory);
        app.update();
        let burning = |body| app.world().get::<Empowered>(body).is_some();
        assert_eq!(
            [burning(first), burning(second), burning(elsewhere)],
            [false, false, true],
            "(first player of the room, second player of the room, a player of another room)"
        );
    }

    /// ⭐ VICTORY RETIRES THE QUASAR, NOT SOMEBODY ELSE'S EMPOWERMENT (effects
    /// 6b). A body whose empowerment another granter holds keeps it when the
    /// flag ends her quasar; the quasar's own grant goes with the quasar.
    #[test]
    fn victory_retires_the_quasar_and_not_another_granters_empowerment() {
        let foreign = Empowered::held(Empowerment::UNTOUCHABLE);
        let mut app = App::new();
        let ours = app.world_mut().spawn((PlayerEntity, PocketQuasar { remaining: 10.0 }, QUASAR_GRANT)).id();
        let theirs = app.world_mut().spawn((PlayerEntity, PocketQuasar { remaining: 10.0 }, foreign)).id();
        app.world_mut().spawn(FlagSequence { phase: FlagPhase::Sliding { score: 400 }, ..Default::default() });
        app.add_systems(Update, end_star_power_at_victory);
        app.update();
        let world = app.world();
        assert!(world.get::<PocketQuasar>(ours).is_none() && world.get::<Empowered>(ours).is_none(), "her quasar and its grant end");
        assert!(world.get::<PocketQuasar>(theirs).is_none(), "the other body's quasar ends too");
        assert_eq!(world.get::<Empowered>(theirs), Some(&foreign), "but the empowerment another granter holds stays");
    }
}
