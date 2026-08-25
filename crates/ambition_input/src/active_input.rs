//! Most recent genuine input device per seat and across the machine.
//!
//! [`SeatActiveDevices::for_seat`] drives seat-specific glyphs and prompts;
//! [`SeatActiveDevices::machine`] drives machine-wide behavior such as whether
//! mouse hover may reclaim menu focus. This is observation only: no input source
//! is disabled. Synthetic `Pointer<Over>` events do not count as genuine mouse
//! input; real cursor motion or button presses do.
//!
//! Keyboard/mouse belongs to its owning seat, gamepads to their associated
//! seats, and touch to the primary seat. Unassigned spare pads claim no seat.

use std::collections::BTreeMap;

use bevy::prelude::*;
// Only the input-enabled update system uses these types.
#[cfg(feature = "input")]
use bevy::input::gamepad::Gamepad;
#[cfg(feature = "input")]
use bevy_window::CursorMoved;

/// How a pad's vendor draws its buttons. Presentation only — WHICH button is
/// bound is [`crate::SeatBindings`]'s answer; this decides whether
/// `GamepadButton::South` is drawn "A" (Xbox), "Cross" (PlayStation) or "B"
/// (Switch mirrors the positions).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum GamepadStyle {
    /// Xbox 360 / One / Series and most XInput pads: A/B/X/Y, LB/RB/LT/RT.
    #[default]
    XboxLike,
    /// DualShock / DualSense: shape names, L1/R1/L2/R2.
    PlayStation,
    /// Switch Pro / Joy-Con: physical labels, mirrored positions.
    Switch,
    /// Unclassified. Drawn Xbox-style.
    Generic,
}

/// Classify a pad from its USB vendor id, falling back to its reported name.
///
/// The id is the strong signal — a DualSense reports Sony's `0x054c` whatever
/// a platform calls it — and the name substrings catch bluetooth stacks and
/// remappers that report no id. Anything unrecognised is [`GamepadStyle::Generic`],
/// which draws Xbox-style: wrong at worst in LABELS, never in behaviour.
pub fn gamepad_style_of(vendor_id: Option<u16>, name: Option<&str>) -> GamepadStyle {
    match vendor_id {
        Some(0x054c) => return GamepadStyle::PlayStation, // Sony
        Some(0x057e) => return GamepadStyle::Switch,      // Nintendo
        Some(0x045e) => return GamepadStyle::XboxLike,    // Microsoft
        _ => {}
    }
    let Some(name) = name else {
        return GamepadStyle::Generic;
    };
    let name = name.to_ascii_lowercase();
    if [
        "dualshock",
        "dualsense",
        "playstation",
        "sony",
        "ps4",
        "ps5",
    ]
    .iter()
    .any(|hint| name.contains(hint))
    {
        GamepadStyle::PlayStation
    } else if ["nintendo", "switch", "joy-con", "pro controller"]
        .iter()
        .any(|hint| name.contains(hint))
    {
        GamepadStyle::Switch
    } else if ["xbox", "x-box", "xinput", "microsoft"]
        .iter()
        .any(|hint| name.contains(hint))
    {
        GamepadStyle::XboxLike
    } else {
        GamepadStyle::Generic
    }
}

/// One seat's most recent genuine input device.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ActiveDevice {
    #[default]
    Keyboard,
    /// The mouse half of the keyboard-and-mouse bundle. Kept distinct from
    /// [`Self::Keyboard`] because the hover gates need exactly this
    /// distinction — but for glyphs a mouse click does not change which KEY a
    /// prompt should name, which is what [`Self::draws_keyboard_glyphs`]
    /// answers.
    Mouse,
    Gamepad(GamepadStyle),
    Touch,
}

impl ActiveDevice {
    /// Whether a prompt for this device shows keyboard glyphs. The mouse is
    /// half of the keyboard bundle: clicking does not move your other hand.
    pub fn draws_keyboard_glyphs(self) -> bool {
        matches!(self, Self::Keyboard | Self::Mouse)
    }

    /// How this device's gamepad buttons should be SPELLED.
    ///
    /// a device that is not a pad answers the default (Xbox-style), which is
    /// what a label has to say when nothing better is known — it is not a claim
    /// that anybody is holding an Xbox pad. Callers that need to know whether
    /// there is a pad at all ask [`Self::draws_keyboard_glyphs`]; a style alone
    /// cannot express "this seat is on a keyboard", which is exactly how a
    /// prompt came to print `Z` under a DualSense.
    pub fn gamepad_style(self) -> GamepadStyle {
        match self {
            Self::Gamepad(style) => style,
            _ => GamepadStyle::default(),
        }
    }
}

