//! Cross-domain simulation assembly for world objects, matches, and gameplay effects.

use bevy::prelude::*;
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
#[cfg(test)]
use ambition_platformer2d_shared_tangle::schedule::ActorDecisionSet;
use ambition_platformer2d_actor_monolith::body::*;
use ambition_platformer2d_actor_monolith::body;
use ambition_boss_encounter::ecs::{
    drive_boss_animators, project_boss_attack_state_from_move, update_ecs_bosses,
};
use ambition_combat::hazards::{advance_hazards, apply_hazard_contacts};
use ambition_breakables::{refresh_breakable_damageable_volumes, update_ecs_breakables};
use ambition_platformer2d_items::chests::open_ecs_chests;
use ambition_platformer2d_items::pickups::{PickupCollect, PickupMagnetize, collect_ecs_pickups, magnetize_pickups};
use ambition_world_items::containers::update_ecs_falling_chests;
use crate::world_overlay::rebuild_feature_ecs_world_overlay;


/// Schedules the gameplay-effect bus chain into
/// [`ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::GameplayEffects`].
pub struct GameplayEffectsSchedulePlugin;


impl bevy::prelude::Plugin for GameplayEffectsSchedulePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_message::<ambition_persistence::npc_provocation::NpcProvocationChanged>();
        let sim = app.sim_schedule();
        use ambition_platformer2d_shared_tangle::schedule::GameplayGated;
        use bevy::prelude::IntoScheduleConfigs;
        app.add_systems(
            sim,
            (
                // ARMING a challenge is first, so the grace that
                // `tick_pending_challenges` counts down starts on the tick the
                // narrative asked for it rather than the one after.
                //
                // A challenge, a pet, a look to wear and the talkers' spacing
                // are asked for only by a conversation (the Yarn vocabulary),
                // so they sleep with dialogue where a game uses none.
                body::arm_requested_challenges
                    .run_if(ambition_platformer2d_core::a_message_waits::<body::ChallengeRequested>)
                    .in_set(ambition_platformer2d_shared_tangle::capability::CapabilitySet(ambition_platformer2d_shared_tangle::capability::Capability::Dialogue)),
                // Before the pet: a pet asked for in the conversation puts its
                // own walk on the initiator after this has moved its step,
                // so the pet's walk is the one that stays.
                body::space_the_talkers.in_set(ambition_platformer2d_shared_tangle::capability::CapabilitySet(ambition_platformer2d_shared_tangle::capability::Capability::Dialogue)),
                body::apply_pet_requests
                    .run_if(ambition_platformer2d_core::a_message_waits::<body::PetRequested>)
                    .in_set(ambition_platformer2d_shared_tangle::capability::CapabilitySet(ambition_platformer2d_shared_tangle::capability::Capability::Dialogue)),
                // The pet's script: the walk's arrival, the gesture's end, and
                // its interruption. After the request, so a petter already on
                // the mark starts the gesture on the tick it asked.
                body::advance_pet_beats
                    .run_if(bevy::prelude::any_with_component::<body::PetBeat>)
                    .in_set(ambition_platformer2d_shared_tangle::capability::CapabilitySet(ambition_platformer2d_shared_tangle::capability::Capability::Dialogue)),
                // The holds and the two poses, from the beats as this tick
                // leaves them: the next control gate reads the pet's state
                // and not the state one tick before it.
                body::project_pet_holds.in_set(ambition_platformer2d_shared_tangle::capability::CapabilitySet(ambition_platformer2d_shared_tangle::capability::Capability::Dialogue)),
                // A look a conversation asked a body to wear. The template is
                // applied by the wear system on the tick after.
                body::apply_wear_requests
                    .run_if(ambition_platformer2d_core::a_message_waits::<body::WearRequested>)
                    .in_set(ambition_platformer2d_shared_tangle::capability::CapabilitySet(ambition_platformer2d_shared_tangle::capability::Capability::Dialogue)),
                // The row a character rests in, when its catalog row states
                // one (the giant gnu whole, with its arms on it).
                body::pin_rest_rows,
                // The rows of a body that fades between its attacks.
                body::pin_fade_rows,
                // Each applier of a request runs on its request.
                ambition_platformer2d_items::narrative::apply_item_grants.run_if(ambition_platformer2d_core::a_message_waits::<ambition_items::ItemGrantRequested>),
                ambition_platformer2d_items::narrative::apply_shop_transactions
                    .run_if(ambition_platformer2d_core::a_message_waits::<ambition_items::shop::ShopTransactionRequested>),
                ambition_platformer2d_items::narrative::apply_item_uses.run_if(ambition_platformer2d_core::a_message_waits::<ambition_items::ItemUseRequested>),
                // AFTER EVERY WRITER OF A HEAL: projectiles (Combat), pickups
                // (FeatureCollection), chests (FeatureInteraction) and item
                // uses (above). A heal written after its apply waits a tick,
                // and a rewind to that tick clears the message, so the
                // resimulation loses the heal.
                ambition_damage::healing::apply_player_heal_requests
                    .run_if(ambition_platformer2d_core::a_message_waits::<ambition_damage::healing::PlayerHealRequested>),
                crate::effect_bus::apply_flag_effects.run_if(ambition_platformer2d_core::a_message_waits::<ambition_combat::events::SetFlagRequested>),
                crate::effect_bus::apply_quest_effects
                    .run_if(ambition_platformer2d_core::a_message_waits::<ambition_persistence::quest::QuestAdvanceRequested>),
                crate::effect_bus::apply_switch_effects
                    .run_if(ambition_platformer2d_core::a_message_waits::<ambition_encounter::switches::SwitchActivated>),
                // Deferred-challenge grace runs only in `Playing` (after the dialog
                // box closes), then emits the `Challenged` stimulus the next system
                // consumes.
                body::tick_pending_challenges
                    .run_if(bevy::prelude::any_with_component::<body::PendingChallenge>)
                    .in_set(GameplayGated)
                    .in_set(ambition_platformer2d_shared_tangle::capability::CapabilitySet(ambition_platformer2d_shared_tangle::capability::Capability::Dialogue)),
                body::apply_actor_stimuli.run_if(ambition_platformer2d_core::a_message_waits::<ambition_combat::events::ActorStimulus>),
                // After the flip that announces a provocation, and after the
                // release that announces its end, so both land this tick.
                ambition_persistence::npc_provocation::record_npc_provocations
                    .run_if(ambition_platformer2d_core::a_message_waits::<ambition_persistence::npc_provocation::NpcProvocationChanged>)
                    .after(ambition_platformer2d_actor_monolith::body::apply_release_provocations),
                crate::effect_bus::apply_gameplay_sfx_effects
                    .run_if(ambition_platformer2d_core::a_message_waits::<ambition_combat::events::GameplaySfxRequested>),
            )
                .chain()
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::GameplayEffects),
        );
    }
}


