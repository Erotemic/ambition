//! Composable movement abilities over the shared body-cluster state.
//! Each ability reads and writes only the cluster fields it owns.

use super::events::FrameEvents;
use super::input::InputState;
use super::model::AxisManeuverState;
use super::ops::MovementOp;
use super::tuning::AxisSweptParams;
use crate::body_clusters::{
    BodyAbilities, BodyComboTrace, BodyDashState, BodyDodgeState, BodyFlightState, BodyGroundState,
    BodyKinematics, BodyShieldState,
};
use crate::MotionFrame;

/// Maneuver a burst press would resolve to in the body's current state.
///
/// This is availability, not intent: it accounts for ground/air state, cooldowns,
/// budgets, and charges without consulting the buffered press itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BurstManeuver {
    /// The press produces nothing: no ability, or every route is on cooldown.
    #[default]
    None,
    /// A grounded evade roll (invulnerable, committed, then a cooldown).
    GroundDodge,
    /// An airborne evade in any stick direction — once per airtime.
    AirDodge,
    /// The traversal burst: a fast committed move in the aimed direction.
    Dash,
}

impl BurstManeuver {
    /// Whichever of the two dodges this is, if it is one.
    pub fn is_dodge(self) -> bool {
        matches!(self, Self::GroundDodge | Self::AirDodge)
    }
}

/// See [`BurstManeuver`]. Pure + frame-agnostic; the precedence (dodge outranks
/// dash on a body that owns both) is the call order of [`apply_dodge`] before
/// [`apply_dash`], stated once here instead of emerging from it.
pub fn resolve_burst_maneuver(
    abilities: &BodyAbilities,
    ground: &BodyGroundState,
    dodge: &BodyDodgeState,
    state: &AxisManeuverState,
    dash: &BodyDashState,
    tuning: AxisSweptParams,
) -> BurstManeuver {
    if let Some(evade) = available_dodge(abilities, ground, dodge, state, tuning) {
        return evade;
    }
    if dash_available(abilities, dash) {
        return BurstManeuver::Dash;
    }
    BurstManeuver::None
}

/// The dodge half of [`resolve_burst_maneuver`], split out so [`apply_dodge`]
/// gates on the SAME expression rather than a second copy of it. (It cannot call
/// the whole resolver: it holds no dash state, and needing it would be threading
/// one ability's cluster through another's step.)
fn available_dodge(
    abilities: &BodyAbilities,
    ground: &BodyGroundState,
    dodge: &BodyDodgeState,
    state: &AxisManeuverState,
    tuning: AxisSweptParams,
) -> Option<BurstManeuver> {
    if !abilities.abilities.dodge {
        return None;
    }
    if ground.on_ground {
        return (dodge.cooldown <= 0.0).then_some(BurstManeuver::GroundDodge);
    }
    // the budget is checked WITHOUT consuming the buffered press, so a body
    // that has already dodged this airtime leaves it standing and the press goes
    // on to mean what it would have meant without the dodge ability at all —
    // which is a dash. That fall-through is deliberate, and it is exactly why
    // an autonomous driver must ask this resolver rather than `abilities.dodge`.
    (!dodge.air_dodge_spent
        && tuning.abilities.air_dodge_time > 0.0
        && state.air_dodge_timer <= 0.0
        && state.air_dodge_endlag_timer <= 0.0)
        .then_some(BurstManeuver::AirDodge)
}

/// How long an evade's invulnerable window lasts for a body that has been
/// evading a lot — and the record that it just spent another one.
///
/// ⭐⭐ STALING WEARS THE I-FRAMES, NOT THE DISTANCE. The thing a spammed roll
/// abuses is invulnerability, so that is what wears out: a stale roll still
/// travels and still recovers, it is simply no longer safe. That is the read the
/// genre wants and it is legible without a HUD.
///
/// ⛔ ONE FUNCTION FOR ALL THREE EVADES. The spot dodge, the ground roll and the
/// air dodge each set their own window, and three copies of this arithmetic
/// would drift the first time one of them was tuned.
fn spend_evade(
    state: &mut super::AxisManeuverState,
    dodge: &mut crate::BodyDodgeState,
    window: f32,
    tuning: AxisSweptParams,
) -> f32 {
    let step = tuning.abilities.dodge_stale_step.max(0.0);
    let scale = if step <= 0.0 {
        // A game that declares no staling gets its authored window, untouched.
        1.0
    } else {
        (1.0 - step * f32::from(dodge.evades_recent))
            .max(tuning.abilities.dodge_stale_floor.clamp(0.0, 1.0))
    };
    // ⛔ SATURATING: a body that evades forever must not wrap its count back to
    // fresh, which is the one way this mechanic could reward spamming.
    dodge.evades_recent = dodge.evades_recent.saturating_add(1);
    // Each spend re-arms the full forgiveness delay, so the count only starts
    // coming down once the body actually stops.
    dodge.stale_decay = tuning.abilities.dodge_stale_recovery.max(0.0);
    // ⭐⭐ THE STALED VALUE ARMS THE I-FRAMES AND NOTHING ELSE. The authored
    // window goes back to the caller, which spends it on the MANEUVER clock:
    // travel, endlag and commitment are what the move is, and a worn-out evade
    // is still that move. Returning the staled number here is precisely the bug
    // this split fixes — it made a spammed roll SHORTER rather than unsafe.
    state.evade_invuln_timer = window * scale;
    window
}

/// The dash half of [`resolve_burst_maneuver`] — see [`available_dodge`].
fn dash_available(abilities: &BodyAbilities, dash: &BodyDashState) -> bool {
    abilities.abilities.dash && dash.charges_available > 0 && dash.cooldown <= 0.0
}

/// Facing + input buffering: turn to face the stick (only when grounded or
/// flying), and buffer jump/burst presses for the short windows the sim phase
/// consumes them in. The intent step at the head of the control phase.
/// One action class asked of the out-of-shield policy.
///
/// Named for CLASSES rather than moves on purpose: "up-smash may, forward-smash
/// may not" spelled move by move is the exception list this policy exists
/// instead of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutOfShieldAction {
    Jump,
    Burst,
    Grab,
    UpAttack,
    UpSpecial,
}

