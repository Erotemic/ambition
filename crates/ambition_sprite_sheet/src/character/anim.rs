//! Animation enum + per-actor animation pickers.
//!
//! `CharacterAnim` is the union of every animation row a character
//! sheet may define; the boss has its own row set, see
//! `boss_encounter::sprites::BossAnim`. A sheet doesn't have to define every
//! row — `CharacterSheetSpec::resolve_anim` falls back to `Idle` for
//! any row a sheet doesn't carry, so simple characters can list only
//! their relevant animations.

/// Animation ids that a character sheet may define.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharacterAnim {
    Idle = 0,
    Walk = 1,
    Run = 2,
    Jump = 3,
    Fall = 4,
    Slash = 5,
    Hit = 6,
    Death = 7,
    BlinkOut = 8,
    BlinkIn = 9,
    Dash = 10,
    /// Free-flight pose (jets / hover). Maps to the generator's
    /// `hover` row — the row we emit when the robot config lists
    /// `hover` after `dash`.
    Fly = 11,
    /// Idle-variant gesture for hostile NPCs (pirate admiral / raider
    /// generators emit a `taunt` row between `slash` and `hurt`).
    /// Not currently produced by `pick_*_anim` — the row exists so
    /// atlas indexing aligns with the PNG even when nothing requests
    /// it, and so future combat-banter systems can pick it up.
    Taunt = 12,
    /// Held hang on a ledge — both arms gripping the ledge top with
    /// the body slumped below. Driven by `pick_player_anim` while
    /// `Player::ledge_grab` is `Some` and not climbing.
    LedgeGrab = 13,
    /// Slow, deliberate "haul-yourself-up" loop against an overhead grip.
    /// The new robot sheet ships a dedicated row; `pick_player_anim` does
    /// not currently auto-route to it (mantle pop-ups go through
    /// `LedgeGetup` instead), but the variant exists so future climb
    /// gestures can request it.
    LedgeClimb = 14,
    /// Mantle pop-up: arms transition from overhead grip to planted push
    /// to standing in one short burst. Driven by `pick_player_anim` when
    /// `Player::ledge_grab.climbing == true`.
    LedgeGetup = 15,
    /// Pinned against a wall (both hands flat). Distinct from `wall_slide`
    /// (which is the engine's downward-scrape state) — `WallGrab` plays
    /// when the player is wall-clinging but not sliding/climbing.
    WallGrab = 16,
    /// Sustained held-jump glide pose (arms out as balance wings).
    /// Driven by `player.gliding`; distinct from `Fly` (rocket jets) and
    /// the airborne `Fall` row.
    FloatGlide = 17,
    /// Heavy landing — big squash, slow rebound. Triggered when the
    /// landing transition was hit at a high downward speed; consumed by
    /// `pick_player_anim` while `BodyAnimFacts::land_anim_timer` is
    /// positive and `land_anim_hard` is set.
    LandHard = 18,
    /// Rising recovery after a (hard) landing. Plays when
    /// `land_anim_timer` is positive but `land_anim_hard` is false, or
    /// during the tail of a hard landing.
    LandRecovery = 19,
    /// Brief animation-only dash pre-roll. Plays for the first ~50ms
    /// after a dash starts so the sprite has a discrete wind-up read
    /// before falling through to the streaking `Dash` row.
    DashStartup = 20,
    /// Grounded side-slash (Marth-style horizontal swing). Drives both
    /// the `Forward` / `Neutral` / `Back` / `DashForward` / `WallOut`
    /// engine attack intents — the four variants share one swing read
    /// since the sprite already flips with the player's facing.
    AttackSide = 21,
    /// Grounded up-tilt — overhead arc.
    AttackUp = 22,
    /// Grounded down-tilt — sweep down to the floor.
    AttackDown = 23,
    /// Aerial neutral spin-slash. No `AttackIntent::AirNeutral` exists
    /// yet, so this row is currently selected by `pick_player_anim` only
    /// when the future intent appears; the row is on the sheet so
    /// designers can iterate the shape regardless.
    AirNeutral = 24,
    /// Aerial forward swing.
    AirForward = 25,
    /// Aerial backward swing (no engine intent yet — placeholder row).
    AirBack = 26,
    /// Aerial down-thrust (spike).
    AirDown = 27,
    /// Aerial up-thrust.
    AirUp = 28,
    /// Smash-Bros style ledge roll: tumble onto the platform with
    /// invulnerability frames. Selected by `pick_player_anim` when
    /// `ledge_grab.climbing && getup_kind == Roll`.
    LedgeRoll = 29,
    /// Ledge getup attack: swing onto the platform with an active
    /// hitbox. Selected by `pick_player_anim` when
    /// `ledge_grab.climbing && getup_kind == Attack`. The slash op
    /// fires at the start of the transition (engine side); the
    /// sprite should peak the swing mid-animation so visual + hitbox
    /// read as a single beat.
    LedgeGetupAttack = 30,
    /// Compressed crouch pose. Selected by `pick_player_anim` while
    /// `body_mode.body_mode == BodyMode::Crouching` and the player is
    /// not actively walking. Matches the generator's `crouch` row.
    Crouch = 31,
    /// Hands-and-knees crawl. Selected while
    /// `body_mode.body_mode == BodyMode::Crawling`. Matches the
    /// generator's `crouch_walk` row (the renderer reuses the crouch
    /// silhouette with a longer stride).
    Crawl = 32,
    /// Forward slide along the ground (low profile, momentum-carrying).
    /// Selected while `body_mode.body_mode == BodyMode::Sliding`.
    /// Matches the generator's `slide` row.
    Slide = 33,
    /// Ladder / vine climb. Selected while
    /// `body_mode.body_mode == BodyMode::Climbing`. Maps to the
    /// generator's `climb` row (one hand over the other).
    LadderClimb = 34,
    /// Submerged swim stroke. Selected while the player is in water
    /// (`env_contact.water.is_some()`) and the `swim` ability is
    /// enabled. Maps to the generator's `swim` row.
    Swim = 35,
    /// Projectile fire pose — single-frame arm extension at the
    /// release point. Generator row `shoot`. Not yet auto-routed by
    /// `pick_player_anim`; needs a `shoot_anim_timer` on
    /// `BodyAnimFacts` set when a projectile spawns.
    Shoot = 36,
    /// Held-projectile charge / aim pose. Generator row `aim`. Not
    /// yet auto-routed; needs the projectile charge state on the
    /// player to surface as a presentation flag.
    Aim = 37,
    /// Held-attack charge pose (heavy/release combo wind-up).
    /// Generator row `charge`. No engine intent maps to this today;
    /// the row exists so designers can iterate the wind-up shape.
    Charge = 38,
    /// Defensive bubble / shield-up pose. Generator row `block`.
    /// Routes from the player's shield-held state once that's mirrored
    /// onto `BodyAnimFacts` (today the input is `ControlFrame.
    /// shield_held` + `AbilitySet::shield`).
    Block = 39,
    /// Tumbling dodge roll across the ground — invulnerability frames
    /// during the curl. Generator row `roll`. Selected by
    /// `pick_player_anim` once the dodge-roll timer surfaces on
    /// `BodyAnimFacts`; distinct from `LedgeRoll` (the latter is
    /// specifically the ledge-getup variant).
    DodgeRoll = 40,
    /// Wall-jump push-off pose. Generator row `wall_jump`. Distinct
    /// from `Jump` (which is the grounded jump arc) and `WallGrab`
    /// (which is the cling pose). Not yet auto-routed; needs a brief
    /// `wall_jump_anim_timer` armed by the wall-jump op.
    WallJump = 41,
    /// Interaction gesture (talk / open / pickup). Generator row
    /// `interact`. Not yet auto-routed; needs an `interact_anim_timer`
    /// armed by the interact buffer firing.
    Interact = 42,
    /// Committal heavy melee (generator row `punch`) — distinct from the
    /// quick `jab` (which aliases to `Slash`). The Perfect Cell-ular
    /// Automaton sheet ships both; the mechanical fast/heavy distinction
    /// lives in the actor's `ActionSet` melee spec, while the sprite
    /// distinguishes the two reads. Not yet auto-routed: `BodyMelee`/`AttackSpec`
    /// carry the swing's directional intent but no per-swing heavy tag, so the
    /// picker reads the directional swing (→ `Slash` after the anim-set walk for a
    /// `jab`-only sheet). The row lives on the sheet so a future heavy melee verb
    /// lights it up with zero picker change.
    Punch = 43,
    /// Charge→thrust "special" pose (kamehameha-style wind-up + release).
    /// Generator row `special`. Drives the glider zoning verb. Not yet
    /// auto-routed: no actor cluster carries a "special active" flag today (the
    /// brain emits the verb as a one-shot `ActorActionMessage`); the `special`
    /// field on [`BodyAnimView`] is wired into the ladder so the moment that state
    /// surfaces on a cluster the picker reads it.
    Special = 44,
    /// Persistent curled-ball locomotion — a body that IS a rolling ball for as
    /// long as some verb holds it there (spin dash roll, morph ball). Generator
    /// row `ball`. LOOPS, which is what distinguishes it from [`Self::DodgeRoll`]
    /// (a one-shot tumble that holds its final frame): a Sonic-style ball keeps
    /// spinning until the body stands back up. Selected while
    /// `BodyAnimFacts::rolling` is set.
    Roll = 45,
    /// Grounded braking against travel — running one way while steering the
    /// other. Generator row `skid`. Selected while `BodyMotionFacts::skidding`
    /// is published (the surface-momentum integration owns the read).
    Skid = 46,
}

