//! Rollback wire-format implementations for combat-owned types.
//!
//! Encode/decode field order is schema. `snapshot_unit_enum!` codes are explicit so inserting a
//! variant does not renumber existing wire values.

use ambition_platformer2d_core::snapshot::{
    put_bool, put_f32, put_i32, put_str, put_u32, put_u8, put_vec2, Reader, SnapshotCursor,
    SnapshotResolve, SnapshotState,
};
use ambition_platformer2d_core::snapshot_unit_enum;

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

// Presence is authoritative match state because elimination removes this marker.
impl SnapshotState for crate::components::ActiveCombatant {
    fn encode(&self, _out: &mut Vec<u8>) {}
    fn decode(_r: &mut Reader<'_>) -> Option<Self> {
        Some(crate::components::ActiveCombatant)
    }
}

// Death interlude state is authoritative simulation state and must rewind with the body.
impl SnapshotState for crate::death_rules::OutOfPlay {
    fn encode(&self, _out: &mut Vec<u8>) {}
    fn decode(_r: &mut Reader<'_>) -> Option<Self> {
        Some(crate::death_rules::OutOfPlay)
    }
}

impl SnapshotState for crate::death_rules::DeathInterlude {
    fn encode(&self, out: &mut Vec<u8>) {
        put_f32(out, self.remaining);
        // Pending consequence crosses a frame boundary and therefore belongs in rollback state.
        put_bool(out, self.consequence_pending);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(crate::death_rules::DeathInterlude {
            remaining: r.f32()?,
            consequence_pending: r.bool()?,
        })
    }
}

// Stocks and elimination are authoritative match state and must rewind with fighters.
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

impl SnapshotState for crate::stocks::FighterEliminated {
    fn encode(&self, _out: &mut Vec<u8>) {}
    fn decode(_r: &mut Reader<'_>) -> Option<Self> {
        Some(crate::stocks::FighterEliminated)
    }
}

impl SnapshotState for crate::stocks::PendingRespawn {
    // A BARE PRESENCE, and the clock it used to carry now rides
    // `DeathInterlude` beside it — which is registered too, so the pair still
    // restores "this fighter is coming back, and on this tick". Encoding a
    // second copy of the countdown here would be two rewindable answers to one
    // question, which is worse than none.
    fn encode(&self, _out: &mut Vec<u8>) {}
    fn decode(_r: &mut Reader<'_>) -> Option<Self> {
        Some(crate::stocks::PendingRespawn)
    }
}

impl SnapshotState for crate::stocks::RespawnGrace {
    // The remaining beat is snapshot state for the same reason every other timer
    // is: a rewind that restored the protection but not its clock resimulates a
    // fighter that is safe for the wrong length of time.
    fn encode(&self, out: &mut Vec<u8>) {
        put_f32(out, self.remaining);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(crate::stocks::RespawnGrace {
            remaining: r.f32()?,
        })
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

/// `ActorTarget` is half derived, half state — see its definition-site snapshot story.
/// `entity` is rebuilt every tick by `select_actor_targets`; `pos` survives the frame
/// where no candidate exists, and a chasing brain aims at it. So `pos` rewinds and
/// `entity` does not.
/// The blob is `(move id, facing, t, landed_hit)`; the `MoveSpec` comes back out of the
/// entity's own `ActorMoveset`, which a patched entity still carries.
///
/// What survives from that story is why the cache is safe to restore: a strike
/// volume's existence is DERIVED from `(t, window)`, and
/// `retire_orphaned_strike_volumes` re-checks that against the live world every
/// frame, so a restored slot naming a dead entity is dropped and respawned.
///
/// and `hit_targets` is in the CHECKSUM now. The restore always carried it;
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
        // the AIM is state, so it is checksummed — added in the same change
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
        // THE CHARGE IS STATE. Two peers whose held smash disagrees about how
        // long it has been held will land different damage and different
        // knockback from the same move, so the hold and the frozen release
        // fraction are both hashed. The POLICY rides along because it is
        // resolved once at the move's start and then frozen — a peer that
        // reconstructed it from a differently-authored spec would agree about
        // the elapsed hold and disagree about what it bought.
        // A LOOPED move's lap count is state: a rewind that restored the clock
        // without it would resimulate a flurry with a fresh maximum, so one
        // peer's rapid jab ends and the other's does not.
        put_f32(out, self.looped_s);
        match self.charge {
            Some(charge) => {
                put_bool(out, true);
                put_f32(out, charge.policy.hold_at_s);
                put_f32(out, charge.policy.max_hold_s);
                put_f32(out, charge.held_s);
                match charge.released_fraction {
                    Some(f) => {
                        put_bool(out, true);
                        put_f32(out, f);
                    }
                    None => put_bool(out, false),
                }
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

/// The stale ring, by hand, because `snapshot_pod!` cannot spell an array.
///
/// That macro maps a field to a READER METHOD, and there is none for `[u32; 9]`.
/// Written out rather than flattened into nine named fields so the ring stays a
/// ring — and the explicit `[0u32; 9]` on the decode side is what the codec-shape
/// checker reads as the width.
impl SnapshotState for crate::stale::BodyStaleMoves {
    fn encode(&self, out: &mut Vec<u8>) {
        for slot in self.recent {
            put_u32(out, slot);
        }
        put_u8(out, self.next);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        let mut recent = [0u32; 9];
        for slot in recent.iter_mut() {
            *slot = r.u32()?;
        }
        Some(Self {
            recent,
            next: r.u8()?,
        })
    }
}
