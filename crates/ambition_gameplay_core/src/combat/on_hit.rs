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

use bevy::prelude::{Component, Entity, Message, MessageReader, MessageWriter, Query, Res, With};

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

/// Fire each on-hit hitbox's technique the first time it overlaps a
/// damage-valid victim. Runs while hitboxes are live (in the combat chain,
/// after `apply_hitbox_damage`); one message per (hitbox, victim). Reuses the
/// same overlap ([`ae::CombatVolume::intersects_aabb`]) and faction
/// ([`damage_lands`]) rules the damage resolver uses, so "the volume LANDS"
/// means the same thing here — but decoupled, so the damage path is untouched.
pub fn dispatch_hitbox_on_hit(
    mut hitboxes: Query<(&Hitbox, &mut HitboxOnHit)>,
    // Owner-box center for FollowOwner tracking: actors carry `CenteredAabb`,
    // the player carries `BodyKinematics` (pos = center). Try the box, then the
    // kinematics; an owner-less hitbox contributes nothing.
    owners: Query<&super::components::CenteredAabb>,
    owner_kin: Query<&ae::BodyKinematics>,
    victims: Query<(
        Entity,
        &super::components::CenteredAabb,
        &ActorFaction,
        Option<&ambition_characters::brain::Brain>,
    )>,
    attacker_aggression: Query<&ActorAggression>,
    friendly_fire: Option<Res<crate::features::FriendlyFire>>,
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
        for (victim_entity, victim_aabb, victim_faction, victim_brain) in &victims {
            if victim_entity == hitbox.owner || on_hit.fired.contains(&victim_entity) {
                continue;
            }
            let vf = effective_faction(*victim_faction, victim_brain);
            if !damage_lands(hitbox.source, vf, friendly_fire, owner_grudge, victim_entity) {
                continue;
            }
            if !world_volume.intersects_aabb(victim_aabb.aabb()) {
                continue;
            }
            on_hit.fired.insert(victim_entity);
            out.write(OnHitEffectMessage {
                owner: hitbox.owner,
                victim: victim_entity,
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
/// speed (engine units); omitted → the default pop.
#[derive(serde::Deserialize)]
struct PogoBounceParams {
    #[serde(default = "default_pogo_rise")]
    rise: f32,
}

fn default_pogo_rise() -> f32 {
    520.0
}

impl Default for PogoBounceParams {
    fn default() -> Self {
        Self {
            rise: default_pogo_rise(),
        }
    }
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
    gravity: crate::physics::GravityCtx,
    mut owners: Query<(&mut ae::BodyKinematics, &mut crate::actor::BodyGroundState)>,
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
        // Malformed params fall back to the default pop (a startup param-schema
        // check — R2.2 — turns this into a hard authoring error).
        let params: PogoBounceParams = msg.effect.params.hydrate().unwrap_or_default();
        let Ok((mut kin, mut ground)) = owners.get_mut(msg.owner) else {
            continue;
        };
        let gdir = gravity.dir_at(kin.pos);
        // SET (not add) the jump velocity → idempotent if two victims land the
        // same frame. No cross-frame dedup needed: the owner bounces away.
        let pos = kin.pos;
        ae::movement::set_jump_velocity(&mut kin.vel, gdir, params.rise);
        ground.on_ground = false;
        sfx.write(SfxMessage::Pogo { pos });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::moveset::{advance_move_playback, MoveEventMessage, MovePlayback};
    use ambition_entity_catalog::{
        ClipBinding, HitVolume, MoveSpec, MoveWindow, VolumeShape, WindowTag,
    };
    use ambition_sfx::SfxMessage;
    use ambition_time::WorldTime;
    use bevy::prelude::*;

    /// A down-air whose single Active volume (below the body) carries the pogo
    /// on-hit effect.
    fn pogo_dair() -> MoveSpec {
        MoveSpec {
            id: "attack_air_down".into(),
            clip: ClipBinding {
                clip: "dair".into(),
                fallbacks: vec![],
            },
            duration_s: 0.12,
            windows: vec![MoveWindow {
                start_s: 0.0,
                end_s: 0.12,
                tag: WindowTag::Active,
                volumes: vec![HitVolume {
                    // Body-local +y = gravity-down: the volume sits below the body.
                    shape: VolumeShape::Rect {
                        offset: (0.0, 24.0),
                        half_extents: (18.0, 18.0),
                    },
                    damage: 4,
                    knockback: 0.0,
                    on_hit: Some(EffectRef::new(POGO_BOUNCE_KEY)),
                }],
                sustain_effect: None,
            }],
            events: vec![],
            gates: Default::default(),
            start_impulse: None,
        }
    }

    /// Owner (Player) playing the pogo down-air, a victim (Enemy) directly below
    /// its down-volume. `victim_is_pogoable` toggles the `PogoTarget` capability.
    fn harness(victim_is_pogoable: bool) -> (App, Entity) {
        let mut app = App::new();
        app.add_message::<MoveEventMessage>();
        app.add_message::<OnHitEffectMessage>();
        app.add_message::<SfxMessage>();
        app.init_resource::<WorldTime>();
        app.world_mut().resource_mut::<WorldTime>().scaled_dt = 0.016;
        app.world_mut().resource_mut::<WorldTime>().raw_dt = 0.016;
        app.add_systems(
            Update,
            (
                advance_move_playback,
                dispatch_hitbox_on_hit,
                apply_pogo_bounce,
            )
                .chain(),
        );
        let owner = app
            .world_mut()
            .spawn((
                ae::CenteredAabb::from_center_size(
                    ae::Vec2::new(100.0, 100.0),
                    ae::Vec2::new(28.0, 46.0),
                ),
                ae::BodyKinematics {
                    pos: ae::Vec2::new(100.0, 100.0),
                    vel: ae::Vec2::ZERO,
                    size: ae::Vec2::new(28.0, 46.0),
                    facing: 1.0,
                },
                crate::actor::BodyGroundState {
                    on_ground: true,
                    ..Default::default()
                },
                ActorFaction::Player,
                MovePlayback::new(pogo_dair(), 1.0),
            ))
            .id();
        let victim = app
            .world_mut()
            .spawn((
                ae::CenteredAabb::from_center_size(
                    ae::Vec2::new(100.0, 130.0),
                    ae::Vec2::new(28.0, 46.0),
                ),
                ActorFaction::Enemy,
            ))
            .id();
        if victim_is_pogoable {
            app.world_mut().entity_mut(victim).insert(PogoTarget);
        }
        (app, owner)
    }

    #[test]
    fn down_air_pogos_off_a_pogo_target() {
        let (mut app, owner) = harness(true);
        for _ in 0..2 {
            app.update();
        }
        let kin = app.world().get::<ae::BodyKinematics>(owner).unwrap();
        assert!(
            kin.vel.y < -1.0,
            "the owner rebounded gravity-up (pogo), vel={:?}",
            kin.vel
        );
        assert!(
            !app.world()
                .get::<crate::actor::BodyGroundState>(owner)
                .unwrap()
                .on_ground,
            "the pogo un-grounds the owner",
        );
    }

    #[test]
    fn no_pogo_off_a_bare_victim_without_the_capability() {
        let (mut app, owner) = harness(false);
        for _ in 0..2 {
            app.update();
        }
        let kin = app.world().get::<ae::BodyKinematics>(owner).unwrap();
        assert_eq!(
            kin.vel,
            ae::Vec2::ZERO,
            "a victim without PogoTarget grants no bounce, vel={:?}",
            kin.vel
        );
    }
}
