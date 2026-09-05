//! Gate portals — phase state machine + registry.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;
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

/// One portal's authored configuration. Live phase is integrated separately in
/// [`GatePortalPhases`].
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
/// The portal's own live phase gates traversal; the switch only commands the
/// boot/shutdown sequence. Authored configuration therefore stays here while
/// live rollback state lives in [`GatePortalPhases`].
#[derive(Resource, Default, Debug, Clone, PartialEq, Eq)]
pub struct GatePortalRegistry {
    /// ⛔ SEALED, and that is the point of the refusal below. While this was
    /// `pub` any crate could `portals.insert(..)` and take a zone from whoever
    /// held it, so `register` refusing a conflict would have been advice rather
    /// than a rule. A registry that validates in one function and leaves its map
    /// open has one authority for the checked road and none for the other.
    /// ⚠ `BTreeMap`, matching [`GatePortalPhases`] next door and for its stated
    /// reason: deterministic key order for every reader. It was a `HashMap`, and
    /// SEALING THE FIELD is what exposed that — the iteration used to happen in
    /// `ambition_render` and `actor_monolith` through the public map, where the
    /// workspace determinism policy did not look. Moving the loop into the
    /// owning crate made a latent rule live, which is the policy working rather
    /// than the policy being in the way.
    portals: std::collections::BTreeMap<String, GatePortalConfig>,
}

/// A second, DIFFERENT portal claiming a zone that already has one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GatePortalConflict {
    pub zone_id: String,
    pub existing: GatePortalConfig,
    pub incoming: GatePortalConfig,
}

impl std::fmt::Display for GatePortalConflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "loading zone `{}` already has a gate portal commanded by switch \
             `{}`; a second registration wants switch `{}`. One zone is one \
             portal — rename the zone or reuse the existing registration.",
            self.zone_id, self.existing.switch_id, self.incoming.switch_id
        )
    }
}

impl GatePortalRegistry {
    /// Claim a loading zone for a portal, REFUSING a conflicting second claim.
    ///
    /// ⛔⛔ THIS USED TO BE A BARE `insert`, one of seven registries in the
    /// 2026-09-02 inventory whose second registration silently overwrote. The
    /// inventory's ruling is that a silent overwrite must not be anyone's
    /// accidental default: each of the seven has to say "replace" in place or
    /// adopt refusal. ⇒ This one refuses, because a loading zone is a place and
    /// two portals cannot both be there. Re-registering the SAME portal is
    /// idempotent, so a plugin whose install runs twice is not an error.
    ///
    /// ⚠ PREVENTIVE, not a repair: MEASURED 2026-09-05, production has exactly
    /// ONE caller (`ambition_content`'s intro portal, itself latch-guarded), so
    /// no conflict is reachable today. What it buys is that the second portal —
    /// which the open-world roadmap wants — cannot silently unseat the first.
    ///
    /// `ambition_registry_core::classify` decides the three cases so this
    /// registry does not re-answer a question `PlacementLoweringRegistry` next
    /// door already answers.
    pub fn try_register(
        &mut self,
        zone_id: impl Into<String>,
        switch_id: impl Into<String>,
        portal_sprite_name: impl Into<String>,
        ring_sprite_name: impl Into<String>,
    ) -> Result<ambition_registry_core::RegistrationOutcome, GatePortalConflict> {
        let zone_id = zone_id.into();
        let incoming = GatePortalConfig {
            switch_id: switch_id.into(),
            portal_sprite_name: portal_sprite_name.into(),
            ring_sprite_name: ring_sprite_name.into(),
        };
        match ambition_registry_core::classify(self.portals.get(&zone_id), &incoming) {
            ambition_registry_core::Classification::Idempotent => {
                Ok(ambition_registry_core::RegistrationOutcome::Idempotent)
            }
            ambition_registry_core::Classification::Conflict { existing } => {
                Err(GatePortalConflict {
                    zone_id,
                    existing: existing.clone(),
                    incoming,
                })
            }
            ambition_registry_core::Classification::New => {
                self.portals.insert(zone_id, incoming);
                Ok(ambition_registry_core::RegistrationOutcome::Inserted)
            }
        }
    }

    pub fn is_portal(&self, zone_id: &str) -> bool {
        self.portals.contains_key(zone_id)
    }

    /// Every registered portal, for the tick and the visuals.
    ///
    /// ⚠ Key order, deterministically — the map is a `BTreeMap` for that reason.
    /// Callers should still not DEPEND on the order (`world/rooms/systems.rs`
    /// states why its tick is indifferent to it); what determinism buys is that
    /// two runs of the same content iterate identically.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &GatePortalConfig)> {
        self.portals.iter()
    }
}

