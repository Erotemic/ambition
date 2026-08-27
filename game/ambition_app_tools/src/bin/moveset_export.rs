//! Export every smash fighter's balance data as one JSON bundle.
//!
//! ⭐ THE POINT IS TO ANSWER BALANCE QUESTIONS WITHOUT A PLAYTEST. Jon,
//! 2026-08-27: *"I want a moveset balance stats diagnostic presentation... things
//! related to balancing or inspecting characters that is faster to do than
//! loading up the game and doing a playtest match with them."*
//!
//! ⛔⛔ AND IT READS THE COMPOSED HOST, NOT THE AUTHORING FILES. A tool that
//! parsed `*_moveset.rs` would report what somebody wrote; this boots
//! `build_visible_app`, waits for the `Startup` systems that fill the prepared
//! registry, and reports what the SHIPPED GAME resolves — which is the only
//! version of the numbers a balance decision can be made against. The two differ
//! whenever a repertoire slot overwrites a move (see `UpSpecial::into_spec`) or a
//! provider composes a fighter out of another one's table.
//!
//! ⛔ THE SCHEMA HERE IS ITS OWN, and deliberately not `serde(MoveSpec)`. The
//! catalog's field names are an internal contract that moves when the engine
//! moves; a viewer keyed to them would break on a rename that changed nothing it
//! displays. This file is the ONE translation, so the UI has one shape to know
//! and one place to update.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use ambition_platformer2d::entity_catalog::{
    MoveEventKind, MoveSpec, MovesetContract, RecoveryUse, VolumeShape, WindowTag,
};

/// The sim tick. Frame counts in this bundle are seconds x this, which is the
/// unit every balance conversation in the genre is held in.
const SIM_HZ: f32 = 60.0;

/// The bundle's schema id. Bump the version when a consumer would break.
const SCHEMA: &str = "ambition.moveset_inspector.v1";

fn frames(seconds: f32) -> f32 {
    seconds * SIM_HZ
}

/// One authored hit volume, flattened to the rectangle a viewer draws.
///
/// A circle is reported with its radius AND the enclosing half-extents, so a
/// viewer that only knows rectangles still draws something true rather than
/// nothing.
fn volume_json(volume: &ambition_platformer2d::entity_catalog::HitVolume) -> serde_json::Value {
    let (offset, half, radius) = match volume.shape {
        VolumeShape::Rect {
            offset,
            half_extents,
        } => (offset, half_extents, None),
        VolumeShape::Circle { offset, radius } => (offset, (radius, radius), Some(radius)),
    };
    serde_json::json!({
        "offset": [offset.0, offset.1],
        "half_extents": [half.0, half.1],
        "radius": radius,
        "damage": volume.damage,
        "knockback": volume.knockback,
        // `None` is "the stage's growth decides", which is a different
        // statement from an authored zero and must survive the export.
        "knockback_growth": volume.knockback_growth,
        "launch_dir": volume.launch_dir.map(|d| vec![d.0, d.1]),
        "reaction": volume.reaction.map(|r| match r {
            ambition_platformer2d::entity_catalog::VolumeReaction::Autolink(_) => "autolink",
            ambition_platformer2d::entity_catalog::VolumeReaction::Windbox(_) => "windbox",
        }),
        "on_hit": volume.on_hit.as_ref().map(|e| e.key.clone()),
        "vfx": volume.vfx.clone(),
        "hit_sfx": volume.hit_sfx.clone(),
    })
}

fn window_json(window: &ambition_platformer2d::entity_catalog::MoveWindow) -> serde_json::Value {
    let (tag, cancel_into) = match &window.tag {
        WindowTag::Startup => ("startup".to_string(), Vec::new()),
        WindowTag::Active => ("active".to_string(), Vec::new()),
        WindowTag::Recovery => ("recovery".to_string(), Vec::new()),
        WindowTag::Invuln => ("invuln".to_string(), Vec::new()),
        WindowTag::Armor => ("armor".to_string(), Vec::new()),
        WindowTag::Cancelable { into, condition } => (
            format!("cancelable:{condition:?}").to_lowercase(),
            into.clone(),
        ),
    };
    serde_json::json!({
        "tag": tag,
        "cancel_into": cancel_into,
        "start_s": window.start_s,
        "end_s": window.end_s,
        "start_f": frames(window.start_s),
        "end_f": frames(window.end_s),
        "motion_scale": window.motion_scale,
        "sustain_effect": window.sustain_effect.as_ref().map(|e| e.key.clone()),
        "volumes": window.volumes.iter().map(volume_json).collect::<Vec<_>>(),
    })
}

