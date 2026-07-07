//! Gate portals — phase state machine + registry.
//!
//! Split out of the former 823-line `rooms/mod.rs` (2026-06-15); the
//! parent re-exports every type so `rooms::*` paths are unchanged.

use super::*;

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

/// One portal's configuration + live phase.
#[derive(Clone, Debug)]
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
    pub phase: GatePortalPhase,
}

/// Per-portal registry mapping `LoadingZone.id` → portal lifecycle.
/// `detect_room_transition_system` consults the registry before
/// writing a `RoomTransitionRequested`: if the zone is a portal,
/// traversal is allowed only while `phase == On`. Empty by default
/// — populated by story-content plugins.
///
/// Replaces the earlier `GatedZoneRegistry` (which only tracked
/// the switch and treated the zone as a thin switch-gate). The
/// portal's *own* state is what gates traversal — the switch just
/// drives the boot/shutdown sequence — so the readiness check lives
/// here, not in the switch system.
#[derive(Resource, Default, Debug, Clone)]
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
                phase: GatePortalPhase::default(),
            },
        );
    }

    pub fn phase(&self, zone_id: &str) -> GatePortalPhase {
        self.portals
            .get(zone_id)
            .map(|c| c.phase)
            // A zone with no recorded portal state is in the default phase.
            .unwrap_or_default()
    }

    pub fn is_portal(&self, zone_id: &str) -> bool {
        self.portals.contains_key(zone_id)
    }

    pub fn allows_traversal(&self, zone_id: &str) -> bool {
        self.portals
            .get(zone_id)
            .map(|c| c.phase.allows_traversal())
            .unwrap_or(true)
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
