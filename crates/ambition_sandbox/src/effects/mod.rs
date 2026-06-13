//! Effect primitives — the generic, actor-agnostic things a *technique* emits.
//!
//! A technique (a boss special, a wielded ability, an authored hazard) decides
//! *what and when*; an **effect** is the consequence it spawns. Effects are
//! faction-tagged and emitter-relative, so the player, an enemy, and a boss all
//! drive the same execution path — the actor-generic `ShockwaveSlam` is the
//! reference shape. Keeping the spawn here, instead of hand-rolled at each call
//! site, is what lets this logic eventually lift into a standalone
//! `ambition_effects` crate, and what makes "comment out effects → nothing
//! spawns, the rest of the game still runs" a real property.
//!
//! First primitive: [`DamageBox`] (a world-anchored damage volume) — the shared
//! shape behind boss hazards, AOE slams, beams, and death blasts. Damage
//! *resolution* stays in the combat layer (`apply_hitbox_damage`); an effect
//! only describes and spawns the box. The `FollowOwner` melee case keeps its
//! own `mechanics::combat::hitbox::spawn_melee_hitbox`.

use bevy::prelude::*;

use crate::engine_core as ae;
use crate::features::{ActorFaction, Hitbox, HitboxAnchor, HitboxHits, HitboxLifetime};

/// The `DamageBox` effect primitive: a world-anchored, time-limited damage
/// volume. `owner` + `source` faction are supplied at spawn (from the emitter),
/// so one shape serves player AOEs, boss hazards, and enemy death blasts.
pub struct DamageBox {
    pub half_extent: ae::Vec2,
    pub damage: i32,
    pub knockback: f32,
    /// Final lifetime in seconds — callers pass the already-clamped value (the
    /// helper does not re-clamp, to stay byte-identical to the old call sites).
    pub lifetime_s: f32,
    /// Optional inspector/debug name. `None` matches the call sites that spawn
    /// no `Name`; kept exact so the spawned archetype — and replay — is
    /// unchanged.
    pub name: Option<&'static str>,
}

/// Spawn a world-anchored [`DamageBox`] at `center`, owned by `owner` and tagged
/// with `source` faction. Returns the entity so callers that track the box (the
/// rotating-cross arms) can despawn it later.
///
/// The single spawn point for world-anchored damage boxes — consolidates five
/// formerly hand-rolled `(Hitbox, HitboxLifetime, HitboxHits[, Name])` sites.
pub fn spawn_damage_box(
    commands: &mut Commands,
    owner: Entity,
    source: ActorFaction,
    center: ae::Vec2,
    dbox: DamageBox,
) -> Entity {
    let mut e = commands.spawn((
        Hitbox {
            owner,
            source,
            anchor: HitboxAnchor::World { center },
            half_extent: dbox.half_extent,
            damage: dbox.damage,
            knockback_strength: dbox.knockback,
        },
        HitboxLifetime {
            remaining_s: dbox.lifetime_s,
        },
        HitboxHits::default(),
    ));
    if let Some(name) = dbox.name {
        e.insert(Name::new(name));
    }
    e.id()
}
