//! Developer-facing logging and HUD summaries for the projectile
//! system. Pure formatting helpers — no Bevy systems live here.

use bevy::prelude::info;

/// One-line summary of the motion buffer plus what the recognizer
/// matched, emitted at INFO every time the player presses fire. The
/// goal is concrete feedback when "the Hadouken won't come out": the
/// player sees the actual sample sequence the recognizer saw, plus
/// the verdict from each gate (full QCF / half-circle / grace QCF /
/// none). If the printed sequence doesn't end in
/// `Down → Right` (or mirror), the player's input wasn't reaching
/// the recognizer and tuning the gate won't help — it's an input-
/// pipeline issue.
pub fn log_press_diagnostics(
    buffer: &crate::MotionInputBuffer,
    super_qcf: Option<f32>,
    half_circle: Option<f32>,
    grace_qcf: Option<f32>,
    motion_kind: Option<crate::ProjectileKind>,
) {
    // Compact recent-direction trail: at most the last 8 distinct
    // samples (collapse runs of the same direction so a long press
    // doesn't dominate the trail).
    let mut trail: Vec<&'static str> = Vec::with_capacity(8);
    let mut last_label: Option<&'static str> = None;
    for sample in buffer.samples.iter() {
        let label = motion_label(sample.dir);
        if Some(label) == last_label {
            continue;
        }
        trail.push(label);
        last_label = Some(label);
    }
    let trail_text = if trail.is_empty() {
        "(empty)".to_string()
    } else {
        // Keep only the last 8 entries so the line stays readable.
        let start = trail.len().saturating_sub(8);
        trail[start..].join(" → ")
    };
    let verdict = match motion_kind {
        Some(crate::ProjectileKind::HadoukenSuper) => "HadoukenSuper",
        Some(crate::ProjectileKind::Hadouken) => "Hadouken (grace)",
        Some(crate::ProjectileKind::Fireball) => "Fireball (motion)",
        None => "no motion → fireball charge",
    };
    info!(
        target: "ambition::projectile",
        "fire press · trail=[{trail_text}] · super_qcf={super_qcf:?} half_circle={half_circle:?} grace_qcf={grace_qcf:?} → {verdict}",
    );
}

pub fn motion_label(dir: crate::MotionDirection) -> &'static str {
    match dir {
        crate::MotionDirection::Neutral => "·",
        crate::MotionDirection::Up => "Up",
        crate::MotionDirection::Down => "Down",
        crate::MotionDirection::Left => "Left",
        crate::MotionDirection::Right => "Right",
        crate::MotionDirection::UpLeft => "UpLeft",
        crate::MotionDirection::UpRight => "UpRight",
        crate::MotionDirection::DownLeft => "DownLeft",
        crate::MotionDirection::DownRight => "DownRight",
    }
}