/// Schedules `FeatureCollection`: pickup collection and mana regeneration.
pub struct FeatureCollectionSchedulePlugin;


impl bevy::prelude::Plugin for FeatureCollectionSchedulePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let sim = app.sim_schedule();
        use bevy::prelude::IntoScheduleConfigs;
        app.add_systems(
            sim,
            (
                // Pull nearby loot toward the player, then collect on overlap.
                magnetize_pickups.in_set(PickupMagnetize),
                collect_ecs_pickups.in_set(PickupCollect),
                // A METER MUTATOR on the controlled subject, scaled by sim dt.
                // In the simulation, not in `Update`: `body.mana` is rollback
                // state, and a render-rate write does not resimulate on a
                // rewind.
                ambition_platformer2d_actor_monolith::avatar::regen_player_mana
                    .run_if(bevy::prelude::resource_exists::<ambition_platformer2d_actor_monolith::avatar::systems::PlayerManaRegen>),
            )
                .chain()
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::FeatureCollection),
        );
    }
}




/// Schedules `FeatureInteraction`: switches, chests, breakables, save sync,
/// and encounter switch-index rebuild — and declares the cross-domain order
/// the phase runs in.
///
/// That is how `conversation` — 1,836 lines with zero `crate::` imports in either direction —
/// stayed pinned inside the monolith while every import measure said it was free.
///
/// the order is now
/// [`FeatureInteractionSet`](ambition_platformer2d_shared_tangle::schedule::FeatureInteractionSet), and each
/// prose rationale lives on the variant it explains rather than beside the
/// system it happened to precede. This plugin declares the total order ONCE and
/// each domain only says which phase it is in — `conversation` says so from
/// [`ambition_conversation::ConversationPlugin`], naming nothing in `features`.
pub struct FeatureInteractionSchedulePlugin;


