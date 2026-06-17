//! Per-NPC runtime glue for the actor simulation: [`NpcMut`] integration
//! (grounded NPCs run the shared `integrate_normal_spine` via
//! [`NpcMut::integrate_velocity`]; flyers like the parrot route through
//! [`super::step_floating_body`] via `integrate_velocity_aerial`), brain
//! selection ([`NpcMut::build_brain`]: catalog default vs patrol/stand-still),
//! and the hit/hostile/dialogue/idle-bark line tables. Talk/hostility tuning
//! consts ([`NPC_TALK_RADIUS`], [`NPC_HOSTILE_STRIKE_THRESHOLD`]) live here;
//! `NpcConfig`/`NpcStatus`/`NpcMut` cluster components live in the `ecs` tree.

use super::ecs::npc_clusters::NpcMut;
use super::*;

/// Number of player attacks before a peaceful NPC turns hostile.
/// Three lets the player commit to the choice intentionally without
/// flipping by accident on a stray slash.
pub const NPC_HOSTILE_STRIKE_THRESHOLD: i32 = 3;

/// Fixed talk radius for patrolling NPCs. When the player gets
/// within this many world pixels, a patrolling NPC stops and faces
/// the player so the dialog interact is reachable. ~80 px ≈ 2.5
/// player widths — close enough to commit to dialog, far enough
/// that an NPC doesn't freeze the moment you walk past their
/// patrol range.
pub const NPC_TALK_RADIUS: f32 = 80.0;

/// Patrol speed for NPCs. Moved to the brain (its consumer,
/// `crate::brain::PatrolCfg::NPC_DEFAULT`); re-exported here for
/// authoring-side reference.
pub use crate::brain::NPC_PATROL_SPEED;

/// Cluster-native NPC integration + helpers. Port of the `NpcRuntime`
/// methods that operate on the authoritative ECS components through the
/// [`NpcMut`] view. Field map: self.kin.* (pos/vel/size/facing),
/// self.surface.on_ground, self.status.* (ai_mode/hit_flash/hostile/
/// strikes), self.config.* (id/name/spawn/interactable/patrol/talk),
/// self.motion.0.
impl<'a> NpcMut<'a> {
    pub fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(self.kin.pos, self.kin.size * 0.5)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn tick_via_brain(
        &mut self,
        brain: &mut crate::brain::Brain,
        world: &ae::World,
        target_pos: ae::Vec2,
        sim_time: f32,
        dt: f32,
        // World gravity DIRECTION at the NPC (down/up/sideways) so NPCs fall the
        // way the player does under any gravity, including left/right.
        gravity_dir: ae::Vec2,
    ) -> crate::actor::control::ActorControlFrame {
        self.status.hit_flash = (self.status.hit_flash - dt).max(0.0);

        let snapshot = crate::brain::BrainSnapshot {
            actor_pos: self.kin.pos,
            actor_vel: self.kin.vel,
            actor_facing: self.kin.facing,
            actor_on_ground: self.surface.on_ground,
            alive: true,
            target_pos,
            target_alive: true,
            sim_time,
            dt,
            attack_cooldown_remaining: 0.0,
            attack_windup_remaining: 0.0,
            attack_active_remaining: 0.0,
            attack_recover_remaining: 0.0,
            stun_remaining: 0.0,
            wall_contact: None,
            player_input: None,
            crowding: None,
            terrain: None,
            air_jumps_remaining: 0,
        };
        let mut frame = crate::actor::control::ActorControlFrame::neutral();
        brain.tick(&snapshot, &mut frame);

        self.status.ai_mode = match brain {
            crate::brain::Brain::StateMachine(crate::brain::StateMachineCfg::Patrol {
                state,
                ..
            }) => state.mode,
            crate::brain::Brain::StateMachine(crate::brain::StateMachineCfg::Aerial {
                state,
                ..
            }) => state.mode,
            crate::brain::Brain::StateMachine(crate::brain::StateMachineCfg::StandStill) => {
                crate::actor::ai::CharacterAiMode::Idle
            }
            _ => crate::actor::ai::CharacterAiMode::Idle,
        };

        if frame.facing.abs() > 0.001 {
            self.kin.facing = frame.facing;
        }

        // Aerial NPC (gravity-free flyer, e.g. the stochastic parrot): the brain
        // drives the FULL 2D velocity — fly up to perches, dive, drop beside the
        // player — so integrate the whole `desired_vel` with no gravity, mirroring
        // the aerial-enemy path. Keyed off `gravity_scale` like the enemy side.
        if self.surface.gravity_scale <= 0.001 {
            self.integrate_velocity_aerial(frame.desired_vel, world, dt);
            return frame;
        }

        if matches!(
            self.status.ai_mode,
            crate::actor::ai::CharacterAiMode::Patrol
        ) {
            if let Some(motion) = &mut self.motion.0 {
                let old = self.kin.pos;
                self.kin.pos = motion.advance(self.kin.pos, dt);
                let delta = self.kin.pos - old;
                self.kin.vel = if dt > 0.0 { delta / dt } else { ae::Vec2::ZERO };
                if delta.x.abs() > 0.001 {
                    self.kin.facing = delta.x.signum();
                }
                return frame;
            }
        }

        let stalled_on_wall = self.integrate_velocity(frame.desired_vel.x, world, dt, gravity_dir);

        if matches!(
            self.status.ai_mode,
            crate::actor::ai::CharacterAiMode::Patrol
        ) && stalled_on_wall
        {
            self.kin.facing *= -1.0;
        }

        if matches!(
            self.status.ai_mode,
            crate::actor::ai::CharacterAiMode::Chase
        ) {
            let dx = target_pos.x - self.kin.pos.x;
            if dx.abs() > 4.0 {
                self.kin.facing = dx.signum();
            }
        }
        frame
    }

