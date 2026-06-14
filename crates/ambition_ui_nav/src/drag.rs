use bevy::prelude::Vec2;

/// Accumulates pointer/touch drag into discrete menu scroll steps.
///
/// Bevy touch/cursor positions are top-left-origin. A phone-style swipe up
/// has negative `dy`, which should become negative `scroll_y` so menus move
/// to the next row through the same path as a down-arrow press.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DragScrollState {
    last_pos: Option<Vec2>,
    accumulated_steps: f32,
}

impl DragScrollState {
    /// Feed the current pointer position. Returns whole row-scroll steps.
    ///
    /// Passing `None` resets the gesture. Tiny hand tremors are ignored until
    /// `deadzone_px` is exceeded; partial rows stay accumulated so a series of
    /// small mobile drags still advances the menu.
    pub fn update(
        &mut self,
        pos: Option<Vec2>,
        pixels_per_step: f32,
        deadzone_px: f32,
        max_steps_per_frame: f32,
    ) -> f32 {
        let Some(pos) = pos else {
            self.reset();
            return 0.0;
        };
        let mut steps = 0.0;
        if let Some(last) = self.last_pos {
            let dy = pos.y - last.y;
            if dy.abs() >= deadzone_px {
                self.accumulated_steps += dy / pixels_per_step.max(1.0);
                steps = self
                    .accumulated_steps
                    .trunc()
                    .clamp(-max_steps_per_frame.abs(), max_steps_per_frame.abs());
                if steps != 0.0 {
                    self.accumulated_steps -= steps;
                }
            }
        }
        self.last_pos = Some(pos);
        steps
    }

    pub fn reset(&mut self) {
        self.last_pos = None;
        self.accumulated_steps = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_drags_accumulate_into_discrete_steps() {
        let mut drag = DragScrollState::default();
        assert_eq!(
            drag.update(Some(Vec2::new(0.0, 100.0)), 30.0, 3.0, 5.0),
            0.0
        );
        assert_eq!(drag.update(Some(Vec2::new(0.0, 88.0)), 30.0, 3.0, 5.0), 0.0);
        assert_eq!(
            drag.update(Some(Vec2::new(0.0, 70.0)), 30.0, 3.0, 5.0),
            -1.0
        );
    }

    #[test]
    fn reset_discards_partial_drag() {
        let mut drag = DragScrollState::default();
        drag.update(Some(Vec2::new(0.0, 100.0)), 30.0, 3.0, 5.0);
        drag.update(Some(Vec2::new(0.0, 88.0)), 30.0, 3.0, 5.0);
        drag.reset();
        assert_eq!(drag.update(Some(Vec2::new(0.0, 70.0)), 30.0, 3.0, 5.0), 0.0);
    }
}
