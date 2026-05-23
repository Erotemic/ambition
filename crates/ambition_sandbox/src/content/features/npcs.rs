use super::*;

#[derive(Clone, Debug)]
pub struct NpcRuntime {
    pub id: String,
    pub name: String,
    pub pos: ae::Vec2,
    /// Authored spawn position. Patrol bounds are derived from this
    /// (`spawn.x ± patrol_radius`) so the NPC always paces around
    /// where the LDtk author placed them, not wherever they last
    /// stopped.
    pub spawn: ae::Vec2,
    pub size: ae::Vec2,
    /// Per-frame velocity. NPCs are physics-simulated like enemies
    /// (gravity + horizontal patrol step); previously they were
    /// static and floated wherever LDtk placed them.
    pub vel: ae::Vec2,
    /// +1 facing right, -1 facing left. Drives the patrol step
    /// direction and the sprite flip.
    pub facing: f32,
    pub on_ground: bool,
    pub interactable: ae::Interactable,
    /// Half-range of the fallback patrol pace, in world pixels. 0.0 → static
    /// unless a `motion` path was authored. > 0 → pace
    /// `[spawn.x - patrol_radius, spawn.x + patrol_radius]`. Mirror of the
    /// engine `InteractionKind::Npc::patrol_radius` — cached here so the
    /// per-frame movement code doesn't have to re-pattern-match every tick.
    pub patrol_radius: f32,
    /// Optional typed authored patrol path. When present this drives `Patrol`
    /// movement instead of the old radius pace.
    pub motion: Option<PathMotion>,
    /// Distance below which a patrolling NPC stops to face the
    /// player so dialog interaction is reachable. 0 disables the
    /// stop behavior. Sandbox-side default; not authored.
    pub talk_radius: f32,
    /// Last-evaluated `CharacterAiMode`. NPCs flex the engine's
    /// shared character_ai vocabulary: `Patrol` paces, `Chase`
    /// (player in talk range) HOLDS POSITION (semantically the
    /// inverse of an enemy "chase" — a peaceful NPC interrupts its
    /// own behavior to face the visitor), `Idle` is the
    /// no-patrol-radius fallback.
    pub ai_mode: ae::CharacterAiMode,
    /// Hostility flag. Becomes true after the player strikes the NPC
    /// enough times to provoke them. The save flag mirrors this so
    /// hostility persists across rooms / saves. ECS actor systems flip
    /// peaceful NPCs into hostile enemy disposition in place.
    pub hostile: bool,
    /// Hits the NPC has taken since the last reset. Crosses
    /// `HOSTILE_THRESHOLD` to flip hostile.
    pub strikes: i32,
    /// Brief flash after a strike — used by the renderer to flicker
    /// the NPC red without changing the dialog system.
    pub hit_flash: f32,
}

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
pub const NPC_PATROL_SPEED: f32 = 60.0;

