//! A move authored from OUTSIDE the engine, by a crate that cannot build a game.
//!
//! ⭐⭐ **THE POINT IS THE DEPENDENCY LIST, NOT THE MOVE.** Fast-iteration packet
//! I1 asks whether authoring a move is a pure value computation. Before it, the
//! primitives lived in `ambition_characters`, which links Bevy, so the answer was
//! no by construction — and several demos reached them through the
//! `ambition_platformer2d` umbrella, which links the whole engine. This crate
//! authors a real multi-window move with a technique reference against
//! `ambition_entity_catalog` alone.
//!
//! ⛔ **A WORKSPACE MEMBER COULD NOT MAKE THIS CLAIM.** Cargo unifies features
//! and shares one lockfile across a workspace, so a member's `cargo tree` shows
//! whatever the union resolved and "I do not need Bevy" is unfalsifiable there.
//! This crate declares its own `[workspace]` and resolves independently, which is
//! what lets `the_authoring_closure_has_no_engine_in_it` be a real measurement.
//!
//! ⚠ **AND THE MOVE HAS TO BE A REAL ONE.** A fixture that built a `MoveSpec`
//! literal would compile against this closure while proving nothing about the
//! BUILDERS — the thing I1 moved. So it goes through `strike` (which authors the
//! startup/active/recovery timeline), `on_hit` (the technique reference) and
//! `charge` (a second window shape), and asserts the emitted values.

use ambition_entity_catalog::authoring::{charge, on_hit, strike, Charge, Strike};
use ambition_entity_catalog::{MoveSpec, WindowTag};

/// The technique a landed hit asks the ruleset for. A KEY, never a handler — the
/// authoring crate cannot see a handler and must not need to.
pub const REBOUND_TECHNIQUE: &str = "pogo_bounce";

/// One authored move: a chargeable smash that offers a rebound where it lands.
pub fn a_chargeable_smash() -> MoveSpec {
    let m = strike(Strike {
        id: "outside_smash_forward",
        clip: "smash_forward",
        startup_s: 0.20,
        active_s: 0.08,
        recover_s: 0.30,
        offset: (18.0, 0.0),
        half_extents: (20.0, 14.0),
        damage: 14,
        knockback: 120.0,
        knockback_growth: 0.8,
        launch_dir: None,
        on_hit: None,
    });
    let m = on_hit(m, REBOUND_TECHNIQUE);
    charge(
        m,
        Charge {
            hold_at_s: 0.10,
            max_hold_s: 1.0,
            // Committed, not banked — the field with no default, because it is
            // the one that most changes what a fighter IS.
            stores: false,
            roots: true,
            sustain: ambition_entity_catalog::ChargeSustain::WhileHeld,
            gesture: ambition_entity_catalog::ChargeGesture::Smash,
            multiplier: 1.4,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⭐ THE BUILDERS RAN, not a literal. Each assertion names a field only a
    /// builder sets, so a fixture that quietly stopped calling them would fail
    /// rather than pass with a hand-written spec.
    #[test]
    fn an_outside_author_emits_a_real_multi_window_move() {
        let m = a_chargeable_smash();
        assert_eq!(m.id, "outside_smash_forward");

        let active: Vec<_> = m
            .windows
            .iter()
            .filter(|w| w.tag == WindowTag::Active)
            .collect();
        assert_eq!(active.len(), 1, "`strike` authors exactly one Active window");
        assert!(
            m.windows.len() >= 2,
            "a strike is startup/active/recovery, so one window means the \
             timeline builder did not run: {:?}",
            m.windows.iter().map(|w| w.tag.clone()).collect::<Vec<_>>()
        );

        let volume = active[0]
            .volumes
            .first()
            .expect("the Active window carries the swing's volume");
        assert_eq!(volume.damage, 14);
        assert_eq!(
            volume.on_hit.as_ref().map(|e| e.key.as_str()),
            Some(REBOUND_TECHNIQUE),
            "the technique reference did not reach the volume, so `on_hit` is \
             not what authored this"
        );

        let charge = m
            .smash_charge
            .as_ref()
            .expect("`charge` authors the hold, and nothing else here can");
        assert_eq!(charge.max_hold_s, 1.0);
    }

    /// ⛔ THE ANTI-VACUITY ARM. If the builders ever emitted an empty timeline the
    /// assertions above would still describe it in prose; this one refuses a move
    /// that spends time doing nothing.
    #[test]
    fn the_authored_move_occupies_real_time() {
        let m = a_chargeable_smash();
        assert!(
            m.duration_s > 0.5,
            "the authored smash lasts {}s, which is shorter than its own \
             startup — the timeline builder did not run",
            m.duration_s
        );
    }
}
