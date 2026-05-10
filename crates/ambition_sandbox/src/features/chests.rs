use super::*;

#[derive(Clone, Debug)]
pub struct ChestRuntime {
    pub id: String,
    pub name: String,
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub chest: ae::Chest,
    pub opened: bool,
    /// True while the chest is dropping toward the floor (e.g. just
    /// released by a defeated boss). Cleared on first solid contact.
    /// Authored / encounter chests start `false` — they're already
    /// where the room author wants them.
    pub falling: bool,
    /// Vertical velocity in y-down world space (positive = downward).
    /// Only meaningful while `falling`.
    pub vel_y: f32,
}

impl ChestRuntime {
    pub(super) fn new(object: &ae::RoomObject, chest: ae::Chest) -> Self {
        Self {
            id: object.id.clone(),
            name: object.name.clone(),
            pos: object.aabb.center(),
            size: object.aabb.half_size() * 2.0,
            chest,
            opened: false,
            falling: false,
            vel_y: 0.0,
        }
    }

    pub fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.size * 0.5)
    }
}
