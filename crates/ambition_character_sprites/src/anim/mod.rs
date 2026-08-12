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
/// Free-flight overrides ground/airborne motion because the engine
/// integrator already disables gravity in flight; the visual should
/// reflect the active mode rather than whatever fall/run inertia
/// happens to read.
/// Death is a TIMER for the player, not a liveness read: it respawns
/// instantly, so it is alive again before anything could draw it dead.
/// Whatever owns the death beat arms `BodyAnimFacts::death_anim_timer`
/// and the `Death` row plays above everything until it runs out.
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
    /// **The AERIAL evade**, distinct from the ground roll on purpose: it picks
    /// the `Roll` row, whose own fallback is `DodgeRoll`, so a sheet with one
    /// curl still animates and a sheet with two shows two maneuvers. Jon's ask
    /// on the air dodge was that animation be able to tell them apart, and a
    /// shared flag would have made that impossible at the source.
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
    /// Punch/Slash for actors). `None` ⇒ not attacking.
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
    /// Grounded braking against travel (`BodyMotionFacts::skidding`).
    pub skidding: bool,
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
/// **The authored ROW a body's fighter state asks to be drawn as**, when the new
/// sheets have one, with the fallback chain for the sheets that do not.
///
/// ⭐⭐ sprite redirect P2. The engine already produces these states and
/// [`pick_body_anim`] already reads them — but it can only answer in
/// [`CharacterAnim`], which has no variant for `air_dodge`, `tumble`,
/// `knockdown` or `getup`, so all four collapse onto `Roll` / `Hit` / `LandHard`
/// / `LandRecovery`. The new fighter sheets draw them separately. This is the
/// same clip-request seam a MOVE uses, for a state that has no `MoveSpec`.
///
/// ⛔ **the facts are READ, never inferred.** No velocity heuristics, no
/// "it looks airborne so it must be tumbling" — every term here is a flag the
/// movement kernel publishes about itself.
///
/// ⚠ **the priority order mirrors `pick_body_anim`'s exactly**, and that is not
/// tidiness: a knocked-down body is still inside its hitstun, so answering the
/// tumble first would draw the launched pose through the whole prone beat and
/// the knockdown would never be seen.
///
/// ⚠ **`tech` / `tech_roll` / `getup_roll` are NOT distinguished here**, and the
/// reason is honest: the simulation publishes ONE `getup_invulnerable` flag, so
/// telling a tech from an ordinary getup would mean inventing a distinction the
/// sim has not made. That fact belongs to the subsystem that knows which
/// `MovementOp` ran, and the row set waits for it.
pub fn body_state_clip(
    facts: &ambition_platformer2d_core::BodyMotionFacts,
) -> Option<&'static [&'static str]> {
    if facts.knocked_down {
        return Some(&["knockdown", "prone", "land_hard", "hit", "idle"]);
    }
    if facts.getup_invulnerable {
        return Some(&["getup", "land_recovery", "idle"]);
    }
    if facts.tumbling {
        return Some(&["tumble", "hit", "fall", "idle"]);
    }
    if facts.air_dodging {
        return Some(&["air_dodge", "roll", "fall", "idle"]);
    }
    None
}

pub fn pick_body_anim(v: &BodyAnimView) -> CharacterAnim {
    use CharacterAnim::*;
    if v.dead {
        return Death;
    }
    // ⭐ **the floor game outranks the hit flash.** A knocked-down body is still
    // inside its hitstun, so reading `hit` first would draw the struck pose for
    // the whole prone beat and the knockdown would be invisible.
    if v.knocked_down {
        return LandHard;
    }
    if v.getting_up {
        return LandRecovery;
    }
    if v.hit || v.tumbling {
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

/// Resolve the published ledge facts ([`ae::LedgeFacts`]) into the visual ledge
/// read (`None` ⇒ not on a ledge): a held hang is `Grab`; once committed the
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
/// (`|vx|`); an aerial adapter overrides it with total speed.
pub fn body_view_from_body(
    kinematics: &ae::BodyKinematics,
    ground: &ae::BodyGroundState,
    facts: &ae::BodyMotionFacts,
    flight: &ae::BodyFlightState,
    body_mode: &ae::BodyModeState,
    env_contact: &ae::BodyEnvironmentContact,
    abilities: &ae::BodyAbilities,
    shield: &ae::BodyShieldState,
) -> BodyAnimView {
    use ambition_platformer2d_core::player_state::BodyMode;
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
            && kinematics.vel.y.abs() < 40.0,
        gliding: facts.gliding,
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
mod state_clip_tests {
    use ambition_platformer2d_core::BodyMotionFacts;

    /// **A FIGHTER STATE ASKS FOR ITS OWN ROW, IN THE LADDER'S ORDER.**
    ///
    /// ⭐ sprite redirect P2. These four states existed and were drawn already —
    /// as `Roll`, `Hit`, `LandHard` and `LandRecovery`, because `CharacterAnim`
    /// has no variant for any of them. The new sheets draw them separately.
    ///
    /// ⛔ **the ORDER is the assertion that matters.** A knocked-down body is
    /// still inside its hitstun, so `tumbling` and `knocked_down` are true
    /// together — answering the tumble first would draw the launched pose for
    /// the whole prone beat and the knockdown would never be seen. That is the
    /// same priority `pick_body_anim` already encodes, and the two must not
    /// drift.
    #[test]
    fn a_fighter_state_asks_for_its_own_row_in_priority_order() {
        let head = |facts: &BodyMotionFacts| {
            super::body_state_clip(facts).map(|chain| chain[0].to_string())
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

        // ⛔ the poison: both true at once is the REAL case, not a contrived one.
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
    }

    /// **EVERY CHAIN ENDS SOMEWHERE A LEAN SHEET CAN REACH.**
    ///
    /// ⚠ expressiveness is opt-in: a character with only `idle`, `walk`, `slash`
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
        ] {
            let chain = super::body_state_clip(&facts).expect("this state asks for a row");
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
