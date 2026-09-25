//! Export every smash fighter's balance data as one JSON bundle.
//!
//! It answers balance questions without a playtest.
//!
//! It reads the composed host, not the authoring files. It boots
//! `build_visible_app`, waits for the `Startup` systems that fill the prepared
//! registry, and reports what the shipped game resolves. That can differ from
//! the source when a repertoire slot overwrites a move (see
//! `UpSpecial::into_spec`) or a provider builds a fighter from another's table.
//!
//! The schema here is its own, not `serde(MoveSpec)`. The catalog's field
//! names change with the engine; this file is the one translation, so the UI
//! has one shape to know.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use ambition_entity_catalog::{
    MoveCoverage, MoveEventKind, MoveSpec, MovesetContract, RecoveryUse, VolumeShape, WindowTag,
};

/// The sim tick. Frame counts in this bundle are seconds x this, which is the
/// unit every balance conversation in the genre is held in.
const SIM_HZ: f32 = 60.0;

/// The bundle's schema id. Bump the version when a consumer would break.
const SCHEMA: &str = "ambition.moveset_inspector.v2";

fn frames(seconds: f32) -> f32 {
    seconds * SIM_HZ
}

/// The atlas table for every sheet the exported cast names.
///
/// Enough to blit a frame: per sheet, the logical frame size, the page
/// images, the body rectangle and feet pixel, and each row's per-frame
/// sub-rects with trim offsets. That is the input `trimmed_render` takes, so
/// the viewer places a frame as the engine does.
///
/// The feet pixel is the horizontal origin, not the frame centre. A frame is
/// a packed cell sized by the widest pose, and the art sits wherever the crop
/// left it (`projectile_polygon` is 17% of a 377px frame left of centre).
fn sheet_atlas_json(characters: &[serde_json::Value]) -> serde_json::Value {
    use ambition_platformer2d::sprite_sheet::character::sheets::record_for_sheet_key;

    let mut wanted: std::collections::BTreeSet<String> = Default::default();
    for c in characters {
        if let Some(sheet) = c["spritesheet"].as_str() {
            // The catalog stores `sprites/<name>_spritesheet.png`; the baked
            // index is keyed by the bare name.
            let base = sheet
                .rsplit('/')
                .next()
                .unwrap_or(sheet)
                .trim_end_matches(".png")
                .trim_end_matches("_spritesheet");
            wanted.insert(base.to_string());
        }
    }

    let mut out = serde_json::Map::new();
    for key in wanted {
        let Some(record) = record_for_sheet_key(&key) else {
            // Not fatal: a fighter may name a sheet this build did not bake
            // (the quality variants are gitignored). The viewer falls back to
            // boxes for that fighter and says so.
            continue;
        };
        let metrics = record.body_metrics.as_ref();
        let rows: Vec<serde_json::Value> = record
            .rows
            .iter()
            .map(|row| {
                serde_json::json!({
                    "animation": row.animation,
                    "row_index": row.row_index,
                    "frame_count": row.frame_count,
                    "duration_secs": row.duration_secs,
                    "page": row.page,
                    // `[x, y, w, h, page, off_x, off_y]` per frame: a flat
                    // tuple, because a big sheet has hundreds of frames.
                    "rects": row
                        .rects
                        .iter()
                        .map(|r| serde_json::json!([r.x, r.y, r.w, r.h, r.page, r.off.0, r.off.1]))
                        .collect::<Vec<_>>(),
                })
            })
            .collect();
        out.insert(
            key.clone(),
            serde_json::json!({
                "image": record.image,
                "images": if record.images.is_empty() {
                    vec![record.image.clone()]
                } else {
                    record.images.clone()
                },
                "frame_width": record.frame_width,
                "frame_height": record.frame_height,
                "label_width": record.label_width,
                "y_offset": record.y_offset,
                "authored_faces_left": record.authored_faces_left,
                "body_pixel_bbox": metrics
                    .and_then(|m| m.body_pixel_bbox)
                    .map(|b| serde_json::json!([b.x, b.y, b.w, b.h])),
                "feet_pixel": metrics
                    .and_then(|m| m.feet_pixel)
                    .map(|p| serde_json::json!([p.x, p.y])),
                "rows": rows,
            }),
        );
    }
    serde_json::Value::Object(out)
}

