//! Guards at play: where each one is this tick, the strikes they turn aside,
//! and what a turned strike does to the two bodies.
//!
//! A character's guards are authored on its catalog row
//! ([`ambition_characters::actor::guard`]). Each tick, [`publish_guard_volumes`]
//! places each guard of a body where its art draws it ([`GuardVolumes`]). The
//! strike sweep (`hitbox::apply_hitbox_damage`) asks the guards before the
//! body: a strike that meets a guard from a side it covers ([`GuardPiece::covers`])
//! is DEFLECTED ([`DeflectedBodyHit`]) and reaches nothing behind it.
//! [`resolve_deflects`] then:
//!
//! - throws the body that swung back from the guard, ends its swing and locks
//!   it out of acting for the guard's `lockout_s`;
//! - pushes the guarded body back a little, and freezes both at the contact;
//! - counts the deflect toward the guard breaking ([`GuardState`]). A broken
//!   guard covers nothing while its body staggers.
//!
//! The guarded body's brain sees each deflect (`GuardState::deflects` rising)
//! and can answer inside the attacker's lockout.

use bevy::prelude::*;

use ambition_characters::actor::{BodyCombat, GuardFaces, GuardSpec, WornCharacter};
use ambition_characters::prepared::PreparedCharacterRegistry;
use ambition_platformer2d_core as ae;

use crate::body_landmarks::{rig_point_in_world, BodyLandmarks};

/// One guard of a body where it is this tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GuardPiece {
    /// Its index in the body's authored guards.
    pub guard: u8,
    /// Where it is, in the world.
    pub volume: ae::Aabb,
    /// The sides it covers.
    pub faces: GuardFaces,
    /// The way the body faces: `+1` or `-1` along its frame's right.
    pub facing: f32,
    /// A strike from above its top goes over it.
    pub open_above: bool,
}

impl GuardPiece {
    /// Does it cover a strike that comes from `source`, for a body whose unit
    /// DOWN is `down`?
    pub fn covers(&self, source: ae::Vec2, down: ae::Vec2) -> bool {
        let corners = [
            self.volume.min,
            ae::Vec2::new(self.volume.max.x, self.volume.min.y),
            ae::Vec2::new(self.volume.min.x, self.volume.max.y),
            self.volume.max,
        ];
        if self.open_above {
            // Over the top: the source is further up than any of the guard.
            let top = corners.iter().map(|corner| corner.dot(down)).fold(f32::INFINITY, f32::min);
            if source.dot(down) < top {
                return false;
            }
        }
        match self.faces {
            GuardFaces::Around => true,
            GuardFaces::Front => {
                let right = ae::Vec2::new(down.y, -down.x);
                let middle = (self.volume.min + self.volume.max) * 0.5;
                (source - middle).dot(right) * self.facing > 0.0
            }
        }
    }
}

/// Every guard of a body where it is this tick: re-derived each tick from its
/// authored guards, its art and its clocks, before any strike is resolved.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct GuardVolumes {
    pub pieces: Vec<GuardPiece>,
}

/// The rollback state of a guarded body's guards.
///
/// The body's, not its form's: a change of form is the same body (Q159), so a
/// break, its stagger and the deflect count go through it, also through a
/// form with no guards (`a_broken_guard_stays_broken_across_a_change_of_form`).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct GuardState {
    /// Deflects in the current run toward a break.
    pub run: u8,
    /// Seconds since the last deflect.
    pub since_deflect_s: f32,
    /// How many strikes its guards have turned aside. A brain sees a new one
    /// as this rising.
    pub deflects: u32,
    /// Broken: seconds of stagger left, with its guards down.
    pub broken_s: f32,
}

impl GuardState {
    pub fn broken(&self) -> bool {
        self.broken_s > 0.0
    }

    /// Count a strike of `damage` that `spec` turned aside. `true`: it broke,
    /// and the body staggers for the guard's `stagger_s`.
    pub fn count_deflect(&mut self, spec: &GuardSpec, damage: i32) -> bool {
        let in_run = spec.breaks.is_some_and(|breaks| self.since_deflect_s <= breaks.within_s);
        self.run = if in_run { self.run.saturating_add(1) } else { 1 };
        self.since_deflect_s = 0.0;
        self.deflects = self.deflects.wrapping_add(1);
        let Some(breaks) = spec.breaks else {
            return false;
        };
        if self.run >= breaks.blocks || breaks.heavy.is_some_and(|heavy| damage >= heavy) {
            self.broken_s = breaks.stagger_s;
            self.run = 0;
            return true;
        }
        false
    }
}

/// A strike a guard turned aside. It did not reach the body: no damage, no
/// knockback, and no `LandedBodyHit`.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct DeflectedBodyHit {
    /// The guarded body.
    pub defender: Entity,
    /// The body that swung.
    pub attacker: Entity,
    /// The strike that was turned.
    pub hitbox: Entity,
    /// Which of the defender's guards turned it.
    pub guard: u8,
    /// Where they met, in the world.
    pub contact: ae::Vec2,
    /// The strike's damage, for a guard that breaks under a heavy one.
    pub damage: i32,
}

