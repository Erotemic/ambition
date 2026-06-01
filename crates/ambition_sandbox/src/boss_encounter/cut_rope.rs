//! Cut-rope boss arena rules.
//!
//! The arena is authored in LDtk as ordinary `Prop` entities named/kinded
//! `cut_rope_rope` and `cut_rope_anvil`, plus a `BossSpawn` whose behavior id
//! is `smirking_behemoth_boss`. This system keeps the one-off mechanic tied to
//! authored level data rather than hard-coded coordinates: cutting the rope prop
//! starts the anvil prop falling; the anvil impact forces the boss encounter
//! through the normal death pipeline.

use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::assets::game_assets::GameAssets;
use crate::audio::SfxMessage;
use crate::boss_encounter::{force_boss_death, BossEncounterRegistry};
use crate::brain::ActorControl;
use crate::brain::BossAttackState;
use crate::config::world_to_bevy;
use crate::engine_core::{self as ae, AabbExt};
use crate::features::{
    actor_component_snapshot, ActorPose, ActorRuntime, BossFeature, BossRuntime, DamageableVolumes,
    EnemyActorBundle, FeatureAabb, FeatureBaseBundle, FeatureId, FeatureName, FeatureSimEntity,
    GameplayBanner, HitEvent, HitSource, PogoPolicy, PogoTargetVolumes, ResetRoomFeaturesEvent,
};
use crate::presentation::character_sprites::{
    build_character_sprite, feet_anchor_for, CharacterAnimator,
};
use crate::presentation::fx::{
    ExplosionKind, ExplosionRequest, FireworksRequest, ParticleKind, VfxMessage,
};
use crate::presentation::rendering::PropVisual;
use crate::rooms::{PropSpec, RoomSet};
use crate::world::physics::{DebrisBurstMessage, PhysicsDebrisCue};

pub const CUT_ROPE_BOSS_ID: &str = "smirking_behemoth_boss";
pub const CUT_ROPE_VICTORY_NPC_ID: &str = "smirking_behemoth_victory_npc";
pub const CUT_ROPE_VICTORY_NPC_DIALOGUE_ID: &str = "smirking_behemoth_victory_npc";
const CUT_ROPE_ROOM_ID: &str = "you_have_to_cut_the_rope";
const CUT_ROPE_VICTORY_NPC_NAME: &str = "The Rope Appreciator";
const CUT_ROPE_VICTORY_NPC_W: f32 = 28.0;
const CUT_ROPE_VICTORY_NPC_H: f32 = 48.0;
const ROPE_KIND: &str = "cut_rope_rope";
const ANVIL_KIND: &str = "cut_rope_anvil";
const PIANO_KIND: &str = "cut_rope_piano";
const ANVIL_GRAVITY: f32 = 1400.0;
const ANVIL_TERMINAL_SPEED: f32 = 920.0;
const ANVIL_Z_OFFSET: f32 = 0.75;
const ROPE_ALIGNMENT_TOLERANCE: f32 = 42.0;
const ROPE_LURE_SPEED: f32 = 150.0;
const ROPE_SPARK_INTERVAL: f32 = 0.22;

pub fn is_cut_rope_boss(id: &str) -> bool {
    id == CUT_ROPE_BOSS_ID
}

#[derive(Message, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CutRopeRoomReplayRequested;

/// Latched by the Yarn `<<reset_cut_rope_room>>` command once the player chooses the
/// replay option. The actual room reset intentionally waits until the dialog UI has
/// closed, so the final NPC line remains visible until the player dismisses it.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PendingCutRopeRoomReplay {
    pub requested: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CutRopeHeavyObjectKind {
    Anvil,
    Piano,
}

impl CutRopeHeavyObjectKind {
    const fn prop_kind(self) -> &'static str {
        match self {
            Self::Anvil => ANVIL_KIND,
            Self::Piano => PIANO_KIND,
        }
    }

    const fn display_name(self) -> &'static str {
        match self {
            Self::Anvil => "anvil",
            Self::Piano => "piano",
        }
    }
}

const CUT_ROPE_HEAVY_OBJECT_CYCLE: [CutRopeHeavyObjectKind; 2] =
    [CutRopeHeavyObjectKind::Anvil, CutRopeHeavyObjectKind::Piano];

