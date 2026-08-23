//! Animation pickers over gameplay-core actor/player state.

#[allow(
    unused_imports,
    reason = "`non_looping` is read by this module's tests"
)]
use ambition_sprite_sheet::character::{non_looping, CharacterAnim};

use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState;

use ambition_characters::actor::body::BodyAnimFacts;
use ambition_combat::components::MeleeSwing;

/// Pick the player's animation from ECS animation state and engine state.
///
/// Priority: hit > blink-in/out > slash > fly > dash > airborne (jump/fall) > run/walk/idle.
/// Free-flight overrides ground/airborne motion because the engine integrator already disables
/// gravity in flight; the visual should reflect the active mode rather than whatever fall/run
/// inertia happens to read. Whatever owns the death beat arms `BodyAnimFacts::death_anim_timer`
/// and the `Death` row plays above everything until it runs out. `BlinkOut` is used while the
/// blink button is held/aiming, and `BlinkIn` is held briefly after a committed blink so
/// VFX/camera have time to sell the arrival.
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
/// row a swing maps to, the locomotion speed metric (the RUN component along the
/// body's own `side` axis when grounded, vs total speed when aerial), and the
/// speed thresholds.
#[derive(Clone, Copy, Debug, Default)]
pub struct BodyAnimView {
    pub dead: bool,
    pub hit: bool,
    pub dodge_roll: bool,
    /// The AERIAL evade, distinct from the ground roll on purpose: it picks the `Roll` row,
    /// whose own fallback is `DodgeRoll`, so a sheet with one curl still animates and a sheet with
    /// two shows two maneuvers.
    pub air_dodge: bool,
    /// Launched and helpless — the struck pose held through the arc.
    pub tumbling: bool,
    /// Prone on the floor after an unteched landing.
    pub knocked_down: bool,
    /// Standing up out of a knockdown or a tech (the invulnerable beat).
    pub getting_up: bool,
    pub blink_in: bool,
    pub blocking: bool,
    /// Actor charge→thrust special (glider zoning); highest combat read after hit.
    pub special: bool,
    pub shooting: bool,
    /// The melee row to play while mid-swing (directional for the player,
    /// Punch/Slash for actors). `None`  not attacking.
    pub melee_attack: Option<CharacterAnim>,
    pub aiming: bool,
    pub wall_jump: bool,
    pub interacting: bool,
    pub blink_out: bool,
    pub ledge: Option<LedgeRead>,
    pub flying: bool,
    pub swimming: bool,
    /// Persistent curled ball (spin dash roll / morph ball). Outranks the
    /// dash + airborne reads: a ball flying off a ramp is still a ball.
    pub rolling: bool,
    /// Held in another body's hands. Outranks every locomotion read: a captive
    /// is not walking, falling or idling, whatever its velocity says.
    pub held: bool,
    /// The guard shattered and this body is dizzy. Outranks `blocking` for the
    /// obvious reason — there is no guard to draw — and sits with the reeling
    /// reads because that is what it is.
    pub guard_broken: bool,
    pub dash_startup: bool,
    pub dashing: bool,
    pub ladder_climbing: bool,
    pub wall_grab: bool,
    pub gliding: bool,
    pub airborne: bool,
    /// Only read while `airborne`: up  Jump, else Fall.
    pub moving_up: bool,
    /// `Some(hard)` while a landing-recovery pose is held (grounded only).
    pub landing: Option<bool>,
    /// Grounded braking against travel (`BodyMotionFacts::skidding`).
    pub skidding: bool,
    pub compact: CompactBody,
    pub locomotion: Locomotion,
    /// Locomotion speed in the metric the style uses: grounded is the component
    /// along the body's OWN run axis (`|vel · side|`), aerial is total speed.
    ///
    /// it was `|vel.x|` until, which is the same number only while
    /// gravity is vertical — see [`body_view_from_body`] for the wall case that
    /// exposed it.
    pub speed: f32,
    /// Grounded: `speed < idle_below`  Idle.
    pub idle_below: f32,
    /// Grounded: this body is in a RUN (`BodyMotionFacts::running`).
    ///
    /// this replaces a `run_above: Option<f32>` whose two setters
    /// DISAGREED, along the player/actor line. `pick_player_anim` set
    /// `Some(220.0)` and `pick_actor_anim` set `None`, and `None` means
    /// *cap at Walk* — so the protagonist could run and every CPU fighter in
    /// the game was capped at a walk, drawing `walk` at full sprint. The `run`
    /// row every fighter sheet draws was unreachable for the bodies that do most
    /// of the running.
    ///
    /// and a raw speed threshold was the wrong question anyway: `220.0` is an
    /// absolute px/s, so a slow heavy could never run and a fast character was
    /// always running. A gait is speed against THIS body's own top speed, which
    /// is what `MovementTuning::run_commit_frac` asks and the kernel publishes.
    pub running: bool,
    /// Aerial: `speed > fly_above`  Fly, else Idle (hover).
    pub fly_above: f32,
}