/// THE out-of-shield question, read once per press and asked identically by the
/// movement kernel and by the moveset trigger.
///
/// ⛔ ONE IMPLEMENTATION, HERE, and that is the point of the type: the rule
/// "raised guard + policy + action class → permitted" was written twice — once
/// in this kernel and once in `ambition_platformer2d::combat::moveset` — which is two
/// authorities over one policy. Combat reads the gate; what it adds is the
/// DIRECTION interpretation (only the up attack and up special RISE), which is
/// its own vocabulary and stays there.
#[derive(Clone, Copy, Debug)]
pub struct OutOfShieldGate {
    policy: Option<crate::OutOfShield>,
    /// A guard that is DOWN, or a game that declares no rule, restricts nothing
    /// — which is exactly what every body did before out-of-shield existed.
    unrestricted: bool,
}

impl OutOfShieldGate {
    /// `guard_up` is the guard's STATE, not the button: a guard comes down
    /// through drop lag, so the two disagree for a whole tick.
    pub fn read(guard_up: bool, policy: Option<crate::OutOfShield>) -> Self {
        Self {
            policy,
            unrestricted: !guard_up || policy.is_none(),
        }
    }

    /// May this action class begin from behind the guard at all?
    pub fn permits(&self, action: OutOfShieldAction) -> bool {
        self.unrestricted || self.policy.is_some_and(|policy| policy.permits(action))
    }

    /// Whether the guard is filtering anything — for a caller whose own rule is
    /// narrower than this one and that must not narrow an unguarded body.
    pub fn unrestricted(&self) -> bool {
        self.unrestricted
    }
}

/// Take an action OUT of this guard: under a declared policy, a raised guard
/// comes down with the action that left it.
///
/// ⭐ the second half of the same decision, and it has to travel with the
/// first: a body that could attack from behind a shield it keeps has no reason
/// to ever lower one. Inert for a game that declares no policy.
pub fn spend_out_of_shield(
    shield: &mut crate::body_clusters::BodyShieldState,
    policy: Option<crate::OutOfShield>,
) {
    if policy.is_some() && shield.active {
        shield.spend_on_action();
    }
}

