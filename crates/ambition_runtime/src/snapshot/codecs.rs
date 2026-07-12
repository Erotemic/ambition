//! The engine's per-type `SnapshotState` / `SnapshotCursor` / `SnapshotResolve`
//! codecs, the `snapshot_pod!` / `snapshot_unit_enum!` code generators, the
//! `PasteEncode` overloads, and the `SimId` minting helpers.
//!
//! Split out of the former `snapshot.rs` for the D-B module-size gate; shares the
//! module core via `use super::*`. `body_clusters as bc` is declared here AND in
//! `mod.rs` (`register_engine_sim_state`, which stays in `mod.rs`, also needs it).
use super::*;

// ── The engine's codecs ──────────────────────────────────────────────────────
//
// Explicit field order, fixed-width LE, every field present. A codec that skips a
// field the sim reads is a restore that silently rewinds to a different world; the
// round-trip oracle in this module's tests is what catches one.

impl SnapshotState for ambition_time::SimTick {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u64(out, self.0);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(ambition_time::SimTick(r.u64()?))
    }
}

impl SnapshotState for ambition_time::WorldTime {
    fn encode(&self, out: &mut Vec<u8>) {
        put_f32(out, self.raw_dt);
        put_f32(out, self.scaled_dt);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(ambition_time::WorldTime {
            raw_dt: r.f32()?,
            scaled_dt: r.f32()?,
        })
    }
}

impl SnapshotState for ambition_engine_core::AbilitySet {
    fn encode(&self, out: &mut Vec<u8>) {
        put_bool(out, self.move_horizontal);
        put_bool(out, self.jump);
        put_bool(out, self.variable_jump);
        put_bool(out, self.double_jump);
        put_bool(out, self.fast_fall);
        put_bool(out, self.wall_jump);
        put_bool(out, self.wall_cling);
        put_bool(out, self.wall_climb);
        put_bool(out, self.dash);
        put_bool(out, self.double_dash);
        put_bool(out, self.fly);
        put_bool(out, self.blink);
        put_bool(out, self.precision_blink);
        put_bool(out, self.blink_through_soft_walls);
        put_bool(out, self.blink_through_hard_walls);
        put_bool(out, self.attack);
        put_bool(out, self.pogo);
        put_bool(out, self.directional_primary);
        put_bool(out, self.directional_special);
        put_bool(out, self.rebound);
        put_bool(out, self.reset);
        put_bool(out, self.ledge_grab);
        put_bool(out, self.swim);
        put_bool(out, self.glide);
        put_bool(out, self.dodge);
        put_bool(out, self.shield);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self {
            move_horizontal: r.bool()?,
            jump: r.bool()?,
            variable_jump: r.bool()?,
            double_jump: r.bool()?,
            fast_fall: r.bool()?,
            wall_jump: r.bool()?,
            wall_cling: r.bool()?,
            wall_climb: r.bool()?,
            dash: r.bool()?,
            double_dash: r.bool()?,
            fly: r.bool()?,
            blink: r.bool()?,
            precision_blink: r.bool()?,
            blink_through_soft_walls: r.bool()?,
            blink_through_hard_walls: r.bool()?,
            attack: r.bool()?,
            pogo: r.bool()?,
            directional_primary: r.bool()?,
            directional_special: r.bool()?,
            rebound: r.bool()?,
            reset: r.bool()?,
            ledge_grab: r.bool()?,
            swim: r.bool()?,
            glide: r.bool()?,
            dodge: r.bool()?,
            shield: r.bool()?,
        })
    }
}

/// The host-code playable kit is derived from this component. Registering the
/// identity without its derivation input made a restore depend on whatever
/// abilities happened to be live at restore time; both now rewind together.
impl SnapshotState for bc::BodyAbilities {
    fn encode(&self, out: &mut Vec<u8>) {
        self.abilities.encode(out);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(bc::BodyAbilities::new(
            <ambition_engine_core::AbilitySet as SnapshotState>::decode(r)?,
        ))
    }
}

impl SnapshotState for BodyKinematics {
    fn encode(&self, out: &mut Vec<u8>) {
        put_vec2(out, self.pos);
        put_vec2(out, self.vel);
        put_vec2(out, self.size);
        put_f32(out, self.facing);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(BodyKinematics {
            pos: r.vec2()?,
            vel: r.vec2()?,
            size: r.vec2()?,
            facing: r.f32()?,
        })
    }
}

// The mutable body-state clusters. These are what a rewind is FOR: a coyote timer
// that survives a rollback is a jump the player did not earn.
#[macro_export]
macro_rules! snapshot_pod {
    ($ty:path { $($field:ident : $get:ident),+ $(,)? }) => {
        impl SnapshotState for $ty {
            fn encode(&self, out: &mut Vec<u8>) {
                $( $crate::snapshot::paste_put(out, self.$field); )+
            }
            fn decode(r: &mut Reader<'_>) -> Option<Self> {
                Some(Self { $( $field: r.$get()? ),+ })
            }
        }
    };
}

/// One overload per encodable primitive, so `snapshot_pod!` does not have to name
/// the writer twice. The reader cannot do this — `Option<T>` inference would need
/// the type back — so the macro names the getter and infers the putter.
pub trait PasteEncode: Copy {
    fn put(self, out: &mut Vec<u8>);
}
impl PasteEncode for f32 {
    fn put(self, out: &mut Vec<u8>) {
        put_f32(out, self);
    }
}
impl PasteEncode for bool {
    fn put(self, out: &mut Vec<u8>) {
        put_bool(out, self);
    }
}
impl PasteEncode for u8 {
    fn put(self, out: &mut Vec<u8>) {
        put_u8(out, self);
    }
}
impl PasteEncode for u32 {
    fn put(self, out: &mut Vec<u8>) {
        put_u32(out, self);
    }
}
impl PasteEncode for i32 {
    fn put(self, out: &mut Vec<u8>) {
        put_i32(out, self);
    }
}
impl PasteEncode for bevy::math::Vec2 {
    fn put(self, out: &mut Vec<u8>) {
        put_vec2(out, self);
    }
}
pub fn paste_put<T: PasteEncode>(out: &mut Vec<u8>, v: T) {
    v.put(out);
}

use ambition_engine_core::body_clusters as bc;

