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

impl SnapshotState for ambition_platformer_primitives::lifecycle::RoomScopedEntity {
    fn encode(&self, _out: &mut Vec<u8>) {}

    fn decode(_r: &mut Reader<'_>) -> Option<Self> {
        Some(Self)
    }
}

impl SnapshotState for ambition_platformer_primitives::lifecycle::SessionScopedEntity {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u64(out, self.0 .0);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self(
            ambition_platformer_primitives::lifecycle::SessionScopeId(r.u64()?),
        ))
    }
}

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

snapshot_pod!(bc::BodyGroundState { on_ground: bool });
snapshot_pod!(bc::BodyWallState {
    on_wall: bool,
    wall_normal_x: f32,
});
snapshot_pod!(bc::BodyJumpState {
    air_jumps_available: u8,
    ladder_jump_boost: f32,
    ladder_drop_through_timer: f32,
    ladder_drop_through_hold_lock: bool,
});
snapshot_pod!(bc::BodyDashState {
    charges_available: u8,
    cooldown: f32,
});
snapshot_pod!(bc::BodyFlightState {
    fly_enabled: bool,
    carried_run: f32,
});
snapshot_pod!(bc::BodyBlinkState { cooldown: f32 });
snapshot_pod!(bc::BodyDodgeState { cooldown: f32 });
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
    attack: f32,
    pogo: f32,
    projectile: f32,
});
snapshot_pod!(bc::BodyBaseSize { base_size: vec2 });
snapshot_pod!(ambition_characters::actor::body::BodyCombat {
    hit_flash: f32,
    hitstop_timer: f32,
    damage_invuln_timer: f32,
    hitstun_timer: f32,
    recoil_lock_timer: f32,
    attacking: bool,
    alive: bool,
    strike_count: i32,
    attack_windup_timer: f32,
    attack_timer: f32,
    training_dummy: bool,
});
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

// The hang state machine (`Option<LedgeGrabState>`) is axis-policy maneuver
// state and rides in the `MotionModel` codec (motion_codec.rs); the shared
// cluster keeps only the re-grab lockout.
snapshot_pod!(bc::BodyLedgeState {
    release_cooldown: f32,
});

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

fn put_attack_intent(out: &mut Vec<u8>, intent: ambition_combat::AttackIntent) {
    use ambition_combat::AttackIntent;
    put_u8(
        out,
        match intent {
            AttackIntent::Neutral => 0,
            AttackIntent::Forward => 1,
            AttackIntent::Back => 2,
            AttackIntent::Up => 3,
            AttackIntent::Down => 4,
            AttackIntent::DashForward => 5,
            AttackIntent::AirForward => 6,
            AttackIntent::AirBack => 7,
            AttackIntent::AirUp => 8,
            AttackIntent::AirDown => 9,
            AttackIntent::WallOut => 10,
        },
    );
}

fn read_attack_intent(r: &mut Reader<'_>) -> Option<ambition_combat::AttackIntent> {
    use ambition_combat::AttackIntent;
    match r.u8()? {
        0 => Some(AttackIntent::Neutral),
        1 => Some(AttackIntent::Forward),
        2 => Some(AttackIntent::Back),
        3 => Some(AttackIntent::Up),
        4 => Some(AttackIntent::Down),
        5 => Some(AttackIntent::DashForward),
        6 => Some(AttackIntent::AirForward),
        7 => Some(AttackIntent::AirBack),
        8 => Some(AttackIntent::AirUp),
        9 => Some(AttackIntent::AirDown),
        10 => Some(AttackIntent::WallOut),
        _ => None,
    }
}

fn put_damage_kind(out: &mut Vec<u8>, kind: ambition_combat::DamageKind) {
    use ambition_combat::DamageKind;
    put_u8(
        out,
        match kind {
            DamageKind::Slash => 0,
            DamageKind::Pogo => 1,
            DamageKind::Contact => 2,
            DamageKind::Hazard => 3,
            DamageKind::Projectile => 4,
            DamageKind::Environmental => 5,
            DamageKind::Custom => 6,
        },
    );
}

fn read_damage_kind(r: &mut Reader<'_>) -> Option<ambition_combat::DamageKind> {
    use ambition_combat::DamageKind;
    match r.u8()? {
        0 => Some(DamageKind::Slash),
        1 => Some(DamageKind::Pogo),
        2 => Some(DamageKind::Contact),
        3 => Some(DamageKind::Hazard),
        4 => Some(DamageKind::Projectile),
        5 => Some(DamageKind::Environmental),
        6 => Some(DamageKind::Custom),
        _ => None,
    }
}

fn put_attack_spec(out: &mut Vec<u8>, spec: ambition_combat::AttackSpec) {
    put_attack_intent(out, spec.intent);
    put_f32(out, spec.startup_seconds);
    put_f32(out, spec.active_seconds);
    put_f32(out, spec.recovery_seconds);
    put_vec2(out, spec.hitbox_offset);
    put_vec2(out, spec.hitbox_half_size);
    put_vec2(out, spec.self_impulse);
    put_vec2(out, spec.knockback);
    put_damage_kind(out, spec.damage_kind);
    put_bool(out, spec.can_pogo);
    match spec.damage_override {
        Some(value) => {
            put_bool(out, true);
            put_i32(out, value);
        }
        None => put_bool(out, false),
    }
}

fn read_attack_spec(r: &mut Reader<'_>) -> Option<ambition_combat::AttackSpec> {
    Some(ambition_combat::AttackSpec {
        intent: read_attack_intent(r)?,
        startup_seconds: r.f32()?,
        active_seconds: r.f32()?,
        recovery_seconds: r.f32()?,
        hitbox_offset: r.vec2()?,
        hitbox_half_size: r.vec2()?,
        self_impulse: r.vec2()?,
        knockback: r.vec2()?,
        damage_kind: read_damage_kind(r)?,
        can_pogo: r.bool()?,
        damage_override: if r.bool()? { Some(r.i32()?) } else { None },
    })
}

impl SnapshotState for ambition_combat::components::BodyMelee {
    fn encode(&self, out: &mut Vec<u8>) {
        match &self.swing {
            Some(swing) => {
                put_bool(out, true);
                put_attack_spec(out, swing.spec);
                put_f32(out, swing.elapsed);
                put_u32(out, swing.hit_targets.len() as u32);
                for target in &swing.hit_targets {
                    put_str(out, target);
                }
                put_bool(out, swing.active_started);
                put_bool(out, swing.pogo_applied);
            }
            None => put_bool(out, false),
        }
        put_f32(out, self.cooldown);
        put_f32(out, self.ranged_cooldown);
        put_vec2(out, self.pending_axis);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        let swing = if r.bool()? {
            let spec = read_attack_spec(r)?;
            let elapsed = r.f32()?;
            let hit_count = r.u32()?;
            let hit_targets = (0..hit_count)
                .map(|_| Some(r.str()?.to_string()))
                .collect::<Option<Vec<_>>>()?;
            Some(ambition_combat::components::MeleeSwing {
                spec,
                elapsed,
                hit_targets,
                active_started: r.bool()?,
                pogo_applied: r.bool()?,
            })
        } else {
            None
        };
        Some(Self {
            swing,
            cooldown: r.f32()?,
            ranged_cooldown: r.f32()?,
            pending_axis: r.vec2()?,
        })
    }
}

snapshot_unit_enum!(ambition_combat::components::ActorDisposition {
    Peaceful = 0,
    Hostile = 1,
});