pub(super) fn apply_intent(
    kinematics: &mut BodyKinematics,
    ground: &BodyGroundState,
    flight: &BodyFlightState,
    state: &mut AxisManeuverState,
    abilities: &BodyAbilities,
    input: InputState,
    tuning: AxisSweptParams,
    // THE GUARD, because starting an action out of one is a spend. WHICH
    // actions a raised one lets out is [`OutOfShieldGate`], owned here and read
    // by the move resolver too.
    shield: &mut crate::body_clusters::BodyShieldState,
    // The evade's own state, so a shield+direction press can ask whether a DODGE
    // is actually available rather than filling a buffer the dash will spend.
    dodge: &BodyDodgeState,
    // THIS PRESS IS A PLATFORM DROP, already decided by the kernel against the
    // surface under the body — see `wants_platform_drop` there. The evade must
    // stand down for it, and it cannot work that out itself: whether the floor
    // can be left downward is a question about the WORLD, which this layer has
    // no business holding.
    platform_drop: bool,
) {
    let oos = tuning.abilities.shield.out_of_shield;
    let gate = OutOfShieldGate::read(shield.active, oos);
    let can_turn = ground.on_ground || flight.fly_enabled;
    let local_stick = input.local_axis();
    // ⭐⭐ REVERSING OUT OF A COMMITTED RUN COSTS A TURNAROUND, and that is what
    // makes the initial dash's free reversal worth anything: inside the dash
    // you turn for nothing, out of a run you pay for it. Either half alone is
    // just a speed.
    //
    // ⛔ ONLY WHILE `running`, which is the gait line the body has actually
    // crossed — a body still inside its dash window is not running (see the
    // clause in `integrate_velocity_clusters`) and turns free, so dash-dancing
    // is untouched.
    //
    // ⛔ IT DELAYS THE FACING FLIP AND NOTHING ELSE. What the body does with
    // its velocity meanwhile is the ordinary run law's business; inventing a
    // skid here would be a second opinion about ground speed.
    let reversing =
        can_turn && local_stick.x.abs() > 0.1 && local_stick.x.signum() != kinematics.facing;
    // ⛔⛔ ON THE EDGE OF THE REQUEST, not on the condition. A body that is
    // still running and still asking to reverse satisfies this every tick, so
    // arming on the condition re-arms the phase the instant it expires and the
    // facing never flips at all — the body turns forever. `prev_steer_dir` is
    // the same edge the initial dash is entered on, and the turnaround's own
    // early-return keeps it fresh while the phase runs.
    // ⭐⭐ A TURNAROUND IS A GROUND PHASE, AND LEAVING THE FLOOR FINISHES IT —
    // the body takes into the air the facing it was already paying for. That is
    // what a REVERSE AERIAL RUSH is: turn, jump out of the turn, and your back
    // is pointed at where you came from while your momentum still carries you
    // there.
    //
    // ⛔ NOT A SEPARATE RAR STATE, which the inventory row rules out by name.
    // The rush emerges because the phase RESOLVES rather than being abandoned:
    // measured before this, a body that jumped mid-turnaround stayed facing its
    // old way forever, because an airborne body may not turn at all.
    //
    // ⛔ THE FLIP IS THE PHASE'S OWN, not the stick's. A turnaround was asked
    // for to reverse this facing, so finishing it reverses this facing — and a
    // player who let go of the stick on the way up still gets what they bought.
    if !ground.on_ground && state.turnaround_timer > 0.0 {
        kinematics.facing = -kinematics.facing;
        state.turnaround_timer = 0.0;
    }
    // ⛔⛔ BOTH SIDES OF THIS COMPARISON AT ONE THRESHOLD. `prev_steer_dir` is
    // written by the initial dash past a 0.5 deadzone; testing it against a bare
    // `local_stick.x.signum()` compared a value recorded at one threshold with a
    // value read at another, so a stick held around -0.2 was NEUTRAL to the
    // writer and A DEFINITE REVERSE to the reader — an edge that was true on
    // every tick.
    //
    // ⭐ SHARING THE DASH'S THRESHOLD RATHER THAN CARRYING A SECOND MEMORY. A
    // turnaround is a committed run being reversed, which is the same firm push
    // the dash asks for; and a separate `prev_turn_steer_dir` would be one more
    // ROLLBACK FIELD bought for a difference nothing can currently observe (the
    // facing snaps to the stick the moment the timer expires — see below — so
    // the phase cannot actually re-arm forever). Same threshold, no new state.
    // ⛔ THE STICK THE PLAYER IS HOLDING, because `prev_steer_dir` records that
    // one — comparing a damped read against an undamped memory is what made a
    // rooted move look like a release. The ARMING below still needs `reversing`,
    // which is the damped question.
    let steer_stick = input.steer_axis();
    let turn_dir = if steer_stick.x.abs() > super::integration::STEER_DEADZONE {
        steer_stick.x.signum()
    } else {
        0.0
    };
    let asked_now = state.prev_steer_dir != turn_dir;
    if tuning.locomotion.turnaround_time > 0.0
        && ground.on_ground
        && state.running
        && reversing
        && asked_now
        && state.turnaround_timer <= 0.0
    {
        state.turnaround_timer = tuning.locomotion.turnaround_time;
    }
    if can_turn && local_stick.x.abs() > 0.1 && state.turnaround_timer <= 0.0 {
        kinematics.facing = local_stick.x.signum();
    }
    // JUMP OUT OF SHIELD, the universal option — and the guard leaves with it.
    if input.jump_pressed() && abilities.abilities.jump && gate.permits(OutOfShieldAction::Jump) {
        state.buffer_jump = tuning.locomotion.jump_buffer;
        spend_out_of_shield(shield, oos);
    }
    // THE BURST PRESS IS GATED ON OWNING A BURST, NOT ON OWNING DASH.
    // This read `abilities.abilities.dash` alone, so a body authored
    // `dodge: true, dash: false` could never dodge: nothing ever filled the
    // buffer `apply_dodge` spends, and the dodge ability was inert on every body
    // that did not ALSO carry the traversal burst it has nothing to do with.
    // Invisible while the shipped fighter kit happened to author both.
    // `movement_actions` in the character action scheme already asks the
    // question this way (`dash || dodge` earns `ControlSlot::Burst`); the kernel
    // now agrees, and the slot is named for the channel rather than for one
    // outcome of it.
    if input.burst_pressed()
        && (abilities.abilities.dash || abilities.abilities.dodge)
        && gate.permits(OutOfShieldAction::Burst)
    {
        state.buffer_burst = tuning.abilities.dash_buffer;
        spend_out_of_shield(shield, oos);
    }
    // ⭐ DOWN ON A RAISED GUARD IS A SPOT DODGE, with no second button. Jon,
    // 2026-08-23: *"Shielding and pressing down should trigger a dodge. But also
    // note that right now dodge isn't really a dodge, it is more like a dash. It
    // actually moves the player."*
    //
    // Both halves of that are one gap. The SPOT DODGE already exists and already
    // evades in place — `apply_dodge` zeroes everything but the descent when the
    // stick is down — and the smash ruleset already authors `spot_dodge_time`.
    // What did not exist is any way to ASK for it from behind a guard: the
    // evade was reachable only through the burst button, and a burst press with
    // no direction is the ROLL, which moves. So the dodge Jon could actually get
    // was the one that travels.
    //
    // ⛔ THIS FILLS THE SAME BUFFER THE BURST BUTTON FILLS and decides nothing
    // else. `apply_dodge` reads the stick itself and picks spot-dodge over roll,
    // so this cannot disagree with it about which evade a down-held stick means
    // — there is one authority for that and it is not here.
    //
    // ⛔ Gated on the guard being ACTIVE, so walking down a slope is untouched:
    // this is an out-of-shield option, and it is spent like one.
    // ⛔ AND NOT WHEN THE SAME PRESS IS A PLATFORM DROP. Guard + down means the
    // spot dodge on solid ground and the drop on a soft platform — the terrain
    // arbitrates, and it arbitrates HERE because the evade runs first and would
    // otherwise spend the press before the drop road ever sees it.
    if !platform_drop
        && shield_evade_direction(input, abilities, ground, dodge, state, tuning).is_some()
        && gate.permits(OutOfShieldAction::Burst)
    {
        state.buffer_burst = tuning.abilities.dash_buffer;
        spend_out_of_shield(shield, oos);
    }
    // ⭐⭐ SHIELD IN THE AIR IS THE AIR DODGE, and in this genre it is the only
    // thing that button means up there. The grounded rule above needs a
    // DIRECTION because a guard is the other thing the press could have meant;
    // airborne there is nothing else, so a bare press is the whole gesture.
    //
    // ⛔⛔ ONLY WHERE THE BUTTON MEANS NOTHING ELSE. A ruleset that ALLOWS an
    // airborne guard (`ShieldTuning::air_guard` — Ambition's deployable bubble)
    // is a game where the press already has a meaning up there, and taking it as
    // a dodge too fires both: `a_deployable_bubble_still_guards_in_the_air` saw
    // exactly `[AirDodge, ShieldUp]` from one press before this clause existed.
    //
    // ⛔ NO OUT-OF-SHIELD SPEND, because there is no guard to leave: the same
    // condition that lets this fire is the one that refused the guard.
    //
    // ⛔ HELD, NOT AN EDGE, and it does not repeat: `air_dodge_spent` is
    // once-per-airtime, so a held button buys exactly one dodge per trip through
    // the air — the same shape the grounded evade gets from its cooldown, and
    // the reason no shield PRESS edge had to be invented.
    if input.shield_held
        && !ground.on_ground
        && !tuning.abilities.shield.air_guard
        && available_dodge(abilities, ground, dodge, state, tuning) == Some(BurstManeuver::AirDodge)
    {
        state.buffer_burst = tuning.abilities.dash_buffer;
    }
}

/// SHIELD PLUS A DIRECTION IS AN EVADE, and which one is the stick's business.
///
/// Jon, 2026-08-23: *"Rolls happen when the player is on ground and holding
/// shield and press left or right... if the player presses left+shield,
/// right+shield, or down+shield, they should get the corresponding roll or dodge
/// without bringing up the shield at all."*
///
/// ⛔ THE BUTTON, NOT `BodyShieldState::active`. The guard state outlives the
/// press that raised it, so reading it makes an evade fire off a shield the
/// player already let go of — the same tick error the shield-grab made and paid
/// for with a red premise guard.
///
/// ⛔ GROUNDED ONLY. In the air, shield is not a guard and the directional
/// evade is the air dodge, which the burst press already reaches.
///
/// Returns the local stick so the caller and [`apply_dodge`] cannot disagree
/// about whether a direction was given — `apply_dodge` reads the same axis to
/// choose spot-dodge over roll, and this only decides whether to ASK.
fn shield_evade_direction(
    input: InputState,
    abilities: &BodyAbilities,
    ground: &BodyGroundState,
    dodge: &BodyDodgeState,
    state: &AxisManeuverState,
    tuning: AxisSweptParams,
) -> Option<bevy_math::Vec2> {
    // ⛔⛔ ONLY WHEN A DODGE IS ACTUALLY AVAILABLE — never falling through to
    // the DASH. `apply_dodge` gives up when the evade is on cooldown and the
    // burst buffer then reaches `apply_dash`, so a shield+direction press with a
    // spent dodge would launch a 760px/s traversal dash out of a guard. That is
    // the same complaint Jon made about the dodge button: *"Agents have told me
    // the dash was removed many times, but it really never has been, at least
    // semantically."*
    //
    // ⭐ Measured: without this clause the guard test read 760 — the dash speed —
    // where it expected a body that had not moved at all.
    if available_dodge(abilities, ground, dodge, state, tuning) != Some(BurstManeuver::GroundDodge)
    {
        return None;
    }
    if !input.shield_held || !ground.on_ground {
        return None;
    }
    let stick = input.local_axis();
    let aimed = stick.y > crate::movement::tuning::SPOT_DODGE_STICK
        || stick.x.abs() > crate::movement::tuning::SPOT_DODGE_STICK;
    aimed.then_some(stick)
}

