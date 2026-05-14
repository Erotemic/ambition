use super::*;

#[derive(Clone, Debug)]
pub struct BreakableRuntime {
    pub id: String,
    pub name: String,
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub breakable: ae::Breakable,
    pub respawn_timer: f32,
    pub stand_timer: f32,
}

impl BreakableRuntime {
    pub fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.size * 0.5)
    }

    pub fn broken(&self) -> bool {
        self.breakable.state == ae::BreakableState::Broken
    }

    pub(super) fn start_respawn_timer(&mut self) {
        self.stand_timer = 0.0;
        if let ae::RespawnPolicy::AfterSeconds(seconds) = self.breakable.respawn {
            self.respawn_timer = seconds;
        }
    }

    pub(super) fn breaks_on_stand(&self) -> bool {
        self.breakable.collision.blocks_movement() && self.breakable.trigger.allows_stand()
    }

    pub(super) fn breaks_on_hit(&self) -> bool {
        self.breakable.trigger.allows_hit()
    }
}
