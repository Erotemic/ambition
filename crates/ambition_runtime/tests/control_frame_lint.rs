//! **The `ControlFrame` allowlist lint** — unified-actors step 5's Phase C, and
//! the guardrail on the S5/S6 player fold.
//!
//! The invariant, in one sentence:
//!
//! > The global `ControlFrame` is ONE player's device frame. Only the INPUT LAYER
//! > may hold it. A body/sim system that reads it is silently slot-0-only.
//!
//! Everything else consumes an ENTITY-LOCAL frame instead — `PlayerInputFrame`
//! (the local player's slot frame, gated on brain ownership) or `ActorControl`
//! (the brain's per-body intent). That is what makes possession, co-op, and
//! netcode slot routing (N1) work: the body is driven by *its* controller, not by
//! *the* controller.
//!
//! ## Why this is a lint and not a paragraph
//!
//! `unified-actors.md` B3 asserted the invariant held, and named the holders. By
//! 2026-07-10 that sentence was stale in BOTH directions: it named
//! `sync_local_player_input_frame` (which reads `Res<SlotControls>`, not the
//! frame) and it missed three real holders. Nothing guarded it —
//! `architecture_boundaries.rs` only asserts that `ControlFrame` *lives* in
//! `engine_core`. So the invariant moved, unnoticed, and the only reason it did
//! not break was luck.
//!
//! This lint is bidirectional on purpose, because that drift was bidirectional:
//!   * a holder that is not in [`ALLOWLIST`] fails the lint;
//!   * an [`ALLOWLIST`] entry that matches no holder ALSO fails the lint.
//!
//! Same shape and same file family as `determinism_lints.rs` (ADR 0023): a grep
//! over the sim crates' non-test sources, an explicit allowlist with a justifying
//! reason per entry, and a failure message naming the file, the line, and the fix.
//!
//! Escape hatch: `AMBITION_REVIEW(control_frame)` on the offending line or in the
//! comment block directly above it. `reviewed_control_frame_exceptions_are_listed`
//! prints every one so an auditor reads the whole set at once.

use std::path::{Path, PathBuf};

/// The crates whose sources ARE the simulation, plus the game's CONTENT crate —
/// content authors rules, and a rule that reads the global frame is as slot-0-only
/// as an engine system that does. Presentation, device-input, audio, render, menu,
/// and the app shell are excluded by construction: sampling and displaying the
/// local player's device frame is exactly their job.
const SIM_CRATES: &[&str] = &[
    "crates/ambition_engine_core",
    "crates/ambition_platformer_primitives",
    "crates/ambition_time",
    "crates/ambition_entity_catalog",
    "crates/ambition_world",
    "crates/ambition_characters",
    "crates/ambition_combat",
    "crates/ambition_projectiles",
    "crates/ambition_portal",
    "crates/ambition_encounter",
    "crates/ambition_items",
    "crates/ambition_cutscene",
    "crates/ambition_interaction",
    "crates/ambition_sim_view",
    "crates/ambition_actors",
    "crates/ambition_runtime",
    "game/ambition_content",
];

/// Paths inside a sim crate that are NOT sim. Mirrors `determinism_lints.rs`.
const NON_SIM_SUBPATHS: &[&str] = &[
    "menu/",       // map/pause UI — presentation
    "dev/",        // trace dump, debug overlays
    "audio/",      // cue playback — feel clock
    "assets/",     // asset loading
    "persistence", // save I/O
];

/// How a holder is allowed to touch the global frame. Every [`ALLOWLIST`] entry
/// declares one, so "why is this here" is answerable without reading the system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Bridge {
    /// DEVICE → frame. Samples a physical controller into this frame. The frame
    /// exists for exactly this.
    DeviceToFrame,
    /// frame ↔ tick LATCH. Folds feel-clock samples into a tick frame and back.
    Latch,
    /// frame → SLOT. Fans the one device frame out to the slot vocabulary every
    /// body actually reads (`SlotControls` / `SlotInteractionState`).
    FrameToSlot,
    /// frame ↔ an adapter's intent struct, in the input phase, for a subsystem
    /// that must not name `ControlFrame` itself.
    IntentBridge,
    /// **A SIM system reading the device frame, slot-0-only by design.** Every
    /// entry here is a multiplayer TODO with a name. Adding one is a decision, not
    /// an accident — which is the whole point of this lint.
    Slot0Gesture,
}