/// Flight toggle: flip fly mode; on entering, clear transient wall/dash/blink
/// maneuver state so the body cleanly enters free flight.
pub(super) fn apply_fly_toggle(
    flight: &mut BodyFlightState,
    state: &mut AxisManeuverState,
    abilities: &BodyAbilities,
    combo_trace: &mut BodyComboTrace,
    input: InputState,
    events: &mut FrameEvents,
) {
    if input.fly_toggle_pressed() && abilities.abilities.fly && abilities.abilities.fly_toggle {
        flight.fly_enabled = !flight.fly_enabled;
        if flight.fly_enabled {
            state.fast_falling = false;
            state.wall_clinging = false;
            state.wall_climbing = false;
            state.dash_timer = 0.0;
            state.blink_grace_timer = 0.0;
            state.phased_jump.clear();
        }
        events.op_clusters(combo_trace, MovementOp::FlyToggle);
    }
}

/// The evade, on the ground and in the air. A buffered burst press spent by
/// a body that owns the dodge ability becomes a roll when its feet are down and
/// an air dodge when they are not (the dodge ability claims
/// [`AxisManeuverState::buffer_burst`] before `apply_dash` would).
///
/// The two share the buffer, the ability gate and the i-frame *idea*, and
/// nothing else — see [`AxisManeuverState::air_dodge_timer`] for why they do not
/// share a timer. Their commitments differ in kind:
///
/// ```text
///                  gate                travel            spent against
/// ground roll      on_ground           facing/stick x     a cooldown clock
/// air dodge        airborne, unspent   full 2D stick      this trip through the air
/// ```
///
/// an air dodge with `air_dodge_time <= 0.0` is a body that has none. The
/// tuning defaults are `#[serde(default)]`, so every authored body baked before
/// the maneuver existed keeps exactly the movement it had; a body opts in by
/// authoring a window.
pub(super) fn apply_dodge(
    kinematics: &mut BodyKinematics,
    dodge: &mut BodyDodgeState,
    state: &mut AxisManeuverState,
    ground: &BodyGroundState,
    abilities: &BodyAbilities,
    combo_trace: &mut BodyComboTrace,
    input: InputState,
    frame: MotionFrame,
    tuning: AxisSweptParams,
    events: &mut FrameEvents,
) {
    if state.buffer_burst <= 0.0 {
        return;
    }
    // The ONE availability rule (see [`BurstManeuver`]) — the same expression
    // autonomous perception reads, so a driver that decided "dodge" and a body
    // that performs one cannot disagree.
    let Some(evade) = available_dodge(abilities, ground, dodge, state, tuning) else {
        return;
    };
    let local_stick = input.local_axis();
    if evade == BurstManeuver::GroundDodge {
        // SPOT DODGE — down on the stick evades IN PLACE. The grounded
        // evade had exactly one shape, so the option a fighter takes when there
        // is nowhere to roll TO — cornered, on a platform, waiting out a
        // committed swing — did not exist. a body that authors no window keeps
        // the roll it always had: the press is not taken away from anybody.
        // ⭐ A NEUTRAL DODGE EVADES IN PLACE. Jon, 2026-08-23: *"when I 'press
        // C' on the keyboard to 'dodge' as the button says, I move horizontally
        // like a dash. Agents have told me the dash was removed many times, but
        // it really never has been, at least semantically."* A neutral burst
        // press used to fall to the ROLL below, which picks `facing` when the
        // stick says nothing and travels `dodge_roll_speed` — a button labelled
        // DODGE that moves you.
        //
        // ⛔⛔ THIS WAS REVERTED ONCE ON A FALSE ATTRIBUTION, and the reason is
        // worth more than the clause. `the_cpu_charges_a_smash_and_techs_a_landing_in_some_match`
        // went red the day it first landed, reverting it alone went green, and
        // that was read as causation. It cannot be: the fighter brain's `Dodge`
        // verb AIMS its stick, and the branch below only fires on
        // `local_stick.y > SPOT_DODGE_STICK`, which an aimed horizontal roll
        // never satisfies. A CPU dodge cannot reach this clause at all.
        //
        // ⇒ re-established by MEASUREMENT, not by argument:
        // `a_fighter_brain_charges_a_smash_through_the_real_chain` walks brain →
        // gesture → move → `MoveCharge` → frozen fraction with one motionless
        // opponent and no sampling in it, and charges identically with this
        // clause in. The emergent test that vetoed it passes too. The original
        // failure was a match-trajectory effect, which is what an emergent test
        // measures and what it cannot attribute.
        if (local_stick.y > crate::movement::tuning::SPOT_DODGE_STICK
            || local_stick.length_squared() < 0.01)
            && tuning.abilities.spot_dodge_time > 0.0
        {
            // the DESCENT is kept and everything else zeroed: a body standing
            // on a floor has none, and one that spot-dodged off a ledge edge
            // must not hang in the air for the window.
            kinematics.vel = frame.down() * kinematics.vel.dot(frame.down()).max(0.0);
            state.dodge_roll_timer =
                spend_evade(state, dodge, tuning.abilities.spot_dodge_time, tuning);
            state.spot_dodging = true;
            state.phased_jump.clear();
            dodge.cooldown = tuning.abilities.dodge_roll_cooldown;
            state.buffer_burst = 0.0;
            events.op_clusters(combo_trace, MovementOp::SpotDodge);
            return;
        }
        let dir = if local_stick.x.abs() > 0.1 {
            local_stick.x.signum()
        } else {
            kinematics.facing
        };
        let descend = kinematics.vel.dot(frame.down()).min(0.0);
        kinematics.vel =
            frame.side() * (dir * tuning.abilities.dodge_roll_speed) + frame.down() * descend;
        state.dodge_roll_timer =
            spend_evade(state, dodge, tuning.abilities.dodge_roll_time, tuning);
        // What this roll ADDED, so its end can take back exactly that and
        // nothing a hit or a platform contributed. See `dodge_roll_push`.
        state.dodge_roll_push = dir * tuning.abilities.dodge_roll_speed;
        state.spot_dodging = false;
        state.phased_jump.clear();
        dodge.cooldown = tuning.abilities.dodge_roll_cooldown;
        state.buffer_burst = 0.0;
        events.op_clusters(combo_trace, MovementOp::DodgeRoll);
        return;
    }
    // ── airborne ────────────────────────────────────────────────────────────
    // The stick aims the evade in the body's own frame: sideways, up, down or
    // any diagonal. A neutral stick dodges in place — a real option, not a
    // degenerate one, because the invulnerability is the point and standing
    // still keeps the body where its drift left it.
    let aim = if local_stick.length_squared() > 0.01 {
        local_stick.normalize()
    } else {
        bevy_math::Vec2::ZERO
    };
    // local `y` points toward the FEET, the same convention
    // `wants_drop_through` reads — so the stick's y composes with `down()`, not
    // against it. Negating here would have aimed every "dodge down through the
    // stage" upward, which is the exact input a recovering body uses.
    kinematics.vel =
        (frame.side() * aim.x + frame.down() * aim.y) * tuning.abilities.air_dodge_speed;
    state.air_dodge_timer = spend_evade(state, dodge, tuning.abilities.air_dodge_time, tuning);
    state.air_dodge_endlag_timer = 0.0;
    state.fast_falling = false;
    state.phased_jump.clear();
    dodge.air_dodge_spent = true;
    state.buffer_burst = 0.0;
    events.op_clusters(combo_trace, MovementOp::AirDodge);
}

