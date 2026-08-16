//! Runtime schedule vocabulary that is independent of Ambition content.
//!
//! `Platformer2dSimulationPhaseMonolith` remains the concrete app schedule for now. These labels document
//! the future crate-level concepts and give new runtime modules names that do
//! not depend on app assembly details.

use core::sync::atomic::{AtomicBool, Ordering};

use bevy::app::{App, FixedUpdate, Update};
use bevy::ecs::schedule::{InternedScheduleLabel, ScheduleLabel};
use bevy::prelude::*;

/// **Which Bevy schedule the SIMULATION runs in** (netcode N0.1, the two clocks).
///
/// The engine has two clocks: the **sim tick** (the canonical timeline — N0.2
/// input streams and N0.4 state hashes key on its count) and the **frame/feel**
/// clock (raw render dt, driving presentation, device sampling, and per-player
/// feel-time effects). This resource names the schedule the *tick* clock lives
/// in. The construction-time `SimulationHost` selected by the runtime selects
/// `Update`, `FixedUpdate`, or the GGRS schedule; this lower crate intentionally
/// stores only the schedule label so it does not depend on the owner crate.
///
/// - [`Update`] (**default**) — frame-stepped, once per rendered frame.
/// - [`FixedUpdate`] — fixed-tick on Bevy's `Time<Fixed>` accumulator.
/// - a host-provided schedule such as `bevy_ggrs::GgrsSchedule` — driven by
///   that host's request/session machinery.
///
/// Bullet-time composes **inside** the tick, never with the tick rate: in
/// fixed-tick/GGRS modes `WorldTime::scaled_dt == TICK_DT × time_scale` while
/// cadence stays pinned. Nothing ever scales the accumulator.
///
/// # Reading it
///
/// Every plugin that registers a SIM system asks the app, rather than naming a
/// schedule literal:
///
/// ```ignore
/// impl Plugin for MySimPlugin {
///     fn build(&self, app: &mut App) {
///         let sim = app.sim_schedule();
///         app.add_systems(sim, my_system.in_set(Platformer2dSimulationPhaseMonolith::WorldPrep));
///     }
/// }
/// ```
///
/// Presentation, input-device, audio, and HUD plugins keep naming [`Update`]
/// literally — they are the feel clock, and that is the point of the split.
///
/// # The seal
///
/// The value is **sealed on first read**: once any plugin has asked for the
/// label, changing it panics rather than silently splitting the schedule graph
/// in half (some sim systems in `Update`, the rest in `FixedUpdate` — a
/// split-brain whose symptom is systems mysteriously never ordering against one
/// another). Set the mode BEFORE adding any sim plugin, or let
/// `PlatformerEnginePlugins` set it as its first act.
#[derive(Resource, Debug)]
pub struct SimSchedule {
    label: InternedScheduleLabel,
    /// Set once some plugin has committed systems to `label`.
    observed: AtomicBool,
}

impl Default for SimSchedule {
    fn default() -> Self {
        Self::new(Update)
    }
}

/// Host-owned marker for a historical replay pass through the simulation.
///
/// Ordinary render-frame and fixed-tick hosts leave this resource absent. A
/// rollback host raises it after loading historical state and clears it after
/// the host finishes servicing that rollback request batch. Diagnostic systems
/// use the marker to avoid treating replayed history as a new irreversible
/// event while gameplay systems continue to run normally.
#[derive(Resource, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SimulationReplayState {
    pub replaying_history: bool,
}

/// Run condition for diagnostics that should sample each authoritative tick but
/// not duplicate observations while a rollback host resimulates history.
pub fn simulation_pass_is_authoritative(replay: Option<Res<SimulationReplayState>>) -> bool {
    !replay.is_some_and(|replay| replay.replaying_history)
}

impl SimSchedule {
    pub fn new(label: impl ScheduleLabel) -> Self {
        Self {
            label: label.intern(),
            observed: AtomicBool::new(false),
        }
    }

    /// The sim schedule label, marking it sealed.
    pub fn label(&self) -> InternedScheduleLabel {
        self.observed.store(true, Ordering::Relaxed);
        self.label
    }

    /// Peek without sealing — for assertions and mode-dependent wiring.
    pub fn peek(&self) -> InternedScheduleLabel {
        self.label
    }

