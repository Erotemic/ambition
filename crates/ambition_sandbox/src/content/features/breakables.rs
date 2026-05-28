use super::*;

#[derive(Clone, Debug)]
pub struct BreakableRuntime {
    pub id: String,
    pub name: String,
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub breakable: crate::interaction::Breakable,
    pub respawn_timer: f32,
    pub stand_timer: f32,
}

impl BreakableRuntime {
    pub fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.size * 0.5)
    }

    pub fn broken(&self) -> bool {
        self.breakable.state == crate::interaction::BreakableState::Broken
    }
}