/// The single animation-priority ladder every body runs. Each archetype's
/// adapter builds a [`BodyAnimView`] and calls this; the ordering here is the
/// player's full ladder, with the actor-only reads (`dead`, `special`, the
/// Punch swing row) folded in at their priorities. Inert states (a `None`
/// ledge, `false` flags) fall straight through, so a sparse actor view lands on
/// the shared locomotion tail.
/// The two fighter facts that are not the movement kernel's, taken beside
/// [`BodyMotionFacts`](ambition_platformer2d_core::BodyMotionFacts) by
/// [`body_state_clip`].
///
/// they are separate because they are owned elsewhere: `held` is combat's
/// capture relation mirrored onto `BodyAnimFacts`, and `guard_break` is the
/// shield cluster's. Neither is something the movement kernel publishes about
/// itself, and folding them into the motion facts would say it does.
/// Which beat of a shield break a body is in.
///
/// the genre draws a break as a SEQUENCE — launched off the ground, falling,
/// collapsed, recovering — and the simulation publishes ONE countdown. The beat
/// is derived from that countdown and the length it started at
/// (`BodyShieldState::break_phase`), so four poses cost no new state.
///
/// the boundaries live HERE and nowhere else. They are presentation
/// timing, not feel: a sheet that draws the four rows wants them apportioned the
/// way the art was drawn, and a sheet that draws none is unaffected because
/// every chain falls back to `dizzy`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuardBreakBeat {
    /// The shatter itself — the body is thrown off the ground.
    Launch,
    /// Falling back down from the shatter.
    Fall,
    /// Face-down and helpless. The long middle, and the beat that reads as the
    /// punish window.
    Collapse,
    /// Coming back up, still unable to act.
    Recover,
}

impl GuardBreakBeat {
    /// `phase` is `0.0` at the shatter, approaching `1.0` at recovery.
    pub fn from_phase(phase: f32) -> Self {
        // The shatter and the fall are quick; the collapse is what the punish is
        // aimed at, so it holds the middle.
        const FALL: f32 = 0.12;
        const COLLAPSE: f32 = 0.30;
        const RECOVER: f32 = 0.85;
        if phase < FALL {
            Self::Launch
        } else if phase < COLLAPSE {
            Self::Fall
        } else if phase < RECOVER {
            Self::Collapse
        } else {
            Self::Recover
        }
    }