/// Mutable aggression policy and provocation count. The `target` and `grudge`
/// fields are entity-handle caches/relationships: target selection republishes
/// `target`, while content-staged batch reconstruction restores authored grudges.
/// Encoding allocator-local `Entity` values would violate the stable-id contract.
impl SnapshotCursor for ambition_combat::components::ActorAggression {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        use ambition_combat::components::AggressionMode;
        match self.mode {
            AggressionMode::Passive => put_u8(out, 0),
            AggressionMode::RetaliatesWhenHit { strike_threshold } => {
                put_u8(out, 1);
                put_u8(out, strike_threshold);
            }
            AggressionMode::Hostile => put_u8(out, 2),
        }
        put_i32(out, self.strikes);
    }

    fn apply_cursor(&mut self, r: &mut Reader<'_>) -> Option<()> {
        use ambition_combat::components::AggressionMode;
        let mode = match r.u8()? {
            0 => AggressionMode::Passive,
            1 => AggressionMode::RetaliatesWhenHit {
                strike_threshold: r.u8()?,
            },
            2 => AggressionMode::Hostile,
            _ => return None,
        };
        let strikes = r.i32()?;
        self.mode = mode;
        self.strikes = strikes;
        // `target` is a derived per-tick cache. `grudge` remains the stable
        // authored/relationship value installed by room/content staging.
        self.target = None;
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
        put_opt_profile(out, &self.active_profile);
        put_f32(out, self.active_remaining);
        put_f32(out, self.active_elapsed);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use ambition_characters::brain::boss_pattern::BossAttackState;
        let telegraph_profile = read_opt_profile(r)?;
        let telegraph_remaining = r.f32()?;
        let telegraph_elapsed = r.f32()?;
        Some(BossAttackState {
            telegraph_profile,
            telegraph_remaining,
            telegraph_elapsed,
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

fn put_smash_mode(out: &mut Vec<u8>, mode: ambition_characters::brain::smash::BroadMode) {
    use ambition_characters::brain::smash::BroadMode;
    put_u8(
        out,
        match mode {
            BroadMode::Idle => 0,
            BroadMode::Approach => 1,
            BroadMode::Retreat => 2,
            BroadMode::Engage => 3,
            BroadMode::Reposition => 4,
            BroadMode::Recover => 5,
        },
    );
}

fn read_smash_mode(r: &mut Reader<'_>) -> Option<ambition_characters::brain::smash::BroadMode> {
    use ambition_characters::brain::smash::BroadMode;
    match r.u8()? {
        0 => Some(BroadMode::Idle),
        1 => Some(BroadMode::Approach),
        2 => Some(BroadMode::Retreat),
        3 => Some(BroadMode::Engage),
        4 => Some(BroadMode::Reposition),
        5 => Some(BroadMode::Recover),
        _ => None,
    }
}

fn put_smash_state(out: &mut Vec<u8>, state: &ambition_characters::brain::smash::SmashState) {
    put_smash_mode(out, state.mode);
    put_f32(out, state.mode_dwell_s);
    put_u64(out, state.rng_seed);
    put_f32(out, state.dash_cooldown_remaining);
    let (samples, write, count) = state.obs_history.snapshot_parts();
    for (time, pos) in samples {
        put_f32(out, *time);
        put_vec2(out, *pos);
    }
    put_u32(out, write as u32);
    put_u32(out, count as u32);
    put_f32(out, state.spacing_phase);
    put_f32(out, state.neutral_jump_cooldown);
    put_f32(out, state.blink_cooldown);
    put_f32(out, state.foray_timer);
    put_f32(out, state.shield_hold_timer);
    put_f32(out, state.neutral_reset_timer);
    put_bool(out, state.was_attacking);
    put_f32(out, state.regroup_timer);
    put_f32(out, state.last_health_fraction);
    put_f32(out, state.damage_accum);
    put_f32(out, state.time_since_offense);
}

fn read_smash_state(r: &mut Reader<'_>) -> Option<ambition_characters::brain::smash::SmashState> {
    use ambition_characters::brain::smash::{SmashState, OBS_HISTORY_LEN};

    let mode = read_smash_mode(r)?;
    let mode_dwell_s = r.f32()?;
    let rng_seed = r.u64()?;
    let dash_cooldown_remaining = r.f32()?;
    let mut samples = [(0.0, ambition_engine_core::Vec2::ZERO); OBS_HISTORY_LEN];
    for sample in &mut samples {
        *sample = (r.f32()?, r.vec2()?);
    }
    let history_write = r.u32()? as usize;
    let history_count = r.u32()? as usize;
    let spacing_phase = r.f32()?;
    let neutral_jump_cooldown = r.f32()?;
    let blink_cooldown = r.f32()?;
    let foray_timer = r.f32()?;
    let shield_hold_timer = r.f32()?;
    let neutral_reset_timer = r.f32()?;
    let was_attacking = r.bool()?;
    let regroup_timer = r.f32()?;
    let last_health_fraction = r.f32()?;
    let damage_accum = r.f32()?;
    let time_since_offense = r.f32()?;

    let mut state = SmashState {
        mode,
        mode_dwell_s,
        rng_seed,
        dash_cooldown_remaining,
        spacing_phase,
        neutral_jump_cooldown,
        blink_cooldown,
        foray_timer,
        shield_hold_timer,
        neutral_reset_timer,
        was_attacking,
        regroup_timer,
        last_health_fraction,
        damage_accum,
        time_since_offense,
        ..SmashState::default()
    };
    state
        .obs_history
        .restore_snapshot_parts(samples, history_write, history_count)?;
    Some(state)
}

/// Rewind the mutable cursor of state-machine brains while leaving authored tuning in place.
/// Boss-pattern and Smash brains both carry replay-significant internal clocks/history.
impl SnapshotCursor for ambition_characters::brain::Brain {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        use ambition_characters::brain::{Brain, StateMachineCfg};
        match self {
            Brain::StateMachine(StateMachineCfg::BossPattern { state, .. }) => {
                put_u8(out, 1);
                match &state.last_phase {
                    None => put_bool(out, false),
                    Some(phase) => {
                        put_bool(out, true);
                        phase.encode(out);
                    }
                }
                put_u32(out, state.step_index as u32);
                put_f32(out, state.step_elapsed);
                put_f32(out, state.movement_timer);
                put_f32(out, state.pattern_timer);
                put_f32(out, state.cycle_rest_remaining);
                state.macro_state.encode(out);
                put_f32(out, state.engage_timer);
                put_u64(out, state.rng_seed);
                put_timeline(out, &state.timeline);
                put_opt_str(out, state.stance.as_deref());
                put_u32(out, state.stance_stack.len() as u32);
                for ret in &state.stance_stack {
                    put_timeline(out, &ret.timeline);
                    put_opt_str(out, ret.stance.as_deref());
                    put_u32(out, ret.step_index as u32);
                    put_f32(out, ret.step_elapsed);
                }
                put_u32(out, state.interrupt_cooldowns.len() as u32);
                for value in &state.interrupt_cooldowns {
                    put_f32(out, *value);
                }
                put_u32(out, state.interrupt_timers.len() as u32);
                for value in &state.interrupt_timers {
                    put_f32(out, *value);
                }
                match state.last_hp {
                    None => put_bool(out, false),
                    Some(hp) => {
                        put_bool(out, true);
                        put_i32(out, hp);
                    }
                }
            }
            Brain::StateMachine(StateMachineCfg::Smash { state, .. }) => {
                put_u8(out, 2);
                put_smash_state(out, state);
            }
            _ => put_u8(out, 0),
        }
    }

    fn apply_cursor(&mut self, r: &mut Reader<'_>) -> Option<()> {
        use ambition_characters::brain::{Brain, StateMachineCfg};
        match r.u8()? {
            0 => Some(()),
            1 => {
                use ambition_characters::brain::boss_pattern::{
                    BossEncounterPhase, BossMacroState, StanceReturn,
                };
                let last_phase = if r.bool()? {
                    Some(BossEncounterPhase::decode(r)?)
                } else {
                    None
                };
                let step_index = r.u32()? as usize;
                let step_elapsed = r.f32()?;
                let movement_timer = r.f32()?;
                let pattern_timer = r.f32()?;
                let cycle_rest_remaining = r.f32()?;
                let macro_state = BossMacroState::decode(r)?;
                let engage_timer = r.f32()?;
                let rng_seed = r.u64()?;
                let timeline = read_timeline(r)?;
                let stance = r.opt_str()?.map(str::to_string);
                let stance_stack = {
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

                let Brain::StateMachine(StateMachineCfg::BossPattern { state, .. }) = self else {
                    return Some(());
                };
                state.last_phase = last_phase;
                state.step_index = step_index;
                state.step_elapsed = step_elapsed;
                state.movement_timer = movement_timer;
                state.pattern_timer = pattern_timer;
                state.cycle_rest_remaining = cycle_rest_remaining;
                state.macro_state = macro_state;
                state.engage_timer = engage_timer;
                state.rng_seed = rng_seed;
                state.attack_intent = Default::default();
                state.timeline = timeline;
                state.stance = stance;
                state.stance_stack = stance_stack;
                state.interrupt_cooldowns = interrupt_cooldowns;
                state.interrupt_timers = interrupt_timers;
                state.last_hp = last_hp;
                Some(())
            }
            2 => {
                let restored = read_smash_state(r)?;
                if let Brain::StateMachine(StateMachineCfg::Smash { state, .. }) = self {
                    *state = restored;
                }
                Some(())
            }
            _ => None,
        }
    }
}

/// **The explicit brain SELECTION** for a catalog-backed NPC: its character
/// default preset plus whether it is on the default or an override. Self-contained
/// (preset-id strings only — no `Entity`, no runtime brain), so it is a plain
/// `register_component` and restores its own presence.
///
/// This is the authoritative snapshot state for "which brain is selected". The
/// live [`Brain`](ambition_characters::brain::Brain) cursor is a no-op for the
/// peaceful/patrol NPC brains, so after a rewind PAST a runtime brain switch the
/// live brain kind could disagree with the restored selection —
/// [`reconcile_brain_bindings`] rebuilds the brain from this binding to make them
/// agree before the next re-simulated tick.
impl SnapshotState for ambition_characters::actor::character_catalog::BrainBinding {
    fn encode(&self, out: &mut Vec<u8>) {
        use ambition_characters::actor::character_catalog::AutonomousBrainSource;
        put_str(out, self.default_preset.as_str());
        match &self.source {
            AutonomousBrainSource::CatalogDefault => put_u8(out, 0),
            AutonomousBrainSource::CatalogPreset(preset) => {
                put_u8(out, 1);
                put_str(out, preset.as_str());
            }
            // Provoked: the live brain is a roster archetype, not a catalog
            // preset. The stable archetype id is all a rebuild needs — reconcile
            // reruns the roster construction from it (never a catalog default).
            AutonomousBrainSource::Provoked { archetype } => {
                put_u8(out, 2);
                put_str(out, archetype.as_str());
            }
        }
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use ambition_characters::actor::character_catalog::{
            AutonomousBrainSource, BrainBinding, BrainPresetId, HostileArchetypeId,
        };
        let default_preset = BrainPresetId::new(r.str()?.to_string());
        let source = match r.u8()? {
            0 => AutonomousBrainSource::CatalogDefault,
            1 => AutonomousBrainSource::CatalogPreset(BrainPresetId::new(r.str()?.to_string())),
            2 => AutonomousBrainSource::Provoked {
                archetype: HostileArchetypeId::new(r.str()?.to_string()),
            },
            _ => return None,
        };
        Some(BrainBinding {
            default_preset,
            source,
        })
    }
}

/// The authored brain-build context (spawn anchor + patrol radius) a catalog NPC
/// rebuilds its default/override brain from. A self-contained POD component, so a
/// plain `register_component`. Snapshot-safe so a restored `RestoreDefault` /
/// [`reconcile_brain_bindings`] recenters a patrol brain on its AUTHORED home, not
/// wherever the actor wandered before the rewind.
impl SnapshotState for ambition_characters::actor::character_catalog::AuthoredBrainContext {
    fn encode(&self, out: &mut Vec<u8>) {
        put_f32(out, self.spawn_anchor_x);
        match self.patrol_radius {
            Some(r) => {
                put_bool(out, true);
                put_f32(out, r);
            }
            None => put_bool(out, false),
        }
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(
            ambition_characters::actor::character_catalog::AuthoredBrainContext {
                spawn_anchor_x: r.f32()?,
                patrol_radius: if r.bool()? { Some(r.f32()?) } else { None },
            },
        )
    }
}

/// **Temporary-control state**: whether an autonomous body is masked by a player
/// possession or a mount, by STABLE `SimId`. Registered so a rewind restores the
/// control MODE across time (not just avoids clobbering a live one): the `Brain`
/// cursor is a no-op for `Brain::Player`, and possession/mount relationships were
/// re-derived from live components, so without this a rollback across a
/// possess/release boundary left the body in the wrong mode. Reconciliation
/// rebuilds the live control (`Brain::Player` / `Mounted`) and its relationships
/// from the restored id.
impl SnapshotState for ambition_actors::features::TemporaryControl {
    fn encode(&self, out: &mut Vec<u8>) {
        use ambition_actors::features::TemporaryControl as T;
        match self {
            T::Autonomous => put_u8(out, 0),
            T::Player { controller } => {
                put_u8(out, 1);
                put_str(out, controller.as_str());
            }
            T::Mounted { mount } => {
                put_u8(out, 2);
                put_str(out, mount.as_str());
            }
        }
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use ambition_actors::features::TemporaryControl as T;
        use ambition_platformer_primitives::sim_id::SimId;
        Some(match r.u8()? {
            0 => T::Autonomous,
            1 => T::Player {
                controller: SimId::from_snapshot(r.str()?.to_string()),
            },
            2 => T::Mounted {
                mount: SimId::from_snapshot(r.str()?.to_string()),
            },
            _ => return None,
        })
    }
}

/// Post-restore reconcile: rebuild an AUTONOMOUS catalog-backed NPC's live `Brain`
/// from its restored [`BrainBinding`] **only when its authored configuration
/// diverged** — i.e. a rewind crossed a runtime brain switch, so the live brain no
/// longer matches the restored selection.
///
/// The `Brain` cursor is a no-op for the peaceful/patrol NPC brains (their kind was
/// authored-immutable before runtime switching existed), so it cannot restore a
/// switched kind. Left unreconciled, the next re-simulated tick would drive the
/// wrong brain — a desync.
///
/// Correctness details:
/// - **Configuration equality, not the label.** We compare via
///   [`Brain::same_authored_configuration`], not `label()`: two presets in the same
///   family (`wanderer_slow` / `wanderer_fast`) share a label but differ here, so a
///   rewind across such a switch is caught. Same config → leave the live brain
///   untouched, preserving the state the `Brain` cursor already restored (this is
///   also the RESTORE ORDER guarantee: the cursor runs first, and reconcile only
///   overwrites when the preset genuinely differs — in which case the cursor state
///   was for the wrong brain anyway).
/// - **Authored home.** A rebuild uses the actor's restored [`AuthoredBrainContext`]
///   (its spawn anchor + patrol radius), not its current pose, so a restored patrol
///   brain recenters where it was authored.
/// - **Temporary control is untouchable.** A body under player possession
///   (`Brain::Player`) or mount control (`Mounted`) is skipped — its live brain is
///   control, not its autonomous selection; reconciling would clobber it.
/// - **Externally-owned brains are left to their authority.** A binding whose
///   selection is `External` (provoke/challenge installed a non-catalog hostile
///   brain) has no `active_preset()` — reconcile skips it, so the disposition/provoke
///   authority owns that brain across the rewind, never the catalog default.
///
/// Skips gracefully when the world has no `CharacterCatalog` (headless fixtures).
pub fn reconcile_brain_bindings(world: &mut bevy::ecs::world::World) {
    use ambition_characters::actor::character_catalog::{
        AuthoredBrainContext, BrainBinding, BrainBuildContext,
    };
    use ambition_characters::actor::ActorPose;
    use ambition_characters::brain::Brain;

    struct Job {
        entity: bevy::ecs::entity::Entity,
        preset: String,
        ctx: BrainBuildContext,
        live: Brain,
    }

    // 1. Collect each AUTONOMOUS catalog-backed NPC's active preset, authored build
    //    context, and a clone of its live brain (an immutable pass). Player /
    //    mounted / external actors are filtered out here (see the doc note).
    //    `query` (not `try_query`) so the optional `AuthoredBrainContext` / `Mounted`
    //    component types are initialized even in a world that never spawned one — a
    //    `try_query` returns `None` there and would silently skip reconciliation.
    let jobs: Vec<Job> = {
        let mut q = world.query::<(
            bevy::ecs::entity::Entity,
            &BrainBinding,
            Option<&AuthoredBrainContext>,
            &ActorPose,
            &Brain,
            bevy::ecs::query::Has<ambition_actors::features::Mounted>,
        )>();
        q.iter(world)
            .filter_map(|(entity, binding, authored, pose, brain, mounted)| {
                if brain.is_player() || mounted {
                    return None;
                }
                // `None` => External => an authority other than the catalog owns it.
                let preset = binding.active_preset()?;
                let ctx = authored
                    .map(AuthoredBrainContext::build_context)
                    .unwrap_or_else(|| BrainBuildContext::at(pose.origin().x));
                Some(Job {
                    entity,
                    preset: preset.0.clone(),
                    ctx,
                    live: brain.clone(),
                })
            })
            .collect()
    };
    if jobs.is_empty() {
        return;
    }

    // 2. Rebuild only where the live brain's authored configuration differs from
    //    the brain the restored selection resolves to, via the same catalog seam as
    //    spawn.
    let rebuilt: Vec<(bevy::ecs::entity::Entity, Brain)> = {
        let Some(catalog) =
            world.get_resource::<ambition_characters::actor::character_catalog::CharacterCatalog>()
        else {
            return;
        };
        jobs.iter()
            .filter_map(|job| {
                let candidate = catalog.build_brain_from_preset(&job.preset, &job.ctx)?;
                (!job.live.same_authored_configuration(&candidate))
                    .then_some((job.entity, candidate))
            })
            .collect()
    };

    // 3. Write the reconciled brains back.
    for (entity, brain) in rebuilt {
        if let Ok(mut entity_mut) = world.get_entity_mut(entity) {
            entity_mut.insert(brain);
        }
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

/// **The active room's live moving platforms.** Each platform's `pos` and motion
/// cursor are advanced every tick by `advance_moving_platforms`, and the state
/// lives only in this resource (the visual entities carry an index into it), so
/// a within-room rollback must restore it or the platforms resume from the tick
/// we rewound FROM. The codec defers to `ambition_world`'s RON round-trip, which
/// keeps the private `MovingPlatformMotion` cursor encapsulated where it is owned.
impl SnapshotState for ambition_world::collision::MovingPlatformSet {
    fn encode(&self, out: &mut Vec<u8>) {
        put_str(out, &self.to_snapshot_ron());
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Self::from_snapshot_ron(r.str()?)
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

// ─── The projectile family: the first blob-rebuildable dynamic family ────────
//
// Every component an in-flight projectile carries is registered below (the ZST
// markers included), and `ProjectileOwner` — the one `Entity` handle — is
// declared derived and healed per identity pass from the spawned id's parent.
// That is what lets `register_engine_sim_state` declare `projectile_gameplay` a
// DYNAMIC ANCHOR: a dead projectile in a snapshot rebuilds from blobs alone,
// exactly, so a rollback window may span a projectile's whole life.

snapshot_unit_enum!(ambition_platformer_primitives::projectile::WorldHitPolicy {
    Bouncing = 0,
    ExpireOnContact = 1,
});

snapshot_unit_enum!(ambition_projectiles::ProjectileKind {
    Fireball = 0,
    Hadouken = 1,
    HadoukenSuper = 2,
});

impl SnapshotState for ambition_platformer_primitives::projectile::ProjectileGameplay {
    fn encode(&self, out: &mut Vec<u8>) {
        put_f32(out, self.age);
        put_f32(out, self.max_lifetime);
        put_f32(out, self.gravity);
        put_i32(out, self.damage);
        put_u8(out, self.bounces_remaining);
        self.world_hit.encode(out);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self {
            age: r.f32()?,
            max_lifetime: r.f32()?,
            gravity: r.f32()?,
            damage: r.i32()?,
            bounces_remaining: r.u8()?,
            world_hit: ambition_platformer_primitives::projectile::WorldHitPolicy::decode(r)?,
        })
    }
}

impl SnapshotState for ambition_projectiles::ProjectileSeq {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u64(out, self.0);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self(r.u64()?))
    }
}

impl SnapshotState for ambition_projectiles::ProjectileOwnerId {
    fn encode(&self, out: &mut Vec<u8>) {
        put_str(out, &self.0);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self(r.str()?.to_string()))
    }
}

impl SnapshotState for ambition_projectiles::ProjectileVisualId {
    fn encode(&self, out: &mut Vec<u8>) {
        put_str(out, &self.0);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self(r.str()?.to_string()))
    }
}

/// The projectile markers are REGISTERED STATE, encoded as empty rows: presence
/// is the datum. Restoring one restores the archetype — `LiveProjectile` routes
/// the unified stepper, the art tags pick the renderer — which is exactly what
/// a blob-alone rebuild needs and what a marker-less "recipe" would have had to
/// reconstruct out-of-band.
macro_rules! snapshot_marker {
    ($ty:path) => {
        impl SnapshotState for $ty {
            fn encode(&self, _out: &mut Vec<u8>) {}
            fn decode(_r: &mut Reader<'_>) -> Option<Self> {
                Some(Self)
            }
        }
    };
}
snapshot_marker!(ambition_projectiles::LiveProjectile);
snapshot_marker!(ambition_projectiles::PlayerProjectile);
snapshot_marker!(ambition_projectiles::enemy::EnemyProjectile);

/// The global spawn-order stamp source. Two sims that stamped a different
/// number of projectiles are not in the same state; a restore that left the
/// counter at the abandoned future's value would stamp the replay's shots with
/// different orderings than the original run's.
impl SnapshotState for ambition_projectiles::ProjectileSeqCounter {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u64(out, self.0);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self(r.u64()?))
    }
}

