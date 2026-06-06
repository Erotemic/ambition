#[allow(unused_imports)]
use super::cli::*;
#[allow(unused_imports)]
use super::dev_runtime::*;
#[allow(unused_imports)]
use super::feedback::*;
#[allow(unused_imports)]
use super::input_systems::*;
#[allow(unused_imports)]
use super::phases::*;
#[allow(unused_imports)]
use super::player_tick::*;
#[allow(unused_imports)]
use super::plugins::*;
#[allow(unused_imports)]
use super::resources::*;
#[allow(unused_imports)]
use super::setup_systems::*;
#[allow(unused_imports)]
use super::world_flow::*;
#[allow(unused_imports)]
use super::*;
use bevy::ecs::system::SystemParam;

#[derive(SystemParam)]
pub(super) struct HudCameraParams<'w, 's> {
    user_settings: Res<'w, crate::persistence::settings::UserSettings>,
    player: bevy::prelude::Query<
        'w,
        's,
        (
            &'static crate::player::BodyKinematics,
            &'static crate::player::PlayerGroundState,
            &'static crate::player::PlayerWallState,
            &'static crate::player::PlayerDashState,
            &'static crate::player::PlayerJumpState,
            &'static crate::player::PlayerMana,
            &'static crate::player::PlayerBodyModeState,
            &'static crate::player::PlayerLedgeState,
            &'static crate::player::PlayerFlightState,
            &'static crate::player::PlayerBlinkState,
            &'static crate::player::PlayerComboTrace,
            &'static crate::player::PlayerHealth,
            &'static crate::player::PlayerCombatState,
            &'static crate::player::ActivePlayerAttack,
        ),
        crate::player::PrimaryPlayerOnly,
    >,
    ecs_actors: bevy::prelude::Query<
        'w,
        's,
        (
            &'static crate::features::FeatureName,
            &'static crate::features::ActorDisposition,
            &'static crate::features::ActorHealth,
            &'static crate::features::ActorCombatState,
        ),
        bevy::prelude::Without<crate::features::BossConfig>,
    >,
}

