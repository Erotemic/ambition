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

use crate::body_landmarks::BodyLandmarks;
use ae::frame::thread::{MaterialPrimitive, Partition, Refusal, RefusalReason, VolumePiece};

/// One guard of a body where it is this tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GuardPiece {
    /// Its index in the body's authored guards.
    pub guard: u8,
    /// The exact region in canonical body coordinates.
    pub domain: ae::Aabb,
    /// The accepted body material map, or its numerical refusal.
    pub material: Result<ae::frame::Sim2, ae::frame::NumericFault>,
    /// The sides it covers.
    pub faces: GuardFaces,
    /// The body's facing along its canonical x axis.
    pub facing: f32,
    /// A strike from above its top goes over it.
    pub open_above: bool,
}

impl GuardPiece {
    fn numeric(fault: ae::frame::NumericFault) -> Refusal {
        Refusal { path: Vec::new(), link: None, reason: RefusalReason::Numeric(fault) }
    }

    pub fn world_volume(&self) -> Result<ae::CombatVolume, Refusal> {
        ae::frame::thread::map_volume(&ae::CombatVolume::aabb(self.domain),
            &self.material.map_err(Self::numeric)?).map_err(Self::numeric)
    }

    /// Use the same canonical region for contact and portal partition.
    pub fn pieces(&self, passages: Option<&ambition_platformer2d_shared_tangle::body::geometry::BodyGeometry>,
        body: Entity, room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
    ) -> Partition {
        let domain = ae::CombatVolume::aabb(self.domain);
        let material = MaterialPrimitive { domain: &domain, to_world: self.material.map_err(Self::numeric)? };
        match passages {
            Some(passages) => passages.body_pieces(material, body, room),
            None => Ok(vec![VolumePiece::from_material(material).map_err(Self::numeric)?]),
        }
    }

