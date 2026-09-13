//! Interaction, pickup, chest, and breakable building blocks.
//!
//! These are reusable mechanics/data components. The Bevy sandbox can render
//! prompts and play animations, but the identity and gameplay semantics belong
//! in the engine so later story crates can share them.

use ambition_characters::actor::Health;
// `Aabb` was reached through `ambition_platformer2d_core`, where it is
// `pub type Aabb = Aabb2d` — a `bevy_math` type. Depending on a crate named
// for one genre to borrow a maths type made this crate's unqualified name a
// lie; it is the only thing it took from there.
use ambition_entity_catalog::placements::HazardRespawn;
use bevy_math::bounding::Aabb2d as Aabb;

/// A player-facing interaction trigger.
///
/// ⛔⛤ **`requires_facing: bool` IS GONE, 2026-09-12.** It was authored on
/// `InteractableSpec`, threaded in by `spawn_static.rs`, set explicitly by
/// content — and consulted by NO production code, so an interactable declaring
/// it must be faced could be used from behind. A public, serializable,
/// documented authoring field that advertises a rule the engine does not
/// implement, and a third-party provider had no way to infer that.
///
/// ⇒ **DELETED, NOT LEFT AS A DESIGN CALL.** This comment used to end *"wiring
/// it or deleting it is a design call"*, and that framing is what kept three
/// no-op fields alive in one crate. **Deciding what the future feature should do
/// is a different question from making the field impossible to misuse**, and
/// only the second one was blocked on anything. Whoever wants facing-gated
/// interaction adds the field and the CHECK together.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Interactable {
    pub id: String,
    pub prompt: String,
    pub aabb: Aabb,
    pub kind: InteractionKind,
    pub enabled: bool,
}

impl Interactable {
    pub fn new(
        id: impl Into<String>,
        prompt: impl Into<String>,
        aabb: Aabb,
        kind: InteractionKind,
    ) -> Self {
        Self {
            id: id.into(),
            prompt: prompt.into(),
            aabb,
            kind,
            enabled: true,
        }
    }
}

/// What an interactable does when activated.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum InteractionKind {
    Door {
        target: Option<String>,
    },
    Npc {
        /// Catalog `character_id` this NPC was authored from (LDtk `NpcSpawn`
        /// field). Carries the catalog join through to spawn so the runtime can
        /// resolve the character's `default_brain` + `body_kind` from data.
        /// `None` for NPCs built without a catalog row (legacy / synthetic
        /// spawns) — those get a plain stand-still brain and no brain binding.
        character_id: Option<String>,
        dialogue_id: Option<String>,
        /// Half-range of a *selected* patrol preset's pace, in world pixels. This
        /// is a PARAMETER a chosen patrol brain preset consumes for its lane
        /// radius — it does NOT select whether the NPC receives a patrol brain
        /// (the explicit `brain_override` / catalog `default_brain` does). `0.0`
        /// (the default) leaves the selected patrol preset's authored radius.
        /// Ignored by every non-patrol preset. The NPC still stops inside the
        /// player's `talk_radius` so dialog is reachable.
        patrol_radius: f32,
        /// Optional authored `KinematicPathSpec` lookup id, threaded to a
        /// selected patrol preset that supports one. Like `patrol_radius`, a
        /// parameter for a chosen patrol brain — never a brain selector.
        patrol_path_id: Option<String>,
        /// Explicit initial brain preset override (a `brain_presets` key). `None`
        /// / empty means use the character's catalog `default_brain`. This is the
        /// authored brain SELECTION; the runtime resolves it via
        /// `ambition_characters`'s `resolve_initial_brain` without ever
        /// inspecting the resulting brain.
        #[serde(default)]
        brain_override: Option<String>,
    },
    Chest,
    Pickup,
    Breakable,
    Custom(String),
}

/// Collectible object semantics.
///
/// ⛔⛤ **`collected: bool` IS GONE, 2026-09-12.** This comment used to say it was
/// *"not the authority and nothing reads it"*, note that it was the same shape as
/// `Chest::persistent`, and leave wiring-or-deleting as a design call. Leaving it
/// there was the wrong half of that choice: `Pickup::collected` was a SECOND
/// REPRESENTATION of a fact whose only live authority is the
/// `ambition_combat::components::Collected` marker — the one `pickups.rs` inserts
/// and queries as `Without<Collected>` — and a spec that said `collected: true`
/// produced a pickup that was still there to be taken.
///
/// ⇒ **One fact, one recorder.** The marker. Deleting the field is what makes a
/// reader unable to trust it, which is stronger than a comment asking them not
/// to. See `PickupSpec` for what was measured before the deletion.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Pickup {
    pub id: String,
    pub kind: PickupKind,
    pub respawn: HazardRespawn,
}