/// The authored guards of a body: its worn character's.
fn guards_of<'a>(characters: Option<&'a PreparedCharacterRegistry>, worn: Option<&WornCharacter>) -> &'a [GuardSpec] {
    characters
        .zip(worn)
        .and_then(|(characters, worn)| characters.get(worn.id()))
        .map_or(&[], |prepared| prepared.guards.as_slice())
}

/// Place every guard of every guarded body where its art draws it this tick.
///
/// A body gets its [`GuardVolumes`] and [`GuardState`] the first tick it wears
/// a character with guards. A dead or broken body's guards cover nothing, and
/// so does a guard in a row it is lowered in.
#[allow(clippy::type_complexity)]
pub fn publish_guard_volumes(
    mut commands: Commands,
    characters: Option<Res<PreparedCharacterRegistry>>,
    landmarks: BodyLandmarks,
    mut bodies: Query<(
        Entity,
        &WornCharacter,
        &ae::BodyKinematics,
        Option<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
        Option<&ambition_characters::actor::BodyHealth>,
        Option<&GuardState>,
        Option<&mut GuardVolumes>,
    )>,
) {
    for (body, worn, kin, frame, health, state, volumes) in &mut bodies {
        let guards = guards_of(characters.as_deref(), Some(worn));
        if guards.is_empty() {
            if let Some(mut volumes) = volumes.filter(|volumes| !volumes.pieces.is_empty()) {
                volumes.pieces.clear();
            }
            continue;
        }
        let down = frame.map_or(ae::DEFAULT_GRAVITY_DIR, |frame| frame.down());
        let up = health.is_none_or(|health| health.alive()) && !state.is_some_and(GuardState::broken);
        let mut pieces = Vec::with_capacity(guards.len());
        for (index, spec) in guards.iter().enumerate().filter(|_| up) {
            let Some((rect, clip)) = landmarks.part_box(body, spec.part, spec.inset_px) else {
                continue;
            };
            if spec.lowered_in.iter().any(|row| *row == clip) {
                continue;
            }
            let corners = [rect.min, ae::Vec2::new(rect.max.x, rect.min.y), ae::Vec2::new(rect.min.x, rect.max.y), rect.max]
                .map(|corner| rig_point_in_world(corner, kin, down));
            let min = corners.iter().copied().reduce(ae::Vec2::min).unwrap_or(kin.pos);
            let max = corners.iter().copied().reduce(ae::Vec2::max).unwrap_or(kin.pos);
            pieces.push(GuardPiece {
                guard: index as u8,
                volume: ae::Aabb { min, max },
                faces: spec.faces,
                facing: if kin.facing < 0.0 { -1.0 } else { 1.0 },
                open_above: spec.open_above,
            });
        }
        match volumes {
            Some(mut volumes) => {
                if volumes.pieces != pieces {
                    volumes.pieces = pieces;
                }
            }
            None => {
                commands.entity(body).try_insert(GuardVolumes { pieces });
            }
        }
        if state.is_none() {
            commands.entity(body).try_insert(GuardState::default());
        }
    }
}

/// Advance every guard's clocks: the time since its last deflect, and its
/// stagger.
pub fn tick_guard_states(
    world_time: Res<ambition_time::WorldTime>,
    mut states: Query<(&mut GuardState, Option<&ambition_time::ProperTimeScale>)>,
) {
    for (mut state, scale) in &mut states {
        let dt = world_time.entity_dt(scale.copied().unwrap_or_default());
        let state = &mut *state;
        state.since_deflect_s += dt;
        state.broken_s = (state.broken_s - dt).max(0.0);
    }
}

