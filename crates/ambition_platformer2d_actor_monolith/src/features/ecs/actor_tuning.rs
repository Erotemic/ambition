//! The actor ARCHETYPE tuning vocabulary (moved out of the combat kit at
//! E2): per-actor numeric/flag tuning + the generic brain-construction
//! inputs, authored per archetype (`character_archetypes.ron`) and projected
//! onto the enemy config component at spawn. Combat reads none of this —
//! spawn projects the combat-relevant facts onto `CombatTuning` (the legal
//! actors → combat arrow).

use crate::combat::{BodyMovementTuning, DeathPolicy};
use ambition_entity_catalog::placements::RespawnPolicy;

/// Per-actor numeric/flag tuning the RUNTIME combat loops read each
/// frame, derived from the actor's authored archetype DATA at spawn
/// (like [`CombatCapabilities`], but plain tuning rather than special
/// behaviors). Carried as a field on the enemy config component so
/// the per-frame systems never call back into a named archetype enum.
///
/// `Clone` (not `Copy`): the open `ranged_visual` id is an owned `String`.
#[derive(Clone, Debug, PartialEq)]
pub struct ActorTuning {
    /// Resolved movement physics for this body (composed from the archetype
    /// hierarchy). The spine reads gravity/run/jump/fall from here, not constants.
    pub movement: BodyMovementTuning,
    /// Full health pool at spawn / respawn-reset.
    pub max_health: i32,
    /// Patrol walking speed (px/s).
    pub patrol_speed: f32,
    /// Chase/steering speed (px/s).
    pub chase_speed: f32,
    /// Ground-run capability (px/s) — the fastest this body locomotes. Grounded
    /// brains emit a normalized throttle of it; the integrator scales velocity
    /// back as `locomotion * max_run_speed`, uniformly with the player.
    pub max_run_speed: f32,
    /// Contact-damage knockback strength.
    pub contact_strength: f32,
    /// Damage dealt by an attack / body contact.
    pub damage_amount: i32,
    /// Multiplier on the shared attack cooldown (fast skirmishers
    /// Hostile by default: actively tracks the player and publishes
    /// contact damage. Peaceful patrollers are false.
    pub is_hostile: bool,
    /// SPAWN-TIME policy selector: this archetype crawls surfaces glued to
    /// the surface normal (the adhesive-crawler movement policy). Consumed
    /// once by [`Self::motion_model`]; runtime dispatch reads the body's
    /// explicit `MotionModel`, never this flag.
    pub surface_walker: bool,
    /// Surface-walker only: a hit knocks the actor off its surface (it
    /// falls with gravity for a moment, then re-attaches). `false` keeps
    /// it clinging when struck.
    pub cling_breaks_on_hit: bool,
    /// When this defeated actor reappears (ADR 0022) — the ONE authored
    /// respawn policy. `InPlace(secs)` self-revives where it stood
    /// (finite training dummies); the flag-writing arms are consumed by
    /// the kill hook; DEFAULT: dead stays dead.
    pub respawn: RespawnPolicy,
    /// Knockback weight (CM1): heavier bodies launch less under the same growth
    /// term (`knockback_growth * damage_taken / weight`). `1.0` is the reference body;
    /// the default keeps every un-authored archetype at the reference.
    pub weight: f32,
    /// How this body's damage meter relates to death (CM1). `HpDepleted`
    /// (default) dies at pool max; `Unbounded` is smash percent — death comes
    /// from the blast-zone/OOB gate, not the meter.
    pub death_policy: DeathPolicy,
    /// Flies: no gravity, aerial slot class.
    pub is_aerial: bool,
    /// Direct-velocity free-mover: the brain commands an EXACT velocity each tick
    /// (a boss pattern's `desired_vel`), so the shared flight limb takes it verbatim
    /// (no accel ramp / drag / deadzone) — byte-identical to the old bespoke SNAP
    /// float. Threaded into the engine `MovementTuning.flight_direct_velocity`
    /// (archetype swap AS4). Ordinary flyers (parrot) leave this false for smoothed
    /// flight.
    pub flight_direct_velocity: bool,
    /// Training-dummy family: excluded from slot pressure and save
    /// persistence.
    pub is_sandbag: bool,
    /// Touching this actor's body hurts the player.
    pub body_contact_damage: bool,
    /// Deep-dream visual jitter seed; `None` = no dream pass.
    pub dream_seed: Option<f32>,
    /// Open visual id of this actor's ranged projectile, authored on the
    /// archetype. The ranged-fire effects consumer stamps it onto the spawned
    /// shot so the render layer resolves art by id through the content-owned
    /// catalog (e.g. the PCA's `"glider"`) rather than by reading the owner-id
    /// string. The empty string is the generic orange shot.
    pub ranged_visual: String,
}

