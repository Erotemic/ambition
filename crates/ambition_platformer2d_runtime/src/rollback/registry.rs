//! Backend-neutral rollback schema metadata.
//!
//! Gameplay domains declare typed rollback obligations through
//! `ambition_platformer2d_core::snapshot::RollbackRegistrar`. This module records the exact
//! managed schema those declarations describe; concrete rollback hosts install storage/checksum
//! machinery separately.

use std::collections::BTreeMap;
use std::fmt;

use bevy::prelude::*;

use crate::content_identity::SnapshotSchemaFingerprint;

/// Same-build rollback compatibility version.
///
/// Bump this for incompatible changes to registered state, encodings,
/// checksum projections, or deterministic simulation behavior. A stable
/// registry dump does not prove the encoded payload is unchanged. Do not
/// reuse retired wire discriminants for different operations. Peers with
/// different schema versions must not exchange rollback state.
/// 350 -> 351: `resource.day_clock` (`ambition_time::DayClock`, the time of
/// day: a day number and the seconds into it) joins the canonical rollback
/// state. One row is added and one type is encoded.
/// 351 -> 352: a fighter brain's reaction varies at each decision (the
/// view's age is drawn from its noise stream), and the brain's checksum
/// projection carries that age. Two builds on either side play different
/// CPUs from the same inputs.
/// 352 -> 353: `portal.backside` (`PortalBackside`, a body on the back of a
/// portal pair) joins the rollback state, and portal transit changes with it:
/// a crossing begins only from a portal's face, and a body that cuts a
/// portal's plane past an end of its opening is snagged and not taken.
/// 353 -> 354: a body in two places (`docs/planning/engine/portals-a-body-in-
/// two-places.md`). `boss.limb_strike_box` (`LimbStrikeBox`, the damage box a
/// boss's limb carries) joins the rollback state. A strike that reaches
/// through a portal pair hurts at its far end, a body half through is hit at
/// both, a held body is threaded and not moved by a pair, and a body in a
/// pair collides with what is in front of its far end.
/// 354 -> 355: a swept portal crossing takes a body only if it fits the
/// opening where its centre crosses (not where its step ends), the rescue only
/// a body in line with the opening, and a shot's contacts end only where that
/// crossing takes it; a glued pair carries what is in front of the far end by
/// its exact turned region, every corner mapped.
/// 355 -> 356: `boss.attack_intent` (`BossAttackIntent`) carries the authored
/// windup and strike lengths of the step's beat (two `f32`s), and a boss
/// bakes one move for each distinct beat. Each pattern step plays at its own
/// lengths; before, a move kept the lengths of its first occurrence.
/// 356 -> 357: `combat.ray_strike` (`RayStrike`, a ray's line on its strike:
/// the volume that hurts and the line that is drawn) joins the rollback state.
/// A held ray is cast through the portal pairs each tick it is asked for.
/// Also recorded at 357: `035706e4f`'s codec change, which shipped under 356
/// unrecorded: a skirmisher brain's state encodes its ranged volley (cooldown,
/// orbit phase, volley phase and aim), and `ActorControlFrame` its
/// `ranged_aim_held` bool.
/// 357 -> 358: `combat.ray_strike` is value-probed (its line and look):
/// a held ray is cast again each tick, so presence alone could not find a
/// replay that put it elsewhere.
/// 358 -> 359: `smash.placed_spring` (`PlacedSpring`) leaves the rollback
/// state. A plate is a module entity (`ability.module_entity`), whose value
/// now also carries the box bodies touch and the spawn's params, and its
/// clocks and uses are the module record `smash/spring.plate`.
/// 359 -> 360: `AxisJumpLaw` gains `SquatDecided` (wire code 2, one `f32`:
/// the short hop's fraction of the launch), the platform fighter's jump, in
/// the motion model's encoded tuning. The encoded ability tuning gains
/// `spot_dodge_endlag` (`f32`), and the axis maneuver state `air_dodge_push`
/// (`Vec2`), what an air dodge put in and still has to run down. The encoded
/// locomotion gains `launch_decay` (`f32`).
/// 360 -> 361: the encoded ability tuning gains the directional air dodge's
/// window and endlag and both air dodges' landing lags (four `f32`s), and
/// the axis maneuver state `air_dodge_directional` (`bool`).
/// `projectile.launch` (`ProjectileLaunch`, two `f32`s: base and growth)
/// enters: a shot that carries one launches like a hitbox, growing with the
/// victim's damage, instead of the fixed projectile nudge.
/// 361 -> 362: `boss.death_animation` (`BossDeathAnimation`) is
/// `body.death_beat` (`DeathBeat`), on every body that can die: the death
/// row's length, the time since the body died, and what its corpse does.
/// 362 -> 363: `boss.encounter`'s cursor encodes the boss's `scale` (one
/// `f32`): a conductor may grow its boss (`ambition.boss.scale`).
/// 363 -> 364: `actor.brain`'s cursor encodes a `MeleeBrute`'s hit and run
/// (wire tag 6: whether its swing was live, a `bool`, and how long it still
/// backs off, an `f32`).
/// 364 -> 365: `actor.worn_skin` (`WornSkin`, a clone of a character id)
/// enters: the look a Smash seat is drawn in, art only.
/// 365 -> 366: `actor.brain`'s cursor encodes more of a `MeleeBrute` (wire
/// tag 6): the route a brute that navigates follows (as Roam's), and its
/// tactics: its health last tick (`f32`), whether it closes in (`bool`), how
/// long it has held (`f32`), and how many swings it has made (`u32`).
/// The same version also covers 6c6cb479d, which landed at 365 with no bump:
/// `actor.control`'s fire request and `MovePlayback`'s aim each carry the
/// point a brain aimed at (`ActorFireRequest::at`: a `bool`, then a `Vec2`).
pub const GGRS_ROLLBACK_SCHEMA_VERSION: u32 = 366;