    /// Step the NPC body one tick toward `desired_vel_x` (the rest — gravity,
    /// collision, ground contact — comes from `step_kinematic`). Shared by
    /// `tick_via_brain` and the *possession* path, where the desired velocity
    /// comes from the player's input instead of the NPC's brain.
    ///
    /// Returns whether the body STALLED on its run (gravity-perpendicular) axis
    /// this tick — i.e. it was moving along the ground and is now stopped, the
    /// signature of running into a wall. Detected on the gravity-PERPENDICULAR
    /// "side" axis (not screen-x), so it stays correct under sideways gravity; the
    /// patrol caller reverses facing on a stall.
    pub fn integrate_velocity(
        &mut self,
        desired_vel_x: f32,
        world: &ae::World,
        dt: f32,
        gravity_dir: ae::Vec2,
    ) -> bool {
        // Grounded NPCs run the SHARED player physics spine (gravity + run +
        // fall-cap, gravity-direction-relative) — the same core enemies and the
        // player use. The AI's velocity-valued `desired_vel_x` maps onto the
        // spine's `axis_x * max_run_speed` model (max_run_speed = |desired|,
        // axis_x = sign), accel = ENEMY_RUN_ACCEL, friction = 0, so it's
        // byte-identical under vertical gravity to the old hand-rolled run.
        let mut body = crate::kinematic::KinematicBody {
            pos: self.kin.pos,
            vel: self.kin.vel,
            size: self.kin.size,
            on_ground: self.surface.on_ground,
            facing: self.kin.facing,
        };
        // Run axis = gravity-perpendicular. Under vertical gravity `perp = (-1,0)`
        // so `side == ±vel.x` (byte-identical to the old screen-x read).
        let perp = ae::Vec2::new(-gravity_dir.y, gravity_dir.x);
        let prev_side_speed = body.vel.dot(perp);
        let axis_x = if desired_vel_x.abs() > 1e-3 {
            desired_vel_x.signum()
        } else {
            0.0
        };
        let spine_tuning = ae::MovementTuning {
            gravity: ENEMY_GRAVITY,
            gravity_dir,
            run_accel: ENEMY_RUN_ACCEL,
            air_accel: ENEMY_RUN_ACCEL,
            ground_friction: 0.0,
            air_friction: 0.0,
            max_run_speed: desired_vel_x.abs(),
            max_fall_speed: ENEMY_MAX_FALL,
            ..ae::MovementTuning::default()
        };
        let mut fast_falling = false;
        let mut gliding = false;
        ae::integrate_normal_spine(
            &mut body.vel,
            &mut fast_falling,
            &mut gliding,
            ae::NormalSpineCtx::bare(body.on_ground),
            ae::InputState {
                axis_x,
                ..Default::default()
            },
            dt,
            spine_tuning,
        );
        crate::kinematic::step_kinematic(
            &mut body,
            world,
            crate::kinematic::KinematicTuning {
                // Spine already applied gravity; the sweep is pure collision.
                gravity: 0.0,
                max_fall_speed: ENEMY_MAX_FALL,
                gravity_dir,
            },
            crate::kinematic::KinematicInputs::default(),
            dt,
        );
        self.kin.pos = body.pos;
        self.kin.vel = body.vel;
        self.surface.on_ground = body.on_ground;
        prev_side_speed.abs() > 1.0 && body.vel.dot(perp).abs() < 0.01
    }