/// What each deflected strike does to the two bodies (the module docs).
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn resolve_deflects(
    mut commands: Commands,
    mut deflects: MessageReader<DeflectedBodyHit>,
    characters: Option<Res<PreparedCharacterRegistry>>,
    worn: Query<&WornCharacter>,
    frames: Query<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
    mut bodies: Query<(
        &mut ae::BodyKinematics,
        &mut BodyCombat,
        Option<&mut ae::BodyFlightState>,
        Option<&mut ae::BodyGroundState>,
    )>,
    mut states: Query<&mut GuardState>,
    mut playing: Query<&mut crate::moveset::MovePlayback>,
    mut sfx: ambition_sfx::SfxWriter,
    mut vfx: ambition_vfx::vfx::VfxWriter,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
) {
    for deflect in deflects.read() {
        let Some(spec) = guards_of(characters.as_deref(), worn.get(deflect.defender).ok()).get(usize::from(deflect.guard))
        else {
            continue;
        };
        let feel = spec.deflect;
        let down_of = |body: Entity| frames.get(body).map_or(ae::DEFAULT_GRAVITY_DIR, |frame| frame.down());
        let defender_at = bodies.get(deflect.defender).map(|(kin, ..)| kin.pos).ok();
        let attacker_at = bodies.get(deflect.attacker).map(|(kin, ..)| kin.pos).ok();
        // Which way along the right of a body whose unit down is `down` the
        // attacker is from the guarded body.
        let side = |down: ae::Vec2| match (defender_at, attacker_at) {
            (Some(defender), Some(attacker)) if (attacker - defender).dot(ae::Vec2::new(down.y, -down.x)) < 0.0 => -1.0,
            _ => 1.0,
        };
        // Each body is thrown in its OWN frame. The attacker's run is carried
        // along its own right; a run measured on the guarded body's right put
        // a body in another gravity on axes it does not move along.
        // A body is thrown along its frame's right through the knockback road:
        // the launch is staged, and its run is carried while the body cannot
        // act (`hit_reaction`), so its own walk does not stop it dead. Flinchless:
        // it is thrown, not struck. A throw with lift leaves the ground on the
        // tick it is staged, as a pogo bounce does: a launch too weak to tumble
        // keeps the body planted, and its footing takes the run.
        let throw = |kin: &mut ae::BodyKinematics,
                     flight: Option<&mut ae::BodyFlightState>,
                     ground: Option<&mut ae::BodyGroundState>,
                     down: ae::Vec2,
                     along: f32,
                     lift: f32,
                     hold: f32| {
            let right = ae::Vec2::new(down.y, -down.x);
            let fall = kin.vel.dot(down);
            let launch = right * along + down * if lift > 0.0 { fall.min(-lift) } else { fall };
            if let Some(ground) = ground.filter(|_| lift > 0.0) {
                ground.on_ground = false;
            }
            match flight {
                Some(flight) => {
                    flight.stage_launch(launch, true);
                    flight.carried_run = along;
                    flight.carried_hold = flight.carried_hold.max(hold);
                }
                None => kin.vel = launch,
            }
        };
        // The body that swung: its swing ends, it is thrown back, and it can
        // do nothing for the lockout.
        if let Ok(mut playback) = playing.get_mut(deflect.attacker) {
            crate::moveset::cancel_move_playback(
                &mut commands,
                deflect.attacker,
                &mut playback,
                crate::moveset::MoveEnd::Interrupted,
            );
        }
        if let Ok((mut kin, mut combat, flight, ground)) = bodies.get_mut(deflect.attacker) {
            let down = down_of(deflect.attacker);
            throw(
                &mut kin,
                flight.map(Mut::into_inner),
                ground.map(Mut::into_inner),
                down,
                side(down) * feel.rebound_speed,
                feel.rebound_lift,
                feel.lockout_s,
            );
            combat.recoil_lock_timer = combat.recoil_lock_timer.max(feel.lockout_s);
            combat.hitstop_timer = combat.hitstop_timer.max(feel.hitstop_s);
        }
        // The guarded body gives a little ground, and counts the deflect.
        let broke = states
            .get_mut(deflect.defender)
            .is_ok_and(|mut state| state.count_deflect(spec, deflect.damage));
        if let Ok((mut kin, mut combat, flight, ground)) = bodies.get_mut(deflect.defender) {
            let down = down_of(deflect.defender);
            throw(
                &mut kin,
                flight.map(Mut::into_inner),
                ground.map(Mut::into_inner),
                down,
                -side(down) * feel.push,
                0.0,
                feel.hitstop_s,
            );
            combat.hitstop_timer = combat.hitstop_timer.max(feel.hitstop_s);
            if broke {
                // Staggered: open, and acting on nothing.
                let stagger = spec.breaks.map_or(0.0, |breaks| breaks.stagger_s);
                combat.recoil_lock_timer = combat.recoil_lock_timer.max(stagger);
            }
        }
        let cue = if broke {
            ambition_sfx::ids::PLAYER_ROBOT_SLASH_IMPACT_METAL_GONG
        } else {
            ambition_sfx::ids::PLAYER_ROBOT_SLASH_IMPACT_METAL_CHINK
        };
        sfx.write(ambition_sfx::SfxMessage::Play { id: cue, pos: deflect.contact });
        let mut room = vfx.for_room(rooms.of(deflect.defender));
        room.write(ambition_vfx::vfx::VfxMessage::Impact { pos: deflect.contact });
        room.write(ambition_vfx::vfx::VfxMessage::Burst {
            pos: deflect.contact,
            count: if broke { 24 } else { 10 },
            speed: if broke { 320.0 } else { 220.0 },
            color: [0.7, 0.88, 1.0, 1.0],
            kind: ambition_vfx::vfx::ParticleKind::Spark,
        });
    }
}

#[cfg(test)]
mod tests;
