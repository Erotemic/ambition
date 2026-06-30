//! Animation enum + per-actor animation pickers.
//!
//! `CharacterAnim` is the union of every animation row a character
//! sheet may define; the boss has its own row set, see
//! `boss_encounter::sprites::BossAnim`. A sheet doesn't have to define every
//! row — `CharacterSheetSpec::resolve_anim` falls back to `Idle` for
//! any row a sheet doesn't carry, so simple characters can list only
//! their relevant animations.

use ambition_engine_core as ae;

use crate::player::PlayerAnimState;

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
    /// `pick_player_anim` while `PlayerAnimState::land_anim_timer` is
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
    /// `PlayerAnimState` set when a projectile spawns.
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
    /// onto `PlayerAnimState` (today the input is `ControlFrame.
    /// shield_held` + `AbilitySet::shield`).
    Block = 39,
    /// Tumbling dodge roll across the ground — invulnerability frames
    /// during the curl. Generator row `roll`. Selected by
    /// `pick_player_anim` once the dodge-roll timer surfaces on
    /// `PlayerAnimState`; distinct from `LedgeRoll` (the latter is
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

pub(super) fn non_looping(anim: CharacterAnim) -> bool {
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

/// Pick the player's animation from ECS animation state and engine state.
///
/// Priority: hit > blink-in/out > slash > fly > dash > airborne (jump/fall) > run/walk/idle.
/// Free-flight overrides ground/airborne motion because the engine
/// integrator already disables gravity in flight; the visual should
/// reflect the active mode rather than whatever fall/run inertia
/// happens to read.
/// Death is not represented yet — the player respawns instantly today.
/// `BlinkOut` is used while the blink button is held/aiming, and
/// `BlinkIn` is held briefly after a committed blink so VFX/camera have
/// time to sell the arrival.
///
/// `anim` is the authoritative ECS component for presentation timers.
/// `combat` provides `hitstun_timer` (now on `BodyCombat`).
/// `blink_cam` provides `blink_in_timer` (now on `PlayerBlinkCameraState`).
/// `attack` is the active swing from `player::BodyMelee`; `None` when idle.
///
/// Phase 2 migration: the remaining player state (velocity, ground,
/// wall, blink/aim, flight, dash, ledge) comes in as five cluster
/// component references so this helper has no dependency on the
/// legacy `ae::Player` aggregate.
/// Compact-body silhouette mode the picker reads at low priority (mirrors the
/// engine `BodyMode` subset that has its own sprite row).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CompactBody {
    #[default]
    None,
    Slide,
    Crawl,
    Crouch,
}

/// A resolved ledge read (already mapped from hang/getup-kind to its row).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LedgeRead {
    Grab,
    Getup,
    Roll,
    GetupAttack,
}

impl LedgeRead {
    fn anim(self) -> CharacterAnim {
        match self {
            LedgeRead::Grab => CharacterAnim::LedgeGrab,
            LedgeRead::Getup => CharacterAnim::LedgeGetup,
            LedgeRead::Roll => CharacterAnim::LedgeRoll,
            LedgeRead::GetupAttack => CharacterAnim::LedgeGetupAttack,
        }
    }
}

/// Whether the locomotion tail reads as a ground walker or an aerial flyer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Locomotion {
    #[default]
    Grounded,
    Aerial,
}

