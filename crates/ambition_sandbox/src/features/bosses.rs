use super::*;

fn mockingbird_combat_size() -> ae::Vec2 {
    ae::Vec2::new(500.0, 185.0)
}

#[derive(Clone, Debug)]
pub struct BossRuntime {
    pub id: String,
    pub name: String,
    pub pos: ae::Vec2,
    pub spawn: ae::Vec2,
    pub size: ae::Vec2,
    pub health: ae::Health,
    pub brain: ae::BossBrain,
    pub alive: bool,
    pub pattern_timer: f32,
    pub movement_timer: f32,
    pub attack_windup_timer: f32,
    pub attack_timer: f32,
    pub attack_cooldown: f32,
    pub hit_flash: f32,
}

impl BossRuntime {
    pub(super) fn new(object: &ae::RoomObject, brain: ae::BossBrain) -> Self {
        Self {
            id: object.id.clone(),
            name: object.name.clone(),
            pos: object.aabb.center(),
            spawn: object.aabb.center(),
            size: object.aabb.half_size() * 2.0,
            health: ae::Health::new(18),
            brain,
            alive: true,
            pattern_timer: 0.0,
            movement_timer: 0.0,
            attack_windup_timer: 0.0,
            attack_timer: 0.0,
            attack_cooldown: 0.35,
            hit_flash: 0.0,
        }
    }

    pub(super) fn update(
        &mut self,
        world: &ae::World,
        player: &ae::Player,
        tuning: FeatureCombatTuning,
        dt: f32,
    ) {
        if !self.alive {
            return;
        }
        self.pattern_timer += dt;
        self.movement_timer += dt;
        // AMBITION_REVIEW(spatial): this is still a cheap authored boss movement
        // prototype, but it now computes chase from the stable spawn anchor and
        // moves toward the target with an axis-separated collision guard. The
        // previous version used current position as feedback in the chase term,
        // which could flip sign every frame near the player and visually split
        // the boss into two flickering locations.
        let anchor_to_player = player.pos - self.spawn;
        let chase = (anchor_to_player.x * 0.18).clamp(-70.0, 70.0);
        let target = ae::Vec2::new(
            self.spawn.x + (self.movement_timer * 0.72).sin() * 130.0 + chase,
            self.spawn.y - (self.movement_timer * 1.10).sin().abs() * 18.0,
        );
        self.move_toward_target(world, target, dt);
        self.hit_flash = (self.hit_flash - dt).max(0.0);
        let was_winding_up = self.attack_windup_timer > 0.0;
        self.attack_windup_timer = (self.attack_windup_timer - dt).max(0.0);
        self.attack_timer = (self.attack_timer - dt).max(0.0);
        self.attack_cooldown = (self.attack_cooldown - dt).max(0.0);
        if was_winding_up && self.attack_windup_timer <= 0.0 {
            self.attack_timer = tuning.boss_attack_active.max(0.01);
        }
        if self.attack_cooldown <= 0.0
            && self.attack_windup_timer <= 0.0
            && self.attack_timer <= 0.0
        {
            self.attack_windup_timer = tuning.boss_attack_windup.max(0.01);
            self.attack_cooldown = BOSS_ATTACK_COOLDOWN;
        }
    }

    pub fn is_mockingbird(&self) -> bool {
        self.name.eq_ignore_ascii_case("mockingbird")
    }

    pub fn render_size(&self) -> ae::Vec2 {
        self.size
    }

    pub fn combat_size(&self) -> ae::Vec2 {
        if self.is_mockingbird() {
            // The mockingbird sprite is a wide flying boss. Its authored LDtk
            // size still drives render scale, but combat should match the
            // visible ship/creature silhouette rather than the old narrow
            // placeholder rectangle. Keep this per-boss so the gradient
            // sentinel and any other BossSpawn keep their original authored box.
            mockingbird_combat_size()
        } else {
            self.size
        }
    }