impl Default for ActorTuning {
    fn default() -> Self {
        Self {
            movement: BodyMovementTuning::default(),
            max_health: 0,
            // `ActorTuning` keeps the DERIVED absolute speeds: this is the
            // body-space projection brains consume, not the authored row.
            patrol_speed: 0.0,
            chase_speed: 0.0,
            max_run_speed: 0.0,
            contact_strength: 0.0,
            damage_amount: 0,
            // Multiplicative identity — a defaulted tuning must not
            // zero out the shared attack cooldown.
            is_hostile: false,
            surface_walker: false,
            cling_breaks_on_hit: false,
            respawn: RespawnPolicy::default(),
            // Reference body: the default tuning must not zero out the growth
            // divisor, and every un-authored archetype dies at pool max.
            weight: 1.0,
            death_policy: DeathPolicy::default(),
            is_aerial: false,
            flight_direct_velocity: false,
            is_sandbag: false,
            body_contact_damage: false,
            dream_seed: None,
            ranged_visual: String::new(),
        }
    }
}

impl ActorTuning {
    /// Slot class this actor requests from the combat slot board.
    pub fn slot_kind(&self) -> crate::combat::slots::SlotKind {
        if self.is_aerial {
            crate::combat::slots::SlotKind::Aerial
        } else {
            crate::combat::slots::SlotKind::Melee
        }
    }

    // ⛔⛔ **`adopting_archetype` WAS HERE, AND THE CONCEPT DIED RATHER THAN THE
    // HELPER** (2026-08-12). It existed so a projected archetype tuning could be
    // assigned WHOLESALE onto a provoked body while the placement kept the one
    // field it owns — respawn policy (ADR 0022). Assigning it wholesale dragged
    // a mob's `OnRoomReenter` onto a named NPC, the kill hook then wrote no
    // death flag, and the NPC was rebuilt alive by the next room construction.
    // That was a live bug, and this function was its fix.
    //
    // ⭐ provocation projects NO tuning at all now (ledger D101): it changes a
    // mind and a kit, never a body. So the respawn policy survives for the same
    // reason the run speed does, and there is no wholesale assignment left to
    // protect a field from. Its last caller went with
    // `project_provoked_archetype` (D104).
    //
    // ⚠ compiler-verified dead by `#[deprecated]` + `cargo check --workspace
    // --all-targets`, not by a grep — see ledger D105 for why that distinction
    // has mattered six times this run.
}

/// Generic kit vocabulary for an archetype's brain.
///
/// ⚠ **DEFINED in `ambition_characters::brain`** since 2026-08-03. Its own doc
/// already said the brain module is the universal-actor abstraction; it was
/// merely LOCATED here. It moved so `character_archetypes.ron`'s authored
/// vocabulary could leave this crate, which is what let the content compiler
/// own the enemy-roster family without linking a renderer.
pub use ambition_characters::brain::CharacterBrainTemplate;

/// **The reusable autonomous-controller profile** — DEFINED in
/// `ambition_characters::brain::profile`, and it is not this crate's type.
///
/// ⭐ **it replaced `BrainProfile` outright** (campaign 2026-08-11, the
/// controller authority). That struct was the same idea one crate too high and
/// one concept too narrow: a *projection* of an archetype, reachable only by
/// having an archetype, so two characters that fight alike could not share a
/// policy without sharing a body. [`BrainProfile`] is authorable, reusable, and
/// carries the three CharacterAI knobs that used to sit in [`ActorTuning`]
/// (`aggro_radius`, `attack_range`, `turns_at_walls`) — those are decisions a
/// DRIVER makes, not facts a body states.
pub use ambition_characters::brain::BrainProfile;

impl ActorTuning {
    /// The explicit movement policy this archetype's bodies carry from spawn.
    ///
    /// Crawler archetypes (`surface_walker`) select the adhesive-crawler policy
    /// with their patrol speed as the crawl speed; everything else starts
    /// axis-swept with its authored body tuning (integration refreshes those
    /// parameters live each tick).
    pub fn motion_model(&self) -> crate::features::MotionModel {
        if self.surface_walker {
            crate::features::MotionModel::adhesive_crawler(
                ambition_platformer2d_core::CrawlerParams {
                    crawl_speed: self.patrol_speed,
                    max_fall_speed: self.movement.max_fall_speed,
                },
            )
        } else {
            crate::features::MotionModel::axis_swept(
                self.movement
                    .body_tuning(self.max_run_speed)
                    .axis_swept_params(),
            )
        }
    }
}