/// Tracks which heavy object is currently hanging from the cut-rope trap.
///
/// This lives outside [`CutRopeBossArenaState`] so leaving/re-entering the room
/// can rebuild transient fall/rope state without changing the chosen prop. The
/// choice advances only on an actual room reset, which makes the variation
/// deterministic and easy to test.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CutRopeHeavyObjectCycle {
    index: usize,
}

impl Default for CutRopeHeavyObjectCycle {
    fn default() -> Self {
        Self { index: 0 }
    }
}

impl CutRopeHeavyObjectCycle {
    fn current(&self) -> CutRopeHeavyObjectKind {
        CUT_ROPE_HEAVY_OBJECT_CYCLE[self.index % CUT_ROPE_HEAVY_OBJECT_CYCLE.len()]
    }

    fn advance(&mut self) {
        self.index = (self.index + 1) % CUT_ROPE_HEAVY_OBJECT_CYCLE.len();
    }

    /// Stable Yarn-facing id for the currently hung heavy object.
    pub fn current_dialogue_id(&self) -> &'static str {
        self.current().display_name()
    }
}

/// Convert a pending dialogue-authored replay into the normal replay message after
/// the final dialog line has been dismissed.
pub fn emit_cut_rope_room_replay_after_dialogue_closes(
    dialogue: Res<crate::dialog::DialogState>,
    mut pending: ResMut<PendingCutRopeRoomReplay>,
    mut replay_requests: MessageWriter<CutRopeRoomReplayRequested>,
) {
    if !pending.requested || dialogue.active() {
        return;
    }
    pending.requested = false;
    replay_requests.write(CutRopeRoomReplayRequested);
}

/// Reset the Smirking Behemoth encounter state so the room can be replayed in-place.
///
/// This deliberately does not move the player or mutate ECS feature entities directly. Callers
/// should also trigger the normal room reset path, which emits `ResetRoomFeaturesEvent`; this
/// helper only clears the boss encounter/save state that would otherwise keep the boss retired.
pub fn reset_cut_rope_boss_attempt(
    registry: &mut BossEncounterRegistry,
    save: Option<&mut crate::persistence::save::SandboxSave>,
    music_request: Option<&mut crate::encounter::BossEncounterMusicRequest>,
) {
    let runtime_id = registry.runtime_ids.get(CUT_ROPE_BOSS_ID).cloned();
    let intro_track = registry.encounters.get_mut(CUT_ROPE_BOSS_ID).map(|state| {
        state.reset_for_retry();
        state.spec.music_intro.clone()
    });
    if let Some(save) = save {
        let data = save.data_mut();
        data.set_boss(
            CUT_ROPE_BOSS_ID,
            crate::save::PersistedEncounterState::Untouched,
        );
        if let Some(runtime_id) = runtime_id.as_deref() {
            data.set_boss(runtime_id, crate::save::PersistedEncounterState::Untouched);
        }
        // The NPC appears only after the victory beat. Replaying the room should
        // make the post-boss conversation available again only after the next kill.
        data.set_flag("smirking_behemoth_victory_npc_seen", false);
    }
    if let Some(music) = music_request {
        music.desired_track = intro_track.filter(|track| !track.is_empty());
    }
}

