//! Limb-control vocabulary shared by articulated actors.
//!
//! A host carries a [`LimbRig`] and [`LimbIntents`]; [`fan_out_limb_intents`]
//! copies each slot's frame to the corresponding limb's `ActorControl`. Limbs
//! are ordinary actor bodies without their own brain or health.
//!
//! The rig is keyed by [`LimbSlot`] for deterministic fan-out, and a missing
//! intent explicitly neutralizes that limb. Mount-specific strike routing stays
//! with the mount code.

use std::collections::BTreeMap;

use ambition_platformer2d_core as ae;
use bevy::prelude::{Component, Entity, Query, With};

use crate::actor::control::ActorControlFrame;
use crate::control::ActorControl;

/// Longest authored slot name. Inline storage keeps [`LimbSlot`] `Copy`, which
/// is what lets it be a `BTreeMap` key and travel through rollback by value.
const LIMB_SLOT_CAP: usize = 24;

/// An authored limb slot name: `"hand_left"`, `"tail"`, `"wing_left"`.
///
/// ⭐ **OPEN, not an enum.** This was `enum LimbSlot { HandLeft, HandRight }`
/// documented as *"grows per content (a serpent boss adds variants)"* — which is
/// the wrong growth direction for an engine type, because it makes the shared
/// character crate the registry of every body part any content pack imagines.
/// Nothing in the engine branches on WHICH slot: it needs a typed name,
/// deterministic ordering, validation, and exact rig composition, and a
/// validated newtype gives all four.
///
/// ⛔ **validated, not a bare `String`.** `from_str` refuses anything that is
/// not non-empty `[a-z0-9_]` within [`LIMB_SLOT_CAP`], so a typo in authored
/// content is a load error rather than a route that silently drives nothing.
///
/// Ordering is bytewise over the zero-padded name, so rig iteration is decided
/// by the CONTENT and is stable across runs.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LimbSlot {
    name: [u8; LIMB_SLOT_CAP],
    len: u8,
}

/// Why an authored slot name was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LimbSlotError {
    Empty,
    TooLong,
    /// Carries the offending byte.
    BadChar(u8),
}

impl std::fmt::Display for LimbSlotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LimbSlotError::Empty => write!(f, "a limb slot name cannot be empty"),
            LimbSlotError::TooLong => {
                write!(f, "a limb slot name may be at most {LIMB_SLOT_CAP} bytes")
            }
            LimbSlotError::BadChar(b) => write!(
                f,
                "a limb slot name may only contain [a-z0-9_]; found {:?}",
                *b as char
            ),
        }
    }
}

impl LimbSlot {
    /// The two slots the engine itself names, for facing-side selection.
    pub const HAND_LEFT: LimbSlot = LimbSlot::literal(b"hand_left");
    pub const HAND_RIGHT: LimbSlot = LimbSlot::literal(b"hand_right");

    /// Const constructor for the engine's own well-known slots. Panics at
    /// COMPILE time on a bad literal, so these cannot drift from `from_str`'s
    /// rule without failing the build.
    const fn literal(bytes: &[u8]) -> LimbSlot {
        assert!(!bytes.is_empty(), "a limb slot literal cannot be empty");
        assert!(bytes.len() <= LIMB_SLOT_CAP, "limb slot literal too long");
        let mut name = [0u8; LIMB_SLOT_CAP];
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            assert!(
                b == b'_' || b.is_ascii_lowercase() || b.is_ascii_digit(),
                "a limb slot literal may only contain [a-z0-9_]"
            );
            name[i] = b;
            i += 1;
        }
        LimbSlot {
            name,
            len: bytes.len() as u8,
        }
    }

    pub fn from_str(name: &str) -> Result<LimbSlot, LimbSlotError> {
        let bytes = name.as_bytes();
        if bytes.is_empty() {
            return Err(LimbSlotError::Empty);
        }
        if bytes.len() > LIMB_SLOT_CAP {
            return Err(LimbSlotError::TooLong);
        }
        let mut stored = [0u8; LIMB_SLOT_CAP];
        for (i, &b) in bytes.iter().enumerate() {
            if !(b == b'_' || b.is_ascii_lowercase() || b.is_ascii_digit()) {
                return Err(LimbSlotError::BadChar(b));
            }
            stored[i] = b;
        }
        Ok(LimbSlot {
            name: stored,
            len: bytes.len() as u8,
        })
    }

    pub fn as_str(&self) -> &str {
        // Validated ASCII on every construction path, including `literal`.
        std::str::from_utf8(&self.name[..self.len as usize]).unwrap_or("")
    }

    /// A deterministic key for the rollback localization probe, which folds it
    /// into a checksum. FNV-1a over the name — the enum discriminant this
    /// replaces served the same purpose.
    pub fn probe_key(&self) -> u64 {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for &b in &self.name[..self.len as usize] {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100_0000_01b3);
        }
        hash
    }
}

