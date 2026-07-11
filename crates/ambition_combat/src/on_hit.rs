//! On-hit techniques — the conditional-hit primitive of the ability model.
//!
//! A [`HitVolume`](ambition_entity_catalog::HitVolume) may carry an
//! `on_hit: Option<EffectRef>` (fable review AJ1): a technique that fires WHEN
//! the volume lands a hit, with the hit context (owner, victim, contact). This
//! is the missing conditional primitive — pogo, lifesteal, on-hit status,
//! launch modifiers — the counterpart to timed [`MoveEvent`]s (which fire on a
//! clock) and sustained windows (which fire every active frame).
//!
//! Two halves:
//! - **The primitive** ([`HitboxOnHit`] sidecar + [`dispatch_hitbox_on_hit`] +
//!   [`OnHitEffectMessage`]): a moveset hitbox carrying an on-hit effect emits
//!   one message per damage-valid victim it overlaps, once per victim. The
//!   dispatcher is DECOUPLED from the damage resolvers — it re-tests overlap
//!   and reuses [`damage_lands`], so it adds zero risk to the delicate damage
//!   path and covers every hitbox source uniformly (the player's broadcast
//!   melee resolves victims downstream of `apply_hitbox_damage`, so a shared
//!   hook there would miss it).
//! - **The `pogo_bounce` engine technique** ([`PogoTarget`] +
//!   [`apply_pogo_bounce`]): the standard-kit platformer pogo. A down-air whose
//!   Active volume authors `on_hit: Effect("pogo_bounce", (rise: …))` rebounds
//!   the OWNER through the shared jump-velocity seam when it lands on a victim
//!   carrying the [`PogoTarget`] capability. Ships with the engine (AJ1: the
//!   generic platformer kit is engine-provided); a game marks what is pogo-able.

use bevy::prelude::{
    Component, Entity, Has, Message, MessageReader, MessageWriter, Query, Res, With,
};

use ambition_engine_core as ae;
use ambition_entity_catalog::EffectRef;

use super::components::{ActorAggression, ActorFaction};
use super::targeting::{damage_lands, effective_faction};
use ambition_sfx::SfxMessage;
use ambition_vfx::Hitbox;

// ---------------------------------------------------------------------------
// The primitive: on-hit dispatch.
// ---------------------------------------------------------------------------

/// Sidecar on a moveset hitbox entity: the technique to fire when this volume
/// lands a hit. Inserted by
/// [`advance_move_playback`](super::moveset::advance_move_playback) for a
/// `HitVolume` whose `on_hit` is `Some`. Kept OFF `ambition_vfx::Hitbox` (a
/// render-tier type) so the ability vocabulary stays in the gameplay tier.
#[derive(Component, Debug, Clone)]
pub struct HitboxOnHit {
    pub effect: EffectRef,
    /// Victims already fired for — one on-hit per target, mirroring
    /// `HitboxHits`. A fresh hitbox spawns per Active window, so this resets
    /// per strike.
    fired: std::collections::HashSet<Entity>,
}

impl HitboxOnHit {
    pub fn new(effect: EffectRef) -> Self {
        Self {
            effect,
            fired: std::collections::HashSet::new(),
        }
    }
}

/// A landed on-hit: `effect` fires with the hit context. The consuming
/// technique (the engine `pogo_bounce`, or a content technique) hydrates
/// `effect.params` to its own type and acts on `owner` / `victim`.
#[derive(Message, Debug, Clone)]
pub struct OnHitEffectMessage {
    /// The body whose move spawned the hitbox (the attacker).
    pub owner: Entity,
    /// The body the volume landed on.
    pub victim: Entity,
    /// Contact point (the overlapping volume's center) in world space.
    pub contact: ae::Vec2,
    pub effect: EffectRef,
}