/// Archetype-agnostic animation facts: everything the ONE anim-priority ladder
/// ([`pick_body_anim`]) reads, in no particular field order. A player fills the
/// rich set from its `Body*` clusters; an enemy/NPC fills a sparse set (the
/// states it can't be in stay at their inert defaults). So an actor "rises" to a
/// richer animation as it gains state, instead of each archetype carrying its
/// own priority ladder — the relativity principle, applied to presentation.
///
/// Built per frame by [`pick_player_anim`] / [`pick_actor_anim`] (the thin
/// per-body adapters), which is also where the per-body quirks live: the attack
/// row a swing maps to, the locomotion speed metric (|vx| grounded vs |v|
/// aerial), and the speed thresholds.
#[derive(Clone, Copy, Debug, Default)]
pub struct BodyAnimView {
    pub dead: bool,
    pub hit: bool,
    pub dodge_roll: bool,
    pub blink_in: bool,
    pub blocking: bool,
    /// Actor charge→thrust special (glider zoning); highest combat read after hit.
    pub special: bool,
    pub shooting: bool,
    /// The melee row to play while mid-swing (directional for the player,
    /// Punch/Slash for actors). `None` ⇒ not attacking.
    pub melee_attack: Option<CharacterAnim>,
    pub aiming: bool,
    pub wall_jump: bool,
    pub interacting: bool,
    pub blink_out: bool,
    pub ledge: Option<LedgeRead>,
    pub flying: bool,
    pub swimming: bool,
    pub dash_startup: bool,
    pub dashing: bool,
    pub ladder_climbing: bool,
    pub wall_grab: bool,
    pub gliding: bool,
    pub airborne: bool,
    /// Only read while `airborne`: up ⇒ Jump, else Fall.
    pub moving_up: bool,
    /// `Some(hard)` while a landing-recovery pose is held (grounded only).
    pub landing: Option<bool>,
    pub compact: CompactBody,
    pub locomotion: Locomotion,
    /// Locomotion speed in the metric the style uses (|vx| grounded, |v| aerial).
    pub speed: f32,
    /// Grounded: `speed < idle_below` ⇒ Idle.
    pub idle_below: f32,
    /// Grounded: `Some(t)` ⇒ `speed >= t` is Run; `None` ⇒ caps at Walk.
    pub run_above: Option<f32>,
    /// Aerial: `speed > fly_above` ⇒ Fly, else Idle (hover).
    pub fly_above: f32,
}

/// The single animation-priority ladder every body runs. Each archetype's
/// adapter builds a [`BodyAnimView`] and calls this; the ordering here is the
/// player's full ladder, with the actor-only reads (`dead`, `special`, the
/// Punch swing row) folded in at their priorities. Inert states (a `None`
/// ledge, `false` flags) fall straight through, so a sparse actor view lands on
/// the shared locomotion tail.
pub fn pick_body_anim(v: &BodyAnimView) -> CharacterAnim {
    use CharacterAnim::*;
    if v.dead {
        return Death;
    }
    if v.hit {
        return Hit;
    }
    if v.dodge_roll {
        return DodgeRoll;
    }
    if v.blink_in {
        return BlinkIn;
    }
    if v.blocking {
        return Block;
    }
    if v.special {
        return Special;
    }
    if v.shooting {
        return Shoot;
    }
    if let Some(swing) = v.melee_attack {
        return swing;
    }
    if v.aiming {
        return Aim;
    }
    if v.wall_jump {
        return WallJump;
    }
    if v.interacting {
        return Interact;
    }
    if v.blink_out {
        return BlinkOut;
    }
    if let Some(ledge) = v.ledge {
        return ledge.anim();
    }
    if v.flying {
        return Fly;
    }
    if v.swimming {
        return Swim;
    }
    if v.dash_startup {
        return DashStartup;
    }
    if v.dashing {
        return Dash;
    }
    if v.ladder_climbing {
        return LadderClimb;
    }
    if v.wall_grab {
        return WallGrab;
    }
    if v.gliding {
        return FloatGlide;
    }
    if v.airborne {
        return if v.moving_up { Jump } else { Fall };
    }
    if let Some(hard) = v.landing {
        return if hard { LandHard } else { LandRecovery };
    }
    match v.compact {
        CompactBody::Slide => return Slide,
        CompactBody::Crawl => return Crawl,
        CompactBody::Crouch => return Crouch,
        CompactBody::None => {}
    }
    match v.locomotion {
        Locomotion::Aerial => {
            if v.speed > v.fly_above {
                Fly
            } else {
                Idle
            }
        }
        Locomotion::Grounded => {
            if v.speed < v.idle_below {
                Idle
            } else if v.run_above.map_or(true, |t| v.speed < t) {
                Walk
            } else {
                Run
            }
        }
    }
}