/// **A unit enum's wire discriminant, written down.**
///
/// The mapping is EXPLICIT and the numbers are load-bearing: reordering a variant in
/// its `enum` must never silently reinterpret a snapshot. Declaration order is a
/// refactor away from being a different order, and `#[derive(Default)]` on a variant
/// makes it look reorderable. Adding a variant means adding a number; changing one
/// means breaking every stored blob, which is what a version tag would be for.
///
/// An unknown discriminant decodes to `None`, not to the default: a blob this build
/// cannot read is a bug to surface, not a state to guess.
#[macro_export]
macro_rules! snapshot_unit_enum {
    ($ty:path { $($variant:ident = $code:literal),+ $(,)? }) => {
        impl SnapshotState for $ty {
            fn encode(&self, out: &mut Vec<u8>) {
                #[allow(unused_imports)]
                use $ty as E;
                put_u8(
                    out,
                    match self {
                        $( E::$variant => $code ),+
                    },
                );
            }
            fn decode(r: &mut Reader<'_>) -> Option<Self> {
                #[allow(unused_imports)]
                use $ty as E;
                match r.u8()? {
                    $( $code => Some(E::$variant), )+
                    _ => None,
                }
            }
        }
    };
}

snapshot_unit_enum!(ambition_engine_core::player_state::BodyMode {
    Standing = 0,
    Crouching = 1,
    Crawling = 2,
    Sliding = 3,
    MorphBall = 4,
    Climbing = 5,
});
snapshot_unit_enum!(ambition_characters::actor::ai::CharacterAiMode {
    Idle = 0,
    Patrol = 1,
    Chase = 2,
    Telegraph = 3,
    Attack = 4,
    Recover = 5,
    Stunned = 6,
    Dead = 7,
});

impl SnapshotState for bc::BodyModeState {
    fn encode(&self, out: &mut Vec<u8>) {
        self.body_mode.encode(out);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(bc::BodyModeState {
            body_mode: ambition_engine_core::player_state::BodyMode::decode(r)?,
        })
    }
}

impl SnapshotState for ambition_actors::features::ActorStatus {
    fn encode(&self, out: &mut Vec<u8>) {
        put_f32(out, self.respawn_timer);
        self.ai_mode.encode(out);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(ambition_actors::features::ActorStatus {
            respawn_timer: r.f32()?,
            ai_mode: ambition_characters::actor::ai::CharacterAiMode::decode(r)?,
        })
    }
}

impl SnapshotState for ambition_combat::components::ActorIntent {
    fn encode(&self, out: &mut Vec<u8>) {
        self.0.encode(out);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(ambition_combat::components::ActorIntent(
            ambition_characters::actor::ai::CharacterAiMode::decode(r)?,
        ))
    }
}

snapshot_pod!(bc::BodyGroundState {
    on_ground: bool,
    coyote_timer: f32,
    drop_through_timer: f32,
    rebound_cooldown: f32,
});
snapshot_pod!(bc::BodyWallState {
    on_wall: bool,
    wall_normal_x: f32,
    wall_clinging: bool,
    wall_climbing: bool,
    pre_wall_vel: vec2,
    pre_wall_vel_age: f32,
});
snapshot_pod!(bc::BodyJumpState {
    air_jumps_available: u8,
    ladder_jump_boost: f32,
    ladder_drop_through_timer: f32,
    ladder_drop_through_hold_lock: bool,
});
snapshot_pod!(bc::BodyDashState {
    charges_available: u8,
    timer: f32,
    cooldown: f32,
});
snapshot_pod!(bc::BodyFlightState {
    fly_enabled: bool,
    flight_phase: f32,
    gliding: bool,
    fast_falling: bool,
    carried_run: f32,
});
snapshot_pod!(bc::BodyBlinkState {
    cooldown: f32,
    hold_active: bool,
    hold_timer: f32,
    aiming: bool,
    aim_offset: vec2,
    grace_timer: f32,
});
snapshot_pod!(bc::BodyDodgeState {
    roll_timer: f32,
    cooldown: f32,
});
snapshot_pod!(bc::BodyShieldState {
    active: bool,
    parry_window_timer: f32,
});
snapshot_pod!(bc::BodyOffense {
    damage_multiplier: i32,
    invincible: bool,
});
snapshot_pod!(bc::BodyLifetime {
    time_alive: f32,
    resets: u32,
    max_speed: f32,
});
snapshot_pod!(bc::BodyActionBuffer {
    jump: f32,
    dash: f32,
    attack: f32,
    pogo: f32,
    projectile: f32,
    blink: f32,
});
snapshot_pod!(bc::BodyBaseSize { base_size: vec2 });
snapshot_pod!(ambition_actors::features::ActorSurfaceState {
    surface_normal: vec2,
    gravity_scale: f32,
});

snapshot_unit_enum!(ambition_engine_core::ledge_grab::LedgeGetupKind {
    Climb = 0,
    Roll = 1,
    Attack = 2,
});
snapshot_unit_enum!(ambition_engine_core::ledge_grab::LedgeGrabQuality {
    Precise = 0,
    Forgiving = 1,
});
snapshot_unit_enum!(ambition_engine_core::movement::MovementOp {
    Jump = 0,
    DoubleJump = 1,
    WallJump = 2,
    WallCling = 3,
    WallClimb = 4,
    LedgeGrab = 5,
    LedgeJump = 6,
    LedgeClimbStart = 7,
    LedgeClimbFinish = 8,
    LedgeDrop = 9,
    LedgeRoll = 10,
    LedgeGetupAttack = 11,
    SwimStroke = 12,
    Dash = 13,
    DoubleDash = 14,
    DodgeRoll = 15,
    FlyToggle = 16,
    Blink = 17,
    PrecisionBlink = 18,
    Pogo = 19,
    Rebound = 20,
    Slash = 21,
    Reset = 22,
    ShieldUp = 23,
});