/// Fire each on-hit hitbox's technique the first time its volume connects.
/// Runs while hitboxes are live (in the combat chain, after
/// `apply_hitbox_damage`); one message per (hitbox, target). "Connects" means:
/// - a **factioned body** — the damage rule ([`damage_lands`]: an enemy, never
///   an ally), so "the volume LANDS" matches the damage resolver;
/// - a **factionless target** (a world breakable / pogo-orb) — opts in via the
///   [`PogoTarget`] capability, unifying world-orb pogo with victim pogo under
///   the one capability (fable review R2.5, Jon's call). One capability today;
///   generalize to an `OnHitReceptive` marker if a second factionless on-hit
///   effect ever lands.
///
/// Decoupled from the damage resolvers (own overlap + rules), so the delicate
/// damage path is untouched and every hitbox source is covered uniformly.
pub fn dispatch_hitbox_on_hit(
    mut hitboxes: Query<(&Hitbox, &mut HitboxOnHit)>,
    // Owner-box center for FollowOwner tracking: actors carry `CenteredAabb`,
    // the player carries `BodyKinematics` (pos = center). Try the box, then the
    // kinematics; an owner-less hitbox contributes nothing.
    owners: Query<&super::components::CenteredAabb>,
    owner_kin: Query<&ae::BodyKinematics>,
    targets: Query<(
        Entity,
        &super::components::CenteredAabb,
        Option<&ActorFaction>,
        Option<&ambition_characters::brain::Brain>,
        Has<PogoTarget>,
    )>,
    attacker_aggression: Query<&ActorAggression>,
    friendly_fire: Option<Res<crate::targeting::FriendlyFire>>,
    mut out: MessageWriter<OnHitEffectMessage>,
) {
    let friendly_fire = friendly_fire.map(|r| *r).unwrap_or_default();
    for (hitbox, mut on_hit) in &mut hitboxes {
        let owner_pos = if let Ok(aabb) = owners.get(hitbox.owner) {
            aabb.center
        } else if let Ok(kin) = owner_kin.get(hitbox.owner) {
            kin.pos
        } else {
            continue;
        };
        let world_volume = hitbox.world_volume(owner_pos);
        let owner_grudge = attacker_aggression
            .get(hitbox.owner)
            .ok()
            .and_then(|a| a.grudge);
        for (target, target_aabb, faction, brain, is_pogo_target) in &targets {
            if target == hitbox.owner || on_hit.fired.contains(&target) {
                continue;
            }
            let connects = match faction {
                Some(f) => {
                    let vf = effective_faction(*f, brain);
                    damage_lands(
                        crate::actor_faction_from_hit_side(hitbox.source),
                        vf,
                        friendly_fire,
                        owner_grudge,
                        target,
                    )
                }
                // Factionless world target: eligible iff pogo-able.
                None => is_pogo_target,
            };
            if !connects || !world_volume.intersects_aabb(target_aabb.aabb()) {
                continue;
            }
            on_hit.fired.insert(target);
            out.write(OnHitEffectMessage {
                owner: hitbox.owner,
                victim: target,
                contact: world_volume.center(),
                effect: on_hit.effect.clone(),
            });
        }
    }
}

// ---------------------------------------------------------------------------
// The `pogo_bounce` engine technique.
// ---------------------------------------------------------------------------

/// The `on_hit` effect key the engine [`apply_pogo_bounce`] technique answers.
pub const POGO_BOUNCE_KEY: &str = "pogo_bounce";

/// Capability marker: this body can be pogo-bounced off. A game adds it to
/// pogo-able enemies / hazards / breakables; the [`apply_pogo_bounce`]
/// technique gates on it (AJ1: "gated on the victim's pogo-target capability").
/// Distinct from the world-block `PogoOrb`/`Rebound` path the legacy player
/// pogo uses — this is the actor-hurtbox capability the moveset down-air reads.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct PogoTarget;

/// Params for the `pogo_bounce` technique. `rise` is the gravity-up rebound
/// speed (engine units); omitted → the default pop (matches the flat player
/// `pogo_speed` for feel parity).
#[derive(serde::Deserialize)]
struct PogoBounceParams {
    #[serde(default = "default_pogo_rise")]
    rise: f32,
}

fn default_pogo_rise() -> f32 {
    720.0
}

impl Default for PogoBounceParams {
    fn default() -> Self {
        Self {
            rise: default_pogo_rise(),
        }
    }
}

/// The rebound speed a `pogo_bounce` [`EffectRef`] carries — hydrated from its
/// params, defaulting when absent/malformed. Shared by the entity pogo
/// ([`apply_pogo_bounce`]) and the world-orb pogo (`pogo_moveset_off_world_orbs`).
pub fn pogo_rise_from(effect: &EffectRef) -> f32 {
    effect
        .params
        .hydrate::<PogoBounceParams>()
        .unwrap_or_default()
        .rise
}

/// The engine pogo technique: rebound the OWNER (gravity-up) when its on-hit
/// volume lands on a [`PogoTarget`] victim. Sets the jump velocity through the
/// shared [`ae::movement::set_jump_velocity`] seam (frame-correct under any
/// gravity) and un-grounds the owner, so a down-air off a pogo-able foe pops
/// the attacker up — the platformer staple, now a data-authored `on_hit` rather
/// than a hardcoded player branch.
pub fn apply_pogo_bounce(
    mut messages: MessageReader<OnHitEffectMessage>,
    pogo_targets: Query<(), With<PogoTarget>>,
    gravity: ambition_platformer_primitives::gravity::GravityCtx,
    mut owners: Query<(
        &mut ae::BodyKinematics,
        &mut ambition_engine_core::BodyGroundState,
    )>,
    mut sfx: MessageWriter<SfxMessage>,
) {
    for msg in messages.read() {
        if msg.effect.key != POGO_BOUNCE_KEY {
            continue;
        }
        // Gate on the victim's pogo-target capability.
        if pogo_targets.get(msg.victim).is_err() {
            continue;
        }
        let rise = pogo_rise_from(&msg.effect);
        let Ok((mut kin, mut ground)) = owners.get_mut(msg.owner) else {
            continue;
        };
        let gdir = gravity.dir_at(kin.pos);
        // SET (not add) the jump velocity → idempotent if two victims land the
        // same frame. No cross-frame dedup needed: the owner bounces away.
        let pos = kin.pos;
        ae::movement::set_jump_velocity(&mut kin.vel, gdir, rise);
        ground.on_ground = false;
        sfx.write(SfxMessage::Pogo { pos });
    }
}

#[cfg(test)]
mod tests;
