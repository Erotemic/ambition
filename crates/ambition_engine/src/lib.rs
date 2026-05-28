//! Ambition Engine
//!
//! Reusable Bevy-native simulation primitives for Ambition. The crate
//! owns the testable gameplay rules a game or story crate should be
//! able to assemble without rewriting details: kinematic player +
//! actor movement, AABB collision semantics, ability gates, combat
//! hitboxes / hurtboxes / damage routing, character AI evaluators,
//! kinematic paths, ledge-grab probes, projectile bodies, quest /
//! save state machines, and the `World` collision/water/climbable
//! room data.
//!
//! The engine does **not** own authored content (LDtk entities,
//! per-room authored Vecs, sandbox dispatch tables) — those live on
//! the sandbox side. Authoring payloads that pass through the engine
//! (`Pickup`, `Chest`, `Breakable`, `Interactable`, `EnemyBrain`,
//! `BossBrain`, `DebugLabel`, `DamageVolume`) are typed config the
//! engine carries between LDtk and sandbox dispatch without
//! simulating against the variants.
//!
//! Story/sandbox crates provide data, presentation, and input wiring.
//! The engine may depend on small Bevy crates (e.g. `bevy_math`) when
//! they provide battle-tested primitives that beat bespoke versions.

pub mod abilities;
pub mod actor;
pub mod actor_control;
pub mod attack_choreography;
pub mod boss_encounter;
pub mod character_ai;
pub mod combat;
pub mod combat_slots;
pub mod player_clusters;
pub mod cutscene;
pub mod debug;
pub mod geometry;
pub mod interaction;
pub mod kinematic;
pub mod ledge_grab;
pub mod movement;
pub mod player_state;
pub mod projectile;
pub mod quest;
pub mod save;
pub mod scalar;
pub mod world;