pub use ambition_platformer2d_core::rollback_kind::RollbackEntryKind;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RollbackRegistrationDescriptor {
    pub name: String,
    pub owner: String,
    pub kind: RollbackEntryKind,
    pub type_name: String,
    pub detail: String,
}

#[derive(Resource, Debug, Default)]
pub struct RollbackRegistry {
    entries: BTreeMap<String, RollbackRegistrationDescriptor>,
    /// Memoised [`Self::schema_fingerprint`]. See that method for why.
    ///
    /// ⭐ SOUND BECAUSE `entries` HAS EXACTLY ONE MUTATION SITE — the `insert` in
    /// `try_register` — which clears this. A second mutation path would have to
    /// clear it too, which is why `entries` stays private.
    fingerprint: std::sync::OnceLock<SnapshotSchemaFingerprint>,
}

// ⛔ HAND-WRITTEN because `OnceLock` is not `Clone`. A clone starts with an EMPTY
// memo rather than copying it: same value on next demand, and no risk of a clone
// carrying a fingerprint its own entries no longer justify.
impl Clone for RollbackRegistry {
    fn clone(&self) -> Self {
        Self {
            entries: self.entries.clone(),
            fingerprint: std::sync::OnceLock::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RollbackRegistrationError {
    EmptyName,
    EmptyOwner,
    Conflict {
        name: String,
        existing: RollbackRegistrationDescriptor,
        incoming: RollbackRegistrationDescriptor,
    },
    /// Two DIFFERENT Rust types reduce to the same [`wire_type_identity`].
    ///
    /// This is what keeps v20's narrower identity sound. The fingerprint hashes
    /// the type's final segment so that relocating a type is not a wire-format
    /// change — and that is only truthful while final segments are unique. Two
    /// crates each registering a `Cooldown` would hash equal, and a peer that
    /// had them the other way round would be declared compatible.
    ///
    ///  registering ONE type under several stable names is not this. The whole
    /// point of a stable name is that it identifies the registration; 39 of the
    /// live rows do exactly that, and they carry identical type names.
    TypeIdentityCollision {
        identity: String,
        existing: RollbackRegistrationDescriptor,
        incoming: RollbackRegistrationDescriptor,
    },
}

impl fmt::Display for RollbackRegistrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => write!(f, "rollback registration name must not be empty"),
            Self::EmptyOwner => write!(f, "rollback registration owner must not be empty"),
            Self::Conflict {
                name,
                existing,
                incoming,
            } => write!(
                f,
                "conflicting rollback registration '{name}': existing {existing:?}, incoming {incoming:?}"
            ),
            Self::TypeIdentityCollision {
                identity,
                existing,
                incoming,
            } => write!(
                f,
                "two different types share the rollback wire identity '{identity}', which the \
                 schema fingerprint cannot tell apart: existing {existing:?}, incoming {incoming:?}. \
                 Since v20 the fingerprint hashes a type's FINAL SEGMENT so that moving a type \
                 between crates or modules is not a wire-format change, and that stays sound only \
                 while final segments are unique. Rename one of the two types."
            ),
        }
    }
}