/// Re-resolve [`ProjectileOwner`](ambition_projectiles::ProjectileOwner) — the
/// projectile family's one `Entity` handle — from the spawned id's parent.
///
/// N3.1 decision (2) forbids `Entity` in blobs, so the owner handle is DERIVED
/// state: the durable fact is the parent prefix of the projectile's own
/// `SimId` (`placement:duel_pca/0` names its firer in the id), and this system
/// re-resolves it wherever the handle is missing or stale — a blob-rebuilt
/// projectile after a restore, or a shot whose firer was itself rebuilt.
/// Scheduled with the identity pair (head and tail of the sim tick), so an
/// owner is healed before anything reads it.
pub fn heal_projectile_owners(
    mut commands: bevy::ecs::system::Commands,
    projectiles: bevy::ecs::system::Query<
        (
            bevy::ecs::entity::Entity,
            &ambition_platformer_primitives::sim_id::SimId,
            Option<&ambition_projectiles::ProjectileOwner>,
        ),
        bevy::ecs::query::With<ambition_projectiles::LiveProjectile>,
    >,
    identities: bevy::ecs::system::Query<(
        bevy::ecs::entity::Entity,
        &ambition_platformer_primitives::sim_id::SimId,
    )>,
) {
    let mut orphans: Vec<(bevy::ecs::entity::Entity, &str)> = Vec::new();
    for (entity, id, owner) in &projectiles {
        // A live, resolvable handle needs no healing.
        if owner.is_some_and(|owner| identities.get(owner.0).is_ok()) {
            continue;
        }
        // The parent is everything before the id's last `/<seq>` segment. An id
        // with no `/` is not a spawned child and has no parent to resolve.
        if let Some((parent, _seq)) = id.as_str().rsplit_once('/') {
            orphans.push((entity, parent));
        }
    }
    if orphans.is_empty() {
        return;
    }
    let by_id: std::collections::BTreeMap<&str, bevy::ecs::entity::Entity> = identities
        .iter()
        .map(|(entity, id)| (id.as_str(), entity))
        .collect();
    for (entity, parent) in orphans {
        if let Some(owner) = by_id.get(parent) {
            commands
                .entity(entity)
                .insert(ambition_projectiles::ProjectileOwner(*owner));
        }
    }
}