impl bevy::prelude::Plugin for FeatureInteractionSchedulePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let sim = app.sim_schedule();
        use ambition_platformer2d_shared_tangle::schedule::FeatureInteractionSet;

        // the conversation domain installs itself: the authority, the
        // cut-bark port channel, its own narrative payload, its presentation
        // pair, and its three sim systems placed by PHASE. Nothing about
        // conversation is registered here any more.
        // (`ConversationPlugin` is a member of `PlatformerEnginePlugins`, so a
        // composition can leave dialogue out; this plugin only orders its phases.)
        // The cut-bark channel is conversation's to PRODUCE, and this plugin
        // consumes it (`speak_conversation_cut_barks`). A consumer registers what
        // it reads (registration is idempotent), or a composition that leaves
        // dialogue out fails the reader's parameter validation on tick one.
        app.add_message::<ambition_conversation::ConversationCutBark>();
        // the ledger payloads that are NOT conversation's. The ledger is
        // the record of what the narrative — which runs outside the simulation —
        // told the simulation, stamped with the tick it applies from. A rewind
        // restores what the simulation DECIDED; erasing what it was TOLD is how
        // the replay reaches a different answer.
        //
        // one per payload, and the list IS the classification — the
        // counterpart to the table in `crate::dialog::yarn_bindings`. A
        // gameplay-bearing Yarn command has a ledger here or it has no replay
        // story; a presentation-facing one must NOT be here, because deferring
        // a sound to a simulation tick would delay it for no reason. Content
        // registers its own vocabulary the same way, so this names no content.
        //
        // these stay here on purpose, and it is a seam rather than a
        // leftover. A payload belongs to whoever CONSUMES it: three of these
        // are `features` types that a carved-out conversation crate could not
        // name at all, and the other three are applied by `features::bus` and
        // `ambition_platformer2d_items::narrative`. Conversation provides the ledger MECHANISM
        // and registers only `ConversationEnded`, the payload it both defines
        // and consumes.
        app.add_plugins((
            // Authored flag writes now ride `RunAuthoredCommand`'s ledger, installed once by
            // the conversation plugin. the CHANNEL is untouched — chests, pickups and
            // interactions still write it from inside the simulation, and `sim_core_resources`
            // registers it.
            ambition_conversation::NarrativeInputPlugin::<ambition_platformer2d_actor_monolith::body::ChallengeRequested>::default(),
            ambition_conversation::NarrativeInputPlugin::<ambition_platformer2d_actor_monolith::body::BrainCommand>::default(),
            ambition_conversation::NarrativeInputPlugin::<ambition_platformer2d_actor_monolith::body::ReleaseProvocation>::default(),
            ambition_conversation::NarrativeInputPlugin::<ambition_platformer2d_actor_monolith::body::PetRequested>::default(),
            ambition_conversation::NarrativeInputPlugin::<ambition_platformer2d_actor_monolith::body::WearRequested>::default(),
            ambition_conversation::NarrativeInputPlugin::<ambition_items::ItemGrantRequested>::default(),
            ambition_conversation::NarrativeInputPlugin::<ambition_items::shop::ShopTransactionRequested>::default(),
            // A menu uses a consumable through the host-intent ledger.
            ambition_platformer2d_shared_tangle::host_intents::HostIntentPlugin::<ambition_items::ItemUseRequested>::default(),
            // A gate terminal's dial, hangup and close, through the same ledger.
            ambition_platformer2d_shared_tangle::host_intents::HostIntentPlugin::<ambition_platformer2d_world::rooms::GateTerminalIntent>::default(),
        ));

        // THE ORDER, SAID OUT LOUD. Every reason each boundary exists is
        // on the `FeatureInteractionSet` variant; the shape of the statement is
        // the part that belongs here.
        //
        // `.chain()` over the whole list, deliberately — `(A, B).before(C)`
        // would order both A and B before C and say nothing about A vs B, which
        // is exactly the gap that let ten systems look ordered while three of the
        // eleven pairwise contracts were unstated. A chain is a total order.
        //
        // the `ApplyDeferred` boundaries survive. The old per-system chain inserted a sync
        // point between every pair; Bevy inserts sync points on dependency edges after sets are
        // flattened to their members, so the set-level chain reproduces them at every phase
        // boundary.
        app.configure_sets(
            sim,
            (
                FeatureInteractionSet::NarrativeIntake,
                FeatureInteractionSet::Actuate,
                FeatureInteractionSet::Continuity,
                FeatureInteractionSet::CutBarkCast,
                FeatureInteractionSet::HoldProjection,
                FeatureInteractionSet::WorldObjects,
                FeatureInteractionSet::SwitchIndex,
            )
                .chain()
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::FeatureInteraction),
        );

        app.add_systems(
            sim,
            interact_ecs_actors_and_switches.in_set(FeatureInteractionSet::Actuate),
        );
        // A script's walk to a mark, on the blanked frame: a held body is
        // walked by the script and not by the stick. In the gate, so every
        // consumer of finished control reads the walk.
        app.add_systems(
            sim,
            ambition_platformer2d_actor_monolith::avatar::drive_commanded_moves
                .run_if(bevy::prelude::any_with_component::<ambition_characters::control::CommandedMove>)
                .after(ambition_platformer2d_actor_monolith::avatar::blank_scripted_control_frames)
                .in_set(ambition_platformer2d_shared_tangle::schedule::PlayerInputSet::ControlGate),
        );
        // The cast half of the conversation break. It sits in a set the
        // conversation ordering vocabulary declares and this domain fills —
        // the temporal twin of the `ConversationCutBark` message port.
        app.add_systems(
            sim,
            ambition_platformer2d_actor_monolith::body::speak_conversation_cut_barks
                .run_if(ambition_platformer2d_core::a_message_waits::<ambition_conversation::ConversationCutBark,
                    >,
                )
                .in_set(FeatureInteractionSet::CutBarkCast),
        );
        app.add_systems(
            sim,
            (
                open_ecs_chests,
                update_ecs_breakables.run_if(any_with_component::<ambition_breakables::BreakableFeature>),
                // OW5: the respawn's due time, after the timer has ticked.
                ambition_breakables::mirror_breakable_respawns,
                // Q152: a collected pickup comes back when its regrowth is due.
                ambition_platformer2d_items::regrowth::regrow_pickups,
                // Q154: a collected pickup authored `Never` is gone for good.
                ambition_platformer2d_items::pickups::record_consumed_pickups,
                update_ecs_falling_chests.run_if(any_with_component::<ambition_world_items::containers::FallingChest>),
            )
                .chain()
                .in_set(FeatureInteractionSet::WorldObjects),
        );
        // A replay rebuilds its room whole, and a checkpoint restore rebuilds
        // from a save that holds no broken breakable and no collected pickup
        // (OW5). The replay set is in `PlayerInput`, before the mirror's
        // phase; see the system's doc.
        ambition_platformer2d_shared_tangle::lifecycle::require_on_session_root::<
            ambition_world_returns::WorldTimeSchedule,
        >(app);
        app.add_systems(
                sim,
                (
                ambition_world_returns::forget_scheduled_returns_on_replay
                        .run_if(ambition_combat::events::a_replay_is_admitted),
                ambition_world_returns::disown_scheduled_returns_on_restore
                        .run_if(ambition_combat::events::a_replay_is_admitted),
                )
                    .in_set(ambition_platformer2d_shared_tangle::lifecycle::ContentRoomReplayResetSet),
            )
            .add_systems(
                ambition_platformer2d_shared_tangle::lifecycle::CheckpointDomainApply,
            ambition_world_returns::forget_scheduled_returns_on_restore,
            )
            // The same two, and the consumed occurrences' owners below, for a
            // checkpoint restore at its publication.
            .add_systems(
                ambition_combat::events::RestoreConsequences,
                (
                ambition_world_returns::forget_scheduled_returns_on_replay,
                ambition_world_returns::disown_scheduled_returns_on_restore,
                ambition_platformer2d_items::pickups::disown_consumed_pickups_on_restore,
                )
                    .in_set(ambition_combat::events::RestoreConsequenceSet::ReplayContent),
            );
        // Q151: who owns each one-time pickup consumed since the checkpoint.
        // The restore's acceptance pins the owned rows into the ledger it
        // restores, and its admission takes the dying participant out.
        app.init_resource::<ambition_platformer2d_items::pickups::ConsumedSinceCheckpoint>()
            .add_systems(
                sim,
                ambition_platformer2d_items::pickups::forget_consumed_pickups_at_checkpoint
                    .run_if(ambition_platformer2d_core::a_message_waits::<ambition_platformer2d_shared_tangle::lifecycle::CheckpointCommitted,
                    >,
                ),
            )
            // An occurrence lying in a live room that ended there, read after
            // the placement producers wrote this tick's rows and before the
            // custody leg retracts an `InCustody` row.
            .add_systems(
                sim,
                ambition_platformer2d_items::pickups::record_ended_occurrences
                    .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated)
                    .in_set(ambition_platformer2d_shared_tangle::schedule::HeldItemStep::Residency)
                    .after(ambition_platformer2d_shared_tangle::schedule::ResidencyStep::Record)
                    .before(ambition_platformer2d_shared_tangle::schedule::ResidencyStep::Project),
            )
            .add_systems(
                sim,
                ambition_platformer2d_items::pickups::disown_consumed_pickups_on_restore
                    .run_if(ambition_combat::events::a_replay_is_admitted)
                    .in_set(ambition_platformer2d_shared_tangle::lifecycle::ContentRoomReplayResetSet,
                    ),
            );
        // ⭐ The encounter switch index registers itself from
        // `ambition_encounter_features` now (2026-09-03). `FeatureInteractionSet`
        // is `shared_tangle` vocabulary, so the owning crate can name its own
        // slot without this crate naming the owning crate.
    }
}


