//! The per-actor tuning vocabulary: numeric/flag facts the runtime loops read
//! each frame, resolved at spawn and carried on the actor's config component.
//!
//! ⛔ **it was the ARCHETYPE tuning vocabulary** — "authored per archetype
//! (`character_archetypes.ron`) and projected onto the enemy config component at
//! spawn". There are no archetypes; every value here is resolved from the
//! body's `CharacterBodyBlueprint`, its `BrainProfile`, or its placement. What
//! survives of the old sentence is the SHAPE: this is mostly a projection,
//! written once at construction.
//!
//! ⚠ **MOSTLY, and the exception is load-bearing**: `body_contact_damage` is
//! toggled per tick by Mary-O's snake shells, so this type does carry one fact
//! whose previous value decides the next frame. That is legal here only because
//! `ActorConfig` is rollback-registered; see the classification test below
//! before adding a second.
//!
//! Combat reads none of this — spawn projects the combat-relevant facts onto
//! `CombatTuning` (the legal actors → combat arrow).

use crate::combat::BodyMovementTuning;
use ambition_entity_catalog::placements::RespawnPolicy;

/// Per-actor numeric/flag tuning the RUNTIME combat loops read each frame,
/// resolved at spawn from the body's blueprint, its controller profile and its
/// placement (like [`CombatCapabilities`], but plain tuning rather than special
/// behaviors). Carried as a field on the actor's config component so the
/// per-frame systems never re-resolve content.
///
/// `Clone` (not `Copy`): the open `ranged_visual` id is an owned `String`.
#[derive(Clone, Debug, PartialEq)]
pub struct ActorTuning {
    /// Resolved movement physics for this body. The spine reads
    /// gravity/run/jump/fall from here, not constants.
    pub movement: BodyMovementTuning,
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
    /// **Does this body's driver seek the player.** Hostile bodies track and
    /// publish contact damage; peaceful patrollers are false. A PLACEMENT
    /// decision (`SpawnDisposition`), not a body fact — the same creature is
    /// ambient wildlife in one room and a threat in another.
    ///
    /// ⚠ **distinct from the `ActorDisposition` component**, which is the live,
    /// rewinding answer to *is this body fighting right now* and gates damage
    /// standing and dialogue. This one is the spawn policy the brain's
    /// aggressiveness is built from. ⛔ a dangling half-sentence about a
    /// deleted `attack_cooldown_mult` was stranded here; it moved to
    /// `BrainProfile` on 2026-08-13.
    pub is_hostile: bool,
    /// SPAWN-TIME policy selector: this body crawls surfaces glued to
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
    /// the default keeps every body that states no weight at the reference.
    pub weight: f32,
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
    /// CHARACTER (`CharacterDefinition::ranged_vfx`). The ranged-fire effects consumer stamps it onto the spawned
    /// shot so the render layer resolves art by id through the content-owned
    /// catalog (e.g. the PCA's `"glider"`) rather than by reading the owner-id
    /// string. The empty string is the generic orange shot.
    pub ranged_visual: String,
}

