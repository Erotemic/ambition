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
use super::plugins::*;
#[allow(unused_imports)]
use super::resources::*;
#[allow(unused_imports)]
use super::setup_systems::*;
#[allow(unused_imports)]
use super::update::*;
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
            &'static crate::player::PlayerBody,
            &'static crate::player::PlayerHealth,
            &'static crate::player::PlayerCombatState,
            &'static crate::player::PlayerMovementAuthority,
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
    ldtk_reload: Res<ldtk_world::LdtkHotReloadState>,
    progression: ProgressionResources,
    windows: Query<&Window, With<PrimaryWindow>>,
    entities: Res<SceneEntities>,
    mut query: Query<&mut Text, With<HudText>>,
) {
    let quest_registry = &progression.quests;
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
        .filter_map(|(name, disposition, health, combat)| {
            disposition.is_hostile().then(|| {
                let name = &name.0;
                let cur = health.health.current.max(0);
                let max = health.health.max;
                let alive = combat.alive;
                format!("{name} hp {cur}/{max} alive {alive}")
            })
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
    let Ok((hud_body, hud_health, hud_combat, hud_authority, hud_attack)) =
        camera_params.player.single()
    else {
        return;
    };
    let feel_line = crate::dev::dev_tools::feel_metrics_summary(
        hud_body.base_size,
        developer_tools.movement_profile.tuning(),
    );
    let player_hp_current = hud_health.current().max(0);
    let player_hp_max = hud_health.max();
    let player_vel = hud_body.vel;
    let player_on_ground = hud_body.on_ground;
    let player_dash_charges = hud_body.dash_charges_available;
    let player_air_jumps = hud_body.air_jumps_available;
    let player_mana_current = hud_body.mana_current as i32;
    let player_hitstun = hud_combat.hitstun_timer;
    let player_invuln = hud_combat.damage_invuln_timer;
    let player_hitstop = hud_combat.hitstop_timer;

    let zone_hint = {
        let hints = room_set.nearby_zone_hints(hud_body.aabb(), hud_body.fly_enabled);
        if hints.is_empty() {
            "zones: none".to_string()
        } else {
            let joined = hints.join(" | ");
            format!("zones: {joined}")
        }
    };
    let feature_banner = if progression.banner.visible() {
        let text = &progression.banner.text;
        format!("\nFEATURE: {text}")
    } else {
        String::new()
    };
    // Quest content now lives in its own UI surface
    // (`update_quest_panel` writes to `QuestPanelText`); the debug HUD
    // no longer carries a `\nQUESTS: ...` trailer. The compact HUD
    // branch keeps emitting the line for the single-screen dump
    // (testers want everything-at-once); the verbose branch omits it.
    let quest_lines = quest_registry.quest_log_lines();
    let quest_line = if quest_lines.is_empty() {
        String::new()
    } else {
        let joined = quest_lines.join("  ::  ");
        format!("\nQUESTS: {joined}")
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
    let locomotion = ae::LocomotionState::from_player(&hud_authority.player).label();
    let body_mode = hud_body.body_mode.label();
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
    let ledge_line = hud_authority
        .player
        .ledge_grab
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
        let combo_symbols = hud_authority.player.combo_symbols();
        let combo_hint = hud_authority.player.current_combo_hint();
        let ldtk_status = &ldtk_reload.last_status;
        let overview = developer_tools.overview_camera;
        let preset_name = &preset.name;
        **text = format!(
            "{world_name} | {mode_label} | room {room_index}/{room_count} | \
             hp {player_hp_current}/{player_hp_max} | vel ({vx:+.0},{vy:+.0}) | \
             grounded {player_on_ground} | dash {player_dash_charges} | jumps {player_air_jumps}\n\
             combo: {combo_symbols} | hint: {combo_hint}\n\
             {zone_hint} | feel: {feel_line} | ldtk: {ldtk_status} | \
             hitstun {player_hitstun:.2} invuln {player_invuln:.2} hitstop {player_hitstop:.2} | \
             preset {preset_name} | {window_line} | \
             F1 debug F3 inspector F4 world F5 overview={overview} F11 reload F12 auto\n\
             {feature_banner}{quest_line}{cutscene_line}{boss_line}\
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
    let combo_symbols = hud_authority.player.combo_symbols();
    let combo_hint = hud_authority.player.current_combo_hint();
    let preset_name = &preset.name;
    let ldtk_status = &ldtk_reload.last_status;
    let overview = developer_tools.overview_camera;
    **text = format!(
        "{world_name}  mode: {mode_label}  room {room_index}/{room_count}\n\
         {zone_hint}\n\
         hp {player_hp_current}/{player_hp_max}  dash {player_dash_charges}  \
         air_jumps {player_air_jumps}  mana {player_mana_current}  combo: {combo_symbols}\n\
         hint: {combo_hint}\n\
         preset: {preset_name}\n\
         F1 debug  F2 slowmo  F3 inspector  F5 overview={overview}  F8 trace  \
         F11 LDtk reload  Esc mode={mode_label}  Delete reset\n\
         LDtk: {ldtk_status}\n\
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