/// Schedules `WorldPrep`: LDtk hot-reload, feature-world overlay rebuild,
/// and per-frame hazard/actor/boss ticks before player simulation reads them.
/// Register the DAMAGE-facing publication of every body's damageable volumes.
///
/// Extracted from `WorldPrepSchedulePlugin` so the ordering is expressed exactly once, and so a
/// test can assert the production wiring instead of asserting a copy of it.
///
/// This is the SECOND invocation of `refresh_body_damageable_volumes`; the first
/// runs in `WorldPrep` for the pogo derivation and the collision overlay. Same
/// function, same rule, two consumers with different timing needs — see the
/// system's own documentation for why that is a refresh and not a clobber.
pub fn register_damage_facing_volume_publication(app: &mut bevy::prelude::App) {
    use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
    let sim = app.sim_schedule();
    // ⛔⛔ **BOSSES JOIN THE BARRIER, AND UNTIL A2a THEY DID NOT.** A boss
    // published its volumes exactly once, in `WorldPrep`, chained BEFORE
    // `tick_boss_brains_system` / `project_boss_attack_state_from_move` /
    // `drive_boss_animators` — so the published value described the PREVIOUS
    // frame's attack state and animation frame, and every damage-side consumer
    // recomputed the geometry itself rather than read a stale publication.
    // Three recomputations of one fact, and the publisher's authored-hurtbox
    // override reached none of them.
    //
    // ⭐ THIS IS THE SAME WINDOW THOSE CONSUMERS WERE HAND-ROLLING: after
    // `Playback`, where `BossAttackState`'s active fields and
    // `BossAnimationFrameSample` are settled, and before `Resolve`, which reads
    // them. Publishing here is what lets the consumers stop asking.
    app.add_systems(
        sim,
        (
            refresh_body_damageable_volumes,
            // Only named bosses have boss volumes.
            refresh_boss_damageable_volumes.run_if(any_with_component::<ambition_characters::brain::BossAttackState>).in_set(ambition_platformer2d_shared_tangle::capability::CapabilitySet(ambition_platformer2d_shared_tangle::capability::Capability::BossEncounters,
                    ),
                ),
        )
            .in_set(ambition_combat::components::DamageFacingVolumesPublished)
            .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::Combat,
            )
            // Victim geometry is published between the move clock and the damage pass: AFTER
            // `Playback`, because a move's first active frame must not publish the previous frame's
            // volumes, and BEFORE `Resolve`, because that is what reads them.
            .after(ambition_platformer2d_shared_tangle::schedule::CombatSet::Playback)
            .before(ambition_platformer2d_shared_tangle::schedule::CombatSet::Resolve)
            // ⛔ ~~The one intra-crate edge that is genuinely between two
            // systems~~ — IT IS A PHASE NOW. The dependency is unchanged and
            // real (the character runtime resolves the silhouette this reads);
            // what changed is that `ambition_combat` publishes
            // `BodyHurtboxesResolved` and the runtime installs the resolver into
            // it, so this states a MEMBERSHIP rather than reaching into another
            // crate's private system list for a name.
            .after(ambition_combat::hurtbox_resolution::BodyHurtboxesResolved),
    );
    // Each guard where its art draws it this tick (`ambition_combat::guard`),
    // in the same window as the body volumes and for the same reason: after
    // the move and pose clocks, before the sweep that asks the guards first.
    app.add_systems(
        sim,
        (ambition_combat::guard::tick_guard_states, ambition_combat::guard::publish_guard_volumes,
        )
            .chain()
            .in_set(ambition_combat::components::DamageFacingVolumesPublished)
            .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::Combat,
            )
            .after(ambition_platformer2d_shared_tangle::schedule::CombatSet::Playback)
            .before(ambition_platformer2d_shared_tangle::schedule::CombatSet::Resolve)
            .after(ambition_combat::hurtbox_resolution::BodyHurtboxesResolved),
    );
}
/// Compose world facts, body execution, and match outcomes in their simulation phases.
pub struct WorldPrepSchedulePlugin;
impl Plugin for WorldPrepSchedulePlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.add_plugins(ambition_platformer2d_actor_monolith::body::BodySimulationPlugin);
        ambition_platformer2d_world::platforms::install_moving_platform_simulation(app, sim);
        app.add_systems(
            sim,
            crate::fall_rescue::rescue_falling_bodies
                .after(ambition_mount::MountsSteeredByRiders)
                .before(ambition_platformer2d_world::platforms::MovingPlatformStep)
                .in_set(ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::BeforeIntegrate),
        );
        // AE6: the rules a MATCH plays under, resolved from its declaration
        // folded over those baselines — so a stage never writes them. In
        // WorldPrep because every reader is later (PlayerSimulation/Combat),
        // and a resolution landing after them would hand the hit kernel last
        // tick's rules on the one tick they differ.
        app.init_resource::<ambition_combat::rules::ResolvedCombatTuning>();
        app.add_systems(
            sim,
            crate::combat_rules::project_combat_rules
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::WorldPrep),
        );
        // S4: spend a stock per KO. `CombatSet:Settle` is the phase for "everything that reads
        // this tick's damage outcome rather than producing it", which is exactly what this is —
        // the KO was decided in Resolve, and spending is bookkeeping over it.
        app.init_resource::<ambition_match::StocksMatchSettled>();
        app.init_resource::<ambition_match::SuddenDeathEntered>();
        app.add_message::<crate::stocks_match::SuddenDeathBegan>();
        // the same rule as `BodyHitResolved` below, applied to the clock.
        // `state_the_matchs_pace` writes a `ClockScaleRequest` and this plugin
        // is what schedules it, so this plugin registers it — a fixture that
        // composes the ruleset without the runtime's time-control assembly
        // would otherwise die on the first tick of a match. `add_message` is
        // idempotent, so the composition that already registers it is
        // unaffected (`transform_beat` does exactly this for the same reason).
        app.add_message::<ambition_time::time_control::ClockScaleRequest>();
        // AN INSTRUMENT REGISTERS WHAT IT READS.
        //
        // found by running `ladder_probe --features causal`, which panicked on the first tick
        // with "Message not initialized". The inspector could not be enabled for the game it
        // was built to inspect.
        //
        // this is the READER half of the rule already learned on the writer
        // side: a knockout is acted on by the SIMULATION and must fail loud, but
        // an INSTRUMENT must degrade. `add_message` is idempotent, so a
        // composition that already registers these is unaffected and one that
        // does not gets an empty stream instead of a panic — the honest reading
        // of "nothing knocked anybody out this tick".
        #[cfg(feature = "causal")]
        {
            app.add_message::<ambition_combat::stocks::BodyKnockedOut>();
            app.add_message::<ambition_combat::stocks::FighterStockSpent>();
            app.add_message::<ambition_combat::stocks::StocksMatchDecided>();
            app.add_message::<ambition_damage::BodyReactionApplied>();
        }
        // `decide_stocks_match` READS this, so a composition that installs the
        // stocks loop must have the channel or the system fails parameter
        // validation before it can run. Idempotent, so the host registering it
        // beside `StocksMatchDecided` costs nothing.
        app.add_message::<ambition_combat::stocks::FighterRespawnDue>();
        // A returning fighter's protection. The engine owns its clock and its
        // retraction, so a ruleset that grants it cannot forget either. Both
        // are inert on a body without `RespawnGrace`.
        app.add_systems(
            sim,
            ambition_combat::stocks::tick_respawn_grace
                .run_if(any_with_component::<ambition_combat::stocks::RespawnGrace>)
                .in_set(ambition_platformer2d_shared_tangle::capability::CapabilitySet(ambition_platformer2d_shared_tangle::capability::Capability::MatchStocks,
                    ),
                )
                .in_set(ambition_combat::stocks::RespawnGraceTicked)
                .after(ambition_combat::stocks::FighterRespawnsDue)
                .in_set(ambition_platformer2d_shared_tangle::schedule::CombatSet::Settle),
        );
        app.add_observer(ambition_combat::stocks::retract_respawn_grace_on_removal);
        app.add_systems(
            sim,
            (
                // The stock systems sleep where a game has no stocks match
                // (`Capability::MatchStocks`). The causal observers below
                // record every hit, and stay awake.
                ambition_combat::stocks::spend_fighter_stocks
                    .run_if(ambition_platformer2d_core::a_message_waits::<ambition_combat::stocks::BodyKnockedOut>)
                    .in_set(ambition_platformer2d_shared_tangle::capability::CapabilitySet(ambition_platformer2d_shared_tangle::capability::Capability::MatchStocks))
                    .in_set(ambition_combat::stocks::FighterStocksSpent),
                // D192: the return is decided AFTER the spend and BEFORE any
                // ruleset places a body. An interval of zero still resolves on
                // this tick, so a mode that authored no beat is unchanged.
                //
                // D201: this no longer TICKS anything — the window it reads is
                // advanced by `tick_death_interlude`, which every death in the
                // process already shares. What is ordered here is the
                // CONSEQUENCE, and it still has to sit between the spend that
                // opens the window and the ruleset that places the body.
                ambition_combat::stocks::respawn_when_the_interlude_closes
                    .run_if(bevy::prelude::any_with_component::<ambition_combat::stocks::PendingRespawn>)
                    .in_set(ambition_platformer2d_shared_tangle::capability::CapabilitySet(ambition_platformer2d_shared_tangle::capability::Capability::MatchStocks))
                    .in_set(ambition_combat::stocks::FighterRespawnsDue)
                    .after(ambition_combat::stocks::FighterStocksSpent),
                crate::stocks_match::decide_stocks_match
                    .in_set(ambition_platformer2d_shared_tangle::capability::CapabilitySet(ambition_platformer2d_shared_tangle::capability::Capability::MatchStocks))
                    .in_set(ambition_combat::stocks::MatchOutcomeDecided),
                // Reading the latch on the frame BEFORE the one that sets it would let the
                // winner play on for one tick, which is the window a final KO happens in.
                crate::stocks_match::state_the_matchs_pace.run_if(resource_exists::<ambition_match::ActiveMatch>).in_set(ambition_platformer2d_shared_tangle::capability::CapabilitySet(ambition_platformer2d_shared_tangle::capability::Capability::MatchStocks)),
                // The causal OBSERVER, last in the chain so it reads this tick's
                // decision rather than the previous one. It holds no authority
                // over any of the above — it reads their messages — which is why
                // it can sit inside the ruleset's own chain safely.
                #[cfg(feature = "causal")]
                ambition_combat::causal::record_stock_lifecycle,
                // The damage observers are the runtime's (`causal::record_hit_*`),
                // installed in `CombatSet::Settle` after this chain.
            )
                .chain()
                .in_set(ambition_platformer2d_shared_tangle::schedule::CombatSet::Settle),
        );
        // Accumulating sim-time for brain perception (reaction latency).
        app.add_plugins(ambition_platformer2d_shared_tangle::lifecycle::SessionElapsedPlugin);
        // ⭐ THE WORLD-SOURCE HOT-RELOAD WATCHER MOVED OUT, resource and system
        // together, to `ambition_dev_tools::DevToolsSimPlugin` — the crate that
        // owns it. A simulation package registering a developer facility is the
        // dependency-direction defect this campaign is about; the reasons the
        // watcher runs in `Update` rather than the sim moved with it.
        app.add_systems(
            sim,
            (
                refresh_body_damageable_volumes,
                refresh_boss_damageable_volumes.run_if(any_with_component::<ambition_characters::brain::BossAttackState>).in_set(ambition_platformer2d_shared_tangle::capability::CapabilitySet(ambition_platformer2d_shared_tangle::capability::Capability::BossEncounters)),
                refresh_breakable_damageable_volumes,
                derive_pogo_target_volumes,
                rebuild_feature_ecs_world_overlay
                    .in_set(ambition_platformer2d_shared_tangle::schedule::FeatureWorldOverlaySet),
                advance_hazards
                    .run_if(any_with_component::<ambition_combat::hazard_runtime::HazardFeature>)
                    .in_set(ambition_combat::hazards::HazardTickSet),
                // Actor targeting/decision, movement, read-model projection, and contact
                // damage are registered below on their owning phase sets.
                // Ambient NPC chatter (parrot squawks, etc.) on its own timer.
                tick_npc_idle_barks,
            )
                .chain()
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::WorldPrep),
        );
        app.add_systems(
            sim,
            update_ecs_bosses
                .run_if(any_with_component::<ambition_combat::components::BossPhase>)
                .in_set(ambition_platformer2d_shared_tangle::capability::CapabilitySet(
                    ambition_platformer2d_shared_tangle::capability::Capability::BossEncounters,
                ))
                .in_set(ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::AfterIntegrate),
        );
        // R1.3: the SIM owns the boss animation frame + writes the geometry sample
        // (retiring the render→sim write-back in `animate_bosses`). Runs after the
        // `BossAttackState` projection so it picks this frame's anim, and before the
        // renderer's presentation `animate_bosses` (a later schedule), which mirrors
        // the sim-driven frame into its draw-only animator.
        app.add_systems(
            sim,
            drive_boss_animators
                .run_if(any_with_component::<ambition_sprite_sheet::boss::BossAnimFrame>)
                .in_set(ambition_platformer2d_shared_tangle::capability::CapabilitySet(ambition_platformer2d_shared_tangle::capability::Capability::BossEncounters,
                    ),
                )
                .after(project_boss_attack_state_from_move)
                // ⛔ IT WRITES AN INPUT OF THE DAMAGE-FACING PUBLICATION. The
                // frame sample decides which hurtbox row a boss exposes, and
                // `refresh_boss_damageable_volumes` now reads it in `Combat`; an
                // `.after` on the projection alone leaves this free to run after
                // the publication and hand it last frame's row.
                .before(ambition_combat::components::DamageFacingVolumesPublished)
                // ⛔ ITS CAPABILITY IS A RESOURCE, SO THE GUARD IS A RESOURCE
                // CONDITION. It takes `Res<BossCatalog>`, whose sole production
                // initializer is `BossEncounterSimulationPlugin`; a host that
                // omits that plugin must SKIP this system, not fail parameter
                // validation. A session gate cannot say that — it answers
                // "is this session authorized", which is a different question
                // and is true in hosts that have no catalog.
                .run_if(bevy::ecs::schedule::common_conditions::resource_exists::<
                    ambition_boss_encounter::BossCatalog,
                >,
                )
                // ⛔⛤ **AND THE SET IS THE GATE, NEVER `WorldPrep`.** This system
                // also runs `.after(project_boss_attack_state_from_move)`, which
                // is `.in_set(CombatSet::Playback)` — LATER in the sim schedule
                // than `WorldPrep`. Naming that phase orders this system both
                // before and after Playback: a dependency cycle, which the fixed
                // loop retries forever while allocating.
                // `GameplaySimulationRoot` carries `simulation_authorized` and
                // pins no position.
                .in_set(ambition_platformer2d_shared_tangle::schedule::GameplaySimulationRoot),
        );
        // ── The SECOND publication of every body's damageable volumes ──
        //
        // Same function, same rule, later phase. The `WorldPrep` copy above feeds the pogo
        // derivation and the feature-world collision overlay, which are rebuilt in that set;
        // this copy exists because DAMAGE resolves in `Combat`, and a body's `CenteredAabb` is
        // written by its own integrator — an actor's in `WorldPrep`, the PLAYER's in
        // `PlayerSimulation`.
        //
        // `in_set(CoreSimulation)` keeps it inside `GameplaySimulationRoot`, so the
        // session gate covers it like everything else.
        register_damage_facing_volume_publication(app);
        // Hazard contacts read the settled path (`BodyPathSet`), against where
        // the hazard moved to this tick.
        app.add_systems(
            sim,
            apply_hazard_contacts
                .run_if(any_with_component::<ambition_combat::hazard_runtime::HazardFeature>)
                .in_set(ambition_platformer2d_shared_tangle::schedule::BodyPathSet::Contacts)
                .after(ambition_combat::hazards::HazardTickSet),
        );
    }
}

