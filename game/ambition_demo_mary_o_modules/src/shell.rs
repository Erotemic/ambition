//! The Solid Snake's shell: a stomp withdraws the same live body into an inert
//! shell, a kick sends the shell sliding, and the snake comes back out.
//!
//! A stomp never kills or replaces the body. While it is withdrawn its control
//! is held (a scripted beat, `ambition.body.control_hold`) and its touch hurts
//! nobody (`ambition.body.contact_threat`); emerging gives both back. A kicked
//! shell drives its own horizontal velocity, and while it runs it is a hazard
//! to every body it covers (`ambition.combat.kinetic_hazard`), credited to the
//! body that kicked it and sparing that body for a grace. The shared top/side
//! rule ([`crate::stomp::player_touch`]) makes a stomp safe in every phase;
//! only a side touch on an armed running shell hurts.
//!
//! It decides in `body_contact`, which invokes it for each snake every tick
//! whether or not anything touches it: the timers run on the tick. The phase,
//! its time left, the slide's direction and the kick's credit are the body's
//! record ([`SHELL`]). The timings are her pack's (`mary_o.rules.shell`), and
//! the stomp's band, bounce and dust are the ones every stomp of hers uses
//! (`mary_o.rules.stomp`). [`step_snake_shell`] is the pure phase logic.

use ambition_combat_port::motion::{SetVelocity, SetVelocityPort};
use ambition_combat_port::{
    BodyContact, BodyContactPort, Burst, BurstPort, ContactBounce, ContactBouncePort, ContactCue, ContactCuePort,
    ContactThreat, ContactThreatPort, ControlHold, ControlHoldPort, KineticHazard, KineticHazardPort, PosePin,
    PosePinPort,
};
use ambition_extension_sdk::{
    phases::BODY_CONTACT, Attachment, CodeIdentity, EntryCode, EntryDescriptor, Fault, FieldDecl, FieldKind, FieldRef,
    IdlePolicy, Invocation, Limits, ModuleDescriptor, ModuleKey, Port, Record, RefKind, SaveEligibility, SchemaKey,
    StateSchema, TriggerBinding, Value, API_VERSION,
};

use crate::rules::{ShellRules, ShellRulesPort, StompRulesPort};
use crate::stomp::{player_touch, Bounds, PlayerTouch};

/// The brain key of a Solid Snake placement, namespaced so it never collides
/// with a host provider's roster.
pub const SNAKE_BRAIN_KEY: &str = "mary_o_snake";

/// The shell's record on each snake.
pub const SHELL: SchemaKey = SchemaKey::new(crate::PROVIDER, "snake.shell", 1);

const PHASE: FieldRef = FieldRef(0);
const LEFT: FieldRef = FieldRef(1);
const DIR: FieldRef = FieldRef(2);
const CREDIT: FieldRef = FieldRef(3);

/// The record of [`SHELL`]: a new snake's is a walker's (every field at its
/// zero).
pub fn shell_schema() -> StateSchema {
    StateSchema {
        key: SHELL,
        attachment: Attachment::Body,
        save: SaveEligibility::Transient,
        fields: vec![
            // 0 walking, 1 retreating, 2 boxed, 3 sliding, 4 peeking, 5 emerging.
            FieldDecl::new(1, "phase", FieldKind::U32),
            // The timed stage's time left; a slide's grace.
            FieldDecl::new(2, "left", FieldKind::F32),
            // A slide's direction: -1 is leftward, 1 rightward.
            FieldDecl::new(3, "dir", FieldKind::F32),
            // The body credited for the running shell's kick.
            FieldDecl::new(4, "credit", FieldKind::Opt(Box::new(FieldKind::Ref(RefKind::Body)))),
        ],
    }
}