// ─── Encounter authority (E11) ───────────────────────────────────────────────
//
// The generic encounter entity (`Encounter` + `SimId::encounter(id)`) carries
// three snapshot-relevant components. `EncounterLifecycle` and
// `EncounterParticipants` are plain state; `EncounterWaves` is a RESOLVED
// codec — its authored `EncounterSpec` is content the surviving entity still
// carries, so the blob stores only the live run (the choice, not the content).
// Participant `entity` handles are NEVER serialized (N3.1 decision 2): the
// durable identity is the id string, and the adapters re-resolve the live
// entity every tick (wave liveness refresh / boss progress update).

fn encounter_phase_tag(phase: ambition_encounter::EncounterPhase) -> u8 {
    use ambition_encounter::EncounterPhase as P;
    match phase {
        P::Inactive => 0,
        P::Starting { .. } => 1,
        P::Active => 2,
        P::Completed => 3,
        P::Failed => 4,
    }
}

impl SnapshotState for ambition_encounter::EncounterLifecycle {
    fn encode(&self, out: &mut Vec<u8>) {
        use ambition_encounter::EncounterPhase as P;
        put_u8(out, encounter_phase_tag(self.phase));
        if let P::Starting { remaining } = self.phase {
            put_f32(out, remaining);
        }
        put_f32(out, self.intro_seconds);
        put_f32(out, self.elapsed_active);
        // BTreeSet iterates sorted — canonical blob bytes by construction.
        put_u32(out, self.signals.len() as u32);
        for signal in &self.signals {
            put_str(out, signal);
        }
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use ambition_encounter::EncounterPhase as P;
        let phase = match r.u8()? {
            0 => P::Inactive,
            1 => P::Starting {
                remaining: r.f32()?,
            },
            2 => P::Active,
            3 => P::Completed,
            4 => P::Failed,
            _ => return None,
        };
        let intro_seconds = r.f32()?;
        let elapsed_active = r.f32()?;
        let n = r.u32()? as usize;
        let mut signals = std::collections::BTreeSet::new();
        for _ in 0..n {
            signals.insert(r.str()?.to_string());
        }
        Some(ambition_encounter::EncounterLifecycle {
            phase,
            intro_seconds,
            elapsed_active,
            signals,
        })
    }
}

