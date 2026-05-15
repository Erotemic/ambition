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
    user_settings: Res<'w, crate::settings::UserSettings>,
    camera_view: Res<'w, crate::rendering::CameraViewState>,
    player: bevy::prelude::Query<
        'w,
        's,
        (
            &'static crate::player::PlayerBody,
            &'static crate::player::PlayerHealth,
            &'static crate::player::PlayerCombatState,
        ),
        bevy::prelude::With<crate::player::PlayerEntity>,
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

pub(super) fn update_hud(
    runtime: Res<SandboxRuntime>,
    mode: Res<State<GameMode>>,
    world: Res<GameWorld>,
    room_set: Res<rooms::RoomSet>,
    display_mode: Res<windowing::DisplayModeState>,
    developer_tools: Res<DeveloperTools>,
    camera_params: HudCameraParams,
    ldtk_reload: Res<ldtk_world::LdtkHotReloadState>,
    ldtk_spine: Res<ldtk_world::LdtkRuntimeSpineStats>,
    ldtk_spine_index: Res<ldtk_world::LdtkRuntimeSpineIndex>,
    trace: Res<crate::trace::GameplayTraceBuffer>,
    mechanics: Res<crate::mechanics::MechanicsRegistry>,
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
    if !runtime.debug {
        **text = "F1 debug | F3 inspector".to_string();
        return;
    }
    let preset = runtime.preset();
    let enemy_health = camera_params
        .ecs_actors
        .iter()
        .filter_map(|(name, disposition, health, combat)| {
            disposition.is_hostile().then(|| {
                format!(
                    "{} hp {}/{} alive {}",
                    name.0,
                    health.health.current.max(0),
                    health.health.max,
                    combat.alive
                )
            })
        })
        .collect::<Vec<_>>()
        .join(" | ");
    let mut gamepad = String::new();
    for (physical, semantic) in GAMEPAD_MAP.iter().take(6) {
        gamepad.push_str(&format!("{} = {}  ", physical, semantic));
    }
    let active_camera_zone = camera_params
        .camera_view
        .active_camera_zone
        .as_deref()
        .unwrap_or("—");
    let camera_view_line = format!(
        "view: {} {} {} req {:.0}x{:.0} vis {:.0}x{:.0} z{:.2} zones={} active={} body={} move={}",
        camera_params.user_settings.video.camera_zoom.label(),
        camera_params.user_settings.video.camera_aspect.label(),
        camera_params.user_settings.video.camera_framing.label(),
        camera_params.camera_view.requested_view.x,
        camera_params.camera_view.requested_view.y,
        camera_params.camera_view.visible_view.x,
        camera_params.camera_view.visible_view.y,
        camera_params.camera_view.zoom_multiplier,
        camera_params.camera_view.active_camera_zones,
        active_camera_zone,
        developer_tools.player_body_profile.label(),
        developer_tools.movement_profile.label(),
    );
    let feel_line = crate::dev_tools::feel_metrics_summary(
        &runtime.player,
        developer_tools.movement_profile.tuning(),
    );
    let window_line = windows
        .single()
        .map(|w| {
            format!(
                "window: {:.0}x{:.0} {} | {}",
                w.width(),
                w.height(),
                display_mode.label(),
                camera_view_line
            )
        })
        .unwrap_or_else(|_| {
            format!(
                "window: unknown {} | {}",
                display_mode.label(),
                camera_view_line
            )
        });
    let Ok((hud_body, hud_health, hud_combat)) = camera_params.player.single() else {
        return;
    };
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
        let hints = room_set.nearby_zone_hints(&runtime.player, runtime.player.fly_enabled);
        if hints.is_empty() {
            "zones: none".to_string()
        } else {
            format!("zones: {}", hints.join(" | "))
        }
    };
    let feature_banner = if progression.banner.visible() {
        format!("\nFEATURE: {}", progression.banner.text)
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
        format!("\nQUESTS: {}", quest_lines.join("  ::  "))
    };
    let cutscene_line = if let Some(rt) = cutscene.runtime.as_ref() {
        let beat = match cutscene.current_dialogue.as_ref() {
            Some((speaker, text)) => format!("[{speaker}]  {text}  (E to continue)"),
            None => match cutscene.current_banner.as_ref() {
                Some((banner, _)) => format!("// {banner}"),
                None => format!("cutscene: beat {}", rt.beat_index),
            },
        };
        // Skip-hold progress: only render the bar while a hold is in
        // progress, so an idle cutscene doesn't show a clutter prompt.
        let skip_progress = progression.cutscene_request.skip_progress();
        let skip_hint = if skip_progress > 0.01 {
            let filled = (skip_progress * 12.0).round().clamp(0.0, 12.0) as usize;
            let empty = 12usize.saturating_sub(filled);
            format!(
                "  hold Backspace/Select to skip [{}{}] {:.0}%",
                "=".repeat(filled),
                "-".repeat(empty),
                skip_progress * 100.0,
            )
        } else {
            "  (Backspace/Select held = skip)".to_string()
        };
        format!("\nCUTSCENE: {beat}{skip_hint}")
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
            let bar = format!("[{}{}]", "=".repeat(filled), "-".repeat(empty));
            format!(
                "\nBOSS [{}] {} hp {}/{} {} {:.0}%",
                id,
                phase.label(),
                state.hp,
                state.spec.max_hp,
                bar,
                frac * 100.0,
            )
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
            format!("\nENCOUNTER {}", bits.join("  ::  "))
        }
    };
    let map_lines = map_state.summary_lines(&room_set.active_spec().id);
    let map_line = if map_lines.is_empty() {
        String::new()
    } else {
        format!("\nMAP\n{}", map_lines.join("\n"))
    };
    let locomotion = ae::LocomotionState::from_player(&runtime.player).label();
    let body_mode = ae::BodyMode::from_player(&runtime.player).label();
    let trace_status = match (&trace.last_dump_status, &trace.last_dump_path) {
        (Some(status), _) => status.clone(),
        (None, _) => format!(
            "{} frames / {} events buffered (F8 dump)",
            trace.frame_count(),
            trace.event_count()
        ),
    };
    let mechanics_summary = format!(
        "stable={} backend={} planned={}",
        mechanics.count_by_maturity(crate::mechanics::MechanicMaturity::Stable),
        mechanics.count_by_maturity(crate::mechanics::MechanicMaturity::Backend),
        mechanics.count_by_maturity(crate::mechanics::MechanicMaturity::Planned),
    );
    let metadata = room_set.active_metadata();
    let metadata_summary = if metadata.is_empty() {
        "—".to_string()
    } else {
        let mut bits: Vec<String> = Vec::new();
        if let Some(b) = &metadata.biome {
            bits.push(format!("biome={b}"));
        }
        if let Some(t) = &metadata.music_track {
            bits.push(format!("music={t}"));
        }
        if let Some(a) = &metadata.ambient_profile {
            bits.push(format!("ambient={a}"));
        }
        if let Some(v) = &metadata.visual_theme {
            bits.push(format!("theme={v}"));
        }
        if let Some(profile) = metadata.visual_profile.label() {
            bits.push(format!("visual={profile}"));
        }
        if let Some(theme) = &metadata.visual_profile.parallax_theme {
            bits.push(format!("parallax={theme}"));
        }
        bits.join(" ")
    };
    let mechanics_line = format!(
        "\nLOCO: {locomotion}  BODY: {body_mode}  MECH: {mechanics_summary}  ROOM: {metadata_summary}  TRACE: {trace_status}"
    );
    let attack_line = runtime
        .player_attack
        .as_ref()
        .map(|attack| {
            let phase = attack.phase().map(|phase| phase.label()).unwrap_or("done");
            format!(
                "\nATTACK: {} {} {:.0}% hits={}",
                attack.spec.intent.label(),
                phase,
                attack.progress() * 100.0,
                attack.hit_targets.len()
            )
        })
        .unwrap_or_default();
    let ledge_line = runtime
        .player
        .ledge_grab
        .as_ref()
        .map(|ledge| {
            if ledge.climbing {
                let progress = (ledge.climb_elapsed / ae::LEDGE_CLIMB_TIME).clamp(0.0, 1.0);
                format!("\nLEDGE: climb {:.0}%", progress * 100.0)
            } else {
                "\nLEDGE: hang  brief delay then up/toward=climb  down/away=drop".to_string()
            }
        })
        .unwrap_or_default();
    if developer_tools.compact_hud {
        **text = format!(
            "{} | {} | room {}/{} | hp {}/{} | vel ({:+.0},{:+.0}) | grounded {} | dash {} | jumps {}\ncombo: {} | hint: {}\n{} | feel: {} | ldtk: {} auto={} pending={} spine={} rev={} promoted={} last={} | hitstun {:.2} invuln {:.2} hitstop {:.2} | preset {} | {} | F1 debug F3 inspector F4 world F5 overview={} F11 reload F12 auto\n{}{}{}{}{}{}{}{}{}\n",
            world.0.name,
            mode.get().label(),
            room_set.active + 1,
            room_set.rooms.len(),
            player_hp_current,
            player_hp_max,
            player_vel.x,
            player_vel.y,
            player_on_ground,
            player_dash_charges,
            player_air_jumps,
            runtime.player.combo_symbols(),
            runtime.player.current_combo_hint(),
            zone_hint,
            feel_line,
            ldtk_reload.last_status,
            ldtk_reload.auto_apply,
            ldtk_reload.pending,
            ldtk_spine.spawned_entities,
            ldtk_spine_index.revision,
            ldtk_spine_index.promoted_summary(),
            if ldtk_spine.last_entity.is_empty() { "none" } else { &ldtk_spine.last_entity },
            player_hitstun,
            player_invuln,
            player_hitstop,
            preset.name,
            window_line,
            developer_tools.overview_camera,
            // Nine trailing blocks: banner, quest, cutscene, boss, encounter, map, attack, ledge, mechanics
            feature_banner,
            quest_line,
            cutscene_line,
            boss_line,
            encounter_line,
            map_line,
            attack_line,
            ledge_line,
            mechanics_line,
        );
        return;
    }
    let flash_line = if runtime.preset_flash > 0.0 {
        format!("\nPRESET: {}", preset.name)
    } else {
        String::new()
    };
    // Verbose HUD: high-level gameplay readout. Low-level player physics
    // (velocities, timers, blink/fly flags, hitstop/hitstun/invuln,
    // time_scale, inspector visibility) live in `bevy-inspector-egui`
    // (F3) — surfacing them again here just clutters the screen during
    // play. The compact HUD branch (above) keeps a single-screen
    // diagnostic dump for when you want everything at once.
    **text = format!(
        "{}  mode: {}  room {}/{}  size {:.0}x{:.0}\n\
         {}\n\
         hp {}/{}  dash {}  air_jumps {}  charges {}  combo: {}\n\
         hint: {}\n\
         preset: {}\n\
         feel: {}\n\
         F1 debug  F2 slowmo  F3 inspector  F4 world-inspector  F5 overview={}  F8 trace dump  F11 LDtk reload  F12 LDtk auto={}  Esc mode={}  Delete reset\n\
         LDtk: {} (spine {} entities, promoted {})\n\
         {}\n\
         enemies: {}\n\
         {}\n\
         {}\n\
         {}\n\
         gamepad: {}{}{}\n",
        world.0.name,
        mode.get().label(),
        room_set.active + 1,
        room_set.rooms.len(),
        world.0.size.x,
        world.0.size.y,
        zone_hint,
        player_hp_current,
        player_hp_max,
        player_dash_charges,
        player_air_jumps,
        player_mana_current,
        runtime.player.combo_symbols(),
        runtime.player.current_combo_hint(),
        preset.name,
        feel_line,
        developer_tools.overview_camera,
        ldtk_reload.auto_apply,
        mode.get().label(),
        ldtk_reload.last_status,
        ldtk_spine.spawned_entities,
        ldtk_spine_index.promoted_summary(),
        window_line,
        enemy_health,
        mechanics_line,
        attack_line,
        ledge_line,
        gamepad,
        flash_line,
        feature_banner,
    );
    // Cutscene / boss / encounter / map lines stay in the verbose HUD
    // because they're tightly coupled to the live combat / traversal
    // status the rest of the HUD shows. Quests live in their own
    // panel (`update_quest_panel`).
    if !cutscene_line.is_empty()
        || !boss_line.is_empty()
        || !encounter_line.is_empty()
        || !map_line.is_empty()
    {
        text.push_str(&cutscene_line);
        text.push_str(&boss_line);
        text.push_str(&encounter_line);
        text.push_str(&map_line);
    }
}

/// Update the dedicated quest-panel text widget.
///
/// Lives separately from `update_hud` so the quest log doesn't trail
/// the giant debug stats dump and can be styled / positioned
/// independently. Writes empty string when there are no active
/// quests, which collapses the panel visually.
pub fn update_quest_panel(
    quests: Res<crate::quest::QuestRegistry>,
    user_settings: Res<crate::settings::UserSettings>,
    entities: Res<SceneEntities>,
    mut query: Query<&mut Text, With<crate::rendering::QuestPanelText>>,
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