// Re-export the public surface so story/sandbox crates can treat the engine as
// the main mechanics API while the internals stay organized by concern.
pub use abilities::AbilitySet;
pub use actor::{
    Actor, ActorFaction, ActorKind, BossBrain, EnemyBrain, Health, KinematicPath,
    KinematicPathMode, RespawnPolicy,
};
pub use actor_control::{ActorControlFrame, ActorFireRequest};
pub use attack_choreography::{
    evaluate_choreography, seed_from_id, AerialRole, AttackChoreography, ChoreographyAction,
    ChoreographyInput, ChoreographyPhase, ChoreographyState, ChoreographyTick,
};
pub use bevy_math::Vec2;
pub use boss_encounter::{
    BossEncounterEvent, BossEncounterPhase, BossEncounterSpec, BossEncounterState,
};
pub use character_ai::{
    evaluate_character_ai, evaluate_character_ai_output, CharacterAiIntent, CharacterAiMode,
    CharacterAiOutput, CharacterAiSnapshot,
};
pub use combat::{
    attack_hitbox, attack_spec, resolve_attack_intent, AttackIntent, AttackPhase, AttackSpec,
    Damage, DamageKind, DamageVolume, Hitbox, Hurtbox,
};
pub use combat_slots::{assign_slots, CombatSlot, CombatSlotBoard, SlotKind, SlotRequest};
pub use cutscene::{CutsceneBeat, CutsceneEvent, CutsceneRuntime, CutsceneScript};
pub use debug::{DebugLabel, DebugLabelKind};
pub use geometry::{aabb_from_min_size, Aabb, AabbExt};
pub use interaction::{
    Breakable, BreakableCollision, BreakableState, BreakableTrigger, Chest, ChestState,
    Interactable, InteractionKind, Pickup, PickupKind,
};
pub use kinematic::{step_kinematic, KinematicBody, KinematicInputs, KinematicTuning};
pub use ledge_grab::{
    probe_ledge_grab, LedgeContact, LedgeGetupKind, LedgeGrabState, LEDGE_CLIMB_TIME,
    LEDGE_GRAB_INVULN_TIME, LEDGE_MIN_CLIMB_DELAY, LEDGE_ROLL_OVERSHOOT, LEDGE_ROLL_TIME,
    LEDGE_TOWARD_CLIMB_DELAY,
};
pub use movement::{
    blink_destination, blink_destination_clusters, blink_destination_to_point,
    blink_destination_to_point_clusters, default_player_body_size, update_player,
    update_player_control, update_player_control_with_tuning, update_player_simulation,
    update_player_simulation_with_tuning, update_player_with_tuning, BlinkEvent, ComboMark,
    FrameEvents, InputState, LedgeMomentumTuning, MovementOp, MovementTuning, Player, AIR_ACCEL,
    AIR_FRICTION, AIR_JUMPS, BLINK_COOLDOWN, BLINK_DISTANCE, BLINK_HOLD_THRESHOLD, COYOTE_TIME,
    DASH_BUFFER, DASH_COOLDOWN, DASH_SPEED, DASH_TIME, DEFAULT_PLAYER_BODY_HEIGHT,
    DEFAULT_PLAYER_BODY_WIDTH, DEFAULT_TUNING, DODGE_ROLL_COOLDOWN, DODGE_ROLL_SPEED,
    DODGE_ROLL_TIME, DOUBLE_JUMP_SPEED, FAST_FALL_ACCEL, FAST_FALL_SPEED, FLIGHT_ACCEL,
    FLIGHT_DRAG, FLIGHT_HOVER_HZ, FLIGHT_HOVER_SPEED, FLIGHT_TERMINAL_SPEED, GRAVITY,
    GROUND_FRICTION, JUMP_BUFFER, JUMP_SPEED, MAX_FALL_SPEED, MAX_RUN_SPEED, PARRY_WINDOW_TIME,
    POGO_SPEED, PRECISION_BLINK_AIM_SPEED, PRECISION_BLINK_DISTANCE, RUN_ACCEL, SLASH_RECOIL,
    WALL_CLIMB_SPEED, WALL_JUMP_X, WALL_SLIDE_SPEED,
};
pub use player_state::{
    classify_player_safety, try_change_body_mode, try_change_body_mode_clusters, BodyMode,
    BodyShape, LocomotionState, PlayerSafetyVerdict, ResourceMeter,
};
pub use projectile::{
    FireballChargeTuning, MotionDirection, MotionInputBuffer, MotionSample, ProjectileBody,
    ProjectileFaction, ProjectileKind, ProjectileSolidHit, ProjectileSpawner, ProjectileSpec,
    SpawnFailure,
};
pub use quest::{QuestAdvanceEvent, QuestSpec, QuestState, QuestStepCondition, QuestStepSpec};
pub use save::{
    PersistedBossDefeat, PersistedDialogVisit, PersistedEncounter, PersistedEncounterState,
    PersistedFlag, PersistedQuest, PersistedQuestState, PersistedSwitch, SandboxSaveData,
    CURRENT_SAVE_VERSION,
};
pub use player_clusters::{
    PlayerAbilities as EnginePlayerAbilities, PlayerActionBuffer as EnginePlayerActionBuffer,
    PlayerBlinkState as EnginePlayerBlinkState,
    PlayerBodyModeState as EnginePlayerBodyModeState,
    PlayerComboTrace as EnginePlayerComboTrace, PlayerDashState as EnginePlayerDashState,
    PlayerDodgeState as EnginePlayerDodgeState,
    PlayerEnvironmentContact as EnginePlayerEnvironmentContact,
    PlayerFlightState as EnginePlayerFlightState,
    PlayerGroundState as EnginePlayerGroundState, PlayerJumpState as EnginePlayerJumpState,
    PlayerKinematics as EnginePlayerKinematics, PlayerLedgeState as EnginePlayerLedgeState,
    PlayerLifetime as EnginePlayerLifetime, PlayerMana as EnginePlayerMana,
    PlayerOffense as EnginePlayerOffense, PlayerShieldState as EnginePlayerShieldState,
    PlayerWallState as EnginePlayerWallState,
};
pub use scalar::approach;
pub use world::{
    BlinkWallTier, Block, BlockKind, ClimbableContact, ClimbableKind, ClimbableRegion,
    ClimbableSpec, WaterContact, WaterKind, WaterRegion, WaterVolumeSpec, World,
};