/// Every registered portal's live rollback phase. The phase is integrated in
/// the simulation schedule and gates traversal, so its elapsed timer is
/// authoritative rollback state rather than a derived cache.
///
/// Kept separate from [`GatePortalRegistry`]: authored portal configuration is
/// not simulated state and must not be restored from rollback snapshots.
/// `BTreeMap` provides deterministic key-order iteration for checksums/readers.
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub struct GatePortalPhases {
    pub phases: std::collections::BTreeMap<String, GatePortalPhase>,
}

impl GatePortalPhases {
    /// A zone with no recorded phase defaults to `Off`, so an unticked portal
    /// is shut rather than open.
    pub fn phase(&self, zone_id: &str) -> GatePortalPhase {
        self.phases.get(zone_id).copied().unwrap_or_default()
    }

    /// The phase slot for a zone, created in its default (`Off`) phase on first
    /// touch. This is the only write seam — the tick system owns it.
    pub fn phase_mut(&mut self, zone_id: &str) -> &mut GatePortalPhase {
        self.phases.entry(zone_id.to_owned()).or_default()
    }

    ///  `false` for an unknown zone, because the caller has already asked
    /// [`GatePortalRegistry::is_portal`]: a zone that IS a portal and has no
    /// phase yet has not booted, and an unbooted gate is shut.
    pub fn allows_traversal(&self, zone_id: &str) -> bool {
        self.phase(zone_id).allows_traversal()
    }
}

/// The value projection behind the `resource.gate_portal_phases` rollback
/// registration — the checksum a GGRS host folds into its per-frame desync
/// detector, and the census a restore audit compares across a rewind.
///
/// Every field that decides when `Opening` becomes `On` is projected here.
///
/// Keys are folded in `BTreeMap` order so identical phase state produces the
/// same checksum on every peer. The projection lives with the domain type so an
/// added [`GatePortalPhase`] variant must update the exhaustive match here.
pub fn gate_portal_phases_checksum(phases: &GatePortalPhases) -> u64 {
    use ambition_platformer2d_core::snapshot::{checksum_bytes, put_f32, put_str, put_u64, put_u8};

    let mut bytes = Vec::new();
    put_u64(&mut bytes, phases.phases.len() as u64);
    for (zone_id, phase) in &phases.phases {
        put_str(&mut bytes, zone_id);
        match phase {
            GatePortalPhase::Off => put_u8(&mut bytes, 0),
            GatePortalPhase::Opening { elapsed } => {
                put_u8(&mut bytes, 1);
                put_f32(&mut bytes, *elapsed);
            }
            GatePortalPhase::On => put_u8(&mut bytes, 2),
            GatePortalPhase::Closing { elapsed } => {
                put_u8(&mut bytes, 3);
                put_f32(&mut bytes, *elapsed);
            }
        }
    }
    checksum_bytes(&bytes)
}

/// Owner label for this domain's rollback registrations. It is organizational,
/// not part of the schema fingerprint, and is reused by required-state checks.
pub const GATE_PORTAL_ROLLBACK_OWNER: &str = "ambition_platformer2d_world";