/// The ONE shield-activation rule, shared by the player body and every actor body
/// (roadmap S6b convergence / invariant I3 — the body owns the gate, the
/// controller only attempts). Given the controller's held-shield attempt and the
/// body's gates — does it have the shield ability, and is it mid-dash (you can't
/// raise a guard while dashing) — it resolves the raised state and refreshes the
/// parry window on the *rising edge*. Returns `true` iff a FRESH guard was raised
/// this tick (the edge that opens a parry window / emits a `ShieldUp` op), so the
/// caller can fire its own side effect. Pure + frame-agnostic.
///
/// Split from [`apply_shield`] so the raise/parry edge can be unit-tested
/// against bare scalars, without a body around it.
pub fn resolve_shield(
    active: &mut bool,
    parry_window_timer: &mut f32,
    ability_enabled: bool,
    dash_active: bool,
    shield_held: bool,
    parry_window_time: f32,
    // WHICH GAME'S PERFECT SHIELD this body plays with. See
    // [`crate::ParryTiming`] — Smash 4 opens the window on the press, Ultimate on
    // the release, and both are settings rather than candidates.
    parry_timing: crate::ParryTiming,
    // A broken guard cannot be raised until the dizzy runs out — the whole
    // point of breaking it.
    broken: bool,
    // ⛔ WHETHER THIS BODY MAY GUARD WHERE IT IS — see `ShieldTuning::air_guard`.
    //
    // ⛔⛔ IT GATES THE SUSTAIN AS WELL AS THE RAISE, and reading it as
    // raise-only was a live contradiction. It said `may_guard_here || *active`,
    // on the argument that a body which left the ground guarding "has not made a
    // new decision" — but under `air_guard: false` a held Shield ALSO fills the
    // air-dodge buffer the moment the body is airborne, so walking off a ledge
    // with the guard up produced the one state the policy exists to forbid: an
    // active ground shield and an air dodge in the same tick. The genre's answer
    // is the plain one — leaving the ground drops the guard, which is what makes
    // jumping out of shield a commitment.
    may_guard_here: bool,
) -> bool {
    if !ability_enabled || broken {
        *active = false;
        *parry_window_timer = 0.0;
        return false;
    }
    let want = shield_held && !dash_active && may_guard_here;
    let fresh = want && !*active;
    let released = *active && !want;
    match parry_timing {
        crate::ParryTiming::OnRaise => {
            if fresh {
                *parry_window_timer = parry_window_time;
            }
            // dropping the guard ENDS the window it opened. Without this the
            // press-timed parry would keep covering a body that is no longer
            // guarding, which is the release-timed reading arrived at by
            // accident rather than by declaration.
            if released {
                *parry_window_timer = 0.0;
            }
        }
        // the FALLING edge arms it, and nothing closes it early: the window is
        // live while the guard is DOWN, which is the whole mechanic.
        crate::ParryTiming::OnRelease => {
            if released {
                *parry_window_timer = parry_window_time;
            }
        }
    }
    *active = want;
    fresh
}

