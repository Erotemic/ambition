//! **The `ControlFrame` allowlist lint** — custom scanner (unified-actors step 5,
//! the R6c player-fold guardrail).
//!
//! Invariant: the global `ControlFrame` is ONE player's device frame; only the
//! INPUT LAYER may hold it. A body/sim system that reads it is silently
//! slot-0-only. The holder detection (`Res<…ControlFrame>` / `ResMut` / direct
//! `World` access, enclosing-fn attribution, whole-word matching, `#[cfg(test)]`
//! skipping) is genuinely semantic and stays Rust; the CONFIG (scoped roots,
//! excluded subpaths, review marker, the justified holder allowlist) is data in
//! `policies/control_frame.toml`.
//!
//! Bidirectional: an unlisted holder fails AND a stale allowlist entry fails.
//! Scoped: engine (crates/*) and game (ambition_content) run independently off
//! this one scanner.

use serde::Deserialize;

use crate::model::{Diagnostic, Report, Scope};
use crate::workspace::{self, Workspace};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    source_doc: String,
    rationale: String,
    review_marker: String,
    non_sim_subpaths: Vec<String>,
    #[serde(default)]
    root: Vec<Root>,
    #[serde(default)]
    allow: Vec<Allow>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Root {
    path: String,
    scope: Scope,
}

/// How a holder is allowed to touch the global frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Bridge {
    /// DEVICE → frame (samples a physical controller).
    DeviceToFrame,
    /// frame ↔ tick LATCH.
    Latch,
    /// frame → SLOT vocabulary every body reads.
    FrameToSlot,
    /// frame ↔ an adapter's intent struct, in the input phase.
    IntentBridge,
    /// A SIM system reading the device frame, slot-0-only by design — a named
    /// multiplayer TODO.
    Slot0Gesture,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Allow {
    file: String,
    func: String,
    bridge: Bridge,
    why: String,
}

impl Allow {
    fn scope(&self) -> Scope {
        if self.file.starts_with("game/") {
            Scope::Game
        } else {
            Scope::Engine
        }
    }
    /// The owning crate, derived from the first two path segments.
    fn owner(&self) -> String {
        self.file.split('/').take(2).collect::<Vec<_>>().join("/")
    }
}