/// The complete, justified set of global-`ControlFrame` holders in the sim crates.
///
/// `(file, fn, bridge, why)`. Keyed by function name rather than line number so
/// the entry survives edits above it — and so a RENAME shows up as a lint failure
/// rather than as silent drift.
const ALLOWLIST: &[(&str, &str, Bridge, &str)] = &[
    // ── engine_core: the frame's owner, and the two halves of the N0.1 latch ──
    (
        "crates/ambition_engine_core/src/control_frame.rs",
        "accumulate_control_frame_latch",
        Bridge::Latch,
        "FEEL clock: folds this render frame's device sample into the tick latch. \
         Runs in Update after every writer.",
    ),
    (
        "crates/ambition_engine_core/src/control_frame.rs",
        "publish_latched_control_frame",
        Bridge::Latch,
        "TICK clock: publishes the latched frame as THIS tick's ControlFrame, at \
         the head of the sim input phase, before any reader.",
    ),
    // ── ambition_actors: the input layer ─────────────────────────────────────
    (
        "crates/ambition_actors/src/schedule/input_systems.rs",
        "populate_control_frame_from_actions",
        Bridge::DeviceToFrame,
        "THE device→frame bridge: leafwing ActionState → ControlFrame. If any \
         system may hold this resource, it is this one.",
    ),
    (
        "crates/ambition_actors/src/player/input_systems.rs",
        "input_timer_system",
        Bridge::DeviceToFrame,
        "Derives edge/timer facts (dash double-tap window, buffered jump) from the \
         raw device frame and writes them back into it. Still the input layer: it \
         refines the frame before any body reads a slot.",
    ),
    (
        "crates/ambition_actors/src/player/input_systems.rs",
        "interaction_input_system",
        Bridge::FrameToSlot,
        "frame→slot bridge: recognizes the interact/double-tap-up gestures and \
         writes SlotInteractionState, which is what bodies actually read.",
    ),
    (
        "crates/ambition_actors/src/player/systems.rs",
        "populate_slot_controls",
        Bridge::FrameToSlot,
        "THE frame→slot bridge: copies the one device frame into SlotControls[0]. \
         N1 replaces this body with a per-slot fan-in; every downstream reader \
         already speaks slots, which is why that change stays local.",
    ),
    (
        "crates/ambition_actors/src/abilities/traversal/possession.rs",
        "possession_trigger_system",
        Bridge::Slot0Gesture,
        "MULTIPLAYER TODO (N1). A SIM system reading the device frame directly. The \
         possession gesture (hold Down+Interact ~2s) is authored as slot 0's, and \
         its own doc comment says so — but that makes possession local-player-only: \
         a second player could never possess anything. The fix is to read \
         SlotInteractionState/SlotControls for the acting slot, exactly as \
         interaction_input_system already does for the interact buffer. Left as-is \
         because rewriting the gesture is a behavior change, not a refactor; \
         enumerated here so N1 has a checklist entry instead of a surprise.",
    ),
    // ── ambition_content: the portal input adapters ──────────────────────────
    (
        "game/ambition_content/src/portal/transit_adapter.rs",
        "sync_movement_intent_from_control",
        Bridge::IntentBridge,
        "frame→intent: copies the held movement axes into PlayerMovementIntent so \
         ambition_portal's core never names ControlFrame. Runs in the input phase, \
         before warp_portal_input.",
    ),
    (
        "game/ambition_content/src/portal/transit_adapter.rs",
        "apply_movement_intent_to_control",
        Bridge::IntentBridge,
        "intent→frame: writes the portal-warped axes back, so the brain and the \
         movement pipeline see the adjusted stick exactly as they did when portal \
         core mutated ControlFrame directly. Still the input phase.",
    ),
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root")
}