/// One authored hit volume, flattened to the rectangle a viewer draws.
///
/// A circle reports its radius and its enclosing half-extents, so a
/// rectangle-only viewer still draws something true.
fn volume_json(volume: &ambition_entity_catalog::HitVolume) -> serde_json::Value {
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
            ambition_entity_catalog::VolumeReaction::Autolink(_) => "autolink",
            ambition_entity_catalog::VolumeReaction::Windbox(_) => "windbox",
        }),
        "on_hit": volume.on_hit.as_ref().map(|e| e.key.clone()),
        "vfx": volume.vfx.clone(),
        "hit_sfx": volume.hit_sfx.clone(),
    })
}

fn window_json(
    window: &ambition_entity_catalog::MoveWindow,
    // The contract this window belongs to: a broad cancel rule means this
    // character's moves.
    moveset: Option<&ambition_entity_catalog::MovesetContract>,
) -> serde_json::Value {
    let (tag, cancel_into) = match &window.tag {
        WindowTag::Startup => ("startup".to_string(), Vec::new()),
        WindowTag::Active => ("active".to_string(), Vec::new()),
        WindowTag::Recovery => ("recovery".to_string(), Vec::new()),
        WindowTag::Invuln => ("invuln".to_string(), Vec::new()),
        WindowTag::Armor => ("armor".to_string(), Vec::new()),
        // Keep the threshold in the tag string. Plain `"armor"` would read as
        // super armor.
        WindowTag::ArmorUnder { damage } => (format!("armor_under:{damage}"), Vec::new()),
        WindowTag::Cancelable { into, condition } => (
            format!("cancelable:{condition:?}").to_lowercase(),
            into.clone(),
        ),
    };
    // Resolve the broad rules (`["attack", "smash", "any_attack"]`) to
    // this character's move ids. `cancel_targets` is the catalog's own
    // resolution, the same verb-class names the trigger path matches.
    let resolved: Vec<String> = if cancel_into.is_empty() {
        Vec::new()
    } else {
        moveset
            .map(|contract| {
                contract
                    .cancel_targets(&cancel_into)
                    .into_iter()
                    .map(|mv| mv.id.clone())
                    .collect()
            })
            .unwrap_or_default()
    };
    serde_json::json!({
        "tag": tag,
        "cancel_into": cancel_into,
        // The move ids `cancel_into` admits for this character. Empty when the
        // window authors no cancel. A rule that resolves to nothing names moves
        // this fighter does not have.
        "cancel_into_resolved": resolved,
        "start_s": window.start_s,
        "end_s": window.end_s,
        "start_f": frames(window.start_s),
        "end_f": frames(window.end_s),
        "motion_scale": window.motion_scale,
        "sustain_effect": window.sustain_effect.as_ref().map(|e| e.key.clone()),
        "volumes": window.volumes.iter().map(volume_json).collect::<Vec<_>>(),
    })
}

/// The move's Active windows grouped into pulses: one continuous stretch of
/// Active time each, in start order.
///
/// Contiguity is the runtime's rule. Windows that touch or overlap are one
/// pulse and share one per-victim ledger; a real gap earns a second
/// connection. Any other rule makes a sweetspot pair look like a multihit.
fn active_pulses(spec: &MoveSpec) -> Vec<Vec<&ambition_entity_catalog::MoveWindow>> {
    let mut actives: Vec<&ambition_entity_catalog::MoveWindow> = spec
        .windows
        .iter()
        .filter(|w| matches!(w.tag, WindowTag::Active))
        .collect();
    actives.sort_by(|a, b| a.start_s.total_cmp(&b.start_s));

    let mut pulses: Vec<Vec<&ambition_entity_catalog::MoveWindow>> = Vec::new();
    let mut open_until = f32::NEG_INFINITY;
    for window in actives {
        // `>` not `>=`: windows that only touch leave no Active gap, so the
        // ledger never reset.
        if window.start_s > open_until {
            pulses.push(Vec::new());
            open_until = f32::NEG_INFINITY;
        }
        open_until = open_until.max(window.end_s);
        pulses
            .last_mut()
            .expect("a pulse was opened above")
            .push(window);
    }
    pulses
}

