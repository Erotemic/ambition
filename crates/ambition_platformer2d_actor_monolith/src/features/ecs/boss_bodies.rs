//! The boss BODY INTEGRATION arm, and the one piece of the boss ECS module that
//! did not leave with it.
//!
//! ⭐⭐ IT STAYS BECAUSE ITS OWN VERDICT SAYS SO. The doc below already recorded
//! that `integrate_actor_body` is the shared seam and that this function is *"the
//! boss orchestrator around that seam"* — so it reaches `actors::ActorSteering`,
//! `actor_clusters::ActorClusterQueryData` and `actors::integrate_actor_body`,
//! three things the monolith still owns. Moving it would have meant moving the
//! generic actor integrator, which is a different carve with a different price.
//!
//! ⛔⛔ AND IT WAS INVISIBLE TO THE ESTIMATE. It reached them through
//! `super::super::`, which is neither a `crate::` path nor a glob — a THIRD shape
//! a carve census misses. The compiler found it the moment the module moved.

use ambition_boss_encounter::BossConfig;
use ambition_characters::control::ActorControl;
use ambition_combat::components::CenteredAabb;
use ambition_combat::events::HitEvent;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
use ambition_time::WorldTime;
use bevy::prelude::{Entity, Query, Res, With, Without};

/// This is the boss sibling of the player's `integrate_home_body` arm and the
/// actor arm of `integrate_sim_bodies`: three disjoint archetypes, ONE shared
/// integrator. It keeps its own scheduled slot (chain 1, between
/// `tick_boss_brains_system` and `update_ecs_bosses`) so the boss's presentation
/// systems still read this frame's already-moved position. Byte-identical to the
/// old bespoke arm: a boss's flight produces no jump/dash/land move-events (no
/// movement FX), never `shark_charge_crash`es (its caps lack `charge_crash_explodes`),
/// and its stagger timers are always zero (the boss victim path arms none), so
/// every extra thing `integrate_actor_body` does is a no-op here.
///
/// E6(d) no-boss-arm fold verdict: do NOT merge this into
/// `integrate_sim_bodies` by adding a boss branch. The cheap bound fails because
/// the fold requires a schedule move (chain-2 body movement ahead of chain-1 boss
/// presentation), boss-only query inputs (`BossConfig` + `BodyEnvelope` +
/// combat-size self-heal), and no-FX/no-mount policy skips. That would be a new
/// adapter branch, not deletion of a path. The shared seam is
/// `integrate_actor_body`; keep this as the boss orchestrator around that seam.
pub fn integrate_boss_bodies(
    // A13: whose cues each boss body emits.
    body_sources: Query<&ambition_sfx::BodyPresentationSource>,
    world_time: Res<WorldTime>,
    // The composed collision read-API rather than its three ingredients.
    collision: ambition_platformer2d_world::collision::CollisionWorld,
    feel_tuning: Res<ambition_combat::feel::Platformer2dFeelTuningMonolith>,
    steering: Res<crate::features::ecs::actors::ActorSteering>,
    mut sfx: ambition_sfx::SfxWriter,
    mut vfx: bevy::prelude::MessageWriter<ambition_vfx::vfx::VfxMessage>,
    mut hit_events: bevy::prelude::MessageWriter<HitEvent>,
    mut bosses: Query<
        (
            Entity,
            crate::features::ecs::actor_clusters::ActorClusterQueryData,
            &BossConfig,
            &ambition_combat::BodyEnvelope,
            Option<&mut ActorControl>,
            Option<&mut ambition_characters::actor::BodyAnimFacts>,
            &ambition_combat::components::ActorTarget,
            &mut CenteredAabb,
            &mut ambition_characters::actor::BodyCombat,
            // The body's explicit movement policy — a boss carries one from
            // spawn like every integrated body (absence is never a policy).
            &'static mut ambition_platformer2d_core::movement::MotionModel,
            // The per-tick resolved frame published by the frame resolution
            // phase — the SAME artifact every other body integrates under.
            &'static ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
            &'static mut ambition_platformer2d_core::BodyMotionFacts,
            Option<&'static ambition_combat::moveset::MovePlayback>,
            // ADR 0033's window, asked rather than assumed — see the call below.
            bevy::prelude::Has<ambition_combat::death_rules::OutOfPlay>,
        ),
        (
            With<FeatureSimEntity>,
            Without<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
        ),
    >,
) {
    let dt = world_time.sim_dt();
    let Some(feature_world) = collision.solids() else {
        return;
    };
    let combat_tuning = feel_tuning.feature_combat_tuning();
    for (
        entity,
        mut cq,
        boss_config,
        envelope,
        mut control,
        mut anim,
        target,
        mut aabb,
        mut combat,
        mut motion_model,
        resolved_frame,
        mut motion_facts,
        playback,
        boss_out_of_play,
    ) in &mut bosses
    {
        // Self-heal the collision envelope onto `kin.size` (the seam sweeps it),
        // robust to the profile / spawn-override / sprite-derive timing that writes
        // `behavior.combat_size`. The coarse render footprint stays in `BodyEnvelope`.
        let combat_size = boss_config.behavior.combat_size.unwrap_or(cq.kin.size);
        let mut em = cq.as_actor_mut();
        em.kin.size = combat_size;
        crate::features::ecs::actors::integrate_actor_body(
            entity,
            body_sources.get(entity).ok().map(|s| s.id()),
            &mut em,
            &mut aabb,
            &mut combat,
            control.as_deref_mut(),
            anim.as_deref_mut(),
            // The boss's coarse render envelope publishes the `CenteredAabb`
            // (byte-identical to the old render-sized box); an ordinary actor
            // would pass `None` and publish from `kin.size`.
            Some(envelope.0),
            &mut motion_model,
            target.pos,
            // A boss is never mounted, and nothing carries one either.
            false,
            false,
            &feature_world,
            combat_tuning,
            &steering,
            resolved_frame.get(),
            playback.map_or(1.0, |pb| pb.motion_scale_now()),
            // A boss authors no recovery, so this is `None` today and the
            // derivation answers "not helpless" — by the rule, not by an
            // exception written into the boss road.
            playback,
            // The same published read the actor loop makes: last tick's tumble,
            // so a launched boss keeps its tech press too.
            motion_facts.tumbling,
            // The same read too, and by the RULE rather than by a boss
            // exemption. Nothing opens a death window on a boss today, so this
            // is `false` in every shipped composition — which is exactly why it
            // is asked instead of asserted: the last road that asserted it
            // silently stopped holding a body still the day a second opener
            // appeared.
            boss_out_of_play,
            dt,
            *feel_tuning,
            // A boss authors its feel through its own catalog today; when one
            // grows an `AuthoredMovementTuning` this is the line that reads it.
            None,
            &mut sfx,
            &mut vfx,
            &mut hit_events,
            #[cfg(feature = "causal")]
            None,
            // a boss is not in the contact snapshot. Body contact is
            // granted per body by a composition; nothing grants it to a boss,
            // and an inert field resolves this body exactly as it did.
            ae::BodyContactField::NONE,
        );
        *motion_facts = ambition_platformer2d_core::BodyMotionFacts::from_model(&motion_model);
    }
}
