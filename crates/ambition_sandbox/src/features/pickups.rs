use super::*;

#[derive(Clone, Debug)]
pub struct PickupRuntime {
    pub id: String,
    pub name: String,
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub pickup: ae::Pickup,
    pub visible: bool,
}

impl PickupRuntime {
    pub(super) fn new(object: &ae::RoomObject, pickup: ae::Pickup) -> Self {
        Self {
            id: object.id.clone(),
            name: object.name.clone(),
            pos: object.aabb.center(),
            size: object.aabb.half_size() * 2.0,
            pickup,
            visible: true,
        }
    }

    pub fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.size * 0.5)
    }
}