/// A body hanging on a ledge. `grab: Option<LedgeGrabState>` is the whole state
/// machine: a rollback into a hang must land on the same anchor, with the same
/// carried momentum, or the getup goes somewhere else.
impl SnapshotState for bc::BodyLedgeState {
    fn encode(&self, out: &mut Vec<u8>) {
        match &self.grab {
            None => put_bool(out, false),
            Some(g) => {
                put_bool(out, true);
                put_f32(out, g.contact.wall_normal_x);
                put_vec2(out, g.contact.anchor);
                put_vec2(out, g.contact.climb_target);
                put_f32(out, g.elapsed);
                put_bool(out, g.climbing);
                g.getup_kind.encode(out);
                put_f32(out, g.climb_elapsed);
                put_vec2(out, g.momentum_at_grab);
                g.grab_quality.encode(out);
            }
        }
        put_f32(out, self.release_cooldown);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use ambition_engine_core::ledge_grab::{
            LedgeContact, LedgeGetupKind, LedgeGrabQuality, LedgeGrabState,
        };
        let grab = if r.bool()? {
            Some(LedgeGrabState {
                contact: LedgeContact {
                    wall_normal_x: r.f32()?,
                    anchor: r.vec2()?,
                    climb_target: r.vec2()?,
                },
                elapsed: r.f32()?,
                climbing: r.bool()?,
                getup_kind: LedgeGetupKind::decode(r)?,
                climb_elapsed: r.f32()?,
                momentum_at_grab: r.vec2()?,
                grab_quality: LedgeGrabQuality::decode(r)?,
            })
        } else {
            None
        };
        Some(bc::BodyLedgeState {
            grab,
            release_cooldown: r.f32()?,
        })
    }
}

/// The recent-movement trace a combo/chain rule reads. A `Vec`, so its order IS its
/// meaning: the ops go out in the order they went in.
impl SnapshotState for bc::BodyComboTrace {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u32(out, self.combo.len() as u32);
        for mark in &self.combo {
            mark.op.encode(out);
            put_f32(out, mark.age);
        }
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use ambition_engine_core::movement::{ComboMark, MovementOp};
        let n = r.u32()?;
        let combo = (0..n)
            .map(|_| {
                Some(ComboMark {
                    op: MovementOp::decode(r)?,
                    age: r.f32()?,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        Some(bc::BodyComboTrace { combo })
    }
}

impl SnapshotState for ambition_combat::components::BodyEnvelope {
    fn encode(&self, out: &mut Vec<u8>) {
        put_vec2(out, self.0);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(ambition_combat::components::BodyEnvelope(r.vec2()?))
    }
}
snapshot_pod!(bc::SweepSample {
    prev: vec2,
    curr: vec2,
    vel: vec2,
    half: vec2,
});

// Actor-side mutable state. An attack cooldown that survives a rollback is an
// attack the enemy did not pay for.
snapshot_pod!(ambition_characters::actor::pose::ActorPose {
    center: vec2,
    feet: vec2,
    facing: f32,
});
snapshot_pod!(ambition_platformer_primitives::orientation::ActorRoll { angle: f32 });
snapshot_pod!(ambition_combat::components::ActorCooldowns {
    attack_cooldown: f32,
    respawn_timer: f32,
});
snapshot_pod!(ambition_engine_core::geometry::CenteredAabb {
    center: vec2,
    half_size: vec2,
});
snapshot_pod!(ambition_engine_core::player_state::ResourceMeter {
    current: f32,
    max: f32,
    regen_rate: f32,
    decay_rate: f32,
});

impl SnapshotState for bc::BodyMana {
    fn encode(&self, out: &mut Vec<u8>) {
        self.meter.encode(out);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(bc::BodyMana {
            meter: ambition_engine_core::player_state::ResourceMeter::decode(r)?,
        })
    }
}

impl SnapshotState for ambition_characters::actor::BodyHealth {
    fn encode(&self, out: &mut Vec<u8>) {
        put_i32(out, self.health.current);
        put_i32(out, self.health.max);
        put_bool(out, self.health.invulnerable);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(ambition_characters::actor::BodyHealth::new(
            ambition_characters::actor::Health {
                current: r.i32()?,
                max: r.i32()?,
                invulnerable: r.bool()?,
            },
        ))
    }
}

/// The canonical playable-persona identity: WHICH catalog character a body
/// wears. A length-delimited string id — the choice, not the content: the
/// catalog it names is authored data that survives the rewind. Registered as a
/// full component (not a resolve) because the id IS the value; the entity's
/// gameplay/presentation are re-derived from the restored identity (and, for
/// HostCode, the restored `BodyAbilities`) the following tick.
impl SnapshotState for ambition_characters::actor::WornCharacter {
    fn encode(&self, out: &mut Vec<u8>) {
        put_str(out, self.id());
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(ambition_characters::actor::WornCharacter::new(r.str()?))
    }
}

/// `ActorTarget` is half derived, half state — see its definition-site snapshot story.
/// `entity` is rebuilt every tick by `select_actor_targets`; `pos` survives the frame
/// where no candidate exists, and a chasing brain aims at it. So `pos` rewinds and
/// `entity` does not.
/// The blob is `(move id, facing, t, landed_hit)`; the `MoveSpec` comes back out of the
/// entity's own `ActorMoveset`, which a patched entity still carries.
///
/// `live_boxes` comes back empty and `fired` is rebuilt from `t` — both by
/// `MovePlayback::resumed`. That is sound because a strike volume's existence is
/// DERIVED from `(t, window)` and `retire_orphaned_strike_volumes` maintains that
/// derivation every frame, so the rewound clock re-creates exactly the boxes it should.
///
/// A move id the moveset no longer knows resolves to `None`, and the component is left
/// off. That is a content change between snapshot and restore — impossible in a
/// rollback, and a loud, correct failure in a save file.
impl SnapshotResolve for ambition_combat::moveset::MovePlayback {
    fn encode_ref(&self, out: &mut Vec<u8>) {
        put_str(out, &self.spec.id);
        put_f32(out, self.facing);
        put_f32(out, self.t);
        put_bool(out, self.landed_hit);
    }

    fn resolve(
        entity: &bevy::ecs::world::EntityWorldMut<'_>,
        r: &mut Reader<'_>,
    ) -> Result<Option<Self>, ResolveDecodeError> {
        // Decode the WHOLE blob first (id + facing + t + landed) — a `Reader`
        // primitive returning `None` is a malformed blob (`Err`), never confused
        // with the move having vanished. Then resolve the id against the entity's
        // still-authored moveset: an absent moveset or an unknown id is `Ok(None)`
        // (a content change), not a decode failure.
        let id = r.str().ok_or(ResolveDecodeError)?;
        let facing = r.f32().ok_or(ResolveDecodeError)?;
        let t = r.f32().ok_or(ResolveDecodeError)?;
        let landed = r.bool().ok_or(ResolveDecodeError)?;
        let Some(moveset) = entity.get::<ambition_combat::moveset::ActorMoveset>() else {
            return Ok(None);
        };
        let Some(spec) = moveset.0.move_by_id(id) else {
            return Ok(None);
        };
        Ok(Some(ambition_combat::moveset::MovePlayback::resumed(
            spec.clone(),
            facing,
            t,
            landed,
        )))
    }
}

/// **The boss's encounter phase**, and the `ActorPhaseState` it is forwarded from.
///
/// A cursor, because the rest of `BossEncounter` is sprite metrics derived from the
/// sheet registry, and because `ActorPhaseState.triggers` is authored data.
///
/// `encounter_phase` is the exposed MIRROR that `sync_boss_encounter_phase` copies out
/// of `encounter` every tick. Rewinding only the mirror is rewinding a thermometer:
/// `mockingbird_arena` telegraphed `wing_sweep` on the replay's tick 21 and stood still
/// on the original's, with every clock, seed, and cooldown identical, because the
/// replay's boss was already awake.
impl SnapshotCursor for ambition_actors::features::BossEncounter {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        self.encounter_phase.encode(out);
        match &self.encounter {
            None => put_bool(out, false),
            Some(e) => {
                put_bool(out, true);
                e.phase.encode(out);
                put_f32(out, e.phase_elapsed);
                put_f32(out, e.transition_lock);
                e.start_phase.encode(out);
            }
        }
    }
    fn apply_cursor(&mut self, r: &mut Reader<'_>) -> Option<()> {
        use ambition_characters::brain::boss_pattern::BossEncounterPhase;
        self.encounter_phase = BossEncounterPhase::decode(r)?;
        if r.bool()? {
            let phase = BossEncounterPhase::decode(r)?;
            let phase_elapsed = r.f32()?;
            let transition_lock = r.f32()?;
            let start_phase = BossEncounterPhase::decode(r)?;
            // The authored `triggers` stay where they are: a snapshot carries what the
            // fight has BECOME, never the rules it became it by.
            if let Some(e) = self.encounter.as_mut() {
                e.phase = phase;
                e.phase_elapsed = phase_elapsed;
                e.transition_lock = transition_lock;
                e.start_phase = start_phase;
            }
        } else {
            self.encounter = None;
        }
        Some(())
    }
}

impl SnapshotCursor for ambition_combat::components::ActorTarget {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        put_vec2(out, self.pos);
    }
    fn apply_cursor(&mut self, r: &mut Reader<'_>) -> Option<()> {
        self.pos = r.vec2()?;
        Some(())
    }
}

impl SnapshotCursor for ambition_actors::features::ActorMotionPath {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        match &self.0 {
            Some(motion) => {
                let (segment, dir) = motion.cursor();
                put_bool(out, true);
                put_u32(out, segment as u32);
                put_i32(out, dir);
            }
            // A body with no path is a state a body with a path can reach.
            None => put_bool(out, false),
        }
    }