fn load_config() -> Config {
    let path = workspace::policies_dir().join("control_frame.toml");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    toml::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

fn is_comment(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("//") || t.starts_with('*') || t.starts_with("#![") || t.starts_with("#[")
}

fn is_reviewed(marker: &str, lines: &[&str], idx: usize) -> bool {
    if lines[idx].contains(marker) {
        return true;
    }
    for line in lines[..idx].iter().rev() {
        if !is_comment(line) {
            return false;
        }
        if line.contains(marker) {
            return true;
        }
    }
    false
}

/// `ControlFrame` occurs at `idx` as a WHOLE type name (not `ControlFrameLatch`,
/// not `MenuControlFrame`; a leading `::` import path is fine).
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct Holder {
    file: String,
    line: usize,
    func: String,
    text: String,
}

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
/// and reviewed lines. Pure `(file, text)` function so poison tests can drive it.
fn control_frame_holders(marker: &str, file: &str, text: &str) -> Vec<Holder> {
    const NAME: &str = "ControlFrame";
    let lines: Vec<&str> = text.lines().collect();
    let mut holders = Vec::new();

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
        if is_comment(line) || is_reviewed(marker, &lines, i) {
            continue;
        }

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

/// Every non-test `.rs` under the scope's roots, minus non-sim subpaths.
fn sim_sources(ws: &Workspace, cfg: &Config, scope: Scope) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for root in cfg.root.iter().filter(|r| r.scope == scope) {
        let src = ws.abs(&root.path).join("src");
        let mut contributed = 0usize;
        for path in workspace::rust_sources_under(&src) {
            let rel = path
                .strip_prefix(&src)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            if workspace::is_test_path(&rel) {
                continue;
            }
            if cfg.non_sim_subpaths.iter().any(|skip| rel.contains(skip)) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            out.push((format!("{}/src/{rel}", root.path), text));
            contributed += 1;
        }
        assert!(
            contributed > 0,
            "ControlFrame scan reached no sources under `{}` ({} scope) — vacuous",
            root.path,
            scope.label()
        );
    }
    out
}

/// The lint for one scope. Bidirectional: unlisted holders AND stale allowlist
/// entries both fail.
pub fn run(ws: &Workspace, scope: Scope, report: &mut Report) {
    let cfg = load_config();
    let roots: Vec<&Root> = cfg.root.iter().filter(|r| r.scope == scope).collect();
    assert!(
        !roots.is_empty(),
        "no ControlFrame roots for the {} scope",
        scope.label()
    );

    let holders: Vec<Holder> = sim_sources(ws, &cfg, scope)
        .iter()
        .flat_map(|(file, text)| control_frame_holders(&cfg.review_marker, file, text))
        .collect();
    let scope_allow: Vec<&Allow> = cfg.allow.iter().filter(|a| a.scope() == scope).collect();

    let mk = |location: String, owner: String, detail: String| Diagnostic {
        policy_id: format!("{}.control-frame", scope.label()),
        owners: vec![owner],
        source_doc: cfg.source_doc.clone(),
        rationale: cfg.rationale.clone(),
        location,
        detail,
    };

    // Unlisted holders.
    for h in &holders {
        let listed = scope_allow
            .iter()
            .any(|a| a.file == h.file && a.func == h.func);
        if !listed {
            let owner = h.file.split('/').take(2).collect::<Vec<_>>().join("/");
            report.push(mk(
                format!("{}:{}", h.file, h.line),
                owner,
                format!(
                    "fn `{}` holds the global ControlFrame but is not an allowlisted input-layer \
                     bridge — read PlayerInputFrame / ActorControl / SlotControls instead, or add a \
                     justified allowlist entry (review with `{}`)",
                    h.func, cfg.review_marker
                ),
            ));
        }
    }

    // Stale allowlist entries (how B3's holder list rotted).
    for a in &scope_allow {
        if !holders.iter().any(|h| h.file == a.file && h.func == a.func) {
            report.push(mk(
                format!("{}::{}", a.file, a.func),
                a.owner(),
                "allowlisted but holds no ControlFrame (renamed, moved, or already converted?) — \
                 delete the entry or fix the name"
                    .to_string(),
            ));
        }
    }
}

// ── poison / self-tests (called from tests/policy.rs) ────────────────────────

/// Config sanity: every allowlist entry is justified, and a Slot0Gesture entry
/// names the multiplayer consequence.
pub fn allowlist_is_justified() {
    let cfg = load_config();
    for a in &cfg.allow {
        assert!(
            a.why.len() > 40,
            "{}::{} ({:?}) needs a real reason",
            a.file,
            a.func,
            a.bridge
        );
        if a.bridge == Bridge::Slot0Gesture {
            assert!(
                a.why.contains("MULTIPLAYER TODO"),
                "{}::{} is a Slot0Gesture; its reason must open with MULTIPLAYER TODO so N1 can grep it",
                a.file,
                a.func
            );
        }
    }
}

/// The scanner catches injected readers (any import path, ResMut, Option, World
/// access), ignores the near-misses, skips `#[cfg(test)]`, and honours the marker.
pub fn poison_self_tests() {
    let m = "AMBITION_REVIEW(control_frame)";
    let holders =
        |_marker: &str, t: &str| control_frame_holders(m, "crates/ambition_actors/src/x.rs", t);

    // An injected sim reader is seen, attributed to its fn.
    let found = control_frame_holders(
        m,
        "crates/ambition_actors/src/features/ecs/actors/update.rs",
        "use bevy::prelude::*;\npub fn step_bodies(frame: Res<ControlFrame>, mut q: Query<&mut BodyKinematics>) {\n    for mut b in &mut q { b.vel.x = frame.axis_x; }\n}\n",
    );
    assert_eq!(
        found.len(),
        1,
        "injected sim reader must be seen: {found:?}"
    );
    assert_eq!(found[0].func, "step_bodies");

    // Any import path, ResMut, Option, lifetime, direct World access — all caught.
    for ty in [
        "ControlFrame",
        "ambition_input::ControlFrame",
        "ambition_engine_core::ControlFrame",
        "ae::ControlFrame",
    ] {
        assert_eq!(
            holders("_", &format!("pub fn sim_thing(c: Res<{ty}>) {{}}\n")).len(),
            1,
            "`Res<{ty}>` must be caught"
        );
    }
    for src in [
        "pub fn w(c: ResMut<ControlFrame>) {}",
        "pub fn o(c: Option<Res<ControlFrame>>) {}",
        "pub fn l(c: Res<'w, ControlFrame>) {}",
        "pub fn d(world: &mut World) { let _ = world.resource_mut::<ControlFrame>(); }",
    ] {
        assert_eq!(holders("_", src).len(), 1, "must be caught: {src}");
    }

    // Near-misses must NOT fire.
    for src in [
        "pub fn a(l: ResMut<ControlFrameLatch>) {}",
        "pub fn b(m: Res<ControlFrameModes>) {}",
        "pub fn c(t: Res<ControlFrameTrace>) {}",
        "pub fn m(f: ResMut<MenuControlFrame>) {}",
        "pub fn n(f: Res<MenuControlFrame>) {}",
        "fn build(app: &mut App) { app.init_resource::<ControlFrame>(); }",
        "fn b2(app: &mut App) { app.init_resource::<ambition_input::ControlFrame>(); }",
        "pub fn body(f: Res<SlotControls>, q: Query<&PlayerInputFrame>) {}",
        "pub fn actor(q: Query<&ActorControl>) {}",
        "/// Reads `Res<ControlFrame>` — but it doesn't, this is prose.",
    ] {
        assert!(holders("_", src).is_empty(), "false positive on: {src}");
    }

    // `#[cfg(test)]` blocks are skipped.
    let cfg_test = "pub fn real_bridge(mut f: ResMut<ControlFrame>) {}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n    fn hold(app: &mut App) { let mut c = app.world_mut().resource_mut::<ControlFrame>(); }\n    fn seed(f: Res<ControlFrame>) {}\n}\n";
    let found = holders(m, cfg_test);
    assert_eq!(found.len(), 1, "only the non-test bridge counts: {found:?}");
    assert_eq!(found[0].func, "real_bridge");

    // The review marker suppresses a holder.
    let reviewed = format!("// {m}: explained here.\npub fn odd_one(f: Res<ControlFrame>) {{}}\n");
    assert!(holders(m, &reviewed).is_empty());
}