    /// every chain ends at `dizzy`, so a sheet that draws none of the four
    /// rows is answered exactly as it was before the sequence existed.
    pub fn clip_chain(self) -> &'static [&'static str] {
        match self {
            Self::Launch => &["shield_break_launch", "shield_break_fall", "dizzy", "hit", "idle"],
            Self::Fall => &["shield_break_fall", "dizzy", "hit", "idle"],
            Self::Collapse => &["shield_break_collapse", "dizzy", "hit", "idle"],
            Self::Recover => &["shield_break_recover", "dizzy", "hit", "idle"],
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FighterClipFacts {
    /// This body is in another body's hands.
    pub held: bool,
    /// This body has somebody in ITS hands.
    pub holding: bool,
    /// This body's guard shattered, and WHICH BEAT of the break it is in.
    /// `None` when no break is running.
    ///
    /// a beat rather than a bool because the sheets draw four poses for one
    /// timer; see [`GuardBreakBeat`].
    pub guard_break: Option<GuardBreakBeat>,
    /// This body's PERFECT-SHIELD window is live
    /// (`BodyShieldState::parrying()`).
    ///
    /// under `ParryTiming::OnRelease` the window is live while the guard is
    /// DOWN, so this is deliberately not conditioned on the guard being raised.
    pub parrying: bool,
    /// This body's guard TOOK a hit and is paying shieldstun for it
    /// (`BodyShieldState::stun_timer`).
    ///
    /// not [`Self::guard_break`] — the shield HELD. A break is the
    /// floor game; this is the beat a blocked hit costs a defender.
    pub guard_stunned: bool,
}

/// The authored ROW a body's fighter state asks to be drawn as, when the new
/// sheets have one, with the fallback chain for the sheets that do not.
///
/// sprite redirect P2. The engine already produces these states and
/// [`pick_body_anim`] already reads them — but it can only answer in
/// [`CharacterAnim`], which has no variant for `air_dodge`, `tumble`,
/// `knockdown` or `getup`, so all four collapse onto `Roll` / `Hit` / `LandHard`
/// / `LandRecovery`. The new fighter sheets draw them separately. This is the
/// same clip-request seam a MOVE uses, for a state that has no `MoveSpec`.
///
/// the facts are READ, never inferred. No velocity heuristics, no
/// "it looks airborne so it must be tumbling" — every term here is a flag the
/// movement kernel publishes about itself.
///
/// the priority order mirrors `pick_body_anim`'s exactly, and that is not
/// tidiness: a knocked-down body is still inside its hitstun, so answering the
/// tumble first would draw the launched pose through the whole prone beat and
/// the knockdown would never be seen.
///
/// `tech` / `tech_roll` / `getup_roll` are NOT distinguished here, and the
/// reason is honest: the simulation publishes ONE `getup_invulnerable` flag, so
/// telling a tech from an ordinary getup would mean inventing a distinction the
/// sim has not made. That fact belongs to the subsystem that knows which
/// `MovementOp` ran, and the row set waits for it.
pub fn body_state_clip(
    facts: &ambition_platformer2d_core::BodyMotionFacts,
    fighter: FighterClipFacts,
) -> Option<&'static [&'static str]> {
    // ABOVE the floor game, for the same reason `pick_body_anim` puts it
    // there: a body in somebody's hands is not doing anything else.
    if fighter.held {
        return Some(&["grabbed", "hit", "idle"]);
    }
    if facts.knocked_down {
        return Some(&["knockdown", "prone", "land_hard", "hit", "idle"]);
    }
    if facts.getup_invulnerable {
        return Some(&["getup", "land_recovery", "idle"]);
    }
    // FOUR beats out of ONE timer, and no new simulation state: the sim still
    // publishes one `break_timer`, and `BodyShieldState::break_total` records how
    // long THIS break was, so the phase — and the beat — is derived.
    //
    // the tech-versus-getup row above is still in the honest position, because ONE
    // `getup_invulnerable` flag really does carry no such distinction.
    if let Some(beat) = fighter.guard_break {
        return Some(beat.clip_chain());
    }
    if facts.tumbling {
        return Some(&["tumble", "hit", "fall", "idle"]);
    }
    // BELOW everything above it: a captor that got knocked down is not holding
    // anybody any more, and a captor mid-throw has a MOVE, which outranks every
    // state row. What is left is the beat between the grab and the decision,
    // which drew the standing pose.
    if fighter.holding {
        return Some(&["grab_hold", "grab", "idle"]);
    }
    if facts.air_dodging {
        return Some(&["air_dodge", "roll", "fall", "idle"]);
    }
    // ABOVE no floor-game state and below all of them: a spot dodge is an
    // ordinary grounded action, and the only thing it has to outrank is the ROLL
    // it shares a timer with. `CharacterAnim` answers both as `DodgeRoll`, so
    // without this row a body that stood its ground is drawn rolling.
    if facts.spot_dodging {
        return Some(&["spot_dodge", "crouch", "roll", "idle"]);
    }
    // THE PERFECT SHIELD, and it is a REFINEMENT of the block — the same
    // shape as `spot_dodge` refining the roll above. `pick_body_anim` answers
    // every guarding body with one `Block`, so a guard merely held and a guard
    // that caught something perfectly drew the identical pose. The sheets have
    // drawn them apart the whole time.
    //
    // ABOVE the shieldstun row: both can be live on the same tick, and the
    // parry is the thing the PLAYER did.
    if fighter.parrying {
        return Some(&["parry", "block", "idle"]);
    }
    // the guard took a hit and HELD — distinct from `guard_broken` far above,
    // which is the floor game. This is the beat a blocked hit costs a defender,
    // and it is what makes shieldstun visible rather than merely true.
    if fighter.guard_stunned {
        return Some(&["shield_hit", "block", "idle"]);
    }
    // LAST, because it is the least urgent thing a body can be doing and every
    // state above it can overlap the squat's few frames. It is here at all
    // because the squat is a COMMITMENT the player can see coming — the beat a
    // short hop is decided in — and it drew the standing pose.
    if facts.jump_squatting {
        return Some(&["jump_squat", "crouch", "idle"]);
    }
    None
}

pub fn pick_body_anim(v: &BodyAnimView) -> CharacterAnim {
    use CharacterAnim::*;
    if v.dead {
        return Death;
    }
    // the floor game outranks the hit flash. A knocked-down body is still inside its
    // hitstun, so reading `hit` first would draw the struck pose for the whole prone beat and
    // the knockdown would be invisible. ABOVE the floor game and the hit flash: a body in
    // somebody's hands is not doing anything else, and its velocity is the captor's.
    if v.held {
        return Hit;
    }
    if v.knocked_down {
        return LandHard;
    }
    if v.getting_up {
        return LandRecovery;
    }
    // a shattered guard reads as reeling, which is the closest row this
    // 56-variant vocabulary owns. this is the TAIL of the ladder, not the
    // answer: `body_state_clip` asks the sheet for `dizzy` first, and a fighter
    // sheet has that row. This arm is what a sheet without one lands on.
    if v.hit || v.tumbling || v.guard_broken {
        return Hit;
    }
    if v.dodge_roll {
        return DodgeRoll;
    }
    if v.air_dodge {
        return Roll;
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
    if v.rolling {
        return Roll;
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
        // A crouch that leaves the ground is a crouch jump, and it is a real
        // move in the games this borrows from. Asked BEFORE the plain airborne
        // answer, because this branch used to run first and swallowed it.
        if matches!(v.compact, CompactBody::Crouch) && v.moving_up {
            return CrouchJump;
        }
        return if v.moving_up { Jump } else { Fall };
    }
    if let Some(hard) = v.landing {
        return if hard { LandHard } else { LandRecovery };
    }
    if v.skidding {
        return Skid;
    }
    match v.compact {
        CompactBody::Slide => return Slide,
        CompactBody::Crawl => return Crawl,
        // A crouch that is MOVING is a crouch walk. The compact stance used to
        // answer with one row for both, so a shuffling duck was a statue.
        CompactBody::Crouch => {
            return if v.speed > v.idle_below {
                CrouchWalk
            } else {
                Crouch
            }
        }
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
            } else if v.running {
                Run
            } else {
                Walk
            }
        }
    }
}

