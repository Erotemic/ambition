use super::*;
use super::ecs::npc_clusters::NpcMut;


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

/// Patrol speed for NPCs. Slightly slower than the standard enemy
/// patrol speed so peaceful NPCs read as casual rather than alert.
/// Consumed by `crate::brain::PatrolCfg::NPC_DEFAULT` (the
/// brain-side mirror); kept here so the legacy NPC path that
/// hasn't migrated yet (none today, but the player polarity flip
/// preserves the value) stays in sync.
pub const NPC_PATROL_SPEED: f32 = 60.0;


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
        // World gravity sign (+1 down / -1 up) so NPCs fall the way the player
        // does when gravity flips.
        gravity_sign: f32,
    ) -> crate::actor_control::ActorControlFrame {
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
        let mut frame = crate::actor_control::ActorControlFrame::neutral();
        brain.tick(&snapshot, &mut frame);

        self.status.ai_mode = match brain {
            crate::brain::Brain::StateMachine(crate::brain::StateMachineCfg::Patrol {
                state,
                ..
            }) => state.mode,
            crate::brain::Brain::StateMachine(crate::brain::StateMachineCfg::StandStill) => {
                crate::character_ai::CharacterAiMode::Idle
            }
            _ => crate::character_ai::CharacterAiMode::Idle,
        };

        if frame.facing.abs() > 0.001 {
            self.kin.facing = frame.facing;
        }

        if matches!(self.status.ai_mode, crate::character_ai::CharacterAiMode::Patrol) {
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

        let target_x = frame.desired_vel.x;
        self.kin.vel.x = approach(self.kin.vel.x, target_x, 650.0 * dt);

        let mut body = crate::kinematic::KinematicBody {
            pos: self.kin.pos,
            vel: self.kin.vel,
            size: self.kin.size,
            on_ground: self.surface.on_ground,
            facing: self.kin.facing,
        };
        let prev_vel_x = body.vel.x;
        crate::kinematic::step_kinematic(
            &mut body,
            world,
            crate::kinematic::KinematicTuning {
                gravity: ENEMY_GRAVITY,
                max_fall_speed: ENEMY_MAX_FALL,
                gravity_sign,
            },
            crate::kinematic::KinematicInputs::default(),
            dt,
        );
        self.kin.pos = body.pos;
        self.kin.vel = body.vel;
        self.surface.on_ground = body.on_ground;

        if matches!(self.status.ai_mode, crate::character_ai::CharacterAiMode::Patrol)
            && prev_vel_x.abs() > 1.0
            && self.kin.vel.x.abs() < 0.01
        {
            self.kin.facing *= -1.0;
        }

        if matches!(self.status.ai_mode, crate::character_ai::CharacterAiMode::Chase) {
            let dx = target_pos.x - self.kin.pos.x;
            if dx.abs() > 4.0 {
                self.kin.facing = dx.signum();
            }
        }
        frame
    }

    pub fn build_brain(&self) -> crate::brain::Brain {
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
// damage system reads position from `FeatureAabb` instead of the
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