impl Pickup {
    pub fn new(id: impl Into<String>, kind: PickupKind) -> Self {
        Self {
            id: id.into(),
            kind,
            respawn: HazardRespawn::Never,
        }
    }
}

/// The reward/effect represented by a pickup or chest.
///
/// Defined in `ambition_entity_catalog` and re-exported here for callers that
/// name rewards through the interaction API. Keeping the type below both
/// interaction and character domains avoids a dependency cycle.
pub use ambition_entity_catalog::PickupKind;

/// Treasure chest reward and persistence policy. Chests are interactables plus
/// persistence.
///
/// ⛔⛤ **IT NO LONGER CARRIES A `state`, AND THAT FIELD WAS A SECOND RECORDER
/// NOTHING READ.** `Chest::state: ChestState` was authored (`ChestStateSpec`),
/// lowered at spawn (`chest_state_from_spec`), stored here — and read by NOTHING
/// in production. MEASURED 2026-09-12 tree-wide: its only two reads were
/// assertions in this crate's own tests.
///
/// ⇒ The live authority for *"is this chest opened"* is the
/// `ambition_combat::Opened` MARKER COMPONENT — inserted and removed by
/// `ambition_boss_encounter::rewards`, rollback-registered as `feature.opened`,
/// and read by `ambition_sim_view`'s proximity affordance. One fact, and now one
/// recorder.
///
/// ⚠ THE LATENT DEFECT THIS REMOVES, from packet A5's census: an authored
/// `ChestStateSpec::Opened` produced a `Chest` whose `state` said `Opened` while
/// the runtime, which gates on the marker, treated it as closed — **so its reward
/// was grantable again.** It was unreachable only because LDtk's `ChestSpawn`
/// declares just `name` and `reward`; nothing about the code prevented it. See
/// the trip-wire in `chest_from_authored`.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Chest {
    pub id: String,
    pub reward: Option<PickupKind>,
}

impl Chest {
    pub fn new(id: impl Into<String>, reward: Option<PickupKind>) -> Self {
        Self {
            id: id.into(),
            reward,
        }
    }
}


/// What causes a breakable to break.
///
/// Replaces an earlier magic-string check that decided "stand-to-crumble" by
/// substring-matching on the entity name/id. Authors now pick the trigger
/// explicitly per LDtk entity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum BreakableTrigger {
    /// Only player attacks deal damage (default; original behavior).
    #[default]
    OnHit,
    /// Crumbles after the player stands on it for a short window.
    /// Stand-to-crumble requires the breakable to contribute non-`None`
    /// collision while intact (see [`BreakableCollision`]).
    OnStand,
    /// Either trigger applies.
    Either,
}

impl BreakableTrigger {
    pub fn allows_hit(self) -> bool {
        matches!(self, BreakableTrigger::OnHit | BreakableTrigger::Either)
    }

    pub fn allows_stand(self) -> bool {
        matches!(self, BreakableTrigger::OnStand | BreakableTrigger::Either)
    }
}

/// What kind of collision a breakable contributes while it is still intact.
///
/// Replaces the older `solid: bool` knob with a typed shape so authoring
/// tooling (LDtk Surface) can compile down a single rectangular volume into
/// either a hard wall, a one-way landing, or a pure trigger volume.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum BreakableCollision {
    /// Pure trigger volume: damage/contact events apply, but the player passes
    /// through it. Useful for breakable scenery that does not block movement.
    #[default]
    None,
    /// Hard collision on both axes while intact (legacy `solid: true`).
    Solid,
    /// One-way landing platform while intact: solid only when crossed from above.
    OneWayUp,
}

impl BreakableCollision {
    /// True if the breakable currently blocks movement on any axis.
    pub fn blocks_movement(self) -> bool {
        !matches!(self, BreakableCollision::None)
    }

    /// True if the breakable presents a hard wall while intact.
    pub fn is_solid(self) -> bool {
        matches!(self, BreakableCollision::Solid)
    }
}

/// Breakable wall/platform/object semantics.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Breakable {
    pub id: String,
    pub state: BreakableState,
    pub health: Health,
    pub respawn: HazardRespawn,
    /// Collision shape contributed while the breakable is intact.
    pub collision: BreakableCollision,
    pub trigger: BreakableTrigger,
    pub debris_cue: Option<String>,
    /// True for breakable pogo orbs: while intact the breakable contributes
    /// a `BlockKind::PogoOrb` to the collision world, and each successful
    /// pogo bounce damages it. Doesn't change collision/trigger semantics.
    pub pogo_refresh: bool,
}