/// HUD reads stats from `HudCameraParams`, which now filters on
/// `PrimaryPlayerOnly` (`With<PlayerEntity> + With<PrimaryPlayer>`).
/// In a future co-op build, the HUD intentionally tracks the primary
/// player; per-`PlayerSlot` panels would be a separate UI surface
/// rather than a generalization of this one.
pub(super) fn update_hud(
    dev_state: Res<SandboxDevState>,
    mode: Res<State<GameMode>>,
    world: Res<GameWorld>,
    room_set: Res<rooms::RoomSet>,
    display_mode: Res<windowing::DisplayModeState>,
    developer_tools: Res<DeveloperTools>,
    camera_params: HudCameraParams,
    _ldtk_reload: Res<ldtk_world::LdtkHotReloadState>,
    progression: ProgressionResources,
    windows: Query<&Window, With<PrimaryWindow>>,
    entities: Res<SceneEntities>,
    mut query: Query<&mut Text, With<HudText>>,
) {
    let _quest_registry = &progression.quests;
    let cutscene = &progression.cutscene;
    let boss_registry = &progression.bosses;
    let encounter_registry = &progression.encounters;
    let map_state = &progression.map;
    let Ok(mut text) = query.get_mut(entities.hud) else {
        return;
    };
    if !developer_tools.show_hud || !camera_params.user_settings.gameplay.debug_hud_visible {
        **text = String::new();
        return;
    }
    if !dev_state.debug {
        **text = "F1 debug | F3 inspector".to_string();
        return;
    }
    let preset = dev_state.preset();
    let enemy_health = camera_params
        .ecs_actors
        .iter()
        .filter(|(_, disposition, _, _)| disposition.is_hostile())
        .map(|(name, _, health, combat)| {
            let name = &name.0;
            let cur = health.health.current.max(0);
            let max = health.health.max;
            let alive = combat.alive;
            format!("{name} hp {cur}/{max} alive {alive}")
        })
        .collect::<Vec<_>>()
        .join(" | ");
    let window_line = windows
        .single()
        .map(|w| {
            let width = w.width();
            let height = w.height();
            let mode = display_mode.label();
            format!("window: {width:.0}x{height:.0} {mode}")
        })
        .unwrap_or_else(|_| {
            let mode = display_mode.label();
            format!("window: unknown {mode}")
        });
    let Ok((
        hud_kin,
        hud_ground,
        hud_wall,
        hud_dash,
        hud_jump,
        hud_mana,
        hud_body_mode,
        hud_ledge,
        hud_flight,
        hud_blink,
        hud_combo,
        hud_health,
        hud_combat,
        hud_attack,
    )) = camera_params.player.single()
    else {
        return;
    };
    let player_hp_current = hud_health.current().max(0);
    let player_hp_max = hud_health.max();
    let player_vel = hud_kin.vel;
    let player_on_ground = hud_ground.on_ground;
    let player_dash_charges = hud_dash.charges_available;
    let player_air_jumps = hud_jump.air_jumps_available;
    let player_mana_current = hud_mana.meter.current as i32;
    let player_hitstun = hud_combat.hitstun_timer;
    let player_invuln = hud_combat.damage_invuln_timer;
    let player_hitstop = hud_combat.hitstop_timer;

    let feature_banner = if progression.banner.visible() {
        let text = &progression.banner.text;
        format!("\nFEATURE: {text}")
    } else {
        String::new()
    };
    // Cutscene UI lives in the dedicated overlay
    // (`crate::presentation::cutscene::sync_cutscene_ui`) — a proper Bevy Node panel
    // with speaker / body / continue prompt and a skip-hold progress
    // bar. The debug HUD just notes that one is active so testers
    // can correlate skip-hold state with the floating overlay; the
    // detailed beat content stays out of the debug text line.
    let cutscene_line = if let Some(rt) = cutscene.runtime.as_ref() {
        let beat_label = match cutscene.current_dialogue.as_ref() {
            Some((speaker, _)) => format!("dialogue @ {speaker}"),
            None => match cutscene.current_banner.as_ref() {
                Some((_, remaining)) => format!("banner ({remaining:.1}s)"),
                None => format!("beat {}", rt.beat_index),
            },
        };
        format!("\nCUTSCENE: {beat_label}")
    } else {
        String::new()
    };
    let boss_line = if let Some((id, phase)) = boss_registry.active_phase() {
        if let Some(state) = boss_registry.get(id) {
            // Health bar: 16-tick string that shrinks as boss HP drops
            // so the player gets a glanceable progress signal even
            // before a real HUD lands.
            let frac = state.hp_fraction();
            let filled = (frac * 16.0).round().clamp(0.0, 16.0) as usize;
            let empty = 16usize.saturating_sub(filled);
            let bar_filled = "=".repeat(filled);
            let bar_empty = "-".repeat(empty);
            let phase = phase.label();
            let hp = state.hp;
            let max_hp = state.spec.max_hp;
            let pct = frac * 100.0;
            format!("\nBOSS [{id}] {phase} hp {hp}/{max_hp} [{bar_filled}{bar_empty}] {pct:.0}%")
        } else {
            String::new()
        }
    } else {
        String::new()
    };
    let encounter_line = {
        let mut bits = Vec::new();
        for (_id, state) in encounter_registry.encounters.iter() {
            if matches!(
                state.phase,
                crate::encounter::EncounterPhase::Starting { .. }
                    | crate::encounter::EncounterPhase::Active { .. }
            ) {
                bits.push(state.hud_summary());
            }
        }
        if bits.is_empty() {
            String::new()
        } else {
            let joined = bits.join("  ::  ");
            format!("\nENCOUNTER {joined}")
        }
    };
    let map_lines = map_state.summary_lines(&room_set.active_spec().id);
    let map_line = if map_lines.is_empty() {
        String::new()
    } else {
        let joined = map_lines.join("\n");
        format!("\nMAP\n{joined}")
    };
    // The engine ships a cluster-native `LocomotionState::from_clusters`
    // that classifies these states from cluster components directly.
    let locomotion = ae::LocomotionState::from_clusters(
        hud_ground, hud_wall, hud_flight, hud_dash, hud_blink, hud_ledge,
    )
    .label();
    let body_mode = hud_body_mode.body_mode.label();
    let movement_line = format!("\nLOCO: {locomotion}  BODY: {body_mode}");
    let attack_line = hud_attack
        .0
        .as_ref()
        .map(|attack| {
            let intent = attack.spec.intent.label();
            let phase = attack.phase().map(|phase| phase.label()).unwrap_or("done");
            let pct = attack.progress() * 100.0;
            let hits = attack.hit_targets.len();
            format!("\nATTACK: {intent} {phase} {pct:.0}% hits={hits}")
        })
        .unwrap_or_default();
    let ledge_line = hud_ledge
        .grab
        .as_ref()
        .map(|ledge| {
            if ledge.climbing {
                let pct = (ledge.climb_elapsed / ae::LEDGE_CLIMB_TIME).clamp(0.0, 1.0) * 100.0;
                format!("\nLEDGE: climb {pct:.0}%")
            } else {
                "\nLEDGE: hang".to_string()
            }
        })
        .unwrap_or_default();
    if developer_tools.compact_hud {
        let world_name = &world.0.name;
        let mode_label = mode.get().label();
        let room_index = room_set.active + 1;
        let room_count = room_set.rooms.len();
        let vx = player_vel.x;
        let vy = player_vel.y;
        let combo_symbols = hud_combo.symbols();
        let preset_name = &preset.name;
        **text = format!(
            "{world_name} | {mode_label} | room {room_index}/{room_count} | \
             hp {player_hp_current}/{player_hp_max} | vel ({vx:+.0},{vy:+.0}) | \
             grounded {player_on_ground} | dash {player_dash_charges} | jumps {player_air_jumps}\n\
             combo: {combo_symbols}\n\
             hitstun {player_hitstun:.2} invuln {player_invuln:.2} hitstop {player_hitstop:.2} | \
             preset {preset_name} | {window_line} | \
             {feature_banner}{cutscene_line}{boss_line}\
             {encounter_line}{map_line}{attack_line}{ledge_line}{movement_line}\n"
        );
        return;
    }
    let flash_line = if dev_state.preset_flash > 0.0 {
        let name = &preset.name;
        format!("\nPRESET: {name}")
    } else {
        String::new()
    };
    // Verbose HUD: high-level gameplay readout. Low-level player physics
    // (velocities, timers, blink/fly flags, hitstop/hitstun/invuln,
    // time_scale, inspector visibility) live in `bevy-inspector-egui`
    // (F3) — surfacing them again here just clutters the screen during
    // play. The compact HUD branch (above) keeps a single-screen
    // diagnostic dump for when you want everything at once. Niche
    // dev-tool telemetry (room metadata, mechanics counts, trace
    // buffer size, LDtk spine entity counts, camera-view diagnostics,
    // gamepad mapping table) intentionally lives elsewhere — promote
    // it to this panel only when there's a gameplay reason to glance
    // at it during play.
    let world_name = &world.0.name;
    let mode_label = mode.get().label();
    let room_index = room_set.active + 1;
    let room_count = room_set.rooms.len();
    let combo_symbols = hud_combo.symbols();
    let preset_name = &preset.name;
    **text = format!(
        "{world_name}  mode: {mode_label}  room {room_index}/{room_count}\n\
         hp {player_hp_current}/{player_hp_max}  dash {player_dash_charges}  \
         air_jumps {player_air_jumps}  mana {player_mana_current}  combo: {combo_symbols}\n\
         preset: {preset_name}\n\
         {window_line}\n\
         enemies: {enemy_health}\
         {attack_line}{ledge_line}{movement_line}{flash_line}{feature_banner}\
         {cutscene_line}{boss_line}{encounter_line}{map_line}\n"
    );
}

/// Update the dedicated quest-panel text widget.
///
/// Lives separately from `update_hud` so the quest log doesn't trail
/// the giant debug stats dump and can be styled / positioned
/// independently. Writes empty string when there are no active
/// quests, which collapses the panel visually.
pub fn update_quest_panel(
    quests: Res<crate::content::quest::QuestRegistry>,
    user_settings: Res<crate::persistence::settings::UserSettings>,
    entities: Res<SceneEntities>,
    mut query: Query<&mut Text, With<crate::presentation::rendering::QuestPanelText>>,
) {
    if entities.quest_panel == Entity::PLACEHOLDER {
        return;
    }
    let Ok(mut text) = query.get_mut(entities.quest_panel) else {
        return;
    };
    if !user_settings.gameplay.quest_hud_visible {
        **text = String::new();
        return;
    }
    let lines = quests.quest_log_lines();
    if lines.is_empty() {
        **text = String::new();
    } else {
        **text = format!("QUESTS\n  {}", lines.join("\n  "));
    }
}