/// Shield / parry hold. Can't raise while dashing; opens a parry window on the
/// rising edge. Thin player-side wrapper over the shared [`resolve_shield`] rule.
pub(super) fn apply_shield(
    shield: &mut BodyShieldState,
    state: &AxisManeuverState,
    // WHERE the body is, because a ruleset may refuse an airborne guard.
    ground: &BodyGroundState,
    abilities: &BodyAbilities,
    combo_trace: &mut BodyComboTrace,
    input: InputState,
    tuning: AxisSweptParams,
    events: &mut FrameEvents,
) {
    let broken = shield.broken();
    // THE LOCK CLEARS WHEN THE BUTTON DOES, and nowhere else. A guard spent on
    // an out-of-shield action stays down for as long as the press that raised
    // it lasts; letting go and pressing again is a new guard.
    if !input.shield_held {
        shield.release_locked = false;
    }
    let was_up = shield.active;
    let fresh = resolve_shield(
        &mut shield.active,
        &mut shield.parry_window_timer,
        abilities.abilities.shield && !shield.release_locked,
        state.dash_timer > 0.0,
        input.shield_held,
        tuning.abilities.parry_window_time,
        tuning.abilities.parry_timing,
        broken,
        ground.on_ground || tuning.abilities.shield.air_guard,
    );
    if fresh {
        events.op_clusters(combo_trace, MovementOp::ShieldUp);
    }
    // SHIELD DROP LAG — the cost of lowering a guard by ITSELF. An
    // out-of-shield action already took the guard down through
    // `spend_on_action`, and this is the other road: you simply let go, and the
    // genre charges you for it. `0.0` for a game that declares no rule, which
    // is what dropping always cost.
    //
    // ⛔⛔ "YOU SIMPLY LET GO" IS A CAUSE, AND THIS ASKED ONLY THE EFFECT. The
    // condition was `was_up && !active`, which is every way a guard can end —
    // including the ones the player did not choose. Under `air_guard: false`
    // (Smash) a fighter that DROPS THROUGH A PLATFORM with Shield still held
    // becomes airborne, the guard is forced down, and this billed the full
    // release penalty — 11 frames of `drop_lag_timer`, which
    // `apply_post_hit_input_gates` folds into `hard_lock_timer`. The same
    // misreading catches a shield that BREAKS and one whose ability goes away.
    //
    // ⭐ THE HAND IS THE AUTHORITY: the button is no longer held, so the player
    // let go. Every other way a guard ends is somebody else's decision and owes
    // nothing.
    let voluntary_release = was_up && !input.shield_held && !shield.release_locked;
    if voluntary_release && !shield.active && tuning.abilities.shield.drop_lag > 0.0 {
        shield.drop_lag_timer = shield.drop_lag_timer.max(tuning.abilities.shield.drop_lag);
    }
    // SHIELD TILT — where the raised guard sits on the body.
    //
    // ⛔ RESOLVED HERE, not at the hit. The coverage rule and the view both
    // read this one value, and a guard the picture draws high while the hit
    // test still centres it is the disagreement this exists to prevent.
    //
    // The WHOLE stick, with no threshold of its own: past
    // `SPOT_DODGE_STICK` the evade normally takes the input first, so what
    // reaches the tilt in practice is the small band — but on a body whose
    // evade is spent or on cooldown the stick has nowhere else to go, and
    // leaning is exactly what it should still buy. A guard that is DOWN leans
    // nowhere.
    shield.shield_tilt = if shield.active {
        tuning.abilities.shield.tilt_bias(input.local_axis().y)
    } else {
        0.0
    };
}

/// Variable jump height: cut the rising jump short on an early button release.
pub(super) fn apply_jump_release(
    kinematics: &mut BodyKinematics,
    state: &mut AxisManeuverState,
    abilities: &BodyAbilities,
    input: InputState,
    frame: MotionFrame,
    tuning: AxisSweptParams,
) {
    if !input.jump_released() {
        return;
    }
    cut_ascent_now(kinematics, state, abilities, frame, tuning);
}

/// The variable-jump CUT itself, with the release edge already established by
/// the caller. Split out because a jump-squat swallows the release edge — the
/// button can come up mid-crouch, when there is no ascent to shorten — so the
/// squat's takeoff replays the cut at the instant the body actually leaves the
/// ground. that is a deferred release, not a second "short hop" mechanic:
/// whichever [`super::tuning::AxisJumpLaw`] the body authored still decides
/// what shortening means.
pub(super) fn cut_ascent_now(
    kinematics: &mut BodyKinematics,
    state: &mut AxisManeuverState,
    abilities: &BodyAbilities,
    frame: MotionFrame,
    tuning: AxisSweptParams,
) {
    if !abilities.abilities.variable_jump {
        return;
    }
    match tuning.locomotion.jump_law {
        super::tuning::AxisJumpLaw::VelocityCut => {
            let ascend_speed = -kinematics.vel.dot(frame.down());
            if ascend_speed > 120.0 {
                let along_down = kinematics.vel.dot(frame.down());
                kinematics.vel += frame.down() * (along_down * 0.54 - along_down);
            }
        }
        super::tuning::AxisJumpLaw::PhasedGravity(_) => {
            // Do not rewrite velocity. The next integration tick observes the
            // latched release and applies the stronger gravity phase in the
            // body's current resolved frame.
            state.phased_jump.cancel_hold();
        }
    }
}

/// Dash: a buffered, charge-gated burst that REPLACES the velocity vector and
/// opens a timed window during which the integrator skips normal physics (see
/// `integrate_velocity_clusters`'s `dash.timer > 0` branch). Picks Dash vs
/// DoubleDash by the charge count before decrement. No-op unless the actor has
/// the dash ability + a buffered press + a free charge + the cooldown clear, so
/// an actor without dash (`abilities.dash == false`) pays only the gate check —
/// note it may still have BUFFERED the press, because the burst button belongs
/// to dodge as well.
///
/// Order: runs in the CONTROL phase after the input buffer is populated and
/// after dodge (which consumes the same buffer on the ground first).
pub(super) fn apply_dash(
    kinematics: &mut BodyKinematics,
    dash: &mut BodyDashState,
    state: &mut AxisManeuverState,
    abilities: &BodyAbilities,
    combo_trace: &mut BodyComboTrace,
    input: InputState,
    frame: MotionFrame,
    tuning: AxisSweptParams,
    events: &mut FrameEvents,
) {
    // Same shared rule as the dodge step above — see [`BurstManeuver`].
    if state.buffer_burst > 0.0 && dash_available(abilities, dash) {
        let fallback = bevy_math::Vec2::new(kinematics.facing, 0.0);
        let aim = input.local_axis().normalize_or(fallback);
        kinematics.vel = frame.to_world(aim) * tuning.abilities.dash_speed;
        state.dash_timer = tuning.abilities.dash_time;
        state.phased_jump.clear();
        dash.cooldown = tuning.abilities.dash_cooldown;
        state.buffer_burst = 0.0;
        let before = dash.charges_available;
        dash.charges_available = dash.charges_available.saturating_sub(1);
        let op = if before >= 2 {
            MovementOp::DoubleDash
        } else {
            MovementOp::Dash
        };
        events.op_clusters(combo_trace, op);
    }
}

#[cfg(test)]
mod burst_maneuver_tests {
    use super::*;
    use crate::body_clusters::{
        BodyAbilities, BodyComboTrace, BodyDashState, BodyDodgeState, BodyGroundState,
        BodyKinematics,
    };
    use crate::movement::events::FrameEvents;
    use crate::movement::input::InputState;
    use crate::movement::model::AxisManeuverState;
    use crate::movement::tuning::AxisSweptParams;
    use crate::MotionFrame;

    /// A body that owns BOTH verbs — the shipped Smash fighter (P4.30).
    fn both_abilities() -> BodyAbilities {
        let mut abilities = BodyAbilities::default();
        abilities.abilities.dodge = true;
        abilities.abilities.dash = true;
        abilities
    }