fn encounter_role_tag(role: ambition_encounter::EncounterRole) -> u8 {
    use ambition_encounter::EncounterRole as R;
    match role {
        R::PrimaryTarget => 0,
        R::Elite => 1,
        R::Minion => 2,
        R::Hazard => 3,
        R::Objective => 4,
        R::Protected => 5,
        R::Escort => 6,
        R::Narrative => 7,
        R::Rival => 8,
    }
}

fn encounter_role_from_tag(tag: u8) -> Option<ambition_encounter::EncounterRole> {
    use ambition_encounter::EncounterRole as R;
    Some(match tag {
        0 => R::PrimaryTarget,
        1 => R::Elite,
        2 => R::Minion,
        3 => R::Hazard,
        4 => R::Objective,
        5 => R::Protected,
        6 => R::Escort,
        7 => R::Narrative,
        8 => R::Rival,
        _ => return None,
    })
}

impl SnapshotState for ambition_encounter::EncounterParticipants {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u32(out, self.members.len() as u32);
        for member in &self.members {
            put_str(out, &member.id);
            put_u8(out, encounter_role_tag(member.role));
            put_bool(
                out,
                matches!(member.ownership, ambition_encounter::Ownership::Spawned),
            );
            put_bool(out, member.alive);
            // `member.entity` is deliberately NOT here — an entity index is an
            // allocator slot, not an identity. Re-resolved live from the id.
        }
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        let n = r.u32()? as usize;
        let mut members = Vec::with_capacity(n);
        for _ in 0..n {
            let id = r.str()?.to_string();
            let role = encounter_role_from_tag(r.u8()?)?;
            let ownership = if r.bool()? {
                ambition_encounter::Ownership::Spawned
            } else {
                ambition_encounter::Ownership::Adopted
            };
            let alive = r.bool()?;
            members.push(ambition_encounter::EncounterParticipant {
                id,
                entity: None,
                role,
                ownership,
                alive,
            });
        }
        Some(ambition_encounter::EncounterParticipants { members })
    }
}

impl SnapshotResolve for ambition_encounter::EncounterWaves {
    fn encode_ref(&self, out: &mut Vec<u8>) {
        // The live run — the spec is authored content resolved from the
        // surviving component. Pending mobs are encoded verbatim (small POD;
        // their delays were already adjusted by the inter-wave rule, so they
        // are run state, not a pure spec subset).
        put_bool(out, self.run.wave_index.is_some());
        if let Some(wave_index) = self.run.wave_index {
            put_u32(out, wave_index as u32);
        }
        put_u32(out, self.run.pending.len() as u32);
        for mob in &self.run.pending {
            put_str(out, &mob.kind);
            put_f32(out, mob.spawn[0]);
            put_f32(out, mob.spawn[1]);
            put_f32(out, mob.size[0]);
            put_f32(out, mob.size[1]);
            put_f32(out, mob.delay);
        }
        put_f32(out, self.run.wave_elapsed);
        put_bool(out, self.run.exhausted_signaled);
        put_u32(out, self.spawn_counter);
    }

    fn resolve(
        entity: &bevy::ecs::world::EntityWorldMut<'_>,
        r: &mut Reader<'_>,
    ) -> Result<Option<Self>, ResolveDecodeError> {
        // Blob first (a truncated blob is Err regardless of content).
        let wave_index = if r.bool().ok_or(ResolveDecodeError)? {
            Some(r.u32().ok_or(ResolveDecodeError)? as usize)
        } else {
            None
        };
        let n = r.u32().ok_or(ResolveDecodeError)? as usize;
        let mut pending = Vec::with_capacity(n);
        for _ in 0..n {
            let kind = r.str().ok_or(ResolveDecodeError)?.to_string();
            let spawn = [
                r.f32().ok_or(ResolveDecodeError)?,
                r.f32().ok_or(ResolveDecodeError)?,
            ];
            let size = [
                r.f32().ok_or(ResolveDecodeError)?,
                r.f32().ok_or(ResolveDecodeError)?,
            ];
            let delay = r.f32().ok_or(ResolveDecodeError)?;
            pending.push(ambition_encounter::EncounterMobSpec {
                kind,
                spawn,
                size,
                delay,
            });
        }
        let wave_elapsed = r.f32().ok_or(ResolveDecodeError)?;
        let exhausted_signaled = r.bool().ok_or(ResolveDecodeError)?;
        let spawn_counter = r.u32().ok_or(ResolveDecodeError)?;
        // The authored spec: content the surviving entity still carries.
        let Some(existing) = entity.get::<ambition_encounter::EncounterWaves>() else {
            return Ok(None);
        };
        let mut waves = ambition_encounter::EncounterWaves::new(existing.spec.clone());
        waves.run = ambition_encounter::EncounterRun {
            wave_index,
            pending,
            wave_elapsed,
            exhausted_signaled,
        };
        waves.spawn_counter = spawn_counter;
        Ok(Some(waves))
    }
}