    fn apply_cursor(&mut self, r: &mut Reader<'_>) -> Option<()> {
        let has_path = r.bool()?;
        let (segment, dir) = if has_path {
            (r.u32()? as usize, r.i32()?)
        } else {
            // The snapshot's body had no path. Ours does; drop it. The authored path
            // is not recoverable from here, which is exactly what `stale_components`
            // and `respawned` exist to make visible — but a body that GAINED a path
            // after the snapshot must not keep it.
            self.0 = None;
            return Some(());
        };
        if let Some(motion) = self.0.as_mut() {
            motion.set_cursor(segment, dir);
        }
        Some(())
    }
}

snapshot_unit_enum!(ambition_characters::actor::ActorFaction {
    Player = 0,
    Enemy = 1,
    Npc = 2,
    Boss = 3,
    Neutral = 4,
});

/// `Strike(key)` / `Special(key)` — a keyed reference by construction, because "a new
/// geometry strike is a new key + authored rects, with NO edit to this enum".
impl SnapshotState for ambition_characters::brain::boss_pattern::BossAttackProfile {
    fn encode(&self, out: &mut Vec<u8>) {
        use ambition_characters::brain::boss_pattern::BossAttackProfile as P;
        match self {
            P::Strike(key) => {
                put_u8(out, 0);
                put_str(out, key);
            }
            P::Special(key) => {
                put_u8(out, 1);
                put_str(out, key);
            }
        }
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use ambition_characters::brain::boss_pattern::BossAttackProfile as P;
        match r.u8()? {
            0 => Some(P::Strike(r.str()?.to_string())),
            1 => Some(P::Special(r.str()?.to_string())),
            _ => None,
        }
    }
}

fn put_opt_profile(
    out: &mut Vec<u8>,
    v: &Option<ambition_characters::brain::boss_pattern::BossAttackProfile>,
) {
    match v {
        None => put_bool(out, false),
        Some(p) => {
            put_bool(out, true);
            p.encode(out);
        }
    }
}

#[allow(clippy::option_option)]
fn read_opt_profile(
    r: &mut Reader<'_>,
) -> Option<Option<ambition_characters::brain::boss_pattern::BossAttackProfile>> {
    use ambition_characters::brain::boss_pattern::BossAttackProfile as P;
    Some(if r.bool()? { Some(P::decode(r)?) } else { None })
}

impl SnapshotState for ambition_characters::brain::boss_pattern::BossAttackState {
    fn encode(&self, out: &mut Vec<u8>) {
        put_opt_profile(out, &self.telegraph_profile);
        put_f32(out, self.telegraph_remaining);
        put_f32(out, self.telegraph_elapsed);
        match &self.telegraph_spec {
            None => put_bool(out, false),
            Some(spec) => {
                put_bool(out, true);
                put_opt_str(out, spec.pose.as_deref());
                put_opt_str(out, spec.cue.as_deref());
                put_opt_str(out, spec.vfx.as_deref());
            }
        }
        put_opt_profile(out, &self.active_profile);
        put_f32(out, self.active_remaining);
        put_f32(out, self.active_elapsed);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use ambition_characters::brain::boss_pattern::{BossAttackState, TelegraphSpec};
        let telegraph_profile = read_opt_profile(r)?;
        let telegraph_remaining = r.f32()?;
        let telegraph_elapsed = r.f32()?;
        let telegraph_spec = if r.bool()? {
            Some(TelegraphSpec {
                pose: r.opt_str()?.map(str::to_string),
                cue: r.opt_str()?.map(str::to_string),
                vfx: r.opt_str()?.map(str::to_string),
            })
        } else {
            None
        };
        Some(BossAttackState {
            telegraph_profile,
            telegraph_remaining,
            telegraph_elapsed,
            telegraph_spec,
            active_profile: read_opt_profile(r)?,
            active_remaining: r.f32()?,
            active_elapsed: r.f32()?,
        })
    }
}