    /// A BODY THAT OWNS THE DODGE AND NOT THE DASH ACTUALLY DODGES.
    #[test]
    fn the_burst_press_belongs_to_whichever_burst_the_body_owns() {
        let tuning = AxisSweptParams::default();
        let mut input = InputState::default();
        input
            .movement
            .set_pressed(crate::movement::input::MovementAction::Burst, true);
        let grounded = BodyGroundState {
            head_contact: false,
            on_ground: true,
            ..Default::default()
        };

        let kit = |dodge: bool, dash: bool| {
            let mut abilities = BodyAbilities::default();
            abilities.abilities.dodge = dodge;
            abilities.abilities.dash = dash;
            abilities
        };

        // (label, abilities, expected maneuver)
        let cases = [
            (
                "dodge only — the D146 smash fighter",
                kit(true, false),
                BurstManeuver::GroundDodge,
            ),
            (
                "dash only — Ambition's traversal kit",
                kit(false, true),
                BurstManeuver::Dash,
            ),
            (
                "neither — the press means nothing",
                kit(false, false),
                BurstManeuver::None,
            ),
        ];

        for (label, abilities, expected) in cases {
            let mut kinematics = BodyKinematics::default();
            let mut state = AxisManeuverState::default();
            let mut combo_trace = BodyComboTrace::default();
            let flight = crate::body_clusters::BodyFlightState::default();
            apply_intent(
                &mut kinematics,
                &grounded,
                &flight,
                &mut state,
                &abilities,
                input,
                tuning,
                // Guard down: this case is about which BURST a body owns.
                &mut crate::body_clusters::BodyShieldState::default(),
                &crate::body_clusters::BodyDodgeState::default(),
                // No platform under this fixture at all.
                false,
            );
            assert_eq!(
                state.buffer_burst > 0.0,
                expected != BurstManeuver::None,
                "{label}: the press was {} but the body {} a burst it owns",
                if state.buffer_burst > 0.0 {
                    "buffered"
                } else {
                    "dropped"
                },
                if expected == BurstManeuver::None {
                    "owns no"
                } else {
                    "owns"
                },
            );
            let mut dodge = BodyDodgeState::default();
            let mut dash = BodyDashState {
                charges_available: 1,
                ..Default::default()
            };
            // Carry the buffer `apply_intent` just decided into the real steps.
            let mut run_state = state;
            let performed = {
                let mut events = FrameEvents::default();
                let frame = MotionFrame::from_direction(bevy_math::Vec2::new(0.0, 1.0), 900.0);
                apply_dodge(
                    &mut kinematics,
                    &mut dodge,
                    &mut run_state,
                    &grounded,
                    &abilities,
                    &mut combo_trace,
                    input,
                    frame,
                    tuning,
                    &mut events,
                );
                let dodged = run_state.dodge_roll_timer > 0.0;
                apply_dash(
                    &mut kinematics,
                    &mut dash,
                    &mut run_state,
                    &abilities,
                    &mut combo_trace,
                    input,
                    frame,
                    tuning,
                    &mut events,
                );
                if dodged {
                    BurstManeuver::GroundDodge
                } else if run_state.dash_timer > 0.0 {
                    BurstManeuver::Dash
                } else {
                    BurstManeuver::None
                }
            };
            assert_eq!(
                performed, expected,
                "{label}: the body performed {performed:?} on a burst press"
            );
        }
    }

    /// Run the kernel's two burst steps in their real order against a buffered
    /// press, and report which maneuver the BODY performed.
    fn what_the_body_does(
        abilities: &BodyAbilities,
        ground: &BodyGroundState,
        dodge: &mut BodyDodgeState,
        dash: &mut BodyDashState,
        tuning: AxisSweptParams,
    ) -> BurstManeuver {
        let mut kinematics = BodyKinematics::default();
        let mut state = AxisManeuverState::default();
        state.buffer_burst = 0.1;
        let mut combo_trace = BodyComboTrace::default();
        let mut events = FrameEvents::default();
        let frame = MotionFrame::from_direction(bevy_math::Vec2::new(0.0, 1.0), 900.0);
        let input = InputState::default();

        apply_dodge(
            &mut kinematics,
            dodge,
            &mut state,
            ground,
            abilities,
            &mut combo_trace,
            input,
            frame,
            tuning,
            &mut events,
        );
        let dodged = if state.dodge_roll_timer > 0.0 {
            Some(BurstManeuver::GroundDodge)
        } else if state.air_dodge_timer > 0.0 {
            Some(BurstManeuver::AirDodge)
        } else {
            None
        };
        apply_dash(
            &mut kinematics,
            dash,
            &mut state,
            abilities,
            &mut combo_trace,
            input,
            frame,
            tuning,
            &mut events,
        );
        match dodged {
            Some(evade) => evade,
            None if state.dash_timer > 0.0 => BurstManeuver::Dash,
            None => BurstManeuver::None,
        }
    }