/// Resolve a [`BodyLedgeState`] into the visual ledge read (`None` ⇒ not on a
/// ledge): a held hang is `Grab`; once committed the getup-kind selects the
/// climb / roll / attack getup. SHARED by every body — the player and any actor
/// that grows a ledge-grab limb route through this one mapping.
fn ledge_read(ledge: &crate::actor::BodyLedgeState) -> Option<LedgeRead> {
    ledge.grab.as_ref().map(|s| {
        if !s.climbing {
            LedgeRead::Grab
        } else {
            match s.getup_kind {
                ae::LedgeGetupKind::Climb => LedgeRead::Getup,
                ae::LedgeGetupKind::Roll => LedgeRead::Roll,
                ae::LedgeGetupKind::Attack => LedgeRead::GetupAttack,
            }
        }
    })
}

/// Map the engine `BodyMode` subset that owns a compact-silhouette sprite row.
fn compact_from_mode(mode: ambition_engine_core::player_state::BodyMode) -> CompactBody {
    use ambition_engine_core::player_state::BodyMode;
    match mode {
        BodyMode::Sliding => CompactBody::Slide,
        BodyMode::Crawling => CompactBody::Crawl,
        BodyMode::Crouching => CompactBody::Crouch,
        _ => CompactBody::None,
    }
}

/// Fill every [`BodyAnimView`] field that is derived purely from the shared
/// `Body*` movement/ability clusters — the reads that are IDENTICAL for every
/// body, player or brain-driven actor. This is the convergence seam: whatever
/// state a body's brain drives its real clusters into (a dash, a blink, flight,
/// a shield, a ladder climb, a wall-grab, a crouch/slide) animates the same way
/// for everyone, because everyone reads it here.
///
/// The per-archetype adapter ([`pick_player_anim`] / [`pick_actor_anim`]) then
/// overlays the fields this builder deliberately leaves at their inert defaults:
/// the `dead` / `hit` source (player combat-cluster vs actor status), the melee
/// row, the presentation-timer reads the player feeds itself (shoot / aim /
/// wall-jump / interact / dash-startup / landing / blink-in), and the locomotion
/// metric + speed thresholds. `speed` is seeded with the grounded metric
/// (`|vx|`); an aerial adapter overrides it with total speed.
pub fn body_view_from_clusters(
    kinematics: &crate::actor::BodyKinematics,
    ground: &crate::actor::BodyGroundState,
    wall: &crate::actor::BodyWallState,
    blink: &crate::actor::BodyBlinkState,
    flight: &crate::actor::BodyFlightState,
    dash: &crate::actor::BodyDashState,
    ledge: &crate::actor::BodyLedgeState,
    body_mode: &crate::actor::BodyModeState,
    env_contact: &crate::actor::BodyEnvironmentContact,
    abilities: &crate::actor::BodyAbilities,
    dodge: &crate::actor::BodyDodgeState,
    shield: &crate::actor::BodyShieldState,
) -> BodyAnimView {
    use ambition_engine_core::player_state::BodyMode;
    BodyAnimView {
        // The dodge↔ledge guard: a roll that is part of a ledge getup keeps the
        // dedicated `LedgeRoll` row instead of the grounded `DodgeRoll`.
        dodge_roll: dodge.roll_timer > 0.0 && ledge.grab.is_none(),
        blocking: shield.active && abilities.abilities.shield,
        blink_out: blink.aiming || blink.hold_active,
        ledge: ledge_read(ledge),
        flying: flight.fly_enabled,
        swimming: env_contact.water.is_some() && abilities.abilities.swim,
        dashing: dash.timer > 0.0,
        // High-priority climb (ladder/vine) vs the low-priority compact silhouette
        // (slide/crawl/crouch) are distinct fields checked at distinct priorities.
        ladder_climbing: matches!(body_mode.body_mode, BodyMode::Climbing),
        wall_grab: !ground.on_ground
            && wall.wall_clinging
            && !wall.wall_climbing
            && kinematics.vel.y.abs() < 40.0,
        gliding: flight.gliding,
        airborne: !ground.on_ground,
        moving_up: kinematics.vel.y < -10.0, // top-left coords: vel.y < 0 = up
        compact: compact_from_mode(body_mode.body_mode),
        speed: kinematics.vel.x.abs(),
        ..Default::default()
    }
}