#[cfg(test)]
mod authority_split_tests {
    use super::*;

    /// **EVERY FIELD OF `ActorTuning` HAS A DECLARED AUTHORITY** — campaign
    /// P2.19, enforced by the compiler rather than by a number in a document.
    ///
    /// The campaign's central rule is that every migrated fact lands in exactly
    /// ONE of three authorities: the character definition (what a body IS), the
    /// controller profile (how a mind drives it), or the placement/session
    /// ruleset (what is true of this instance here). `ActorTuning` is the legacy
    /// bag those facts are being split OUT of, so the useful invariant is not
    /// its size — it is that nothing sits in it unclassified.
    ///
    /// ⛔⛔ **a COUNT in prose cannot hold this and has now failed three times.**
    /// The acceptance list sized this type at 275 lines; a hand grep on
    /// 2026-08-13 reported 14 fields and this destructure immediately refuted it
    /// — there are 20, and `is_sandbag`, which that grep called deleted, is one
    /// of them. Worse, the campaign row's
    /// placement/session column named `attacks_player`, which was renamed to
    /// `is_hostile` — a field nobody could grep for, in a document nobody could
    /// tell was wrong.
    ///
    /// ⭐ **an exhaustive destructure does not rot.** Add a field and this stops
    /// COMPILING until somebody puts it in a column; remove one and the same.
    /// There is no number to edit and no census to redo.
    #[test]
    fn every_tuning_field_belongs_to_one_of_the_three_authorities() {
        let ActorTuning {
            // ── BODY (13) — intrinsic, belongs on the `CharacterDefinition` ──
            movement: _,
            max_health: _,
            max_run_speed: _,
            contact_strength: _,
            damage_amount: _,
            surface_walker: _,
            cling_breaks_on_hit: _,
            weight: _,
            is_aerial: _,
            flight_direct_velocity: _,
            body_contact_damage: _,
            // ⚠ **PRESENTATION, riding in a gameplay bag.** Neither is a fact
            // about how the body behaves — `dream_seed` is a visual jitter pass
            // and `ranged_visual` names the art its shot wears. They are filed
            // under BODY because a body is what they are a projection OF, and
            // they leave with it; the campaign's rule is one authority per fact,
            // and presentation observing a body is not a fourth one.
            dream_seed: _,
            ranged_visual: _,
            // ── CONTROLLER (0) — how a mind paces the body ──────────────────
            //
            // ⭐ **two of these three have ALREADY MOVED and are now DERIVED,**
            // which is worth saying because the campaign row calls this "the
            // column that has not moved at all" and I repeated it here before
            // checking. `ActorClusterSeed` computes
            // `patrol_speed = run_speed * brain_profile.patrol_effort` and the
            // same for chase — so the AUTHORITY is `BrainProfile`'s normalized
            // effort and these two are its resolved projection against a body.
            // A projection is not a second authority; it is what "one authority"
            // looks like once something reads it.
            patrol_speed: _,
            chase_speed: _,
            // ⛔ **`attack_cooldown_mult` USED TO BE HERE and is gone**
            // (2026-08-13): it moved to `BrainProfile`, which is what this
            // column's remaining work turned out to be. The column is now empty
            // — every controller fact `ActorTuning` held is either on the
            // profile or is the profile's own effort resolved against a body.
            // ── PLACEMENT / SESSION (4) — true of THIS instance, here ──
            //
            // ⚠ `is_hostile` reads as a body fact and is not one: the same
            // creature is ambient wildlife in one room and a threat in another,
            // which is why it left the archetype row.
            is_hostile: _,
            respawn: _,
            death_policy: _,
            // ⚠ `is_sandbag` is the one genuinely ARGUABLE entry. It reads as a
            // body fact ("this creature is a training dummy") and behaves as a
            // session one: excluded from slot pressure and from save
            // persistence. It is filed here because both consequences are about
            // how the SESSION treats the instance, and because the character
            // road already carries the body half under a different name —
            // `practice_target`, asserted by
            // `enemy_roster::tests::practice_target_characters_do_not_strike_back`.
            is_sandbag: _,
        } = ActorTuning::default();

        // The destructure above is the assertion. This one only states the
        // consequence, so a reader who lands here from a compile error knows
        // what is being asked of them.
        assert!(
            true,
            "if this file failed to compile, a field was added to or removed \
             from `ActorTuning` — put it in one of the three columns above, or \
             establish that it belongs in none and delete it"
        );
    }
}
