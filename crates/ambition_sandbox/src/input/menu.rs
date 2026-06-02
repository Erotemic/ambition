use super::*;

/// Per-frame menu navigation snapshot. Decoded from `SandboxAction`'s
/// `Menu*` actions plus the analog left-stick (with deadzone + repeat)
/// so the pause-menu controller doesn't have to know about leafwing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MenuInputFrame {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub select: bool,
    pub back: bool,
    pub start: bool,
}

impl MenuInputFrame {
    pub fn any_directional(self) -> bool {
        self.up || self.down || self.left || self.right
    }
}

/// Device-agnostic per-frame UI/menu intent.
///
/// This is the menu-side companion to [`ControlFrame`]. Keyboard/gamepad,
/// mouse wheel, touch gestures, on-screen buttons, and eventually Android
/// system back should all fold into this resource before menu systems run.
/// Menus consume semantic intents instead of raw Leafwing `ActionState` or
/// raw touch events, which keeps RL/gameplay controls separate from UI
/// ergonomics.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct MenuControlFrame {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub select: bool,
    pub select_held: bool,
    pub back: bool,
    pub back_held: bool,
    pub start: bool,
    pub inventory: bool,
    pub map: bool,
    /// Positive values mean “navigate/scroll up”, negative values mean
    /// “navigate/scroll down”. Mouse wheels and touch drags both add here.
    pub scroll_y: f32,
}

impl MenuControlFrame {
    pub fn from_menu_input(input: MenuInputFrame) -> Self {
        Self {
            up: input.up,
            down: input.down,
            left: input.left,
            right: input.right,
            select: input.select,
            select_held: input.select,
            back: input.back,
            back_held: input.back,
            start: input.start,
            ..Default::default()
        }
    }

    pub fn any_directional(self) -> bool {
        self.up || self.down || self.left || self.right
    }

    pub fn any_navigation(self) -> bool {
        self.any_directional() || self.scroll_y.abs() >= 0.5
    }

    /// Convert accumulated scroll/drag into discrete row navigation steps.
    ///
    /// Mouse wheels usually arrive as small integer deltas. Touch drag uses
    /// pixel deltas divided by a coarse divisor before entering this frame.
    /// Clamping keeps one giant swipe from skipping an entire menu page.
    pub fn vertical_scroll_steps(self) -> i32 {
        self.scroll_y.round().clamp(-6.0, 6.0) as i32
    }
}

/// State the menu input system carries across frames so analog repeat
/// behaves predictably.
///
/// `held_dir` records the currently-held direction (or `None`).
/// `time_since_repeat` is the accumulated dt since the last emitted
/// repeat tick. When `held_dir` changes, both timers reset.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MenuInputState {
    pub held_dir: Option<MenuDir>,
    /// Time the current direction has been continuously held. Reset on
    /// new direction.
    held_for_centiseconds: u16,
    /// Time since the last repeat tick was emitted on this direction.
    repeat_accum_centiseconds: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuDir {
    Up,
    Down,
    Left,
    Right,
}

impl MenuInputState {
    /// Resolve a per-frame menu input given the analog stick + button
    /// edge state plus the user's repeat tuning.
    ///
    /// `analog_dir` is the discrete direction the analog stick is
    /// currently pushed toward (after deadzone), or None. `edge_*` are
    /// the discrete edge events from D-pad / arrow keys / WASD.
    pub fn step(
        &mut self,
        edge_up: bool,
        edge_down: bool,
        edge_left: bool,
        edge_right: bool,
        analog_dir: Option<MenuDir>,
        select_pressed: bool,
        back_pressed: bool,
        start_pressed: bool,
        dt_seconds: f32,
        initial_delay: f32,
        repeat_interval: f32,
    ) -> MenuInputFrame {
        // Cardinal edges (D-pad / keyboard) always emit on the press
        // edge regardless of the held analog state. Repeat is reserved
        // for the analog axis so users who hold a stick get predictable
        // pacing rather than cardinal-edge mashing.
        let mut frame = MenuInputFrame {
            up: edge_up,
            down: edge_down,
            left: edge_left,
            right: edge_right,
            select: select_pressed,
            back: back_pressed,
            start: start_pressed,
        };

        match analog_dir {
            Some(dir) if Some(dir) == self.held_dir => {
                // Continuing to hold the same direction: count time
                // toward the next repeat tick.
                self.held_for_centiseconds = self
                    .held_for_centiseconds
                    .saturating_add(centiseconds(dt_seconds));
                let initial_cs = centiseconds(initial_delay);
                if self.held_for_centiseconds >= initial_cs {
                    self.repeat_accum_centiseconds = self
                        .repeat_accum_centiseconds
                        .saturating_add(centiseconds(dt_seconds));
                    let interval_cs = centiseconds(repeat_interval).max(1);
                    if self.repeat_accum_centiseconds >= interval_cs {
                        self.repeat_accum_centiseconds = 0;
                        match dir {
                            MenuDir::Up => frame.up = true,
                            MenuDir::Down => frame.down = true,
                            MenuDir::Left => frame.left = true,
                            MenuDir::Right => frame.right = true,
                        }
                    }
                }
            }
            Some(dir) => {
                // New direction: emit immediately, then wait for the
                // initial delay before repeating.
                self.held_dir = Some(dir);
                self.held_for_centiseconds = 0;
                self.repeat_accum_centiseconds = 0;
                match dir {
                    MenuDir::Up => frame.up = true,
                    MenuDir::Down => frame.down = true,
                    MenuDir::Left => frame.left = true,
                    MenuDir::Right => frame.right = true,
                }
            }
            None => {
                // Analog stick released — reset so the next push fires
                // immediately again.
                self.held_dir = None;
                self.held_for_centiseconds = 0;
                self.repeat_accum_centiseconds = 0;
            }
        }
        frame
    }
}