impl std::error::Error for RollbackRegistrationError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RollbackRegistrationOutcome {
    /// The descriptor was inserted and the active GGRS host should install its
    /// runtime snapshot/checksum machinery.
    Inserted,
    /// The exact descriptor was already present.
    Idempotent,
    /// The descriptor was inserted for schema/content identity, but this host
    /// does not run GGRS and therefore must not install rollback machinery.
    RecordedOnly,
}

/// The part of a type's name that a CARVE leaves alone.
///
///  the final segment, and not the module path below the crate, which is what the answer
/// was until the diff it cited was read.
///
/// Every path INSIDE the name is shortened, not only the outermost one, so a
/// generic keeps its constructor: `Vec<foo::Bar>` is `Vec<Bar>` and not `Bar>`.
/// No registration is generic today; taking a single `rsplit` would quietly give
/// `Vec<X>` and `VecDeque<X>` one identity, and this is a hash whose entire job
/// is telling wire formats apart.
fn wire_type_identity(type_name: &str) -> String {
    let mut out = String::with_capacity(type_name.len());
    let mut path = String::new();
    for ch in type_name.chars() {
        if ch.is_alphanumeric() || ch == '_' || ch == ':' {
            path.push(ch);
        } else {
            out.push_str(final_segment(&path));
            path.clear();
            out.push(ch);
        }
    }
    out.push_str(final_segment(&path));
    out
}

fn final_segment(path: &str) -> &str {
    path.rsplit("::").next().unwrap_or(path)
}

impl RollbackRegistry {
    pub fn try_register(
        &mut self,
        descriptor: RollbackRegistrationDescriptor,
    ) -> Result<RollbackRegistrationOutcome, RollbackRegistrationError> {
        // The protocol every canonical registry answers, read from
        // `ambition_registry_core` (2026-09-03): blank identity refused by
        // name, a second registration classified as idempotent or conflict,
        // never silently replaced. The wire-identity collision below is this
        // registry's own extra rule on top of it.
        if let Err(empty) = ambition_registry_core::require_non_empty(&[
            ("name", &descriptor.name),
            ("owner", &descriptor.owner),
        ]) {
            return Err(match empty.field {
                "name" => RollbackRegistrationError::EmptyName,
                _ => RollbackRegistrationError::EmptyOwner,
            });
        }
        match ambition_registry_core::classify(self.entries.get(&descriptor.name), &descriptor) {
            ambition_registry_core::Classification::Idempotent => {
                return Ok(RollbackRegistrationOutcome::Idempotent);
            }
            ambition_registry_core::Classification::Conflict { existing } => {
                return Err(RollbackRegistrationError::Conflict {
                    name: descriptor.name.clone(),
                    existing: existing.clone(),
                    incoming: descriptor,
                });
            }
            ambition_registry_core::Classification::New => {}
        }
        // What keeps v20's narrower identity sound. The fingerprint hashes
        // [`wire_type_identity`] so that relocating a type is not a wire-format
        // change; two crates each registering a `Cooldown` would then hash equal,
        // and a peer that had the two the other way round would be declared
        // compatible with this one. The duplicate-NAME refusal above does not
        // reach it — these arrive under different stable names, which is exactly
        // the case that looks legitimate.
        let identity = wire_type_identity(&descriptor.type_name);
        let collision = self
            .entries
            .values()
            .find(|existing| {
                existing.type_name != descriptor.type_name
                    && wire_type_identity(&existing.type_name) == identity
            })
            .cloned();
        if let Some(existing) = collision {
            return Err(RollbackRegistrationError::TypeIdentityCollision {
                identity,
                existing,
                incoming: descriptor,
            });
        }
        self.entries.insert(descriptor.name.clone(), descriptor);
        // The memo describes the entry set that just changed.
        self.fingerprint = std::sync::OnceLock::new();
        Ok(RollbackRegistrationOutcome::Inserted)
    }