    pub fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.combat_size() * 0.5)
    }

    pub fn attack_volumes(&self) -> Vec<ae::Aabb> {
        if self.attack_timer <= 0.0 {
            return Vec::new();
        }
        self.pattern_volumes()
    }

    pub fn attack_telegraph_volumes(&self) -> Vec<ae::Aabb> {
        if self.attack_windup_timer <= 0.0 {
            return Vec::new();
        }
        self.pattern_volumes()
    }

    pub fn body_damage_aabb(&self) -> ae::Aabb {
        self.aabb()
    }

    pub(super) fn move_toward_target(&mut self, world: &ae::World, target: ae::Vec2, dt: f32) {
        let move_size = self.combat_size();
        let half = move_size * 0.5;
        let margin = 8.0;
        let max_x = (world.size.x - half.x - margin).max(half.x + margin);
        let max_y = (world.size.y - half.y - margin).max(half.y + margin);
        let clamped_target = ae::Vec2::new(
            target.x.clamp(half.x + margin, max_x),
            target.y.clamp(half.y + margin, max_y),
        );
        let delta = clamped_target - self.pos;
        let max_step = 220.0 * dt.max(0.0);
        let step = if delta.length() > max_step && max_step > 0.0 {
            delta.normalize_or_zero() * max_step
        } else {
            delta
        };

        let try_x = ae::Vec2::new(self.pos.x + step.x, self.pos.y);
        if boss_space_is_free(world, try_x, move_size) {
            self.pos.x = try_x.x;
        }
        let try_y = ae::Vec2::new(self.pos.x, self.pos.y + step.y);
        if boss_space_is_free(world, try_y, move_size) {
            self.pos.y = try_y.y;
        }
    }

    pub(super) fn pattern_volumes(&self) -> Vec<ae::Aabb> {
        let size = self.combat_size();
        let phase = ((self.pattern_timer / BOSS_ATTACK_COOLDOWN) as i32).rem_euclid(3);
        match phase {
            0 => vec![ae::Aabb::new(
                self.pos + ae::Vec2::new(0.0, size.y * 0.5 + 22.0),
                ae::Vec2::new(size.x * 0.75, 18.0),
            )],
            1 => vec![
                ae::Aabb::new(
                    self.pos + ae::Vec2::new(-size.x * 0.50, 0.0),
                    ae::Vec2::new(size.x * 0.25, size.y * 0.72),
                ),
                ae::Aabb::new(
                    self.pos + ae::Vec2::new(size.x * 0.50, 0.0),
                    ae::Vec2::new(size.x * 0.25, size.y * 0.72),
                ),
            ],
            _ => vec![ae::Aabb::new(self.pos, size * 0.70)],
        }
    }

    pub(super) fn player_damage(&self, player_body: ae::Aabb) -> Option<PlayerDamageEvent> {
        if self.attack_timer > 0.0 {
            if let Some(volume) = self
                .attack_volumes()
                .into_iter()
                .find(|volume| volume.strict_intersects(player_body))
            {
                return Some(PlayerDamageEvent {
                    mode: PlayerDamageMode::Knockback,
                    source: PlayerDamageSource::BossAttack,
                    source_pos: self.pos,
                    impact_pos: midpoint(player_body.center(), volume.center()),
                    knockback_dir: (player_body.center().x - self.pos.x).signum_or(1.0),
                    strength: 1.25,
                    amount: 2,
                });
            }
        }
        let body_damage = self.body_damage_aabb();
        if body_damage.strict_intersects(player_body) {
            return Some(PlayerDamageEvent {
                mode: PlayerDamageMode::Knockback,
                source: PlayerDamageSource::BossBody,
                source_pos: self.pos,
                impact_pos: midpoint(player_body.center(), body_damage.center()),
                knockback_dir: (player_body.center().x - self.pos.x).signum_or(1.0),
                strength: 1.0,
                amount: 1,
            });
        }
        None
    }
}