    /// True when the sim advances on `Time<Fixed>` rather than the render frame.
    pub fn is_fixed_tick(&self) -> bool {
        self.is(FixedUpdate)
    }

    /// Compare the configured host schedule without sealing it. This keeps the
    /// platformer vocabulary independent of optional schedule-owner crates such
    /// as `bevy_ggrs`; callers name the label they understand.
    pub fn is(&self, label: impl ScheduleLabel) -> bool {
        self.label == label.intern()
    }
}

/// App-level accessors for [`SimSchedule`]. See that type for the contract.
pub trait SimScheduleExt {
    /// The schedule SIM systems register into. Seals the value.
    fn sim_schedule(&mut self) -> InternedScheduleLabel;

    /// Choose the sim schedule. Panics if some plugin already read a different
    /// one — see [`SimSchedule`]'s seal.
    fn set_sim_schedule(&mut self, label: impl ScheduleLabel) -> &mut Self;

    /// True when the sim advances on `Time<Fixed>`. Does not seal.
    fn sim_is_fixed_tick(&self) -> bool;

    /// Compare the configured host schedule without sealing it.
    fn sim_is(&self, label: impl ScheduleLabel) -> bool;
}

impl SimScheduleExt for App {
    fn sim_schedule(&mut self) -> InternedScheduleLabel {
        self.init_resource::<SimSchedule>();
        self.world().resource::<SimSchedule>().label()
    }

    fn set_sim_schedule(&mut self, label: impl ScheduleLabel) -> &mut Self {
        let label = label.intern();
        if let Some(existing) = self.world().get_resource::<SimSchedule>() {
            assert!(
                !(existing.observed.load(Ordering::Relaxed) && existing.label != label),
                "sim schedule already sealed as {:?}; cannot change it to {:?} after a sim \
                 plugin has registered systems (that would split the sim schedule graph). \
                 Call set_sim_schedule before adding any sim plugin.",
                existing.label,
                label,
            );
        }
        self.insert_resource(SimSchedule {
            label,
            observed: AtomicBool::new(false),
        });
        self
    }

    fn sim_is_fixed_tick(&self) -> bool {
        self.world()
            .get_resource::<SimSchedule>()
            .is_some_and(SimSchedule::is_fixed_tick)
    }

    fn sim_is(&self, label: impl ScheduleLabel) -> bool {
        self.world()
            .get_resource::<SimSchedule>()
            .is_some_and(|schedule| schedule.is(label))
    }
}

/// Generic platformer runtime phases.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum PlatformerRuntimeSet {
    /// Build or refresh world-derived runtime inputs before actors tick.
    WorldPrep,
    /// Translate input/control intent into actor control frames.
    ControlInput,
    /// Integrate actors, held items, projectiles, and other gameplay bodies.
    ActorSimulation,
    /// Handle room unload/load, room-scoped cleanup, and authored room respawn.
    RoomLifecycle,
    /// Resolve damage, hitboxes, combat intents, and gameplay consequences.
    Combat,
    /// Publish simulation state to presentation-facing mirrors/caches.
    PresentationSync,
}

/// Startup-phase slot for the app's presentation setup (camera, root
/// UI scaffolding). Machinery that must initialize after presentation
/// setup (e.g. audio channel/cue loading) orders `.after(this set)`
/// instead of naming the app's setup system.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct PresentationSetupSet;

/// Startup-phase slot for the host's SIMULATION setup (room geometry,
/// player spawn, sim registries). The machinery-facing label for "the
/// world exists now": engine/host systems that must initialize after
/// the sim world is set up (e.g. attaching input components to the
/// spawned player) order `.after(this set)` instead of naming the
/// host's setup system — the same inversion as [`PresentationSetupSet`].
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct SimulationSetupSet;

/// Slot inside the `WorldPrep` boss tick chain where the content layer
/// inserts per-boss steering systems (e.g. the cut-rope boss tracking
/// its anvil). Configured `.after(tick_boss_brains_system)` and
/// `.before(update_ecs_bosses)` so a content system in this set runs at
/// exactly the point the old inline registration occupied.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct BossSteerSlot;