#[cfg(test)]
mod brain_reconcile_tests {
    //! `reconcile_brain_bindings` is the post-restore step that keeps a
    //! catalog-backed NPC's live `Brain` in agreement with its restored
    //! `BrainBinding` after a rewind crosses a runtime brain switch.

    use ambition_characters::actor::character_catalog::{
        parse_catalog, AutonomousBrainSource, BrainBinding, BrainPresetId, CharacterCatalog,
        HostileArchetypeId,
    };
    use ambition_characters::actor::ActorPose;
    use ambition_characters::brain::{Brain, PlayerSlot, StateMachineCfg};
    use ambition_engine_core as ae;
    use ambition_platformer_primitives::sim_id::SimId;
    use bevy::prelude::World;

    const CATALOG: &str = r#"(
        brain_presets: {
            "stand_still": StandStill,
            "wanderer_puppy_slug": Wanderer(speed: 36.0, aggressiveness: 0.0),
            "wanderer_slow": Wanderer(speed: 20.0, aggressiveness: 0.0),
            "wanderer_fast": Wanderer(speed: 200.0, aggressiveness: 0.0),
        },
        action_set_presets: { "peaceful": (move_style: Walk) },
        characters: {
            "npc_puppy_slug": (
                display_name: "Puppy Slug", spritesheet: "x.png", manifest: "x_spritesheet.ron",
                tier: MainHall, body_kind: Crawler, composition: None,
                default_brain: "wanderer_puppy_slug", default_action_set: "peaceful", tags: [],
            ),
        },
    )"#;

    fn wanderer_speed(brain: &Brain) -> f32 {
        match brain {
            Brain::StateMachine(StateMachineCfg::Wanderer { cfg }) => cfg.speed,
            other => panic!("expected a Wanderer brain, got {other:?}"),
        }
    }

    fn world_with_npc(brain: Brain, binding: BrainBinding) -> (World, bevy::ecs::entity::Entity) {
        let mut world = World::new();
        world.insert_resource(CharacterCatalog::from_data(parse_catalog(CATALOG)));
        let e = world
            .spawn((
                SimId::placement("puppy"),
                brain,
                binding,
                ActorPose::from_parts(ae::Vec2::new(100.0, 0.0), ae::Vec2::new(8.0, 8.0), 1.0),
            ))
            .id();
        (world, e)
    }

    #[test]
    fn reconcile_rebuilds_the_brain_when_the_kind_diverged() {
        // A rewind PAST a switch: the binding is Default (wanderer) but the live
        // brain is a since-rewound stand_still. Reconcile rebuilds it to agree
        // with the restored selection — otherwise the next re-simulated tick
        // drives the wrong brain (a desync).
        let (mut world, e) = world_with_npc(
            Brain::stand_still(),
            BrainBinding::new(
                BrainPresetId::new("wanderer_puppy_slug"),
                AutonomousBrainSource::CatalogDefault,
            ),
        );
        super::reconcile_brain_bindings(&mut world);
        assert_eq!(
            world.get::<Brain>(e).unwrap().label(),
            "wanderer",
            "a diverged brain is rebuilt from the restored binding"
        );
    }

    #[test]
    fn reconcile_leaves_a_matching_brain_untouched() {
        // Binding is Override(stand_still) and the live brain IS stand_still: no
        // divergence, so reconcile must not rebuild (it preserves live state).
        let (mut world, e) = world_with_npc(
            Brain::stand_still(),
            BrainBinding::new(
                BrainPresetId::new("wanderer_puppy_slug"),
                AutonomousBrainSource::CatalogPreset(BrainPresetId::new("stand_still")),
            ),
        );
        super::reconcile_brain_bindings(&mut world);
        assert_eq!(
            world.get::<Brain>(e).unwrap().label(),
            "stand_still",
            "a matching brain kind is left as-is"
        );
    }

    #[test]
    fn reconcile_distinguishes_same_family_presets() {
        // Live brain is `wanderer_fast`; the restored binding selects
        // `wanderer_slow`. Both label "wanderer", so a label check would MISS the
        // divergence — configuration equality catches it and rebuilds to slow.
        let cat = CharacterCatalog::from_data(parse_catalog(CATALOG));
        let fast = cat
            .build_brain_from_preset(
                "wanderer_fast",
                &ambition_characters::actor::character_catalog::BrainBuildContext::at(100.0),
            )
            .unwrap();
        let (mut world, e) = world_with_npc(
            fast,
            BrainBinding::new(
                BrainPresetId::new("wanderer_puppy_slug"),
                AutonomousBrainSource::CatalogPreset(BrainPresetId::new("wanderer_slow")),
            ),
        );
        super::reconcile_brain_bindings(&mut world);
        assert_eq!(
            wanderer_speed(world.get::<Brain>(e).unwrap()),
            20.0,
            "reconcile rebuilds to the restored preset's config, not just its family label"
        );
    }

    #[test]
    fn catalog_reconcile_skips_a_provoked_brain() {
        // A provoked source names a roster archetype, not a catalog preset, so the
        // CATALOG reconcile pass leaves it alone (it has no `active_preset`). The
        // roster reconstruction is the autonomous-actor pass's job (ambition_actors),
        // exercised by the full-host integration tests where a roster exists.
        let (mut world, e) = world_with_npc(
            Brain::stand_still(),
            BrainBinding::new(
                BrainPresetId::new("wanderer_puppy_slug"),
                AutonomousBrainSource::Provoked {
                    archetype: HostileArchetypeId::new("combatant"),
                },
            ),
        );
        super::reconcile_brain_bindings(&mut world);
        assert_eq!(
            world.get::<Brain>(e).unwrap().label(),
            "stand_still",
            "the catalog pass never rebuilds a provoked brain to the catalog default"
        );
    }

    #[test]
    fn reconcile_skips_a_player_controlled_body() {
        // A possessed body carries `Brain::Player`; reconcile must not overwrite
        // player control with the autonomous selection.
        let (mut world, e) = world_with_npc(
            Brain::Player(PlayerSlot::PRIMARY),
            BrainBinding::new(
                BrainPresetId::new("wanderer_puppy_slug"),
                AutonomousBrainSource::CatalogDefault,
            ),
        );
        super::reconcile_brain_bindings(&mut world);
        assert!(
            world.get::<Brain>(e).unwrap().is_player(),
            "player control survives reconcile"
        );
    }
}

/// True snapshot take/restore ("rewind") tests for NPC brain switching — these
/// drive the real [`take`](super::take)/[`restore`](super::restore) machinery
/// (not just `reconcile_brain_bindings` in isolation), proving a rewind across a
/// runtime brain switch / challenge restores the exact authored state.
#[cfg(test)]
mod brain_switch_rewind_tests {
    use ambition_characters::actor::character_catalog::{
        parse_catalog, AuthoredBrainContext, AutonomousBrainSource, BrainBinding,
        BrainBuildContext, BrainPresetId, CharacterCatalog, HostileArchetypeId,
    };
    use ambition_characters::actor::ActorPose;
    use ambition_characters::brain::{Brain, PlayerSlot, StateMachineCfg};
    use ambition_combat::components::ActorDisposition;
    use ambition_engine_core as ae;
    use ambition_platformer_primitives::sim_id::SimId;
    use bevy::prelude::World;