impl std::fmt::Debug for LimbSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LimbSlot({:?})", self.as_str())
    }
}

impl std::fmt::Display for LimbSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for LimbSlot {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<LimbSlot, D::Error> {
        let raw = <&str as serde::Deserialize>::deserialize(d)?;
        LimbSlot::from_str(raw).map_err(serde::de::Error::custom)
    }
}

/// Host-owned mapping from limb slot to limb entity.
/// Slot keys prevent duplicate slots and give deterministic iteration order.
#[derive(Component, Clone, Default, Debug)]
pub struct LimbRig {
    pub limbs: BTreeMap<LimbSlot, Entity>,
}

impl LimbRig {
    pub fn from_pairs(pairs: impl IntoIterator<Item = (LimbSlot, Entity)>) -> Self {
        Self {
            limbs: pairs.into_iter().collect(),
        }
    }

    pub fn get(&self, slot: LimbSlot) -> Option<Entity> {
        self.limbs.get(&slot).copied()
    }

    /// Return the slot assigned to `limb`, if present.
    pub fn slot_of(&self, limb: Entity) -> Option<LimbSlot> {
        self.limbs
            .iter()
            .find(|(_, &entity)| entity == limb)
            .map(|(&slot, _)| slot)
    }
}

/// On each LIMB body: which host it belongs to and which slot it fills.
#[derive(Component, Clone, Debug)]
pub struct Limb {
    pub of: Entity,
    pub slot: LimbSlot,
    /// Host-local idle anchor in pixels. With no routed strike, the router steers
    /// the limb toward this offset in the host's gravity frame.
    pub home_offset: ae::Vec2,
}

/// Remembers the move currently driving limbs so `melee_pressed` fires only on
/// the active-window onset.
#[derive(Component, Clone, Default, Debug)]
pub struct LimbRouteState {
    active_move: Option<String>,
}

impl LimbRouteState {
    /// Advance the active-move memo and report whether a new strike began.
    pub fn begin_strike(&mut self, active_strike_move: Option<String>) -> bool {
        let onset = matches!(&active_strike_move, Some(m)
            if self.active_move.as_deref() != Some(m.as_str()));
        self.active_move = active_strike_move;
        onset
    }
}

/// Host-owned per-limb control intents for the current tick.
#[derive(Component, Clone, Default, Debug)]
pub struct LimbIntents(pub BTreeMap<LimbSlot, ActorControlFrame>);

/// Copy host limb intents into each rigged limb's `ActorControl`.
/// Missing slot intents are neutralized. Runs after host brain updates and before integration.
pub fn fan_out_limb_intents(
    hosts: Query<(&LimbRig, &LimbIntents)>,
    mut limbs: Query<&mut ActorControl, With<Limb>>,
) {
    for (rig, intents) in &hosts {
        // The host rig is authoritative for slot membership; do not derive the
        // slot from the limb's forward link, which may disagree with the rig.
        for (&slot, &limb_entity) in &rig.limbs {
            let Ok(mut control) = limbs.get_mut(limb_entity) else {
                continue; // despawned/unspawned limb: the rig tolerates gaps
            };
            control.0 = intents
                .0
                .get(&slot)
                .copied()
                .unwrap_or_else(ActorControlFrame::neutral);
        }
    }
}

impl bevy::ecs::entity::MapEntities for LimbRig {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        // Keys are slots, not entities, so remapping touches values only and
        // cannot collide two limbs onto one key.
        for entity in self.limbs.values_mut() {
            *entity = mapper.get_mapped(*entity);
        }
    }
}

impl bevy::ecs::entity::MapEntities for Limb {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        self.of = mapper.get_mapped(self.of);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::{App, Update};

    /// An open slot id must still refuse nonsense, or content typos become
    /// routes that silently drive nothing — the failure the old enum prevented
    /// and the reason this is validated rather than a bare `String`.
    #[test]
    fn a_slot_name_is_validated_and_orders_by_its_bytes() {
        assert_eq!(LimbSlot::from_str("tail").unwrap().as_str(), "tail");
        assert_eq!(
            LimbSlot::from_str("wing_left").unwrap().as_str(),
            "wing_left"
        );
        assert_eq!(LimbSlot::HAND_LEFT.as_str(), "hand_left");

        assert_eq!(LimbSlot::from_str(""), Err(LimbSlotError::Empty));
        assert_eq!(
            LimbSlot::from_str("Hand_Left"),
            Err(LimbSlotError::BadChar(b'H')),
            "an authored name is lowercase; accepting both spellings makes two \
             names for one slot"
        );
        assert_eq!(
            LimbSlot::from_str("hand-left"),
            Err(LimbSlotError::BadChar(b'-'))
        );
        assert_eq!(
            LimbSlot::from_str(&"x".repeat(LIMB_SLOT_CAP + 1)),
            Err(LimbSlotError::TooLong)
        );

        // Rig iteration order is the CONTENT's, and it is plain lexicographic —
        // including the prefix case, which zero padding is what gets right.
        let mut sorted = [
            LimbSlot::from_str("wing_left").unwrap(),
            LimbSlot::HAND_RIGHT,
            LimbSlot::from_str("hand").unwrap(),
            LimbSlot::HAND_LEFT,
        ];
        sorted.sort();
        assert_eq!(
            sorted.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            vec!["hand", "hand_left", "hand_right", "wing_left"]
        );

        // The probe key stands in for the discriminant the rollback census used
        // to fold in; distinct slots must not collapse to one digest entry.
        assert_ne!(
            LimbSlot::HAND_LEFT.probe_key(),
            LimbSlot::HAND_RIGHT.probe_key()
        );
        assert_eq!(
            LimbSlot::HAND_LEFT.probe_key(),
            LimbSlot::from_str("hand_left").unwrap().probe_key(),
            "the same name must key the same, however it was built"
        );
    }

