//! Mechanics registry (NOT YET WIRED INTO THE HUD).
//!
//! Lightweight in-memory catalog of player-facing mechanics, organized
//! by category and tagged with maturity status. The registry is meant
//! as a HUD-friendly index for the sandbox's "what mechanics are wired
//! up right now" view, and as a discoverability hook so future patches
//! that add a verb can register it once and have it show up in the
//! sandbox without touching dozens of call sites.
//!
//! This is intentionally a sandbox-side resource. The engine owns
//! reusable primitives (`LocomotionState`, `BodyMode`, `ResourceMeter`,
//! ...), but the *catalog of which verbs the current sandbox demos*
//! is presentation concerns: HUD label, station id in LDtk, current
//! status, etc.
//!
//! Nothing currently *consumes* the registry — the labels, queries,
//! and category iterators are all reserved for a HUD slice that
//! hasn't landed. Module-wide `allow(dead_code)` keeps the orphan
//! API visible in the type system without polluting the warning
//! stream.
#![allow(dead_code)]

use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MechanicCategory {
    CoreLocomotion,
    BodyState,
    TeleportBlink,
    GrappleTether,
    Combat,
    Resources,
    Environment,
    MathTraversal,
    Time,
}

impl MechanicCategory {
    pub fn label(self) -> &'static str {
        match self {
            MechanicCategory::CoreLocomotion => "Core locomotion",
            MechanicCategory::BodyState => "Body state",
            MechanicCategory::TeleportBlink => "Teleport / blink",
            MechanicCategory::GrappleTether => "Grapple / tether",
            MechanicCategory::Combat => "Combat",
            MechanicCategory::Resources => "Resources",
            MechanicCategory::Environment => "Environment",
            MechanicCategory::MathTraversal => "Mathematical traversal",
            MechanicCategory::Time => "Time / clocks",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MechanicMaturity {
    /// Backend primitive exists; no playable verb yet.
    Backend,
    /// Playable but rough.
    Prototype,
    /// Playable, tuned, debuggable.
    Stable,
    /// Listed for visibility but not implemented yet.
    Planned,
}

impl MechanicMaturity {
    pub fn label(self) -> &'static str {
        match self {
            MechanicMaturity::Backend => "backend",
            MechanicMaturity::Prototype => "prototype",
            MechanicMaturity::Stable => "stable",
            MechanicMaturity::Planned => "planned",
        }
    }
}

/// One catalog entry. Field shapes are intentionally `String` /
/// `&'static str` mixes so adding a new mechanic requires only a literal
/// table entry, not a builder.
#[derive(Clone, Debug)]
pub struct MechanicEntry {
    pub id: &'static str,
    pub name: &'static str,
    pub category: MechanicCategory,
    pub maturity: MechanicMaturity,
    pub input_hint: &'static str,
    pub requires: &'static str,
    pub station: Option<&'static str>,
    pub doc: &'static str,
}

/// Bevy resource holding the catalog.
#[derive(Resource, Clone, Debug)]
pub struct MechanicsRegistry {
    pub entries: Vec<MechanicEntry>,
}

impl Default for MechanicsRegistry {
    fn default() -> Self {
        Self {
            entries: default_entries(),
        }
    }
}

impl MechanicsRegistry {
    pub fn by_category(&self, category: MechanicCategory) -> impl Iterator<Item = &MechanicEntry> {
        self.entries.iter().filter(move |e| e.category == category)
    }

