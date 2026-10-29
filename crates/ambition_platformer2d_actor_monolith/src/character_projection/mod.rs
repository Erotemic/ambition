//! Apply prepared character mechanics to live bodies.

use bevy::prelude::*;
use ambition_platformer2d_shared_tangle::construction::EntityScope;
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
use ambition_platformer2d_actor_spawn::character_body::{
    grant_prepared_character_body, KitOwnership, ProjectedCharacterKit,
};

/// Project prepared character-authored combat/presentation facts onto bodies.
///
/// Identity is the worn character. Prepared registry
/// values override older catalog-derived facts where authored. Change detection
/// makes the projection replay safely after rollback recreation and character
/// replacement. Live vitals are intentionally not projected here.
pub fn project_prepared_character_definitions(sprite_content: bevy::prelude::Res<ambition_sprite_sheet::SpriteContent>,
    mut commands: Commands,
    // The session's cast (`SessionCast`), not the App's: a cast a reload
    // publishes reaches the bodies when its generation is activated.
    cast: ambition_characters::prepared::SessionCast,
    changed_bodies: Query<
        (
            Entity,
            Option<&ambition_characters::actor::WornCharacter>,
            Option<&ProjectedCharacterKit>,
        ),
        Or<(
            Changed<ambition_characters::actor::WornCharacter>,
            Added<ambition_combat::CombatTuning>,
        )>,
    >,
    // The bodies a CAST REPLACEMENT invalidates. (H6)
    //
    // A new cast changes nothing on a body, so the change-detection query above
    // cannot see one — the worn id is still the worn id. That is exactly how a
    // body kept a retired cast's moves with every check green. This query is
    // The bodies `apply_worn_character_gameplay` can actually see.
    //
    // Its required columns, spelled out, because "does the derive match this
    // entity" has no shorter honest form. `WornCharacter` looks like the answer
    // and is not — it REQUIRES `IdentityKit`, so gating on either one silently
    // means "does it wear a character", which is a different and wrong question:
    // a hand-assembled body wearing a character without a moveset column matches
    // neither writer and would get no kit at all while reading as covered.
    //
    // this list is coupled to that system's query.
    persona_bodies: Query<
        Entity,
        (
            With<ambition_characters::actor::WornCharacter>,
            With<Name>,
            With<ambition_characters::brain::ActionSet>,
            With<ambition_combat::moveset::ActorMoveset>,
            With<ambition_characters::brain::action_set::IdentityKit>,
            With<ambition_platformer2d_core::BodyAbilities>,
            With<ambition_platformer2d_core::movement::MotionModel>,
        ),
    >,
    // walked ONLY on the tick the registry changes, which is startup and hot
    // reload; the ordinary tick still pays only for identities that moved.
    all_bodies: Query<(
        Entity,
        Option<&ambition_characters::actor::WornCharacter>,
        Option<&ProjectedCharacterKit>,
    )>,
    // The hand a re-granted kit is folded with.
    held: Query<&ambition_combat::held_items::HeldItem>,
    // Which way a retracted sprite body's feet face, for standing it back in
    // the box it displaced — the body's own frame, as the pose pass reads it.
    frames: Query<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
    #[cfg(feature = "portal")] guns: Query<&ambition_portal2d::PortalGun>,
) {
    let Some(registry) = cast.get() else {
        return;
    };
    let candidates: Vec<_> = if cast.is_changed() {
        all_bodies.iter().collect()
    } else {
        changed_bodies.iter().collect()
    };
    for (entity, worn, projected) in candidates {
        let character_id = worn.map(ambition_characters::actor::WornCharacter::id);
        let resolved = character_id.filter(|id| registry.get(id).is_some());
        let unchanged = projected.is_some_and(|projected| {
            Some(projected.id.as_str()) == resolved && projected.generation == registry.generation()
        });
        if unchanged || (projected.is_none() && resolved.is_none()) {
            continue;
        }
        // Looked up by the recorded id, so the removal is exactly what this system granted and
        // never something the spawn seeded.
        //
        // Do not retract `ActorMoveset`: every repertoire-bearing body carries one,
        // and the repertoire fold replaces its value wholesale.
        let incoming = resolved.and_then(|id| registry.get(id));
        if let Some(previous) = projected {
            let gravity_dir = frames
                .get(entity)
                .map_or(ambition_platformer2d_core::DEFAULT_GRAVITY_DIR, |frame| frame.down());
            previous.granted.clone().retract(
                &mut EntityScope::new(&mut commands, entity),
                gravity_dir,
                incoming.map_or(&[][..], |prepared| prepared.carries.as_slice()),
            );
        }
        let Some(prepared) = incoming else {
            if projected.is_some() {
                commands.entity(entity).remove::<ProjectedCharacterKit>();
            }
            continue;
        };
        // this marker records what THIS system granted, and nothing else.
        //
        // The body recorded that it was up to date and no later pass revisited it, which is worse
        // than a missed update.
        //
        // That writer keeps its own record now, `avatar::PersonaBaseline`, stamped
        // after IT applies the baseline. One writer, one record. This one covers
        // the authored silhouette, the movement feel and the motion model below.
        grant_prepared_character_body(&sprite_content,
            &mut EntityScope::new(&mut commands, entity),
            prepared,
            registry.generation(),
            if persona_bodies.contains(entity) {
                KitOwnership::PersonaDerive
            } else {
                KitOwnership::Grant
            },
            // This body answers to no match, so the character's own feel is
            // the whole answer. A SEAT resolves the same question against its
            // match's rules and hands the result in — see
            // `MatchRules::body_over`, which is the only place the two are
            // weighed against each other.
            prepared.movement_tuning,
            ambition_combat::hand::hand(held.get(entity).ok(), {
                #[cfg(feature = "portal")]
                let active = guns.get(entity).is_ok_and(|gun| gun.active);
                #[cfg(not(feature = "portal"))]
                let active = false;
                active
            }),
        );
    }
}


