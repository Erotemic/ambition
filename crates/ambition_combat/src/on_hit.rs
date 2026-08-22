//! On-hit techniques — conditional effects driven by resolved strike facts.
//!
//! A [`HitVolume`](ambition_entity_catalog::HitVolume) may carry an `on_hit: Option<EffectRef>`: a
//! technique that fires WHEN the volume lands a body hit, with attacker/victim/contact context. The
//! important ownership rule is that on-hit code does not rediscover contact. This module
//! projects that fact to the authored effect.

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

/// `POGO_BOUNCE_KEY`, `PogoBounceParams` and their three accessors sat in this
/// module beside the system that executes the rebound. The moveset PREFABS name
/// the key and call `set_pogo_sfx` while building a contract, and character
/// PREPARATION calls the prefabs — so while the technique's SCHEMA lived in
/// `ambition_combat`, which depends on `ambition_characters`, the authoritative
/// character model could not follow it down. Those three lines were the last
/// obstacle on that row.
pub use ambition_characters::technique::{pogo_rise_from, set_pogo_sfx, POGO_BOUNCE_KEY};

/// The contact cue a `pogo_bounce` effect authored, as an [`SfxId`].
///
/// the adapter, and it is why the lowered accessor returns a `String`.
/// Wrapping the cue down there would mean an `ambition_characters →
/// ambition_sfx` edge for one newtype; the layering is better with the low crate
/// owning the authored TEXT and this crate deciding the text names a cue.
///
/// [`SfxId`]: ambition_sfx::SfxId
pub fn pogo_sfx_from(effect: &EffectRef) -> Option<ambition_sfx::SfxId> {
    ambition_characters::technique::pogo_sfx_cue_from(effect)
        .map(|cue| ambition_sfx::SfxId::new(&cue))
}

/// The engine pogo technique: rebound the OWNER (gravity-up) when its authored
/// on-hit effect came from a resolved body strike and the victim's pogo policy
/// accepts that same contact.
pub fn apply_pogo_bounce(
    mut messages: MessageReader<OnHitEffectMessage>,
    // `Option`, like every other reader of the projection: a composition that
    // never installs the rules projection keeps the baseline, which pogos.
    rules: Option<bevy::prelude::Res<crate::rules::ResolvedCombatTuning>>,
    pogo_targets: Query<(&PogoPolicy, Option<&PogoTargetVolumes>)>,
    mut owners: Query<(
        &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
        &mut ae::BodyKinematics,
        &mut ambition_platformer2d_core::BodyGroundState,
    )>,
    mut sfx: ambition_sfx::BodySfxWriter,
) {
    // read ONCE, and the read is what makes a spike a spike: a stage that
    // declares `Spike` drops every rebound this frame rather than some of them.
    if matches!(
        rules.as_deref().copied().unwrap_or_default().downward_hit,
        crate::rules::DownwardHitStyle::Spike
    ) {
        messages.clear();
        return;
    }
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
        // SET (not add) the jump velocity → idempotent if two victims land the same frame.
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