impl SnapshotState for ambition_characters::brain::boss_pattern::BossAttackIntent {
    fn encode(&self, out: &mut Vec<u8>) {
        put_opt_profile(out, &self.telegraph_profile);
        put_opt_profile(out, &self.active_profile);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(ambition_characters::brain::boss_pattern::BossAttackIntent {
            telegraph_profile: read_opt_profile(r)?,
            active_profile: read_opt_profile(r)?,
        })
    }
}

/// `Omniscient` reads the global `ActorTarget`; `Sighted` carries its viewport. Not a
/// unit enum, so `snapshot_unit_enum!` cannot have it — but the discriminant is still
/// explicit for exactly the same reason.
impl SnapshotState for ambition_actors::features::ecs::perception::Perception {
    fn encode(&self, out: &mut Vec<u8>) {
        use ambition_actors::features::ecs::perception::Perception as P;
        match self {
            P::Omniscient => put_u8(out, 0),
            P::Sighted { viewport_half } => {
                put_u8(out, 1);
                put_vec2(out, *viewport_half);
            }
        }
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use ambition_actors::features::ecs::perception::Perception as P;
        match r.u8()? {
            0 => Some(P::Omniscient),
            1 => Some(P::Sighted {
                viewport_half: r.vec2()?,
            }),
            _ => None,
        }
    }
}

/// The brain's memory of what it has seen — FB5's habit model reads it, and FB6's
/// rollouts cannot run until it rewinds. Ordered by actor id, because `WorldMemory`
/// is a `BTreeMap` (ADR 0023, and a real bug: see `last_known_hostile`).
impl SnapshotState for ambition_actors::features::ecs::perception::PerceptionMemory {
    fn encode(&self, out: &mut Vec<u8>) {
        let rows: Vec<_> = self.0.entries().collect();
        put_u32(out, rows.len() as u32);
        for (id, m) in rows {
            put_str(out, id);
            put_vec2(out, m.pos);
            put_vec2(out, m.vel);
            m.faction.encode(out);
            put_bool(out, m.hostile_to_self);
            put_f32(out, m.last_seen);
            put_f32(out, m.confidence);
        }
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use ambition_characters::perception::{RememberedActor, WorldMemory};
        let n = r.u32()?;
        let mut rows = Vec::with_capacity(n as usize);
        for _ in 0..n {
            let id = r.str()?.to_string();
            rows.push((
                id,
                RememberedActor {
                    pos: r.vec2()?,
                    vel: r.vec2()?,
                    faction: ambition_characters::actor::ActorFaction::decode(r)?,
                    hostile_to_self: r.bool()?,
                    last_seen: r.f32()?,
                    confidence: r.f32()?,
                },
            ));
        }
        Some(
            ambition_actors::features::ecs::perception::PerceptionMemory(
                WorldMemory::from_snapshot(rows),
            ),
        )
    }
}

snapshot_unit_enum!(ambition_characters::brain::boss_pattern::BossEncounterPhase {
    Dormant = 0,
    Intro = 1,
    Phase1 = 2,
    Transition = 3,
    Phase2 = 4,
    Stagger = 5,
    Enrage = 6,
    Death = 7,
});
snapshot_unit_enum!(ambition_characters::brain::boss_pattern::CyclePhase {
    Cooldown = 0,
    Windup = 1,
    Active = 2,
});
/// Not a unit enum — `Approach` and `Retreat` carry their own clocks, and a boss
/// that rewinds into `Retreat` must rewind to the same retreat POSITION. Explicit
/// discriminants for the same reason as `snapshot_unit_enum!`.
impl SnapshotState for ambition_characters::brain::boss_pattern::BossMacroState {
    fn encode(&self, out: &mut Vec<u8>) {
        use ambition_characters::brain::boss_pattern::BossMacroState as M;
        match self {
            M::Engage => put_u8(out, 0),
            M::Approach { remaining_s } => {
                put_u8(out, 1);
                put_f32(out, *remaining_s);
            }
            M::Retreat {
                remaining_s,
                retreat_pos,
            } => {
                put_u8(out, 2);
                put_f32(out, *remaining_s);
                put_vec2(out, *retreat_pos);
            }
        }
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use ambition_characters::brain::boss_pattern::BossMacroState as M;
        match r.u8()? {
            0 => Some(M::Engage),
            1 => Some(M::Approach {
                remaining_s: r.f32()?,
            }),
            2 => Some(M::Retreat {
                remaining_s: r.f32()?,
                retreat_pos: r.vec2()?,
            }),
            _ => None,
        }
    }
}