/// Spawn the post-Smirking-Behemoth NPC after the boss encounter has fully resolved.
///
/// The NPC is runtime-spawned rather than LDtk-authored so the room layout stays stable and the
/// entity can feel like it crawled out of the dead boss body. It is still a normal peaceful NPC
/// actor with a Yarn dialogue id, so interaction, sprite fallback, pogo/damage volumes, and reset
/// behavior use the existing ECS actor path.
pub fn spawn_cut_rope_victory_npc(
    mut commands: Commands,
    room_set: Res<RoomSet>,
    registry: Res<BossEncounterRegistry>,
    save: Res<crate::persistence::save::SandboxSave>,
    existing: Query<&FeatureId, With<SmirkingBehemothVictoryNpc>>,
    bosses: Query<(&FeatureId, &FeatureAabb, &BossFeature), With<FeatureSimEntity>>,
) {
    if room_set.active_spec().id != CUT_ROPE_ROOM_ID {
        return;
    }
    if existing
        .iter()
        .any(|id| id.as_str() == CUT_ROPE_VICTORY_NPC_ID)
    {
        return;
    }
    let Some((_boss_id, boss_aabb, boss_feature)) = bosses.iter().find(|(id, _, feature)| {
        id.as_str() == CUT_ROPE_BOSS_ID || is_cut_rope_boss(feature.boss.behavior.id.as_str())
    }) else {
        return;
    };
    let boss = &boss_feature.boss;
    let encounter_death_complete =
        registry
            .encounters
            .get(CUT_ROPE_BOSS_ID)
            .is_some_and(|encounter| {
                matches!(
                    encounter.phase,
                    crate::boss_encounter::BossEncounterPhase::Death
                ) && encounter.death_complete()
            });
    let boss_persisted_cleared = {
        let data = save.data();
        matches!(
            data.boss(CUT_ROPE_BOSS_ID),
            crate::save::PersistedEncounterState::Cleared
        ) || matches!(
            data.boss(&boss.behavior.id),
            crate::save::PersistedEncounterState::Cleared
        ) || matches!(
            data.boss(&boss.id),
            crate::save::PersistedEncounterState::Cleared
        )
    };
    if !encounter_death_complete && !boss_persisted_cleared {
        return;
    }
    let boss_bottom_y = boss_aabb.center.y + boss_aabb.half_size.y;
    let spawn_pos = ae::Vec2::new(boss.pos.x, boss_bottom_y - CUT_ROPE_VICTORY_NPC_H * 0.5);
    spawn_victory_npc_entity(&mut commands, spawn_pos);
}

fn victory_npc_size() -> ae::Vec2 {
    ae::Vec2::new(CUT_ROPE_VICTORY_NPC_W, CUT_ROPE_VICTORY_NPC_H)
}

fn spawn_victory_npc_entity(commands: &mut Commands, pos: ae::Vec2) -> Entity {
    let size = victory_npc_size();
    let aabb = ae::Aabb::new(pos, size * 0.5);
    let interactable = crate::interaction::Interactable {
        id: CUT_ROPE_VICTORY_NPC_ID.to_string(),
        prompt: "Talk".to_string(),
        aabb,
        kind: crate::interaction::InteractionKind::Npc {
            dialogue_id: Some(CUT_ROPE_VICTORY_NPC_DIALOGUE_ID.to_string()),
            patrol_radius: 0.0,
            patrol_path_id: None,
        },
        requires_facing: false,
        enabled: true,
    };
    let npc = crate::features::NpcRuntime {
        id: CUT_ROPE_VICTORY_NPC_ID.to_string(),
        name: CUT_ROPE_VICTORY_NPC_NAME.to_string(),
        pos,
        spawn: pos,
        size,
        vel: ae::Vec2::ZERO,
        facing: -1.0,
        on_ground: false,
        interactable,
        patrol_radius: 0.0,
        motion: None,
        talk_radius: 80.0,
        ai_mode: crate::character_ai::CharacterAiMode::Idle,
        hostile: false,
        strikes: 0,
        hit_flash: 0.0,
    };
    let brain = npc.build_brain();
    let combat_kit = crate::features::CombatKit::default();
    let actor = ActorRuntime::Peaceful(npc);
    let (identity, disposition, health, combat, intent, cooldowns) =
        actor_component_snapshot(&actor);
    commands
        .spawn((
            Name::new("Post-boss NPC: Smirking Behemoth victory"),
            SmirkingBehemothVictoryNpc,
            EnemyActorBundle {
                base: FeatureBaseBundle::new(
                    CUT_ROPE_VICTORY_NPC_ID,
                    CUT_ROPE_VICTORY_NPC_NAME,
                    FeatureAabb::from_aabb(aabb),
                ),
                identity,
                disposition,
                faction: crate::features::ActorFaction::Npc,
                target: crate::features::ActorTarget::default(),
                pose: ActorPose::from_aabb(FeatureAabb::from_aabb(aabb), actor.facing()),
                combat_kit,
                aggression: crate::features::ActorAggression::passive(),
                health,
                combat,
                intent,
                cooldowns,
                damageable_volumes: DamageableVolumes::default(),
                pogo_policy: PogoPolicy::FromDamageable,
                pogo_target_volumes: PogoTargetVolumes::default(),
            },
            actor,
            brain,
            crate::brain::ActionSet::peaceful(),
            ActorControl::default(),
        ))
        .id()
}
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SmirkingBehemothVictoryNpc;