/// Every non-test `.rs` file in the sim crates, as `(repo-relative label, text)`.
fn sim_sources() -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                out.push(path);
            }
        }
    }

    let root = repo_root();
    let mut out = Vec::new();
    for krate in SIM_CRATES {
        let src = root.join(krate).join("src");
        let mut files = Vec::new();
        walk(&src, &mut files);
        for path in files {
            let rel = path
                .strip_prefix(&src)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if rel.ends_with("tests.rs") || rel.contains("/tests/") || rel.starts_with("tests/") {
                continue;
            }
            if NON_SIM_SUBPATHS.iter().any(|skip| rel.contains(skip)) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            out.push((format!("{krate}/src/{rel}"), text));
        }
    }
    assert!(
        out.len() > 200,
        "the sim-source walk found only {} files — the crate paths are probably wrong, \
         and a lint that scans nothing passes vacuously",
        out.len()
    );
    out
}

fn is_comment(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("//") || t.starts_with('*') || t.starts_with("#![") || t.starts_with("#[")
}

/// `AMBITION_REVIEW(control_frame)` on the line, or in the comment block above it.
fn is_reviewed(lines: &[&str], idx: usize) -> bool {
    const MARKER: &str = "AMBITION_REVIEW(control_frame)";
    if lines[idx].contains(MARKER) {
        return true;
    }
    for line in lines[..idx].iter().rev() {
        if !is_comment(line) {
            return false;
        }
        if line.contains(MARKER) {
            return true;
        }
    }
    false
}

/// `true` when `ControlFrame` occurs at `idx` as a WHOLE type name.
///
/// Both sides matter, and each caught a bug in this scanner's first draft:
///   * suffix — `ControlFrameLatch` / `ControlFrameModes` / `ControlFrameTrace`;
///   * prefix — `MenuControlFrame`, which is the MENU's frame and not this one.
/// A leading `::` is fine: that is an import path (`ambition_input::ControlFrame`).
fn is_whole_word_at(line: &str, idx: usize) -> bool {
    const NAME: &str = "ControlFrame";
    let after_ok = line[idx + NAME.len()..]
        .chars()
        .next()
        .is_none_or(|c| !c.is_alphanumeric() && c != '_');
    let before_ok = line[..idx]
        .chars()
        .next_back()
        .is_none_or(|c| !c.is_alphanumeric() && c != '_');
    after_ok && before_ok
}

/// One `Res<…ControlFrame>` / `ResMut<…ControlFrame>` (or direct `World` resource
/// access) found in a source file, attributed to its enclosing function.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Holder {
    file: String,
    line: usize,
    func: String,
    text: String,
}

/// Walk back from `idx` to the nearest enclosing `fn NAME(`.
fn enclosing_fn(lines: &[&str], idx: usize) -> String {
    for line in lines[..=idx].iter().rev() {
        let t = line.trim_start();
        let after_vis = t
            .strip_prefix("pub(crate) ")
            .or_else(|| t.strip_prefix("pub(super) "))
            .or_else(|| t.strip_prefix("pub "))
            .unwrap_or(t);
        let after_async = after_vis.strip_prefix("async ").unwrap_or(after_vis);
        if let Some(rest) = after_async.strip_prefix("fn ") {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                return name;
            }
        }
    }
    "<free-standing>".to_string()
}