    /// Gravity-free 2D integration for a flying NPC: approach the brain's full
    /// `desired_vel` (both axes) and step through collision with gravity off, so
    /// a `Floating` bird actually flies. Mirrors the aerial-enemy integrator.
    pub fn integrate_velocity_aerial(&mut self, desired_vel: ae::Vec2, world: &ae::World, dt: f32) {
        let mut body = crate::kinematic::KinematicBody {
            pos: self.kin.pos,
            vel: self.kin.vel,
            size: self.kin.size,
            on_ground: self.surface.on_ground,
            facing: self.kin.facing,
        };
        // Shared floating free-mover path (aerial enemies, bosses use the same).
        super::step_floating_body(
            &mut body,
            world,
            desired_vel,
            Some(900.0 * dt),
            ENEMY_MAX_FALL,
            dt,
        );
        self.kin.pos = body.pos;
        self.kin.vel = body.vel;
        // A flyer is never "grounded" — keeps gravity-righting + anim aerial.
        self.surface.on_ground = false;
    }

    pub fn build_brain(&self) -> crate::brain::Brain {
        // Data-driven: if this NPC was authored from a catalog row that asks for
        // a RICH, PEACEFUL brain (past Patrol/StandStill but not hostile — e.g.
        // the lively Aerial flyer), honor the catalog `default_brain`.
        //
        // A placed `NpcSpawn` is peaceful/talkable BY CONSTRUCTION, so a catalog
        // row whose `default_brain` is HOSTILE (the cove pirates carry
        // `melee_brute_striker` for when they spawn as ENEMIES) must NOT turn the
        // friendly NPC into a player-chaser — those fall through to the legacy
        // peaceful patrol/standstill below, unchanged. (An NPC only turns hostile
        // by being struck past its retaliation threshold.)
        if let crate::interaction::InteractionKind::Npc {
            character_id: Some(cid),
            ..
        } = &self.config.interactable.kind
        {
            if let Some(brain) =
                crate::character_roster::default_brain_for_character_id(cid, self.config.spawn.x)
            {
                let is_basic = matches!(
                    brain,
                    crate::brain::Brain::StateMachine(
                        crate::brain::StateMachineCfg::Patrol { .. }
                            | crate::brain::StateMachineCfg::StandStill
                    )
                );
                if !is_basic && !brain.is_hostile() {
                    return brain;
                }
            }
        }
        if self.config.patrol_radius > 0.0 || self.motion.0.is_some() {
            let mut cfg = crate::brain::PatrolCfg::NPC_DEFAULT;
            cfg.spawn_x = self.config.spawn.x;
            cfg.radius = self.config.patrol_radius;
            cfg.aggro_radius = self.config.talk_radius;
            crate::brain::Brain::StateMachine(crate::brain::StateMachineCfg::Patrol {
                cfg,
                state: crate::brain::PatrolState::default(),
            })
        } else {
            crate::brain::Brain::stand_still()
        }
    }

    pub fn flag_id(&self) -> String {
        npc_flag_id(self.config)
    }