#[cfg(test)]
mod actor_decision_phase_tests {
    use super::*;
    use bevy::ecs::schedule::graph::Direction;
    use bevy::ecs::schedule::{NodeId, ScheduleGraph, Schedules, SystemKey, SystemSetKey};

    const DECISION_PHASES: [ActorDecisionSet; 6] = [
        ActorDecisionSet::Targeting,
        ActorDecisionSet::Prepare,
        ActorDecisionSet::Observe,
        ActorDecisionSet::StateMaintenance,
        ActorDecisionSet::Decide,
        ActorDecisionSet::Publish,
    ];

    const DECISION_MEMBERSHIP: [(&str, ActorDecisionSet); 11] = [
        ("dissolve_settled_grudges", ActorDecisionSet::Targeting),
        ("select_actor_targets", ActorDecisionSet::Targeting),
        ("assess_dormancy", ActorDecisionSet::Prepare),
        ("project_authored_fighter_ladder", ActorDecisionSet::Prepare),
        ("collect_perception_peers", ActorDecisionSet::Observe),
        ("collect_perception_projectiles", ActorDecisionSet::Observe),
        ("observe_actor_decision_inputs", ActorDecisionSet::Observe),
        ("advise_navigation", ActorDecisionSet::Observe),
        (
            "maintain_actor_pre_decision_state",
            ActorDecisionSet::StateMaintenance,
        ),
        ("tick_actor_brains", ActorDecisionSet::Decide),
        ("publish_actor_decision_frames", ActorDecisionSet::Publish),
    ];