    #[test]
    fn pilot_intents_fan_out_to_the_right_limbs_and_absent_slots_neutralize() {
        let mut app = App::new();
        app.add_systems(Update, fan_out_limb_intents);

        let host = app.world_mut().spawn_empty().id();
        let hand_l = app
            .world_mut()
            .spawn((
                Limb {
                    of: host,
                    slot: LimbSlot::HAND_LEFT,
                    home_offset: ae::Vec2::ZERO,
                },
                ActorControl(ActorControlFrame::neutral()),
            ))
            .id();
        let hand_r = app
            .world_mut()
            .spawn((
                Limb {
                    of: host,
                    slot: LimbSlot::HAND_RIGHT,
                    home_offset: ae::Vec2::ZERO,
                },
                ActorControl(ActorControlFrame::neutral()),
            ))
            .id();

        // The pilot's brain writes two DIVERGING limb intents: left hand
        // sweeps left and strikes; right hand climbs.
        let mut intents = LimbIntents::default();
        let mut left = ActorControlFrame::neutral();
        left.velocity_target = ae::WorldVec2::new(-300.0, 0.0);
        left.melee_pressed = true;
        intents.0.insert(LimbSlot::HAND_LEFT, left);
        let mut right = ActorControlFrame::neutral();
        right.velocity_target = ae::WorldVec2::new(0.0, -200.0);
        intents.0.insert(LimbSlot::HAND_RIGHT, right);
        app.world_mut().entity_mut(host).insert((
            LimbRig::from_pairs([
                (LimbSlot::HAND_LEFT, hand_l),
                (LimbSlot::HAND_RIGHT, hand_r),
            ]),
            intents,
        ));

        app.update();

        let l = app.world().get::<ActorControl>(hand_l).unwrap();
        assert_eq!(l.0.velocity_target, ae::WorldVec2::new(-300.0, 0.0));
        assert!(l.0.melee_pressed, "left hand got its strike edge");
        let r = app.world().get::<ActorControl>(hand_r).unwrap();
        assert_eq!(r.0.velocity_target, ae::WorldVec2::new(0.0, -200.0));
        assert!(!r.0.melee_pressed, "intents do not bleed across slots");

        // Next tick the pilot only drives the right hand: the left hand is
        // explicitly neutralized, not left running its stale sweep.
        let mut only_right = LimbIntents::default();
        let mut r2 = ActorControlFrame::neutral();
        r2.velocity_target = ae::WorldVec2::new(150.0, 0.0);
        only_right.0.insert(LimbSlot::HAND_RIGHT, r2);
        app.world_mut().entity_mut(host).insert(only_right);
        app.update();

        let l = app.world().get::<ActorControl>(hand_l).unwrap();
        assert_eq!(
            l.0.velocity_target,
            ae::WorldVec2::ZERO,
            "stale intent cleared"
        );
        assert!(!l.0.melee_pressed);
        let r = app.world().get::<ActorControl>(hand_r).unwrap();
        assert_eq!(r.0.velocity_target, ae::WorldVec2::new(150.0, 0.0));
    }

    /// The edge memo advances INSIDE the onset question: a strike that stays
    /// live reports onset exactly once, and a new move re-arms it.
    #[test]
    fn a_live_strike_reports_its_onset_exactly_once() {
        let mut state = LimbRouteState::default();
        assert!(state.begin_strike(Some("hand_slam".into())), "first tick");
        assert!(
            !state.begin_strike(Some("hand_slam".into())),
            "the same strike, still live, is not a second edge"
        );
        assert!(
            state.begin_strike(Some("hand_sweep".into())),
            "a different move re-arms"
        );
        assert!(!state.begin_strike(None), "no strike is never an onset");
        assert!(
            state.begin_strike(Some("hand_sweep".into())),
            "and clears the memo, so the same move re-arms after a gap"
        );
    }
}