/// A Solid Snake's shell lifecycle. `Walking` is the ordinary patroller; the
/// rest are the withdraw cycle. Each timed stage's `f32` is the time it has
/// left.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum SnakeShell {
    #[default]
    Walking,
    Retreating(f32),
    Boxed(f32),
    /// Sliding along the ground.
    Sliding {
        /// Travel direction: `-1.0` is leftward, `1.0` rightward.
        dir: f32,
        /// Seconds left of immunity for the body that kicked the shell. A
        /// wall bounce ends the immunity.
        grace: f32,
    },
    Peeking(f32),
    Emerging(f32),
}

impl SnakeShell {
    /// The phase a [`SHELL`] record holds. A value the record cannot hold is
    /// a walker.
    pub fn of_record(record: &Record) -> SnakeShell {
        let number = |field| record.get(field).ok().and_then(Value::as_f32).unwrap_or(0.0);
        let left = number(LEFT);
        match record.get(PHASE).ok().and_then(Value::as_u32).unwrap_or(0) {
            1 => SnakeShell::Retreating(left),
            2 => SnakeShell::Boxed(left),
            3 => SnakeShell::Sliding { dir: number(DIR), grace: left },
            4 => SnakeShell::Peeking(left),
            5 => SnakeShell::Emerging(left),
            _ => SnakeShell::Walking,
        }
    }

    /// Write this phase into a [`SHELL`] record. The credit is the record's.
    pub fn write(self, record: &mut Record) -> Result<(), ambition_extension_sdk::schema::SchemaError> {
        let (phase, left, dir) = match self {
            SnakeShell::Walking => (0, 0.0, 0.0),
            SnakeShell::Retreating(t) => (1, t, 0.0),
            SnakeShell::Boxed(t) => (2, t, 0.0),
            SnakeShell::Sliding { dir, grace } => (3, grace, dir),
            SnakeShell::Peeking(t) => (4, t, 0.0),
            SnakeShell::Emerging(t) => (5, t, 0.0),
        };
        record.set(PHASE, Value::U32(phase))?;
        record.set(LEFT, Value::F32(left))?;
        record.set(DIR, Value::F32(dir))
    }
}

/// The row of the snake's sheet a shelled stage pins.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellPose {
    Retreat,
    ShellIdle,
    Peek,
    Emerge,
}

impl ShellPose {
    /// The row's name, as the pose port takes it.
    pub fn row(self) -> &'static str {
        match self {
            ShellPose::Retreat => "retreat",
            ShellPose::ShellIdle => "boxed_idle",
            ShellPose::Peek => "peek",
            ShellPose::Emerge => "emerge",
        }
    }
}

/// What the pure step wants reflected for one snake this tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShellEffects {
    /// The next phase.
    pub phase: SnakeShell,
    /// Pose to pin, or `None` to let the shared picker choose (Walking).
    pub anim: Option<ShellPose>,
    /// Whether the snake is a live, threatening WALKER this tick (`true`) or an
    /// inert SHELL (`false`): held in place and harmless to the touch. The
    /// snake never dies of this; it toggles only its hold and its contact
    /// threat, never its health.
    pub alive: bool,
    /// Horizontal velocity to command, or `None` to leave the body's own
    /// physics alone (Walking). Shell phases hold still (`Some(0.0)`) or slide.
    pub vel_x: Option<f32>,
    /// A fresh withdraw just started (bounce the stomper, the dust).
    pub just_squashed: bool,
    /// A resting shell was just kicked into a slide (the kicker's thud).
    pub just_kicked: bool,
}

/// The discrete things that can happen TO a snake this tick.
///
/// `stomped` and the kick are derived from the ONE [`PlayerTouch`]
/// classification, so a contact is a stomp or a side, never both and never
/// neither-when-touching.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ShellInputs {
    /// A player is on this body's head this tick ([`PlayerTouch::Top`]).
    pub stomped: bool,
    /// A player is touching a RESTING shell, from any side; `Some(dir)` is the
    /// way to launch it: `-1.0` left, `1.0` right.
    ///
    /// Set for a stomp as well as a side bump, because in the game this pays
    /// homage to both kick it. The direction is away from whichever side of the
    /// shell the player is more on, and a body exactly at its centre kicks it
    /// RIGHT: some answer has to be picked, and picking by position keeps it a
    /// pure function of the two poses rather than of arrival order.
    pub kick_dir: Option<f32>,
    /// A sliding shell's velocity collapsed against a wall this tick.
    pub blocked: bool,
}