/// **The phases inside [`Platformer2dSimulationPhaseMonolith::Combat`], and the content slots between
/// them.**
///
/// The engine owns the combat spine — trigger, playback, materialize, resolve,
/// settle — and named content hangs on [`Self::ContentSpecials`] /
/// [`Self::ContentFlavor`] instead of being registered inline by the app.
///
/// The five engine phases were added 2026-07-27 for the same reason
/// [`PlayerInputSet`] was: everything that needed to run at a point in this chain
/// had to name a LEAF SYSTEM, which is what Task 6 rules out and what produced a
/// `GgrsSchedule` cycle when a caller could not tell which SET a named leaf lived
/// in. Naming the phases changed no order — this is the chain the runtime already
/// had — but `.in_set(CombatSet::Resolve)` is a complete statement of intent and
/// `.after(apply_feature_hit_events)` is a coupling to a name.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum CombatSet {
    /// **Intent becomes a started move.** Cooldowns decay, an attack gesture
    /// resolves to a verb, and a moveset move begins — for a player body and for
    /// a boss alike.
    Trigger,
    /// **The move clock advances and its volumes open and close.** Strike volumes
    /// are spawned and retired here, timed events fire, and the `BodyMelee`
    /// read-model is projected back for every consumer that still reads it.
    Playback,
    /// **Declarations become entities.** Effects execute, projectiles spawn and
    /// step, summons and programmatic actor spawns materialize. The phase that
    /// exists because a thing must EXIST before it can hit anything — the reason
    /// projectile presentation is stamped here rather than inherited later.
    Materialize,
    /// **Overlaps become damage.** Hitbox resolution, landed-hit marking, on-hit
    /// techniques, hitbox retirement, feature-hit application.
    Resolve,
    /// **Post-damage bookkeeping.** Victim staging and mount/rider link
    /// enforcement — everything that reads this tick's damage outcome rather than
    /// producing it.
    Settle,
    /// Per-boss special-attack Techniques (content-owned). Sits inside
    /// [`Self::Materialize`]: a special dispatched this frame reaches its content
    /// technique THIS frame.
    ContentSpecials,
    /// Post-damage encounter flavor (content-owned). Sits between
    /// [`Self::Resolve`] and [`Self::Settle`], so it observes this frame's
    /// alive-flag transitions before the bookkeeping runs.
    ContentFlavor,
}

/// The one umbrella set containing EVERY gameplay-simulation phase in the sim
/// schedule: all of [`Platformer2dSimulationPhaseMonolith`], the portal/projectile/combat sub-chains
/// nested inside them, and the pre-`CoreSimulation` strays (sim-id minting,
/// class-B log clear, portal carves).
///
/// Its purpose is the session gate: the whole gameplay simulation carries ONE
/// run condition
/// ([`crate::lifecycle::simulation_authorized`]) so a host that routes
/// gameplay through shell sessions gets a sleeping simulation — frozen tick
/// timeline included — at launcher/title/loading routes, while direct-entry
/// and headless apps (no [`crate::lifecycle::SessionGatedSimulation`] marker)
/// keep today's always-on behavior.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct GameplaySimulationRoot;

