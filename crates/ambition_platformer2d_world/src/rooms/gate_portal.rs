//! Gate portals — phase state machine + registry.
//!
//! Split out of the former 823-line `rooms/mod.rs` (2026-06-15); the
//! parent re-exports every type so `rooms::*` paths are unchanged.

use bevy_ecs::prelude::Resource;

/// Portal lifecycle phase. A portal's traversal readiness lives in
/// the *portal*, not in its controlling switch — the switch only
/// commands open/close; the portal runs the boot/shutdown sequence.
///
/// Sprite mapping (gate_portal_spritesheet rows):
/// - `Off`          → no portal sprite visible (only the ring)
/// - `Opening`      → opening animation (one-shot, ~0.64s)
/// - `On`           → stable animation (looping; traversal allowed)
/// - `Closing`      → closing animation (one-shot, ~0.64s)
///
/// Switch-flip behavior:
/// - off → on: Off→Opening, or Closing→Opening (resumes mid-close)
/// - on → off: On→Closing, or Opening→Closing (interrupts mid-open)
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum GatePortalPhase {
    #[default]
    Off,
    Opening {
        elapsed: f32,
    },
    On,
    Closing {
        elapsed: f32,
    },
}

impl GatePortalPhase {
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Opening { .. } => "opening",
            Self::On => "on",
            Self::Closing { .. } => "closing",
        }
    }

    pub fn portal_sprite_visible(self) -> bool {
        !matches!(self, Self::Off)
    }

    pub fn allows_traversal(self) -> bool {
        matches!(self, Self::On)
    }
}

/// One portal's AUTHORED configuration.
///
/// ⛔ **no live phase here.** This value is written once, by the content plugin
/// that authors the portal, and never again; the phase it used to carry is
/// integrated every simulated tick and lives in [`GatePortalPhases`]. See that
/// type for why the two cannot share a resource.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GatePortalConfig {
    /// The switch whose on/off state commands this portal's boot /
    /// shutdown sequence. Read from `save.data().switch(switch_id)`.
    pub switch_id: String,
    /// LDtk display name of the portal sprite entity (NpcSpawn
    /// name). The visibility system matches this against
    /// `FeatureName` to hide the portal sprite when phase == Off.
    pub portal_sprite_name: String,
    /// LDtk display name of the ring sprite entity. Used by the
    /// ring-spin visual flourish during `Opening`.
    pub ring_sprite_name: String,
}

/// Per-portal AUTHORED registry mapping `LoadingZone.id` → portal
/// configuration. `detect_room_transition_system` consults it before
/// recording a crossing: if the zone is a portal, traversal is allowed
/// only while the zone's phase (in [`GatePortalPhases`]) is `On`. Empty by
/// default — populated by story-content plugins.
///
/// Replaces the earlier `GatedZoneRegistry` (which only tracked
/// the switch and treated the zone as a thin switch-gate). The
/// portal's *own* state is what gates traversal — the switch just
/// drives the boot/shutdown sequence — so the readiness check lives
/// beside the portal, not in the switch system.
///
/// ⭐ **this resource is genuinely authored, and it was not before 2026-08-15.**
/// It carried each portal's live `phase` alongside the authored strings, which
/// made its rollback waiver ("authored gate portals") a wrong answer to the
/// checker's question — see [`GatePortalPhases`].
#[derive(Resource, Default, Debug, Clone, PartialEq, Eq)]
pub struct GatePortalRegistry {
    pub portals: std::collections::HashMap<String, GatePortalConfig>,
}

impl GatePortalRegistry {
    pub fn register(
        &mut self,
        zone_id: impl Into<String>,
        switch_id: impl Into<String>,
        portal_sprite_name: impl Into<String>,
        ring_sprite_name: impl Into<String>,
    ) {
        self.portals.insert(
            zone_id.into(),
            GatePortalConfig {
                switch_id: switch_id.into(),
                portal_sprite_name: portal_sprite_name.into(),
                ring_sprite_name: ring_sprite_name.into(),
            },
        );
    }

