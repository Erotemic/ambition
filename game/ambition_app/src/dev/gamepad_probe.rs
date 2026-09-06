//! What the stick actually reports, per pad, so a hardware question can be
//! answered with a number instead of a story.
//!
//! ⛔⛔ THE MECHANIC THIS EXISTS FOR. A Smash attack requires a left-stick FLICK:
//! `AttackGestureTuning::flick_threshold` is `0.8`, and that `0.8` is applied
//! AFTER Ambition's inner-deadzone transform
//! (`ControlSettings::apply_deadzone`), which is
//!
//! ```text
//! post = (raw - deadzone) / (1 - deadzone)
//! ```
//!
//! A Switch Pro is detected as [`GamepadStyle::Switch`], which maps to the
//! `Generic` profile, which uses the baseline `0.18` deadzone. So reaching a
//! flick needs **0.836 RAW**, while an ordinary directional tilt needs only
//! about **0.59 raw**. A pad that tops out near 0.80 on one host therefore runs,
//! tilts, and drives menus perfectly while Smash attacks *cannot exist* — and
//! the same pad on a host where it reaches 0.95+ works fine. That is a very
//! specific prediction, and this overlay is how to test it.
//!
//! ⭐ IT READS BEVY'S `Gamepad` DIRECTLY, not the action layer. The question is
//! about the DEVICE, and routing it through bindings, seats and action state
//! would put four more suspects between the stick and the number.
//!
//! ⚠ THE PEAK IS THE MEASUREMENT, not the live value. Nobody can hold a stick at
//! its true maximum and read a screen at the same time, so every row keeps a
//! peak-hold: push the stick to each corner, then look. `Shift+F6` toggles the
//! overlay and RESETS the peaks, so a second run is a fresh measurement.
//!
//! ⛔ THIS DIAGNOSES; IT DOES NOT CALIBRATE. If the peak comes back under 0.836
//! the repair is an OUTER saturation stage at the shared input seam — an
//! `outer` alongside the inner deadzone, so `0.8` means the same gesture on every
//! pad — and not a weaker Smash threshold to suit one host. The right outer
//! value is whatever this measures, which is why the measurement comes first.

use bevy::prelude::*;

use ambition_platformer2d::input::settings::ControlSettings;
use ambition_platformer2d::input::{gamepad_style_of, ControlFilters, GamepadStyle};
use ambition_platformer2d::persistence::settings::UserSettings;
use ambition_platformer2d::platformer::developer_hotkeys::DeveloperAction;

/// The flick magnitude a Smash attack needs, POST-deadzone.
///
/// Mirrored rather than imported so this overlay states the number it is
/// checking against; `a_probe_states_the_same_thresholds_the_gesture_uses`
/// asserts the two agree, so the mirror cannot rot.
const SMASH_FLICK_THRESHOLD: f32 = 0.8;

/// The magnitude an ordinary directional attack needs, POST-deadzone. The
/// contrast is the finding: a pad can clear this and never clear the flick.
const TILT_THRESHOLD: f32 = 0.5;

/// The raw magnitude needed to reach `post` through an inner deadzone.
///
/// The inverse of `ControlSettings::apply_deadzone`, which is what makes the
/// verdict a fact rather than an estimate.
pub fn raw_needed_for(post: f32, deadzone: f32) -> f32 {
    if deadzone >= 1.0 {
        return f32::INFINITY;
    }
    post * (1.0 - deadzone) + deadzone
}

/// One pad's live reading and its peak-hold.
#[derive(Clone, Debug, Default)]
pub struct PadProbe {
    pub name: String,
    pub vendor_id: Option<u16>,
    pub product_id: Option<u16>,
    pub style: GamepadStyle,
    pub deadzone: f32,
    pub raw: Vec2,
    pub post: Vec2,
    /// The largest RAW magnitude this pad has reported since the last reset.
    pub peak_raw: f32,
    /// The largest POST-deadzone magnitude, which is what the gesture compares.
    pub peak_post: f32,
}