/// Coarse simulation ordering for sandbox gameplay systems.
///
/// This is the concrete sandbox app realization of the lower
/// [`PlatformerRuntimeSet`] vocabulary, plus Ambition-specific tail phases. It
/// lives here because host, runtime, content, sim-view, and render all need to
/// order against the same labels without depending on the actor-domain crate.
///
/// Every variant is nested inside [`GameplaySimulationRoot`]
/// (`configure_platformer2d_simulation_phases`), which carries the session-gate run condition.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum Platformer2dSimulationPhaseMonolith {
    /// Top-level set that contains the six sub-sets below. Kept as a
    /// distinct label so existing `.before/.after(CoreSimulation)`
    /// constraints from presentation/audio/HUD systems continue to
    /// cover the full main chain after this finer-grained split.
    CoreSimulation,

    /// Pre-player-tick world prep: LDtk hot-reload polling, feature
    /// ECS world overlay rebuild, feature ticks (hazards / actors /
    /// bosses). Feeds the collision world that the player simulation
    /// consults.
    WorldPrep,
    /// Pre-player-tick input pipeline: dev-edit sync, input-driven
    /// reset, gameplay timer decay, interaction buffer update, and
    /// the suspended-time fallback.
    PlayerInput,
    /// Main player tick: `player_control_system` + `player_simulation_system`
    /// (control + simulation) plus the post-sim damage / safe-respawn
    /// resolver.
    PlayerSimulation,
    /// Room transition detection + apply + per-room feature reset.
    RoomTransition,
    /// Attack lifecycle, projectile updates, and feature damage apply.
    Combat,
    /// Player ECS write-back + presentation timer decays.
    PresentationSync,

    /// Pickup collection and player heal request consumption.
    FeatureCollection,
    /// Actor/switch/chest/breakable interaction systems.
    FeatureInteraction,
    /// LDtk runtime spine index rebuild + parity check.
    LdtkRuntimeSpine,
    /// Moving platforms + encounter state + gameplay banner.
    EncounterSimulation,
    /// Auto-triggered cutscenes, cutscene drain/tick.
    Cutscene,
    /// Flag/quest/switch/boss/NPC/sfx gameplay-effect routing.
    GameplayEffects,
    /// Boss save sync, quest events, body-mode, room metadata, map sync.
    Progression,
    /// Sandbox reset request processor. Joined into the main post-core
    /// chain (between `Progression` and `FeatureViewSync`) because the
    /// reset path despawns every `RoomScopedEntity` (including every
    /// `RoomVisual`) and every feature sim entity, flips the active
    /// room, and re-lowers the start room's feature set through the
    /// installed placement registry — all mutations the cache must
    /// observe before presentation reads it.
    ResetProcessing,
    /// Rebuild the presentation-facing feature-view cache after every
    /// same-frame mutation to feature state.
    FeatureViewSync,
    /// Presentation-side container set for visual systems that read
    /// the feature view cache. Configured after [`Platformer2dSimulationPhaseMonolith::FeatureViewSync`].
    PresentationVisualSync,
    /// Trace recording + dump flush. Runs after CoreSimulation.
    Trace,
}

/// **The phases inside [`Platformer2dSimulationPhaseMonolith::PlayerInput`], as an orderable vocabulary.**
///
/// `PlayerInput` is one set containing a single long `.chain()`, and for a while
/// that meant anything needing to run at a particular point in it had to name a
/// LEAF SYSTEM — `.after(apply_worn_character_gameplay)`, `.before(..)` — which
/// the engine roadmap's Task 6 explicitly rules out ("runtime orders semantic
/// sets rather than naming leaf systems"). Two costs, both paid:
///
/// * A caller cannot tell which SET a named leaf lives in. On 2026-07-27 that
///   produced a `GgrsSchedule` before/after cycle: a system was ordered
///   `.in_set(Combat).before(apply_worn_character_gameplay)`, and that leaf turns
///   out to live in `PlayerInput`, which precedes `Combat`. Nothing in the call
///   site could have revealed it.
/// * An external consumer ordering against a leaf is coupled to a name the engine
///   is free to rename or split. Ordering against a phase is not.
///
/// The variants are the phases the chain already had; naming them changed no
/// order. They are chained in [`Platformer2dSimulationPhaseMonolith::PlayerInput`], so
/// `.in_set(PlayerInputSet::Persona)` is a complete statement of intent.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum PlayerInputSet {
    /// This frame's device input reaches the slot model: timers, interaction
    /// buffer, the controlled subject, `SlotControls`, the input-stream
    /// recorder, and the per-body `PlayerInputFrame` mirror.
    Device,
    /// Content-declared character data lands on the body — a registered
    /// `CharacterDefinition`'s moveset and silhouette — BEFORE the persona is
    /// constructed from it. Its own phase because the dependency is real:
    /// [`Self::Persona`] treats what this projects as its baseline.
    CharacterProjection,
    /// The canonical persona derive: action set, moveset and identity kit built
    /// together, with equipment grants overlaid. The phase most external code
    /// wants to order against, and the one that was hardest to name before.
    Persona,
    /// The universal-brain seam: this frame's slot input becomes each controlled
    /// body's `ActorControl`.
    Brain,
    /// Everything that may VETO or rewrite the control frame the brain just
    /// wrote — scripted blanking, worn-kit gating, a sustained shield. After the
    /// brain, before anything reads the frame.
    ControlGate,
    /// Body-mode policy (crouch / morph / climb) and the actor pose sync, both
    /// of which consume the finished control frame.
    BodyMode,
}