    pub fn find(&self, id: &str) -> Option<&MechanicEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    pub fn count_by_maturity(&self, maturity: MechanicMaturity) -> usize {
        self.entries
            .iter()
            .filter(|e| e.maturity == maturity)
            .count()
    }
}

fn default_entries() -> Vec<MechanicEntry> {
    use MechanicCategory::*;
    use MechanicMaturity::*;
    vec![
        MechanicEntry {
            id: "run_walk",
            name: "Run / Walk",
            category: CoreLocomotion,
            maturity: Stable,
            input_hint: "L-stick / WASD (Shift = walk)",
            requires: "—",
            station: None,
            doc: "Single-body kinematic controller with input buffering.",
        },
        MechanicEntry {
            id: "jump",
            name: "Variable / coyote / buffered jump",
            category: CoreLocomotion,
            maturity: Stable,
            input_hint: "A / Space",
            requires: "—",
            station: None,
            doc: "Coyote time, jump buffering, double jump / air jumps.",
        },
        MechanicEntry {
            id: "dash",
            name: "Dash + dash charges",
            category: CoreLocomotion,
            maturity: Stable,
            input_hint: "X / Shift",
            requires: "Dash ability",
            station: None,
            doc: "Charge-based dash with refresh on ground / pogo.",
        },
        MechanicEntry {
            id: "blink",
            name: "Blink (quick + precision)",
            category: TeleportBlink,
            maturity: Stable,
            input_hint: "B / Q",
            requires: "Blink ability",
            station: None,
            doc: "Bullet-time aim mode, soft/hard wall phasing.",
        },
        MechanicEntry {
            id: "pogo_rebound",
            name: "Pogo / rebound impulse",
            category: Combat,
            maturity: Stable,
            input_hint: "Down + Attack / dedicated Pogo",
            requires: "—",
            station: None,
            doc: "Surface and orb pogo refreshes movement resources.",
        },
        MechanicEntry {
            id: "slash",
            name: "Basic slash / melee",
            category: Combat,
            maturity: Stable,
            input_hint: "Y / J",
            requires: "—",
            station: None,
            doc: "Hitstop + camera shake, slash hitbox routes through engine combat module.",
        },
        MechanicEntry {
            id: "locomotion_state_enum",
            name: "LocomotionState enum",
            category: CoreLocomotion,
            maturity: Backend,
            input_hint: "—",
            requires: "—",
            station: None,
            doc: "Explicit player movement-mode enum for HUD/trace/AI; replaces ad-hoc booleans.",
        },
        MechanicEntry {
            id: "body_mode_enum",
            name: "BodyMode + collision-safe shape",
            category: BodyState,
            maturity: Backend,
            input_hint: "—",
            requires: "—",
            station: None,
            doc: "Stance enum with per-mode AABB shape and `fits_at` query for collision-safe resize.",
        },
        MechanicEntry {
            id: "resource_meter",
            name: "ResourceMeter primitive",
            category: Resources,
            maturity: Backend,
            input_hint: "—",
            requires: "—",
            station: None,
            doc: "Generic stamina/mana/ammo/charge primitive with regen+decay tick.",
        },
        MechanicEntry {
            id: "trace_recorder",
            name: "Gameplay flight recorder",
            category: Time,
            maturity: Stable,
            input_hint: "F8",
            requires: "—",
            station: None,
            doc: "Rolling per-frame trace + auto-OOB dump under debug_traces/.",
        },
        MechanicEntry {
            id: "crouch",
            name: "Crouch",
            category: BodyState,
            maturity: Prototype,
            input_hint: "Down (held while grounded)",
            requires: "BodyMode backend",
            station: Some("basement_mechanics/body"),
            doc: "Half-height stance with collision-safe stand-up gating; blocked stand-up keeps the player crouched under low ceilings.",
        },
        MechanicEntry {
            id: "morph_ball",
            name: "Morph ball",
            category: BodyState,
            maturity: Prototype,
            input_hint: "Down + Down (grounded); Jump to unmorph",
            requires: "BodyMode backend",
            station: Some("basement_mechanics/body"),
            doc: "Compact stance for narrow tunnels; unmorph gated by collision-safe stand-up.",
        },
        MechanicEntry {
            id: "grapple_clawline",
            name: "Grapple / clawline (planned)",
            category: GrappleTether,
            maturity: Planned,
            input_hint: "RB",
            requires: "Targeting backend",
            station: Some("basement_mechanics/grapple"),
            doc: "Linecast-to-anchor pull. Backend not yet wired.",
        },
        MechanicEntry {
            id: "projectile",
            name: "Projectile / fireball / Hadouken",
            category: Combat,
            maturity: Prototype,
            input_hint: "F (kbd) / West face button (gamepad)",
            requires: "ResourceMeter, projectile backend, motion-input buffer",
            station: Some("basement_mechanics/projectile"),
            doc: "Resource-cost player projectile. Half-circle + fire upgrades to Hadouken.",
        },
        MechanicEntry {
            id: "parry",
            name: "Parry / counter (planned)",
            category: Combat,
            maturity: Planned,
            input_hint: "LB",
            requires: "Combat windows",
            station: Some("basement_mechanics/parry"),
            doc: "Active window with miss cooldown and counter-knockback.",
        },
        MechanicEntry {
            id: "functional_zip",
            name: "Functional zip (planned)",
            category: MathTraversal,
            maturity: Planned,
            input_hint: "—",
            requires: "Curve backend",
            station: Some("basement_mechanics/curve"),
            doc: "Parametric-curve movement with collision-safe traversal and preview.",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_default_entries() {
        let registry = MechanicsRegistry::default();
        assert!(registry.entries.len() >= 10);
    }

    #[test]
    fn registry_find_returns_known_id() {
        let registry = MechanicsRegistry::default();
        assert!(registry.find("blink").is_some());
        assert!(registry.find("nonexistent_id").is_none());
    }

    #[test]
    fn registry_by_category_returns_only_matching() {
        let registry = MechanicsRegistry::default();
        let combat: Vec<_> = registry.by_category(MechanicCategory::Combat).collect();
        assert!(!combat.is_empty());
        assert!(combat
            .iter()
            .all(|e| e.category == MechanicCategory::Combat));
    }

    #[test]
    fn registry_count_by_maturity_consistent_with_entries() {
        let registry = MechanicsRegistry::default();
        let total: usize = [
            MechanicMaturity::Backend,
            MechanicMaturity::Prototype,
            MechanicMaturity::Stable,
            MechanicMaturity::Planned,
        ]
        .iter()
        .map(|m| registry.count_by_maturity(*m))
        .sum();
        assert_eq!(total, registry.entries.len());
    }

    #[test]
    fn registry_entry_ids_are_unique() {
        // If two entries share an id, `find` becomes ambiguous and HUD/UI
        // wiring that keys on id silently picks one. Catch the
        // collision at boot time.
        let registry = MechanicsRegistry::default();
        let mut ids: Vec<&str> = registry.entries.iter().map(|e| e.id).collect();
        ids.sort();
        let count = ids.len();
        ids.dedup();
        assert_eq!(count, ids.len(), "duplicate mechanic id in registry");
    }

    #[test]
    fn registry_entries_have_nonempty_text_fields() {
        // A registry entry with an empty id, name, or doc means the
        // HUD will render a blank cell for it. Treat as a contract.
        let registry = MechanicsRegistry::default();
        for entry in &registry.entries {
            assert!(!entry.id.is_empty(), "entry id is empty");
            assert!(!entry.name.is_empty(), "entry {} has empty name", entry.id);
            assert!(!entry.doc.is_empty(), "entry {} has empty doc", entry.id);
        }
    }

    #[test]
    fn category_labels_are_distinct() {
        // `label()` is rendered in HUDs, so collisions would show two
        // categories under the same heading.
        let cats = [
            MechanicCategory::CoreLocomotion,
            MechanicCategory::BodyState,
            MechanicCategory::TeleportBlink,
            MechanicCategory::GrappleTether,
            MechanicCategory::Combat,
            MechanicCategory::Resources,
            MechanicCategory::Environment,
            MechanicCategory::MathTraversal,
            MechanicCategory::Time,
        ];
        let mut labels: Vec<&str> = cats.iter().map(|c| c.label()).collect();
        labels.sort();
        let count = labels.len();
        labels.dedup();
        assert_eq!(count, labels.len(), "category label collision");
    }

    #[test]
    fn maturity_labels_are_distinct() {
        let labels = [
            MechanicMaturity::Backend.label(),
            MechanicMaturity::Prototype.label(),
            MechanicMaturity::Stable.label(),
            MechanicMaturity::Planned.label(),
        ];
        let mut sorted: Vec<&str> = labels.iter().copied().collect();
        sorted.sort();
        let count = sorted.len();
        sorted.dedup();
        assert_eq!(count, sorted.len(), "maturity label collision");
    }
}