/// The whole shell choreography, as a pure function. Given the current phase,
/// the tick length, what happened to the body and her pack's shell rules,
/// return the next phase and what the game should reflect.
pub fn step_snake_shell(phase: SnakeShell, dt: f32, inputs: ShellInputs, shell: &ShellRules) -> ShellEffects {
    use SnakeShell::*;
    // Defaults for the shelled stages: pinned pose, held still, inert.
    let shelled = |phase, anim| ShellEffects {
        phase,
        anim: Some(anim),
        alive: false,
        vel_x: Some(0.0),
        just_squashed: false,
        just_kicked: false,
    };
    // A stomp pre-empts every phase. It is the one input that always means "be
    // in the shell", and it always bounces the stomper. That is what makes
    // standing over ANY snake safe: whatever the body under the player's feet
    // was doing (walking, running as a kicked shell, peeking back out), this
    // tick it is inert, and the bounce carries them off it.
    if inputs.stomped {
        let reseat = ShellEffects { just_squashed: true, ..shelled(Boxed(shell.boxed_seconds), ShellPose::ShellIdle) };
        // Landing on a RESTING shell kicks it, exactly as a side bump does.
        //
        // Only a RESTING shell. A shell already running is stopped by a stomp
        // (that is the tech), and a walker, a mid-withdraw snake or one
        // climbing back out has no shell to kick yet.
        if let (Boxed(_), Some(dir)) = (phase, inputs.kick_dir) {
            return ShellEffects {
                just_squashed: true,
                just_kicked: true,
                vel_x: Some(dir * shell.slide_speed),
                ..shelled(Sliding { dir, grace: shell.kick_grace_seconds }, ShellPose::ShellIdle)
            };
        }
        return match phase {
            // A live walker starts the withdraw; the retreat row plays first.
            Walking => ShellEffects { just_squashed: true, ..shelled(Retreating(shell.retreat_seconds), ShellPose::Retreat) },
            // Mid-withdraw: let the retreat run out on its own clock.
            Retreating(t) => {
                let t = t - dt;
                if t <= 0.0 {
                    reseat
                } else {
                    ShellEffects { just_squashed: true, ..shelled(Retreating(t), ShellPose::Retreat) }
                }
            }
            // Resting, running, peeking or climbing out: re-seat the shell.
            // This is the "stop the runaway" tech, and it also shoves a snake
            // that was about to emerge under the player's feet back into its
            // shell.
            Boxed(_) | Sliding { .. } | Peeking(_) | Emerging(_) => reseat,
        };
    }
    let walker = ShellEffects { phase: Walking, anim: None, alive: true, vel_x: None, just_squashed: false, just_kicked: false };
    match phase {
        // The ordinary walker: the shared picker chooses walk or idle, and the
        // brain drives movement.
        Walking => walker,
        Retreating(t) => {
            let t = t - dt;
            if t <= 0.0 {
                shelled(Boxed(shell.boxed_seconds), ShellPose::ShellIdle)
            } else {
                shelled(Retreating(t), ShellPose::Retreat)
            }
        }
        Boxed(t) => {
            if let Some(dir) = inputs.kick_dir {
                ShellEffects {
                    just_kicked: true,
                    vel_x: Some(dir * shell.slide_speed),
                    ..shelled(Sliding { dir, grace: shell.kick_grace_seconds }, ShellPose::ShellIdle)
                }
            } else {
                let t = t - dt;
                if t <= 0.0 {
                    shelled(Peeking(shell.peek_seconds), ShellPose::Peek)
                } else {
                    shelled(Boxed(t), ShellPose::ShellIdle)
                }
            }
        }
        // A SIDE contact never stops a running shell: that is the shell
        // running you (or an enemy) down, a real hit through the shared damage
        // pipeline while the shell keeps going. (The stop-it-dead branch is the
        // stomp pre-emption above.)
        Sliding { dir, grace } => {
            // A wall ARMS it. The grace exists for one reason: the shell is
            // still inside the body that just kicked it, and a shell you cannot
            // get off is not a mechanic. A shell that has hit a wall and turned
            // around is coming BACK at you, which is the whole danger of
            // kicking one down a corridor. So the bounce spends the grace
            // outright rather than letting it run out on a timer that might
            // still be ticking when the shell returns.
            let (dir, grace) = if inputs.blocked { (-dir, 0.0) } else { (dir, (grace - dt).max(0.0)) };
            ShellEffects { vel_x: Some(dir * shell.slide_speed), ..shelled(Sliding { dir, grace }, ShellPose::ShellIdle) }
        }
        Peeking(t) => {
            let t = t - dt;
            if t <= 0.0 {
                shelled(Emerging(shell.emerge_seconds), ShellPose::Emerge)
            } else {
                shelled(Peeking(t), ShellPose::Peek)
            }
        }
        Emerging(t) => {
            let t = t - dt;
            if t <= 0.0 {
                walker
            } else {
                shelled(Emerging(t), ShellPose::Emerge)
            }
        }
    }
}