/// Every seat's active device, keyed by participant slot, plus which seat
/// spoke most recently — so the old global's semantics survive as the
/// [`Self::machine`] projection instead of as a second resource.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct SeatActiveDevices {
    seats: BTreeMap<u8, ActiveDevice>,
    newest: Option<u8>,
}

impl SeatActiveDevices {
    /// The seat's last genuine device. A seat that has never spoken reads as
    /// keyboard — the cold-start desktop answer, and what a fresh prompt
    /// should draw.
    pub fn for_seat(&self, slot: u8) -> ActiveDevice {
        self.seats.get(&slot).copied().unwrap_or_default()
    }

    /// The newest genuine device across every seat — the machine-level
    /// answer the mouse-hover gates ask. Idle frames keep the prior value;
    /// nothing ever fired means keyboard.
    pub fn machine(&self) -> ActiveDevice {
        self.newest
            .map(|slot| self.for_seat(slot))
            .unwrap_or_default()
    }

    /// Record genuine input from `device` on `slot`. Skips the no-op write —
    /// same seat, same device, already newest — so `Changed<SeatActiveDevices>`
    /// stays honest for change-gated readers.
    pub fn mark(&mut self, slot: u8, device: ActiveDevice) {
        if self.newest == Some(slot) && self.seats.get(&slot) == Some(&device) {
            return;
        }
        self.seats.insert(slot, device);
        self.newest = Some(slot);
    }

    /// Mark the primary seat — for detectors that know the device but have no
    /// seat question to ask (touch: the machine's own screen).
    pub fn mark_primary(&mut self, device: ActiveDevice) {
        self.mark(crate::participant::ParticipantId::PRIMARY.slot(), device);
    }

    /// How this seat's gamepad buttons should be SPELLED.
    ///
    /// A seat that is not on a pad right now answers the default (Xbox-style),
    /// which is what a label has to say when nothing better is known — the same
    /// rule [`GamepadStyle::Generic`] follows. It is not a claim that the seat
    /// holds an Xbox pad.
    pub fn gamepad_style_for(&self, slot: u8) -> GamepadStyle {
        self.for_seat(slot).gamepad_style()
    }
}