/// The one-line summary a balance table sorts on.
///
/// ⛔ `startup` IS THE FIRST ACTIVE WINDOW'S START, not the first Startup
/// window's end. Those are the same number for an ordinary strike and NOT the
/// same for a move whose timeline has a gap, a multi-hit, or an `Invuln` window
/// wedged in — and "how long until this can hit me" is the question the genre
/// asks.
fn derived_json(spec: &MoveSpec) -> serde_json::Value {
    let actives: Vec<_> = spec
        .windows
        .iter()
        .filter(|w| matches!(w.tag, WindowTag::Active))
        .collect();
    let startup_s = actives.iter().map(|w| w.start_s).fold(f32::NAN, f32::min);
    let last_active_end = actives.iter().map(|w| w.end_s).fold(f32::NAN, f32::max);
    let active_s: f32 = actives
        .iter()
        .map(|w| w.end_s - w.start_s)
        .sum::<f32>()
        .max(0.0);
    let volumes: Vec<_> = spec
        .windows
        .iter()
        .flat_map(|w| w.volumes.iter())
        .collect();
    // The biggest single connection, which is what a kill-power comparison
    // wants; the SUM is what a multi-hit's full carry is worth, and they are
    // different questions, so both ship.
    let max_damage = volumes.iter().map(|v| v.damage).max().unwrap_or(0);
    let sum_damage: i32 = volumes.iter().map(|v| v.damage).sum();
    let max_knockback = volumes
        .iter()
        .map(|v| v.knockback)
        .fold(0.0f32, f32::max);
    // Reach is measured to the FAR EDGE of the volume along the facing axis:
    // a box centred at 34 with a 26 half-extent reaches 60, and that is the
    // spacing a player actually feels.
    let reach = volumes
        .iter()
        .map(|v| match v.shape {
            VolumeShape::Rect {
                offset,
                half_extents,
            } => offset.0.abs() + half_extents.0,
            VolumeShape::Circle { offset, radius } => offset.0.abs() + radius,
        })
        .fold(0.0f32, f32::max);
    let vertical_reach = volumes
        .iter()
        .map(|v| match v.shape {
            VolumeShape::Rect {
                offset,
                half_extents,
            } => offset.1.abs() + half_extents.1,
            VolumeShape::Circle { offset, radius } => offset.1.abs() + radius,
        })
        .fold(0.0f32, f32::max);
    let fires_projectile = spec
        .events
        .iter()
        .any(|e| matches!(e.kind, MoveEventKind::Ranged));
    // Endlag: from the last hittable instant to the end of the move. A move
    // with no active window owes its whole duration, which is the honest answer
    // for a pure-mobility special.
    let endlag_s = if actives.is_empty() {
        spec.duration_s
    } else {
        (spec.duration_s - last_active_end).max(0.0)
    };
    serde_json::json!({
        "startup_s": (!startup_s.is_nan()).then_some(startup_s),
        "startup_f": (!startup_s.is_nan()).then(|| frames(startup_s)),
        "active_s": active_s,
        "active_f": frames(active_s),
        "endlag_s": endlag_s,
        "endlag_f": frames(endlag_s),
        "max_damage": max_damage,
        "sum_damage": sum_damage,
        "max_knockback": max_knockback,
        "reach": reach,
        "vertical_reach": vertical_reach,
        "hits": volumes.len(),
        "fires_projectile": fires_projectile,
        // The charge payoff a fully held release applies, so a smash's real
        // ceiling is one multiplication away rather than folklore.
        "max_damage_charged": (max_damage as f32 * spec.smash_charge_mult).round() as i32,
    })
}