/// **The phases inside [`Platformer2dSimulationPhaseMonolith::Progression`], as
/// an orderable vocabulary.**
///
/// Same shape and same reason as [`PlayerInputSet`], one phase later. Progression
/// was a single `.chain()` of seventeen systems, and every slot that had to sit
/// at a particular point in it pinned itself with `.after(<leaf system>)` /
/// `.before(<leaf system>)` — eight such orderings, the largest concentration in
/// the runtime after `PlayerInputSet` fixed the input phase.
///
/// ⚠ **the boundaries are not arbitrary: they are where the pins already were.**
/// Two slots (`ContentEncounterScriptSet`, `ambition_encounter::EncounterLifecycleSet`)
/// anchored INSIDE the boss group, both against `update_encounter_progress` —
/// which is why the boss work is two phases rather than one. A vocabulary that
/// could not express an existing anchor would have forced the anchor to stay a
/// leaf, and the leaf is the thing being removed.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProgressionSet {
    /// Boss encounters advance: mount-death notification, encounter + entity
    /// sync, and the participant-liveness refresh that ends with
    /// `update_encounter_progress`. Everything that DECIDES what the encounter
    /// is this frame.
    BossAdvance,
    /// What the advanced encounter then DOES: falling hazards, encounter
    /// scripts, death payloads, and the phase-transition feedback that closes
    /// the boss group.
    BossHazards,
    /// Save → ECS mirrors for actors and bosses, once the encounter state they
    /// mirror is settled.
    SaveMirror,
    /// Quest events pushed and then applied. `apply_quest_advance_events` is the
    /// one system in this phase that lives in `ambition_persistence` rather than
    /// the monolith, which is why a slot ordering against it had to name a leaf
    /// from a third crate.
    Quest,
    /// Room metadata, music request and portal phase timers — the world catching
    /// up with the progression that just happened.
    WorldSync,
    /// Map-menu visit tracking and the map's own save mirror. Last, because the
    /// dev inspector mirror anchors after it.
    Map,
}

/// **The phases inside [`Platformer2dSimulationPhaseMonolith::RoomTransition`].**
///
/// Same shape, and same reason, as [`PlayerSimulationSet`]: this set carried an
/// ordering slot described in prose — *"the host's transition APPLY slots in
/// between"* — with the two systems it slots between named as leaves from another
/// crate. The engine fills that slot itself now (the readiness transaction and
/// the authorized commit), which makes it more important to name and not less: a
/// game replacing the transition policy needs somewhere to put it.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum RoomTransitionSet {
    /// **Has a transition been requested?** Edge/door/walk detection publishes the
    /// intent; nothing has moved yet.
    Detect,
    /// **The transaction: readiness, authorization, commit.** The slot the module
    /// docs used to describe in a sentence. A game that replaces the transition
    /// policy replaces what is here.
    Apply,
    /// **Per-room feature reset** over the unified actor cluster, once the
    /// transition has committed. `ContentRoomResetSet` follows it, and generic
    /// plugins (gravity, portal) order against that SET rather than against any
    /// content system.
    Reset,
}