fn centiseconds(seconds: f32) -> u16 {
    (seconds * 100.0).clamp(0.0, u16::MAX as f32) as u16
}

/// Convert an analog stick vector (post-deadzone) into a single
/// discrete direction. Returns `None` when below `threshold`.
pub fn analog_to_dir(x: f32, y: f32, threshold: f32) -> Option<MenuDir> {
    let mag = (x * x + y * y).sqrt();
    if mag < threshold {
        return None;
    }
    if x.abs() > y.abs() {
        if x > 0.0 {
            Some(MenuDir::Right)
        } else {
            Some(MenuDir::Left)
        }
    } else if y > 0.0 {
        Some(MenuDir::Up)
    } else {
        Some(MenuDir::Down)
    }
}

#[cfg(test)]
mod menu_input_tests {
    //! Pure menu-input logic: the analog stick -> direction mapping, the
    //! scroll-to-rows quantizer, and the analog auto-repeat state machine
    //! (cardinal edges always fire; a held stick waits an initial delay
    //! then repeats; release resets).
    use super::*;

    fn step(s: &mut MenuInputState, dir: Option<MenuDir>) -> MenuInputFrame {
        s.step(false, false, false, false, dir, false, false, false, 0.1, 0.3, 0.1)
    }

    #[test]
    fn analog_to_dir_picks_dominant_axis_past_threshold() {
        assert_eq!(analog_to_dir(0.0, 0.0, 0.5), None);
        assert_eq!(analog_to_dir(1.0, 0.0, 0.5), Some(MenuDir::Right));
        assert_eq!(analog_to_dir(-1.0, 0.0, 0.5), Some(MenuDir::Left));
        assert_eq!(analog_to_dir(0.0, 1.0, 0.5), Some(MenuDir::Up));
        assert_eq!(analog_to_dir(0.0, -1.0, 0.5), Some(MenuDir::Down));
        // x dominates a shallow diagonal.
        assert_eq!(analog_to_dir(1.0, 0.4, 0.5), Some(MenuDir::Right));
    }

    #[test]
    fn vertical_scroll_steps_rounds_and_clamps() {
        let frame = |s: f32| MenuControlFrame {
            scroll_y: s,
            ..Default::default()
        };
        assert_eq!(frame(2.4).vertical_scroll_steps(), 2);
        assert_eq!(frame(0.4).vertical_scroll_steps(), 0);
        assert_eq!(frame(10.0).vertical_scroll_steps(), 6);
        assert_eq!(frame(-10.0).vertical_scroll_steps(), -6);
    }

    #[test]
    fn from_menu_input_mirrors_fields_and_navigation_predicates() {
        let cf = MenuControlFrame::from_menu_input(MenuInputFrame {
            down: true,
            ..Default::default()
        });
        assert!(cf.down && cf.any_directional() && cf.any_navigation());
        let scroll = MenuControlFrame {
            scroll_y: 1.0,
            ..Default::default()
        };
        assert!(!scroll.any_directional());
        assert!(scroll.any_navigation(), "a scroll past 0.5 counts as navigation");
    }

    #[test]
    fn step_emits_new_direction_then_waits_under_the_initial_delay() {
        let mut s = MenuInputState::default();
        assert!(step(&mut s, Some(MenuDir::Down)).down, "new direction emits at once");
        assert_eq!(s.held_dir, Some(MenuDir::Down));
        assert!(
            !step(&mut s, Some(MenuDir::Down)).down,
            "still under the initial delay -> no repeat yet"
        );
    }

    #[test]
    fn step_repeats_after_the_initial_delay() {
        let mut s = MenuInputState::default();
        step(&mut s, Some(MenuDir::Up)); // initial emit
        let repeats = (0..20).filter(|_| step(&mut s, Some(MenuDir::Up)).up).count();
        assert!(repeats > 0, "a held direction eventually repeats");
    }

    #[test]
    fn step_release_resets_held_dir() {
        let mut s = MenuInputState::default();
        step(&mut s, Some(MenuDir::Left));
        assert_eq!(s.held_dir, Some(MenuDir::Left));
        step(&mut s, None);
        assert_eq!(s.held_dir, None, "releasing the stick clears held_dir");
    }

    #[test]
    fn step_cardinal_edges_always_emit() {
        let mut s = MenuInputState::default();
        let f = s.step(true, false, false, false, None, false, false, false, 0.1, 0.3, 0.1);
        assert!(f.up, "a cardinal edge emits regardless of the analog repeat state");
        assert_eq!(s.held_dir, None);
    }
}