    const MOVEMENT_MEMBERSHIP: [(
        &str,
        ambition_platformer2d_shared_tangle::schedule::WorldPrepSet,
    ); 8] = [
        (
            "tick_capture_holds",
            ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::BeforeIntegrate,
        ),
        (
            "steer_mount_from_rider",
            ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::BeforeIntegrate,
        ),
        (
            "rescue_falling_bodies",
            ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::BeforeIntegrate,
        ),
        (
            "advance_moving_platforms",
            ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::BeforeIntegrate,
        ),
        (
            "snapshot_body_contact",
            ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::BeforeIntegrate,
        ),
        (
            "integrate_sim_bodies",
            ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::Integrate,
        ),
        (
            "maintain_existing_capture_pose",
            ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::AfterIntegrate,
        ),
        (
            "apply_actor_contact_damage",
            ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::ContactDamage,
        ),
    ];

    fn composed_app() -> App {
        let mut app = App::new();
        crate::schedule::configure_platformer2d_simulation_phases(&mut app);
        app.add_plugins(WorldPrepSchedulePlugin);
        // The player schedule installs the causal observers.
        #[cfg(feature = "causal")]
        app.add_plugins(crate::player_schedule::PlayerSchedulePlugin);
        app
    }

    fn system_key(graph: &ScheduleGraph, leaf: &str) -> SystemKey {
        let mut found = None;
        for (key, system, _) in graph.systems.iter() {
            let name = format!("{}", system.name());
            if name.rsplit("::").next() == Some(leaf) {
                assert!(
                    found.is_none(),
                    "{leaf} resolved to more than one sim system; phase membership is ambiguous"
                );
                found = Some(key);
            }
        }
        found.unwrap_or_else(|| panic!("{leaf} must be scheduled by WorldPrepSchedulePlugin"))
    }

    fn assert_membership<S>(graph: &ScheduleGraph, leaf: &str, set: S)
    where
        S: SystemSet + Copy + std::fmt::Debug,
    {
        let system_key = system_key(graph, leaf);
        let set_key = graph
            .system_sets
            .get_key(set.intern())
            .unwrap_or_else(|| panic!("{set:?} must be a registered SystemSet"));
        assert!(
            phase_ancestors(graph, system_key).contains(&set_key),
            "{leaf} must run in {set:?}, and no chain of set memberships puts it there"
        );
    }

    /// Every set that contains `system`, directly or through other sets.
    ///
    /// ⛔⛔ THIS USED TO BE A SINGLE `contains_edge`, AND THE DIFFERENCE IS THE
    /// WHOLE CAPABILITY-PLUGIN PATTERN. When `ambition_mount` grew its own
    /// `install_mount_pose_systems`, `steer_mount_from_rider` stopped being a
    /// direct member of `BeforeIntegrate` and became a member of
    /// `MountsSteeredByRiders`, which this layer places IN `BeforeIntegrate`. It
    /// runs in exactly the same phase; only the number of hops changed. A
    /// one-edge check reads that as the system having left the phase.
    ///
    /// ⇒ The question worth guarding is "does this run in that phase", and
    /// membership is transitive. ⚠ What is NOT relaxed: a system in no phase at
    /// all still fails, and so does one whose set is placed in a DIFFERENT phase
    /// — see `a_system_outside_the_phase_is_still_caught`.
    fn phase_ancestors(graph: &ScheduleGraph, system: SystemKey) -> Vec<SystemSetKey> {
        let hierarchy = graph.hierarchy().graph();
        let mut seen: Vec<SystemSetKey> = Vec::new();
        let mut frontier = vec![NodeId::System(system)];
        while let Some(node) = frontier.pop() {
            for parent in hierarchy.neighbors_directed(node, Direction::Incoming) {
                if let NodeId::Set(key) = parent {
                    if !seen.contains(&key) {
                        seen.push(key);
                        frontier.push(parent);
                    }
                }
            }
        }
        seen
    }