    pub fn is_portal(&self, zone_id: &str) -> bool {
        self.portals.contains_key(zone_id)
    }
}

/// **Every registered portal's LIVE phase — rollback state.**
///
/// `tick_portal_phases_system` integrates each phase forward by
/// `WorldTime::scaled_dt` in the SIM schedule (`GgrsSchedule` under the shipped
/// rollback host), and `detect_room_transition_system` — in the same schedule —
/// refuses a crossing unless the phase reads `On`. So this is per-frame
/// authoritative state, and the two facts together are what make it rewindable
/// state rather than a cache:
///
/// - the phase is a TIME INTEGRAL of the switch, not a function of it. The
///   switch lives in `AmbitionGameSave`, which IS rollback-registered, so a
///   rewind restores the input and — before this type existed — left the
///   integrator holding the speculative timeline's elapsed. `Opening` runs
///   [`PORTAL_OPENING_DURATION_SECS`] ≈ 38 ticks at 60 Hz, which is far wider
///   than any rollback depth: the window is not a corner case, it is the
///   ordinary case for a player who just flipped the switch.
/// - the phase decides a room transition. Two peers whose `elapsed` differ by
///   the depth of their last rollback promote `Opening → On` on different
///   frames, so one records a crossing the other refuses. That is a desync, not
///   a visual difference.
///
/// ⚠ **kept separate from [`GatePortalRegistry`] on purpose.** Registering the
/// merged resource would have put the AUTHORED half under the snapshot too, and
/// the content plugin that populates it runs in `Update` behind a one-shot
/// `installed` flag that does NOT rewind — so a rewind to a frame before the
/// populate would have restored an empty registry that nothing ever refills,
/// and `is_portal` would then answer `false` for a gate that exists. Authored
/// content and simulated state are different concerns; only the second one
/// rewinds.
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub struct GatePortalPhases {
    pub phases: std::collections::HashMap<String, GatePortalPhase>,
}

impl GatePortalPhases {
    /// A zone with no recorded phase is in the default phase (`Off`) — the same
    /// answer `register` used to seed, so a portal that has not ticked yet is
    /// shut rather than open.
    pub fn phase(&self, zone_id: &str) -> GatePortalPhase {
        self.phases.get(zone_id).copied().unwrap_or_default()
    }

    /// The phase slot for a zone, created in its default (`Off`) phase on first
    /// touch. This is the only write seam — the tick system owns it.
    pub fn phase_mut(&mut self, zone_id: &str) -> &mut GatePortalPhase {
        self.phases.entry(zone_id.to_owned()).or_default()
    }

    /// ⛔ **`false` for an unknown zone**, because the caller has already asked
    /// [`GatePortalRegistry::is_portal`]: a zone that IS a portal and has no
    /// phase yet has not booted, and an unbooted gate is shut.
    pub fn allows_traversal(&self, zone_id: &str) -> bool {
        self.phase(zone_id).allows_traversal()
    }
}

/// 8 frames × 80ms = 640ms. Mirrors the `opening` row duration in
/// `interdimensional_gate_portal_spritesheet.yaml`.
pub const PORTAL_OPENING_DURATION_SECS: f32 = 0.640;
/// Mirrors the `closing` row duration.
pub const PORTAL_CLOSING_DURATION_SECS: f32 = 0.640;