impl NpcRuntime {
    #[cfg(test)]
    pub(super) fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: ae::Aabb,
        interactable: ae::Interactable,
    ) -> Self {
        Self::new_with_paths(id, name, aabb, interactable, &[])
    }

    pub(super) fn new_with_paths(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: ae::Aabb,
        interactable: ae::Interactable,
        paths: &[(String, ae::KinematicPath)],
    ) -> Self {
        let authored_pos = aabb.center();
        let (patrol_radius, motion) = match &interactable.kind {
            ae::InteractionKind::Npc {
                patrol_radius,
                patrol_path_id,
                ..
            } => {
                let motion = patrol_path_id.as_deref().and_then(|path_id| {
                    paths
                        .iter()
                        .find(|(p_id, _)| p_id == path_id)
                        .map(|(_, path)| PathMotion::new(path.clone()))
                });
                (patrol_radius.max(0.0), motion)
            }
            _ => (0.0, None),
        };
        let pos = motion
            .as_ref()
            .and_then(PathMotion::start_pos)
            .unwrap_or(authored_pos);
        Self {
            id: id.into(),
            name: name.into(),
            pos,
            spawn: pos,
            size: aabb.half_size() * 2.0,
            vel: ae::Vec2::ZERO,
            facing: 1.0,
            on_ground: false,
            interactable,
            patrol_radius,
            motion,
            talk_radius: NPC_TALK_RADIUS,
            ai_mode: ae::CharacterAiMode::Idle,
            hostile: false,
            strikes: 0,
            hit_flash: 0.0,
        }
    }

    pub fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.size * 0.5)
    }

    /// Per-frame physics + AI tick for an NPC.
    ///
    /// - Always: gravity + floor/wall collision (the bug the user
    ///   reported was that NPCs didn't fall onto the floor — they
    ///   just floated at their authored spawn).
    /// - If `patrol_radius > 0`: paces between `spawn.x ± radius`,
    ///   reversing facing on bounds + on horizontal collision.
    /// - If the player is within `talk_radius`: AI flips to `Chase`
    ///   (which for an NPC means STOP and face the player) so the
    ///   player can interact without chasing a moving target. The
    ///   `Chase` semantics here are inverse of the enemy path:
    ///   enemies pursue, NPCs hold. The shared
    ///   `evaluate_character_ai` evaluator just tells us "the
    ///   player is in range" — the per-actor caller decides what
    ///   that means in motion terms.
    /// `target_pos` is the per-frame "who is this NPC facing" position,
    /// populated from `ActorTarget` (OVERNIGHT-TODO #17.8). Peaceful
    /// NPCs use it both to detect "player in talk range" via
    /// `evaluate_character_ai` and to flip `facing` toward the player
    /// while in `Chase` mode so dialogue prompts render on the
    /// correct side of the NPC sprite.
    pub fn update(&mut self, world: &ae::World, target_pos: ae::Vec2, dt: f32) {
        self.hit_flash = (self.hit_flash - dt).max(0.0);

        // Re-evaluate AI mode each tick. We feed `talk_radius` as
        // the `aggro_radius` so "player in range" → `Chase` mode
        // (which the NPC interprets as "hold position"). Attack
        // ranges / windups all stay at zero — peaceful NPCs don't
        // attack. `patrol_enabled` toggles whether the
        // out-of-range fallback is `Patrol` or `Idle`.
        self.ai_mode = ae::evaluate_character_ai(ae::CharacterAiSnapshot {
            actor_pos: self.pos,
            player_pos: target_pos,
            aggro_radius: self.talk_radius,
            attack_range: 0.0,
            attack_windup_remaining: 0.0,
            attack_active_remaining: 0.0,
            attack_recover_remaining: 0.0,
            stun_remaining: 0.0,
            alive: true,
            patrol_enabled: self.motion.is_some() || self.patrol_radius > 0.0,
        });

        // Pick a horizontal target velocity from the AI mode. The
        // patrol step also flips `facing` on bound contact so the
        // NPC doesn't freeze when it reaches the patrol edge.
        let target_x = match self.ai_mode {
            // Player is close → stop and face the player so the
            // interact prompt lands. The actual facing flip
            // happens after movement so the NPC reads a fresh
            // delta this frame.
            ae::CharacterAiMode::Chase => 0.0,
            ae::CharacterAiMode::Patrol if self.motion.is_some() => 0.0,
            ae::CharacterAiMode::Patrol => {
                // Reverse at patrol bounds.
                let from_spawn = self.pos.x - self.spawn.x;
                if from_spawn > self.patrol_radius {
                    self.facing = -1.0;
                } else if from_spawn < -self.patrol_radius {
                    self.facing = 1.0;
                }
                self.facing * NPC_PATROL_SPEED
            }
            // Idle / Dead / Stunned / Telegraph / Attack / Recover
            // → no horizontal motion. (NPCs never enter Stunned /
            // attack states today; left here for completeness so
            // future patches can extend without re-shaping the
            // match.)
            _ => 0.0,
        };

        if matches!(self.ai_mode, ae::CharacterAiMode::Patrol) {
            if let Some(motion) = &mut self.motion {
                let old = self.pos;
                self.pos = motion.advance(self.pos, dt);
                let delta = self.pos - old;
                self.vel = if dt > 0.0 { delta / dt } else { ae::Vec2::ZERO };
                if delta.x.abs() > 0.001 {
                    self.facing = delta.x.signum();
                }
                return;
            }
        }

        // Velocity smoothing — same shape as EnemyRuntime so the
        // NPC accelerates / decelerates at a similar pace and the
        // patrol pacing reads as a deliberate gait.
        self.vel.x = approach(self.vel.x, target_x, 650.0 * dt);

        // Bridge into the engine's shared kinematic sweep so NPCs
        // hit the same OneWay / Solid / BlinkWall rules the player
        // does (the predecessor's per-NPC sweep had OneWay-as-wall
        // semantics that diverged from the player and broke
        // hostile-NPC chase paths). Peaceful NPCs never set
        // `drop_through` — they are not trying to navigate
        // vertically toward the player, so OneWay platforms stay as
        // floors.
        let mut body = ae::KinematicBody {
            pos: self.pos,
            vel: self.vel,
            size: self.size,
            on_ground: self.on_ground,
            facing: self.facing,
        };
        let prev_vel_x = body.vel.x;
        ae::step_kinematic(
            &mut body,
            world,
            ae::KinematicTuning {
                gravity: ENEMY_GRAVITY,
                max_fall_speed: ENEMY_MAX_FALL,
            },
            ae::KinematicInputs::default(),
            dt,
        );
        self.pos = body.pos;
        self.vel = body.vel;
        self.on_ground = body.on_ground;

        // Patrol-style facing flip when we hit a wall horizontally.
        if matches!(self.ai_mode, ae::CharacterAiMode::Patrol)
            && prev_vel_x.abs() > 1.0
            && self.vel.x.abs() < 0.01
        {
            self.facing *= -1.0;
        }

        // After moving, re-face the player while in talk range so
        // the dialog prompt sits on the correct side of the NPC.
        if matches!(self.ai_mode, ae::CharacterAiMode::Chase) {
            let dx = target_pos.x - self.pos.x;
            if dx.abs() > 4.0 {
                self.facing = dx.signum();
            }
        }
    }

    pub fn flag_id(&self) -> String {
        format!("npc_{}_hostile", self.id)
    }

    pub(super) fn bark_anchor(&self) -> ae::Vec2 {
        self.pos + ae::Vec2::new(0.0, -self.size.y * 0.72 - 16.0)
    }

    fn dialogue_key(&self) -> String {
        match &self.interactable.kind {
            ae::InteractionKind::Npc {
                dialogue_id: Some(dialogue_id),
                ..
            } => dialogue_id.to_ascii_lowercase(),
            _ => self.id.to_ascii_lowercase(),
        }
    }

    pub(super) fn hit_bark(&self) -> &'static str {
        let key = self.dialogue_key();
        let name = self.name.to_ascii_lowercase();
        let strike_index = self.strikes.saturating_sub(1).max(0) as usize;
        let lines = npc_hit_barks(&key, &name);
        lines[strike_index.min(lines.len().saturating_sub(1))]
    }

    pub(super) fn hostile_bark(&self) -> &'static str {
        let key = self.dialogue_key();
        let name = self.name.to_ascii_lowercase();
        npc_hostile_bark(&key, &name)
    }

    pub(super) fn message(&self) -> String {
        if self.hostile {
            return format!("{} attacks!", self.name);
        }
        match &self.interactable.kind {
            ae::InteractionKind::Npc {
                dialogue_id: Some(dialogue_id),
                ..
            } => {
                format!("{} opens dialogue {}", self.name, dialogue_id)
            }
            _ => format!("{} opens fallback dialogue", self.name),
        }
    }

    pub(super) fn dialogue_request(&self) -> NpcDialogueRequest {
        let dialogue_id = match &self.interactable.kind {
            ae::InteractionKind::Npc {
                dialogue_id: Some(dialogue_id),
                ..
            } => dialogue_id.clone(),
            _ => "generic_npc".to_string(),
        };
        NpcDialogueRequest {
            npc_id: self.id.clone(),
            npc_name: self.name.clone(),
            dialogue_id,
        }
    }

    // Hostile NPCs are converted to `EnemyRuntime` instances in
    // `apply_save`. The legacy `hostile_damage` body-volume method
    // was removed because the spawned enemy now handles contact
    // damage through the standard `EnemyRuntime::player_damage`.
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
    } else if key.contains("broadside_bess") || name.contains("broadside bess") || name.contains("bess") {
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