fn reset_cut_rope_arena_state_for_room(state: &mut CutRopeBossArenaState, room_id: &str) {
    *state = CutRopeBossArenaState {
        active_room: room_id.to_string(),
        ..Default::default()
    };
}

/// Reset cut-rope-specific prop state immediately when a same-room reset is requested.
///
/// The main arena tick is gameplay-gated because it advances the falling anvil. Dialogue
/// commands can request a room replay while gameplay is suspended, so this reset bridge runs in
/// the ungated room-reset chain and restores rope/anvil visuals on the reset frame instead of
/// relying on the combat tick to observe a short-lived reset message later.
pub fn reset_cut_rope_boss_arena_on_room_reset(
    room_set: Res<RoomSet>,
    mut state: ResMut<CutRopeBossArenaState>,
    mut heavy_object: ResMut<CutRopeHeavyObjectCycle>,
    mut reset_events: MessageReader<ResetRoomFeaturesEvent>,
    mut prop_visuals: Query<(
        &FeatureName,
        &mut PropVisual,
        &mut Transform,
        &mut Sprite,
        Option<&mut CharacterAnimator>,
        Option<&mut Anchor>,
        Option<&mut Visibility>,
    )>,
    assets: Option<Res<GameAssets>>,
) {
    if reset_events.read().next().is_none() {
        return;
    }
    let room = room_set.active_spec();
    if room.id != CUT_ROPE_ROOM_ID {
        if state.active_room != room.id {
            reset_cut_rope_arena_state_for_room(&mut *state, &room.id);
        }
        return;
    }
    heavy_object.advance();
    reset_cut_rope_arena_state_for_room(&mut *state, &room.id);
    if let Some(anvil) = authored_prop(room_set.active_props(), ANVIL_KIND) {
        sync_cut_rope_prop_visuals(
            &mut prop_visuals,
            room_set.active_world(),
            &state,
            anvil,
            heavy_object.current(),
            assets.as_deref(),
        );
    }
}

#[derive(Resource, Default)]
pub struct CutRopeBossArenaState {
    active_room: String,
    rope_cut: bool,
    awaiting_alignment: bool,
    anvil_center: Option<ae::Vec2>,
    anvil_velocity_y: f32,
    kill_sent: bool,
    anvil_exploded: bool,
    rope_fx_timer: f32,
    rope_fx_pulse: u32,
    death_fireworks_sent: bool,
}

