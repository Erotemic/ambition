//! Leave a plate on the stage that throws whoever steps on it.
//!
//! ⭐ THE SAME SPLIT `smash_mine`, `smash_bomb` AND `smash_portal` USE. A key and
//! its params are what a MOVESET authors; the object, its clock and what it does
//! to a body are a RULESET's, and that half is `ambition_demo_smash::spring`.
//!
//! ⭐⭐ IT IS THE CAMPAIGN'S "reusable launch object": *a fighter can create a
//! persistent world actuator another fighter interacts with*. ⛔ And the
//! interesting word is ANOTHER — a spring that only served its owner would be a
//! second recovery with extra steps. This one throws whoever touches it, which is
//! what makes it a piece of STAGE rather than a piece of kit.
//!
//! ⛔ NO OWNER IS RECORDED, and that is the design rather than an omission — the
//! same ruling `LiveBomb` makes about itself. A plate on the floor belongs to
//! whoever is standing on it; "whose spring is this" has no answer anybody would
//! act on, and keeping an `Entity` out of rollback state costs nothing here.

use serde::{Deserialize, Serialize};

use ambition_entity_catalog::{EffectRef, MoveEvent, MoveEventKind, MoveSpec, ParamValue};

/// The authored effect key. Namespaced like every other smash technique.
pub const PLACE_SPRING: &str = "smash.place_spring";

/// Authored parameters of one placed spring.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlaceSpringParams {
    /// How hard it throws, in world px per second. UPWARD is negative `y`, and
    /// the launch is authored as a vector so a plate can be angled.
    pub launch: (f32, f32),
    /// The plate's size on the floor.
    pub half_extents: (f32, f32),
    /// Seconds before it is taken away.
    ///
    /// ⚠ A SPRING WITH NO CLOCK IS STAGE GEOMETRY. The lifetime is what keeps
    /// this a MOVE — an actuator that outlived the match would be terrain a
    /// fighter authored, and terrain is somebody else's authority.
    pub lifetime_s: f32,
    /// How many launches it has in it before it is spent.
    ///
    /// ⭐ SEPARATE FROM THE CLOCK ON PURPOSE. "It lasts eight seconds" and "it
    /// works three times" are different limits and a move may want either — a
    /// one-shot plate that sits until used is a trap, and a many-use plate that
    /// expires is a platform.
    pub uses: u8,
    /// Where it lands, body-local (`+x` toward facing, `+y` gravity-down).
    pub offset: (f32, f32),
    /// The cosmetic row drawn when it is PLACED and again when it FIRES.
    /// `None` draws nothing, which is what every plate authored before this
    /// field existed did.
    ///
    /// ⛔⛔ A PLATE NOBODY CAN SEE IS NOT A MOVE, IT IS AN AMBUSH. Measured
    /// 2026-09-05: `PlacedSpring` draws NOTHING — no sprite, no effect, no cue —
    /// while the remote mine is visible for free because it is a `GroundItem`
    /// and `item_visuals` gives those a sprite. ⇒ Two objects a fighter puts on
    /// the floor, one readable and one invisible, and only the invisible one
    /// launches you.
    ///
    /// ⚠ THIS IS THE ANNOUNCEMENT HALF ONLY, AND SAYING SO IS THE POINT. A cue
    /// at placement and at fire means the other player SEES it happen; it does
    /// not make the plate persistently visible in between. The shipped road for
    /// that is the mine's — a `GroundItem` with authored art — and it is a
    /// content decision rather than a field.
    ///
    /// ⛔⛔ REQUIRED, AND IT WAS `Option<String>` WITH `#[serde(default)]` FOR
    /// ONE COMMIT. A peer caught the shape in a sentence I had written myself:
    /// *"`None` draws nothing, which is what every plate authored before this
    /// field existed did"* — ⇒ **the DEFAULT value of the new field was exactly
    /// the invisible-ambush state the field exists to end.** Both shipped authors
    /// set it, and nothing made a third do so; worse, `serde(default)` meant a
    /// plate arriving by deserialization was silent without anyone typing
    /// anything. ⭐ Non-optional and asserted non-empty turns *"an author
    /// remembered"* into *"an author could not omit it"*, which is the same move
    /// as gating the gravity modifier on its timer and deriving `overlapped`
    /// instead of mirroring it.
    pub vfx: String,
}

/// Author a spring placement onto a move's timeline.
///
/// # Panics
///
/// If `at_s` is past the move's own duration — the plate would never appear. If
/// `uses` is zero, because a spring nobody can use is an invisible object that
/// costs a move its recovery. And if the launch is zero-length, for the same
/// reason: there is no frame at which a player could see that it had failed.
pub fn author_place_spring(mut spec: MoveSpec, at_s: f32, params: PlaceSpringParams) -> MoveSpec {
    assert!(
        at_s <= spec.duration_s,
        "move `{}` drops its plate at {at_s}s but only lasts {}s, so the plate \
         would never appear and the move would spend a recovery to do nothing",
        spec.id,
        spec.duration_s,
    );
    // ⛔ AN UNANNOUNCED PLATE IS AN AMBUSH, NOT A MOVE. The invisible-object
    // assertion below refuses a plate with no USES for the same reason; this
    // refuses one nobody can see arrive.
    assert!(
        !params.vfx.trim().is_empty(),
        "move `{}` drops a plate that announces nothing — `PlacedSpring` draws no \
         sprite of its own, so a plate with no cue is an object the other player \
         is never told about",
        spec.id,
    );
    assert!(
        params.uses > 0,
        "move `{}` drops a plate with no uses left, which is an invisible object \
         that costs a recovery",
        spec.id,
    );
    let (lx, ly) = params.launch;
    assert!(
        lx.abs() + ly.abs() > 0.0,
        "move `{}` drops a plate that throws nobody anywhere",
        spec.id,
    );
    spec.events.push(MoveEvent {
        at_s,
        kind: MoveEventKind::Effect(EffectRef {
            key: PLACE_SPRING.to_string(),
            params: ParamValue::from_typed(&params).expect("place-spring params serialize"),
        }),
    });
    spec
}
