//! Rollback declarations for state owned by this crate.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;
const OWNER: &str = env!("CARGO_PKG_NAME");

/// Register this domain's rollback state.
pub fn register_rollback_state<R: RollbackRegistrar>(registrar: &mut R) {    // ⛔⛔ TWO AUTONOMOUS EMITTERS THAT WERE NOT IN THE ROLLBACK VOCABULARY AT
    // ALL — not inert, ABSENT. A turret and a singularity are both spawned
    // mid-match by an ability, both outlive the frame that made them, and both
    // carry timers that decide when the world changes: the sentry's cooldown
    // decides which tick a bolt is emitted on, the well's `remaining_s` decides
    // how long every body in radius keeps being pulled. Nothing saved either, so
    // a rewind across a deployment kept the future's turret, its future
    // cooldown, and the shots that cooldown had already authorized.
    //
    // ⚠ THE ANCHOR IS WHY THE COVERAGE SWEEP NEVER SAID SO. A one-shot census
    // walks the entities a booted room HAS; a turret exists only after somebody
    // fires, so its absence from the registry read exactly like a pass. See the
    // scenario sweep this landed with.
    //
    // The turret and the well are now module entities (2026-10-01): their
    // position and lifetime are `ModuleEntity`, and the turret's cooldown and
    // the well's clock are module records, in the extension host's
    // `BodyRecords` on the same entity.
    registrar.require_rollback::<crate::module_entity::ModuleEntity>(
        OWNER,
        "entity:module_entity",
    );
    // `kind` is in the value: two entities at one place with one lifetime and
    // a different kind are a different world. So is the age: a mine whose
    // lifetime is too long for a step to register on still ages.
    registrar.rollback_component_clone_probed::<crate::module_entity::ModuleEntity>(
        OWNER,
        "ability.module_entity",
        |entity| {
            let kind = entity
                .kind
                .bytes()
                .fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
                (h ^ b as u64).wrapping_mul(0x0100_0000_01b3)
            });
            ((entity.pos.x.to_bits() as u64) << 32)
                ^ (entity.pos.y.to_bits() as u64)
                ^ ((entity.remaining_s.to_bits() as u64) << 16)
                ^ ((entity.age_s.to_bits() as u64) << 8)
                ^ kind
        },
    );
    // A course a module holds on a body (`ambition.body.hold_course`): it
    // decides where the body goes each tick. Both axes are in the value.
    registrar.rollback_component_clone_probed::<crate::ride::HeldCourse>(
        OWNER,
        "ability.held_course",
        crate::ride::held_course_probe,
    );
    registrar.declare_rollback_derived_resource::<crate::module_entity::ModuleEntityTouchers>(
        OWNER,
        "derived.module_entity_touchers",
        "written again in each tick's module_entity_tick collect and read only by that tick's lowering",
    );
    // ⛔⛔ EVENT-CREATED AUTHORITATIVE STATE, AND THAT IS WHY IT WAS MISSING. The
    // boot census can only see components that exist in the INITIAL world; every
    // component below is inserted later, by an ability firing, so nothing
    // structural ever asked whether it rewinds. Four gameplay decisions were
    // being made from state a rewind left at its future value.
    //
    // ⭐ PROBED, NOT PRESENCE-ONLY. Each of these is a NUMBER that decides an
    // outcome — where a recall puts a body, which tick a bomb goes off — and a
    // presence probe satisfies the coverage oracle while seeing none of it.
    registrar
        .rollback_component_clone_probed::<crate::traversal::mark_recall::PlayerMark>(
            OWNER,
            "ability.player_mark",
            // WHERE the mark is, not merely that one exists. Recall teleports to this
            // position, so a rewind across setting or moving the mark that kept the
            // future position puts the body somewhere the resimulation never chose.
            |mark| match mark.pos {
                Some(pos) => {
                    ((pos.x.to_bits() as u64) << 32) ^ (pos.y.to_bits() as u64) ^ 0x9e37_79b9
                }
                None => 0,
            },
        );
    registrar.rollback_component_clone_probed::<crate::ability_cooldown::AbilityCooldown>(
        OWNER,
        "ability.cooldown",
        // The time left, because it decides whether the next blink or grapple
        // fires. The first use inserts it, so a rewind across that use must
        // remove it again.
        |cooldown| cooldown.remaining.to_bits() as u64,
    );
    registrar.rollback_component_clone_probed::<crate::ranged::bomb::BombFuse>(
        OWNER,
        "ability.bomb_fuse",
        // The countdown, because WHICH TICK it reaches zero is the explosion.
        |fuse| fuse.timer.to_bits() as u64,
    );
    registrar
        .rollback_component_clone_probed::<crate::thrown::gravity_grenade::GravityGrenadeFuse>(
            OWNER,
            "ability.gravity_grenade_fuse",
            // Same shape as the bomb: the tick this reaches zero is the tick a
            // gravity well opens, and a well moves every body inside it.
            |fuse| fuse.timer.to_bits() as u64,
        );

}
