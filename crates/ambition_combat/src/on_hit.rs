//! On-hit techniques — conditional effects driven by resolved strike facts.
//!
//! A [`HitVolume`](ambition_entity_catalog::HitVolume) may carry an
//! `on_hit: Option<EffectRef>`: a technique that fires WHEN the volume lands a
//! body hit, with attacker/victim/contact context. The important ownership rule
//! is that on-hit code does **not** rediscover contact. The shared hitbox
//! resolver already knows whether a strike landed, and publishes one
//! [`LandedBodyHit`](super::hitbox::LandedBodyHit) fact. This module projects
//! that fact to the authored effect.
//!
//! Two halves:
//! - **The primitive** ([`HitboxOnHit`] + [`dispatch_landed_hit_effects`] +
//!   [`OnHitEffectMessage`]): attach an effect to a live strike, then consume the
//!   resolver's landed-body fact. No second overlap pass, faction pass, or
//!   self-exclusion rule exists here.
//! - **The `pogo_bounce` engine technique** ([`apply_pogo_bounce`]): rebound the
//!   attacker when the resolved victim's [`PogoPolicy`](super::components::PogoPolicy)
//!   accepts the contact. Genuine world pogo surfaces stay on the separate
//!   collision-world path because they have no victim body.

use bevy::prelude::{Component, Entity, Message, MessageReader, MessageWriter, Query};

use ambition_entity_catalog::EffectRef;
use ambition_platformer2d_core as ae;

use super::components::{PogoPolicy, PogoTargetVolumes};
use super::hitbox::LandedBodyHit;
use ambition_sfx::SfxMessage;

// ---------------------------------------------------------------------------
// The primitive: resolved body-hit -> authored effect.
// ---------------------------------------------------------------------------

/// Sidecar on a moveset hitbox entity: the technique to fire when this strike
/// lands a body hit. Inserted by
/// [`advance_move_playback`](super::moveset::advance_move_playback) for a
/// `HitVolume` whose `on_hit` is `Some`.
///
/// Body-hit deduplication belongs to `HitboxHits`, the same state that prevents
/// duplicate damage. `world_fired` exists only for entity-less world pogo
/// surfaces, which cannot participate in that entity-keyed set.
#[derive(Component, Debug, Clone)]
pub struct HitboxOnHit {
    pub effect: EffectRef,
    world_fired: bool,
}

impl HitboxOnHit {
    pub fn new(effect: EffectRef) -> Self {
        Self {
            effect,
            world_fired: false,
        }
    }

    /// Has this strike already fired its entity-less world-contact effect?
    pub fn world_fired(&self) -> bool {
        self.world_fired
    }

    /// Mark the strike's entity-less world-contact effect as fired.
    pub fn mark_world_fired(&mut self) {
        self.world_fired = true;
    }
}

/// One authored on-hit effect projected from an authoritative landed-body fact.
#[derive(Message, Debug, Clone)]
pub struct OnHitEffectMessage {
    /// Body whose move spawned the strike.
    pub owner: Entity,
    /// Concrete body selected by the shared hit resolver.
    pub victim: Entity,
    /// Exact world-space strike volume that connected.
    pub volume: ae::CombatVolume,
    /// Representative world-space contact point.
    pub contact: ae::Vec2,
    pub effect: EffectRef,
}

/// Project resolved body contacts to their authored on-hit techniques.
///
/// The resolver has already performed self-exclusion, relationship/team policy,
/// published-hurtbox overlap, tangibility, and per-strike deduplication. Repeating
/// any of those decisions here would create a second definition of "landed" and
/// let damage, move confirms, and on-hit techniques disagree.
pub fn dispatch_landed_hit_effects(
    mut landed_hits: MessageReader<LandedBodyHit>,
    on_hit: Query<&HitboxOnHit>,
    mut out: MessageWriter<OnHitEffectMessage>,
) {
    for landed in landed_hits.read() {
        let Ok(on_hit) = on_hit.get(landed.hitbox) else {
            continue;
        };
        out.write(OnHitEffectMessage {
            owner: landed.attacker,
            victim: landed.victim,
            volume: landed.volume.clone(),
            contact: landed.contact,
            effect: on_hit.effect.clone(),
        });
    }
}

// ---------------------------------------------------------------------------
// The `pogo_bounce` engine technique.
// ---------------------------------------------------------------------------

/// The `on_hit` effect key the engine [`apply_pogo_bounce`] technique answers.
pub const POGO_BOUNCE_KEY: &str = "pogo_bounce";

/// Params for the `pogo_bounce` technique. `rise` is the gravity-up rebound
/// speed (engine units); omitted → the default pop (matches the flat player
/// `pogo_speed` for feel parity). `sfx` names the contact cue this particular
/// body's rebound makes; omitted → the engine's generic `Pogo` cue.
#[derive(serde::Serialize, serde::Deserialize)]
struct PogoBounceParams {
    #[serde(default = "default_pogo_rise")]
    rise: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sfx: Option<String>,
}