/// How much of the body's own silhouette a move's forward reach covers, and
/// where the holes are.
///
/// D203: a directional smash should cover a contiguous region in the facing
/// direction at least as tall as the character, and a forward smash should
/// leave no hole a crouching opponent can duck under. Hit volumes are authored
/// in absolute numbers (`VolumeShape::Rect { offset, half_extents }` is
/// entity-local), so a bigger fighter inherits a box drawn for a smaller one.
///
/// It reports; it does not judge. Some moves break the rule on purpose, and a
/// pass/fail here would have no way to declare an exception.
///
/// `coverage` is the union of a move's Active volumes in body-local space,
/// where `+x` is facing and the origin is the body's centre.
///
/// It describes the volume, not where the move lands. A move with a
/// `start_impulse` moves the body while its window is open, so its world
/// reach exceeds `reach_px`. Example: the oni leader's `iaijutsu`
/// (`impulse(0.05, (700.0, 0.0))` at the start of its Active window). So a
/// small `covers_body_fraction` on a lunge is expected. Many moves thrown past
/// their reach have no impulse, so this does not explain that pattern (see
/// D190).
fn coverage_census(coverage: &MoveCoverage, body_height: f32) -> Option<serde_json::Value> {
    // A move that reaches nowhere forward is not a directional attack; this
    // question does not apply.
    if coverage.max.0 <= 0.0 || body_height <= 0.0 {
        return None;
    }
    let half = body_height / 2.0;
    // The silhouette, in the same body-local frame. World y is down, so the
    // "duck under" hole is the one toward +y.
    let (top, bottom) = (-half, half);
    let covered_top = coverage.min.1.max(top);
    let covered_bottom = coverage.max.1.min(bottom);
    let covered = (covered_bottom - covered_top).max(0.0);
    // Round in f64, not f32. People read these and bundles are byte-diffed;
    // an f32 third of 48 widens to `0.3330000042915344`.
    let round = |v: f32, places: f64| (f64::from(v) * places).round() / places;
    Some(serde_json::json!({
        // What a reader compares between two characters: 1.0 is a box as tall
        // as its owner.
        "covers_body_fraction": round(covered / body_height, 1000.0),
        // The band above the volume: an opponent standing tall on a slope, or
        // an aerial the move passes under.
        "gap_above_px": round((coverage.min.1 - top).max(0.0), 10.0),
        // The band between the bottom of the volume and the owner's feet.
        "gap_below_px": round((bottom - coverage.max.1).max(0.0), 10.0),
        // The question that band stands for, answered directly. A crouch is
        // half height in this engine (`BodyShape::Crouching`, same width), so
        // a same-sized crouching opponent fills the lower half of the
        // silhouette. This is how much of that half the volume overlaps: `0.0`
        // is duckable. The pixel gap alone cannot say this.
        "covers_crouched_fraction": round(
            ((coverage.max.1.min(bottom) - coverage.min.1.max(0.0)).max(0.0)) / half,
            1000.0,
        ),
        "reach_px": round(coverage.max.0, 10.0),
        "body_height_px": round(body_height, 10.0),
    }))
}