/// **The movement anchor inside [`Platformer2dSimulationPhaseMonolith::WorldPrep`].**
///
/// Unlike [`PlayerInputSet`] and [`CombatSet`], this is deliberately NOT a full
/// decomposition of its set. `WorldPrep` is the biggest chain in the engine, its
/// actor and boss sub-chains interleave through two anchors, and its own comments
/// record a before/after CYCLE that panicked the app at startup in 2026-07-05.
/// Splitting it wholesale is a large change with a startup-crash failure mode and
/// almost no consumer-facing payoff — an audit of every `.before`/`.after` naming
/// a `WorldPrep` system found exactly ONE leaf reached from another crate.
///
/// That one is worth naming, because it is the question every game asks: *does my
/// system run before bodies move, or after they have landed?* Mary-O's stomp
/// classifier asks it (a stomp is classified from RESOLVED positions) and so does
/// the portal transit schedule.
///
/// These are placement sets around the existing anchor, not a restructuring:
/// [`Self::Integrate`] contains the one movement system, and the two neighbours
/// are where a consumer joins.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum WorldPrepSet {
    /// **Before any non-boss body moves.** A system here still sees last frame's
    /// resolved positions and can change what the movement phase will sweep — a
    /// posed collision box, a routed limb intent, a portal carve.
    BeforeIntegrate,
    /// **The ONE movement phase** for every non-boss sim body: actor bodies and
    /// the home/player body integrate through the same engine entry.
    Integrate,
    /// **After bodies have landed.** Positions and contacts are resolved, which is
    /// what a contact classifier needs — classifying a stomp from pre-movement
    /// positions is a stomp that reads the wrong frame.
    AfterIntegrate,
    /// **The shared body-contact damage pass**, as a boundary a consumer can order
    /// against.
    ///
    /// Deliberately NOT chained after [`Self::AfterIntegrate`]. Chaining it would
    /// add an ordering that does not exist today — everything in `AfterIntegrate`
    /// would suddenly be required to precede contact damage — and a refactor that
    /// silently adds edges is how a schedule acquires constraints nobody chose.
    /// It is a label on an existing system, and a consumer that needs to run
    /// before contact damage says so itself.
    ///
    /// Mary-O's stomp rules are the reason it exists: a stomp must resolve the
    /// enemy (snake to shell, walker to dead) in time for this pass to skip it, or
    /// the stomper is also hurt.
    ContactDamage,
}

/// **The phases inside [`Platformer2dSimulationPhaseMonolith::PlayerSimulation`].**
///
/// Third of the six `Platformer2dSimulationPhaseMonolith` phases to get named sub-sets (after
/// [`PlayerInputSet`] and [`CombatSet`]), for the same reason and with the same
/// rule: naming them changed no order.
///
/// This one had an explicit LEAF-NAMED SLOT in its docs — *"the
/// home-reset/presentation pair pins `.after(release_possession_if_target_lost)
/// .before(apply_player_hit_events)`"*. A documented slot is still a coupling: a
/// host reading that sentence has to trust it stays true, and nothing checks that
/// it does. [`Self::PostPossession`] IS that slot, and a host joins it by name.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum PlayerSimulationSet {
    /// **Who is driving which body.** Possession triggers and releases; a target
    /// that stopped existing hands control back.
    Possession,
    /// **The host's slot between control settling and damage landing.** Home
    /// reset policy and player presentation live here in Ambition: they read the
    /// movement phase's hand-off and move no body, so they must run once
    /// possession is settled and before the frame's damage is applied.
    ///
    /// Empty in a host that registers nothing, which is what makes it a slot
    /// rather than a phase the engine owes systems to.
    PostPossession,
    /// **This frame's damage and death facts applied to the player body.**
    /// Includes the kernel's own death path (pit, drown, tile hazard), which
    /// never reaches the hit resolver and publishes here instead.
    Outcome,
}