/// The player of the contact the shell answers, and how it touches the
/// snake: a stomp before a side touch, then the nearer, then the first in
/// identity order. A toucher no seat drives is not a player.
pub fn the_player(contact: &BodyContact, band: f32) -> Option<(usize, PlayerTouch)> {
    let body = Bounds::of(contact.body.position, contact.body.size);
    let [bx, by] = contact.body.position;
    contact
        .touchers
        .iter()
        .enumerate()
        .filter(|(_, toucher)| toucher.driven)
        .filter_map(|(index, toucher)| {
            let touch =
                player_touch(body, Bounds::of(toucher.body.position, toucher.body.size), toucher.body.velocity, band)?;
            let [px, py] = toucher.body.position;
            let distance = (px - bx).powi(2) + (py - by).powi(2);
            Some((index, touch, distance))
        })
        .min_by(|a, b| {
            let rank = |touch| match touch {
                PlayerTouch::Top => 0,
                PlayerTouch::Side => 1,
            };
            (rank(a.1), a.2, a.0).partial_cmp(&(rank(b.1), b.2, b.0)).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(index, touch, _)| (index, touch))
}

/// What happened to the snake this tick, from the one classified touch.
pub fn shell_inputs(phase: SnakeShell, contact: &BodyContact, player: Option<(usize, PlayerTouch)>, slide_speed: f32) -> ShellInputs {
    // ANY touch on a RESTING shell kicks it away from the player: a stomp from
    // above as much as a bump from the side. Ties (dead centre) go right.
    let kick_dir = match (phase, player) {
        (SnakeShell::Boxed(_), Some((index, _))) => {
            Some(if contact.touchers[index].body.position[0] <= contact.body.position[0] { 1.0 } else { -1.0 })
        }
        _ => None,
    };
    ShellInputs {
        stomped: matches!(player, Some((_, PlayerTouch::Top))),
        kick_dir,
        // A sliding shell whose speed collapsed hit something.
        blocked: matches!(phase, SnakeShell::Sliding { .. }) && contact.body.velocity[0].abs() < slide_speed * 0.25,
    }
}

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "shell"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            build: None,
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![shell_schema()],
        entries: vec![EntryDescriptor {
            replies: Vec::new(),
            key: SNAKE_BRAIN_KEY.into(),
            phase: BODY_CONTACT,
            trigger: TriggerBinding { port: BodyContactPort::KEY, selector: SNAKE_BRAIN_KEY.into() },
            reads: vec![StompRulesPort::KEY, ShellRulesPort::KEY],
            writes: vec![SHELL],
            requests: vec![
                ContactBouncePort::KEY,
                ContactCuePort::KEY,
                BurstPort::KEY,
                ContactThreatPort::KEY,
                ControlHoldPort::KEY,
                PosePinPort::KEY,
                SetVelocityPort::KEY,
                KineticHazardPort::KEY,
            ],
            after: Vec::new(),
            // The threat, the pose, the velocity, the hazard, the hold, the
            // bounce, the dust and two cues.
            limits: Limits { max_requests: 9 },
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(shell),
        }],
    }
}