/// One beat of a **resolved** boss timeline.
///
/// `resolve_timeline` rolls every `Select` away before the first tick of the fight runs
/// — *"Select rolled away, Stance markers left in place as jumps"* — so a resolved
/// timeline holds only these four. A `Select` that survives into one is an invariant
/// violation, and this encodes it as a tag no decoder accepts: rejected, never silently
/// reinterpreted as a `Rest`.
///
/// The steps are *resolved instance state*, not authored content. The authored thing is
/// the `BossPattern`; the timeline is what one weighted roll made of it. Rewinding a
/// boss without rewinding the roll gives it a different fight.
impl SnapshotState for ambition_characters::brain::boss_pattern::BossPatternStep {
    fn encode(&self, out: &mut Vec<u8>) {
        use ambition_characters::brain::boss_pattern::BossPatternStep as S;
        match self {
            S::Telegraph {
                profile,
                duration,
                telegraph,
            } => {
                put_u8(out, 0);
                profile.encode(out);
                put_f32(out, *duration);
                match telegraph {
                    None => put_bool(out, false),
                    Some(spec) => {
                        put_bool(out, true);
                        put_opt_str(out, spec.pose.as_deref());
                        put_opt_str(out, spec.cue.as_deref());
                        put_opt_str(out, spec.vfx.as_deref());
                    }
                }
            }
            S::Strike { profile, duration } => {
                put_u8(out, 1);
                profile.encode(out);
                put_f32(out, *duration);
            }
            S::Rest { duration } => {
                put_u8(out, 2);
                put_f32(out, *duration);
            }
            S::Stance { id } => {
                put_u8(out, 3);
                put_str(out, id);
            }
            // Unreachable in a resolved timeline. Tag 4 decodes to `None`.
            S::Select { .. } => {
                debug_assert!(false, "a resolved timeline still holds a `Select`");
                put_u8(out, 4);
            }
        }
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use ambition_characters::brain::boss_pattern::{
            BossAttackProfile, BossPatternStep as S, TelegraphSpec,
        };
        match r.u8()? {
            0 => {
                let profile = BossAttackProfile::decode(r)?;
                let duration = r.f32()?;
                let telegraph = if r.bool()? {
                    Some(TelegraphSpec {
                        pose: r.opt_str()?.map(str::to_string),
                        cue: r.opt_str()?.map(str::to_string),
                        vfx: r.opt_str()?.map(str::to_string),
                    })
                } else {
                    None
                };
                Some(S::Telegraph {
                    profile,
                    duration,
                    telegraph,
                })
            }
            1 => Some(S::Strike {
                profile: BossAttackProfile::decode(r)?,
                duration: r.f32()?,
            }),
            2 => Some(S::Rest { duration: r.f32()? }),
            3 => Some(S::Stance {
                id: r.str()?.to_string(),
            }),
            _ => None,
        }
    }
}

fn put_timeline(
    out: &mut Vec<u8>,
    steps: &[ambition_characters::brain::boss_pattern::BossPatternStep],
) {
    put_u32(out, steps.len() as u32);
    for s in steps {
        s.encode(out);
    }
}

fn read_timeline(
    r: &mut Reader<'_>,
) -> Option<Vec<ambition_characters::brain::boss_pattern::BossPatternStep>> {
    use ambition_characters::brain::boss_pattern::BossPatternStep;
    let n = r.u32()?;
    (0..n).map(|_| BossPatternStep::decode(r)).collect()
}

/// **The boss's mind, rewound.**
///
/// A `SnapshotCursor`, because `Brain` is half authored and half state: the brain's
/// KIND and its tuning came from content and survive the patch, and only
/// `BossPatternState`'s clocks, cursors, and **`rng_seed`** ride the blob. A seeded
/// RNG that is not snapshot state is a determinism bug the canary would eventually
/// catch, and netcode.md's checklist names it.
///
/// ## The `timeline` is instance state, not authored content
///
/// I first left `timeline` and `stance_stack` un-rewound, and called the resulting
/// hazard a *constraint*: "a rollback window must not span a pattern re-resolve."
/// `mockingbird_arena` then replayed exactly for twenty ticks and broke on the
/// twenty-first, which is what a re-resolve inside the window looks like.
///
/// The framing was wrong. The AUTHORED thing is the `BossPattern`; the timeline is what
/// **one weighted roll** made of it — *"the roll happens at RESOLUTION, not at the
/// cursor, so a fight's timeline is a concrete list of beats before the first tick of
/// it runs."* That is instance state by any definition, and rewinding a boss without
/// rewinding the roll gives it a different fight. It is encoded, and so is the
/// `stance_stack`, whose entries carry timelines of their own.
///
/// A resolved timeline holds only `Telegraph` / `Strike` / `Rest` / `Stance`: the
/// `Select`s are rolled away at resolution. So the beats are small, and the blob is a
/// handful of tags and floats — not the pattern, not the arms, not the weights.
impl SnapshotCursor for ambition_characters::brain::Brain {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        let Some(s) = self.boss_pattern_state() else {
            // Not a boss brain: nothing mutable that a rollback needs. The tag keeps
            // "no state" distinguishable from a truncated blob.
            put_u8(out, 0);
            return;
        };
        put_u8(out, 1);
        match &s.last_phase {
            None => put_bool(out, false),
            Some(p) => {
                put_bool(out, true);
                p.encode(out);
            }
        }
        put_u32(out, s.step_index as u32);
        put_f32(out, s.step_elapsed);
        put_f32(out, s.movement_timer);
        put_f32(out, s.pattern_timer);
        s.cycle_phase.encode(out);
        put_f32(out, s.cycle_phase_remaining);
        s.macro_state.encode(out);
        put_f32(out, s.engage_timer);
        put_u64(out, s.rng_seed);
        s.attack_state.encode(out);
        put_timeline(out, &s.timeline);
        put_opt_str(out, s.stance.as_deref());
        put_u32(out, s.stance_stack.len() as u32);
        for ret in &s.stance_stack {
            put_timeline(out, &ret.timeline);
            put_opt_str(out, ret.stance.as_deref());
            put_u32(out, ret.step_index as u32);
            put_f32(out, ret.step_elapsed);
        }
        put_u32(out, s.interrupt_cooldowns.len() as u32);
        for v in &s.interrupt_cooldowns {
            put_f32(out, *v);
        }
        put_u32(out, s.interrupt_timers.len() as u32);
        for v in &s.interrupt_timers {
            put_f32(out, *v);
        }
        match s.last_hp {
            None => put_bool(out, false),
            Some(hp) => {
                put_bool(out, true);
                put_i32(out, hp);
            }
        }
    }

    fn apply_cursor(&mut self, r: &mut Reader<'_>) -> Option<()> {
        use ambition_characters::brain::boss_pattern::{
            BossAttackState, BossEncounterPhase, BossMacroState, CyclePhase,
        };
        if r.u8()? == 0 {
            return Some(());
        }
        let last_phase = if r.bool()? {
            Some(BossEncounterPhase::decode(r)?)
        } else {
            None
        };
        let step_index = r.u32()? as usize;
        let step_elapsed = r.f32()?;
        let movement_timer = r.f32()?;
        let pattern_timer = r.f32()?;
        let cycle_phase = CyclePhase::decode(r)?;
        let cycle_phase_remaining = r.f32()?;
        let macro_state = BossMacroState::decode(r)?;
        let engage_timer = r.f32()?;
        let rng_seed = r.u64()?;
        let attack_state = BossAttackState::decode(r)?;
        let timeline = read_timeline(r)?;
        let stance = r.opt_str()?.map(str::to_string);
        let stance_stack = {
            use ambition_characters::brain::boss_pattern::StanceReturn;
            let n = r.u32()?;
            (0..n)
                .map(|_| {
                    Some(StanceReturn {
                        timeline: read_timeline(r)?,
                        stance: r.opt_str()?.map(str::to_string),
                        step_index: r.u32()? as usize,
                        step_elapsed: r.f32()?,
                    })
                })
                .collect::<Option<Vec<_>>>()?
        };
        fn read_f32s(r: &mut Reader<'_>) -> Option<Vec<f32>> {
            let n = r.u32()?;
            (0..n).map(|_| r.f32()).collect()
        }
        let interrupt_cooldowns = read_f32s(r)?;
        let interrupt_timers = read_f32s(r)?;
        let last_hp = if r.bool()? { Some(r.i32()?) } else { None };

        // A blob written by a boss brain, applied to one that is no longer a boss
        // brain, would be a content change across a rollback. Leave it alone.
        let Some(s) = self.boss_pattern_state_mut() else {
            return Some(());
        };
        s.last_phase = last_phase;
        s.step_index = step_index;
        s.step_elapsed = step_elapsed;
        s.movement_timer = movement_timer;
        s.pattern_timer = pattern_timer;
        s.cycle_phase = cycle_phase;
        s.cycle_phase_remaining = cycle_phase_remaining;
        s.macro_state = macro_state;
        s.engage_timer = engage_timer;
        s.rng_seed = rng_seed;
        s.attack_state = attack_state;
        s.timeline = timeline;
        s.stance = stance;
        s.stance_stack = stance_stack;
        s.interrupt_cooldowns = interrupt_cooldowns;
        s.interrupt_timers = interrupt_timers;
        s.last_hp = last_hp;
        Some(())
    }
}