/// Drive the Smirking Behemoth's environmental win condition.
pub fn tick_cut_rope_boss_arena(
    world_time: Res<crate::WorldTime>,
    room_set: Res<RoomSet>,
    mut state: ResMut<CutRopeBossArenaState>,
    heavy_object: Res<CutRopeHeavyObjectCycle>,
    mut hit_events: MessageReader<HitEvent>,
    mut reset_events: MessageReader<ResetRoomFeaturesEvent>,
    mut bosses: Query<(&FeatureAabb, &mut BossFeature), With<FeatureSimEntity>>,
    mut boss_registry: Option<ResMut<BossEncounterRegistry>>,
    mut music_request: Option<ResMut<crate::encounter::BossEncounterMusicRequest>>,
    mut cutscene_queue: Option<ResMut<crate::presentation::cutscene::CutsceneTriggerQueue>>,
    mut banner: ResMut<GameplayBanner>,
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
    mut explosions: MessageWriter<ExplosionRequest>,
    mut fireworks: MessageWriter<FireworksRequest>,
    mut debris: MessageWriter<DebrisBurstMessage>,
) {
    let room = room_set.active_spec();
    if room.id != CUT_ROPE_ROOM_ID {
        if state.active_room != room.id {
            reset_cut_rope_arena_state_for_room(&mut *state, &room.id);
        }
        // Advance the readers so old slash/reset messages do not get interpreted if
        // the player warps into the cut-rope room on the next frame.
        for _ in hit_events.read() {}
        for _ in reset_events.read() {}
        return;
    }
    if state.active_room != room.id {
        reset_cut_rope_arena_state_for_room(&mut *state, &room.id);
    }

    let reset_requested = reset_events.read().next().is_some();
    if reset_requested {
        reset_cut_rope_arena_state_for_room(&mut *state, &room.id);
    }

    let Some(rope) = authored_prop(room_set.active_props(), ROPE_KIND) else {
        for _ in hit_events.read() {}
        return;
    };
    let Some(anvil) = authored_prop(room_set.active_props(), ANVIL_KIND) else {
        for _ in hit_events.read() {}
        return;
    };

    let rope_aabb = prop_aabb(rope);
    for event in hit_events.read() {
        if state.rope_cut {
            continue;
        }
        if !matches!(&event.source, HitSource::PlayerSlash { .. }) {
            continue;
        }
        if !event.volume.strict_intersects(rope_aabb) {
            continue;
        }
        state.rope_cut = true;
        state.awaiting_alignment = true;
        state.anvil_center = Some(anvil.pos);
        state.anvil_velocity_y = 0.0;
        state.rope_fx_timer = 0.0;
        state.rope_fx_pulse = 0;
        vfx.write(VfxMessage::Impact {
            pos: event.volume.center(),
        });
        vfx.write(VfxMessage::Burst {
            pos: rope.pos,
            count: 14,
            speed: 160.0,
            color: [0.90, 0.82, 0.58, 0.78],
            kind: ParticleKind::Shard,
        });
        sfx.write(SfxMessage::Slash { pos: rope.pos });
    }

    if state.kill_sent || !state.rope_cut {
        return;
    }

    let dt = world_time.sim_dt().max(0.0);
    let Some(mut center) = state.anvil_center else {
        return;
    };

    let mut boss_under_anvil = false;
    let mut live_boss_pos = None;
    for (_aabb, feature) in bosses.iter_mut() {
        let boss = &feature.boss;
        if !is_cut_rope_boss(&boss.behavior.id) || !boss.alive {
            continue;
        }
        live_boss_pos = Some(boss.pos);
        if boss_is_under_anvil(boss, center.x) {
            boss_under_anvil = true;
            break;
        }
    }

    if !boss_under_anvil {
        state.awaiting_alignment = true;
        state.anvil_velocity_y = 0.0;
        pulse_waiting_rope_explosions(
            &mut state,
            dt,
            rope.pos,
            live_boss_pos.unwrap_or(rope.pos),
            &mut explosions,
        );
        return;
    }

    state.awaiting_alignment = false;
    state.anvil_velocity_y =
        (state.anvil_velocity_y + ANVIL_GRAVITY * dt).min(ANVIL_TERMINAL_SPEED);
    center.y += state.anvil_velocity_y * dt;
    state.anvil_center = Some(center);

    let anvil_aabb = ae::Aabb::new(center, anvil.size * 0.5);
    let floor_y = room.world.size.y - anvil.size.y * 0.5;
    if center.y > floor_y {
        state.anvil_center = Some(ae::Vec2::new(center.x, floor_y));
        state.anvil_velocity_y = 0.0;
    }

    for (_aabb, mut feature) in &mut bosses {
        let boss = &mut feature.boss;
        if !is_cut_rope_boss(&boss.behavior.id) || !boss.alive {
            continue;
        }
        if !anvil_aabb.strict_intersects(boss.aabb()) {
            continue;
        }
        state.kill_sent = true;
        state.anvil_exploded = true;
        boss.alive = false;
        boss.health.current = 0;
        // The death animation should render as-authored. A lingering
        // hit-flash overlay reads as a white silhouette stuck over the body.
        boss.hit_flash = 0.0;

        if let (Some(registry), Some(music), Some(cutscene)) = (
            boss_registry.as_deref_mut(),
            music_request.as_deref_mut(),
            cutscene_queue.as_deref_mut(),
        ) {
            let _ = force_boss_death(registry, music, cutscene, &mut banner, boss.id.as_str());
        }

        banner.show(
            format!(
                "Smirking Behemoth was flattened by a {}",
                heavy_object.current().display_name()
            ),
            2.8,
        );
        explosions.write(ExplosionRequest::classic(center).with_scale(1.25));
        if !state.death_fireworks_sent {
            let mut death_show = FireworksRequest::around(boss.pos);
            death_show.count = 18;
            death_show.spread = ae::Vec2::new(420.0, 280.0);
            death_show.duration = 2.75;
            fireworks.write(death_show);
            state.death_fireworks_sent = true;
        }
        vfx.write(VfxMessage::Burst {
            pos: boss.pos,
            count: 28,
            speed: 260.0,
            color: [0.84, 0.95, 1.0, 0.86],
            kind: ParticleKind::Spark,
        });
        debris.write(DebrisBurstMessage {
            pos: boss.pos,
            cue: PhysicsDebrisCue::BossRagdoll,
        });
        break;
    }
}