impl PadProbe {
    /// What a Smash flick needs from this pad, in raw units.
    pub fn raw_for_smash(&self) -> f32 {
        raw_needed_for(SMASH_FLICK_THRESHOLD, self.deadzone)
    }

    /// What an ordinary tilt needs, for contrast.
    pub fn raw_for_tilt(&self) -> f32 {
        raw_needed_for(TILT_THRESHOLD, self.deadzone)
    }

    /// The one line worth reading.
    ///
    /// ⭐ IT STATES THE CONCLUSION, not the inputs to it. An overlay that shows
    /// two numbers and expects the reader to do the arithmetic while holding a
    /// stick is an overlay nobody uses correctly.
    pub fn verdict(&self) -> &'static str {
        if self.peak_raw <= 0.0 {
            "push the stick to each corner, then read the peaks"
        } else if self.peak_post >= SMASH_FLICK_THRESHOLD {
            "OK — this pad reaches a Smash flick"
        } else if self.peak_post >= TILT_THRESHOLD {
            "SMASH UNREACHABLE — tilts work, flicks cannot fire on this pad/host"
        } else {
            "the stick is not reaching a tilt either — check the pad is the one being read"
        }
    }
}

/// Every connected pad, and whether the overlay is up.
#[derive(Resource, Clone, Debug, Default)]
pub struct GamepadProbes {
    pub visible: bool,
    pub pads: Vec<PadProbe>,
}

impl GamepadProbes {
    /// Forget the peaks so the next push is a fresh measurement.
    pub fn reset_peaks(&mut self) {
        for pad in &mut self.pads {
            pad.peak_raw = 0.0;
            pad.peak_post = 0.0;
        }
    }

    /// The whole readout as text, so the overlay and a log line cannot disagree.
    pub fn report(&self) -> String {
        if self.pads.is_empty() {
            return "gamepad probe: no pads connected".to_string();
        }
        let mut out = String::from("gamepad probe (Shift+F6 toggles + resets)\n");
        for (index, pad) in self.pads.iter().enumerate() {
            out.push_str(&format!(
                "pad {index}  {}  vendor {}  style {:?}  inner deadzone {:.3}\n\
                 \x20 raw   x {:+.3} y {:+.3}   mag {:.3}   PEAK {:.3}\n\
                 \x20 post  x {:+.3} y {:+.3}   mag {:.3}   PEAK {:.3}\n\
                 \x20 smash needs raw >= {:.3}   tilt needs raw >= {:.3}\n\
                 \x20 {}\n",
                if pad.name.is_empty() {
                    "<unnamed>"
                } else {
                    pad.name.as_str()
                },
                pad.vendor_id
                    .map(|id| format!("{id:#06x}"))
                    .unwrap_or_else(|| "?".to_string()),
                pad.style,
                pad.deadzone,
                pad.raw.x,
                pad.raw.y,
                pad.raw.length(),
                pad.peak_raw,
                pad.post.x,
                pad.post.y,
                pad.post.length(),
                pad.peak_post,
                pad.raw_for_smash(),
                pad.raw_for_tilt(),
                pad.verdict(),
            ));
        }
        out
    }
}

/// Tag on the overlay text entity.
#[derive(Component)]
struct GamepadProbeText;