/// Scan one source file for global-frame holders, skipping `#[cfg(test)]` blocks
/// and `AMBITION_REVIEW(control_frame)` lines.
///
/// This is a pure function of `(file, text)` so the poison test can exercise it on
/// synthetic sources without touching the tree. A lint you cannot make fail is
/// worse than no lint.
fn control_frame_holders(file: &str, text: &str) -> Vec<Holder> {
    const NAME: &str = "ControlFrame";
    let lines: Vec<&str> = text.lines().collect();
    let mut holders = Vec::new();

    // `#[cfg(test)]` block skipping, by brace balance. Test modules legitimately
    // seed the frame to drive a system under test.
    let mut skip_depth: i32 = 0;
    let mut in_cfg_test = false;

    for (i, line) in lines.iter().enumerate() {
        if in_cfg_test {
            skip_depth += line.matches('{').count() as i32;
            skip_depth -= line.matches('}').count() as i32;
            if skip_depth <= 0 {
                in_cfg_test = false;
            }
            continue;
        }
        if line.trim() == "#[cfg(test)]" {
            in_cfg_test = true;
            skip_depth = 0;
            continue;
        }
        if is_comment(line) || is_reviewed(&lines, i) {
            continue;
        }

        // A `SystemParam` borrow: find `Res<` / `ResMut<`, then look for a
        // whole-word `ControlFrame` inside the generic argument. Handles
        // `Option<Res<ControlFrame>>`, `Res<'w, ControlFrame>`, and any import
        // path (`ambition_input::ControlFrame`, `ae::ControlFrame`, …) — the path
        // is exactly how the fifth holder hid from a plain name grep.
        let mut hit = false;
        for open in ["ResMut<", "Res<"] {
            let mut from = 0usize;
            while let Some(rel) = line[from..].find(open) {
                let start = from + rel + open.len();
                let end = line[start..].find('>').map_or(line.len(), |e| start + e);
                if let Some(rel_name) = line[start..end].find(NAME) {
                    if is_whole_word_at(line, start + rel_name) {
                        hit = true;
                    }
                }
                from = start;
            }
            if hit {
                break;
            }
        }
        // Direct `World` access is the same read by another door. `init_resource`
        // is REGISTRATION, not a read — and because `init_resource::<` contains
        // `resource::<`, the door must begin at a word boundary or the frame's own
        // owner (`SimCoreResourcesPlugin::build`) trips its own lint.
        if !hit {
            for door in ["resource::<", "resource_mut::<", "get_resource::<"] {
                let mut from = 0usize;
                while let Some(rel) = line[from..].find(door) {
                    let at = from + rel;
                    let boundary = line[..at]
                        .chars()
                        .next_back()
                        .is_none_or(|c| !c.is_alphanumeric() && c != '_');
                    let start = at + door.len();
                    let end = line[start..].find('>').map_or(line.len(), |e| start + e);
                    if boundary {
                        if let Some(rel_name) = line[start..end].find(NAME) {
                            if is_whole_word_at(line, start + rel_name) {
                                hit = true;
                            }
                        }
                    }
                    from = start;
                }
            }
        }
        if hit {
            holders.push(Holder {
                file: file.to_string(),
                line: i + 1,
                func: enclosing_fn(&lines, i),
                text: line.trim().to_string(),
            });
        }
    }
    holders
}

fn all_holders() -> Vec<Holder> {
    sim_sources()
        .iter()
        .flat_map(|(file, text)| control_frame_holders(file, text))
        .collect()
}

const FIX: &str = "\
A body/sim system must NOT hold the global `ControlFrame`: it is ONE player's
device frame, so reading it makes the system silently slot-0-only — possession,
co-op, and netcode slot routing (N1) all break quietly.

Read an ENTITY-LOCAL frame instead:
  * `PlayerInputFrame`  — the local player's slot frame, gated on brain ownership
                          (a vacated body sees neutral input, for free).
  * `ActorControl`      — the brain's per-body intent frame; every actor has one,
                          and the player brain passes its buttons through 1:1.
  * `SlotControls` / `SlotInteractionState` — when you genuinely need a SLOT's
                          input rather than a body's.

If this really is an input-layer bridge, add it to `ALLOWLIST` in this file with a
`Bridge` category and a reason. If it is a deliberate slot-0 gesture, say so with
`Bridge::Slot0Gesture` and describe the multiplayer consequence — that entry is
then a checklist item for N1, not a hiding place.";