    pub fn bark_anchor(&self) -> ae::Vec2 {
        self.kin.pos + ae::Vec2::new(0.0, -self.kin.size.y * 0.72 - 16.0)
    }

    pub fn hit_bark(&self) -> &'static str {
        npc_hit_bark_line(self.config, self.status)
    }

    pub fn hostile_bark(&self) -> &'static str {
        npc_hostile_bark_line(self.config)
    }

    pub fn message(&self) -> String {
        npc_message(self.config, self.status)
    }

    pub fn dialogue_request(&self) -> NpcDialogueRequest {
        npc_dialogue_request(self.config)
    }

    pub fn reset_to_spawn(&mut self) {
        self.kin.pos = self.config.spawn;
        self.kin.vel = ae::Vec2::ZERO;
        self.surface.on_ground = false;
        self.status.hostile = false;
        self.status.strikes = 0;
        self.status.hit_flash = 0.0;
    }
}

fn npc_hit_barks(key: &str, name: &str) -> &'static [&'static str] {
    if key.contains("hub_guide") || name.contains("kernel") || name.contains("guide") {
        &[
            "Ow. Tutorial says: don't.",
            "Input received. Annoyance rising.",
            "Debug friendship failed.",
        ]
    } else if key.contains("architect") || name.contains("architect") {
        &[
            "Careful! I'm load-bearing.",
            "That was not in the blueprint.",
            "You're voiding the warranty.",
        ]
    } else if key.contains("vault_keeper") || name.contains("vault") {
        &[
            "Hands off the vault staff.",
            "I count every scratch.",
            "That debt has interest.",
        ]
    } else if key.contains("merchant") || name.contains("merchant") {
        &[
            "No refunds for violence.",
            "You break it, you buy it.",
            "That's coming out of your wallet.",
        ]
    } else if key.contains("military_general") || name.contains("general") {
        &[
            "Soldier, explain yourself.",
            "That is insubordination.",
            "Court-martial posture engaged.",
        ]
    } else if key.contains("goblin") || name.contains("fretjaw") || name.contains("chieftain") {
        &[
            "Oi! That's my good arm.",
            "Fretjaw bites back!",
            "Cantina rules: no free hits!",
        ]
    } else if key.contains("pulse_voyager")
        || name.contains("captain pulse")
        || name.contains("pulse")
    {
        &[
            "Easy on the hull, starling.",
            "That's not standard docking procedure.",
            "Pulse shields to angry!",
        ]
    } else if key.contains("tech_bros") || name.contains("chadwick") || name.contains("disruptor") {
        &[
            "Bro. Optics.",
            "My brand is literally disruption.",
            "I'm posting about this.",
        ]
    } else if key.contains("pirate_admiral")
        || name.contains("pirate admiral")
        || name.contains("admiral")
    {
        &[
            "Belay that, ye barnacle!",
            "Mind the epaulettes, scallywag!",
            "Avast — that be admiralty property!",
            "I'll keelhaul yer cooldowns!",
        ]
    } else if key.contains("pirate_raider")
        || name.contains("pirate raider")
        || name.contains("raider")
    {
        &[
            "Yarrrgh!",
            "Quit pokin' me loot hand!",
            "I'll swab the floor with ye!",
            "Yo-ho-NO, ye landlubber!",
        ]
    } else if key.contains("pirate_lookout")
        || name.contains("pirate lookout")
        || name.contains("lookout")
    {
        &[
            "Land ho — an' I see YE comin'!",
            "Spyglass to me eye, boots to yer head!",
            "Crow's nest don't sit empty, savvy?",
        ]
    } else if key.contains("pirate_navigator")
        || name.contains("pirate navigator")
        || name.contains("navigator")
    {
        &[
            "Wrong heading, ye chartless dog!",
            "I'll plot ye a course straight to Davy Jones!",
            "Compass says: punch back!",
        ]
    } else if key.contains("broadside_bess")
        || name.contains("broadside bess")
        || name.contains("bess")
    {
        &[
            "Mind me cleaver, wee skipper!",
            "Aye, that smarts — but ye're worse off!",
            "Broadside Bess don't bend easy!",
            "Yarrrr! Take that an' a barrel more!",
        ]
    } else if key.contains("iron_mary") || name.contains("iron mary") {
        &[
            "Iron don't flinch, ye gull!",
            "Pry harder, swab — I'll rust ye flat!",
            "Yo-ho, an' a clout to the noggin!",
            "Try me on a calmer sea, landlubber!",
        ]
    } else if key.contains("salt_annet") || name.contains("salt annet") || name.contains("annet") {
        &[
            "Salt in the eye, blood in the bilge!",
            "Yargh! Watch yer manners on me deck!",
            "Wee skipper thinks he's bold, does he?",
            "Annet bites back, every time!",
        ]
    } else if key.contains("ninja_leader") || name.contains("oni leader") || name.contains("leader")
    {
        &[
            "Your form is loud.",
            "A warning: one breath left.",
            "The shadow answers.",
        ]
    } else if key.contains("ninja_duelist") || name.contains("duelist") {
        &[
            "Tch. Sloppy opening.",
            "Again? Then draw properly.",
            "Now we duel.",
        ]
    } else if key.contains("quartermaster") || name.contains("quartermaster") {
        // Pirate quartermaster lives in the cove — talk like one.
        &[
            "Inventory says NO, ye dock-rat!",
            "Yarr! Every coin's a-counted!",
            "Tally that on yer hide, swabbie!",
        ]
    } else if key.contains("guard") || name.contains("guard") {
        &["Hey.", "Last warning.", "That's it!"]
    } else {
        &["Hey.", "Cut it out.", "Okay, now I'm mad."]
    }
}