#[allow(clippy::too_many_arguments)]
pub fn pick_player_anim(
    anim: &PlayerAnimState,
    combat: &crate::actor::BodyCombat,
    blink_cam: &crate::player::PlayerBlinkCameraState,
    attack: Option<&crate::MeleeSwing>,
    kinematics: &crate::actor::BodyKinematics,
    ground: &crate::actor::BodyGroundState,
    wall: &crate::actor::BodyWallState,
    blink: &crate::actor::BodyBlinkState,
    flight: &crate::actor::BodyFlightState,
    dash: &crate::actor::BodyDashState,
    ledge: &crate::actor::BodyLedgeState,
    body_mode: &crate::actor::BodyModeState,
    env_contact: &crate::actor::BodyEnvironmentContact,
    abilities: &crate::actor::BodyAbilities,
    dodge: &crate::actor::BodyDodgeState,
    shield: &crate::actor::BodyShieldState,
) -> CharacterAnim {
    // Movement/ability fields come from the shared cluster builder (identical to
    // every actor); the player overlays its combat-cluster hit read, its own
    // presentation-timer reads, and its grounded thresholds. Each line below is
    // the exact predicate the old per-branch ladder used.
    let mut v = body_view_from_clusters(
        kinematics,
        ground,
        wall,
        blink,
        flight,
        dash,
        ledge,
        body_mode,
        env_contact,
        abilities,
        dodge,
        shield,
    );
    v.hit = combat.hitstun_timer > 0.05;
    v.blink_in = blink_cam.blink_in_timer > 0.0;
    v.shooting = anim.shoot_anim_timer > 0.0;
    v.melee_attack = (anim.slash_anim_timer > 0.0).then(|| directional_attack_anim(attack));
    v.aiming = anim.aim_anim_active;
    v.wall_jump = anim.wall_jump_anim_timer > 0.0;
    v.interacting = anim.interact_anim_timer > 0.0;
    v.dash_startup = anim.dash_startup_timer > 0.0;
    v.landing = (anim.land_anim_timer > 0.0).then_some(anim.land_anim_hard);
    v.idle_below = 12.0;
    v.run_above = Some(220.0);
    v.fly_above = 0.0;
    pick_body_anim(&v)
}

/// Map the active player attack intent onto the directional swing rows.
///
/// The engine's `AttackIntent` is finer-grained than the visible swing
/// shapes — multiple intents share one row because the sprite already
/// flips with the player's facing.
fn directional_attack_anim(attack: Option<&crate::MeleeSwing>) -> CharacterAnim {
    use crate::combat::AttackIntent;
    let Some(attack) = attack else {
        // Defensive fallback: slash_anim_timer is set but no attack
        // state — keep the old side-swing read until the timer drains.
        return CharacterAnim::AttackSide;
    };
    match attack.spec.intent {
        AttackIntent::Up => CharacterAnim::AttackUp,
        AttackIntent::Down => CharacterAnim::AttackDown,
        AttackIntent::AirUp => CharacterAnim::AirUp,
        AttackIntent::AirDown => CharacterAnim::AirDown,
        AttackIntent::AirForward => CharacterAnim::AirForward,
        AttackIntent::AirBack => CharacterAnim::AirBack,
        AttackIntent::Neutral
        | AttackIntent::Forward
        | AttackIntent::Back
        | AttackIntent::DashForward
        | AttackIntent::WallOut => CharacterAnim::AttackSide,
    }
}