/// The lint. Bidirectional: an unlisted holder fails, and so does a listed entry
/// that no longer matches any holder (which is precisely how B3's sentence rotted).
#[test]
fn control_frame_holders_match_the_allowlist() {
    let holders = all_holders();

    let mut unlisted = Vec::new();
    for h in &holders {
        let listed = ALLOWLIST
            .iter()
            .any(|(f, func, _, _)| *f == h.file && *func == h.func);
        if !listed {
            unlisted.push(format!(
                "  {}:{}: {} — in `{}`",
                h.file, h.line, h.text, h.func
            ));
        }
    }

    let mut stale = Vec::new();
    for (file, func, _, _) in ALLOWLIST {
        if !holders.iter().any(|h| h.file == *file && h.func == *func) {
            stale.push(format!(
                "  {file}: `{func}` is allowlisted but holds no `ControlFrame` \
                 (renamed, moved, or already converted?)"
            ));
        }
    }

    assert!(
        unlisted.is_empty(),
        "\nControlFrame allowlist lint — {} unlisted holder(s).\n\n{FIX}\n\nHolders:\n{}\n",
        unlisted.len(),
        unlisted.join("\n"),
    );
    assert!(
        stale.is_empty(),
        "\nControlFrame allowlist lint — {} STALE allowlist entry/entries.\n\n\
         An entry that matches nothing is how this invariant rotted the first time: \
         `unified-actors.md` B3 named `sync_local_player_input_frame` as a holder \
         long after it had stopped being one. Delete the entry, or fix the name.\n\n{}\n",
        stale.len(),
        stale.join("\n"),
    );
}