    /// Test coverage in the guard's canonical axes after undoing its root map.
    pub fn covers(&self, source: ae::Vec2) -> Result<bool, Refusal> {
        let source = self.material.and_then(|map| map.inverse()?.point(source)).map_err(Self::numeric)?;
        if self.open_above && source.y < self.domain.min.y { return Ok(false); }
        Ok(match self.faces {
            GuardFaces::Around => true,
            GuardFaces::Front => (source.x - (self.domain.min.x + self.domain.max.x) * 0.5) * self.facing > 0.0,
        })
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

/// One body's root chart at a guard contact.
#[derive(Clone, Copy, Debug, PartialEq)]
struct DeflectChart {
    into_contact: ae::frame::Sim2,
    // Root velocity that keeps this contact point fixed in the contact chart.
    stationary_velocity: ae::Vec2,
}

impl DeflectChart {
    fn at(piece: &VolumePiece, point: ae::Vec2,
        admission: Option<ae::frame::thread::MotionAdmission<'_>>,
    ) -> Result<Self, Refusal> {
        let stationary_velocity = match admission {
            Some(admission) => admission.uncarry_velocity(&piece.path, point, ae::Vec2::ZERO, 1.0)?,
            None if piece.path.is_empty() => ae::Vec2::ZERO,
            None => return Err(Refusal { path: piece.path.clone(), link: None, reason: RefusalReason::MissingMaterial }),
        };
        Ok(Self { into_contact: piece.path_transform().map_err(GuardPiece::numeric)?, stationary_velocity })
    }

    fn velocity(self, velocity: ae::Vec2) -> Result<ae::Vec2, ae::frame::NumericFault> {
        self.into_contact.vector_from_wide(velocity.as_dvec2() - self.stationary_velocity.as_dvec2())
    }

    fn prepare_throw(self, kin: &ae::BodyKinematics, down: ae::Vec2,
        out: Option<ae::Vec2>, default_side: f32, speed: f32, lift: f32, contact_velocity: ae::Vec2,
    ) -> Result<PreparedThrow, ae::frame::NumericFault> {
        use bevy::math::DVec2;
        let right = ae::Vec2::new(down.y, -down.x);
        let contact_down = self.into_contact.direction(down).as_dvec2();
        let contact_right = self.into_contact.direction(right).as_dvec2();
        let velocity = self.velocity(kin.vel)?.as_dvec2() - contact_velocity.as_dvec2();
        let side = out.map_or(f64::from(default_side), |out| if contact_right.dot(out.as_dvec2()) < 0.0 { -1.0 } else { 1.0 });
        let along = side * f64::from(speed);
        let fall = velocity.dot(contact_down);
        let lifted = contact_right * along + contact_down * if lift > 0.0 { fall.min(-f64::from(lift)) } else { fall };
        let launch = match out.filter(|out| lifted.dot(out.as_dvec2()) <= 0.0) {
            Some(out) => {
                let away = out.as_dvec2().dot(contact_down) * along.abs();
                contact_right * along + contact_down * if away > 0.0 { fall.max(away) } else { fall.min(away) }
            }
            None => lifted,
        };
        let into_root = self.into_contact.inverse()?;
        let leaves_ground = into_root.vector_from_wide(launch)?.as_dvec2().dot(down.as_dvec2()) < 0.0;
        let root = into_root.vector_from_wide(launch + contact_velocity.as_dvec2())?;
        let launch = ae::frame::Sim2::IDENTITY.vector_from_wide(root.as_dvec2() + self.stationary_velocity.as_dvec2())?;
        let run = f64::from(launch.x) * f64::from(right.x) + f64::from(launch.y) * f64::from(right.y);
        let run = ae::frame::Sim2::IDENTITY.vector_from_wide(DVec2::new(run, 0.0))?.x;
        Ok(PreparedThrow { launch, run, leaves_ground })
    }
}

/// The contact chart and both root charts, retained from exact guard contact.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeflectContact {
    pub point: ae::Vec2,
    attacker: DeflectChart,
    defender: DeflectChart,
    outward: Option<ae::Vec2>,
    guard_material: ae::frame::Sim2,
}

impl DeflectContact {
    pub(crate) fn at(strike: &VolumePiece, guard: &VolumePiece, source: ae::Vec2,
        defender: ae::Vec2, point: ae::Vec2,
        strike_admission: Option<ae::frame::thread::MotionAdmission<'_>>,
        guard_admission: Option<ae::frame::thread::MotionAdmission<'_>>,
    ) -> Result<Self, Refusal> {
        let source = strike.carry(source).map_err(GuardPiece::numeric)?;
        let defender = guard.carry(defender).map_err(GuardPiece::numeric)?;
        let outward = (source.as_dvec2() - defender.as_dvec2()).try_normalize().map(|v| v.as_vec2());
        Ok(Self { point,
            attacker: DeflectChart::at(strike, point, strike_admission)?,
            defender: DeflectChart::at(guard, point, guard_admission)?,
            outward, guard_material: guard.transform(),
        })
    }
}

struct PreparedThrow {
    launch: ae::Vec2,
    run: f32,
    leaves_ground: bool,
}

impl PreparedThrow {
    fn commit(self, kin: &mut ae::BodyKinematics, flight: Option<&mut ae::BodyFlightState>,
        ground: Option<&mut ae::BodyGroundState>, hold: f32,
    ) {
        if let Some(ground) = ground.filter(|_| self.leaves_ground) { ground.on_ground = false; }
        match flight {
            Some(flight) => {
                flight.stage_launch(self.launch, true);
                flight.carried_run = self.run;
                flight.carried_hold = flight.carried_hold.max(hold);
            }
            None => kin.vel = self.launch,
        }
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
    /// The exact contact and its chart conversions.
    pub contact: DeflectContact,
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
        Option<&crate::body_rig::RigFeetOffset>,
        Option<&ambition_characters::actor::BodyHealth>,
        Option<&GuardState>,
        Option<&mut GuardVolumes>,
    )>,
) {
    for (body, worn, kin, feet, health, state, volumes) in &mut bodies {
        let guards = guards_of(characters.as_deref(), Some(worn));
        if guards.is_empty() {
            if let Some(mut volumes) = volumes.filter(|volumes| !volumes.pieces.is_empty()) {
                volumes.pieces.clear();
            }
            continue;
        }
        let feet = crate::body_rig::rig_feet_from_centre(feet, Some(kin));
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
                .map(|corner| crate::body_rig::BodyRigPose::to_body(feet + corner, kin.facing, ae::Vec2::Y));
            let min = corners.iter().copied().reduce(ae::Vec2::min).unwrap_or(kin.pos);
            let max = corners.iter().copied().reduce(ae::Vec2::max).unwrap_or(kin.pos);
            pieces.push(GuardPiece {
                guard: index as u8,
                domain: ae::Aabb { min, max },
                material: kin.material_map(),
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
    mut states: Query<(Entity, &mut GuardState, Option<&ambition_time::ProperTimeScale>)>,
    // A connect's room holds its bodies' guards too (ISL-1b).
    freeze: crate::impact_hitstop::RoomFreeze,
) {
    for (entity, mut state, scale) in &mut states {
        let dt = world_time.entity_dt(scale.copied().unwrap_or_default()) * freeze.scale_of(entity);
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
        // Prepare both launches before any body, guard, or move state changes.
        let prepared = (|| {
            let attacker = bodies.get(deflect.attacker).ok().map(|(kin, ..)| kin);
            let defender = bodies.get(deflect.defender).ok().map(|(kin, ..)| kin);
            // The guard sets the contact frame's velocity. A common boost must
            // not change either body's relative response.
            let contact_velocity = defender.map(|kin| deflect.contact.defender.velocity(kin.vel)).transpose()?.unwrap_or(ae::Vec2::ZERO);
            let material = deflect.contact.guard_material;
            let rebound = material.length(feel.rebound_speed)?;
            let lift = material.length(feel.rebound_lift)?;
            let push = material.length(feel.push)?;
            let attacker = attacker.map(|kin| deflect.contact.attacker.prepare_throw(kin,
                down_of(deflect.attacker), deflect.contact.outward, 1.0, rebound, lift, contact_velocity)).transpose()?;
            let defender = defender.map(|kin| deflect.contact.defender.prepare_throw(kin,
                down_of(deflect.defender), deflect.contact.outward.map(|out| -out), -1.0, push, 0.0, contact_velocity)).transpose()?;
            Ok::<_, ae::frame::NumericFault>((attacker, defender))
        })();
        let (attacker_throw, defender_throw) = match prepared {
            Ok(prepared) => prepared,
            Err(fault) => {
                warn!(?fault, attacker = ?deflect.attacker, defender = ?deflect.defender, "guard response refused");
                continue;
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
        if let (Some(prepared), Ok((mut kin, mut combat, flight, ground))) =
            (attacker_throw, bodies.get_mut(deflect.attacker)) {
            prepared.commit(&mut kin, flight.map(Mut::into_inner), ground.map(Mut::into_inner), feel.lockout_s);
            combat.recoil_lock_timer = combat.recoil_lock_timer.max(feel.lockout_s);
            combat.hitstop_timer = combat.hitstop_timer.max(feel.hitstop_s);
        }
        // The guarded body gives a little ground, and counts the deflect.
        let broke = states
            .get_mut(deflect.defender)
            .is_ok_and(|mut state| state.count_deflect(spec, deflect.damage));
        if let (Some(prepared), Ok((mut kin, mut combat, flight, ground))) =
            (defender_throw, bodies.get_mut(deflect.defender)) {
            prepared.commit(&mut kin, flight.map(Mut::into_inner), ground.map(Mut::into_inner), feel.hitstop_s);
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
        sfx.in_space(ambition_vfx::sfx_space(rooms.of(deflect.defender))).write(ambition_sfx::SfxMessage::Play { id: cue, emitter: deflect.contact.point.into() });
        let mut room = vfx.for_room(rooms.of(deflect.defender));
        room.write(ambition_vfx::vfx::VfxMessage::Impact { pos: deflect.contact.point, blow: ambition_vfx::ImpactBlow::PLAIN });
        room.write(ambition_vfx::vfx::VfxMessage::Burst {
            pos: deflect.contact.point,
            count: if broke { 24 } else { 10 },
            speed: if broke { 320.0 } else { 220.0 },
            color: [0.7, 0.88, 1.0, 1.0],
            kind: ambition_vfx::vfx::ParticleKind::Spark,
        });
    }
}

#[cfg(test)]
mod tests;