/// Install gate-portal rollback state through the domain-neutral
/// [`RollbackRegistrar`]. Registration uses a value projection so authoritative
/// phase timers participate in restore/desync checks. The registrar is generic
/// because its registration methods are not object-safe.
pub fn register_gate_portal_rollback_state<R>(registrar: &mut R)
where
    R: RollbackRegistrar,
{
    registrar.rollback_resource_clone_checksum::<GatePortalPhases>(
        GATE_PORTAL_ROLLBACK_OWNER,
        "resource.gate_portal_phases",
        "key-ordered phase/elapsed checksum projection",
        gate_portal_phases_checksum,
    );
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
    //! Gate-portal phase-transition tests for the world-owned state machine.
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

    ///  the phase is not recoverable from the switch — which is the whole
    /// argument for [`GatePortalPhases`] being rollback state.
    ///
    /// Two timelines that agree exactly on the switch (on, the entire time) but
    /// disagree on how many ticks the portal has already been opening give
    /// DIFFERENT traversal verdicts on the same frame. A rewind that restored
    /// `AmbitionGameSave` (where the switch lives, and which IS registered) and
    /// left the phase alone reproduces exactly this disagreement — six ticks of
    /// divergence is well inside an ordinary rollback depth.
    ///
    ///  both terms are observed: the test fails if the ahead timeline is NOT
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

    fn phases_of(entries: &[(&str, GatePortalPhase)]) -> GatePortalPhases {
        let mut phases = GatePortalPhases::default();
        for (zone_id, phase) in entries {
            *phases.phase_mut(zone_id) = *phase;
        }
        phases
    }

    ///  a presence-only projection would agree with the bug. The registration
    /// this backs exists because an `elapsed` timer ran ahead of the switch that
    /// drove it, and every zone stayed present the whole time.
    ///
    ///  both terms are observed: identical states must AGREE, and a one-tick
    /// difference in `elapsed` — plus a variant change carrying no payload at all
    /// — must DISAGREE. A projection that hashed only the key set passes the
    /// first assertion and fails the rest.
    #[test]
    fn the_phase_projection_sees_the_elapsed_timer_and_the_variant() {
        let dt = 1.0 / 60.0;
        let ahead = phases_of(&[("gate.a", GatePortalPhase::Opening { elapsed: 4.0 * dt })]);
        let behind = phases_of(&[("gate.a", GatePortalPhase::Opening { elapsed: 3.0 * dt })]);
        let twin = phases_of(&[("gate.a", GatePortalPhase::Opening { elapsed: 4.0 * dt })]);

        assert_eq!(
            gate_portal_phases_checksum(&ahead),
            gate_portal_phases_checksum(&twin),
            "identical phase states must project equal — otherwise the detector \
             manufactures desyncs"
        );
        assert_ne!(
            gate_portal_phases_checksum(&ahead),
            gate_portal_phases_checksum(&behind),
            "one tick of divergence in `elapsed` must be visible; that divergence \
             is exactly the defect the registration closes"
        );

        let off = phases_of(&[("gate.a", GatePortalPhase::Off)]);
        let on = phases_of(&[("gate.a", GatePortalPhase::On)]);
        assert_ne!(
            gate_portal_phases_checksum(&off),
            gate_portal_phases_checksum(&on),
            "the payload-free variants decide traversal, so the tag must be projected"
        );
    }

    /// The projection and its container must use key order so peers with
    /// identical state produce identical checksums regardless of insertion path.
    /// This test checks both the checksum and container iteration order.
    #[test]
    fn the_phase_projection_folds_in_key_order_whatever_the_insertion_order() {
        let entries: Vec<(&str, GatePortalPhase)> = vec![
            ("gate.alpha", GatePortalPhase::On),
            ("gate.bravo", GatePortalPhase::Opening { elapsed: 0.1 }),
            ("gate.charlie", GatePortalPhase::Off),
            ("gate.delta", GatePortalPhase::Closing { elapsed: 0.2 }),
            ("gate.echo", GatePortalPhase::On),
            ("gate.foxtrot", GatePortalPhase::Opening { elapsed: 0.3 }),
            ("gate.golf", GatePortalPhase::Off),
            ("gate.hotel", GatePortalPhase::Closing { elapsed: 0.4 }),
        ];
        let forward = phases_of(&entries);
        let reversed_entries: Vec<(&str, GatePortalPhase)> =
            entries.iter().rev().copied().collect();
        let reversed = phases_of(&reversed_entries);

        assert_eq!(forward, reversed, "the two maps must hold the same state");

        let sorted_keys: Vec<&str> = {
            let mut keys: Vec<&str> = entries.iter().map(|(id, _)| *id).collect();
            keys.sort_unstable();
            keys
        };
        for built in [&forward, &reversed] {
            let seen: Vec<&str> = built.phases.keys().map(String::as_str).collect();
            assert_eq!(
                seen, sorted_keys,
                "the phase container must ITERATE in key order — an unordered one \
                 puts the checksum back on a discipline the next editor can drop"
            );
        }

        assert_eq!(
            gate_portal_phases_checksum(&forward),
            gate_portal_phases_checksum(&reversed),
            "the projection must fold in key order, not in iteration order"
        );
    }

    /// A registrar that records what it was handed, and nothing else.
    ///
    /// It has no rollback backend and no `App` — which is the point: the
    /// registration this domain performs is expressible against the floor
    /// vocabulary alone, so a test in THIS crate can watch it happen.
    #[derive(Default)]
    struct CapturingRegistrar {
        calls: Vec<(&'static str, &'static str, &'static str, &'static str)>,
        checksums: Vec<Box<dyn std::any::Any>>,
    }

    impl RollbackRegistrar for CapturingRegistrar {
        fn rollback_resource_clone_checksum<T>(
            &mut self,
            owner: &'static str,
            name: &'static str,
            projection: &'static str,
            checksum: for<'a> fn(&'a T) -> u64,
        ) -> &mut Self
        where
            T: bevy_ecs::resource::Resource + Clone,
        {
            self.calls
                .push((owner, name, projection, std::any::type_name::<T>()));
            self.checksums.push(Box::new(checksum));
            self
        }
    }

    ///  the registration must hand over the VALUE projection, not a
    /// presence probe. `the_phase_projection_sees_the_elapsed_timer_and_the_variant`
    /// proves the projection is value-sensitive; it says nothing about whether the
    /// registration actually uses it. This closes that gap from the domain side —
    /// the function under test is the whole registration, and the checksum it
    /// registered is pulled back out and fed diverging states.
    ///
    ///  both terms are observed: the call is asserted to have happened at all (an empty
    /// `calls` fails), AND the registered function is asserted to separate two states that
    /// differ only in `elapsed`.
    #[test]
    fn the_domain_registers_its_own_phase_state_with_the_value_projection() {
        let mut registrar = CapturingRegistrar::default();
        register_gate_portal_rollback_state(&mut registrar);

        assert_eq!(
            registrar.calls.len(),
            1,
            "the gate-portal domain registers exactly one piece of rollback state"
        );
        let (owner, name, projection, type_name) = registrar.calls[0];
        assert_eq!(owner, GATE_PORTAL_ROLLBACK_OWNER);
        assert_eq!(
            name, "resource.gate_portal_phases",
            "the recorded schema name is wire-visible and must not drift when the \
             registration moves between crates"
        );
        assert_eq!(projection, "key-ordered phase/elapsed checksum projection");
        assert!(
            type_name.ends_with("GatePortalPhases"),
            "registered the wrong type: {type_name}"
        );

        let registered = registrar.checksums[0]
            .downcast_ref::<for<'a> fn(&'a GatePortalPhases) -> u64>()
            .copied()
            .expect("the registered checksum must project `GatePortalPhases` itself");

        let dt = 1.0 / 60.0;
        let ahead = phases_of(&[("gate.a", GatePortalPhase::Opening { elapsed: 4.0 * dt })]);
        let behind = phases_of(&[("gate.a", GatePortalPhase::Opening { elapsed: 3.0 * dt })]);
        assert_eq!(
            registered(&ahead),
            gate_portal_phases_checksum(&ahead),
            "the registration must hand over this domain's own projection"
        );
        assert_ne!(
            registered(&ahead),
            registered(&behind),
            "the REGISTERED projection must see the elapsed timer — a presence-only \
             probe passes coverage while restoring nothing of the value"
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

#[cfg(test)]
mod gate_portal_registry_tests {
    use super::*;

    fn config(switch: &str) -> (&str, &str, &str) {
        (switch, "portal_sprite", "ring_sprite")
    }

    /// ⭐ THE THREE ANSWERS, one test each, because they are three different
    /// decisions and a single "it works" arm would only exercise the first.
    #[test]
    fn a_fresh_zone_is_inserted() {
        let mut registry = GatePortalRegistry::default();
        let (s, p, r) = config("gate_switch");
        assert_eq!(
            registry.try_register("cove_gate", s, p, r),
            Ok(ambition_registry_core::RegistrationOutcome::Inserted)
        );
        assert!(registry.is_portal("cove_gate"));
    }

    /// A plugin whose install runs twice is not an error.
    #[test]
    fn the_same_portal_registered_twice_is_idempotent() {
        let mut registry = GatePortalRegistry::default();
        let (s, p, r) = config("gate_switch");
        registry.try_register("cove_gate", s, p, r).expect("first");
        assert_eq!(
            registry.try_register("cove_gate", s, p, r),
            Ok(ambition_registry_core::RegistrationOutcome::Idempotent)
        );
    }

    /// ⛔⛔ THE ONE THAT USED TO BE SILENT. A bare `insert` accepted this and the
    /// first portal simply stopped existing, with no error and no log — the
    /// behaviour the 2026-09-02 registry inventory found in seven registries.
    #[test]
    fn a_different_portal_may_not_take_a_zone_that_is_already_claimed() {
        let mut registry = GatePortalRegistry::default();
        let (s, p, r) = config("gate_switch");
        registry.try_register("cove_gate", s, p, r).expect("first");

        let refused = registry
            .try_register("cove_gate", "other_switch", p, r)
            .expect_err("a different portal on a claimed zone must be refused");
        assert_eq!(refused.existing.switch_id, "gate_switch");
        assert_eq!(refused.incoming.switch_id, "other_switch");

        // ⚠ AND THE REGISTRY IS UNCHANGED. A refusal that had already mutated
        // would be an overwrite wearing an error's clothes -- the fourth of the
        // inventory's four protocol questions, and the one a caller cannot check
        // for itself.
        assert!(registry
            .iter()
            .any(|(zone, held)| zone == "cove_gate" && held.switch_id == "gate_switch"));
        assert_eq!(registry.iter().count(), 1);
    }
}
