//! THE E4 BOUNDARY TEST: render never names a live sim-STATE component.
//!
//! The observation boundary (docs/planning/engine/decomposition.md, E4)
//! says presentation reads ONLY the `ambition_sim_view` read-model — never
//! the sim heart's live `Body*` clusters, actor/boss cluster refs, control
//! seams, or item/ability sim components. This test greps this crate's own
//! sources for those type names in CODE (comments and doc lines are
//! stripped), so the boundary survives every future edit: reintroducing a
//! `Query<&BodyKinematics>` in render fails CI here, with the offending
//! file:line in the message.
//!
//! Vocabulary/data types (`FeatureView` rows, `CharacterAnim`,
//! `ProjectileVisualKind`, `Health` as plain data, asset registries) are
//! deliberately NOT forbidden — the boundary is about live sim STATE, not
//! about naming shared data shapes.

use std::path::{Path, PathBuf};

/// Live sim-state types render must never name in code. Each entry is
/// matched as a whole identifier (so `GroundItemVisual` does not trip
/// `GroundItem`).
const FORBIDDEN_SIM_STATE: &[&str] = &[
    // kinematics + the Body* cluster set
    "BodyKinematics",
    "BodyGroundState",
    "BodyWallState",
    "BodyBlinkState",
    "BodyFlightState",
    "BodyDashState",
    "BodyLedgeState",
    "BodyModeState",
    "BodyEnvironmentContact",
    "BodyAbilities",
    "BodyDodgeState",
    "BodyShieldState",
    "BodyCombat",
    "BodyHealth",
    "BodyMana",
    "BodyWallet",
    "BodyMelee",
    "BodyAnimFacts",
    "BodyBaseSize",
    // control / identity seams
    "ControlledSubject",
    "ActorControl",
    "SlotControls",
    "PlayerBlinkCameraState",
    // actor/boss cluster views
    "ActorSpriteData",
    "BossClusterRef",
    "BossAttackState",
    "BossPhase",
    "ActorDisposition",
    "ActorIdentity",
    "ActorStatus",
    "ActorConfig",
    "ActorRoll",
    "MeleeSwing",
    "CenteredAabb",
    "FeatureSimEntity",
    "FeatureName",
    // item / ability / projectile sim state
    "HeldItem",
    "GroundItem",
    "HeldProjectile",
    "PlayerMark",
    "HealShrine",
    "GravityFlipSwitch",
    "LiveProjectile",
    "PlayerProjectileState",
];

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("readable src dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            rust_sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Strip line comments (`// …`) and doc comments so prose mentioning a sim
/// type doesn't trip the check. String literals are rare in render and none
/// carry these names, so no string-aware parsing is needed.
fn code_only(line: &str) -> &str {
    match line.find("//") {
        Some(idx) => &line[..idx],
        None => line,
    }
}

fn is_ident_char(c: u8) -> bool {
    c == b'_' || c.is_ascii_alphanumeric()
}

/// Whole-identifier containment: `needle` appears in `hay` with non-ident
/// (or edge) characters on both sides.
fn contains_ident(hay: &str, needle: &str) -> bool {
    let bytes = hay.as_bytes();
    let mut start = 0;
    while let Some(pos) = hay[start..].find(needle) {
        let at = start + pos;
        let end = at + needle.len();
        let left_ok = at == 0 || !is_ident_char(bytes[at - 1]);
        let right_ok = end == bytes.len() || !is_ident_char(bytes[end]);
        if left_ok && right_ok {
            return true;
        }
        start = at + 1;
    }
    false
}

#[test]
fn render_never_names_live_sim_state() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_sources(&src, &mut files);
    assert!(!files.is_empty(), "found no render sources under {src:?}");

    let mut violations = Vec::new();
    for file in &files {
        let text = std::fs::read_to_string(file).expect("readable source");
        for (i, line) in text.lines().enumerate() {
            let code = code_only(line);
            for forbidden in FORBIDDEN_SIM_STATE {
                if contains_ident(code, forbidden) {
                    violations.push(format!(
                        "{}:{}: names sim-state `{}`: {}",
                        file.display(),
                        i + 1,
                        forbidden,
                        line.trim()
                    ));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "render must consume ambition_sim_view, never live sim state \
         (E4 observation boundary). Violations:\n{}",
        violations.join("\n")
    );
}