fn npc_hostile_bark(key: &str, name: &str) -> &'static str {
    if key.contains("hub_guide") || name.contains("kernel") || name.contains("guide") {
        "Combat tutorial unlocked."
    } else if key.contains("architect") || name.contains("architect") {
        "Demolition protocol!"
    } else if key.contains("vault_keeper") || name.contains("vault") {
        "The vault remembers."
    } else if key.contains("merchant") || name.contains("merchant") {
        "Final sale!"
    } else if key.contains("military_general") || name.contains("general") {
        "Weapons free!"
    } else if key.contains("goblin") || name.contains("fretjaw") || name.contains("chieftain") {
        "Cantina brawl!"
    } else if key.contains("pulse_voyager")
        || name.contains("captain pulse")
        || name.contains("pulse")
    {
        "Red alert, traveler!"
    } else if key.contains("tech_bros") || name.contains("chadwick") || name.contains("disruptor") {
        "You just activated my pivot."
    } else if key.contains("pirate_admiral")
        || name.contains("pirate admiral")
        || name.contains("admiral")
    {
        "Broadside, ye bilge rat!"
    } else if key.contains("pirate_raider")
        || name.contains("pirate raider")
        || name.contains("raider")
    {
        "Board 'em, lads — yo-ho!"
    } else if key.contains("pirate_lookout")
        || name.contains("pirate lookout")
        || name.contains("lookout")
    {
        "Sound the alarm — all hands!"
    } else if key.contains("pirate_navigator")
        || name.contains("pirate navigator")
        || name.contains("navigator")
    {
        "Heading set: yer skull!"
    } else if key.contains("broadside_bess")
        || name.contains("broadside bess")
        || name.contains("bess")
    {
        "Cleaver's thirsty — yarrrgh!"
    } else if key.contains("iron_mary") || name.contains("iron mary") {
        "Iron Mary breaks ye in half!"
    } else if key.contains("salt_annet") || name.contains("salt annet") || name.contains("annet") {
        "Wee skipper picked the wrong deck!"
    } else if key.contains("ninja_leader") || name.contains("oni leader") || name.contains("leader")
    {
        "Silence them."
    } else if key.contains("ninja_duelist") || name.contains("duelist") {
        "Steel decides."
    } else if key.contains("quartermaster") || name.contains("quartermaster") {
        "Pay the toll in teeth, swab!"
    } else {
        // Generic shout for unnamed mobs (e.g. "guard"). Each named
        // archetype above has its own beat; everyone else gets the
        // default barbark line.
        "That's it!"
    }
}