    pub fn descriptors(&self) -> impl Iterator<Item = &RollbackRegistrationDescriptor> {
        self.entries.values()
    }

    /// Stable human-readable representation; byte-identical under equivalent
    /// plugin/registration insertion orders.
    pub fn deterministic_dump(&self) -> String {
        let rows: Vec<String> = self
            .entries
            .values()
            .map(|entry| {
                ambition_registry_core::canonical_row(&[
                    &entry.name,
                    &entry.owner,
                    entry.kind.canonical_name(),
                    &entry.type_name,
                    &entry.detail,
                ])
            })
            .collect();
        ambition_registry_core::canonical_section(
            Some(&format!("ggrs-rollback-schema-v{GGRS_ROLLBACK_SCHEMA_VERSION}")),
            rows.iter().map(String::as_str),
        )
    }

    /// What the schema actually IS, with every organisational label removed.
    ///
    /// [`Self::deterministic_dump`] carries `owner` and the type's full path because a human
    /// reading a conflict wants to know which module registered a thing and where the type
    /// lives.
    ///
    /// Both moves require the schema fingerprint to stay unchanged — which was impossible while
    /// the fingerprint hashed who registered a row. `owner` left in v5; [`wire_type_identity`]
    /// is the second half of that decision, in v20.
    pub fn schema_dump(&self) -> String {
        let rows: Vec<String> = self
            .entries
            .values()
            // ⛔⛤ THE ONLY FILTER IN THIS DUMP, AND IT IS THE PEER QUESTION.
            // `deterministic_dump` keeps every entry because it describes what
            // this build registered; this one describes what a peer can observe,
            // and a registration outside the schema identity is not part of
            // that. Measured 2026-09-16: without this, `--features causal` moved
            // the fingerprint (494 -> 497 rows) for a simulation that is
            // mechanically identical, so two such peers would refuse each other
            // for no mechanical reason.
            .filter(|entry| entry.kind.in_peer_schema_identity())
            .map(|entry| {
                ambition_registry_core::canonical_row(&[
                    &entry.name,
                    entry.kind.canonical_name(),
                    &wire_type_identity(&entry.type_name),
                    &entry.detail,
                ])
            })
            .collect();
        ambition_registry_core::canonical_section(
            Some(&format!("ggrs-rollback-schema-v{GGRS_ROLLBACK_SCHEMA_VERSION}")),
            rows.iter().map(String::as_str),
        )
    }

    /// The peer schema identity: [`Self::schema_dump`]'s rows with each
    /// row's prose `detail` replaced by its mechanism token
    /// ([`ambition_platformer2d_core::rollback_kind::mechanism_of`]). The
    /// schema fingerprint hashes this (`Q122`: the mechanical identity hashes
    /// mechanical facts, not explanatory prose). `schema_dump` keeps the prose
    /// for readers and the recorded baseline.
    pub fn mechanical_dump(&self) -> String {
        let rows: Vec<String> = self
            .entries
            .values()
            .filter(|entry| entry.kind.in_peer_schema_identity())
            .map(|entry| {
                ambition_registry_core::canonical_row(&[
                    &entry.name,
                    entry.kind.canonical_name(),
                    &wire_type_identity(&entry.type_name),
                    ambition_platformer2d_core::rollback_kind::mechanism_of(
                        entry.kind,
                        &entry.detail,
                    ),
                ])
            })
            .collect();
        ambition_registry_core::canonical_section(
            Some(&format!("ggrs-rollback-schema-v{GGRS_ROLLBACK_SCHEMA_VERSION}")),
            rows.iter().map(String::as_str),
        )
    }