/// Every allowlist entry carries a real justification — the reason field is what
/// makes the list reviewable rather than a rubber stamp.
#[test]
fn every_allowlist_entry_is_justified() {
    for (file, func, bridge, why) in ALLOWLIST {
        assert!(
            why.len() > 40,
            "{file}::{func} ({bridge:?}) needs a real reason, not `{why}`"
        );
    }
    // The one category that is a bug-in-waiting must say so, by name.
    for (file, func, bridge, why) in ALLOWLIST {
        if *bridge == Bridge::Slot0Gesture {
            assert!(
                why.contains("MULTIPLAYER TODO"),
                "{file}::{func} is a Slot0Gesture; its reason must open with \
                 `MULTIPLAYER TODO` so N1 can grep for it"
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// POISON TESTS — a grep lint that cannot fail is worse than no lint.
//
// The determinism pass proved this: its four properties were measured
// "already true", and poison-testing them turned up a real `HashSet<Entity>`
// iteration on the hottest combat path. So: exercise the scanner on synthetic
// sources that DO violate, and confirm it sees them.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn the_lint_catches_an_injected_sim_reader() {
    let poison = r#"
use bevy::prelude::*;
pub fn step_bodies(frame: Res<ControlFrame>, mut bodies: Query<&mut BodyKinematics>) {
    for mut b in &mut bodies { b.vel.x = frame.axis_x; }
}
"#;
    let found = control_frame_holders(
        "crates/ambition_actors/src/features/ecs/actors/update.rs",
        poison,
    );
    assert_eq!(
        found.len(),
        1,
        "the injected sim reader must be seen: {found:?}"
    );
    assert_eq!(found[0].func, "step_bodies");
    assert_eq!(found[0].line, 3);
}

/// The fifth holder hid behind an import path (`Res<ambition_input::ControlFrame>`),
/// which is exactly why `unified-actors.md` B3's count said four. Any path must hit.
#[test]
fn the_lint_catches_a_reader_written_through_any_import_path() {
    for ty in [
        "ControlFrame",
        "ambition_input::ControlFrame",
        "ambition_engine_core::ControlFrame",
        "ae::ControlFrame",
    ] {
        let poison = format!("pub fn sim_thing(c: Res<{ty}>) {{}}\n");
        let found = control_frame_holders("crates/ambition_actors/src/x.rs", &poison);
        assert_eq!(found.len(), 1, "`Res<{ty}>` must be caught");
    }
    // ...and through `ResMut`, `Option<Res<..>>`, a lifetime, and direct World access.
    for src in [
        "pub fn w(c: ResMut<ControlFrame>) {}",
        "pub fn o(c: Option<Res<ControlFrame>>) {}",
        "pub fn l(c: Res<'w, ControlFrame>) {}",
        "pub fn d(world: &mut World) { let _ = world.resource_mut::<ControlFrame>(); }",
    ] {
        let found = control_frame_holders("crates/ambition_actors/src/x.rs", src);
        assert_eq!(found.len(), 1, "must be caught: {src}");
    }
}

/// ...and it must NOT fire on the near-misses, or it will be disabled within a week.
#[test]
fn the_lint_ignores_near_misses() {
    for src in [
        // SUFFIX collisions: three real types start with `ControlFrame`.
        "pub fn a(l: ResMut<ControlFrameLatch>) {}",
        "pub fn b(m: Res<ControlFrameModes>) {}",
        "pub fn c(t: Res<ControlFrameTrace>) {}",
        // PREFIX collision: the MENU's frame is a different resource. The first
        // draft of this scanner flagged both real menu systems.
        "pub fn m(f: ResMut<MenuControlFrame>) {}",
        "pub fn n(f: Res<MenuControlFrame>) {}",
        // Registration is not a read — and `init_resource::<` contains
        // `resource::<`, which the first draft happily matched.
        "fn build(app: &mut App) { app.init_resource::<ControlFrame>(); }",
        "fn b2(app: &mut App) { app.init_resource::<ambition_input::ControlFrame>(); }",
        // The entity-local frames are the whole point; they must never trip it.
        "pub fn body(f: Res<SlotControls>, q: Query<&PlayerInputFrame>) {}",
        "pub fn actor(q: Query<&ActorControl>) {}",
        // A doc comment naming the type.
        "/// Reads `Res<ControlFrame>` — but it doesn't, this is prose.",
    ] {
        let found = control_frame_holders("crates/ambition_actors/src/x.rs", src);
        assert!(found.is_empty(), "false positive on: {src} -> {found:?}");
    }
}

/// `#[cfg(test)]` modules seed the frame to drive the system under test. That is
/// not a sim read, and the lint must not count it — `possession.rs` has exactly
/// such a helper, one screen below the real holder.
#[test]
fn the_lint_skips_cfg_test_modules() {
    let src = r#"
pub fn real_bridge(mut f: ResMut<ControlFrame>) {}

#[cfg(test)]
mod tests {
    use super::*;
    fn hold(app: &mut App) {
        let mut c = app.world_mut().resource_mut::<ControlFrame>();
        c.axis_y = 1.0;
    }
    fn seed(f: Res<ControlFrame>) {}
}
"#;
    let found = control_frame_holders("crates/ambition_actors/src/x.rs", src);
    assert_eq!(found.len(), 1, "only the non-test bridge counts: {found:?}");
    assert_eq!(found[0].func, "real_bridge");
}

/// The escape hatch works, and a reviewed line drops out of the holder set.
#[test]
fn the_review_marker_suppresses_a_holder() {
    let src = "\
// AMBITION_REVIEW(control_frame): explained here.
pub fn odd_one(f: Res<ControlFrame>) {}
";
    assert!(control_frame_holders("crates/ambition_actors/src/x.rs", src).is_empty());
}

// ─────────────────────────────────────────────────────────────────────────────
// The escape hatch is itself auditable.
// ─────────────────────────────────────────────────────────────────────────────

/// Not a lint — an inventory. Prints the reviewed exceptions AND every
/// `Slot0Gesture` allowlist entry, because those are the multiplayer TODOs N1
/// must work through.
#[test]
fn reviewed_control_frame_exceptions_are_listed() {
    let mut found = Vec::new();
    for (file, text) in sim_sources() {
        for (i, line) in text.lines().enumerate() {
            if line.contains("AMBITION_REVIEW(control_frame)") {
                found.push(format!("  {file}:{}: {}", i + 1, line.trim()));
            }
        }
    }
    println!(
        "AMBITION_REVIEW(control_frame) exceptions ({}):\n{}",
        found.len(),
        found.join("\n")
    );

    let todos: Vec<String> = ALLOWLIST
        .iter()
        .filter(|(_, _, b, _)| *b == Bridge::Slot0Gesture)
        .map(|(f, func, _, _)| format!("  {f}::{func}"))
        .collect();
    println!(
        "\nSlot0Gesture allowlist entries — the N1 multiplayer checklist ({}):\n{}",
        todos.len(),
        todos.join("\n")
    );
}
