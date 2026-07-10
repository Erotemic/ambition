//! Animation pickers over gameplay-core actor/player state.

#[allow(unused_imports)]
pub(crate) use ambition_sprite_sheet::character::non_looping;
pub use ambition_sprite_sheet::character::CharacterAnim;

use ambition_engine_core as ae;

use crate::actor::BodyAnimFacts;

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
    anim: &BodyAnimFacts,
    combat: &ambition_characters::actor::BodyCombat,
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
    /// [`crate::actor::BodyAnimFacts`] — the SAME poses the player shows, now
    /// available to any body (fable review §A9). `landing` carries hard-vs-soft.
    /// A sheet without a given row falls back through `resolve_anim`, so these are
    /// always safe to request.
    pub wall_jump: bool,
    pub dash_startup: bool,
    pub landing: Option<bool>,
    pub shooting: bool,
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
    // Movement-driven overlays from the actor's BodyAnimFacts — the SAME reads
    // `pick_player_anim` applies, so an AI fighter shows wall-jump / dash-startup /
    // landing / shoot poses (whatever its sheet owns) instead of only the base
    // ladder (fable review §A9).
    v.wall_jump = state.wall_jump;
    v.dash_startup = state.dash_startup;
    v.landing = state.landing;
    v.shooting = state.shooting;
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