    const CATALOG: &str = r#"(
        brain_presets: {
            "stand_still": StandStill,
            "wanderer_puppy_slug": Wanderer(speed: 36.0, aggressiveness: 0.0),
            "wanderer_slow": Wanderer(speed: 20.0, aggressiveness: 0.0),
            "wanderer_fast": Wanderer(speed: 200.0, aggressiveness: 0.0),
            "melee_brute_striker": MeleeBrute(
                aggressiveness: 1.0, aggro_radius: 220.0, attack_range: 36.0, chase_speed: 110.0,
            ),
            "patrol_peaceful": Patrol(
                spawn_local_x: 0.0, radius: 64.0, speed: 28.0,
                aggressiveness: 0.0, aggro_radius: 80.0, attack_range: 0.0,
            ),
            // Two Smash presets of the SAME variant, differing only in authored
            // tuning (aggro radius / chase speed): a label check would confuse them.
            "smash_grunt": Smash(
                aggro_radius: 460.0, engage_distance: 70.0, attack_range: 56.0,
                too_close_distance: 30.0, chase_speed: 170.0, retreat_speed: 130.0,
                crowding_threshold: 0.65, dash_to_close: false,
                reaction_delay_s: 0.15, commit_probability: 0.85, accuracy: 0.85, mash_speed_hz: 6.0,
            ),
            "smash_duelist": Smash(
                aggro_radius: 900.0, engage_distance: 90.0, attack_range: 56.0,
                too_close_distance: 30.0, chase_speed: 240.0, retreat_speed: 130.0,
                crowding_threshold: 0.65, dash_to_close: true,
                reaction_delay_s: 0.05, commit_probability: 0.95, accuracy: 0.95, mash_speed_hz: 8.0,
            ),
            // Two BossPattern presets differing only in encounter id (the authored
            // input; the rest of the cfg is derived from it).
            "boss_alpha": BossPattern(aggressiveness: 1.0, encounter_id: "alpha"),
            "boss_beta": BossPattern(aggressiveness: 1.0, encounter_id: "beta"),
        },
        action_set_presets: { "peaceful": (move_style: Walk) },
        characters: {
            "npc_puppy_slug": (
                display_name: "Puppy Slug", spritesheet: "x.png", manifest: "x_spritesheet.ron",
                tier: MainHall, body_kind: Crawler, composition: None,
                default_brain: "wanderer_puppy_slug", default_action_set: "peaceful", tags: [],
            ),
            "npc_patroller": (
                display_name: "Patroller", spritesheet: "x.png", manifest: "x_spritesheet.ron",
                tier: MainHall, body_kind: Standard, composition: None,
                default_brain: "patrol_peaceful", default_action_set: "peaceful", tags: [],
            ),
        },
    )"#;

    fn registry() -> super::super::SnapshotRegistry {
        let mut reg = super::super::SnapshotRegistry::default();
        reg.register_cursor::<Brain>("brain");
        reg.register_component::<BrainBinding>("brain_binding");
        reg.register_component::<AuthoredBrainContext>("authored_brain_context");
        reg.register_component::<ActorDisposition>("actor_disposition");
        reg
    }

    fn world() -> World {
        let mut w = World::new();
        w.insert_resource(CharacterCatalog::from_data(parse_catalog(CATALOG)));
        w
    }

    fn build(world: &World, preset: &str, anchor_x: f32) -> Brain {
        world
            .resource::<CharacterCatalog>()
            .build_brain_from_preset(preset, &BrainBuildContext::at(anchor_x))
            .expect("preset builds")
    }

    fn spawn(
        world: &mut World,
        sim: &str,
        brain: Brain,
        binding: BrainBinding,
        anchor_x: f32,
    ) -> bevy::ecs::entity::Entity {
        world
            .spawn((
                SimId::placement(sim),
                brain,
                binding,
                AuthoredBrainContext::from_placement(anchor_x, 0.0),
                ActorPose::from_parts(ae::Vec2::new(anchor_x, 0.0), ae::Vec2::new(8.0, 8.0), 1.0),
                ActorDisposition::Peaceful,
            ))
            .id()
    }

    fn wanderer_speed(brain: &Brain) -> f32 {
        match brain {
            Brain::StateMachine(StateMachineCfg::Wanderer { cfg }) => cfg.speed,
            other => panic!("expected Wanderer, got {other:?}"),
        }
    }

    /// Override rewind: default selected, snapshot, switch to an override, restore
    /// → the DEFAULT is back (selection + exact config), because the rewound
    /// binding says Default and reconcile rebuilds it.
    #[test]
    fn override_rewind_restores_the_default() {
        let reg = registry();
        let mut w = world();
        let brain = build(&w, "wanderer_puppy_slug", 100.0);
        let e = spawn(
            &mut w,
            "npc",
            brain,
            BrainBinding::new(
                BrainPresetId::new("wanderer_puppy_slug"),
                AutonomousBrainSource::CatalogDefault,
            ),
            100.0,
        );

        let snap = super::super::take(&w, &reg);

        // Switch to stand_still (as `UsePreset` would).
        *w.get_mut::<Brain>(e).unwrap() = Brain::stand_still();
        w.get_mut::<BrainBinding>(e)
            .unwrap()
            .use_preset(BrainPresetId::new("stand_still"));

        super::super::restore(&mut w, &snap, &reg).expect("restore");

        assert_eq!(
            w.get::<BrainBinding>(e).unwrap().source,
            AutonomousBrainSource::CatalogDefault
        );
        assert_eq!(
            wanderer_speed(w.get::<Brain>(e).unwrap()),
            36.0,
            "the exact default wanderer config is restored"
        );
    }

    /// Same-family rewind: `wanderer_slow` selected, snapshot, switch to
    /// `wanderer_fast`, restore → the exact SLOW config is back — a label check
    /// would have missed it (both "wanderer").
    #[test]
    fn same_family_rewind_restores_the_exact_preset() {
        let reg = registry();
        let mut w = world();
        let slow = build(&w, "wanderer_slow", 100.0);
        let e = spawn(
            &mut w,
            "npc",
            slow,
            BrainBinding::new(
                BrainPresetId::new("wanderer_puppy_slug"),
                AutonomousBrainSource::CatalogPreset(BrainPresetId::new("wanderer_slow")),
            ),
            100.0,
        );

        let snap = super::super::take(&w, &reg);

        let fast = build(&w, "wanderer_fast", 100.0);
        *w.get_mut::<Brain>(e).unwrap() = fast;
        w.get_mut::<BrainBinding>(e)
            .unwrap()
            .use_preset(BrainPresetId::new("wanderer_fast"));

        super::super::restore(&mut w, &snap, &reg).expect("restore");

        assert_eq!(
            wanderer_speed(w.get::<Brain>(e).unwrap()),
            20.0,
            "restore brings back the slow preset's config, not the fast one"
        );
    }

    fn smash_aggro(brain: &Brain) -> f32 {
        match brain {
            Brain::StateMachine(StateMachineCfg::Smash { cfg, .. }) => cfg.aggro_radius,
            other => panic!("expected Smash, got {other:?}"),
        }
    }

    /// Same-family SMASH rewind: `smash_grunt` selected, snapshot, switch to
    /// `smash_duelist` (same variant, different authored tuning), restore → the
    /// exact grunt config is back. The old variant-only comparison would have kept
    /// the duelist's tuning; the full-`SmashCfg` comparison catches it.
    #[test]
    fn same_family_smash_rewind_restores_the_exact_tuning() {
        let reg = registry();
        let mut w = world();
        let grunt = build(&w, "smash_grunt", 100.0);
        let e = spawn(
            &mut w,
            "npc",
            grunt,
            BrainBinding::new(
                BrainPresetId::new("wanderer_puppy_slug"),
                AutonomousBrainSource::CatalogPreset(BrainPresetId::new("smash_grunt")),
            ),
            100.0,
        );

        let snap = super::super::take(&w, &reg);

        let duelist = build(&w, "smash_duelist", 100.0);
        *w.get_mut::<Brain>(e).unwrap() = duelist;
        w.get_mut::<BrainBinding>(e)
            .unwrap()
            .use_preset(BrainPresetId::new("smash_duelist"));

        super::super::restore(&mut w, &snap, &reg).expect("restore");

        assert_eq!(
            smash_aggro(w.get::<Brain>(e).unwrap()),
            460.0,
            "the grunt's authored aggro radius is restored, not the duelist's 900"
        );
    }

    fn boss_encounter(brain: &Brain) -> String {
        match brain {
            Brain::StateMachine(StateMachineCfg::BossPattern { cfg, .. }) => {
                cfg.encounter_id.clone()
            }
            other => panic!("expected BossPattern, got {other:?}"),
        }
    }

    /// Same-family BOSSPATTERN rewind: `boss_alpha` selected, snapshot, switch to
    /// `boss_beta` (same variant, different encounter id), restore → the alpha
    /// encounter is back. Variant-only comparison would have kept beta.
    #[test]
    fn same_family_boss_rewind_restores_the_exact_encounter() {
        let reg = registry();
        let mut w = world();
        let alpha = build(&w, "boss_alpha", 100.0);
        let e = spawn(
            &mut w,
            "npc",
            alpha,
            BrainBinding::new(
                BrainPresetId::new("wanderer_puppy_slug"),
                AutonomousBrainSource::CatalogPreset(BrainPresetId::new("boss_alpha")),
            ),
            100.0,
        );

        let snap = super::super::take(&w, &reg);

        let beta = build(&w, "boss_beta", 100.0);
        *w.get_mut::<Brain>(e).unwrap() = beta;
        w.get_mut::<BrainBinding>(e)
            .unwrap()
            .use_preset(BrainPresetId::new("boss_beta"));

        super::super::restore(&mut w, &snap, &reg).expect("restore");

        assert_eq!(
            boss_encounter(w.get::<Brain>(e).unwrap()),
            "alpha",
            "the alpha encounter is restored, not beta"
        );
    }

    /// Challenge rewind (inverse boundary): snapshot BEFORE the challenge, provoke
    /// (a hostile roster brain + `Provoked` source + Hostile disposition), restore →
    /// the CATALOG pass rebuilds the pre-challenge stand_still from the restored
    /// `CatalogPreset` binding, and the peaceful disposition comes back from its
    /// blob. (The full config reconstruction — tuning / caps / action set — is the
    /// autonomous-actor pass, covered by the integration tests where a roster exists.)
    #[test]
    fn rewind_to_before_a_challenge_restores_peaceful_stand_still() {
        let reg = registry();
        let mut w = world();
        let brain = build(&w, "stand_still", 100.0);
        let e = spawn(
            &mut w,
            "npc",
            brain,
            BrainBinding::new(
                BrainPresetId::new("wanderer_puppy_slug"),
                AutonomousBrainSource::CatalogPreset(BrainPresetId::new("stand_still")),
            ),
            100.0,
        );

        let snap = super::super::take(&w, &reg);

        // Provoke: a hostile roster brain, Hostile disposition, Provoked source.
        let attack = build(&w, "melee_brute_striker", 100.0);
        *w.get_mut::<Brain>(e).unwrap() = attack;
        *w.get_mut::<ActorDisposition>(e).unwrap() = ActorDisposition::Hostile;
        w.get_mut::<BrainBinding>(e)
            .unwrap()
            .provoke(HostileArchetypeId::new("combatant"));

        super::super::restore(&mut w, &snap, &reg).expect("restore");

        assert_eq!(
            w.get::<Brain>(e).unwrap().label(),
            "stand_still",
            "the pre-challenge stand_still brain is rebuilt from the restored binding"
        );
        assert_eq!(
            *w.get::<ActorDisposition>(e).unwrap(),
            ActorDisposition::Peaceful,
            "the pre-challenge peaceful disposition is restored"
        );
    }

    /// Challenge rewind (forward): snapshot AFTER the challenge (Provoked, hostile),
    /// then wrongly calm the actor while its attack brain stays live, restore → the
    /// Hostile disposition and the roster attack brain are BOTH retained (the catalog
    /// pass never rebuilds a `Provoked` brain to the catalog default). With no roster
    /// present here the brain is simply left untouched; the roster reconstruction of a
    /// present that ALSO changed the brain is covered by the integration tests.
    #[test]
    fn rewind_to_after_a_challenge_keeps_hostile_and_the_attack_brain() {
        let reg = registry();
        let mut w = world();
        let attack = build(&w, "melee_brute_striker", 100.0);
        let mut binding = BrainBinding::new(
            BrainPresetId::new("wanderer_puppy_slug"),
            AutonomousBrainSource::CatalogPreset(BrainPresetId::new("stand_still")),
        );
        binding.provoke(HostileArchetypeId::new("combatant"));
        let e = spawn(&mut w, "npc", attack, binding, 100.0);
        *w.get_mut::<ActorDisposition>(e).unwrap() = ActorDisposition::Hostile;

        let snap = super::super::take(&w, &reg);

        // The "present" we rewind FROM keeps the attack brain live (as real
        // rollback does) but drifts other state.
        *w.get_mut::<ActorDisposition>(e).unwrap() = ActorDisposition::Peaceful;

        super::super::restore(&mut w, &snap, &reg).expect("restore");

        assert_eq!(
            *w.get::<ActorDisposition>(e).unwrap(),
            ActorDisposition::Hostile,
            "hostile disposition is restored"
        );
        assert_eq!(
            w.get::<Brain>(e).unwrap().label(),
            "melee_brute",
            "the roster attack brain is retained, not rebuilt to the catalog default"
        );
        assert!(
            w.get::<BrainBinding>(e).unwrap().is_provoked(),
            "the binding agrees: still Provoked"
        );
    }

    /// Authored-home rewind: a patroller that wandered far, its default reselected
    /// on rewind, rebuilds its patrol lane around the AUTHORED anchor (carried by
    /// the snapshot-restored `AuthoredBrainContext`), not its drifted pose.
    #[test]
    fn rewind_rebuilds_patrol_around_the_authored_home() {
        let reg = registry();
        let mut w = world();
        let patrol = build(&w, "patrol_peaceful", 100.0);
        let e = spawn(
            &mut w,
            "npc",
            patrol,
            BrainBinding::new(
                BrainPresetId::new("patrol_peaceful"),
                AutonomousBrainSource::CatalogDefault,
            ),
            100.0,
        );

        let snap = super::super::take(&w, &reg);

        // Wander far and switch brains (as a runtime override would).
        w.get_mut::<ActorPose>(e).unwrap().center.x = 900.0;
        *w.get_mut::<Brain>(e).unwrap() = Brain::stand_still();
        w.get_mut::<BrainBinding>(e)
            .unwrap()
            .use_preset(BrainPresetId::new("stand_still"));

        super::super::restore(&mut w, &snap, &reg).expect("restore");

        match w.get::<Brain>(e).unwrap() {
            Brain::StateMachine(StateMachineCfg::Patrol { cfg, .. }) => {
                assert_eq!(
                    cfg.lane.center_x, 100.0,
                    "the rebuilt lane centers on the authored home, not the drifted pose (900)"
                );
            }
            other => panic!("expected Patrol, got {other:?}"),
        }
    }

    /// Temporary control survives a rewind: a possessed body (`Brain::Player`) is
    /// NOT rebuilt to its autonomous selection by the post-restore reconcile.
    #[test]
    fn a_possessed_body_survives_restore_as_player() {
        let reg = registry();
        let mut w = world();
        let e = spawn(
            &mut w,
            "npc",
            Brain::Player(PlayerSlot::PRIMARY),
            BrainBinding::new(
                BrainPresetId::new("wanderer_puppy_slug"),
                AutonomousBrainSource::CatalogDefault,
            ),
            100.0,
        );

        let snap = super::super::take(&w, &reg);
        // Drift some state, then restore.
        w.get_mut::<ActorPose>(e).unwrap().center.x = 500.0;
        super::super::restore(&mut w, &snap, &reg).expect("restore");

        assert!(
            w.get::<Brain>(e).unwrap().is_player(),
            "player control is not clobbered by the autonomous reconcile across a rewind"
        );
    }
}