impl Breakable {
    pub fn new(id: impl Into<String>, max_hp: i32) -> Self {
        Self {
            id: id.into(),
            state: BreakableState::Intact,
            health: Health::new(max_hp),
            respawn: HazardRespawn::Never,
            collision: BreakableCollision::None,
            trigger: BreakableTrigger::OnHit,
            debris_cue: None,
            pogo_refresh: false,
        }
    }

    /// ⚠ `#[must_use]`: TRUE MEANS IT BROKE THIS CALL, and breaking is an event
    /// with consequences a caller owes -- debris, a gate opening, an occurrence
    /// recorded. Dropping it damages the feature and drops the moment it died.
    #[must_use = "true means the breakable BROKE on this call: the caller owes \
                  the break its consequences"]
    pub fn apply_damage(&mut self, amount: i32) -> bool {
        let broke = self.health.damage(amount);
        if broke {
            self.state = BreakableState::Broken;
        } else if self.health.current < self.health.max {
            self.state = BreakableState::Cracking;
        }
        broke
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BreakableState {
    Intact,
    Cracking,
    Broken,
    Respawning,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breakable_moves_through_cracking_to_broken() {
        let mut block = Breakable::new("test", 4);
        assert!(!block.apply_damage(1));
        assert_eq!(block.state, BreakableState::Cracking);
        assert!(block.apply_damage(3));
        assert_eq!(block.state, BreakableState::Broken);
    }

    #[test]
    fn breakable_trigger_predicates() {
        assert!(BreakableTrigger::OnHit.allows_hit());
        assert!(!BreakableTrigger::OnHit.allows_stand());
        assert!(!BreakableTrigger::OnStand.allows_hit());
        assert!(BreakableTrigger::OnStand.allows_stand());
        assert!(BreakableTrigger::Either.allows_hit());
        assert!(BreakableTrigger::Either.allows_stand());
    }

    #[test]
    fn breakable_collision_predicates() {
        assert!(!BreakableCollision::None.blocks_movement());
        assert!(BreakableCollision::Solid.blocks_movement());
        assert!(BreakableCollision::OneWayUp.blocks_movement());
        assert!(!BreakableCollision::None.is_solid());
        assert!(BreakableCollision::Solid.is_solid());
        // OneWayUp is "blocks movement" but not strictly Solid in the
        // hard-wall sense — it only stops the falling player from above.
        assert!(!BreakableCollision::OneWayUp.is_solid());
    }

    #[test]
    fn breakable_default_state_is_intact() {
        let block = Breakable::new("test", 1);
        assert_eq!(block.state, BreakableState::Intact);
        assert_eq!(block.collision, BreakableCollision::None);
        assert_eq!(block.trigger, BreakableTrigger::OnHit);
        assert!(!block.pogo_refresh);
    }

    #[test]
    fn a_chest_carries_its_reward() {
        // `reward` is propagated as given (None for empty chests / triggers).
        //
        // ⛔⛤ **`persistent` WAS THE SECOND WRITE-ONLY FIELD IN THIS STRUCT AND
        // IS NOW GONE TOO, 2026-09-12.** The note that stood here said `state`
        // was deleted because it had a LATENT DEFECT attached while `persistent`
        // was *"merely dead"*, so removing it would be "a product-surface change
        // with no correctness argument behind it" — recorded rather than done.
        //
        // ⇒ **THAT DISTINCTION WAS THE WRONG ONE.** A serializable, documented
        // authoring field that decides nothing is a FALSE CAPABILITY in the
        // engine's API whether or not a second live authority contradicts it:
        // `persistent: false` on an authored chest changed nothing, and the
        // comment above it used to claim *"so the save system records them
        // automatically"*. What actually remembers an opened chest is
        // `encounter_reward_looted_flag`, which never consulted this field.
        //
        // ⚠ And deciding what per-chest persistence SHOULD do is a different
        // question from making the field impossible to misuse; only the second
        // was ever blocked on anything.
        let chest = Chest::new("hub_chest", Some(PickupKind::Health { amount: 2 }));
        // ⛔ THE REWARD IS THE FACT THIS TYPE CARRIES, AND IT IS ASSERTED ONCE.
        // "Is it open" lives on the `Opened` marker; the `state` assertions that
        // used to stand here were this struct's only readers of a field nothing
        // in production read, so they went with it rather than being repointed —
        // a repointed assertion would have restated `reward`, which the next
        // line already pins.
        assert_eq!(chest.reward, Some(PickupKind::Health { amount: 2 }));

        let empty = Chest::new("decoration", None);
        assert!(empty.reward.is_none());
    }
}
