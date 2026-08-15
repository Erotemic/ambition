//! Gate portals — phase state machine + registry.
//!
//! Split out of the former 823-line `rooms/mod.rs` (2026-06-15); the
//! parent re-exports every type so `rooms::*` paths are unchanged.

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

/// **The value projection behind the `resource.gate_portal_phases` rollback
/// registration** — the checksum a GGRS host folds into its per-frame desync
/// detector, and the census a restore audit compares across a rewind.
///
/// ⭐ **the elapsed timer is the whole point.** The defect this projection
/// closes was an integrator running ahead of the input that drove it, so a
/// checksum that saw only *which zones have a phase* would agree with the bug.
/// Every field that decides when `Opening` becomes `On` is projected here.
///
/// ⛔ **keys are SORTED before hashing.** `phases` is a `HashMap`, and hashing it
/// in iteration order would make the checksum disagree between two peers holding
/// identical state — a desync detector that manufactures desyncs.
///
/// ⚠ **it lives beside the type, not in the rollback runtime.** It moved here
/// because it is domain semantics: it names every [`GatePortalPhase`] variant
/// and the field each one carries, so adding a variant must break an exhaustive
/// match in THIS file rather than in the netcode crate — the same argument
/// `snapshot_impls` makes for this crate's wire codecs. The encoders are
/// `ambition_platformer2d_core`'s, which this crate already depends on, so the
/// move costs no dependency edge. ⭐ **and the registration followed it**: see
/// [`register_gate_portal_rollback_state`] — the runtime no longer names this
/// type at all.
pub fn gate_portal_phases_checksum(phases: &GatePortalPhases) -> u64 {
    use ambition_platformer2d_core::snapshot::{checksum_bytes, put_f32, put_str, put_u64, put_u8};

    let mut ordered: Vec<(&String, &GatePortalPhase)> = phases.phases.iter().collect();
    ordered.sort_by(|left, right| left.0.cmp(right.0));

    let mut bytes = Vec::new();
    put_u64(&mut bytes, ordered.len() as u64);
    for (zone_id, phase) in ordered {
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

/// The owner label on this domain's rollback registrations — **this crate**,
/// because this crate now does the registering.
///
/// ⚠ `owner` is an organisational label: it is not in `schema_dump` and not
/// hashed into the schema fingerprint (that is exactly why it left the wire form
/// in schema v5), so moving the registration down here is a no-op for the
/// recorded baseline. It IS matched by `missing_required_state`, which is why it
/// is a named constant a `RequiredRollbackState` declaration can reuse verbatim.
pub const GATE_PORTAL_ROLLBACK_OWNER: &str = "ambition_platformer2d_world";

/// **Install the gate-portal domain's rollback state.**
///
/// ⭐ **this is the domain naming its own type.** It used to be one line in
/// `ambition_platformer2d_runtime::rollback::register_engine_rollback_state`,
/// on the argument that `bevy_ggrs` registration is generic over the concrete
/// type and only the netcode crate may name `bevy_ggrs`. The first half is true
/// and the second half is a boundary worth keeping — but neither implies the
/// netcode crate must own the LIST. It takes a [`RollbackRegistrar`] and the
/// host passes one in; the monomorphisation happens at the host's call
/// site, and this crate stays `bevy_ggrs`-free (its whole path-dependency
/// closure is seven crates, none of which names it).
///
/// ⛔⛔ **registered with a VALUE projection, not a presence probe.** The whole
/// defect this closes is an INTEGRATOR running ahead of the input that drove it:
/// a probe that saw only *which zones have a phase* would restore that phases
/// exist, say nothing about `elapsed`, and pass while reproducing the bug. See
/// [`gate_portal_phases_checksum`] for what is folded, and [`GatePortalPhases`]
/// for why the timer is authoritative rather than a cache.
///
/// ⚠ **taken by `&mut impl`, not `&mut dyn`.** The registrar's methods are
/// generic, so the trait is not object-safe by construction, and must not become
/// so — see the trait's own note.
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

    fn phases_of(entries: &[(&str, GatePortalPhase)]) -> GatePortalPhases {
        let mut phases = GatePortalPhases::default();
        for (zone_id, phase) in entries {
            *phases.phase_mut(zone_id) = *phase;
        }
        phases
    }

    /// ⛔ **a presence-only projection would agree with the bug.** The registration
    /// this backs exists because an `elapsed` timer ran ahead of the switch that
    /// drove it, and every zone stayed present the whole time.
    ///
    /// ⚠ both terms are observed: identical states must AGREE, and a one-tick
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

    /// ⛔ **the projection must not read `HashMap` iteration order.** Two peers
    /// holding identical state build their maps by different insertion routes and
    /// hash their keys with different per-instance seeds; folding in iteration
    /// order would report a desync between two worlds that agree.
    #[test]
    fn the_phase_projection_ignores_hash_map_insertion_order() {
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
        assert_eq!(
            gate_portal_phases_checksum(&forward),
            gate_portal_phases_checksum(&reversed),
            "the projection must sort its keys, not fold them in iteration order"
        );
    }

    /// **A registrar that records what it was handed, and nothing else.**
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

    /// ⛔⛔ **the registration must hand over the VALUE projection, not a
    /// presence probe.** `the_phase_projection_sees_the_elapsed_timer_and_the_variant`
    /// proves the projection is value-sensitive; it says nothing about whether the
    /// registration actually uses it. This closes that gap from the domain side —
    /// the function under test is the whole registration, and the checksum it
    /// registered is pulled back out and fed diverging states.
    ///
    /// ⚠ both terms are observed: the call is asserted to have happened at all
    /// (an empty `calls` fails), AND the registered function is asserted to
    /// separate two states that differ only in `elapsed`. A registration that
    /// passed a presence-only projection fails the second assertion while
    /// satisfying every coverage sweep — which is the exact defect
    /// `resource.gate_portal_phases` was created to close.
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