/// The actor-only animation facts that DON'T live in the shared movement
/// clusters — the disposition reads ([`pick_actor_anim`] pulls everything else,
/// the rich movement/ability state, straight from the actor's real `Body*`
/// clusters via [`body_view_from_clusters`], exactly like the player). "Enemy"
/// and "NPC" were never different animation contracts, just dispositions: both
/// walk, attack, fly, take a hit, and die from the SAME cluster reads, so what an
/// actor shows is its real ECS state, not its label.
#[derive(Clone, Copy, Debug)]
pub struct ActorAnimState {
    /// Liveness (from `ActorStatus.alive`) → `Death`. The body's combat cluster
    /// drives the player's death; an actor's liveness lives on its status.
    pub alive: bool,
    /// Recent-hit flash (from `ActorStatus.hit_flash`) → `Hit`. The actor's
    /// damage path uses `hit_flash` where the player uses `BodyCombat.hitstun`.
    pub hit_flash: bool,
    /// Gravity-free FLIGHT archetype (sky parrot / shark): the locomotion tail
    /// reads `Fly` while moving and `Idle` while hovering, and the airborne
    /// (Jump/Fall) gate is suppressed. A non-aerial actor knocked off the ground
    /// is NOT aerial — it falls through to the Jump/Fall gate like the player.
    pub aerial: bool,
}

/// Pick any brain-driven actor's animation through the shared [`pick_body_anim`]
/// ladder, building the FULL [`BodyAnimView`] from the actor's real `Body*`
/// clusters — the same clusters and the same builder the player uses. So any
/// ability a brain (or an LLM) drives an actor's clusters into — dash, blink,
/// flight, shield, ladder climb, wall-grab, dodge-roll, crouch/slide — animates
/// with no per-archetype branch; the sheet's anim set ([`CharacterSheetSpec::
/// resolve_anim`]) decides how richly each pose reads.
///
/// The actor overlays only what isn't in those clusters: liveness / hit-flash
/// (from `ActorStatus`), its melee row (the swing's own intent, shared with the
/// player via [`directional_attack_anim`]), and the aerial locomotion metric +
/// thresholds (total speed, `Walk`-capped on the ground via `run_above: None`).
pub fn pick_actor_anim(
    kinematics: &crate::actor::BodyKinematics,
    ground: &crate::actor::BodyGroundState,
    wall: &crate::actor::BodyWallState,
    blink: &crate::actor::BodyBlinkState,
    flight: &crate::actor::BodyFlightState,
    dash: &crate::actor::BodyDashState,
    ledge: &crate::actor::BodyLedgeState,
    body_mode: &crate::actor::BodyModeState,
    env_contact: &crate::actor::BodyEnvironmentContact,
    abilities: &crate::actor::BodyAbilities,
    dodge: &crate::actor::BodyDodgeState,
    shield: &crate::actor::BodyShieldState,
    swing: Option<&crate::MeleeSwing>,
    state: ActorAnimState,
) -> CharacterAnim {
    let mut v = body_view_from_clusters(
        kinematics,
        ground,
        wall,
        blink,
        flight,
        dash,
        ledge,
        body_mode,
        env_contact,
        abilities,
        dodge,
        shield,
    );
    v.dead = !state.alive;
    v.hit = state.hit_flash;
    // Melee shares the player's directional mapping: the in-flight swing's own
    // intent picks the row, and `resolve_anim` walks it down to whatever swing
    // pose the actor's sheet actually owns (a slash-only sheet still reads its
    // slash; a sheet that drew `attack_up` reads the up-tilt distinctly). Gated to
    // the telegraph + hit window (startup/active), like the old actor read — the
    // recovery tail falls back to locomotion rather than holding the swing pose.
    v.melee_attack = swing
        .filter(|s| {
            matches!(
                s.phase(),
                Some(crate::combat::AttackPhase::Startup | crate::combat::AttackPhase::Active)
            )
        })
        .map(|s| directional_attack_anim(Some(s)));
    if state.aerial {
        // A flyer reads Fly/Idle from the locomotion tail; suppress the airborne
        // Jump/Fall gate (it floats — `on_ground` is false but it isn't falling).
        v.airborne = false;
        v.locomotion = Locomotion::Aerial;
        v.speed = kinematics.vel.length();
    }
    v.idle_below = 8.0;
    v.run_above = None;
    v.fly_above = 12.0;
    pick_body_anim(&v)
}

#[cfg(test)]
mod tests;