snapshot_unit_enum!(ambition_engine_core::reference_frame::GameplayFramePolicy {
    ControlledBodyLocal = 0,
    AccelerationFrame = 1,
    WorldSpace = 2,
    ScreenSpace = 3,
});

/// **The brain's last-tick intent**, which the sim reads on the NEXT tick — the
/// `brain/README.md` calls it exactly that. So it is state, not a per-frame scratchpad,
/// and a rewind that leaves it stale hands the body an input it never chose.
///
/// Every field, in declaration order. There is no clever half of this component.
impl SnapshotState for ambition_characters::brain::ActorControl {
    fn encode(&self, out: &mut Vec<u8>) {
        let f = &self.0;
        put_vec2(out, f.locomotion);
        put_vec2(out, f.velocity_target);
        put_bool(out, f.drop_through);
        put_f32(out, f.facing);
        put_bool(out, f.melee_pressed);
        match &f.fire {
            None => put_bool(out, false),
            Some(fire) => {
                put_bool(out, true);
                put_vec2(out, fire.dir);
                fire.dir_policy.encode(out);
                put_f32(out, fire.speed);
            }
        }
        put_vec2(out, f.attack_axis);
        for b in [
            f.jump_pressed,
            f.jump_held,
            f.jump_released,
            f.dash_pressed,
            f.interact_pressed,
            f.body_contact_damage_enabled,
            f.shield_held,
            f.special_pressed,
            f.pogo_pressed,
            f.fast_fall_pressed,
            f.fly_toggle_pressed,
            f.projectile_pressed,
            f.projectile_held,
            f.projectile_released,
            f.blink_pressed,
            f.blink_held,
            f.blink_released,
        ] {
            put_bool(out, b);
        }
        put_vec2(out, f.blink_quick_dir);
        put_vec2(out, f.blink_aim_step);
        put_vec2(out, f.aim);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use ambition_characters::actor::control::{ActorControlFrame, ActorFireRequest};
        use ambition_engine_core::reference_frame::GameplayFramePolicy;
        let locomotion = r.vec2()?;
        let velocity_target = r.vec2()?;
        let drop_through = r.bool()?;
        let facing = r.f32()?;
        let melee_pressed = r.bool()?;
        let fire = if r.bool()? {
            Some(ActorFireRequest {
                dir: r.vec2()?,
                dir_policy: GameplayFramePolicy::decode(r)?,
                speed: r.f32()?,
            })
        } else {
            None
        };
        let attack_axis = r.vec2()?;
        let mut flags = [false; 17];
        for f in flags.iter_mut() {
            *f = r.bool()?;
        }
        Some(ambition_characters::brain::ActorControl(
            ActorControlFrame {
                locomotion,
                velocity_target,
                drop_through,
                facing,
                melee_pressed,
                fire,
                attack_axis,
                jump_pressed: flags[0],
                jump_held: flags[1],
                jump_released: flags[2],
                dash_pressed: flags[3],
                interact_pressed: flags[4],
                body_contact_damage_enabled: flags[5],
                shield_held: flags[6],
                special_pressed: flags[7],
                pogo_pressed: flags[8],
                fast_fall_pressed: flags[9],
                fly_toggle_pressed: flags[10],
                projectile_pressed: flags[11],
                projectile_held: flags[12],
                projectile_released: flags[13],
                blink_pressed: flags[14],
                blink_held: flags[15],
                blink_released: flags[16],
                blink_quick_dir: r.vec2()?,
                blink_aim_step: r.vec2()?,
                aim: r.vec2()?,
            },
        ))
    }
}

snapshot_unit_enum!(ambition_combat::components::BossPhase {
    Active = 0,
    Defeated = 1,
});

impl SnapshotState for ambition_combat::components::BossPatternTimer {
    fn encode(&self, out: &mut Vec<u8>) {
        put_f32(out, self.0);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(ambition_combat::components::BossPatternTimer(r.f32()?))
    }
}

/// **An accumulating sim clock**, and netcode.md's N3.1 checklist names it: *"`WorldTime`
/// + every sim clock"*. A brain stamps `RememberedActor.last_seen` with it, so a rewind
/// that leaves it running makes every memory look older than it is — which is exactly
/// how `gnu_ton_arena` diverged on `perception_memory` and nothing else.
impl SnapshotState for ambition_actors::features::GameplayElapsed {
    fn encode(&self, out: &mut Vec<u8>) {
        put_f32(out, self.0);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(ambition_actors::features::GameplayElapsed(r.f32()?))
    }
}