/// Advance a portal phase one tick. Pure function — exposed so a
/// system can call it without holding `&mut GatePortalConfig`.
pub fn tick_gate_portal_phase(phase: &mut GatePortalPhase, switch_on: bool, dt: f32) {
    match phase {
        GatePortalPhase::Off => {
            if switch_on {
                *phase = GatePortalPhase::Opening { elapsed: 0.0 };
            }
        }
        GatePortalPhase::Opening { elapsed } => {
            *elapsed += dt;
            if !switch_on {
                // Interrupted mid-open — start closing from the same
                // visual progress (so the player sees a smooth reverse,
                // not a snap back to fully-open).
                let opened_frac = (*elapsed / PORTAL_OPENING_DURATION_SECS).clamp(0.0, 1.0);
                *phase = GatePortalPhase::Closing {
                    elapsed: PORTAL_CLOSING_DURATION_SECS * (1.0 - opened_frac),
                };
            } else if *elapsed >= PORTAL_OPENING_DURATION_SECS {
                *phase = GatePortalPhase::On;
            }
        }
        GatePortalPhase::On => {
            if !switch_on {
                *phase = GatePortalPhase::Closing { elapsed: 0.0 };
            }
        }
        GatePortalPhase::Closing { elapsed } => {
            *elapsed += dt;
            if switch_on {
                let closed_frac = (*elapsed / PORTAL_CLOSING_DURATION_SECS).clamp(0.0, 1.0);
                *phase = GatePortalPhase::Opening {
                    elapsed: PORTAL_OPENING_DURATION_SECS * (1.0 - closed_frac),
                };
            } else if *elapsed >= PORTAL_CLOSING_DURATION_SECS {
                *phase = GatePortalPhase::Off;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    //! Gate-portal PHASE-transition unit tests. These pin the pure world-owned
    //! state machine (`tick_gate_portal_phase` + `GatePortalPhase`); they
    //! travelled here from `ambition_platformer2d_actor_monolith::world::rooms::tests` (fable audit
    //! F5.4 test-travel) so a gate-portal regression fails IN this crate.
    use super::*;

    #[test]
    fn portal_phase_default_is_off() {
        assert_eq!(GatePortalPhase::default(), GatePortalPhase::Off);
    }

    #[test]
    fn portal_phase_off_transitions_to_opening_when_switch_turns_on() {
        let mut phase = GatePortalPhase::Off;
        tick_gate_portal_phase(&mut phase, true, 0.01);
        assert!(matches!(phase, GatePortalPhase::Opening { .. }));
    }

    #[test]
    fn portal_phase_opening_completes_to_on_after_duration() {
        let mut phase = GatePortalPhase::Opening { elapsed: 0.0 };
        tick_gate_portal_phase(&mut phase, true, PORTAL_OPENING_DURATION_SECS + 0.01);
        assert_eq!(phase, GatePortalPhase::On);
    }

    #[test]
    fn portal_phase_on_transitions_to_closing_when_switch_turns_off() {
        let mut phase = GatePortalPhase::On;
        tick_gate_portal_phase(&mut phase, false, 0.01);
        assert!(matches!(phase, GatePortalPhase::Closing { .. }));
    }

    #[test]
    fn portal_phase_closing_completes_to_off_after_duration() {
        let mut phase = GatePortalPhase::Closing { elapsed: 0.0 };
        tick_gate_portal_phase(&mut phase, false, PORTAL_CLOSING_DURATION_SECS + 0.01);
        assert_eq!(phase, GatePortalPhase::Off);
    }

    #[test]
    fn portal_phase_mid_open_interruption_resumes_close_from_same_visual_progress() {
        // Half-open: opening at elapsed = 0.32s (50% of 0.64s).
        let mut phase = GatePortalPhase::Opening {
            elapsed: PORTAL_OPENING_DURATION_SECS * 0.5,
        };
        // Switch flips off mid-open.
        tick_gate_portal_phase(&mut phase, false, 0.0);
        // Should be closing with elapsed = 50% of closing duration (so the
        // remaining close time is half — symmetric with the open progress).
        if let GatePortalPhase::Closing { elapsed } = phase {
            let close_progress_remaining =
                (PORTAL_CLOSING_DURATION_SECS - elapsed) / PORTAL_CLOSING_DURATION_SECS;
            // Should be ~0.5 (half a close still to go, matching the
            // half-open visual we interrupted).
            assert!(
                (close_progress_remaining - 0.5).abs() < 0.01,
                "close-remaining should be ~0.5; got {close_progress_remaining}"
            );
        } else {
            panic!("expected Closing after interrupted Opening; got {phase:?}");
        }
    }

    #[test]
    fn portal_phase_only_on_allows_traversal() {
        assert!(!GatePortalPhase::Off.allows_traversal());
        assert!(!GatePortalPhase::Opening { elapsed: 0.0 }.allows_traversal());
        assert!(GatePortalPhase::On.allows_traversal());
        assert!(!GatePortalPhase::Closing { elapsed: 0.0 }.allows_traversal());
    }

    /// ⭐ **the phase is not recoverable from the switch** — which is the whole
    /// argument for [`GatePortalPhases`] being rollback state.
    ///
    /// Two timelines that agree exactly on the switch (on, the entire time) but
    /// disagree on how many ticks the portal has already been opening give
    /// DIFFERENT traversal verdicts on the same frame. A rewind that restored
    /// `AmbitionGameSave` (where the switch lives, and which IS registered) and
    /// left the phase alone reproduces exactly this disagreement — six ticks of
    /// divergence is well inside an ordinary rollback depth.
    ///
    /// ⚠ both terms are observed: the test fails if the ahead timeline is NOT
    /// traversable, and fails if the behind timeline IS.
    #[test]
    fn the_phase_is_not_a_function_of_the_switch_alone() {
        let dt = 1.0 / 60.0;
        let mut ahead = GatePortalPhase::Opening {
            elapsed: PORTAL_OPENING_DURATION_SECS - 2.0 * dt,
        };
        let mut behind = GatePortalPhase::Opening {
            elapsed: PORTAL_OPENING_DURATION_SECS - 8.0 * dt,
        };
        for _ in 0..3 {
            tick_gate_portal_phase(&mut ahead, true, dt);
            tick_gate_portal_phase(&mut behind, true, dt);
        }
        assert!(
            ahead.allows_traversal(),
            "the further-along timeline should have promoted to On; got {ahead:?}"
        );
        assert!(
            !behind.allows_traversal(),
            "the six-tick-behind timeline should still be Opening; got {behind:?}"
        );
    }

    /// The phase machine has no terminal state and reverses through the SAME
    /// visual progress, so "it will settle anyway" is not a defence: the two
    /// timelines above stay apart for as long as the switch keeps commanding
    /// the same thing, and they disagree about traversal the entire time.
    #[test]
    fn a_reversed_phase_keeps_the_divergence_rather_than_collapsing_it() {
        let dt = 1.0 / 60.0;
        let mut ahead = GatePortalPhase::Opening {
            elapsed: PORTAL_OPENING_DURATION_SECS * 0.75,
        };
        let mut behind = GatePortalPhase::Opening {
            elapsed: PORTAL_OPENING_DURATION_SECS * 0.25,
        };
        // The switch flips off: both reverse, mapping their progress into the
        // closing timer.
        tick_gate_portal_phase(&mut ahead, false, 0.0);
        tick_gate_portal_phase(&mut behind, false, 0.0);
        let (GatePortalPhase::Closing { elapsed: a }, GatePortalPhase::Closing { elapsed: b }) =
            (ahead, behind)
        else {
            panic!("both should be Closing; got {ahead:?} and {behind:?}");
        };
        assert!(
            (a - b).abs() > 0.1,
            "the reversal must PRESERVE the divergence, not erase it; got {a} vs {b}"
        );
    }

    #[test]
    fn portal_phase_portal_sprite_visible_only_when_not_off() {
        assert!(!GatePortalPhase::Off.portal_sprite_visible());
        assert!(GatePortalPhase::Opening { elapsed: 0.0 }.portal_sprite_visible());
        assert!(GatePortalPhase::On.portal_sprite_visible());
        assert!(GatePortalPhase::Closing { elapsed: 0.0 }.portal_sprite_visible());
    }
}