fn event_json(event: &ambition_platformer2d::entity_catalog::MoveEvent) -> serde_json::Value {
    let (kind, detail) = match &event.kind {
        MoveEventKind::Sfx { cue } => ("sfx", cue.clone()),
        MoveEventKind::Vfx { effect, .. } => ("vfx", effect.clone()),
        MoveEventKind::Effect(effect) => ("effect", effect.key.clone()),
        // ⭐ EXHAUSTIVE ON PURPOSE. A new event kind is a new thing a move can
        // do, and a catch-all would report it as `"other"` in a tool whose whole
        // job is to show what a move does. The compiler names the gap instead.
        MoveEventKind::Impulse { local, mode, .. } => (
            "impulse",
            format!("{mode:?} ({}, {})", local.0, local.1),
        ),
        MoveEventKind::Ranged => ("ranged", String::new()),
    };
    serde_json::json!({
        "at_s": event.at_s,
        "at_f": frames(event.at_s),
        "kind": kind,
        "detail": detail,
    })
}

fn move_json(spec: &MoveSpec, verbs: &[String]) -> serde_json::Value {
    serde_json::json!({
        "id": spec.id,
        "display_name": spec.display_name.clone(),
        "verbs": verbs,
        "clip": spec.clip.clip,
        "duration_s": spec.duration_s,
        "duration_f": frames(spec.duration_s),
        "gates": {
            "grounded": spec.gates.grounded,
            "recovery": match spec.gates.recovery {
                RecoveryUse::None => "none",
                RecoveryUse::SpendAndFreefall => "spend_and_freefall",
                RecoveryUse::SpendWithoutFreefall => "spend_without_freefall",
            },
            "forbidden_while_held": spec.gates.forbidden_while_held,
            "roots_steering": spec.gates.roots_steering,
        },
        "start_impulse": spec.start_impulse.map(|i| vec![i.0, i.1]),
        "smash_charge_mult": spec.smash_charge_mult,
        "charge": spec.smash_charge.as_ref().map(|c| serde_json::json!({
            "hold_at_s": c.hold_at_s,
            "max_hold_s": c.max_hold_s,
            "gesture": format!("{:?}", spec.charge_gesture).to_lowercase(),
        })),
        "landing_lag_s": spec.landing_lag_s,
        "autocancel_after_s": spec.autocancel_after_s,
        "repeat": spec.repeat.as_ref().map(|r| serde_json::json!({
            "from_s": r.from_s, "to_s": r.to_s, "max_s": r.max_s,
        })),
        "windows": spec.windows.iter().map(window_json).collect::<Vec<_>>(),
        "events": spec.events.iter().map(event_json).collect::<Vec<_>>(),
        "derived": derived_json(spec),
    })
}

/// Move id → the verbs that reach it, so the viewer can show a move under the
/// button a player presses instead of under an internal id.
fn verbs_by_move(contract: &MovesetContract) -> BTreeMap<String, Vec<String>> {
    let mut by_move: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (verb, move_id) in &contract.verbs {
        by_move.entry(move_id.clone()).or_default().push(verb.clone());
    }
    by_move
}