/// `body_ranged` is the fighter's standing ranged kit: the action a firing
/// move uses when it equips nothing of its own.
///
/// `startup` is the first Active window's start, not the end of the first
/// Startup window. They differ for a timeline with a gap, a multi-hit, or an
/// `Invuln` window.
fn derived_json(
    spec: &MoveSpec,
    body_ranged: Option<&ambition_platformer2d::characters::brain::RangedActionSpec>,
) -> serde_json::Value {
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
    let volumes: Vec<_> = spec.windows.iter().flat_map(|w| w.volumes.iter()).collect();
    // The biggest single connection (for kill power) and the full carry of a
    // multi-hit are different questions, so both ship.
    let max_damage = volumes.iter().map(|v| v.damage).max().unwrap_or(0);
    // The carry is by pulse, not by volume. Sibling volumes inside a pulse
    // share one per-victim ledger, so they are alternatives; only separate
    // pulses accumulate. This is the static upper bound: the best volume of
    // each pulse, summed.
    let sum_damage: i32 = active_pulses(spec)
        .iter()
        .map(|pulse| {
            pulse
                .iter()
                .flat_map(|w| w.volumes.iter())
                .map(|v| v.damage)
                .max()
                .unwrap_or(0)
        })
        .sum();
    let max_knockback = volumes.iter().map(|v| v.knockback).fold(0.0f32, f32::max);
    // Reach is to the far edge of the volume along the facing axis: a box
    // centred at 34 with a 26 half-extent reaches 60.
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
    // A ranged move is described in its own terms: when it fires, and what
    // the shot is worth. Melee fields alone would show no startup, zero
    // damage, and the whole duration as endlag. A move can have both body
    // damage and projectile damage; both ship.
    let fire_at_s = spec
        .events
        .iter()
        .filter(|e| matches!(e.kind, MoveEventKind::Ranged))
        .map(|e| e.at_s)
        .fold(f32::NAN, f32::min);
    let fires_projectile = !fire_at_s.is_nan();
    // The shot, in the runtime's precedence: what the move equips, then the
    // body's standing ranged kit. A move whose content is firing that kit has
    // no other offence, so the kit must be reported for it.
    let equipped = spec
        .equips
        .as_deref()
        .and_then(ambition_platformer2d::characters::brain::held_item_by_id)
        .and_then(|item| item.ranged);
    let shot_source = if equipped.is_some() {
        "equipped"
    } else {
        "body"
    };
    // Only for a move that fires. Otherwise every move of a fighter who owns
    // a cannon would report cannon damage. The precedence above is unchanged;
    // this gates whether the question is asked.
    let shot = fires_projectile
        .then(|| equipped.or_else(|| body_ranged.cloned()))
        .flatten();
    // A chargeable shot reports its ceiling too: a tap and a full hold are
    // different balance facts.
    let charged = shot.as_ref().and_then(|r| {
        r.charge.as_ref().map(|c| {
            (
                (r.damage as f32 * c.damage_mult).round() as i32,
                r.speed * c.speed_mult,
                c.size_mult,
            )
        })
    });
    // Endlag: from the last committed instant to the end of the move. For a
    // ranged move that is the fire frame. With neither, the move owes its
    // whole duration.
    let last_committed = if actives.is_empty() {
        (!fire_at_s.is_nan()).then_some(fire_at_s)
    } else {
        Some(last_active_end)
    };
    let endlag_s = match last_committed {
        Some(at) => (spec.duration_s - at).max(0.0),
        // A pure-mobility special owes its whole duration.
        None => spec.duration_s,
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
        // The ranged analogue of startup: the instant the shot leaves.
        "fire_at_s": fires_projectile.then_some(fire_at_s),
        "fire_f": fires_projectile.then(|| frames(fire_at_s)),
        "projectile_damage": shot.as_ref().map(|r| r.damage),
        "projectile_speed": shot.as_ref().map(|r| r.speed),
        // Whose shot: an equipped weapon and the body's own kit are different
        // balance facts.
        "projectile_source": shot.as_ref().map(|_| shot_source),
        // `None` for a shot that does not charge, which is every ranged action
        // that has not opted in.
        "projectile_damage_charged": charged.map(|(d, _, _)| d),
        "projectile_speed_charged": charged.map(|(_, s, _)| s),
        "projectile_size_charged": charged.map(|(_, _, m)| m),
        // The charge payoff a full hold applies, so a smash's ceiling is one
        // multiplication away.
        "max_damage_charged": (max_damage as f32 * spec.smash_charge_mult).round() as i32,
    })
}

fn event_json(event: &ambition_entity_catalog::MoveEvent) -> serde_json::Value {
    let (kind, detail) = match &event.kind {
        MoveEventKind::Sfx { cue } => ("sfx", cue.clone()),
        MoveEventKind::Vfx { effect, .. } => ("vfx", effect.clone()),
        MoveEventKind::Effect(effect) => ("effect", effect.key.clone()),
        // Exhaustive on purpose: a new event kind is a new thing a move can
        // do, and the compiler should name it instead of a catch-all "other".
        MoveEventKind::Impulse { local, mode, .. } => {
            ("impulse", format!("{mode:?} ({}, {})", local.0, local.1))
        }
        MoveEventKind::Ranged => ("ranged", String::new()),
        // The duration is in the detail: a scale says how much lighter the
        // body gets, and only the seconds say whether it outlives the move.
        MoveEventKind::GravityModifier { scale, seconds } => {
            ("gravity_modifier", format!("x{scale} for {seconds}s"))
        }
        MoveEventKind::HoldVelocity { seconds } => ("hold_velocity", format!("{seconds}s")),
    };
    serde_json::json!({
        "at_s": event.at_s,
        "at_f": frames(event.at_s),
        "kind": kind,
        "detail": detail,
    })
}