    /// THE RESOLVER AND THE BODY ANSWER THE SAME QUESTION.
    ///
    /// this is the whole reason [`resolve_burst_maneuver`] exists. An
    /// autonomous driver was choosing its maneuver from the body's
    /// CAPABILITIES, and a dodge on cooldown declines without consuming the
    /// buffered press — so `apply_dash` takes it and the body dashes while the
    /// brain goes on saying "dodge". Every state below is a state a Smash
    /// fighter is in several times a match.
    ///
    /// the second row is the poison: on a body owning both, the ONLY thing
    /// separating a roll from a dash is the cooldown, so a resolver that
    /// answered from capabilities would give the same answer for both rows and
    /// this test would be measuring nothing.
    #[test]
    fn what_the_resolver_says_is_what_the_body_does() {
        let abilities = both_abilities();
        // NOT `AxisSweptParams::default()` UNMODIFIED. Its
        // `air_dodge_time` is 0.0, so on default tuning the air-dodge branch can
        // never fire and both airborne rows below would agree for the wrong
        // reason — the `AirDodge` variant would go entirely unexercised while
        // the test read as covering it. A fixture that cannot reach a case is
        // not covering it.
        let mut tuning = AxisSweptParams::default();
        tuning.abilities.air_dodge_time = 0.3;
        let grounded = BodyGroundState {
            head_contact: false,
            on_ground: true,
            ..Default::default()
        };
        let airborne = BodyGroundState::default();

        let cases: [(&str, BodyGroundState, BodyDodgeState, BodyDashState); 5] = [
            (
                "grounded, everything ready",
                grounded.clone(),
                BodyDodgeState::default(),
                BodyDashState {
                    charges_available: 1,
                    ..Default::default()
                },
            ),
            (
                "grounded, the dodge is on cooldown — THE PRESS DASHES",
                grounded.clone(),
                BodyDodgeState {
                    cooldown: 0.4,
                    ..Default::default()
                },
                BodyDashState {
                    charges_available: 1,
                    ..Default::default()
                },
            ),
            (
                "airborne with the air dodge in hand",
                airborne.clone(),
                BodyDodgeState::default(),
                BodyDashState {
                    charges_available: 1,
                    ..Default::default()
                },
            ),
            (
                "airborne, the air dodge is spent — THE PRESS DASHES",
                airborne.clone(),
                BodyDodgeState {
                    air_dodge_spent: true,
                    ..Default::default()
                },
                BodyDashState {
                    charges_available: 1,
                    ..Default::default()
                },
            ),
            (
                "grounded, dodge on cooldown and no dash charge — nothing happens",
                grounded.clone(),
                BodyDodgeState {
                    cooldown: 0.4,
                    ..Default::default()
                },
                BodyDashState::default(),
            ),
        ];

        let mut seen = Vec::new();
        for (label, ground, dodge, dash) in cases {
            let predicted = resolve_burst_maneuver(
                &abilities,
                &ground,
                &dodge,
                &AxisManeuverState::default(),
                &dash,
                tuning,
            );
            let (mut dodge, mut dash) = (dodge, dash);
            let performed = what_the_body_does(&abilities, &ground, &mut dodge, &mut dash, tuning);
            assert_eq!(
                predicted, performed,
                "{label}: the resolver said {predicted:?} and the kernel did \
                 {performed:?} — a brain that trusted the resolver would have \
                 named a maneuver its own body did not perform"
            );
            seen.push(predicted);
        }

        assert!(
            seen.contains(&BurstManeuver::Dash),
            "poison: at least one case must resolve to Dash on a body that OWNS \
             the dodge, or availability and capability are not being told apart \
             and this test proves nothing: {seen:?}"
        );
        assert!(
            seen.contains(&BurstManeuver::GroundDodge) && seen.contains(&BurstManeuver::AirDodge),
            "and BOTH evades must be reached, or one of the two is unexercised \
             while the test reads as covering it: {seen:?}"
        );
    }
}

#[cfg(test)]
mod resolve_shield_tests {
    use super::resolve_shield;

    /// The shared rule's contract (the one both the player wrapper and the actor
    /// resolver depend on): ability-gated, rising-edge parry, dash-blocked, sustain.
    #[test]
    fn resolve_shield_is_the_one_rule() {
        // Disabled ability forces the guard down and clears the parry window.
        let (mut active, mut parry) = (true, 0.5);
        let fresh = resolve_shield(
            &mut active,
            &mut parry,
            false,
            false,
            true,
            0.2,
            crate::ParryTiming::OnRaise,
            false,
            true,
        );
        assert!(!active && parry == 0.0 && !fresh, "no ability → no guard");

        // Rising edge: a held shield with the ability raises a FRESH guard and opens
        // the parry window.
        let (mut active, mut parry) = (false, 0.0);
        let fresh = resolve_shield(
            &mut active,
            &mut parry,
            true,
            false,
            true,
            0.2,
            crate::ParryTiming::OnRaise,
            false,
            true,
        );
        assert!(
            active && parry == 0.2 && fresh,
            "rising edge opens a fresh parry"
        );

        // Held across a second tick: still raised, but NOT a fresh edge (no re-arm).
        let fresh = resolve_shield(
            &mut active,
            &mut parry,
            true,
            false,
            true,
            0.2,
            crate::ParryTiming::OnRaise,
            false,
            true,
        );
        assert!(active && !fresh, "sustained hold is not a fresh parry");

        // Can't raise while dashing — the gate that binds the player AND the actor.
        let (mut active, mut parry) = (false, 0.0);
        let fresh = resolve_shield(
            &mut active,
            &mut parry,
            true,
            true,
            true,
            0.2,
            crate::ParryTiming::OnRaise,
            false,
            true,
        );
        assert!(!active && !fresh, "dashing blocks the guard");

        // Release drops the guard (sustain re-evaluated every tick) AND ends the
        // window it opened — see the `OnRaise` arm.
        let (mut active, mut parry) = (true, 0.2);
        resolve_shield(
            &mut active,
            &mut parry,
            true,
            false,
            false,
            0.2,
            crate::ParryTiming::OnRaise,
            false,
            true,
        );
        assert!(!active, "releasing the button drops the guard");
        assert_eq!(parry, 0.0, "a press-timed window outlived the guard");
    }

    /// THE TWO SETTINGS ARE TWO GAMES, AND BOTH ARE REACHABLE.
    ///
    /// ultimate, so release style shielding is in scope as an option."*
    ///
    /// the pair is the assertion. A test of `OnRelease` alone would pass
    /// on an implementation that had simply MOVED the parry rather than made it
    /// a knob, and that is the change this is deliberately not: `OnRaise` is
    /// Smash 4's and stays the default, so no shipped body's feel moves.
    #[test]
    fn the_parry_window_opens_where_the_ruleset_says_it_does() {
        // ── Ultimate: the FALLING edge arms it, and the window is live with the
        // guard DOWN.
        let (mut active, mut parry) = (false, 0.0);
        resolve_shield(
            &mut active,
            &mut parry,
            true,
            false,
            true,
            0.2,
            crate::ParryTiming::OnRelease,
            false,
            true,
        );
        assert!(active, "the guard still goes up");
        assert_eq!(parry, 0.0, "a release-timed parry armed on the PRESS");

        resolve_shield(
            &mut active,
            &mut parry,
            true,
            false,
            false,
            0.2,
            crate::ParryTiming::OnRelease,
            false,
            true,
        );
        assert!(!active, "the guard came down");
        assert_eq!(parry, 0.2, "the release did not open the window");
        // and it is a PARRY while the guard is down, which is the whole
        // mechanic and the reason `parrying()` cannot ask for `active`.
        let shield = crate::BodyShieldState {
            active,
            parry_window_timer: parry,
            ..Default::default()
        };
        assert!(shield.parrying(), "the window is live but nothing reads it");

        // ── Smash 4: the RISING edge arms it, and the drop ends it.
        let (mut active, mut parry) = (false, 0.0);
        resolve_shield(
            &mut active,
            &mut parry,
            true,
            false,
            true,
            0.2,
            crate::ParryTiming::OnRaise,
            false,
            true,
        );
        assert_eq!(parry, 0.2, "a press-timed parry did not arm on the press");
        resolve_shield(
            &mut active,
            &mut parry,
            true,
            false,
            false,
            0.2,
            crate::ParryTiming::OnRaise,
            false,
            true,
        );
        assert_eq!(parry, 0.0, "a press-timed window survived the drop");
    }
}