    /// Which of these requirements is NOT installed.
    ///
    /// A capability offers its rollback state and the composition installs it,
    /// which keeps the capability's dependency closure to foundations. The hole
    /// that leaves is that nothing forces the composition to accept the offer —
    /// and a skipped registration is a DESYNC, not a missing feature.
    ///
    /// This closes it the way the content compiler closes the same shape: the
    /// obligation is declared next to the thing that has it
    /// ([`ambition_platformer2d_core::snapshot::RequiredRollbackState`]) and the
    /// assembler can refuse when it is unmet.
    ///
    ///  it checks the OWNER too. A name registered by somebody else is not
    /// this capability's state — two capabilities may reasonably both want a
    /// `cooldown`, and only the owner distinguishes them.
    pub fn missing_required_state<'a>(
        &self,
        required: &'a [ambition_platformer2d_core::snapshot::RequiredRollbackState],
    ) -> Vec<&'a ambition_platformer2d_core::snapshot::RequiredRollbackState> {
        required
            .iter()
            .filter(|req| {
                !self
                    .entries
                    .values()
                    .any(|entry| entry.name == req.name && entry.owner == req.owner)
            })
            .collect()
    }

    /// ⛔⛔ MEMOISED SINCE 2026-08-29, BECAUSE IT WAS COSTING 292us OF EVERY FRAME.
    /// `enforce_session_contract` calls this once per frame to notice a schema
    /// that changed under a live session. Uncached that meant building the entire
    /// schema DUMP as a ~40KB `String` — the same table
    /// `rollback_schema_baseline.txt` records — and blake3ing it, 60+ times a
    /// second, to detect a change that can only happen when `entries` is mutated:
    /// **8.29s of a four-minute hardware run, the second largest recurring zone
    /// in the trace.**
    ///
    /// ⭐ The VALUE is unchanged, which is the point — this is a pure memo, so
    /// every existing schema/baseline test still pins the same fingerprint and
    /// no new correctness surface appears. `try_register` clears it.
    pub fn schema_fingerprint(&self) -> SnapshotSchemaFingerprint {
        if let Some(memo) = self.fingerprint.get() {
            return memo.clone();
        }
        let computed = self.compute_schema_fingerprint();
        // A racing caller may have filled it first; either value is identical.
        let _ = self.fingerprint.set(computed.clone());
        computed
    }

    /// HAS THIS REGISTRY ALREADY COMPUTED ITS FINGERPRINT?
    ///
    /// ⭐ THE INSTRUMENT THAT CATCHES A DEFEATED MEMO. The memo is per-instance
    /// and a clone starts empty, so a hot-path caller that reads the fingerprint
    /// off a CLONE recomputes it every single time while every value-based test
    /// still passes. Asking the world's own registry whether it is memoised is
    /// the one question that separates "the cache exists" from "the cache is the
    /// thing being read". Callers outside a test have no reason for it.
    pub fn fingerprint_is_memoised(&self) -> bool {
        self.fingerprint.get().is_some()
    }

    fn compute_schema_fingerprint(&self) -> SnapshotSchemaFingerprint {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"ambition.ggrs-rollback-schema\0");
        hasher.update(&GGRS_ROLLBACK_SCHEMA_VERSION.to_le_bytes());
        let dump = self.mechanical_dump();
        hasher.update(&(dump.len() as u64).to_le_bytes());
        hasher.update(dump.as_bytes());
        SnapshotSchemaFingerprint::from_bytes(*hasher.finalize().as_bytes())
    }
}

pub fn descriptor<T: 'static>(
    owner: &'static str,
    name: &'static str,
    kind: RollbackEntryKind,
    detail: &'static str,
) -> RollbackRegistrationDescriptor {
    descriptor_owned::<T>(owner, name, kind, detail.to_string())
}

