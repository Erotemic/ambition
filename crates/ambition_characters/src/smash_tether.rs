//! Reel the fighter to a ledge she threw a tether at: the authored vocabulary.
//!
//! ⭐⭐ THIS TECHNIQUE DOES NOT CATCH THE LEDGE, AND THAT IS THE WHOLE DESIGN.
//! Ledge grabbing is already an engine authority: the movement kernel calls
//! `ledge_grab::try_start_ledge_grab_clusters_in_frame` every frame, which
//! probes with `probe_ledge_grab_in_frame` and even auto-snaps a falling body
//! Smash-style. A `LedgeContact` carries an `anchor` documented as *"world
//! position the player should snap to (their center while hanging)"*.
//! ⇒ So a tether's job is to DELIVER HER TO THE ANCHOR. The authority then
//! catches her on its own terms, from her real position, with its own rules
//! about cooldowns and eligibility — none of which this technique may know.
//!
//! ⛔ THE RULE THIS OBEYS: a complex move may coordinate many authorities but
//! must not become the authority for their state. A tether that put her into
//! `LedgeHang` itself would own ledge state, and every ledge rule written since
//! (trumping, release cooldown, getup) would have a second implementation that
//! nobody updates.
//!
//! ⚠ SO WHAT IS AUTHORED HERE IS ONLY THE REEL: how far the line can find a
//! ledge, how fast it pulls, and how long it may pull before giving up.
//!
//! ⭐ THE CONTRAST WITH `smash_homing` IS DELIBERATE AND WORTH KEEPING. A homing
//! dash RE-ASKS its target every tick, so a foe who moves out of the cone stops
//! attracting it — that is what makes it a read. A tether LATCHES its anchor at
//! the moment the line bites, because a ledge does not move and a line that
//! re-aimed itself mid-reel would not be a line.
//!
//! The ruleset half is `ambition_demo_smash::tether`.

use serde::{Deserialize, Serialize};

use ambition_entity_catalog::{EffectRef, MoveEvent, MoveEventKind, MoveSpec, ParamValue};

/// The authored effect key.
pub const TETHER_PULL: &str = "smash.tether_pull";

/// Authored parameters of one tether reel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TetherPullParams {
    /// How far from the fighter the line may find a ledge, in world px.
    ///
    /// ⭐ AUTHOR THIS AS THE GRAB'S OWN REACH. The tether recovery and the
    /// tether grab are one fiction, and a line that reaches further off-stage
    /// than it does on-stage is two moves wearing one animation.
    pub reach: f32,
    /// How fast the reel carries her, in world px per second.
    pub speed: f32,
    /// How long the reel may last before it gives up, in seconds.
    ///
    /// ⚠ THIS IS A FAILSAFE, NOT THE DURATION. The reel normally ends by
    /// ARRIVING; the clock is what stops a fighter being carried forever if the
    /// ledge she latched stops being reachable (a moving platform, a portal
    /// carve, a stage that rebuilt itself under her).
    pub timeout_s: f32,
}

/// Author a tether reel onto a move's timeline.
///
/// # Panics
///
/// If `at_s` is past the move's duration; if any parameter is not positive; or
/// if the reel cannot cross its own reach before the clock expires.
pub fn author_tether_pull(mut spec: MoveSpec, at_s: f32, params: TetherPullParams) -> MoveSpec {
    assert!(
        at_s <= spec.duration_s,
        "move `{}` throws a tether at {at_s}s but only lasts {}s",
        spec.id,
        spec.duration_s,
    );
    assert!(
        params.reach > 0.0,
        "move `{}` authors a tether with {}px of reach, so the line can never \
         bite anything",
        spec.id,
        params.reach,
    );
    assert!(
        params.speed > 0.0,
        "move `{}` authors a tether that reels at {}px/s, so it latches a ledge \
         and then never arrives",
        spec.id,
        params.speed,
    );
    assert!(
        params.timeout_s > 0.0,
        "move `{}` authors a {}s reel, which expires on the frame it starts",
        spec.id,
        params.timeout_s,
    );
    // ⭐⭐ THE ONE ASSERT THAT CATCHES A REAL AUTHORING MISTAKE RATHER THAN A
    // TYPO. Each parameter above can be sane on its own while the THREE
    // together describe a move that always fails: a line that bites at its full
    // reach and then runs out of clock halfway there drops her into the blast
    // zone, which the author will read as a bug in the engine rather than as
    // arithmetic they own.
    assert!(
        params.speed * params.timeout_s >= params.reach,
        "move `{}` reels at {}px/s for {}s — {}px — but its line bites out to \
         {}px, so a tether thrown at full reach expires before it arrives",
        spec.id,
        params.speed,
        params.timeout_s,
        params.speed * params.timeout_s,
        params.reach,
    );
    spec.events.push(MoveEvent {
        at_s,
        kind: MoveEventKind::Effect(EffectRef {
            key: TETHER_PULL.to_string(),
            params: ParamValue::from_typed(&params).expect("tether-pull params serialize"),
        }),
    });
    spec
}
