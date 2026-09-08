//! Actor read-model snapshots + the in-place hostile flip.
//!
//! Provoking a peaceful actor (an NPC struck past its retaliation threshold, or
//! a persisted-hostile NPC on load) no longer swaps clusters or churns the
//! entity: every actor is the SAME cluster, so the flip just re-resolves the
//! hostile archetype, overwrites the cluster `config` in place, swaps the
//! `Brain`/`ActionSet`, and flips `ActorDisposition` (the single source of
//! truth for hostility — "enemy" is a state, not a class).

// ⚠ Named for the two functions that came down from the feature layer; they had
// inherited these through a `use super::*` there.

// Named rather than inherited: this module used to sit under a `use super::*`
// that surfaced it from the reusable actor crate.
use ambition_characters::actor::BodyCombat;

use ambition_combat::components::{ActorDisposition, ActorIdentity};

// It began as a matcher over an id, a display name and a dialogue node — *does any of them
// contain "pirate"* — and handed the struck body a whole archetype.
//
// Both branches read their `BrainProfile` now — the policy that always owned both numbers — so
// `provoked_projection` derives the read-model with `config_brain_for` like every other road,
// and provocation names no roster key.
//
// its test-only twin `hostile_spec_for_actor` went with it: its whole purpose
// was to be the roster's side of an equivalence test against this function.

/// Build the read-model mirror components for an actor cluster seed at the given
/// disposition. Peaceful actors get a peaceful `BodyCombat`; hostile actors
/// the full hostile combat state.
pub fn actor_component_snapshot(
    seed: &ambition_body_seed::ActorClusterSeed,
    disposition: ActorDisposition,
) -> (ActorIdentity, ActorDisposition, BodyCombat) {
    // THE SEED'S OWN, not a rebuild (AC6.2). This constructed a fresh
    // `BodyCombat` and filled its one authored flag from
    // `ActorTuning::is_sandbag` — a copy of the character's `practice_target`
    // made at spawn so it could be copied AGAIN here. The seed decides a body's
    // components; this reads the one it decided.
    //
    // AC3.1.A: a fresh body's `BodyCombat` is its reaction history at rest plus
    // one authored flag. Liveness is `BodyHealth`'s, so a seed does not state it
    // here and cannot state it wrongly. AC3.1.D: the flag is authored, so it is
    // written once at construction rather than re-derived every frame by the
    // read-model sync — and the disposition gate that sync applied is
    // deliberately gone: a body authored as a training dummy is one whether or
    // not it currently reads as hostile.
    let combat = seed.combat.clone();
    (
        ActorIdentity::new(seed.config.id.clone(), seed.config.name.clone())
            .with_sprite_override(seed.config.sprite_override_npc_name.clone()),
        disposition,
        combat,
    )
}

/// Hostile spawn read-models (the common case for authored enemies).
pub fn enemy_component_snapshot(
    enemy: &ambition_body_seed::ActorClusterSeed,
) -> (ActorIdentity, ActorDisposition, BodyCombat) {
    actor_component_snapshot(enemy, ActorDisposition::Hostile)
}

// ⭐⭐ BOTH OF THESE CAME DOWN 2026-09-06, and the pair had to travel together.
// `provoked_projection` is consumed here and in `features::ecs::autonomous_reconcile`;
// moving it alone would have RELOCATED its edge rather than removed it, because it
// calls `config_brain_for` — which turned out to be a twelve-line pure leaf with no
// intra-crate reference of its own, filed in `features::brain_command` beside its
// first caller.
//