impl Default for ActorTuning {
    fn default() -> Self {
        Self {
            movement: BodyMovementTuning::default(),
            // `ActorTuning` keeps the DERIVED absolute speeds: this is the
            // body-space projection brains consume, not the authored row.
            patrol_speed: 0.0,
            chase_speed: 0.0,
            max_run_speed: 0.0,
            contact_strength: 0.0,
            damage_amount: 0,
            is_hostile: false,
            surface_walker: false,
            cling_breaks_on_hit: false,
            respawn: RespawnPolicy::default(),
            // Reference body: the default must not zero out the growth divisor.
            weight: 1.0,
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

    /// **EVERY FIELD OF `ActorTuning` HAS A DECLARED AUTHORITY**, and the
    /// compiler is what holds it rather than a number in a document.
    ///
    /// ⭐ **the columns are the CAMPAIGN's six, not the three this test was born
    /// with** (AC6.2). Its original split — character / controller / placement —
    /// was the right question for the migration that created it and the wrong
    /// one for the destination: it had to file `dream_seed` and `ranged_visual`
    /// under BODY with a note explaining that they are presentation, which is a
    /// classification admitting it does not fit. The plan's taxonomy has a
    /// column for them, so they sit in it.
    ///
    /// ⛔⛔ **a COUNT in prose cannot hold this and has now failed three times.**
    /// The acceptance list sized this type at 275 lines; a hand grep on
    /// 2026-08-13 reported 14 fields and this destructure immediately refuted it.
    /// Worse, the campaign row's placement/session column named `attacks_player`,
    /// which had been renamed to `is_hostile` — a field nobody could grep for, in
    /// a document nobody could tell was wrong.
    ///
    /// ⭐ **an exhaustive destructure does not rot.** Add a field and this stops
    /// COMPILING until somebody puts it in a column; remove one and the same.
    /// There is no number to edit and no census to redo.
    #[test]
    fn every_tuning_field_belongs_to_one_of_the_campaigns_authorities() {
        let ActorTuning {
            // ── REUSABLE CHARACTER FACT — what this body IS ─────────────────
            movement: _,
            max_run_speed: _,
            contact_strength: _,
            damage_amount: _,
            surface_walker: _,
            cling_breaks_on_hit: _,
            weight: _,
            is_aerial: _,
            flight_direct_velocity: _,
            // ── CONTROLLER POLICY, RESOLVED AGAINST THE BODY ────────────────
            //
            // ⭐ **both are PROJECTIONS, not a second authority.**
            // `ActorClusterSeed` computes `patrol_speed = run_speed *
            // brain_profile.patrol_effort` and the same for chase, so the
            // authority is `BrainProfile`'s normalized effort and these are what
            // it looks like once a body has been named. ⛔ `attack_cooldown_mult`
            // stood here and MOVED to the profile (2026-08-13), which is what
            // this column's remaining work turned out to be.
            patrol_speed: _,
            chase_speed: _,
            // ── PLACEMENT / SESSION — true of THIS instance, here ───────────
            //
            // ⚠ `is_hostile` reads as a body fact and is not one: the same
            // creature is ambient wildlife in one room and a threat in another,
            // which is why it left the archetype row.
            is_hostile: _,
            respawn: _,
            // ⚠ `is_sandbag` is the one genuinely ARGUABLE entry. It reads as a
            // body fact ("this creature is a training dummy") and behaves as a
            // session one: excluded from slot pressure and from save
            // persistence. It is filed here because both consequences are about
            // how the SESSION treats the instance, and because the body half is
            // already carried elsewhere under a truer name —
            // `CharacterDefinition::practice_target`, projected onto
            // `BodyCombat::training_dummy` at construction.
            is_sandbag: _,
            // ── PRESENTATION ────────────────────────────────────────────────
            //
            // Neither is a fact about how the body behaves: `dream_seed` is a
            // visual jitter pass and `ranged_visual` names the art its shot
            // wears. They rode in a gameplay bag because they were authored on
            // the same archetype row as the gameplay numbers.
            dream_seed: _,
            ranged_visual: _,
            // ── RUNTIME STATE — one entry, and I claimed there were none ────
            //
            // ⛔⛔ **`body_contact_damage` IS MUTATED PER TICK BY A SHIPPED GAME**,
            // and this column said "deliberately empty" until somebody read
            // Mary-O. `step_snake_shell` clears it when a stomped snake becomes a
            // shell and sets it again when the shell walks — *a shell is
            // harmless to touch* — so its PREVIOUS value decides whether the next
            // frame's contact hurts.
            //
            // ⚠ **legal, and only because `ActorConfig` is rollback-registered**
            // (`actor.config`, component-clone). A mutable gameplay fact on a
            // component that did NOT rewind would be a desync waiting for a
            // rollback; check that before adding a second one.
            //
            // ⭐ it is filed here rather than under CHARACTER FACT even though a
            // character authors the underlying `ContactDamage`, because what this
            // bool holds is the CURRENT answer, not the authored one — and the
            // two differ for exactly as long as a snake is a shell.
            body_contact_damage: _,
            // ── OBSOLETE / DEAD — empty, and it is checked ──────────────────
            //
            // ⛔⛔ `max_health` and `death_policy` WERE HERE and left for
            // `BodyHealth` (AC6.2), which already carried both for every body
            // including the player. The pool copy was written independently at
            // three construction sites; the policy copy was never set to anything
            // but the default, while the match road set the real one on
            // `BodyHealth` — so the actor damage gate asked the copy and could
            // read `HpDepleted` for a fighter playing under `Unbounded`.
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