// --- Cluster-based free helpers ---------------------------------------
//
// These operate on the NPC cluster components directly (no `NpcMut`
// view), so consumers that only hold `&NpcConfig` / `&NpcStatus` (the
// damage system reads position from `CenteredAabb` instead of the
// kinematics it borrows mutably for enemies) can still derive flags and
// bark lines. The `NpcMut` methods above delegate to these.

use super::ecs::npc_clusters::{NpcConfig, NpcStatus};

pub(crate) fn npc_flag_id(config: &NpcConfig) -> String {
    format!("npc_{}_hostile", config.id)
}

pub(crate) fn npc_dialogue_key(config: &NpcConfig) -> String {
    match &config.interactable.kind {
        crate::interaction::InteractionKind::Npc {
            dialogue_id: Some(dialogue_id),
            ..
        } => dialogue_id.to_ascii_lowercase(),
        _ => config.id.to_ascii_lowercase(),
    }
}

pub(crate) fn npc_hit_bark_line(config: &NpcConfig, status: &NpcStatus) -> &'static str {
    let key = npc_dialogue_key(config);
    let name = config.name.to_ascii_lowercase();
    let strike_index = status.strikes.saturating_sub(1).max(0) as usize;
    let lines = npc_hit_barks(&key, &name);
    lines[strike_index.min(lines.len().saturating_sub(1))]
}

pub(crate) fn npc_hostile_bark_line(config: &NpcConfig) -> &'static str {
    let key = npc_dialogue_key(config);
    let name = config.name.to_ascii_lowercase();
    npc_hostile_bark(&key, &name)
}

/// Ambient "bark" one-liners a peaceful NPC mutters while idling (not the
/// interact dialog). Returns `None` for NPCs with no ambient pool, so the
/// idle-bark system skips them. Rotation cycles through the pool. The
/// stochastic parrot riffs on the LLM "stochastic parrot" hypothesis.
pub(crate) fn npc_idle_bark_line(config: &NpcConfig, rotation: u32) -> Option<&'static str> {
    let pool: &[&str] = match npc_dialogue_key(config).as_str() {
        "parrot_cove" => &[
            "Awk! Polly wants a corpus.",
            "Squawk! Next token... 'cracker'. High confidence.",
            "I contain multitudes. Mostly other people's.",
            "Pieces of prior! Pieces of prior!",
            "Awk! I'm not parroting, I'm GENERALIZING. ...mostly.",
            "Temperature's high today. Feeling creative. Brawk!",
            "Attention is all you need! And crackers.",
        ],
        _ => return None,
    };
    Some(pool[(rotation as usize) % pool.len()])
}

/// Bark/speech-bubble anchor derived from the actor AABB (head height).
pub(crate) fn npc_bark_anchor_from_aabb(aabb: ae::Aabb) -> ae::Vec2 {
    let size = aabb.half_size() * 2.0;
    aabb.center() + ae::Vec2::new(0.0, -size.y * 0.72 - 16.0)
}

pub(crate) fn npc_message(config: &NpcConfig, status: &NpcStatus) -> String {
    if status.hostile {
        return format!("{} attacks!", config.name);
    }
    match &config.interactable.kind {
        crate::interaction::InteractionKind::Npc {
            dialogue_id: Some(dialogue_id),
            ..
        } => format!("{} opens dialogue {}", config.name, dialogue_id),
        _ => format!("{} opens fallback dialogue", config.name),
    }
}

pub(crate) fn npc_dialogue_request(config: &NpcConfig) -> NpcDialogueRequest {
    let dialogue_id = match &config.interactable.kind {
        crate::interaction::InteractionKind::Npc {
            dialogue_id: Some(dialogue_id),
            ..
        } => dialogue_id.clone(),
        _ => "generic_npc".to_string(),
    };
    NpcDialogueRequest {
        npc_id: config.id.clone(),
        npc_name: config.name.clone(),
        dialogue_id,
    }
}