fn character_json(
    id: &str,
    prepared: &ambition_platformer2d::actors::character_runtime::PreparedCharacterDefinition,
    catalog: Option<&ambition_platformer2d::characters::actor::character_catalog::CharacterCatalogEntry>,
    on_grid: bool,
) -> serde_json::Value {
    let contract = prepared.kit.projectable_moveset();
    let by_move = contract.map(verbs_by_move).unwrap_or_default();
    let moves: Vec<_> = contract
        .map(|c| {
            c.moves
                .iter()
                .map(|m| move_json(m, by_move.get(&m.id).map(Vec::as_slice).unwrap_or(&[])))
                .collect()
        })
        .unwrap_or_default();
    serde_json::json!({
        "id": id,
        "display_name": prepared.display_name,
        "provider": prepared.provider,
        "on_smash_grid": on_grid,
        "description": catalog.map(|e| e.gameplay_description.clone()),
        "spritesheet": catalog.map(|e| e.spritesheet.clone()),
        "portrait": prepared.portrait.clone(),
        "vitals": {
            "max_health": prepared.vitals.max_health,
            "mass": prepared.vitals.mass,
            // The knockback axis: heavier launches less under the same growth.
            "knockback_weight": prepared.vitals.knockback_weight,
            "canonical_height": prepared.vitals.canonical_height,
        },
        "locomotion": prepared.locomotion.map(|l| serde_json::json!({
            "run_speed": l.run_speed,
            "move_style": format!("{:?}", l.move_style),
            "surface_walker": l.surface_walker,
        })),
        "movement_tuning": prepared.movement_tuning.map(|t| serde_json::json!({
            "gravity": t.gravity,
            "run_accel": t.run_accel,
            "air_accel": t.air_accel,
            "ground_friction": t.ground_friction,
            "air_friction": t.air_friction,
            "max_run_speed": t.max_run_speed,
            "max_air_speed": t.max_air_speed,
        })),
        "abilities": prepared.abilities.map(|a| serde_json::json!({
            "double_jump": a.double_jump,
            "wall_jump": a.wall_jump,
            "dash": a.dash,
            "fly": a.fly,
            "blink": a.blink,
            "fast_fall": a.fast_fall,
        })),
        "mount": prepared.mount.as_ref().map(|m| serde_json::json!({
            "pilotable_classes": m.pilotable_classes,
        })),
        "held_item": prepared.held_item.clone(),
        // The silhouette the volumes are offset FROM. Without it a viewer can
        // draw a hitbox rectangle and nothing to judge its size against, which
        // is the one thing a reach number cannot tell you by itself.
        "body": prepared.body.as_ref().map(|body| match body {
            ambition_platformer2d::characters::actor::definition::BodySource::Explicit {
                half_extents,
            } => serde_json::json!({
                "kind": "explicit",
                "half_extents": [half_extents.0, half_extents.1],
            }),
            ambition_platformer2d::characters::actor::definition::BodySource::SpriteAuthored {
                world_per_pixel,
            } => serde_json::json!({
                "kind": "sprite_authored",
                "world_per_pixel": world_per_pixel,
            }),
        }),
        "verbs": contract.map(|c| c.verbs.clone()).unwrap_or_default(),
        "moves": moves,
    })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = args
        .windows(2)
        .find(|w| w[0] == "--out")
        .map(|w| w[1].clone())
        .unwrap_or_else(|| "tools/ambition_moveset_inspector/data/moveset_bundle.json".to_string());

    let mut app = ambition_app::app::build_visible_app(
        ambition_app::app::VisibleRenderMode::NoWindow,
        true,
    );
    // ⛔ THE REGISTRY IS FILLED BY A `Startup` SYSTEM, so a build that has never
    // updated has a catalog and no registry at all — the exact trap
    // `smash_roster_movesets` records. One frame is enough; several are cheap.
    for _ in 0..4 {
        app.update();
    }

    let world = app.world();
    let registry = world
        .get_resource::<ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry>()
        .expect("the composed host has a prepared-character registry");
    let catalog = world
        .get_resource::<ambition_platformer2d::character::CharacterCatalog>()
        .expect("the composed host has an assembled character catalog");
    let grid = ambition_demo_smash::select::SmashRoster::assemble(registry);
    let on_grid: Vec<String> = grid.ids().map(|id| id.to_string()).collect();

    let mut characters = Vec::new();
    for (id, prepared) in registry.iter() {
        // Only fighters with a moveset are balance subjects. A Hall NPC with no
        // authored table is a real catalog row and an empty inspector page.
        //
        // ⛔ AND AN EMPTY TABLE IS THE SAME ABSENCE. `projectable_moveset` answers
        // `Some` for a contract with no moves in it, which reaches the viewer as a
        // fighter whose whole page is blank — indistinguishable from a load failure.
        if prepared
            .kit
            .projectable_moveset()
            .is_none_or(|set| set.moves.is_empty())
        {
            continue;
        }
        characters.push(character_json(
            id,
            prepared,
            catalog.get(id),
            on_grid.iter().any(|g| g == id),
        ));
    }

    let bundle = serde_json::json!({
        "schema": SCHEMA,
        "sim_hz": SIM_HZ,
        "cast_generation": registry.generation().get(),
        "smash_grid": on_grid,
        "characters": characters,
    });

    let path = std::path::Path::new(&out);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).expect("the output directory is creatable");
    }
    let text = serde_json::to_string_pretty(&bundle).expect("the bundle serializes");
    std::fs::write(path, &text).expect("the bundle is writable");

    let mut summary = String::new();
    let _ = writeln!(
        summary,
        "[moveset-export] {} fighter(s), {} on the smash grid -> {}",
        bundle["characters"].as_array().map_or(0, Vec::len),
        on_grid.len(),
        out
    );
    print!("{summary}");
}