fn move_json(
    spec: &MoveSpec,
    verbs: &[String],
    // The whole contract, so a cancel rule can be resolved against the moves
    // this fighter actually has.
    moveset: Option<&ambition_entity_catalog::MovesetContract>,
    body_ranged: Option<&ambition_platformer2d::characters::brain::RangedActionSpec>,
    // How tall the owner stands, for the coverage census. `0.0` when the
    // character declares none and its `body_kind` has no default.
    body_height: f32,
) -> serde_json::Value {
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
        "windows": spec
            .windows
            .iter()
            .map(|window| window_json(window, moveset))
            .collect::<Vec<_>>(),
        "events": spec.events.iter().map(event_json).collect::<Vec<_>>(),
        "derived": derived_json(spec, body_ranged),
        // D203's census. `None` for a move with no forward Active volume, or
        // a character with no stated height: "cannot say", not "covers
        // nothing". Read it; do not gate on it. See `coverage_census`.
        "coverage": spec
            .frame_data()
            .coverage
            .as_ref()
            .and_then(|c| coverage_census(c, body_height)),
    })
}

/// Move id → the verbs that reach it, so the viewer can show a move under the
/// button a player presses instead of under an internal id.
fn verbs_by_move(contract: &MovesetContract) -> BTreeMap<String, Vec<String>> {
    let mut by_move: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (verb, move_id) in &contract.verbs {
        by_move
            .entry(move_id.clone())
            .or_default()
            .push(verb.clone());
    }
    by_move
}

fn portrait_json(
    id: &str,
    prepared: &ambition_platformer2d::characters::prepared::PreparedCharacterDefinition,
    catalog: &ambition_platformer2d::character::CharacterCatalog,
    portraits: &ambition_platformer2d::sprite_sheet::PortraitSheetRegistry,
) -> Option<serde_json::Value> {
    let reference = ambition_platformer2d::character::portrait_for_declared_character(
        Some(portraits),
        catalog,
        prepared.portrait.as_deref(),
        id,
    )?;
    let (clip, frame) = portraits.resolve_still(
        &reference.manifest,
        None,
        Some(&reference.still_clip),
    )?;
    Some(serde_json::json!({
        "image": reference.image,
        "manifest": reference.manifest,
        "clip": clip,
        "frame": [frame.x, frame.y, frame.w, frame.h],
    }))
}

