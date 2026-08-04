//! `SnapshotState` for this crate's own types — the rollback wire format.
//!
//! ⚠ These impls live HERE, beside the types they encode, because
//! `ambition_platformer2d_core::snapshot` owns the trait and the orphan rule binds an
//! impl to the crate owning the trait OR the type. Until 2026-07-30 the trait
//! sat in `ambition_platformer2d_runtime`, above every domain crate, so the only place all
//! ~100 of them could compile was one 2688-line file in `ambition_platformer2d_runtime`. The
//! orphan rule is what proves this file is in the right crate: if a type moves,
//! this stops compiling rather than drifting.
//!
//! ⚠ A field added to an encoded type is a WIRE FORMAT change. Encode and
//! decode must stay in the same order, and `snapshot_unit_enum!` codes are
//! authored per variant so inserting one never renumbers the rest.

use ambition_platformer2d_core::snapshot::{
    put_bool, put_f32, put_i32, put_opt_str, put_str, put_u32, put_u8, put_vec2, Reader,
    SnapshotCursor, SnapshotResolve, SnapshotState,
};
use ambition_platformer2d_core::{snapshot_pod, snapshot_unit_enum};

impl SnapshotState for crate::targeting::MatchTeam {
    fn encode(&self, out: &mut Vec<u8>) {
        put_str(out, self.as_str());
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(crate::targeting::MatchTeam::new(r.str()?))
    }
}

impl SnapshotState for crate::components::RulesetOwnsDeath {
    fn encode(&self, _out: &mut Vec<u8>) {}
    fn decode(_r: &mut Reader<'_>) -> Option<Self> {
        Some(crate::components::RulesetOwnsDeath)
    }
}

// S4 — the stocks loop's own state. A stock count that is not rollback state
// UN-SPENDS itself on a rewind: the body comes back, the count does not, and a
// fighter can lose the same stock twice or never lose it at all. Elimination is
// the same fact one step later.
impl SnapshotState for crate::components::FighterStocks {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u32(out, self.remaining);
        put_u32(out, self.started_with);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        let remaining = r.u32()?;
        let started_with = r.u32()?;
        Some(crate::components::FighterStocks {
            remaining,
            started_with,
        })
    }
}

impl SnapshotState for crate::stocks::StocksMatchSettled {
    fn encode(&self, out: &mut Vec<u8>) {
        put_bool(out, self.0);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(crate::stocks::StocksMatchSettled(r.bool()?))
    }
}

impl SnapshotState for crate::stocks::FighterEliminated {
    fn encode(&self, _out: &mut Vec<u8>) {}
    fn decode(_r: &mut Reader<'_>) -> Option<Self> {
        Some(crate::stocks::FighterEliminated)
    }
}

impl SnapshotState for crate::components::ActorIntent {
    fn encode(&self, out: &mut Vec<u8>) {
        self.0.encode(out);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(crate::components::ActorIntent(
            ambition_characters::actor::ai::CharacterAiMode::decode(r)?,
        ))
    }
}

impl SnapshotState for crate::components::BodyEnvelope {
    fn encode(&self, out: &mut Vec<u8>) {
        put_vec2(out, self.0);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(crate::components::BodyEnvelope(r.vec2()?))
    }
}

snapshot_pod!(crate::components::ActorCooldowns {
    attack_cooldown: f32,
    respawn_timer: f32,
});