/// **The phases inside [`Platformer2dSimulationPhaseMonolith::FeatureInteraction`].**
///
/// Same shape and same reason as [`ProgressionSet`] and [`PlayerInputSet`], with
/// one difference that is the point of the whole vocabulary: this phase held ONE
/// anonymous `.chain()` of ten systems spanning **four domains** —
/// `conversation`, the interaction feature systems, the NPC cast, and
/// `encounter` — and every cross-domain interleave in it was load-bearing and
/// recorded ONLY as adjacency in a tuple plus prose at the call site.
///
/// ⭐ **the generalisable finding this enum exists for: a module with zero
/// inward imports can still be pinned by the SCHEDULE.** `conversation` measured
/// 1,836 lines with zero `crate::` edges in either direction and its own header
/// claimed *"the carve is a Cargo.toml"* — but three of its systems sat wedged
/// between `interact_ecs_actors_and_switches` and the chest systems in a chain
/// it could not name, so lifting it out of the crate would have silently
/// dissolved the ordering. An import graph cannot see a `.chain()`.
///
/// ⚠ **naming these changed no order.** The variants are the boundaries the
/// prose comments already drew; each one carries the sentence that justified it.
/// The chain is declared once (`FeatureInteractionSchedulePlugin`) and every
/// domain plugin only says which phase it belongs to — so `conversation` states
/// its own placement against a vocabulary that lives BELOW the monolith and
/// survives the carve.
///
/// ⛔ **`.chain()` on the set list, not `(a, b).before(c)`.** `(A, B).before(C)`
/// orders both A and B before C and says nothing about A vs C's siblings; only a
/// chain states a total order. And because Bevy inserts sync points on
/// dependency edges after flattening sets to systems, the `ApplyDeferred`
/// boundaries the original per-system chain provided are preserved — which
/// matters at [`Self::SwitchIndex`], whose whole job is to see what the systems
/// before it just spawned or despawned.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum FeatureInteractionSet {
    /// **The narrative running out of lines is an INPUT to the simulation**, and
    /// it lands before anything judges the conversation for separation —
    /// otherwise a conversation that ended this frame gets barked about on its
    /// way out.
    NarrativeIntake,
    /// **Somebody pressed Interact**: actors and switches. The phase that OPENS
    /// a conversation, which is why [`Self::Continuity`] may not precede it.
    Actuate,
    /// **The break rule.** ⚠ AFTER [`Self::Actuate`]: a dialogue opened this
    /// frame must not be judged for separation before the bodies that opened it
    /// have been read. Both use the same `strict_intersects` reach, so a
    /// conversation cannot begin and immediately break.
    Continuity,
    /// **The CAST half of the break**: continuity said who should speak, this
    /// says what they say. Immediately after [`Self::Continuity`], so the bubble
    /// lands on the same tick the conversation ended.
    ///
    /// ⭐ **a slot `conversation` names and the cast fills.** The set is declared
    /// by the ordering vocabulary and its member lives in `features::npcs`,
    /// which is the temporal twin of the `ConversationCutBark` message port:
    /// continuity owns WHEN, the cast owns WHAT.
    CutBarkCast,
    /// **The hold, PROJECTED** — whatever [`Self::Continuity`] decided (a break,
    /// a body that stopped existing, or nothing at all), the world is made to
    /// match the authority on the same frame. ⛔ it is not a "release": it both
    /// takes and releases the hold, because a projection that only let go would
    /// be a second rule about when to hold.
    HoldProjection,
    /// **Interactable world objects**: chests opening, breakables breaking,
    /// falling chests falling, and the save → switch mirror. Downstream of
    /// [`Self::Actuate`] because that is what opens a chest.
    WorldObjects,
    /// **The encounter switch index, rebuilt last.** It is a cache of
    /// `SwitchFeature + SwitchOn` over the whole world, so it must observe every
    /// switch mutation this phase makes — the Interact toggle in
    /// [`Self::Actuate`] and the save mirror in [`Self::WorldObjects`] — or the
    /// encounter arms a frame late off a stale index.
    SwitchIndex,
}

/// Bevy run condition: returns `true` only in [`GameMode::Playing`].
///
/// Use this to gate simulation systems that must not run while paused,
/// in dialogue, in a room transition, or in a cutscene.
///
/// ```ignore
/// app.add_systems(Update, my_system.run_if(gameplay_allowed));
/// ```
pub fn gameplay_allowed(mode: Res<State<GameMode>>) -> bool {
    mode.get().allows_gameplay()
}

/// Bevy run condition: complement of [`gameplay_allowed`]. True in any mode
/// that suspends gameplay (paused, dialogue, room transition, cutscene).
///
/// Use this to gate the small set of systems that should only run while
/// gameplay is suspended, such as forcing world time to zero.
/// ⚠ **this asks [`GameMode::stops_the_world`], not `allows_gameplay`.** It used
/// to ask the latter, which meant a conversation froze every body in the level
/// including the ones nobody was talking to. On a couch that is player two
/// stopped mid-jump because player one walked into an NPC; in single player it
/// is every NPC and hazard in the room holding still for a text box.
pub fn gameplay_suspended(
    mode: Res<State<GameMode>>,
    dialogue_policy: Option<Res<DialogueStopsTheWorld>>,
) -> bool {
    mode.get()
        .stops_the_world(dialogue_policy.map(|p| *p).unwrap_or_default())
}