    /// ⭐⭐ THE POISON FOR THE RELAXATION ABOVE, and it is the reason the
    /// relaxation is allowed to exist.
    ///
    /// `assert_membership` was widened from ONE hierarchy edge to any chain of
    /// them, so that a capability installing its own systems into its own
    /// published set still counts as running in the phase this layer places that
    /// set in. ⛔ A widened check is the classic way a guard dies quietly: the
    /// obvious over-widening — "walk far enough and everything is inside
    /// everything" — would make it pass for any system in the schedule at all.
    ///
    /// ⇒ So this pins the two failures that must survive: a system that belongs
    /// to NO set anywhere, and a system whose set sits in a DIFFERENT phase. If
    /// either of these ever passes, the transitive walk has stopped
    /// discriminating and every assertion above it is decoration.
    #[test]
    fn a_system_outside_the_phase_is_still_caught() {
        use ambition_platformer2d_shared_tangle::schedule::WorldPrepSet;
        use bevy::prelude::IntoScheduleConfigs as _;

        #[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
        struct SomeOtherSet;

        fn orphan_system() {}
        fn wrongly_placed_system() {}

        let mut app = composed_app();
        let sim = app.sim_schedule();
        app.add_systems(sim, orphan_system);
        app.add_systems(sim, wrongly_placed_system.in_set(SomeOtherSet));
        app.configure_sets(sim, SomeOtherSet.in_set(WorldPrepSet::ContactDamage));

        let schedules = app.world().resource::<Schedules>();
        let schedule = schedules.get(sim).expect("sim schedule must exist");
        let graph = schedule.graph();

        let before_integrate = graph
            .system_sets
            .get_key(WorldPrepSet::BeforeIntegrate.intern())
            .expect("the phase is registered");

        // ⚠ NOT "has no ancestors at all" — the first draft of this asserted that
        // and failed, correctly. Bevy gives EVERY system its own automatic
        // `SystemTypeSet`, so a system in no authored set still reports one
        // ancestor. ⇒ The claim worth making is about AUTHORED PHASES, which is
        // also the only thing `assert_membership` is ever asked about.
        let orphan = system_key(graph, "orphan_system");
        assert!(
            !phase_ancestors(graph, orphan).contains(&before_integrate),
            "a system in no authored set is being reported as a member of a phase, \
             so the walk is inventing membership"
        );

        let misplaced = system_key(graph, "wrongly_placed_system");
        let ancestors = phase_ancestors(graph, misplaced);
        assert!(
            !ancestors.contains(&before_integrate),
            "a system placed in ContactDamage is being reported as a member of \
             BeforeIntegrate, so the transitive walk no longer discriminates \
             between phases"
        );
        let contact_damage = graph
            .system_sets
            .get_key(WorldPrepSet::ContactDamage.intern())
            .expect("the phase is registered");
        assert!(
            ancestors.contains(&contact_damage),
            "and the positive half: a two-hop membership must still be FOUND, or \
             the walk has been narrowed back to one edge"
        );
    }

    #[test]
    fn actor_decision_authority_phases_are_explicitly_ordered() {
        let mut app = composed_app();
        let sim = app.sim_schedule();
        let schedules = app.world().resource::<Schedules>();
        let schedule = schedules.get(sim).expect("sim schedule must exist");
        let graph = schedule.graph();

        let world_prep = graph
            .system_sets
            .get_key(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::WorldPrep.intern())
            .expect("WorldPrep must be registered");
        for phase in DECISION_PHASES {
            let phase_key = graph
                .system_sets
                .get_key(phase.intern())
                .unwrap_or_else(|| panic!("{phase:?} must be registered"));
            assert!(
                graph
                    .hierarchy()
                    .graph()
                    .contains_edge(NodeId::Set(world_prep), NodeId::Set(phase_key)),
                "{phase:?} must remain inside WorldPrep"
            );
        }

        for pair in DECISION_PHASES.windows(2) {
            let before = graph
                .system_sets
                .get_key(pair[0].intern())
                .unwrap_or_else(|| panic!("{:?} must be registered", pair[0]));
            let after = graph
                .system_sets
                .get_key(pair[1].intern())
                .unwrap_or_else(|| panic!("{:?} must be registered", pair[1]));
            assert!(
                graph
                    .dependency()
                    .graph()
                    .contains_edge(NodeId::Set(before), NodeId::Set(after)),
                "actor decision edge {:?} -> {:?} must be explicit",
                pair[0],
                pair[1]
            );
        }

        let publish = graph
            .system_sets
            .get_key(ActorDecisionSet::Publish.intern())
            .expect("Publish must be registered");
        let before_integrate = graph
            .system_sets
            .get_key(
                ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::BeforeIntegrate
                    .intern(),
            )
            .expect("BeforeIntegrate must be registered");
        assert!(
            graph
                .dependency()
                .graph()
                .contains_edge(NodeId::Set(publish), NodeId::Set(before_integrate)),
            "autonomous control publication must finish before movement preconditions begin"
        );
    }

    #[test]
    fn actor_decision_systems_are_members_of_their_authority_phases() {
        let mut app = composed_app();
        let sim = app.sim_schedule();
        let schedules = app.world().resource::<Schedules>();
        let graph = schedules.get(sim).expect("sim schedule must exist").graph();

        for (leaf, phase) in DECISION_MEMBERSHIP {
            assert_membership(graph, leaf, phase);
        }

    }