/// [`descriptor`] for a detail this crate COMPOSES rather than quotes.
///
/// The recorded `detail` is two halves: how the value is stored (the backend's
/// half — "bevy_ggrs clone snapshot") and what the checksum sees (the domain's
/// half). A domain that registers its own state through
/// [`ambition_platformer2d_core::snapshot::RollbackRegistrar`] supplies only the
/// second half, precisely so a crate with no `bevy_ggrs` dependency never has to
/// write the word; this joins them back into the exact string the schema baseline
/// records.
pub fn descriptor_owned<T: 'static>(
    owner: &'static str,
    name: &'static str,
    kind: RollbackEntryKind,
    detail: String,
) -> RollbackRegistrationDescriptor {
    RollbackRegistrationDescriptor {
        name: name.to_string(),
        owner: owner.to_string(),
        kind,
        type_name: std::any::type_name::<T>().to_string(),
        detail,
    }
}

/// Record one schema descriptor on an app, independent of the active rollback backend.
///
/// Backend installation deliberately has a separate idempotence authority: a row may
/// already have been recorded by a capability plugin before a concrete rollback host
/// installs its typed snapshot machinery. Therefore callers must not interpret an
/// `Idempotent` schema row as evidence that a backend registration already exists.
pub fn record_descriptor(
    app: &mut App,
    descriptor: RollbackRegistrationDescriptor,
) -> RollbackRegistrationOutcome {
    app.init_resource::<RollbackRegistry>();
    app.world_mut()
        .resource_mut::<RollbackRegistry>()
        .try_register(descriptor)
        .unwrap_or_else(|error| panic!("{error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⭐⭐ THE GUARD ON THE FINGERPRINT MEMO, AND IT IS THE WHOLE REASON THE MEMO
    /// IS SAFE. `schema_fingerprint` caches into a `OnceLock`; if `try_register`
    /// ever stops clearing it, this registry would keep reporting a fingerprint
    /// its own entries no longer justify — and a stale schema fingerprint is
    /// exactly what `enforce_session_contract` exists to catch, so the failure
    /// would be silent AND load-bearing.
    #[test]
    fn registering_after_reading_the_fingerprint_changes_it() {
        let mut registry = RollbackRegistry::default();
        registry
            .try_register(entry("a", "owner", "detail"))
            .expect("first registration");
        // Read it FIRST, so the memo is populated before the mutation.
        let before = registry.schema_fingerprint();
        // ...and reading twice must agree, or the memo is not a memo.
        assert_eq!(
            before,
            registry.schema_fingerprint(),
            "two reads with no mutation between them disagreed"
        );

        registry
            .try_register(entry("b", "owner", "detail"))
            .expect("second registration");
        assert_ne!(
            before,
            registry.schema_fingerprint(),
            "a registration after the fingerprint was memoised did not change it — \
             the memo is stale, and a stale schema fingerprint is what \
             enforce_session_contract cannot afford to be wrong about"
        );
    }

    /// A CLONE must not inherit a memo, because a clone that is then mutated
    /// would otherwise answer for entries it no longer has.
    #[test]
    fn a_clone_agrees_with_its_source_and_still_notices_its_own_changes() {
        let mut registry = RollbackRegistry::default();
        registry
            .try_register(entry("a", "owner", "detail"))
            .expect("registration");
        let _ = registry.schema_fingerprint();
        let mut copy = registry.clone();
        assert_eq!(
            registry.schema_fingerprint(),
            copy.schema_fingerprint(),
            "a clone of the same entries must fingerprint the same"
        );
        copy.try_register(entry("b", "owner", "detail"))
            .expect("registration on the clone");
        assert_ne!(
            registry.schema_fingerprint(),
            copy.schema_fingerprint(),
            "the clone changed and still reported the source's fingerprint"
        );
    }

    fn entry(name: &str, owner: &str, detail: &str) -> RollbackRegistrationDescriptor {
        RollbackRegistrationDescriptor {
            name: name.to_owned(),
            owner: owner.to_owned(),
            kind: RollbackEntryKind::Derived,
            type_name: "test::Type".to_owned(),
            detail: detail.to_owned(),
        }
    }

    #[test]
    fn schema_is_insertion_order_independent() {
        let mut a = RollbackRegistry::default();
        a.try_register(entry("z", "provider-b", "second")).unwrap();
        a.try_register(entry("a", "provider-a", "first")).unwrap();

        let mut b = RollbackRegistry::default();
        b.try_register(entry("a", "provider-a", "first")).unwrap();
        b.try_register(entry("z", "provider-b", "second")).unwrap();

        assert_eq!(a.deterministic_dump(), b.deterministic_dump());
        assert_eq!(a.schema_fingerprint(), b.schema_fingerprint());
    }

    #[test]
    fn identical_registration_is_idempotent() {
        let descriptor = entry("same", "provider", "same");
        let mut registry = RollbackRegistry::default();
        assert_eq!(
            registry.try_register(descriptor.clone()).unwrap(),
            RollbackRegistrationOutcome::Inserted
        );
        assert_eq!(
            registry.try_register(descriptor).unwrap(),
            RollbackRegistrationOutcome::Idempotent
        );
        assert_eq!(registry.descriptors().count(), 1);
    }

    fn typed_entry(name: &str, type_name: &str) -> RollbackRegistrationDescriptor {
        RollbackRegistrationDescriptor {
            name: name.to_owned(),
            owner: "test-owner".to_owned(),
            kind: RollbackEntryKind::Derived,
            type_name: type_name.to_owned(),
            detail: "test-only descriptor".to_owned(),
        }
    }

    fn registry_of(rows: &[(&str, &str)]) -> RollbackRegistry {
        let mut registry = RollbackRegistry::default();
        for (name, type_name) in rows {
            registry.try_register(typed_entry(name, type_name)).unwrap();
        }
        registry
    }

    /// Where a type LIVES is not part of the wire format (v20).
    ///
    /// Only the final segment survived either move.
    #[test]
    fn relocating_a_type_leaves_the_fingerprint_alone() {
        let before = registry_of(&[
            (
                "actor.anim_override",
                "ambition_platformer2d_actor_monolith::features::ecs::actor_clusters::ActorAnimOverride",
            ),
            (
                "player.blink_camera_state",
                "ambition_platformer2d_actor_monolith::avatar::components::PlayerBlinkCameraState",
            ),
        ]);
        let after = registry_of(&[
            (
                "actor.anim_override",
                "ambition_sprite_sheet::character::anim::ActorAnimOverride",
            ),
            (
                "player.blink_camera_state",
                "ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState",
            ),
        ]);
        assert_eq!(
            before.schema_fingerprint(),
            after.schema_fingerprint(),
            "moving a rollback-registered type to another crate and another \
             module moved the schema fingerprint. Nothing a peer can observe \
             changed, so two peers running byte-identical snapshot logic would \
             refuse to agree — which makes every carve in the decomposition \
             campaign a netplay compatibility break."
        );

        // POISON. Without it this test is equally green for a fingerprint that
        // hashes nothing about the type at all, and dropping `type_name` from
        // the dump entirely was a real alternative — it costs the last signal
        // that a DIFFERENT Rust type got registered under an existing name.
        let renamed = registry_of(&[
            (
                "actor.anim_override",
                "ambition_sprite_sheet::character::anim::ActorAnimOverride",
            ),
            (
                "player.blink_camera_state",
                "ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkEaseState",
            ),
        ]);
        assert_ne!(
            after.schema_fingerprint(),
            renamed.schema_fingerprint(),
            "a stable name that changed which TYPE it registers left the \
             fingerprint alone, so the dump is no longer hashing the type in \
             any form."
        );
    }

    /// What makes the narrower identity sound.
    ///
    /// Two `Cooldown`s in two crates hash equal once the final segment is the
    /// identity, so a peer holding them the other way round would be declared
    /// compatible. The second half asserts the guard is not merely strict: one
    /// type registered under two stable names is the ordinary case, and 39 of
    /// the live rows are it.
    #[test]
    fn two_types_sharing_a_final_segment_are_rejected_and_one_type_twice_is_not() {
        let mut registry = RollbackRegistry::default();
        registry
            .try_register(typed_entry(
                "ability.cooldown",
                "ambition_combat::ability::Cooldown",
            ))
            .unwrap();

        let error = registry
            .try_register(typed_entry(
                "weapon.cooldown",
                "ambition_projectiles::weapon::Cooldown",
            ))
            .unwrap_err();
        assert!(
            matches!(
                error,
                RollbackRegistrationError::TypeIdentityCollision { .. }
            ),
            "two different types whose names end in `Cooldown` were accepted, \
             and the fingerprint cannot tell them apart: {error}"
        );

        registry
            .try_register(typed_entry(
                "ability.cooldown_mirror",
                "ambition_combat::ability::Cooldown",
            ))
            .expect(
                "registering ONE type under a second stable name is not a \
                 collision — the stable name is what identifies a registration, \
                 and refusing this would reject 39 of the live rows",
            );
    }

    fn spelled(kind: RollbackEntryKind, detail: &str) -> RollbackRegistry {
        let mut registry = RollbackRegistry::default();
        registry
            .try_register(RollbackRegistrationDescriptor {
                name: "row".to_owned(),
                owner: "test-owner".to_owned(),
                kind,
                type_name: "test::Type".to_owned(),
                detail: detail.to_owned(),
            })
            .unwrap();
        registry
    }

    /// `Q122`: the fingerprint is the mechanical identity, so rewording a
    /// row's prose leaves it alone. A custom checksum's description and a
    /// derived row's reason are prose; the code they describe is what the
    /// schema version answers for.
    #[test]
    fn rewording_a_row_leaves_the_fingerprint_alone() {
        for (kind, before, after) in [
            (
                RollbackEntryKind::ResourceCloneCustomChecksum,
                "bevy_ggrs clone snapshot + quest identity, progression and pending-advance checksum projection",
                "bevy_ggrs clone snapshot + quest identity and progression checksum projection",
            ),
            (
                RollbackEntryKind::Derived,
                "republished every tick",
                "rebuilt from its sources every tick",
            ),
        ] {
            assert_eq!(
                spelled(kind, before).schema_fingerprint(),
                spelled(kind, after).schema_fingerprint(),
                "a {kind:?} row whose prose changed moved the fingerprint"
            );
        }
    }

    /// The other half: a row on a different MECHANISM of the same kind moves
    /// the fingerprint. An unhashed clone and a clone probed for localization
    /// are both `component-clone`, and only the road tells them apart.
    #[test]
    fn a_row_on_another_mechanism_moves_the_fingerprint() {
        use ambition_platformer2d_core::rollback_kind::spelling;
        let unhashed = spelling::COMPONENT_CLONE_UNHASHED;
        let probed = spelling::COMPONENT_CLONE_PROBED_FOR_LOCALIZATION;
        let set = spelling::COMPONENT_CLONE_ENTITY_SET_REMAPPED;
        assert_eq!((unhashed.kind, probed.kind, set.kind), (probed.kind, set.kind, unhashed.kind));
        let fingerprints = [unhashed, probed, set].map(|road| spelled(road.kind, road.detail).schema_fingerprint());
        assert!(
            fingerprints[0] != fingerprints[1] && fingerprints[1] != fingerprints[2] && fingerprints[0] != fingerprints[2],
            "three component-clone roads share a fingerprint"
        );
    }

    #[test]
    fn conflicting_registration_is_transactional() {
        let mut registry = RollbackRegistry::default();
        registry
            .try_register(entry("same", "provider-a", "old"))
            .unwrap();
        let before = registry.deterministic_dump();
        let error = registry
            .try_register(entry("same", "provider-b", "new"))
            .unwrap_err();
        assert!(matches!(error, RollbackRegistrationError::Conflict { .. }));
        assert_eq!(registry.deterministic_dump(), before);
    }
}