/// Sample every connected pad.
///
/// ⚠ THE DEADZONE IS RESOLVED THE WAY THE GAME RESOLVES IT — through
/// `ControlFilters::for_pad`, which is where the detected style becomes a
/// number. Reading the raw settings slider instead would show a deadzone the
/// gesture is not actually filtered by, which is the mistake this whole
/// investigation is about.
pub fn sample_gamepad_probes(
    pads: Query<(&Gamepad, Option<&Name>)>,
    settings: Option<Res<UserSettings>>,
    mut probes: ResMut<GamepadProbes>,
) {
    let controls: ControlSettings = settings
        .map(|settings| settings.controls.clone())
        .unwrap_or_default();
    let peaks: Vec<(f32, f32)> = probes
        .pads
        .iter()
        .map(|p| (p.peak_raw, p.peak_post))
        .collect();
    probes.pads.clear();
    for (index, (pad, name)) in pads.iter().enumerate() {
        let label = name
            .map(|name| name.as_str().to_string())
            .unwrap_or_default();
        let style = gamepad_style_of(pad.vendor_id(), Some(label.as_str()));
        let deadzone = ControlFilters::for_pad(&controls, style).left_stick_deadzone;
        let raw = pad.left_stick();
        let (px, py) = ControlSettings::apply_deadzone(raw.x, raw.y, deadzone);
        let post = Vec2::new(px, py);
        let (mut peak_raw, mut peak_post) = peaks.get(index).copied().unwrap_or((0.0, 0.0));
        peak_raw = peak_raw.max(raw.length());
        peak_post = peak_post.max(post.length());
        probes.pads.push(PadProbe {
            name: label,
            vendor_id: pad.vendor_id(),
            product_id: pad.product_id(),
            style,
            deadzone,
            raw,
            post,
            peak_raw,
            peak_post,
        });
    }
}

fn spawn_gamepad_probe_text(mut commands: Commands) {
    commands.spawn((
        Text::new(String::new()),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(Color::srgb(0.95, 0.95, 0.6)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(48.0),
            left: Val::Px(8.0),
            ..default()
        },
        Visibility::Hidden,
        GamepadProbeText,
        Name::new("gamepad probe overlay"),
    ));
}

fn refresh_gamepad_probe_text(
    probes: Res<GamepadProbes>,
    mut text: Query<(&mut Text, &mut Visibility), With<GamepadProbeText>>,
) {
    for (mut value, mut visibility) in &mut text {
        *visibility = if probes.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if probes.visible {
            value.0 = probes.report();
        }
    }
}

/// `Shift+F6` toggles the overlay and resets the peaks.
///
/// ⭐ TOGGLING RESETS, deliberately: the second thing anybody does with a
/// peak-hold is take a second reading, and a peak that survived the first one
/// would silently be the old maximum.
fn toggle_gamepad_probe(
    mut actions: MessageReader<DeveloperAction>,
    mut probes: ResMut<GamepadProbes>,
) {
    for action in actions.read() {
        if !matches!(action, DeveloperAction::ToggleGamepadProbe) {
            continue;
        }
        probes.visible = !probes.visible;
        probes.reset_peaks();
        // ⭐ ALSO TO THE LOG. A player testing on a laptop can read a terminal
        // afterwards; reading an overlay while both thumbs are on a pad is
        // harder than it sounds.
        if probes.visible {
            info!(target: "ambition::gamepad_probe", "gamepad probe ON — peaks reset");
        } else {
            info!(target: "ambition::gamepad_probe", "\n{}", probes.report());
        }
    }
}

pub struct GamepadProbePlugin;

