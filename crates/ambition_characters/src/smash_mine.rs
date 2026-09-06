//! Place a mine the placer can set off from anywhere: the authored vocabulary.
//!
//! ⭐ THE SAME SPLIT `smash_bomb`, `smash_capture` AND `smash_portal` USE. A key
//! and its params are what a MOVESET authors; the object on the stage, the
//! arming clock and the blast are a RULESET's, and that half is
//! `ambition_demo_smash::mine`.
//!
//! ⭐⭐ JON'S ASSIGNMENT, 2026-09-05: *"projectile polygon get a tether, and
//! probably the remote mine as their down smash."*
//!
//! ⛔⛔ THE MINE IS A `GroundItem`, EXACTLY LIKE THE BOMB, and that is the whole
//! reason this technique is small. "A thing that sits on the stage, falls, and
//! can be picked up and thrown" is an authority this engine already has; a mine
//! that spawned its own body would have been a second answer to a question that
//! was already answered — the mistake this campaign's plan names first.
//! ⇒ The consequence is a FEATURE and is not a compromise: an opponent can pick
//! your mine up, and it is still yours to detonate while they hold it.
//!
//! ⭐ WHAT MAKES IT A MINE RATHER THAN A BOMB IS THE TRIGGER, and only that. The
//! bomb answers to a fuse and to being thrown hard; the mine answers to nobody
//! but the fighter who placed it. There is no countdown to read and no contact
//! threshold to bait — which is why it is worth having both.

use serde::{Deserialize, Serialize};

use ambition_entity_catalog::{EffectRef, MoveEvent, MoveEventKind, MoveSpec, ParamValue};

/// The authored effect key. Namespaced like every other smash technique so an
/// unrecognised key falls through other rulesets untouched.
pub const PLACE_MINE: &str = "smash.place_mine";

/// Authored parameters of one placed mine.
///
/// ⛔ THERE IS NO `fuse_s` AND ITS ABSENCE IS THE DESIGN. A mine that expired on
/// its own would be a slow bomb, and the choice of WHEN is the only thing the
/// move is really selling.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlaceMineParams {
    /// The held-item id this object becomes in somebody's hands. It must be a
    /// registered held item, for the same reason the bomb's must be: without one
    /// the object cannot be picked up, and half of what a stage object IS is
    /// that somebody else can come and take it.
    pub item_id: String,
    /// Seconds before the mine will answer its owner.
    ///
    /// ⭐⭐ THE ARMING DELAY IS THE MOVE'S ONLY BRAKE. Placement and detonation
    /// are the same press, so without this a fighter could place and detonate on
    /// consecutive frames and the mine would be an ordinary disjointed hitbox
    /// with extra steps. The delay is what makes placing it a COMMITMENT.
    pub arm_s: f32,
    /// Damage at the centre of the blast.
    pub damage: i32,
    /// How far the blast reaches, in world px.
    pub blast_radius: f32,
    /// The object's own size in the world.
    pub half_extents: (f32, f32),
    /// Where it appears, body-local (`+x` toward facing, `+y` gravity-down).
    pub offset: (f32, f32),
}

/// Author a mine placement onto a move's timeline.
///
/// ⚠ ONE EVENT AUTHORS BOTH HALVES OF THE MOVE. Pressing this move while a mine
/// of yours is already armed detonates that mine and places nothing; the
/// ruleset decides which of the two happened, because only it can see the stage.
/// ⇒ There is deliberately no `smash.detonate_mine` key to author beside this
/// one: a second key would let a moveset offer detonation without placement,
/// and a fighter who can detonate mines they cannot place is not a design
/// anybody asked for.
///
/// # Panics
///
/// If `at_s` is past the move's own duration. A mine scheduled after the move
/// ends never appears, and the move would spend its recovery to do nothing.
pub fn author_place_mine(mut spec: MoveSpec, at_s: f32, params: PlaceMineParams) -> MoveSpec {
    assert!(
        at_s <= spec.duration_s,
        "move `{}` places its mine at {at_s}s but only lasts {}s, so the mine \
         would never appear and the move would spend a recovery to do nothing",
        spec.id,
        spec.duration_s,
    );
    spec.events.push(MoveEvent {
        at_s,
        kind: MoveEventKind::Effect(EffectRef {
            key: PLACE_MINE.to_string(),
            params: ParamValue::from_typed(&params).expect("place-mine params serialize"),
        }),
    });
    spec
}