impl CharacterAnim {
    /// Map a generator-emitted row name (e.g. the lowercase strings in
    /// `*_spritesheet.ron`'s `rows[*].animation` field) to its enum
    /// variant. Returns `None` for names the runtime doesn't have a
    /// variant for — the row is silently dropped from the sheet spec.
    ///
    /// Accepted aliases:
    /// - `hurt` ↔ `Hit` (the goblin / pirate generators emit `hurt`,
    ///   but the runtime ECS animation picker uses `Hit`).
    /// - `hover` ↔ `Fly` (robot generator emits `hover` for the
    ///   jet-flight pose).
    /// - `opening` ↔ `Idle`, `stable` / `spin` ↔ `Walk`,
    ///   `closing` ↔ `Run` (interdimensional gate portal / ring sheets
    ///   borrow `CharacterAnim` slots for their phase-machine rows;
    ///   see `GATE_PORTAL_SHEET` / `GATE_RING_SHEET` docstrings in
    ///   `sheets.rs` for the runtime mapping).
    pub fn from_name(name: &str) -> Option<Self> {
        // Lowercase + strip nothing; we want exact matches against the
        // generator output strings.
        Some(match name {
            // `rest` (boss-encounter sheets), `front_idle` / `side_idle`
            // (girdle's facing-split sheet) — alias to Idle so the
            // catalog can pull every character in. A fully typed
            // CharacterAnim::Rest can land later if a consumer
            // distinguishes them.
            "idle" | "opening" | "rest" | "front_idle" | "side_idle" | "classic_burst" => {
                Self::Idle
            }
            "walk" | "stable" | "spin" | "side_walk" | "burst_round" => Self::Walk,
            "run" | "closing" | "shockwave" => Self::Run,
            "jump" => Self::Jump,
            "fall" => Self::Fall,
            // `jab` is the quick poke; it shares the generic `Slash` read.
            // `punch` is the committal heavy with its own row.
            "slash" | "starburst" | "jab" => Self::Slash,
            "punch" => Self::Punch,
            // Charge→thrust special (glider release). Distinct from `Charge`
            // (the held wind-up only) — `special` is the full beat.
            "special" => Self::Special,
            "hit" | "hurt" | "smoke_burst" => Self::Hit,
            "death" => Self::Death,
            "blink_out" => Self::BlinkOut,
            "blink_in" => Self::BlinkIn,
            "dash" => Self::Dash,
            "fly" | "hover" => Self::Fly,
            "taunt" => Self::Taunt,
            "ledge_grab" => Self::LedgeGrab,
            "ledge_climb" => Self::LedgeClimb,
            "ledge_getup" => Self::LedgeGetup,
            "wall_grab" => Self::WallGrab,
            "float_glide" => Self::FloatGlide,
            "land_hard" => Self::LandHard,
            "land_recovery" => Self::LandRecovery,
            "dash_startup" => Self::DashStartup,
            "attack_side" => Self::AttackSide,
            "attack_up" => Self::AttackUp,
            "attack_down" => Self::AttackDown,
            "air_neutral" => Self::AirNeutral,
            "air_forward" => Self::AirForward,
            "air_back" => Self::AirBack,
            "air_down" => Self::AirDown,
            "air_up" => Self::AirUp,
            "ledge_roll" => Self::LedgeRoll,
            "ledge_getup_attack" => Self::LedgeGetupAttack,
            "crouch" => Self::Crouch,
            // The generator emits `crouch_walk` for the crawl pose.
            "crouch_walk" | "crawl" => Self::Crawl,
            "slide" => Self::Slide,
            "climb" | "ladder_climb" => Self::LadderClimb,
            "swim" => Self::Swim,
            "shoot" => Self::Shoot,
            "aim" => Self::Aim,
            "charge" => Self::Charge,
            "block" | "shield" => Self::Block,
            "roll" | "dodge_roll" => Self::DodgeRoll,
            // `ball` is the LOOPING curl (spin dash / morph ball); `roll` stays
            // the one-shot dodge tumble so existing sheets keep their read.
            "ball" => Self::Roll,
            "skid" => Self::Skid,
            "wall_jump" => Self::WallJump,
            "interact" => Self::Interact,
            _ => return None,
        })
    }