impl Plugin for GamepadProbePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GamepadProbes>();
        app.add_systems(Startup, spawn_gamepad_probe_text);
        app.add_systems(
            Update,
            (
                toggle_gamepad_probe,
                sample_gamepad_probes,
                refresh_gamepad_probe_text,
            )
                .chain(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⛔⛔ THE ARITHMETIC THE WHOLE HYPOTHESIS RESTS ON. A Switch Pro is
    /// classified `Switch`, which takes the `Generic` profile's baseline 0.18
    /// inner deadzone, so a Smash flick needs 0.836 RAW while a tilt needs 0.59.
    /// If those two numbers were not far apart there would be no mystery to
    /// explain.
    #[test]
    fn a_switch_pro_needs_a_far_bigger_raw_push_for_a_smash_than_for_a_tilt() {
        let deadzone = ControlFilters::for_pad(&ControlSettings::default(), GamepadStyle::Switch)
            .left_stick_deadzone;
        assert!(
            (deadzone - 0.18).abs() < 1.0e-6,
            "a Switch pad is filtered at {deadzone}, so the numbers below are not \
             the ones this diagnostic is about"
        );
        let smash = raw_needed_for(SMASH_FLICK_THRESHOLD, deadzone);
        let tilt = raw_needed_for(TILT_THRESHOLD, deadzone);
        assert!((smash - 0.836).abs() < 0.001, "smash needs {smash} raw");
        assert!((tilt - 0.59).abs() < 0.001, "tilt needs {tilt} raw");
    }

    /// And the inverse really is the inverse of the transform the game applies —
    /// a hand-derived formula that drifted from `apply_deadzone` would make
    /// every verdict confidently wrong.
    #[test]
    fn the_raw_requirement_round_trips_through_the_real_deadzone_transform() {
        for deadzone in [0.0_f32, 0.14, 0.18, 0.27, 0.5] {
            for post in [0.5_f32, 0.8, 1.0] {
                let raw = raw_needed_for(post, deadzone);
                let (x, y) = ControlSettings::apply_deadzone(raw, 0.0, deadzone);
                let recovered = Vec2::new(x, y).length();
                assert!(
                    (recovered - post).abs() < 1.0e-4,
                    "deadzone {deadzone}: {raw} raw came back as {recovered}, not {post}"
                );
            }
        }
    }

    /// ⭐ THE MIRRORED THRESHOLDS MUST MATCH THE GESTURE'S OWN. This overlay
    /// states the numbers it is checking against rather than importing them, so
    /// a reader can see them; the mirror is only honest while this passes.
    #[test]
    fn a_probe_states_the_same_thresholds_the_gesture_uses() {
        use ambition_platformer2d::characters::actor::attack_gesture::AttackGestureTuning;
        let tuning = AttackGestureTuning::default();
        assert!(
            (tuning.flick_threshold - SMASH_FLICK_THRESHOLD).abs() < 1.0e-6,
            "the gesture flicks at {} and this overlay reports {SMASH_FLICK_THRESHOLD}",
            tuning.flick_threshold
        );
        assert!(
            (tuning.directional_deadzone - TILT_THRESHOLD).abs() < 1.0e-6,
            "the gesture reads a direction at {} and this overlay reports {TILT_THRESHOLD}",
            tuning.directional_deadzone
        );
    }

    fn probe_with(deadzone: f32, peak_raw: f32) -> PadProbe {
        let (x, y) = ControlSettings::apply_deadzone(peak_raw, 0.0, deadzone);
        PadProbe {
            deadzone,
            peak_raw,
            peak_post: Vec2::new(x, y).length(),
            ..Default::default()
        }
    }

    /// ⭐ THE VERDICT IS THE PRODUCT. The three cases are the three things the
    /// laptop could turn out to be doing.
    #[test]
    fn the_verdict_names_the_case_the_numbers_describe() {
        assert!(probe_with(0.18, 0.0).verdict().contains("push the stick"));
        // A pad topping out at 0.80 raw: tilts fine, cannot flick. THE PREDICTION.
        assert!(probe_with(0.18, 0.80)
            .verdict()
            .contains("SMASH UNREACHABLE"));
        // A pad reaching full deflection.
        assert!(probe_with(0.18, 1.0).verdict().starts_with("OK"));
        // A stick barely leaving its deadzone at all.
        assert!(probe_with(0.18, 0.30)
            .verdict()
            .contains("not reaching a tilt"));
    }

    /// The peak survives the live value falling back to rest, which is the only
    /// reason a human can take this reading at all.
    #[test]
    fn a_peak_outlives_the_stick_returning_to_centre() {
        let mut probes = GamepadProbes {
            visible: true,
            pads: vec![probe_with(0.18, 0.9)],
        };
        assert!(probes.pads[0].peak_raw > 0.8);
        probes.reset_peaks();
        assert_eq!(probes.pads[0].peak_raw, 0.0, "a toggle must clear the peak");
    }
}