/// Install character projection and deterministic pose resolution.
pub struct ActorCharacterPlugin;

impl Plugin for ActorCharacterPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.add_systems(
                // The SIM schedule, not `Update`. (§4.11)
                //
                // These two read and write simulation state — a pose clock and the volumes
                // damage resolves against — so under rollback they must recompute on every
                // resimulated tick. In `Update` they ran once per FRAME while the sim re-ran
                // many times, which left `ResolvedHurtboxes` stale for every rewound tick even
                // though it is declared rollback-DERIVED on the promise that the sim rebuilds
                // it.
                sim,
                (
                    // Gated, not `Option<Res<..>>`: a world with no clock has no
                    // pose elapsed to advance, and a system that quietly treats a
                    // missing clock as dt=0 would freeze every pose timeline
                    // without saying so.
                    ambition_combat::hurtbox_resolution::advance_body_pose_clocks.run_if(
                        bevy::ecs::schedule::common_conditions::resource_exists::<
                            ambition_time::WorldTime,
                        >,
                    ),
                    // The rigged body's pose, from the clocks just advanced and
                    // before anything reads a joint: hurt parts below, and an
                    // attachment (a hand muzzle) that orders after this set.
                    // Iterates nothing where no body has a rig.
                    ambition_combat::body_rig::resolve_body_rig_poses
                        .in_set(ambition_combat::body_rig::BodyRigPoseResolved),
                    // ⭐ INTO THE PUBLISHED SET, so the volume refresher in
                    // `features` can order against a phase rather than against
                    // this function's identity. The domain owns the vocabulary;
                    // this composition decides which window it sits in.
                    ambition_combat::hurtbox_resolution::resolve_body_hurtboxes
                        .in_set(ambition_combat::hurtbox_resolution::BodyHurtboxesResolved),
                )
                    .chain()
                    // Pinned to one exact window inside `Combat`: AFTER the move
                    // clock advances, BEFORE damage resolves.
                    //
                    // Both edges are load-bearing. And every body's position is already
                    // post-movement here (`PlayerSimulation` and `WorldPrep` both precede
                    // `Combat`), so this is the one slot where clocks and positions are
                    // simultaneously current.
                    .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::Combat)
                    .after(ambition_platformer2d_shared_tangle::schedule::CombatSet::Playback)
                    .before(ambition_platformer2d_shared_tangle::schedule::CombatSet::Resolve),
            ).add_systems(
                sim,
                // C3/I1: a registered character's authored moveset and silhouette
                // land on the body BEFORE the PERSONA construction, which builds a
                // worn body's action set, moveset and identity baseline together
                // and then has equipment overlaid onto it. Landing after that pair
                // erased equipment-granted moves; the two agree now because
                // `apply_worn_character_kit` consults the same registry.
                //
                // A phase name cannot make that mistake: the set says where it runs.
                project_prepared_character_definitions
                    .in_set(ambition_platformer2d_shared_tangle::schedule::PlayerInputSet::CharacterProjection),
            );
    }
}

#[cfg(test)]
mod tests;