    /// The next *less-specific* pose in the same family — the fixed structural
    /// shape of the pose space (`AttackUp` is a refinement of `AttackSide` is a
    /// refinement of `Slash`; `Dash`→`Run`→`Walk`; airborne→`Fall`). `None` once
    /// `Idle` is the floor.
    ///
    /// This is NOT a list of who-falls-back-to-what authored by hand, and it is
    /// NOT a second source of truth about which poses an actor *has* — that's the
    /// actor's **anim set**, the rows the sprite generator wrote into the
    /// manifest RON ([`CharacterSheetSpec::maps`]). [`CharacterSheetSpec::
    /// resolve_anim`] walks this taxonomy to render the most-specific pose the
    /// actor's set actually contains: an actor whose sheet only drew `slash`
    /// shows `slash` for an up-tilt; author an `attack_up` row and up-tilts read
    /// distinctly, with zero code change. Expressiveness is opt-in per sheet;
    /// a lean set never snaps to `Idle`.
    pub fn base_pose(self) -> Option<Self> {
        use CharacterAnim::*;
        Some(match self {
            Idle => return None,
            // Ground locomotion.
            Walk => Idle,
            Run => Walk,
            Crouch => Idle,
            Crawl => Walk,
            Slide => Run,
            // Air.
            Jump => Fall,
            Fall => Idle,
            FloatGlide => Fall,
            Fly => Idle,
            WallJump => Jump,
            WallGrab => Idle,
            // Dash.
            DashStartup => Dash,
            Dash => Run,
            // Melee — the directional / aerial swings are refinements of the
            // side swing, then the generic slash.
            AttackUp => AttackSide,
            AttackDown => AttackSide,
            AttackSide => Slash,
            AirNeutral => Slash,
            AirForward => AttackSide,
            AirBack => AttackSide,
            AirUp => AttackUp,
            AirDown => AttackDown,
            Punch => Slash,
            Special => Slash,
            Slash => Idle,
            LedgeGetupAttack => LedgeGetup,
            // Ranged / charge.
            Shoot => Idle,
            Charge => Aim,
            Aim => Idle,
            // Defensive / utility.
            Block => Idle,
            // A sheet without a looping ball still shows its curl for the
            // persistent roll (held final tumble frame beats a standing run).
            Roll => DodgeRoll,
            Skid => Run,
            DodgeRoll => Idle,
            Interact => Idle,
            Swim => Idle,
            LadderClimb => Idle,
            // Ledge.
            LedgeClimb => LedgeGrab,
            LedgeGetup => LedgeGrab,
            LedgeRoll => DodgeRoll,
            LedgeGrab => Idle,
            // Blink.
            BlinkIn => Idle,
            BlinkOut => Idle,
            // Reactions.
            Death => Hit,
            Hit => Idle,
            LandHard => LandRecovery,
            LandRecovery => Idle,
            // Idle-variant gesture.
            Taunt => Idle,
        })
    }
}

pub fn non_looping(anim: CharacterAnim) -> bool {
    matches!(
        anim,
        CharacterAnim::Slash
            | CharacterAnim::Hit
            | CharacterAnim::Death
            | CharacterAnim::LedgeClimb
            | CharacterAnim::LedgeGetup
            | CharacterAnim::LedgeRoll
            | CharacterAnim::LedgeGetupAttack
            | CharacterAnim::LandHard
            | CharacterAnim::LandRecovery
            | CharacterAnim::DashStartup
            | CharacterAnim::AttackSide
            | CharacterAnim::AttackUp
            | CharacterAnim::AttackDown
            | CharacterAnim::AirNeutral
            | CharacterAnim::AirForward
            | CharacterAnim::AirBack
            | CharacterAnim::AirDown
            | CharacterAnim::AirUp
            // New action poses: one-shot reads that should hold the
            // final frame instead of looping back.
            | CharacterAnim::Shoot
            | CharacterAnim::DodgeRoll
            | CharacterAnim::WallJump
            | CharacterAnim::Interact
            | CharacterAnim::Punch
            | CharacterAnim::Special
    )
}