/// Detect the most recent genuine input per seat.
///
/// Within one frame later checks win: keyboard, then mouse, then pads, then
/// touch — so a finger on the screen owns the frame it is down (a stray
/// keystroke from an attached bluetooth keyboard must not flip glyphs away
/// from touch while a thumb is on the stick). Idle frames leave everything
/// unchanged.
///
/// `Pointer<Over>` is deliberately NOT consulted — only a real
/// [`CursorMoved`] or a mouse-button press counts as mouse input, so a
/// rebuild-induced `Over` under a stationary mouse can never flip the machine
/// to `Mouse` (the menu snap-back root cause).
///
/// All inputs are `Option`al / drain-free so the system is a harmless no-op
/// under `MinimalPlugins` (headless / RL), where Bevy's input resources are
/// absent.
#[cfg(feature = "input")]
pub fn update_seat_active_devices(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mouse_buttons: Option<Res<ButtonInput<MouseButton>>>,
    mut cursor_moved: MessageReader<CursorMoved>,
    touches: Option<Res<bevy::input::touch::Touches>>,
    pads: Query<(Entity, &Gamepad, Option<&Name>)>,
    offer: Option<Res<crate::seating::LocalSeatOffer>>,
    keyboard_owner: Option<Res<crate::sources::KeyboardOwner>>,
    topology: Option<Res<crate::local_seats::LocalSeatTopology>>,
    participants: Query<(
        &crate::participant::InputParticipant,
        &leafwing_input_manager::prelude::InputMap<crate::Platformer2dInputActionMonolith>,
    )>,
    mut devices: ResMut<SeatActiveDevices>,
) {
    let seat_count = participants.iter().len();
    // The keyboard-and-mouse bundle's seat: its exclusive owner when the
    // session named one, the primary otherwise.
    // ⭐⭐ A FROZEN PLAN OWNS THIS QUESTION, exactly as it owns seat devices one
    // layer down (`assign_local_seat_devices`). This read model used to answer it
    // independently and default to PRIMARY, so a match whose plan put the
    // keyboard on channel 1 still lit up channel 0's prompts and picked channel
    // 0's control filters on every keypress — wrong glyphs and wrong filtering,
    // even after fighter control itself was repaired.
    let declared = topology
        .as_deref()
        .and_then(|topology| topology.declared_channels().cloned());
    let keyboard_seat = match &declared {
        // ⛔ AND `None` HERE MEANS NOBODY. A plan that names no keyboard is a
        // match played entirely on pads: a keypress during it belongs to no
        // fighter's seat, and attributing it to one is the alias this fixes.
        Some(plan) => plan.keyboard_channel().map(|id| id.slot()),
        None => Some(
            crate::sources::keyboard_owner_for(
                offer.map(|offer| offer.policy()).unwrap_or_default(),
                keyboard_owner.map(|owner| *owner).unwrap_or_default(),
                seat_count,
            )
            .unwrap_or(crate::participant::ParticipantId::PRIMARY)
            .slot(),
        ),
    };

    // Keyboard: any key newly pressed this frame.
    if let (Some(keys), Some(keyboard_seat)) = (keys.as_deref(), keyboard_seat) {
        if keys.get_just_pressed().next().is_some() {
            devices.mark(keyboard_seat, ActiveDevice::Keyboard);
        }
    }

    // Mouse: a REAL cursor move (actual motion) OR a mouse-button press —
    // never `Pointer<Over>` (the snap-back bug).
    let real_cursor_motion = cursor_moved.read().next().is_some();
    let mouse_pressed = mouse_buttons
        .as_deref()
        .is_some_and(|buttons| buttons.get_just_pressed().next().is_some());
    // ⛔ THE MOUSE RIDES WITH THE KEYBOARD, including into "nobody". They are one
    // bundle (`LocalInputSource::Keyboard`), so a match played entirely on pads
    // must not light a fighter's prompts because somebody moved the cursor.
    if let Some(keyboard_seat) = keyboard_seat {
        if real_cursor_motion || mouse_pressed {
            devices.mark(keyboard_seat, ActiveDevice::Mouse);
        }
    }

    // Gamepads: a button just-pressed OR an axis past a generous deflection,
    // attributed to the seat whose map is associated with that pad. With one
    // seat, leafwing's any-pad fallback means every pad is the primary's;
    // with more, a pad nobody holds is a spare on the desk and marks no seat.
    const GAMEPAD_AXIS_DEFLECTION: f32 = 0.5;
    for (pad_entity, pad, name) in pads.iter() {
        let button = pad.get_just_pressed().next().is_some();
        let axis = pad.get_analog_axes().any(|axis| {
            pad.get(*axis)
                .is_some_and(|value| value.abs() >= GAMEPAD_AXIS_DEFLECTION)
        });
        if !button && !axis {
            continue;
        }
        let holder = participants
            .iter()
            .find(|(_, map)| map.gamepad() == Some(pad_entity))
            .map(|(participant, _)| participant.id.slot());
        let seat = match holder {
            Some(slot) => slot,
            None if seat_count < 2 => crate::participant::ParticipantId::PRIMARY.slot(),
            None => continue,
        };
        let style = gamepad_style_of(pad.vendor_id(), name.map(|name| name.as_str()));
        devices.mark(seat, ActiveDevice::Gamepad(style));
    }

    // Touch: any finger down owns the frame (checked LAST so it wins). The
    // screen is the machine's own, so it is the primary's device. The touch
    // overlay's virtual stick/buttons additionally mark from their own fold —
    // they can be driven by a mouse, which `Touches` cannot see.
    if touches
        .as_deref()
        .is_some_and(|touches| touches.iter().next().is_some())
    {
        devices.mark_primary(ActiveDevice::Touch);
    }
}

#[cfg(all(test, feature = "input"))]
mod tests {
    use super::*;
    use crate::participant::{InputParticipant, ParticipantId};
    use crate::presets::KeyboardPreset;