fn shell(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let contact: BodyContact = inv.trigger::<BodyContactPort>()?.clone();
    let stomp = inv.observe::<StompRulesPort>()?.clone();
    let rules = *inv.observe::<ShellRulesPort>()?;
    let dt = inv.dt();
    let schema_fault = |error| Fault::Schema { schema: SHELL, error };
    let record = inv.state(&SHELL)?;
    let phase = SnakeShell::of_record(record);
    let credit = record.get(CREDIT).ok().and_then(Value::as_opt).flatten().and_then(Value::as_ref_id).map(str::to_owned);

    let player = the_player(&contact, stomp.band);
    let fx = step_snake_shell(phase, dt, shell_inputs(phase, &contact, player, rules.slide_speed), &rules);
    let credit = if fx.just_kicked {
        player.map(|(index, _)| contact.touchers[index].id.clone())
    } else if matches!(fx.phase, SnakeShell::Sliding { .. }) {
        credit
    } else {
        None
    };
    let record = inv.state(&SHELL)?;
    fx.phase.write(record).map_err(schema_fault)?;
    record
        .set(CREDIT, Value::Opt(credit.clone().map(|id| Box::new(Value::Ref(id)))))
        .map_err(schema_fault)?;

    // The stomp bounces the stomper (a fresh stomp on a walker, or one from
    // above onto a running shell: both stop the threat), with its pogo cue,
    // and kicks up the dust every stomp of hers does.
    if let (true, Some((index, _))) = (fx.just_squashed, player) {
        inv.submit::<ContactBouncePort>(ContactBounce { toucher: index as u32, speed: stomp.bounce_speed, pogo_cue: true })?;
        let dust = &stomp.dust;
        inv.submit::<BurstPort>(Burst {
            at: contact.body.position,
            count: dust.count,
            speed: dust.speed,
            color: dust.color,
            kind: dust.kind.clone(),
        })?;
    }
    // A kick is the kicker's verb, so the thud is the kicker's.
    if let (true, Some((index, _))) = (fx.just_kicked, player) {
        inv.submit::<ContactCuePort>(ContactCue { toucher: index as u32 })?;
    }
    // The threat is a read of the phase, so it is written each tick; the
    // authored tuning is never touched.
    inv.submit::<ContactThreatPort>(ContactThreat { withdrawn: !fx.alive })?;
    // Held in place while shelled: the shell cycle is a scripted beat, so it
    // holds control from the withdraw to the emerge, as a transition.
    let was_walking = matches!(phase, SnakeShell::Walking);
    if was_walking != fx.alive {
        inv.submit::<ControlHoldPort>(ControlHold { held: !fx.alive })?;
    }
    if let Some(vx) = fx.vel_x {
        inv.submit::<SetVelocityPort>(SetVelocity { velocity: [vx, contact.body.velocity[1]] })?;
    }
    inv.submit::<PosePinPort>(PosePin { row: fx.anim.map(|pose| pose.row().to_owned()) })?;
    // A running shell runs down every body it covers, credited to its kicker
    // and sparing the kicker until the grace is spent.
    if let SnakeShell::Sliding { grace, .. } = fx.phase {
        let spare = if grace > 0.0 { credit.iter().cloned().collect() } else { Vec::new() };
        inv.submit::<KineticHazardPort>(KineticHazard { damage: rules.damage, knockback: 1.0, credit, spare })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