/// Keep the authored rope/heavy-object prop visuals in sync with the cut-rope
/// arena state. This is intentionally separate from `tick_cut_rope_boss_arena`:
/// the gameplay tick already uses Bevy's maximum practical system-parameter
/// arity after boss death/music/VFX plumbing, and adding the rendering query
/// there makes it stop satisfying `IntoSystem`/`.run_if(...)`.
pub fn sync_cut_rope_boss_arena_prop_visuals(
    room_set: Res<RoomSet>,
    state: Res<CutRopeBossArenaState>,
    heavy_object: Res<CutRopeHeavyObjectCycle>,
    mut prop_visuals: Query<(
        &FeatureName,
        &mut PropVisual,
        &mut Transform,
        &mut Sprite,
        Option<&mut CharacterAnimator>,
        Option<&mut Anchor>,
        Option<&mut Visibility>,
    )>,
    assets: Option<Res<GameAssets>>,
) {
    if state.active_room != CUT_ROPE_ROOM_ID || room_set.active_spec().id != CUT_ROPE_ROOM_ID {
        return;
    }
    let Some(anvil) = authored_prop(room_set.active_props(), ANVIL_KIND) else {
        return;
    };
    sync_cut_rope_prop_visuals(
        &mut prop_visuals,
        room_set.active_world(),
        &state,
        anvil,
        heavy_object.current(),
        assets.as_deref(),
    );
}

/// After the rope is cut, override the boss brain output with a horizontal
/// lure toward the authored anvil center until impact. The movement still goes
/// through `BossRuntime::integrate_body`, so authored solids and future
/// player-control constraints remain authoritative.
pub fn steer_cut_rope_boss_under_anvil(
    state: Res<CutRopeBossArenaState>,
    mut bosses: Query<
        (&BossFeature, &mut ActorControl, &mut BossAttackState),
        With<FeatureSimEntity>,
    >,
) {
    if state.active_room != CUT_ROPE_ROOM_ID || !state.rope_cut || state.kill_sent {
        return;
    }
    let Some(center) = state.anvil_center else {
        return;
    };
    for (feature, mut control, mut attack_state) in &mut bosses {
        let boss = &feature.boss;
        if !boss.alive || !is_cut_rope_boss(&boss.behavior.id) {
            continue;
        }
        let dx = center.x - boss.pos.x;
        attack_state.clear();
        control.0.melee_pressed = false;
        control.0.special_pressed = false;
        control.0.facing = if dx.abs() > 2.0 {
            dx.signum()
        } else {
            boss.facing
        };
        control.0.desired_vel = if dx.abs() <= boss_alignment_tolerance(boss) {
            ae::Vec2::ZERO
        } else {
            ae::Vec2::new(dx.signum() * ROPE_LURE_SPEED, 0.0)
        };
    }
}

fn boss_is_under_anvil(boss: &BossRuntime, anvil_x: f32) -> bool {
    (boss.pos.x - anvil_x).abs() <= boss_alignment_tolerance(boss)
}

fn boss_alignment_tolerance(boss: &BossRuntime) -> f32 {
    ROPE_ALIGNMENT_TOLERANCE.max(boss.combat_size().x * 0.18)
}

