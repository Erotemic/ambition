//! Canonical Player Robot slash presentation overlay.
//!
//! Generic slash cue/VFX vocabulary remains in `ambition_characters`; this module
//! owns the robot-specific cue ids and their compile-time pins to `ambition_sfx`.
//! The overlay is applied only by protagonist construction, not generic character
//! preparation.

use ambition_characters::moveset_prefabs::{SLASH_ARC_VFX, SLASH_POKE_VFX, SWING_SFX_CUE};
use ambition_characters::technique::{set_pogo_sfx, POGO_BOUNCE_KEY};
use ambition_entity_catalog::{MoveEventKind, MovesetContract, ATTACK_VERB};

/// Dry blade-through-air cue reserved for the canonical robot protagonist.
pub const PLAYER_ROBOT_SWING_SFX_CUE: &str = "player.robot.slash.air";
/// Material selector carried by the canonical robot protagonist's slash volume.
pub const PLAYER_ROBOT_IMPACT_SFX_CUE: &str = "player.robot.slash.impact";
/// Rebound cue the canonical robot protagonist's down-air pogo authors onto its
/// `pogo_bounce` effect. Every other body leaves it unauthored and keeps the
/// engine's generic pogo cue.
pub const PLAYER_ROBOT_POGO_SFX_CUE: &str = "player.robot.slash.impact.pogo";

/// Retarget the engine-default blade presentation to the canonical robot
/// protagonist's private SFX family. This is an explicit post-build overlay:
/// generic actors keep `player.slash` (or their authored cues), while the robot
/// player gets a dry air swing, a material selector on slash volumes, and its
/// own rebound cue on the down-air's pogo.
///
/// It writes into the DATA, and only where the data said nothing — so every
/// consumer downstream (the strike resolver, the pogo technique) reads one
/// authored moveset and never has to ask which character is swinging.
pub fn apply_player_robot_slash_sfx(moveset: &mut MovesetContract) {
    for move_spec in &mut moveset.moves {
        if !move_spec.id.starts_with(ATTACK_VERB) {
            continue;
        }
        for event in &mut move_spec.events {
            if let MoveEventKind::Sfx { cue } = &mut event.kind {
                if cue == SWING_SFX_CUE {
                    *cue = PLAYER_ROBOT_SWING_SFX_CUE.to_string();
                }
            }
        }
        for window in &mut move_spec.windows {
            for volume in &mut window.volumes {
                let is_slash = matches!(
                    volume.vfx.as_deref(),
                    Some(SLASH_ARC_VFX) | Some(SLASH_POKE_VFX)
                );
                if is_slash && volume.hit_sfx.is_none() {
                    volume.hit_sfx = Some(PLAYER_ROBOT_IMPACT_SFX_CUE.to_string());
                }
                if let Some(effect) = volume.on_hit.as_mut().filter(|e| e.key == POGO_BOUNCE_KEY) {
                    set_pogo_sfx(effect, PLAYER_ROBOT_POGO_SFX_CUE);
                }
            }
        }
    }
}
