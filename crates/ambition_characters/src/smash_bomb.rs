//! Put a live bomb on the stage: the authored vocabulary.
//!
//! ⭐ THE SAME SPLIT `smash_capture`, `smash_ride` AND `smash_teleport` USE. A
//! key and its params are what a MOVESET authors; the fuse, the blast and the
//! object somebody can pick up are a RULESET's, and that half is
//! `ambition_demo_smash::bomb`.
//!
//! ⭐⭐ JON'S DESIGN, 2026-08-27: *"The projectile polygon should poop a bomb
//! onto the stage, they should be able to pick it up and throw it. The bomb
//! should detonate in 4 seconds or if it hits something with enough velocity,
//! whichever comes first."*
//!
//! ⛔ THE OBJECT IS A GROUND ITEM, not a summon, and that is what makes the
//! second sentence free. Picking things up and throwing them is machinery this
//! engine already has — `GroundItem`, `ItemCustody`, `throw_held_item_system` —
//! and a bomb that was a summoned body would have needed all of it written
//! again in order to be a thing you can hold.

use serde::{Deserialize, Serialize};

use ambition_entity_catalog::{EffectRef, MoveEvent, MoveEventKind, MoveSpec, ParamValue};

/// The authored effect key. Namespaced like every other smash technique so an
/// unrecognised key falls through other rulesets untouched.
pub const DROP_BOMB: &str = "smash.drop_bomb";

/// Authored parameters of one dropped bomb.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DropBombParams {
    /// The held-item id this object becomes in somebody's hands. It must be a
    /// registered held item or nobody can pick the bomb up — which is half the
    /// move.
    pub item_id: String,
    /// Seconds until it goes off by itself.
    pub fuse_s: f32,
    /// Damage at the centre of the blast.
    pub damage: i32,
    /// How far the blast reaches, in world px.
    pub blast_radius: f32,
    /// ⭐⭐ HOW HARD IS HARD ENOUGH. Jon: *"or if it hits something with enough
    /// velocity, whichever comes first."* Below this speed the bomb bounces and
    /// keeps its fuse; at or above it, contact is the detonation.
    ///
    /// ⛔ A THRESHOLD, NOT A FLAG, because both outcomes have to be reachable:
    /// a bomb that always went off on contact could never be placed, and one
    /// that never did would make the thrown bomb identical to the dropped one.
    pub impact_speed: f32,
    /// The object's own size in the world.
    pub half_extents: (f32, f32),
    /// Where it appears, body-local (`+x` toward facing, `+y` gravity-down).
    pub offset: (f32, f32),
}

/// Author a bomb drop onto a move's timeline.
///
/// # Panics
///
/// If `at_s` is past the move's own duration. A bomb scheduled after the move
/// ends never appears, and the move would spend its recovery to do nothing.
pub fn author_drop_bomb(mut spec: MoveSpec, at_s: f32, params: DropBombParams) -> MoveSpec {
    assert!(
        at_s <= spec.duration_s,
        "move `{}` drops its bomb at {at_s}s but only lasts {}s, so the bomb \
         would never appear and the move would spend a recovery to do nothing",
        spec.id,
        spec.duration_s,
    );
    spec.events.push(MoveEvent {
        at_s,
        kind: MoveEventKind::Effect(EffectRef {
            key: DROP_BOMB.to_string(),
            params: ParamValue::from_typed(&params).expect("drop-bomb params serialize"),
        }),
    });
    spec
}