/// Resolve the published ledge facts ([`ae::LedgeFacts`]) into the visual ledge
/// read (`None`  not on a ledge): a held hang is `Grab`; once committed the
/// getup-kind selects the climb / roll / attack getup. SHARED by every body —
/// the player and any actor that grows a ledge-grab limb route through this one
/// mapping.
fn ledge_read(ledge: Option<ae::LedgeFacts>) -> Option<LedgeRead> {
    ledge.map(|s| {
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
fn compact_from_mode(mode: ambition_platformer2d_core::player_state::BodyMode) -> CompactBody {
    use ambition_platformer2d_core::player_state::BodyMode;
    match mode {
        BodyMode::Sliding => CompactBody::Slide,
        BodyMode::Crawling => CompactBody::Crawl,
        BodyMode::Crouching => CompactBody::Crouch,
        _ => CompactBody::None,
    }
}

/// Fill every [`BodyAnimView`] field that is derived purely from the shared
/// `Body*` clusters + the published [`ae::BodyMotionFacts`] projection — the
/// reads that are IDENTICAL for every body, player or brain-driven actor. This is the convergence seam: whatever
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
/// (the run component along the body's own `side` axis); an aerial adapter
/// overrides it with total speed.
pub fn body_view_from_body(
    kinematics: &ae::BodyKinematics,
    ground: &ae::BodyGroundState,
    facts: &ae::BodyMotionFacts,
    flight: &ae::BodyFlightState,
    body_mode: &ae::BodyModeState,
    env_contact: &ae::BodyEnvironmentContact,
    abilities: &ae::BodyAbilities,
    shield: &ae::BodyShieldState,
    frame: ae::AccelerationFrame,
) -> BodyAnimView {
    use ambition_platformer2d_core::player_state::BodyMode;
    // THE THREE READS BELOW ARE FRAME-RELATIVE, AND EACH WAS A WORLD-AXIS
    // TEST THAT ONLY LOOKED RIGHT UNDER VERTICAL GRAVITY.
    //
    // row never played — but upside-down on a ceiling it did. Turn gravity sideways and the run
    // axis becomes world-y: a body at full sprint reads `|vel.x| ≈ 0`, lands under `idle_below`,
    // and stands still.
    //
    // each of these reduces to its old form exactly when `down == (0, 1)`
    // (`vel.dot((0,1)) == vel.y`), so ordinary rooms are untouched by construction
    // rather than by re-tuning.
    let along_run = kinematics.vel.dot(frame.side);
    let along_fall = kinematics.vel.dot(frame.down);
    BodyAnimView {
        // The dodge↔ledge guard: a roll that is part of a ledge getup keeps the
        // dedicated `LedgeRoll` row instead of the grounded `DodgeRoll`.
        dodge_roll: facts.dodge_rolling && facts.ledge.is_none(),
        air_dodge: facts.air_dodging,
        tumbling: facts.tumbling && !facts.knocked_down,
        knocked_down: facts.knocked_down,
        // The getup beat is the invulnerable window that follows standing up or
        // teching, and only while the body is NOT already prone again.
        getting_up: facts.getup_invulnerable && !facts.knocked_down,
        blocking: shield.active && abilities.abilities.shield,
        guard_broken: shield.broken(),
        blink_out: facts.blink_telegraph,
        ledge: ledge_read(facts.ledge),
        flying: flight.fly_enabled,
        swimming: env_contact.water.is_some() && abilities.abilities.swim,
        dashing: facts.dashing,
        skidding: facts.skidding,
        // High-priority climb (ladder/vine) vs the low-priority compact silhouette
        // (slide/crawl/crouch) are distinct fields checked at distinct priorities.
        ladder_climbing: matches!(body_mode.body_mode, BodyMode::Climbing),
        wall_grab: !ground.on_ground
            && facts.wall_clinging
            && !facts.wall_climbing
            && along_fall.abs() < 40.0,
        gliding: facts.gliding,
        airborne: !ground.on_ground,
        // Rising = moving AGAINST gravity. (Under `down == (0,1)` this is the
        // old `vel.y < -10.0`, top-left coords, unchanged.)
        moving_up: along_fall < -10.0,
        compact: compact_from_mode(body_mode.body_mode),
        // the grounded metric is the RUN component, not total speed: a body
        // sliding straight down its own wall must stay Idle, not walk in place.
        speed: along_run.abs(),
        running: facts.running,
        ..Default::default()
    }
}

#[allow(clippy::too_many_arguments)]
pub fn pick_player_anim(
    anim: &BodyAnimFacts,
    combat: &ambition_characters::actor::BodyCombat,
    blink_cam: &PlayerBlinkCameraState,
    attack: Option<&MeleeSwing>,
    kinematics: &ae::BodyKinematics,
    ground: &ae::BodyGroundState,
    facts: &ae::BodyMotionFacts,
    flight: &ae::BodyFlightState,
    body_mode: &ae::BodyModeState,
    env_contact: &ae::BodyEnvironmentContact,
    abilities: &ae::BodyAbilities,
    shield: &ae::BodyShieldState,
    frame: ae::AccelerationFrame,
) -> CharacterAnim {
    // Movement/ability fields come from the shared cluster builder (identical to
    // every actor); the player overlays its combat-cluster hit read, its own
    // presentation-timer reads, and its grounded thresholds. Each line below is
    // the exact predicate the old per-branch ladder used.
    let mut v = body_view_from_body(
        kinematics,
        ground,
        facts,
        flight,
        body_mode,
        env_contact,
        abilities,
        shield,
        frame,
    );
    // The player is ALIVE again by the time anything draws it — the respawn is
    // immediate — so its death cannot be read off liveness the way an actor's is.
    // The beat that owns the death arms this timer; see `death_anim_timer`.
    v.dead = anim.death_anim_timer > 0.0;
    v.hit = combat.hitstun_timer > 0.05;
    v.blink_in = blink_cam.blink_in_timer > 0.0;
    v.shooting = anim.shoot_anim_timer > 0.0;
    // Gate the attack row on the live swing's PHASE (startup/active), the same
    // read `pick_actor_anim` uses — NOT `slash_anim_timer`. Every body melees
    // through the moveset runtime, which projects `BodyMelee.swing` but never arms
    // `slash_anim_timer`, so reading the timer left the body stuck on its
    // locomotion row for the whole swing; the phase read is the melee row source.
    v.melee_attack = attack
        .filter(|s| {
            matches!(
                s.phase(),
                Some(ambition_combat::AttackPhase::Startup | ambition_combat::AttackPhase::Active)
            )
        })
        .map(|s| directional_attack_anim(Some(s)));
    v.aiming = anim.aim_anim_active;
    v.wall_jump = anim.wall_jump_anim_timer > 0.0;
    v.interacting = anim.interact_anim_timer > 0.0;
    v.rolling = anim.rolling;
    v.held = anim.held;
    v.dash_startup = anim.dash_startup_timer > 0.0;
    v.landing = (anim.land_anim_timer > 0.0).then_some(anim.land_anim_hard);
    v.idle_below = 12.0;
    v.fly_above = 0.0;
    pick_body_anim(&v)
}

/// Map the active player attack intent onto the directional swing rows.
///
/// The engine's `AttackIntent` is finer-grained than the visible swing
/// shapes — multiple intents share one row because the sprite already
/// flips with the player's facing.
fn directional_attack_anim(attack: Option<&MeleeSwing>) -> CharacterAnim {
    use ambition_combat::AttackIntent;
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
/// clusters via [`body_view_from_body`], exactly like the player). "Enemy"
/// and "NPC" were never different animation contracts, just dispositions: both
/// walk, attack, fly, take a hit, and die from the SAME cluster reads, so what an
/// actor shows is its real ECS state, not its label.
#[derive(Clone, Copy, Debug, Default)]
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
    /// Movement-driven presentation overlays, read from the actor's
    /// [`BodyAnimFacts`] — the SAME poses the player shows, now
    /// available to any body (fable review §A9). `landing` carries hard-vs-soft.
    /// A sheet without a given row falls back through `resolve_anim`, so these are
    /// always safe to request.
    pub wall_jump: bool,
    pub dash_startup: bool,
    pub landing: Option<bool>,
    pub shooting: bool,
    /// Persistent curled ball (`BodyAnimFacts::rolling`) — the same read the
    /// player overlays, so a brain-driven body that curls up shows its ball.
    pub rolling: bool,
    /// Held in another body's hands (`BodyAnimFacts::held`). A CPU fighter is
    /// grabbed exactly as often as a human one, so this rides the actor road
    /// too — the alternative is a captive that reads as held only when a person
    /// is playing it.
    pub held: bool,
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
/// metric (total speed), with the grounded gait taken from the published
/// `BodyMotionFacts::running` rather than a threshold.
pub fn pick_actor_anim(
    kinematics: &ae::BodyKinematics,
    ground: &ae::BodyGroundState,
    facts: &ae::BodyMotionFacts,
    flight: &ae::BodyFlightState,
    body_mode: &ae::BodyModeState,
    env_contact: &ae::BodyEnvironmentContact,
    abilities: &ae::BodyAbilities,
    shield: &ae::BodyShieldState,
    swing: Option<&MeleeSwing>,
    state: ActorAnimState,
    frame: ae::AccelerationFrame,
) -> CharacterAnim {
    let mut v = body_view_from_body(
        kinematics,
        ground,
        facts,
        flight,
        body_mode,
        env_contact,
        abilities,
        shield,
        frame,
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
                Some(ambition_combat::AttackPhase::Startup | ambition_combat::AttackPhase::Active)
            )
        })
        .map(|s| directional_attack_anim(Some(s)));
    // Movement-driven overlays from the actor's BodyAnimFacts — the SAME reads
    // `pick_player_anim` applies, so an AI fighter shows wall-jump / dash-startup /
    // landing / shoot poses (whatever its sheet owns) instead of only the base
    // ladder (fable review §A9).
    v.wall_jump = state.wall_jump;
    v.dash_startup = state.dash_startup;
    v.landing = state.landing;
    v.shooting = state.shooting;
    v.rolling = state.rolling;
    v.held = state.held;
    if state.aerial {
        // A flyer reads Fly/Idle from the locomotion tail; suppress the airborne
        // Jump/Fall gate (it floats — `on_ground` is false but it isn't falling).
        v.airborne = false;
        v.locomotion = Locomotion::Aerial;
        v.speed = kinematics.vel.length();
    }
    v.idle_below = 8.0;
    v.fly_above = 12.0;
    pick_body_anim(&v)
}

#[cfg(test)]
mod state_clip_tests {
    use ambition_platformer2d_core::BodyMotionFacts;

    /// Fighter-specific guard and shield-break clips precede generic animation
    /// fallbacks. Shield-break beat is derived from the existing timer, and every
    /// beat chain ends in `dizzy` so sheets without fighter rows retain fallback art.
    #[test]
    fn a_shield_break_is_four_beats_that_all_fall_back_to_dizzy() {
        use super::GuardBreakBeat;
        let first = |phase: f32| {
            super::body_state_clip(
                &BodyMotionFacts::default(),
                super::FighterClipFacts {
                    guard_break: Some(GuardBreakBeat::from_phase(phase)),
                    ..Default::default()
                },
            )
            .map(|chain| chain[0].to_string())
        };

        assert_eq!(first(0.0), Some("shield_break_launch".to_string()));
        assert_eq!(first(0.2), Some("shield_break_fall".to_string()));
        assert_eq!(first(0.5), Some("shield_break_collapse".to_string()));
        assert_eq!(first(0.95), Some("shield_break_recover".to_string()));

        // NON-VACUITY: the four must be DISTINCT. A `from_phase` that
        // returned one beat for everything would satisfy "it returns something"
        // at every phase above, and this is a sequence or it is nothing.
        let beats: std::collections::BTreeSet<String> = [0.0, 0.2, 0.5, 0.95]
            .into_iter()
            .filter_map(first)
            .collect();
        assert_eq!(beats.len(), 4, "the four beats collapsed onto {beats:?}");

        // THE FLOOR. A sheet with no break art must still reach `dizzy`,
        // which is what every chain ends at.
        for phase in [0.0, 0.2, 0.5, 0.95] {
            let chain = super::body_state_clip(
                &BodyMotionFacts::default(),
                super::FighterClipFacts {
                    guard_break: Some(GuardBreakBeat::from_phase(phase)),
                    ..Default::default()
                },
            )
            .expect("a broken guard names a chain");
            assert!(
                chain.contains(&"dizzy"),
                "the {phase} beat cannot fall back to `dizzy`, so a sheet \
                 without break art loses the pose it already had: {chain:?}"
            );
        }

        // and no break, no rows: a body whose guard is intact is untouched.
        assert_eq!(
            super::body_state_clip(&BodyMotionFacts::default(), Default::default()),
            None,
        );
    }

    #[test]
    fn a_parry_and_a_struck_guard_are_not_the_same_pose_as_holding_one() {
        let fighter = |f: super::FighterClipFacts| {
            super::body_state_clip(&BodyMotionFacts::default(), f).map(|chain| chain[0].to_string())
        };

        assert_eq!(
            fighter(super::FighterClipFacts {
                parrying: true,
                ..Default::default()
            }),
            Some("parry".to_string()),
        );
        assert_eq!(
            fighter(super::FighterClipFacts {
                guard_stunned: true,
                ..Default::default()
            }),
            Some("shield_hit".to_string()),
        );

        // the poison, and it is the REAL case rather than a contrived one:
        // a perfect shield that catches a hit has a live parry window AND the
        // shieldstun that hit costs, on the same tick. The parry is what the
        // player did, so it has to win.
        assert_eq!(
            fighter(super::FighterClipFacts {
                parrying: true,
                guard_stunned: true,
                ..Default::default()
            }),
            Some("parry".to_string()),
            "a perfect shield drew the struck-guard pose, so the beat the player \
             earned is invisible exactly when they earn it"
        );

        // and a SHATTERED guard outranks both — it is the floor game.
        // the row is the BEAT now, not `dizzy`; what is asserted is still that
        // the break wins, which is why the fixture keeps both losers set.
        assert_eq!(
            fighter(super::FighterClipFacts {
                guard_break: Some(super::GuardBreakBeat::Collapse),
                parrying: true,
                guard_stunned: true,
                ..Default::default()
            }),
            Some("shield_break_collapse".to_string()),
        );

        assert_eq!(
            fighter(super::FighterClipFacts::default()),
            None,
            "an ordinary held guard was handed a clip, which takes `Block` away \
             from every sheet that has no fighter rows"
        );
    }

    /// the ORDER is the assertion that matters. A knocked-down body is
    /// still inside its hitstun, so `tumbling` and `knocked_down` are true
    /// together — answering the tumble first would draw the launched pose for
    /// the whole prone beat and the knockdown would never be seen. That is the
    /// same priority `pick_body_anim` already encodes, and the two must not
    /// drift.
    #[test]
    fn a_fighter_state_asks_for_its_own_row_in_priority_order() {
        let head = |facts: &BodyMotionFacts| {
            super::body_state_clip(facts, Default::default()).map(|chain| chain[0].to_string())
        };
        let fighter = |f: super::FighterClipFacts| {
            super::body_state_clip(&BodyMotionFacts::default(), f).map(|chain| chain[0].to_string())
        };

        assert_eq!(
            head(&BodyMotionFacts::default()),
            None,
            "a body doing none of \
             these draws its ordinary pose, and must not be handed a clip"
        );

        let tumbling = BodyMotionFacts {
            tumbling: true,
            ..Default::default()
        };
        assert_eq!(head(&tumbling), Some("tumble".to_string()));

        // the poison: both true at once is the REAL case, not a contrived one.
        let downed = BodyMotionFacts {
            tumbling: true,
            knocked_down: true,
            ..Default::default()
        };
        assert_eq!(
            head(&downed),
            Some("knockdown".to_string()),
            "a knocked-down body drew its LAUNCH pose for the whole prone beat"
        );

        assert_eq!(
            head(&BodyMotionFacts {
                air_dodging: true,
                ..Default::default()
            }),
            Some("air_dodge".to_string())
        );
        assert_eq!(
            head(&BodyMotionFacts {
                getup_invulnerable: true,
                ..Default::default()
            }),
            Some("getup".to_string())
        );
        // `spot_dodging` is a REFINEMENT of `dodge_rolling`, so both are true
        // together and the pair is the assertion: the spot dodge must win, or a
        // body that stood its ground is drawn rolling.
        assert_eq!(
            head(&BodyMotionFacts {
                dodge_rolling: true,
                spot_dodging: true,
                ..Default::default()
            }),
            Some("spot_dodge".to_string())
        );

        // the two facts the movement kernel does NOT publish, and the pair of
        // them is the assertion: both drew `hit` before this seam existed, so a
        // captive and a fighter with a shattered guard were the same picture.
        assert_eq!(
            fighter(super::FighterClipFacts {
                held: true,
                ..Default::default()
            }),
            Some("grabbed".to_string())
        );
        assert_eq!(
            fighter(super::FighterClipFacts {
                guard_break: Some(super::GuardBreakBeat::Collapse),
                ..Default::default()
            }),
            Some("shield_break_collapse".to_string())
        );
        assert_eq!(
            fighter(super::FighterClipFacts {
                holding: true,
                ..Default::default()
            }),
            Some("grab_hold".to_string())
        );
        assert_eq!(
            head(&BodyMotionFacts {
                jump_squatting: true,
                ..Default::default()
            }),
            Some("jump_squat".to_string())
        );
        // and it LOSES to everything: a body knocked down during its own
        // squat is knocked down, not squatting.
        assert_eq!(
            head(&BodyMotionFacts {
                jump_squatting: true,
                knocked_down: true,
                ..Default::default()
            }),
            Some("knockdown".to_string())
        );
        // and the priority poison: a body knocked down while STILL held is
        // held — the captor's pose owns it, the way `pick_body_anim` orders it.
        assert_eq!(
            super::body_state_clip(
                &BodyMotionFacts {
                    knocked_down: true,
                    ..Default::default()
                },
                super::FighterClipFacts {
                    held: true,
                    ..Default::default()
                }
            )
            .map(|chain| chain[0].to_string()),
            Some("grabbed".to_string()),
            "a held body drew the floor game it is not in"
        );
    }

    /// EVERY CHAIN ENDS SOMEWHERE A LEAN SHEET CAN REACH.
    ///
    /// expressiveness is opt-in: a character with only `idle`, `walk`, `slash`
    /// and `hit` must still draw something for every one of these states, or the
    /// new vocabulary becomes a requirement rather than an opportunity.
    #[test]
    fn every_state_chain_ends_at_idle() {
        for facts in [
            BodyMotionFacts {
                knocked_down: true,
                ..Default::default()
            },
            BodyMotionFacts {
                getup_invulnerable: true,
                ..Default::default()
            },
            BodyMotionFacts {
                tumbling: true,
                ..Default::default()
            },
            BodyMotionFacts {
                air_dodging: true,
                ..Default::default()
            },
            BodyMotionFacts {
                dodge_rolling: true,
                spot_dodging: true,
                ..Default::default()
            },
            BodyMotionFacts {
                jump_squatting: true,
                ..Default::default()
            },
        ] {
            let chain = super::body_state_clip(&facts, Default::default())
                .expect("this state asks for a row");
            assert_eq!(
                chain.last(),
                Some(&"idle"),
                "chain {chain:?} can fall through to nothing on a lean sheet"
            );
        }
        for f in [
            super::FighterClipFacts {
                held: true,
                ..Default::default()
            },
            super::FighterClipFacts {
                guard_break: Some(super::GuardBreakBeat::Collapse),
                ..Default::default()
            },
            super::FighterClipFacts {
                holding: true,
                ..Default::default()
            },
        ] {
            let chain = super::body_state_clip(&BodyMotionFacts::default(), f)
                .expect("this state asks for a row");
            assert_eq!(
                chain.last(),
                Some(&"idle"),
                "chain {chain:?} can fall through to nothing on a lean sheet"
            );
        }
    }
}

#[cfg(test)]
mod tests;