/// Coarse gameplay/session mode shared by runtime, input, host, and render.
///
/// `GameMode` is intentionally broader than per-entity behavior. It belongs
/// with the schedule vocabulary because it answers the same question as the
/// runtime sets: which groups of systems may mutate gameplay state this frame?
/// Enemy, chest, boss, and dialogue state machines can layer narrower state on
/// top of this coarse mode without teaching every mechanic how to pause itself.
#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default, Reflect)]
pub enum GameMode {
    /// Normal gameplay: controlled actors, NPCs, enemies, hazards, room
    /// triggers, and pickups may consume gameplay inputs and advance
    /// simulation time.
    #[default]
    Playing,
    /// Simulation is stopped, but pause/menu input and developer tools remain
    /// responsive. Gameplay actions are deliberately not converted into an
    /// engine `InputState` while this mode is active.
    Paused,
    /// Reserved for NPC conversations and other text-driven interactions.
    Dialogue,
    /// Reserved for scripted room loads or door/edge transition presentation.
    RoomTransition,
    /// Reserved for future cutscenes or scripted set pieces.
    Cutscene,
}

/// **Does a conversation stop the world?**
///
/// Jon, 2026-08-03: *"dialogue should have the option to stop the world. I'm not
/// decided on what I want it to do in game."* So both are expressible and this
/// is the policy; Jon decided the DEFAULT on 2026-08-06, and it is per-seat —
/// a conversation claims the talker's input and leaves the world running.
///
/// An experience that wants the old modal beat back sets this to `true` and gets
/// exactly the previous behaviour. Nothing else has to change, because the
/// world-stop and the input claim were already two different mechanisms wearing
/// one switch.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DialogueStopsTheWorld(pub bool);

impl GameMode {
    /// Whether GAMEPLAY INPUT may route this frame.
    ///
    /// ⚠ **not the same question as whether the world is running** — see
    /// [`Self::stops_the_world`]. They were one predicate until 2026-08-06, and
    /// that conflation is what made "a conversation the world keeps running
    /// through" inexpressible.
    pub fn allows_gameplay(self) -> bool {
        matches!(self, Self::Playing)
    }

    /// Whether the SIM CLOCK freezes in this mode.
    ///
    /// `Paused` is the pause. `RoomTransition` and `Cutscene` are genuinely
    /// global — a room is loading, or a scripted beat owns the screen — and are
    /// explicitly NOT the same question as dialogue.
    ///
    /// Dialogue answers `false` by default and defers to
    /// [`DialogueStopsTheWorld`] when an experience has an opinion.
    pub fn stops_the_world(self, dialogue_policy: DialogueStopsTheWorld) -> bool {
        match self {
            Self::Playing => false,
            Self::Dialogue => dialogue_policy.0,
            Self::Paused | Self::RoomTransition | Self::Cutscene => true,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Playing => "playing",
            Self::Paused => "paused",
            Self::Dialogue => "dialogue",
            Self::RoomTransition => "room-transition",
            Self::Cutscene => "cutscene",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_gameplay_only_in_playing() {
        assert!(GameMode::Playing.allows_gameplay());
        assert!(!GameMode::Paused.allows_gameplay());
        assert!(!GameMode::Dialogue.allows_gameplay());
        assert!(!GameMode::RoomTransition.allows_gameplay());
        assert!(!GameMode::Cutscene.allows_gameplay());
    }

    #[test]
    fn default_is_playing() {
        assert_eq!(GameMode::default(), GameMode::Playing);
    }

    #[test]
    fn gameplay_suspended_is_complement_of_allowed() {
        for mode in [
            GameMode::Playing,
            GameMode::Paused,
            GameMode::Dialogue,
            GameMode::RoomTransition,
            GameMode::Cutscene,
        ] {
            assert_eq!(mode.allows_gameplay(), !gameplay_suspended_for_value(mode));
        }
    }

    #[test]
    fn labels_are_unique_and_non_empty() {
        let labels = [
            GameMode::Playing.label(),
            GameMode::Paused.label(),
            GameMode::Dialogue.label(),
            GameMode::RoomTransition.label(),
            GameMode::Cutscene.label(),
        ];
        for label in labels {
            assert!(!label.is_empty());
        }
        let unique: std::collections::HashSet<_> = labels.iter().collect();
        assert_eq!(unique.len(), labels.len(), "labels must be unique");
    }

    fn gameplay_suspended_for_value(mode: GameMode) -> bool {
        !mode.allows_gameplay()
    }
}
