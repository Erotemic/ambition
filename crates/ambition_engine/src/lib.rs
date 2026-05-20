//! Ambition Engine
//!
//! This crate is the reusable Bevy-native mechanics layer for Ambition. It owns
//! features a game or story crate should be able to assemble without rewriting
//! details: movement, collision semantics, ability gates, combat hitboxes,
//! enemies, room geometry, generated audio/music specs, and testable gameplay
//! rules.
//!
//! Story/sandbox crates should generally provide data, presentation, and input
//! wiring. The engine may depend on small Bevy crates such as `bevy_math` when
//! they provide battle-tested primitives that are better than bespoke versions.

pub mod abilities;
pub mod actor;
pub mod attack_choreography;
pub mod boss_encounter;
pub mod boss_patterns;
pub mod character_ai;
pub mod combat;
pub mod combat_slots;
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
pub use attack_choreography::{
    evaluate_choreography, seed_from_id, AerialRole, AttackChoreography, ChoreographyAction,
    ChoreographyInput, ChoreographyPhase, ChoreographyState, ChoreographyTick,
};
pub use bevy_math::Vec2;
pub use boss_encounter::{
    BossEncounterEvent, BossEncounterPhase, BossEncounterSpec, BossEncounterState,
};
pub use boss_patterns::{
    ActiveBossBeat, ArenaAnchor, BossAttackKind, BossBeatPhase, BossMovementKind,
    BossPatternSchedule, BossPatternStep,
};
pub use character_ai::{
    evaluate_character_ai, evaluate_character_ai_output, CharacterAiIntent, CharacterAiMode,
    CharacterAiOutput, CharacterAiSnapshot,
};
pub use combat::{
    attack_hitbox, attack_spec, resolve_attack_intent,
    AttackIntent, AttackPhase, AttackSpec, Damage, DamageKind, DamageVolume, Hitbox, Hurtbox,
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
    probe_ledge_grab, LedgeContact, LedgeGrabState, LEDGE_CLIMB_TIME, LEDGE_MIN_CLIMB_DELAY,
    LEDGE_TOWARD_CLIMB_DELAY,
};
pub use movement::{
    blink_destination, blink_destination_to_point, default_player_body_size, update_player,
    update_player_control, update_player_control_with_tuning, update_player_simulation,
    update_player_simulation_with_tuning, update_player_with_tuning, BlinkEvent, ComboMark,
    FrameEvents, InputState, MovementOp, MovementTuning, Player, AIR_ACCEL, AIR_FRICTION,
    AIR_JUMPS, BLINK_COOLDOWN, BLINK_DISTANCE, BLINK_HOLD_THRESHOLD, COYOTE_TIME, DASH_BUFFER,
    DASH_COOLDOWN, DASH_SPEED, DASH_TIME, DEFAULT_PLAYER_BODY_HEIGHT, DEFAULT_PLAYER_BODY_WIDTH,
    DEFAULT_TUNING, DODGE_ROLL_COOLDOWN, DODGE_ROLL_SPEED, DODGE_ROLL_TIME, DOUBLE_JUMP_SPEED,
    FAST_FALL_ACCEL, FAST_FALL_SPEED, FLIGHT_ACCEL, FLIGHT_DRAG, FLIGHT_HOVER_HZ,
    FLIGHT_HOVER_SPEED, FLIGHT_TERMINAL_SPEED, GRAVITY, GROUND_FRICTION, JUMP_BUFFER, JUMP_SPEED,
    MAX_FALL_SPEED, MAX_RUN_SPEED, PARRY_WINDOW_TIME, POGO_SPEED, PRECISION_BLINK_AIM_SPEED,
    PRECISION_BLINK_DISTANCE, RUN_ACCEL, SLASH_RECOIL, WALL_CLIMB_SPEED, WALL_JUMP_X,
    WALL_SLIDE_SPEED,
};
pub use player_state::{
    classify_player_safety, try_change_body_mode, BodyMode, BodyShape, LocomotionState,
    PlayerSafetyVerdict, ResourceMeter,
};
pub use projectile::{
    FireballChargeTuning, MotionDirection, MotionInputBuffer, MotionSample, ProjectileBody,
    ProjectileKind, ProjectileSolidHit, ProjectileSpawner, ProjectileSpec, SpawnFailure,
};
pub use quest::{QuestAdvanceEvent, QuestSpec, QuestState, QuestStepCondition, QuestStepSpec};
pub use save::{
    PersistedBossDefeat, PersistedEncounter, PersistedEncounterState, PersistedFlag,
    PersistedQuest, PersistedQuestState, PersistedSwitch, SandboxSaveData, CURRENT_SAVE_VERSION,
};
pub use scalar::approach;
pub use world::{
    BlinkWallTier, Block, BlockKind, ClimbableContact, ClimbableKind, ClimbableRegion,
    ClimbableSpec, WaterContact, WaterKind, WaterRegion, WaterVolumeSpec, World,
};