    /// `drive_boss_animators` is under the GATE and under NO PHASE.
    ///
    /// ⛔⛤ **THE PHASE IS THE DEFECT, NOT THE FIX.** `b9f2ece18` gave this
    /// system `.in_set(WorldPrep)` to buy it the `simulation_authorized` gate.
    /// It also runs `.after(project_boss_attack_state_from_move)`, which is
    /// `.in_set(CombatSet::Playback)` — LATER in the sim schedule than
    /// `WorldPrep`. That orders one system both before and after Playback, and
    /// the fixed loop retries the broken schedule forever: MEASURED ~27 MB/s for
    /// the first minute, ~234 MB/s after, 64,629,160 kB of anonymous RSS in
    /// 316 s, global OOM.
    ///
    /// ⇒ So the guard is TWO-SIDED. The positive half alone would pass with the
    /// phase re-added, because `WorldPrep` is itself inside
    /// `GameplaySimulationRoot`.
    ///
    /// ⚠ **THERE IS DELIBERATELY NO POISON THAT REBUILDS THE CYCLE.** A poison
    /// here would have to construct the runaway to prove the guard sees it, and
    /// that costs whoever runs it their machine. The relaxation this check rests
    /// on — transitive `phase_ancestors` — already has its poison in
    /// `a_system_outside_the_phase_is_still_caught`, which pins that a system in
    /// a DIFFERENT phase still fails. This test inherits that discrimination
    /// rather than re-earning it.
    #[test]
    fn the_boss_animator_takes_the_gate_and_not_a_phase() {
        use ambition_platformer2d_shared_tangle::schedule::{
            GameplaySimulationRoot, Platformer2dSimulationPhase,
        };

        let mut app = composed_app();
        let sim = app.sim_schedule();
        let schedules = app.world().resource::<Schedules>();
        let graph = schedules.get(sim).expect("sim schedule must exist").graph();

        assert_membership(graph, "drive_boss_animators", GameplaySimulationRoot);

        let world_prep = graph
            .system_sets
            .get_key(Platformer2dSimulationPhase::WorldPrep.intern())
            .expect("WorldPrep must be a registered SystemSet");
        let ancestors = phase_ancestors(graph, system_key(graph, "drive_boss_animators"));
        assert!(
            !ancestors.contains(&world_prep),
            "drive_boss_animators acquired WorldPrep ancestry again. It orders \
             .after(project_boss_attack_state_from_move), which is in \
             CombatSet::Playback — LATER than WorldPrep — so this is a dependency \
             cycle, and the fixed loop retries it until the machine dies."
        );
    }

    /// The boss steps sleep where the room's game has no named bosses: they
    /// are in the boss capability's set, whose condition asks the room's game.
    #[test]
    fn the_boss_steps_are_in_the_boss_capability() {
        use ambition_platformer2d_shared_tangle::capability::{Capability, CapabilitySet};

        let mut app = composed_app();
        let sim = app.sim_schedule();
        let schedules = app.world().resource::<Schedules>();
        let graph = schedules.get(sim).expect("sim schedule must exist").graph();
        for leaf in ["update_ecs_bosses", "tick_boss_brains_system", "drive_boss_animators",
        ] {
            assert_membership(graph, leaf, CapabilitySet(Capability::BossEncounters));
        }
    }

    #[test]
    fn actor_movement_systems_are_members_of_named_world_prep_phases() {
        let mut app = composed_app();
        let sim = app.sim_schedule();
        let schedules = app.world().resource::<Schedules>();
        let graph = schedules.get(sim).expect("sim schedule must exist").graph();

        for (leaf, phase) in MOVEMENT_MEMBERSHIP {
            assert_membership(graph, leaf, phase);
        }

        #[cfg(feature = "causal")]
        assert_membership(
            graph,
            "record_movement_operations",
            ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::AfterIntegrate,
        );
    }

    /// CONTACT DAMAGE RUNS AFTER THE POSES IT READS, AND BEFORE THE BARKS THAT
    /// REACT TO IT.
    ///
    /// ⛔ THE SUBJECT IS THE SET, NOT ONE SYSTEM, and deliberately: contact
    /// damage depends on POSES BEING SETTLED, which is a property of the whole
    /// phase — a system-to-system edge names one contributor to it and goes
    /// stale the moment another joins. So it is asserted in the two halves that
    /// compose it: membership, and the ordering between the sets.
    #[test]
    fn contact_damage_runs_after_settled_poses_and_before_the_barks() {
        let mut app = composed_app();
        let sim = app.sim_schedule();
        let schedules = app.world().resource::<Schedules>();
        let graph = schedules.get(sim).expect("sim schedule must exist").graph();

        // Half one: the captive constraint is inside the phase that settles poses.
        assert_membership(
            graph,
            "maintain_existing_capture_pose",
            ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::AfterIntegrate,
        );
        assert_membership(
            graph,
            "apply_actor_contact_damage",
            ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::ContactDamage,
        );

        // Half two: that phase is ordered ahead of contact damage's.
        let set_key = |set: ambition_platformer2d_shared_tangle::schedule::WorldPrepSet| {
            graph
                .system_sets
                .get_key(set.intern())
                .unwrap_or_else(|| panic!("{set:?} must be registered"))
        };
        let settled =
            set_key(ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::AfterIntegrate);
        let contact_set =
            set_key(ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::ContactDamage);
        assert!(
            graph
                .dependency()
                .graph()
                .contains_edge(NodeId::Set(settled), NodeId::Set(contact_set)),
            "contact damage must be ordered after the phase that settles poses"
        );
        //  the zero floor: the reverse edge must NOT exist, or the pair
        // would be ordered both ways and this would measure nothing.
        assert!(
            !graph
                .dependency()
                .graph()
                .contains_edge(NodeId::Set(contact_set), NodeId::Set(settled)),
            "the two phases are ordered in BOTH directions"
        );

        // And the tail edge, which is an ordinary singleton-to-singleton order.
        let contact = system_key(graph, "apply_actor_contact_damage");
        let idle_barks = system_key(graph, "tick_npc_idle_barks");
        //  the edge lands on `tick_npc_idle_barks`'s OWN `SystemTypeSet`,
        // not on its system node: that is what `.before(some_fn)` compiles to,
        // and it is why an assertion written as system-to-system cannot pass for
        // an ordering expressed that way. Asked as "is contact damage ordered
        // ahead of a set that idle barks belongs to", which is true for both
        // spellings.
        let ordered_ahead = graph
            .dependency()
            .graph()
            .neighbors_directed(
                NodeId::System(contact),
                bevy::ecs::schedule::graph::Direction::Outgoing,
            )
            .any(|node| match node {
                NodeId::System(system) => system == idle_barks,
                NodeId::Set(set) => graph
                    .hierarchy()
                    .graph()
                    .contains_edge(NodeId::Set(set), NodeId::System(idle_barks)),
            });
        assert!(
            ordered_ahead,
            "contact damage must be ordered ahead of the idle barks that read it"
        );
    }
}