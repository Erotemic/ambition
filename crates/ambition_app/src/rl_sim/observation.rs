/// Per-tick observation surfaced to an RL agent or scripted driver.
///
/// All fields are simple owned types so this struct can be cheaply moved
/// across language boundaries (PyO3, FFI) without lifetime entanglements.
/// Strings (`body_mode`, `active_room`) are owned `String` for the same
/// reason. Add fields here when the agent needs more state — the cost is
/// one or two `world.resource()` reads per tick, which is negligible.
#[derive(Clone, Debug)]
pub struct AgentObservation {
    /// Number of `app.update()` calls since `SandboxSim::new`. The first
    /// observation (after `new()`) returns `tick = 0`; the first `step`
    /// returns `tick = 1`.
    pub tick: u64,
    pub player_pos: (f32, f32),
    pub player_vel: (f32, f32),
    pub player_size: (f32, f32),
    pub on_ground: bool,
    pub on_wall: bool,
    pub wall_clinging: bool,
    pub wall_climbing: bool,
    pub facing: f32,
    pub fast_falling: bool,
    pub fly_enabled: bool,
    pub gliding: bool,
    pub dash_charges: u8,
    pub air_jumps: u8,
    pub blink_aiming: bool,
    pub hp: i32,
    pub hp_max: i32,
    pub mana: i32,
    pub mana_max: i32,
    pub time_alive: f32,
    pub resets: u32,
    pub body_mode: String,
    pub active_room: String,
    pub world_size: (f32, f32),
    pub world_spawn: (f32, f32),
    pub last_safe_pos: (f32, f32),
    /// True if `damage_invuln_timer` is positive — the player took damage
    /// recently. Useful as a sparse negative-reward signal.
    pub recently_damaged: bool,
    /// True while the player is in hitstun. Movement input is reduced
    /// during this window.
    pub in_hitstun: bool,
    /// True if invincibility is on (debug toggle / future invuln frames).
    pub invincible: bool,
    /// True if the player AABB overlaps a water region this frame.
    /// `water_kind` carries `Some("Clear")` / `Some("Murky")` only when
    /// `in_water` is true; cheap one-bit-plus-label encoding lets RL
    /// policies condition on water without a full struct copy.
    pub in_water: bool,
    pub water_kind: Option<String>,
    /// `[0, 1]` how submerged the player is. 0 when not in water.
    pub water_submersion: f32,
    /// True if the player AABB overlaps a climbable region (ladder /
    /// wall / vine) this frame.
    pub on_climbable: bool,
    pub climbable_kind: Option<String>,
}

impl AgentObservation {
    /// Player health fraction in `[0.0, 1.0]`. Returns 0.0 when `hp_max`
    /// is zero (defensive against a future schema change).
    pub fn hp_fraction(&self) -> f32 {
        if self.hp_max <= 0 {
            0.0
        } else {
            (self.hp as f32 / self.hp_max as f32).clamp(0.0, 1.0)
        }
    }

    /// True iff the player is alive (hp > 0). Cheap accessor for reward
    /// shaping.
    pub fn alive(&self) -> bool {
        self.hp > 0
    }
}