    /// A `CursorMoved` requires a window entity; spawn a throwaway one.
    fn dummy_window(app: &mut App) -> Entity {
        app.world_mut().spawn_empty().id()
    }

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<SeatActiveDevices>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<ButtonInput<MouseButton>>();
        app.add_message::<CursorMoved>();
        app.add_systems(Update, update_seat_active_devices);
        app
    }

    fn machine(app: &App) -> ActiveDevice {
        app.world().resource::<SeatActiveDevices>().machine()
    }

    #[test]
    fn defaults_to_keyboard() {
        assert_eq!(
            SeatActiveDevices::default().machine(),
            ActiveDevice::Keyboard
        );
        assert_eq!(
            SeatActiveDevices::default().for_seat(3),
            ActiveDevice::Keyboard
        );
    }

    #[test]
    fn key_press_flips_the_machine_to_keyboard() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<SeatActiveDevices>()
            .mark_primary(ActiveDevice::Mouse);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowDown);
        app.update();
        assert_eq!(machine(&app), ActiveDevice::Keyboard);
    }

    #[test]
    fn real_cursor_move_flips_to_mouse() {
        let mut app = app();
        let window = dummy_window(&mut app);
        app.world_mut().write_message(CursorMoved {
            window,
            position: Vec2::new(10.0, 10.0),
            delta: Some(Vec2::new(3.0, 0.0)),
        });
        app.update();
        assert_eq!(machine(&app), ActiveDevice::Mouse);
    }

    #[test]
    fn pointer_over_does_not_flip_to_mouse() {
        // `Pointer<Over>` is an entity-picking event, NOT a `CursorMoved` and
        // NOT a mouse-button press, so this system never reads it. A frame
        // with neither (the exact state during a rebuild-induced `Over`)
        // must keep the prior device.
        let mut app = app();
        app.update();
        assert_eq!(
            machine(&app),
            ActiveDevice::Keyboard,
            "a frame with no CursorMoved / mouse press (the rebuild Over case) keeps the prior kind"
        );
    }

    #[test]
    fn mouse_button_flips_to_mouse() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        assert_eq!(machine(&app), ActiveDevice::Mouse);
    }

    #[test]
    fn idle_frame_keeps_previous_value() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<SeatActiveDevices>()
            .mark_primary(ActiveDevice::Touch);
        app.update();
        assert_eq!(
            machine(&app),
            ActiveDevice::Touch,
            "nothing fired -> the previous (touch) value survives"
        );
    }

    #[test]
    fn a_pad_press_is_the_holding_seats_device_and_nobody_elses() {
        let mut app = app();
        // Seat 1 holds the pad (its map is associated with it); seat 0 is on
        // the keyboard. This is the couch arrangement the per-seat fact
        // exists for.
        let pad = app
            .world_mut()
            .spawn((
                Gamepad::default(),
                Name::new("DualSense Wireless Controller"),
            ))
            .id();
        app.world_mut().spawn((
            InputParticipant::primary(),
            KeyboardPreset::arrows_zxc().input_map(),
        ));
        let mut seat_one_map = KeyboardPreset::of(KeyboardPreset::by_index(0).id)
            .map_for(crate::BindingSources::GamepadOnly);
        seat_one_map.set_gamepad(pad);
        app.world_mut()
            .spawn((InputParticipant::with_id(ParticipantId(1)), seat_one_map));

        app.world_mut()
            .entity_mut(pad)
            .get_mut::<Gamepad>()
            .expect("pad")
            .digital_mut()
            .press(GamepadButton::South);
        app.update();

        let devices = app.world().resource::<SeatActiveDevices>();
        assert_eq!(
            devices.for_seat(1),
            ActiveDevice::Gamepad(GamepadStyle::PlayStation),
            "the press lands on the seat holding the pad, styled by its vendor"
        );
        assert_eq!(
            devices.for_seat(0),
            ActiveDevice::Keyboard,
            "the keyboard seat's device is untouched by somebody else's pad"
        );
        assert_eq!(
            devices.machine(),
            ActiveDevice::Gamepad(GamepadStyle::PlayStation),
            "and the machine-level answer is the newest speaker, for the hover gates"
        );
    }

    #[test]
    fn a_touch_wins_the_frame_it_is_down() {
        // The real message path: `InputPlugin`'s touch system folds
        // `TouchInput` into `Touches` in `PreUpdate`, before the detector.
        let mut app = app();
        app.add_plugins(bevy::input::InputPlugin);
        let window = dummy_window(&mut app);
        // A key fires the same frame a finger is down: touch wins — a stray
        // bluetooth keystroke must not flip glyphs away from a thumb on the
        // stick.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyA);
        app.world_mut()
            .write_message(bevy::input::touch::TouchInput {
                phase: bevy::input::touch::TouchPhase::Started,
                position: Vec2::new(5.0, 5.0),
                window,
                force: None,
                id: 7,
            });
        app.update();
        assert_eq!(machine(&app), ActiveDevice::Touch);
    }

    #[test]
    fn vendor_classification_prefers_ids_and_falls_back_to_names() {
        assert_eq!(
            gamepad_style_of(Some(0x054c), None),
            GamepadStyle::PlayStation
        );
        assert_eq!(gamepad_style_of(Some(0x057e), None), GamepadStyle::Switch);
        assert_eq!(gamepad_style_of(Some(0x045e), None), GamepadStyle::XboxLike);
        assert_eq!(
            gamepad_style_of(None, Some("Nintendo Switch Pro Controller")),
            GamepadStyle::Switch
        );
        assert_eq!(
            gamepad_style_of(None, Some("8BitDo SN30 Pro")),
            GamepadStyle::Generic
        );
    }
}