fn pulse_waiting_rope_explosions(
    state: &mut CutRopeBossArenaState,
    dt: f32,
    rope_pos: ae::Vec2,
    boss_pos: ae::Vec2,
    explosions: &mut MessageWriter<ExplosionRequest>,
) {
    state.rope_fx_timer -= dt;
    if state.rope_fx_timer > 0.0 {
        return;
    }
    state.rope_fx_timer = ROPE_SPARK_INTERVAL;
    let i = state.rope_fx_pulse;
    state.rope_fx_pulse = state.rope_fx_pulse.wrapping_add(1);
    let horizontal_pull = (boss_pos.x - rope_pos.x).clamp(-80.0, 80.0) * 0.18;
    let x = (((i.wrapping_mul(37).wrapping_add(11)) % 101) as f32 / 100.0 - 0.5) * 44.0;
    let y = -16.0 - ((i.wrapping_mul(53).wrapping_add(7)) % 59) as f32;
    let kind = match i % 5 {
        0 => ExplosionKind::Starburst,
        1 => ExplosionKind::ClassicBurst,
        2 => ExplosionKind::BurstRound,
        3 => ExplosionKind::Shockwave,
        _ => ExplosionKind::SmokeBurst,
    };
    explosions.write(
        ExplosionRequest::new(rope_pos + ae::Vec2::new(horizontal_pull + x, y), kind)
            .with_scale(0.48),
    );
}

fn authored_prop<'a>(props: &'a [PropSpec], kind: &str) -> Option<&'a PropSpec> {
    props
        .iter()
        .find(|prop| prop.kind == kind || prop.name == kind)
}

fn prop_aabb(prop: &PropSpec) -> ae::Aabb {
    ae::Aabb::new(prop.pos, prop.size * 0.5)
}

fn sync_cut_rope_prop_visuals(
    prop_visuals: &mut Query<(
        &FeatureName,
        &mut PropVisual,
        &mut Transform,
        &mut Sprite,
        Option<&mut CharacterAnimator>,
        Option<&mut Anchor>,
        Option<&mut Visibility>,
    )>,
    world: &ae::World,
    state: &CutRopeBossArenaState,
    anvil: &PropSpec,
    object_kind: CutRopeHeavyObjectKind,
    assets: Option<&GameAssets>,
) {
    for (name, mut prop, mut transform, mut sprite, animator, anchor, visibility) in
        prop_visuals.iter_mut()
    {
        let key_matches = |needle: &str, prop: &PropVisual| prop.kind == needle || name.0 == needle;
        if key_matches(ROPE_KIND, &prop) {
            if let Some(mut visibility) = visibility {
                *visibility = if state.rope_cut {
                    Visibility::Hidden
                } else {
                    Visibility::Visible
                };
            }
        } else if key_matches(ANVIL_KIND, &prop) || key_matches(PIANO_KIND, &prop) {
            apply_cut_rope_heavy_object_sprite(
                &mut prop,
                &mut sprite,
                animator,
                anchor,
                anvil.size,
                object_kind,
                assets,
            );
            if let Some(mut visibility) = visibility {
                *visibility = if state.anvil_exploded {
                    Visibility::Hidden
                } else {
                    Visibility::Visible
                };
            }
            if state.anvil_exploded {
                continue;
            }
            if let Some(center) = state.anvil_center {
                let mut translation = world_to_bevy(world, center, transform.translation.z);
                translation.z = transform.translation.z + ANVIL_Z_OFFSET;
                transform.translation = translation;
            } else {
                let mut translation = world_to_bevy(world, anvil.pos, transform.translation.z);
                translation.z = transform.translation.z;
                transform.translation = translation;
            }
        }
    }
}

fn apply_cut_rope_heavy_object_sprite(
    prop: &mut PropVisual,
    sprite: &mut Sprite,
    animator: Option<Mut<CharacterAnimator>>,
    anchor: Option<Mut<Anchor>>,
    collision: ae::Vec2,
    object_kind: CutRopeHeavyObjectKind,
    assets: Option<&GameAssets>,
) {
    let desired_kind = object_kind.prop_kind();
    if prop.kind == desired_kind {
        return;
    }
    let Some(asset) = assets.and_then(|assets| assets.characters.prop_asset_for_kind(desired_kind))
    else {
        return;
    };
    prop.kind = desired_kind.to_string();
    *sprite = build_character_sprite(asset, Vec2::new(collision.x, collision.y));
    if let Some(mut animator) = animator {
        *animator = CharacterAnimator::new(&asset.spec);
    }
    if let Some(mut anchor) = anchor {
        *anchor = feet_anchor_for(&asset.spec, Vec2::new(collision.x, collision.y));
    }
}