fn default_pogo_rise() -> f32 {
    720.0
}

impl Default for PogoBounceParams {
    fn default() -> Self {
        Self {
            rise: default_pogo_rise(),
            sfx: None,
        }
    }
}

/// The rebound speed a `pogo_bounce` [`EffectRef`] carries — hydrated from its
/// params, defaulting when absent/malformed. Shared by resolved-body pogo
/// ([`apply_pogo_bounce`]) and world-surface pogo (`pogo_moveset_off_world_orbs`).
pub fn pogo_rise_from(effect: &EffectRef) -> f32 {
    effect
        .params
        .hydrate::<PogoBounceParams>()
        .unwrap_or_default()
        .rise
}

/// The contact cue a `pogo_bounce` [`EffectRef`] authored, if any. `None` means
/// "this body has nothing special to say about rebounding" and the caller falls
/// back to the engine's generic pogo cue.
///
/// This is what keeps the pogo sound ATTACK-owned: without it, a body whose
/// blade should clang differently on a rebound could only be told apart by its
/// character id, and the technique doc's claim to be "a data-authored `on_hit`
/// rather than a hardcoded player branch" would stop being true.
pub fn pogo_sfx_from(effect: &EffectRef) -> Option<ambition_sfx::SfxId> {
    effect
        .params
        .hydrate::<PogoBounceParams>()
        .ok()
        .and_then(|params| params.sfx)
        .map(|cue| ambition_sfx::SfxId::new(&cue))
}

/// Author `cue` as this `pogo_bounce` effect's contact sound, preserving any
/// `rise` already on it. Applied when a body's presentation family is overlaid
/// onto its derived moveset, so the runtime never has to ask WHO bounced.
pub fn set_pogo_sfx(effect: &mut EffectRef, cue: &str) {
    let mut params = effect
        .params
        .hydrate::<PogoBounceParams>()
        .unwrap_or_default();
    params.sfx = Some(cue.to_string());
    // The params are opaque `ron::Value` by design, so this stores exactly the
    // text an author would have written by hand. The value being serialized is
    // this module's own two-field struct, so a failure here is a broken schema,
    // not bad content — and swallowing it would spend the rest of the session
    // playing the generic pogo with nothing to say why.
    effect.params = ambition_entity_catalog::ParamValue::from_typed(&params)
        .expect("PogoBounceParams must round-trip through its own authored RON form");
}

/// The engine pogo technique: rebound the OWNER (gravity-up) when its authored
/// on-hit effect came from a resolved body strike and the victim's pogo policy
/// accepts that same contact.
///
/// `FromDamageable` needs no geometry re-test: the shared strike resolver already
/// proved the attack reached the victim's damageable silhouette. `Custom` is the
/// one case with a distinct pogo silhouette, so it explicitly tests the exact
/// landed strike volume against [`PogoTargetVolumes`]. `Disabled` rejects it.
pub fn apply_pogo_bounce(
    mut messages: MessageReader<OnHitEffectMessage>,
    pogo_targets: Query<(&PogoPolicy, Option<&PogoTargetVolumes>)>,
    mut owners: Query<(
        &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
        &mut ae::BodyKinematics,
        &mut ambition_platformer2d_core::BodyGroundState,
    )>,
    mut sfx: ambition_sfx::BodySfxWriter,
) {
    for msg in messages.read() {
        if msg.effect.key != POGO_BOUNCE_KEY {
            continue;
        }
        let Ok((policy, pogo_volumes)) = pogo_targets.get(msg.victim) else {
            continue;
        };
        let pogoable = match *policy {
            PogoPolicy::FromDamageable => true,
            PogoPolicy::Custom => pogo_volumes.is_some_and(|volumes| {
                volumes
                    .volumes
                    .iter()
                    .copied()
                    .any(|aabb| msg.volume.intersects_aabb(aabb))
            }),
            PogoPolicy::Disabled => false,
        };
        if !pogoable {
            continue;
        }

        let rise = pogo_rise_from(&msg.effect);
        let cue = pogo_sfx_from(&msg.effect);
        let Ok((resolved_frame, mut kin, mut ground)) = owners.get_mut(msg.owner) else {
            continue;
        };
        // The owner's per-tick resolved frame (ADR 0024): the bounce launches
        // opposite the same down its movement integrated under.
        let gdir = resolved_frame.down();
        // SET (not add) the jump velocity → idempotent if two victims land the
        // same frame. No cross-frame dedup is needed: `HitboxHits` guarantees one
        // landed fact per victim per strike, and the bounce sends the owner away.
        let pos = kin.pos;
        ae::movement::set_jump_velocity(&mut kin.vel, gdir, rise);
        ground.on_ground = false;
        sfx.write_for(
            msg.owner,
            match cue {
                Some(id) => SfxMessage::Play { id, pos },
                None => SfxMessage::Pogo { pos },
            },
        );
    }
}

#[cfg(test)]
mod tests;