/// **The combat slot board**: which attacker holds which approach slot around the
/// target. The slot GEOMETRY is authored (`kind`, `offset`, `holding_offset`); the
/// `assigned_to: Option<String>` is live, and it is a stable id rather than an `Entity`,
/// so it rewinds cleanly. A boss holding a slot it never claimed attacks on a tick it
/// never earned.
impl SnapshotCursor for ambition_combat::slots::CombatSlotsRes {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        put_u32(out, self.0.slots.len() as u32);
        for slot in &self.0.slots {
            put_opt_str(out, slot.assigned_to.as_deref());
        }
    }
    fn apply_cursor(&mut self, r: &mut Reader<'_>) -> Option<()> {
        let n = r.u32()? as usize;
        // A board of a different SHAPE cannot be faithfully rewound by a cursor: the snapshot's
        // assignments would not line up with the live authored slots, and silently zipping the
        // shorter length leaves live slots untouched or drops snapshot assignments while
        // reporting success (re-audit finding 4). Within a supported window the shape is stable
        // — content does not change, and a cross-room rollback is already refused — so this
        // never fires there; if it ever did, refusing loudly (`None` → `DecodeFailed`) beats a
        // silent partial restore.
        if n != self.0.slots.len() {
            return None;
        }
        for slot in self.0.slots.iter_mut() {
            slot.assigned_to = r.opt_str()?.map(str::to_string);
        }
        Some(())
    }
}

/// **A body's proper-time dilation** (ADR 0011): hitstop, bullet-time, a boss's slow.
/// Every move clock and every brain timer advances on `world_time.entity_dt(scale)`, so
/// a stale scale makes a rewound body live in a differently-paced universe.
impl SnapshotState for ambition_time::ProperTimeScale {
    fn encode(&self, out: &mut Vec<u8>) {
        put_f32(out, self.0);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(ambition_time::ProperTimeScale(r.f32()?))
    }
}

impl SnapshotState for SimIdCounter {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u64(out, self.0);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(SimIdCounter(r.u64()?))
    }
}

/// Give every body the sim can identify a [`SimId`], once.
///
/// Two facts exist today, and this system reads exactly those two: an authored
/// placement's `FeatureId` (the LDtk iid a save file already keys on) and the
/// primary player's slot. **Dynamically-spawned entities are NOT covered** —
/// N3.1's pin says they get `(spawner SimId, per-spawner counter)`, which the
/// spawn sites must mint at spawn (they know their spawner; this system does not).
/// `unidentified_bodies` counts what is left, so the migration has a number.
///
/// Runs at the head of the sim, before anything reads identity.
pub fn ensure_sim_id(
    mut commands: bevy::ecs::system::Commands,
    unidentified: bevy::ecs::system::Query<
        (
            bevy::ecs::entity::Entity,
            Option<&ambition_combat::components::FeatureId>,
            Option<&ambition_platformer_primitives::markers::PrimaryPlayer>,
        ),
        (
            bevy::ecs::query::With<ambition_platformer_primitives::body::BodyKinematics>,
            bevy::ecs::query::Without<ambition_platformer_primitives::sim_id::SimId>,
        ),
    >,
) {
    use ambition_platformer_primitives::sim_id::{SimId, SimIdCounter};
    for (entity, feature_id, primary) in &unidentified {
        let id = match (feature_id, primary) {
            (Some(id), _) => SimId::placement(&id.0),
            (None, Some(_)) => SimId::player_slot(0),
            // Not identifiable from an authored fact. Its spawn site must mint it.
            (None, None) => continue,
        };
        // Every identified body is a potential spawner (a boss summons, a player
        // fires), and its counter is snapshot state.
        commands
            .entity(entity)
            .insert((id, SimIdCounter::default()));
    }
}

/// Mint `SimId::spawned(spawner, counter.next())` for every in-flight projectile
/// that has none — N3.1's rule for dynamically-spawned sim entities.
///
/// ## Why this is one system rather than an edit at every spawn site
///
/// A projectile already carries the fact this needs: `ProjectileOwner`. Threading
/// a `SimIdCounter` through a dozen fire paths would put the same lookup in a
/// dozen places and leave the thirteenth out.
///
/// ## Why the order is deterministic
///
/// A `Query` walks archetypes, not spawn order, so two sims could mint a pair of
/// same-tick projectiles' ids in opposite order. Sorting by
/// `(owner SimId, ProjectileSeq)` fixes that: `ProjectileSeq` is the existing
/// monotonic spawn-sequence the step system already sorts by to keep iteration
/// deterministic. Its counter is global — which N3.1 forbids for *identity*,
/// because it couples unrelated spawners — but a global counter is a perfectly
/// good *total order*, which is all this uses it for. The identity itself comes
/// from the owner's own `SimIdCounter`, one stream per spawner.
pub fn mint_spawned_sim_ids(
    mut commands: bevy::ecs::system::Commands,
    newborns: bevy::ecs::system::Query<
        (
            bevy::ecs::entity::Entity,
            &ambition_projectiles::ProjectileOwner,
            &ambition_projectiles::ProjectileSeq,
        ),
        (
            bevy::ecs::query::With<ambition_projectiles::LiveProjectile>,
            bevy::ecs::query::Without<ambition_platformer_primitives::sim_id::SimId>,
        ),
    >,
    mut owners: bevy::ecs::system::Query<(
        &ambition_platformer_primitives::sim_id::SimId,
        &mut ambition_platformer_primitives::sim_id::SimIdCounter,
    )>,
) {
    use ambition_platformer_primitives::sim_id::SimId;

    let mut rows: Vec<(
        String,
        u64,
        bevy::ecs::entity::Entity,
        bevy::ecs::entity::Entity,
    )> = Vec::new();
    for (entity, owner, seq) in &newborns {
        // An owner with no identity cannot lend one. Its own migration comes first.
        let Ok((owner_id, _)) = owners.get(owner.0) else {
            continue;
        };
        rows.push((owner_id.as_str().to_string(), seq.0, entity, owner.0));
    }
    rows.sort();

    for (_, _, entity, owner_entity) in rows {
        let Ok((owner_id, mut counter)) = owners.get_mut(owner_entity) else {
            continue;
        };
        let id = SimId::spawned(owner_id, counter.next());
        // A projectile can itself spawn (a splitting shot), so it gets a counter.
        commands.entity(entity).insert((
            id,
            ambition_platformer_primitives::sim_id::SimIdCounter::default(),
        ));
    }
}
