use super::*;

#[derive(Clone, Debug)]
pub struct ChestRuntime {
    pub id: String,
    pub name: String,
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub chest: ae::Chest,
    pub opened: bool,
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
        }
    }

    pub fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.size * 0.5)
    }
}