/// `ActorTarget` is half derived, half state — see its definition-site snapshot story.
/// `entity` is rebuilt every tick by `select_actor_targets`; `pos` survives the frame
/// where no candidate exists, and a chasing brain aims at it. So `pos` rewinds and
/// `entity` does not.
/// The blob is `(move id, facing, t, landed_hit)`; the `MoveSpec` comes back out of the
/// entity's own `ActorMoveset`, which a patched entity still carries.
///
/// ⛔ **the narrative that used to sit here described a DELETED mechanism.** It
/// said `live_boxes` comes back empty and `fired` is rebuilt from `t`, "both by
/// `MovePlayback::resumed`". Under bevy_ggrs (ADR 0027) the registration is
/// `rollback_component_resolved`, a CLONE snapshot: the whole component is
/// restored, `fired` and `hit_targets` included, and `live_boxes` is
/// entity-remapped rather than emptied. A comment describing a snapshot engine
/// that no longer exists is worse than none — it tells the next reader the dedup
/// state is not preserved when it is (GPT 5.6 review, 2026-08-04).
///
/// What survives from that story is why the cache is safe to restore: a strike
/// volume's existence is DERIVED from `(t, window)`, and
/// `retire_orphaned_strike_volumes` re-checks that against the live world every
/// frame, so a restored slot naming a dead entity is dropped and respawned.
///
/// ⭐ **and `hit_targets` is in the CHECKSUM now.** The restore always carried it;
/// the projection did not, so two peers could disagree about which target a
/// multi-tick strike had already hit and still agree on the hash — a divergence
/// that surfaces later as different damage and SFX. Sorted before hashing because
/// it is a SET: the same targets struck in either order are the same state, and
/// hashing the insertion order would report a false divergence.
///
/// A move id the moveset no longer knows resolves to `None`, and the component is left
/// off. That is a content change between snapshot and restore — impossible in a
/// rollback, and a loud, correct failure in a save file.
impl SnapshotResolve for crate::moveset::MovePlayback {
    fn encode_ref(&self, out: &mut Vec<u8>) {
        put_str(out, &self.spec.id);
        put_f32(out, self.facing);
        put_f32(out, self.t);
        put_bool(out, self.landed_hit);
        let mut targets: Vec<&str> = self.hit_targets.iter().map(String::as_str).collect();
        targets.sort_unstable();
        put_u32(out, targets.len() as u32);
        for target in targets {
            put_str(out, target);
        }
        // ⚠ **the AIM is state, so it is checksummed** — added in the same change
        // that started carrying it. Two peers whose in-flight move disagrees
        // about the direction it will fire have diverged, and the shot it
        // produces later is the visible consequence. The POLICY is part of the
        // value: body-local and world `(1,0)` are different shots.
        match self.aim {
            Some((dir, policy)) => {
                put_bool(out, true);
                put_vec2(out, dir);
                put_u32(out, policy as u32);
            }
            None => put_bool(out, false),
        }
    }
}

impl SnapshotState for crate::components::BodyMelee {
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
            Some(crate::components::MeleeSwing {
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

snapshot_unit_enum!(crate::components::ActorDisposition {
    Peaceful = 0,
    Hostile = 1,
});

/// Mutable aggression policy and provocation count. The `target` and `grudge`
/// fields are entity-handle caches/relationships: target selection republishes
/// `target`, while content-staged batch reconstruction restores authored grudges.
/// Encoding allocator-local `Entity` values would violate the stable-id contract.
impl SnapshotCursor for crate::components::ActorAggression {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        use crate::components::AggressionMode;
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
}

impl SnapshotCursor for crate::components::ActorTarget {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        put_vec2(out, self.pos);
    }
}

snapshot_unit_enum!(crate::components::BossPhase {
    Active = 0,
    Defeated = 1,
});

impl SnapshotState for crate::components::BossPatternTimer {
    fn encode(&self, out: &mut Vec<u8>) {
        put_f32(out, self.0);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(crate::components::BossPatternTimer(r.f32()?))
    }
}

/// **The combat slot board**: which attacker holds which approach slot around the
/// target. The slot GEOMETRY is authored (`kind`, `offset`, `holding_offset`); the
/// `assigned_to: Option<String>` is live, and it is a stable id rather than an `Entity`,
/// so it rewinds cleanly. A boss holding a slot it never claimed attacks on a tick it
/// never earned.
impl SnapshotCursor for crate::slots::CombatSlotsRes {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        put_u32(out, self.0.slots.len() as u32);
        for slot in &self.0.slots {
            put_opt_str(out, slot.assigned_to.as_deref());
        }
    }
}

fn put_attack_intent(out: &mut Vec<u8>, intent: crate::AttackIntent) {
    use crate::AttackIntent;
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

fn read_attack_intent(r: &mut Reader<'_>) -> Option<crate::AttackIntent> {
    use crate::AttackIntent;
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

fn put_damage_kind(out: &mut Vec<u8>, kind: crate::DamageKind) {
    use crate::DamageKind;
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

fn read_damage_kind(r: &mut Reader<'_>) -> Option<crate::DamageKind> {
    use crate::DamageKind;
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

fn put_attack_spec(out: &mut Vec<u8>, spec: crate::AttackSpec) {
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

fn read_attack_spec(r: &mut Reader<'_>) -> Option<crate::AttackSpec> {
    Some(crate::AttackSpec {
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
