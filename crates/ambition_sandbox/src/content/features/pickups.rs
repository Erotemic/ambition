use super::*;

#[derive(Clone, Debug)]
pub struct PickupRuntime {
    pub id: String,
    pub name: String,
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub pickup: crate::interaction::Pickup,
    pub visible: bool,
}

impl PickupRuntime {
    pub fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.size * 0.5)
    }
}