fn character_json(
    id: &str,
    prepared: &ambition_platformer2d::characters::prepared::PreparedCharacterDefinition,
    catalog: Option<
        &ambition_platformer2d::characters::actor::character_catalog::CharacterCatalogEntry,
    >,
    portrait_art: Option<serde_json::Value>,
    on_grid: bool,
) -> serde_json::Value {
    let contract = prepared.kit.projectable_moveset();
    let by_move = contract.map(verbs_by_move).unwrap_or_default();
    // The body's standing ranged kit: what a firing move that equips nothing
    // fires. See `derived_json`.
    let body_ranged = prepared
        .kit
        .action_set()
        .and_then(|set| set.ranged.as_ref());
    // The character's own height, which D203's rules are stated against: an
    // authored `standing_height`, else its `body_kind` default (48 for
    // `Standard`, none for the rest).
    let body_height = catalog
        .and_then(|e| {
            e.standing_height
                .or_else(|| e.body_kind.default_standing_height())
        })
        .unwrap_or(0.0);
    let moves: Vec<_> = contract
        .map(|c| {
            c.moves
                .iter()
                .map(|m| {
                    move_json(
                        m,
                        by_move.get(&m.id).map(Vec::as_slice).unwrap_or(&[]),
                        Some(c),
                        body_ranged,
                        body_height,
                    )
                })
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
        // `portrait` is the authored logical target. `portrait_art` is the
        // resolved still frame the shipped select screen would draw, including
        // its crop inside the independently published portrait sheet.
        "portrait": prepared.portrait.clone(),
        "portrait_art": portrait_art,
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
        // The silhouette the volumes are offset from, so a viewer can judge a
        // hitbox's size.
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

const USAGE: &str = "\
moveset_export — export every smash fighter's balance data as one JSON bundle.

USAGE:
    moveset_export [--out PATH]

OPTIONS:
    --out PATH   where to write the bundle
                 [default: tools/ambition_moveset_inspector/data/moveset_bundle.json]
    -h, --help   print this and exit

NOTES:
    Boots the real composed host and reports what it RESOLVES, not what an
    authoring file writes — the two differ wherever a repertoire slot overwrites
    a move or a provider composes one fighter out of another's table.

    There is no positional argument. `moveset_export out.json` writes to the
    DEFAULT path, silently; use --out.

    Takes ~1 minute: it builds the whole app to ask it questions.
";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // Handle `--help` before the app boots.
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print!("{USAGE}");
        return;
    }
    // An unknown flag is a refusal. The parser scans pairs for `--out`, so
    // other arguments would otherwise be ignored silently.
    if let Some(bad) = args
        .iter()
        .skip(1)
        .filter(|a| a.starts_with('-'))
        .find(|a| *a != "--out")
    {
        eprintln!("moveset_export: unknown option '{bad}'\n");
        print!("{USAGE}");
        std::process::exit(2);
    }
    let out = args
        .windows(2)
        .find(|w| w[0] == "--out")
        .map(|w| w[1].clone())
        .unwrap_or_else(|| "tools/ambition_moveset_inspector/data/moveset_bundle.json".to_string());

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    // A `Startup` system fills the registry, so before the first update there
    // is a catalog and no registry. One frame is enough; several are cheap.
    for _ in 0..4 {
        app.update();
    }

    let world = app.world();
    let registry = world
        .get_resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>()
        .expect("the composed host has a prepared-character registry");
    let catalog = world
        .get_resource::<ambition_platformer2d::character::CharacterCatalog>()
        .expect("the composed host has an assembled character catalog");
    let grid = ambition_demo_smash::select::SmashRoster::assemble(registry);
    let on_grid: Vec<String> = grid.ids().map(|id| id.to_string()).collect();
    // The select screen resolves still portraits through this baked registry.
    // Build it once, not once per fighter.
    let portraits = ambition_platformer2d::sprite_sheet::baked_portrait_registry();

    let mut characters = Vec::new();
    for (id, prepared) in registry.iter() {
        // Only fighters with a moveset are balance subjects. A Hall NPC with
        // no table would be an empty page. An empty table is the same:
        // `projectable_moveset` returns `Some` for a contract with no moves,
        // and a blank page looks like a load failure.
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
            portrait_json(id, prepared, catalog, &portraits),
            on_grid.iter().any(|g| g == id),
        ));
    }

    // The art: one atlas table per sheet the cast uses, so the viewer can
    // blit the same sub-rect the engine does.
    let sheets = sheet_atlas_json(&characters);

    let bundle = serde_json::json!({
        "schema": SCHEMA,
        "sim_hz": SIM_HZ,
        "cast_generation": registry.generation().get(),
        "smash_grid": on_grid,
        "characters": characters,
        "sheets": sheets,
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

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::characters::brain::RangedActionSpec;

    /// A cannon on the body is not a cannon on every move.
    ///
    /// Both arms are asserted from the same body kit: a move that fires must
    /// report the kit, and a move that does not must not.
    fn cannon() -> RangedActionSpec {
        RangedActionSpec::pistol(900.0, 11)
    }

    fn spec_named(id: &str, fires: bool) -> MoveSpec {
        let mut spec = ambition_entity_catalog::authoring::strike(
            ambition_entity_catalog::authoring::Strike {
                id,
                clip: "jab",
                startup_s: 0.05,
                active_s: 0.04,
                recover_s: 0.10,
                offset: (20.0, 0.0),
                half_extents: (14.0, 10.0),
                damage: 3,
                knockback: 40.0,
                knockback_growth: 1.0,
                launch_dir: None,
                on_hit: None,
            },
        );
        if fires {
            spec.events
                .push(ambition_entity_catalog::MoveEvent {
                    at_s: 0.05,
                    kind: MoveEventKind::Ranged,
                });
        }
        spec
    }

    /// D203's census on the forward-smash case: no hole a crouching opponent
    /// can duck under.
    ///
    /// A hit volume is absolute, so the same box fronts less of a taller
    /// fighter. The census reports rather than judges, so these arms check
    /// the measurement, not a verdict.
    #[test]
    fn the_census_names_the_band_a_crouching_opponent_would_duck_into() {
        // World y is down, so a 48-tall body spans -24 (head) to +24 (feet).
        let waist_high = MoveCoverage {
            min: (10.0, -12.0),
            max: (46.0, 4.0),
        };
        let census = coverage_census(&waist_high, 48.0).expect("it reaches forward");
        assert_eq!(census["reach_px"], serde_json::json!(46.0));
        // 16 of 48 covered.
        assert_eq!(census["covers_body_fraction"], serde_json::json!(0.333));
        assert_eq!(census["gap_above_px"], serde_json::json!(12.0));
        // 20px of standing room below the swing.
        assert_eq!(census["gap_below_px"], serde_json::json!(20.0));
        // A crouching opponent fills the lower half (`0..24` here). This swing
        // reaches `+4`, so it overlaps 4 of those 24.
        assert_eq!(census["covers_crouched_fraction"], serde_json::json!(0.167));
    }

    /// The pixel gap alone cannot answer the crouch question, so the crouch
    /// fraction is its own field. A crouch is half height
    /// (`BodyShape::Crouching`): a swing that stops at the standing midline
    /// touches a crouching opponent at one point, and a swing one pixel above
    /// it touches nothing.
    #[test]
    fn a_swing_that_stops_above_the_midline_is_the_one_a_crouch_ducks() {
        let stops_at_the_midline = MoveCoverage {
            min: (10.0, -20.0),
            max: (46.0, 0.0),
        };
        let just_above = MoveCoverage {
            min: (10.0, -20.0),
            max: (46.0, -1.0),
        };
        assert_eq!(
            coverage_census(&stops_at_the_midline, 48.0).unwrap()["covers_crouched_fraction"],
            serde_json::json!(0.0),
        );
        assert_eq!(
            coverage_census(&just_above, 48.0).unwrap()["covers_crouched_fraction"],
            serde_json::json!(0.0),
        );
        // One pixel lower and it is no longer duckable.
        let reaches_in = MoveCoverage {
            min: (10.0, -20.0),
            max: (46.0, 1.0),
        };
        assert_eq!(
            coverage_census(&reaches_in, 48.0).unwrap()["covers_crouched_fraction"],
            serde_json::json!(0.042),
        );
    }

    /// A box as tall as its owner has no hole either way: the reference shape.
    #[test]
    fn a_volume_as_tall_as_its_owner_reports_no_gap() {
        let full = MoveCoverage {
            min: (8.0, -24.0),
            max: (52.0, 24.0),
        };
        let census = coverage_census(&full, 48.0).expect("it reaches forward");
        assert_eq!(census["covers_body_fraction"], serde_json::json!(1.0));
        assert_eq!(census["gap_above_px"], serde_json::json!(0.0));
        assert_eq!(census["gap_below_px"], serde_json::json!(0.0));
    }

    /// Two cases the census must refuse: a move whose volumes are all behind
    /// the owner (not a directional attack), and a character with no stated
    /// height (nothing to compare against).
    #[test]
    fn the_census_says_nothing_rather_than_guessing() {
        let behind = MoveCoverage {
            min: (-40.0, -10.0),
            max: (-8.0, 10.0),
        };
        assert!(coverage_census(&behind, 48.0).is_none());
        let forward = MoveCoverage {
            min: (8.0, -10.0),
            max: (40.0, 10.0),
        };
        assert!(coverage_census(&forward, 0.0).is_none());
    }

    #[test]
    fn a_move_that_fires_nothing_reports_no_shot_even_on_an_armed_body() {
        let kit = cannon();
        let quiet = derived_json(&spec_named("jab", false), Some(&kit));
        assert_eq!(quiet["fires_projectile"], serde_json::json!(false));
        for field in ["projectile_damage", "projectile_speed", "projectile_source"] {
            assert!(
                quiet[field].is_null(),
                "a jab exported `{field}` = {} because the fighter owns a cannon",
                quiet[field]
            );
        }

        let firing = derived_json(&spec_named("neutral_b", true), Some(&kit));
        assert_eq!(firing["fires_projectile"], serde_json::json!(true));
        assert_eq!(firing["projectile_damage"], serde_json::json!(11));
        assert_eq!(firing["projectile_speed"], serde_json::json!(900.0));
        assert_eq!(firing["projectile_source"], serde_json::json!("body"));
    }
}
