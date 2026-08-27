//! Movement-event presentation facts: translate a frame's engine
//! [`ae::FrameEvents`] (jump/dash/blink/ledge/etc. ops + blink endpoints) into
//! `SfxMessage`/`VfxMessage` *facts*, and arm the short presentation timers the
//! ops imply (wall-jump pose, blink-camera lerp, hit flash).
//!
//! Pure sim + message emission — `VfxMessage` is `ambition_vfx`, not
//! `ambition_render`, so this carries no render dependency. It is called from the
//! host's player-tick control/sim phases; it lived in `ambition_app` only because
//! it was authored beside that glue.

use bevy::prelude::MessageWriter;

use ambition_platformer2d_core as ae;
use ambition_vfx::vfx::{ParticleKind, VfxMessage};

// ⛔ NAMED FROM `ambition_characters`, not through `crate::actor`, which merely
// re-exports it. This file imported `BodyCombat` from the owning crate and
// `BodyAnimFacts` from the monolith's forward of the SAME module — one file
// spelling one crate two ways, which is the tell a facade leaves and the only
// reason a census counted this module as monolith-coupled.
use ambition_characters::actor::{BodyAnimFacts, BodyCombat};
use ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState;
use ambition_sfx::{SfxMessage, SfxWriter};

/// How long the blink-in camera ease runs, in seconds.
///
/// ⭐ IT LIVES BESIDE ITS ONE CONSUMER. This was `crate::BLINK_IN_ANIM_TIME` at
/// the monolith ROOT, and the whole tree named it exactly once — here. A constant
/// parked at a 95k-line crate's address reads as shared vocabulary and is a
/// coupling row on every census of this module; it was neither.
const BLINK_IN_ANIM_TIME: f32 = 0.34;

/// How long the wall-jump push-off pose holds after the WallJump op fires. Short
/// enough to clear before the apex of the jump arc so the regular `Jump` row
/// picks back up; long enough that the kick reads at typical playback rates.
const WALL_JUMP_ANIM_HOLD_SECS: f32 = 0.18;

/// Advance a body's presentation overlay timers one frame. Semantic ground
/// transitions arm/clear the landing overlay through
/// [`arm_ground_contact_anim_overlay`]; this function only decays active poses
/// and detects the dash rising edge.
pub fn advance_body_anim_overlays(dashing: bool, anim: &mut BodyAnimFacts, frame_dt: f32) {
    /// Brief pre-roll for the dash startup pose (below the dash's own duration so
    /// the streaking dash row still gets airtime).
    const DASH_STARTUP_SECS: f32 = 0.05;

    // Op-armed poses just decay here (armed by attack / projectile / movement ops).
    anim.slash_anim_timer = (anim.slash_anim_timer - frame_dt).max(0.0);
    anim.shoot_anim_timer = (anim.shoot_anim_timer - frame_dt).max(0.0);
    anim.wall_jump_anim_timer = (anim.wall_jump_anim_timer - frame_dt).max(0.0);
    anim.interact_anim_timer = (anim.interact_anim_timer - frame_dt).max(0.0);
    anim.death_anim_timer = (anim.death_anim_timer - frame_dt).max(0.0);

    anim.land_anim_timer = (anim.land_anim_timer - frame_dt).max(0.0);

    // Dash rising edge: no dash last frame, a dash this frame.
    if dashing && !anim.anim_prev_dashing {
        anim.dash_startup_timer = DASH_STARTUP_SECS;
    } else {
        anim.dash_startup_timer = (anim.dash_startup_timer - frame_dt).max(0.0);
    }

    anim.anim_prev_dashing = dashing;
}

/// The impact speed at which the engine already calls a landing HARD.
///
/// Shared by the landing pose and the splat below so the two agree about what
/// "hard" means. A splat that started somewhere else would be a second opinion
/// on one question.
///
/// RE-MEASURED 2026-08-23 after the perception fix (D190) changed what a CPU
/// match does. Smash CPU-vs-CPU, `smash_george_booul` vs itself (⚠ one matchup
/// — the demo shell carries three fighters and the full app sixteen, so this is
/// the best available sample and not a general one), 90s × 5 runs, every
/// `GroundContactTransition::Landed` sampled: n = 315,
/// `p25 81  p50 218  p75 604  p90 1524  max 1669`. 520 now sits at ≈ p72.
///
/// ⛔ AND IT IS DELIBERATELY NOT RE-FITTED TO THAT SAMPLE. This is an ENGINE
/// constant: `arm_ground_contact_anim_overlay` picks the hard landing POSE off
/// it for every body in every game, so re-fitting it to one Smash matchup would
/// retune Mary-O's landings from a fight Mary-O is not in. The splat's own full
/// read below is presentation-only and is re-fitted; this line stays until
/// somebody measures landings across the games that share it.
pub const HARD_LAND_SPEED: f32 = 520.0;

/// Where a floor splat reaches its full read.
///
/// ⭐ THE GAP, not a percentile — the landing population has two clusters and
/// the full read belongs at the boundary between them. The 100-px histogram of
/// the 315 landings above:
///
/// ```text
///   0:93  100:57  200:26  300:19  400:21  500:17  600:4  700:15  800:1
///   1000:17  1100:2   [1200-1499: NOTHING]   1500:40  1600:3
/// ```
///
/// A 345-px gap between 1155 and 1500, and the cluster above it holds only six
/// distinct values (1500, 1524, 1548, 1572, 1597, 1669) spaced by exactly one
/// tick of gravity — a body that fell a fixed distance, not a body that landed
/// hard. So the splat saturates just below that cluster: every long fall lands
/// at the full read, and everything an ordinary exchange produces ramps below
/// it.
///
/// It was `HARD_LAND_SPEED * 2.0` = 1040, which sat inside the sparse tail
/// between the two clusters — a value with no population on either side of it.
const SPLAT_FULL_SPEED: f32 = 1330.0;

/// The band one surface's arrivals live in: where a splat begins to read, and
/// where it stops getting harder.
///
/// ⭐ A FLOOR AND A WALL NEED DIFFERENT BANDS, and that is not tuning taste.
/// Gravity accelerates a body into a floor and never into a wall, so a body
/// only ever reaches a wall as fast as something threw it. The two populations
/// do not overlap: over the same five matches the hardest side contact was
/// `440` px/s while the MEDIAN landing was `299` and the hardest `1669`. A wall
/// splat gated on [`HARD_LAND_SPEED`] could not fire at all — which is the same
/// shape of bug as a launch trail whose onset sat above every launch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplatBand {
    pub onset: f32,
    pub full: f32,
}

impl SplatBand {
    /// The floor. See [`HARD_LAND_SPEED`] for the sample behind both numbers.
    pub const FLOOR: Self = Self {
        onset: HARD_LAND_SPEED,
        full: SPLAT_FULL_SPEED,
    };

    /// A wall, fitted to WALL arrivals.
    ///
    /// Same five matches: 63 `ContactKind::Side` contacts in 7.5 minutes, and
    /// the population is BIMODAL rather than a curve — 54 of them read exactly
    /// `52` px/s (a body leaning on the platform's lip), and the other nine are
    /// `158`, `270` ×4 and `440` ×4. So `onset` is the gap between the two
    /// clusters rather than a percentile of one distribution, and `full` is the
    /// hardest side contact ever sampled.
    ///
    /// ⚠ NINE real wall arrivals in seven and a half minutes. The effect is
    /// rare because THE STAGE has no wall game, not because the gate is tight;
    /// a stage with walls in play would want this re-measured, not re-tuned.
    ///
    ///  RE-MEASURED 2026-08-23 after D190 on a bigger sample (n = 85) and
    /// UNCHANGED: 74 contacts at exactly `52`, then 7 at `270` and 4 at `440`.
    /// The same three values, the same gap, a fight that plays completely
    /// differently. These are geometry — the platform's lip and the speeds a
    /// body can approach it at — rather than anything the fight decides, which
    /// is why the band did not move when everything else did.
    pub const WALL: Self = Self {
        onset: 150.0,
        full: 440.0,
    };
}

/// What one arrival at a surface asks for, or `None` when nobody needs to see
/// it. Pure, so the whole rule is asserted without a renderer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImpactSplat {
    pub particles: u32,
    pub speed: f32,
    pub alpha: f32,
    /// How hard, `0..=1`, across this surface's own [`SplatBand`].
    pub force: f32,
    /// The body did not CHOOSE this arrival — it was still falling out of a
    /// launch when the surface got there. A crash, not a landing.
    ///
    /// It is the one thing that makes a splat unconditional: a body slammed
    /// down helpless reads as slammed at any speed, and 26 of the 340 sampled
    /// landings were crashes with a median of `712` px/s against `254` for the
    /// rest. Speed says how hard; this says whether it was done TO the body.
    pub crash: bool,
}

/// How hard a body hit this surface, `0..=1` — `0.0` at the band's onset and
/// `1.0` at its full read.
pub fn splat_force(impact_speed: f32, band: SplatBand) -> f32 {
    let span = band.full - band.onset;
    if span <= 0.0 {
        return 0.0;
    }
    ((impact_speed - band.onset) / span).clamp(0.0, 1.0)
}

/// How hard a body hit the FLOOR, `0..=1`. The landing pose's own reading.
pub fn landing_force(impact_speed: f32) -> f32 {
    splat_force(impact_speed, SplatBand::FLOOR)
}

/// The splat this arrival asks for.
///
/// An ordinary arrival under the band's onset asks for nothing and keeps the
/// puff it always had. A CRASH always asks for something, however slow: the
/// speed shapes the dust and `involuntary` is what earns the effect at all.
pub fn impact_splat(impact_speed: f32, involuntary: bool, band: SplatBand) -> Option<ImpactSplat> {
    let force = splat_force(impact_speed, band);
    if force <= 0.0 && !involuntary {
        return None;
    }
    Some(ImpactSplat {
        // A crash spends its whole budget on being seen; an ordinary hard
        // arrival earns its particles from speed alone.
        particles: 6 + (14.0 * force) as u32 + if involuntary { 8 } else { 0 },
        speed: 180.0 + 280.0 * force,
        alpha: 0.72 + 0.18 * force,
        force,
        crash: involuntary,
    })
}

/// Apply one semantic ground-contact transition to the landing animation
/// overlay. Initialization never arms a landing pose; only a known airborne
/// baseline becoming grounded does.
pub fn arm_ground_contact_anim_overlay(
    anim: &mut BodyAnimFacts,
    transition: ae::GroundContactTransition,
) {
    const LAND_HARD_HOLD_SECS: f32 = 0.34;
    const LAND_SOFT_HOLD_SECS: f32 = 0.16;

    match transition {
        ae::GroundContactTransition::Landed { impact_speed, .. } => {
            let hard = impact_speed >= HARD_LAND_SPEED;
            anim.land_anim_hard = hard;
            anim.land_anim_timer = if hard {
                LAND_HARD_HOLD_SECS
            } else {
                LAND_SOFT_HOLD_SECS
            };
        }
        ae::GroundContactTransition::LeftGround
        | ae::GroundContactTransition::InitializedGrounded
        | ae::GroundContactTransition::InitializedAirborne => {
            anim.land_anim_timer = 0.0;
        }
        ae::GroundContactTransition::Unchanged => {}
    }
}

/// Arm the op-driven presentation overlays a movement frame implies on ANY body's
/// [`BodyAnimFacts`]: the wall-jump push-off pose fires on the
/// `WallJump` op. Body-generic so the player tick AND the actor tick arm the SAME
/// pose from the SAME frame data — an AI fighter that wall-jumps shows the kick pose
/// the player does, not just its dust/SFX (fable review §A9 follow-up). The other
/// op-armed overlays (slash / shoot) are armed at their own effect sites — attack
/// (`combat::attack`) and projectile fire (`brain_effects` / `projectile::systems`)
/// — because those aren't movement ops; this covers the movement ops only, and
/// [`advance_body_anim_overlays`] decays every op-armed timer afterward.
pub fn arm_movement_anim_overlays(anim: &mut BodyAnimFacts, events: &ae::FrameEvents) {
    for op in &events.operations {
        if matches!(op, ae::MovementOp::WallJump) {
            anim.wall_jump_anim_timer = WALL_JUMP_ANIM_HOLD_SECS;
        }
    }
}

/// Body-generic movement presentation: translate a frame's [`ae::FrameEvents`]
/// (jump/dash/dodge/wall-jump/pogo/swim/ledge/shield/fly ops + blink endpoints)
/// into `SfxMessage`/`VfxMessage` facts at the body's position, plus the
/// grounded-transition landing dust.
///
/// Carries NO body-specific state — the wall-jump anim pose, the blink-camera lerp, and the
/// action hit-flash stay with each caller ([`handle_player_events`] arms them for the player;
/// the actor tick does not).
#[allow(clippy::too_many_arguments)]
pub fn emit_movement_fx(
    sfx: &mut SfxWriter,
    vfx: &mut MessageWriter<VfxMessage>,
    events: &ae::FrameEvents,
    pos: ae::Vec2,
    facing: f32,
    size: ae::Vec2,
    // The body's presentation source, so a jump sounds like the character that
    // jumped. `None` = the session provider, which is the honest answer for a body
    // wearing no character.
    source: Option<&ambition_sfx::PresentationSourceId>,
) {
    for op in &events.operations {
        match op {
            ae::MovementOp::Jump | ae::MovementOp::WallJump => {
                sfx.write_for_body(source, SfxMessage::Jump { pos });
                vfx.write(VfxMessage::Dust { pos, facing });
            }
            ae::MovementOp::DoubleJump => {
                sfx.write_for_body(source, SfxMessage::DoubleJump { pos });
                vfx.write(VfxMessage::Burst {
                    pos,
                    count: 14,
                    speed: 210.0,
                    color: [0.70, 1.0, 0.86, 0.82],
                    kind: ParticleKind::Dust,
                });
            }
            ae::MovementOp::Dash | ae::MovementOp::DoubleDash => {
                sfx.write_for_body(source, SfxMessage::Dash { pos });
                vfx.write(VfxMessage::Burst {
                    pos,
                    count: 10,
                    speed: 330.0,
                    color: [1.0, 0.86, 0.38, 0.90],
                    kind: ParticleKind::Spark,
                });
            }
            ae::MovementOp::DodgeRoll => {
                sfx.write_for_body(source, SfxMessage::Dash { pos });
                vfx.write(VfxMessage::Burst {
                    pos,
                    count: 8,
                    speed: 240.0,
                    color: [0.60, 1.0, 0.70, 0.80],
                    kind: ParticleKind::Dust,
                });
            }
            // the spot dodge is quieter and lower. It covers no ground, so
            // the roll's wide kick-up would read as travel that did not happen;
            // a small puff at the feet is the tell that the body stood still and
            // meant to.
            ae::MovementOp::SpotDodge => {
                sfx.write_for_body(source, SfxMessage::Dash { pos });
                vfx.write(VfxMessage::Burst {
                    pos,
                    count: 4,
                    speed: 90.0,
                    color: [0.60, 1.0, 0.70, 0.65],
                    kind: ParticleKind::Dust,
                });
            }
            // The aerial evade reads COOLER and thinner than the roll's dust —
            // no ground to kick up, and the colour is the tell a player uses to
            // recognize the maneuver mid-air.
            // The floor game's beats. A knockdown thumps and kicks up dust;
            // a tech is the crisp recovery that refused it; a getup roll reads
            // like the ground roll it is. `Tumble` and `Getup` are deliberately
            // silent — the first is a state the launch already announced with
            // its own hit feedback, and the second is what a body does when it
            // ran out of options.
            // A knockdown thumps and kicks up dust. It CANNOT scale with the
            // impact that caused it: this op is pushed by the control phase,
            // which runs before integration and therefore only sees
            // `on_ground` on the tick AFTER touchdown — one bundle later than
            // the `Landed { impact_speed }` that measured the fall. The
            // impact-scaled beat is on the landing itself, below.
            ae::MovementOp::Knockdown => {
                vfx.write(VfxMessage::Burst {
                    pos,
                    count: 12,
                    speed: 200.0,
                    color: [0.72, 0.66, 0.56, 0.85],
                    kind: ParticleKind::Dust,
                });
            }
            // A TECH is not a getup roll and must not sound like one. It is the
            // defender REFUSING the knockdown, decided in a handful of frames,
            // and it is the one floor-game beat worth reading from across the
            // stage. It gets a bright, fast spark ring at the contact point and
            // its own crisp cue; the dust is a small response underneath rather
            // than the whole effect.
            ae::MovementOp::Tech => {
                sfx.write_for_body(
                    source,
                    SfxMessage::Play {
                        id: ambition_sfx::ids::PLAYER_TECH,
                        pos,
                    },
                );
                vfx.write(VfxMessage::Burst {
                    pos,
                    count: 10,
                    speed: 430.0,
                    color: [0.92, 0.98, 1.0, 0.95],
                    // Sparks SHRINK as they age, so the ring snaps shut instead
                    // of blooming — which is what makes it read as an impact
                    // refused rather than as more dust.
                    kind: ParticleKind::Spark,
                });
                vfx.write(VfxMessage::Burst {
                    pos,
                    count: 4,
                    speed: 120.0,
                    color: [0.80, 0.92, 1.0, 0.55],
                    kind: ParticleKind::Dust,
                });
            }
            // The getup roll keeps the travelling-dust read it always had: it
            // IS a roll along the floor, and it shares that shape with the
            // dodge roll on purpose.
            ae::MovementOp::GetupRoll => {
                sfx.write_for_body(source, SfxMessage::Dash { pos });
                vfx.write(VfxMessage::Burst {
                    pos,
                    count: 8,
                    speed: 260.0,
                    color: [0.80, 0.92, 1.0, 0.80],
                    kind: ParticleKind::Dust,
                });
            }
            ae::MovementOp::Tumble | ae::MovementOp::Getup | ae::MovementOp::GetupAttack => {}
            ae::MovementOp::AirDodge => {
                sfx.write_for_body(source, SfxMessage::Dash { pos });
                vfx.write(VfxMessage::Burst {
                    pos,
                    count: 6,
                    speed: 190.0,
                    color: [0.62, 0.86, 1.0, 0.75],
                    kind: ParticleKind::Spark,
                });
            }
            ae::MovementOp::Blink | ae::MovementOp::PrecisionBlink => {
                // Blink visuals use the explicit `events.blinks` endpoint data below.
            }
            ae::MovementOp::FlyToggle => {
                vfx.write(VfxMessage::Burst {
                    pos,
                    count: 12,
                    speed: 180.0,
                    color: [0.45, 0.82, 1.0, 0.72],
                    kind: ParticleKind::Dust,
                });
            }
            ae::MovementOp::Pogo | ae::MovementOp::Rebound => {
                sfx.write_for_body(source, SfxMessage::Pogo { pos });
            }
            ae::MovementOp::SwimStroke => {
                sfx.write_for_body(source, SfxMessage::Jump { pos });
                vfx.write(VfxMessage::Burst {
                    pos,
                    count: 8,
                    speed: 150.0,
                    color: [0.50, 0.85, 1.0, 0.70],
                    kind: ParticleKind::Dust,
                });
            }
            ae::MovementOp::LedgeGrab => {
                vfx.write(VfxMessage::Dust { pos, facing });
            }
            ae::MovementOp::LedgeJump => {
                sfx.write_for_body(source, SfxMessage::Jump { pos });
                vfx.write(VfxMessage::Burst {
                    pos,
                    count: 8,
                    speed: 180.0,
                    color: [0.70, 1.0, 0.86, 0.82],
                    kind: ParticleKind::Dust,
                });
            }
            ae::MovementOp::LedgeRoll => {
                // Reuse the dash sfx — the ledge roll IS a dodge-roll
                // semantically (invuln rolling motion). Adds a small
                // dust burst at the platform lip for visual feedback.
                sfx.write_for_body(source, SfxMessage::Dash { pos });
                vfx.write(VfxMessage::Dust { pos, facing });
            }
            ae::MovementOp::LedgeGetupAttack => {
                // The engine pairs this op with MovementOp::Slash on
                // the same frame, so the slash SFX/VFX (and the
                // attack hitbox) fire through the normal slash path.
                // Here we only add the lift-up dust so the swing
                // reads as "coming off the ledge," not "in mid-air."
                // TODO: when a dedicated getup-attack sprite lands,
                // route a distinct VFX/SFX here too.
                vfx.write(VfxMessage::Dust { pos, facing });
            }
            ae::MovementOp::ShieldUp => {
                // Reuse the quick blink tone as a placeholder until a
                // dedicated Shield SoundCue is added to the sfxbank.
                sfx.write_for_body(
                    source,
                    SfxMessage::Blink {
                        pos,
                        precision: false,
                    },
                );
                vfx.write(VfxMessage::Burst {
                    pos,
                    count: 12,
                    speed: 120.0,
                    color: [0.50, 0.80, 1.0, 0.70],
                    kind: ParticleKind::Dust,
                });
            }
            ae::MovementOp::ShieldBreak => {
                // The loudest thing a guard can do: a shatter burst at the body
                // and the impact tone, so a break reads without a meter.
                sfx.write_for_body(source, SfxMessage::Hit { pos });
                vfx.write(VfxMessage::Burst {
                    pos,
                    count: 28,
                    speed: 260.0,
                    color: [0.85, 0.95, 1.0, 0.95],
                    kind: ParticleKind::Shard,
                });
            }
            ae::MovementOp::Footstool => {
                // The bounce reads like a landing that went the other way: a
                // dust puff where the feet were and the jump tone.
                sfx.write_for_body(source, SfxMessage::DoubleJump { pos });
                vfx.write(VfxMessage::Dust { pos, facing });
            }
            ae::MovementOp::LedgeClimbStart
            | ae::MovementOp::LedgeClimbFinish
            | ae::MovementOp::LedgeDrop
            | ae::MovementOp::WallCling
            | ae::MovementOp::WallClimb
            // the crawl edges are published for the CAUSAL LOG, not for
            // presentation. What a body seating on a ceiling should sound like is
            // a game-feel choice, and inventing one here to satisfy the match
            // would ship an effect nobody asked for. Silent on purpose; this arm
            // already exists for exactly that.
            | ae::MovementOp::CrawlAttach
            | ae::MovementOp::CrawlDetach
            | ae::MovementOp::Slash => {}
            ae::MovementOp::Reset => {
                sfx.write_for_body(source, SfxMessage::Reset { pos });
            }
        }
    }
    for blink in &events.blinks {
        sfx.write_for_body(
            source,
            SfxMessage::Blink {
                pos: blink.from,
                precision: blink.precision,
            },
        );
        vfx.write(VfxMessage::BlinkEffects {
            from: blink.from,
            to: blink.to,
            precision: blink.precision,
        });
    }
    // THE TOUCHDOWN, scaled by the impact the landing itself measured.
    //
    // Every landing used to throw the identical puff, so a body that dropped
    // off a ledge and one that arrived at speed left the same mark. The impact
    // speed rides the transition, in this bundle, on this tick — which is what
    // makes this the one place a floor impact can be read at all. See
    // `MovementOp::Knockdown` above for why the knockdown arm cannot do it.
    if let ae::GroundContactTransition::Landed {
        impact_speed,
        involuntary,
    } = events.ground_contact
    {
        let feet = pos + ae::Vec2::new(0.0, size.y * 0.5);
        // Touchdown footfall. Emitted for every body; provider authority gates
        // it, so a game hears it only by authoring `player.land`.
        sfx.write_for_body(source, SfxMessage::Land { pos: feet });
        vfx.write(VfxMessage::Dust { pos: feet, facing });
        if let Some(splat) = impact_splat(impact_speed, involuntary, SplatBand::FLOOR) {
            write_splat(vfx, feet, splat);
        }
    }
    // THE OTHER SURFACES. A body thrown into a wall used to arrive in silence:
    // the step zeroes velocity along the contact axis as it resolves, so by the
    // time anything downstream saw the body its approach was gone, and there is
    // no arrangement of the remaining facts that reconstructs it.
    // `Contact::impact_speed` is captured at the resolution site for exactly
    // that reason, and `ContactKind` is the sim's own frame-relative answer to
    // which surface this was — ⛔ presentation must not re-classify a normal
    // against gravity to find the wall, least of all against screen -Y.
    for contact in &events.contacts {
        if contact.kind != ae::collision_semantics::ContactKind::Side {
            continue;
        }
        // ⛔ no `involuntary` here, and it is not an oversight: nothing
        // publishes whether a body CHOSE to arrive at a wall. `Landed` carries
        // that flag and `Contact` does not, so a wall arrival is judged on
        // speed alone until the sim says otherwise.
        let Some(splat) = impact_splat(contact.impact_speed, false, SplatBand::WALL) else {
            continue;
        };
        // Off the surface along its own outward normal, so the dust sits in
        // front of the wall rather than inside it. The normal points away from
        // the surface toward the body, which is the direction that works under
        // any gravity and for either facing.
        write_splat(vfx, contact.point + contact.normal * (size.x * 0.25), splat);
    }
}

/// Draw one arrival's splat: dust from the contact, plus the ring a CRASH gets
/// and an ordinary hard arrival does not.
fn write_splat(vfx: &mut MessageWriter<VfxMessage>, pos: ae::Vec2, splat: ImpactSplat) {
    vfx.write(VfxMessage::Burst {
        pos,
        count: splat.particles,
        speed: splat.speed,
        // A crash kicks up brighter, paler dust than an arrival under its own
        // power — the one thing that separates being slammed into a surface
        // from choosing to land hard on it.
        color: if splat.crash {
            [0.93, 0.90, 0.84, splat.alpha]
        } else {
            [0.72, 0.66, 0.56, splat.alpha]
        },
        kind: ParticleKind::Dust,
    });
    if splat.crash {
        vfx.write(VfxMessage::Impact { pos });
    }
}

#[allow(clippy::too_many_arguments)]
pub fn handle_player_events(
    sfx: &mut SfxWriter,
    vfx: &mut MessageWriter<VfxMessage>,
    clusters: &ae::BodyClustersMut<'_>,
    combat: &mut BodyCombat,
    blink_cam: &mut PlayerBlinkCameraState,
    anim: &mut BodyAnimFacts,
    events: ae::FrameEvents,
    // A13: the player is a character too. Its jump should sound like ITS provider,
    // not like whoever owns the session.
    source: Option<&ambition_sfx::PresentationSourceId>,
) {
    let pos = clusters.kinematics.pos;
    let facing = clusters.kinematics.facing;
    let size = clusters.kinematics.size;
    // Body-generic SFX/VFX — the SAME emitter the actor tick uses.
    emit_movement_fx(sfx, vfx, &events, pos, facing, size, source);
    arm_ground_contact_anim_overlay(anim, events.ground_contact);
    // Body-generic op-driven overlay poses (the wall-jump push-off) — the SAME
    // arming the actor tick runs (§A9). Player-specific presentation the shared
    // arming deliberately omits stays inline below: the blink-camera lerp.
    arm_movement_anim_overlays(anim, &events);
    for blink in &events.blinks {
        blink_cam.blink_in_duration = BLINK_IN_ANIM_TIME;
        blink_cam.blink_in_timer = blink_cam.blink_in_duration;
        blink_cam.blink_camera_from = blink.from;
        blink_cam.blink_camera_to = blink.to;
    }
    // The white hit-flash is DAMAGE feedback — a hazard hit reads as being hurt.
    // Movement operations (jump, dash, blink, …) deliberately do NOT flash: an
    // action is not a hit, and flashing the sprite white on every jump reads as
    // taking damage. Real combat/hazard damage arms `hit_flash` through the damage
    // path (`ambition_damage`).
    if events.reset.is_some_and(ae::ResetCause::is_hazardous) {
        combat.hit_flash = 0.12;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    #[derive(Resource)]
    struct TestEvents(ae::FrameEvents);

    fn emit_system(
        mut sfx: SfxWriter,
        mut vfx: MessageWriter<VfxMessage>,
        events: Res<TestEvents>,
    ) {
        emit_movement_fx(
            &mut sfx,
            &mut vfx,
            &events.0,
            ae::Vec2::ZERO,
            1.0,
            ae::Vec2::new(20.0, 40.0),
            None,
        );
    }

    /// The body-generic emitter (shared by the player tick AND the actor tick)
    /// turns a frame's ops into movement SFX/VFX: a `Jump` op yields one `Jump`
    /// SFX + a `Dust` VFX, and the air→ground transition adds the landing dust AND
    /// the body-generic landing SFX. Pins that a future edit can't silently drop
    /// actor (or player) movement presentation the way the old blink-only actor
    /// branch did (§A8) — and that the landing cue fires for ANY body, not just the
    /// player (the emit site is body-generic; a provider gates who voices it).
    #[test]
    fn emit_movement_fx_emits_jump_sfx_and_dust_plus_landing() {
        let mut events = ae::FrameEvents::default();
        events.operations.push(ae::MovementOp::Jump);
        events.ground_contact = ae::GroundContactTransition::Landed {
            impact_speed: 640.0,
            involuntary: false,
        };
        let mut app = App::new();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.add_message::<VfxMessage>();
        app.insert_resource(TestEvents(events));
        app.add_systems(Update, emit_system);
        app.update();
        let sfx: Vec<SfxMessage> = app
            .world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ambition_sfx::OwnedSfxMessage>>()
            .drain()
            .map(|message| message.request)
            .collect();
        let vfx: Vec<VfxMessage> = app
            .world_mut()
            .resource_mut::<bevy::ecs::message::Messages<VfxMessage>>()
            .drain()
            .collect();
        assert_eq!(
            sfx.len(),
            2,
            "a Jump op plus the air→ground edge yield a Jump SFX and a Land SFX"
        );
        assert!(
            sfx.iter().any(|m| matches!(m, SfxMessage::Jump { .. })),
            "the jump cue"
        );
        assert!(
            sfx.iter().any(|m| matches!(m, SfxMessage::Land { .. })),
            "the body-generic landing cue (emitted for the player AND any actor)"
        );
        // The Jump dust, the air→ground landing dust, and — because 640 is
        // above the hard-landing line — the impact sheet that arrival now
        // throws. The fixture always landed this hard; only the sheet is new.
        assert_eq!(vfx.len(), 3, "jump dust + landing dust + the impact sheet");
        assert_eq!(
            vfx.iter()
                .filter(|m| matches!(m, VfxMessage::Dust { .. }))
                .count(),
            2,
            "the two puffs are unchanged"
        );
        assert!(
            vfx.iter().any(|m| matches!(
                m,
                VfxMessage::Burst {
                    kind: ParticleKind::Dust,
                    ..
                }
            )),
            "and the arrival adds a sheet: {vfx:?}"
        );
    }

    /// A TECH and a GETUP ROLL are different beats and must not sound or look
    /// alike. They shared one arm — the dash cue and one dust burst — so a
    /// spectator watching the floor game could not tell the defender who
    /// refused the knockdown from the one who ate it and rolled away.
    #[test]
    fn a_tech_and_a_getup_roll_are_told_apart() {
        let (tech_sfx, tech_vfx) = presentation_for(ae::MovementOp::Tech);
        let (roll_sfx, roll_vfx) = presentation_for(ae::MovementOp::GetupRoll);

        // The cues are different sounds, not the same sound twice.
        assert!(
            matches!(tech_sfx.as_slice(), [SfxMessage::Play { .. }]),
            "a tech has its own cue: {tech_sfx:?}"
        );
        assert!(
            matches!(roll_sfx.as_slice(), [SfxMessage::Dash { .. }]),
            "the getup roll keeps the dash cue it shares with the dodge roll: {roll_sfx:?}"
        );

        // And the tech's flash is a SPARK ring, which the roll never emits.
        assert!(
            tech_vfx.iter().any(|m| matches!(
                m,
                VfxMessage::Burst {
                    kind: ParticleKind::Spark,
                    ..
                }
            )),
            "the tech flashes: {tech_vfx:?}"
        );
        assert!(
            roll_vfx.iter().all(|m| !matches!(
                m,
                VfxMessage::Burst {
                    kind: ParticleKind::Spark,
                    ..
                }
            )),
            "the getup roll is dust, not a flash: {roll_vfx:?}"
        );
    }

    /// A landing's dust scales with the impact the transition measured, and a
    /// gentle step down keeps the puff it always had.
    #[test]
    fn a_landing_throws_dust_in_proportion_to_the_arrival() {
        assert_eq!(landing_force(0.0), 0.0, "stepping down is not an impact");
        assert_eq!(
            landing_force(HARD_LAND_SPEED),
            0.0,
            "the hard-landing line is the floor, and it is the line the landing POSE already uses"
        );
        let middling = landing_force(HARD_LAND_SPEED * 1.5);
        assert!(middling > 0.0 && middling < 1.0, "{middling}");
        // Saturation is read off the BAND, not off an arithmetic relationship
        // to the onset. This line used to assert `HARD_LAND_SPEED * 2.0`, which
        // was true only while the full read happened to be twice the onset —
        // so re-fitting the full read to the measured gap broke a test that was
        // pinning a coincidence rather than the rule.
        assert_eq!(landing_force(SplatBand::FLOOR.full), 1.0);
        assert_eq!(
            landing_force(SplatBand::FLOOR.full * 40.0),
            1.0,
            "and it saturates"
        );

        // The emitted beat follows: a heavy arrival adds a sheet the gentle one
        // does not, and throws more of it.
        let gentle = presentation_for_landing_only(HARD_LAND_SPEED);
        let heavy = presentation_for_landing_only(HARD_LAND_SPEED * 3.0);
        assert!(
            heavy.1.len() > gentle.1.len(),
            "a heavy arrival adds the sheet: {} vs {}",
            heavy.1.len(),
            gentle.1.len()
        );
        assert!(
            gentle
                .1
                .iter()
                .all(|m| matches!(m, VfxMessage::Dust { .. })),
            "a gentle landing is exactly the puff it always was: {:?}",
            gentle.1
        );
    }

    /// THE ORDERING FACT this cue is built on, stated where it will be read.
    ///
    /// `MovementOp::Knockdown` is pushed by the CONTROL phase, which runs
    /// before integration and so only observes `on_ground` on the tick AFTER
    /// touchdown — one bundle later than the `Landed { impact_speed }` that
    /// measured the fall. So a knockdown's bundle carries no impact, and any
    /// cue that tried to scale the knockdown by it would read zero forever.
    ///
    /// This pins the CONSEQUENCE rather than the kernel's ordering: given a
    /// knockdown with no landing in its bundle, the emitter must not pretend to
    /// measure one.
    #[test]
    fn a_knockdown_without_a_landing_in_its_bundle_measures_nothing() {
        let mut events = ae::FrameEvents::default();
        events.operations.push(ae::MovementOp::Knockdown);
        // Deliberately NOT `Landed`: this is the bundle the kernel really emits.
        assert_eq!(events.ground_contact.landing_impact_speed(), None);
        let (sfx, vfx) = run_events(events);
        assert!(
            !sfx.iter().any(|m| matches!(m, SfxMessage::Land { .. })),
            "no landing in the bundle means no footfall: {sfx:?}"
        );
        assert_eq!(vfx.len(), 1, "the knockdown's own thump, and nothing else");
    }

    /// A FLOOR BAND CANNOT JUDGE A WALL, and this is the assertion that says so.
    ///
    /// The hardest side contact measured over five matches was 440 px/s; the
    /// floor's onset is 520. So a wall splat sharing the floor's band would be
    /// unreachable — green, shipped, and never once seen — which is the same
    /// bug as a launch trail whose onset sat above every launch in the game.
    #[test]
    fn a_wall_and_a_floor_do_not_share_a_band() {
        const HARDEST_WALL_ARRIVAL_MEASURED: f32 = 440.0;

        assert!(
            impact_splat(HARDEST_WALL_ARRIVAL_MEASURED, false, SplatBand::FLOOR).is_none(),
            "the floor band cannot see the hardest wall arrival this game produces"
        );
        let wall = impact_splat(HARDEST_WALL_ARRIVAL_MEASURED, false, SplatBand::WALL)
            .expect("and the wall band reaches it");
        assert_eq!(wall.force, 1.0, "at its full read");

        // The gap the wall's onset sits in: 54 of 63 sampled side contacts read
        // 52 px/s, a body leaning on the platform lip, and must not splat.
        assert!(impact_splat(52.0, false, SplatBand::WALL).is_none());
        assert!(impact_splat(158.0, false, SplatBand::WALL).is_some());
    }

    /// A CRASH is unconditional; a chosen arrival has to earn its splat.
    ///
    /// `involuntary` rides `GroundContactTransition::Landed` because
    /// presentation cannot reconstruct it — see the ordering note on
    /// `MovementOp::Knockdown`. A quarter of the crashes sampled land under the
    /// hard-landing line, so gating a crash on speed would drop them.
    #[test]
    fn a_crash_splats_at_any_speed_and_a_step_down_does_not() {
        let crawling_crash =
            impact_splat(1.0, true, SplatBand::FLOOR).expect("a crash is a crash at any speed");
        assert!(crawling_crash.crash);
        assert_eq!(
            crawling_crash.force, 0.0,
            "and it is not pretending to be hard"
        );

        assert!(
            impact_splat(1.0, false, SplatBand::FLOOR).is_none(),
            "stepping down is not an event"
        );

        // Same arrival, chosen or not: the crash is the bigger beat.
        let chosen = impact_splat(HARD_LAND_SPEED * 1.5, false, SplatBand::FLOOR).unwrap();
        let crashed = impact_splat(HARD_LAND_SPEED * 1.5, true, SplatBand::FLOOR).unwrap();
        assert_eq!(chosen.force, crashed.force, "speed says how hard");
        assert!(
            crashed.particles > chosen.particles,
            "the flag says who chose"
        );
    }

    /// The emitted beat: a crash adds the ring, an ordinary hard landing does
    /// not, and a wall arrival reaches the same emitter through its own band.
    #[test]
    fn a_crash_rings_and_a_hard_landing_only_dusts() {
        let landed = |speed: f32, involuntary: bool| {
            let mut events = ae::FrameEvents::default();
            events.ground_contact = ae::GroundContactTransition::Landed {
                impact_speed: speed,
                involuntary,
            };
            run_events(events).1
        };

        let hard = landed(HARD_LAND_SPEED * 2.0, false);
        assert!(
            hard.iter().all(|m| !matches!(m, VfxMessage::Impact { .. })),
            "a body that chose to land hard is dust, not a ring: {hard:?}"
        );
        let crash = landed(HARD_LAND_SPEED * 2.0, true);
        assert_eq!(
            crash
                .iter()
                .filter(|m| matches!(m, VfxMessage::Impact { .. }))
                .count(),
            1,
            "a crash rings once: {crash:?}"
        );
    }

    /// The WALL half, end to end — and the negative case is the whole test.
    ///
    /// A `Support` contact must not reach the wall road: the landing transition
    /// already owns the floor, and every tick of a body standing still produces
    /// one (15,330 of them across five matches). A wall loop that matched on
    /// speed instead of on the sim's own `ContactKind` would splat the ground
    /// under a body that was merely standing on it.
    #[test]
    fn only_a_side_contact_splats_against_a_wall() {
        let contact = |kind, impact_speed| {
            let mut events = ae::FrameEvents::default();
            events.contacts.push(ae::collision_semantics::Contact {
                involuntary: false,
                kind,
                point: ae::Vec2::new(40.0, 0.0),
                normal: ae::Vec2::new(-1.0, 0.0),
                toi: 0.0,
                surface_velocity: ae::Vec2::ZERO,
                impact_speed,
                source: ae::collision_semantics::ContactSource::Chain {
                    chain: 0,
                    segment: 0,
                },
            });
            run_events(events).1
        };
        use ae::collision_semantics::ContactKind;

        let wall = contact(ContactKind::Side, 400.0);
        assert_eq!(wall.len(), 1, "a hard side arrival splats: {wall:?}");

        assert!(
            contact(ContactKind::Side, 52.0).is_empty(),
            "and leaning on it does not"
        );
        assert!(
            contact(ContactKind::Support, 1600.0).is_empty(),
            "the floor is the landing transition's, at any speed: a support \
             contact reaching this road would splat every standing body"
        );
        assert!(
            contact(ContactKind::Head, 1600.0).is_empty(),
            "so is a ceiling"
        );
    }

    /// One op plus a landing of the given impact, as `(sfx, vfx)`.
    fn presentation_for_landing(
        op: ae::MovementOp,
        impact_speed: f32,
    ) -> (Vec<SfxMessage>, Vec<VfxMessage>) {
        let mut events = ae::FrameEvents::default();
        events.operations.push(op);
        events.ground_contact = ae::GroundContactTransition::Landed {
            impact_speed,
            involuntary: false,
        };
        run_events(events)
    }

    /// A landing with no floor-game op resolving it.
    fn presentation_for_landing_only(impact_speed: f32) -> (Vec<SfxMessage>, Vec<VfxMessage>) {
        let mut events = ae::FrameEvents::default();
        events.ground_contact = ae::GroundContactTransition::Landed {
            impact_speed,
            involuntary: false,
        };
        run_events(events)
    }

    fn run_events(events: ae::FrameEvents) -> (Vec<SfxMessage>, Vec<VfxMessage>) {
        let mut app = App::new();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.add_message::<VfxMessage>();
        app.insert_resource(TestEvents(events));
        app.add_systems(Update, emit_system);
        app.update();
        let sfx = app
            .world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ambition_sfx::OwnedSfxMessage>>()
            .drain()
            .map(|message| message.request)
            .collect();
        let vfx = app
            .world_mut()
            .resource_mut::<bevy::ecs::message::Messages<VfxMessage>>()
            .drain()
            .collect();
        (sfx, vfx)
    }

    /// Everything one movement op asks for, as `(sfx, vfx)`.
    fn presentation_for(op: ae::MovementOp) -> (Vec<SfxMessage>, Vec<VfxMessage>) {
        let mut events = ae::FrameEvents::default();
        events.operations.push(op);
        let mut app = App::new();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.add_message::<VfxMessage>();
        app.insert_resource(TestEvents(events));
        app.add_systems(Update, emit_system);
        app.update();
        let sfx = app
            .world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ambition_sfx::OwnedSfxMessage>>()
            .drain()
            .map(|message| message.request)
            .collect();
        let vfx = app
            .world_mut()
            .resource_mut::<bevy::ecs::message::Messages<VfxMessage>>()
            .drain()
            .collect();
        (sfx, vfx)
    }

    #[test]
    fn initialized_ground_contact_does_not_emit_landing_presentation() {
        let mut events = ae::FrameEvents::default();
        events.ground_contact = ae::GroundContactTransition::InitializedGrounded;
        let mut app = App::new();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.add_message::<VfxMessage>();
        app.insert_resource(TestEvents(events));
        app.add_systems(Update, emit_system);
        app.update();
        assert_eq!(
            app.world_mut()
                .resource_mut::<bevy::ecs::message::Messages<ambition_sfx::OwnedSfxMessage>>()
                .drain()
                .count(),
            0
        );
        assert_eq!(
            app.world_mut()
                .resource_mut::<bevy::ecs::message::Messages<VfxMessage>>()
                .drain()
                .count(),
            0
        );
    }

    #[test]
    fn landing_animation_arms_only_from_semantic_landing() {
        let mut anim = BodyAnimFacts {
            land_anim_timer: 0.2,
            ..Default::default()
        };
        arm_ground_contact_anim_overlay(
            &mut anim,
            ae::GroundContactTransition::InitializedGrounded,
        );
        assert_eq!(anim.land_anim_timer, 0.0);
        arm_ground_contact_anim_overlay(
            &mut anim,
            ae::GroundContactTransition::Landed {
                impact_speed: 700.0,
                involuntary: false,
            },
        );
        assert!(anim.land_anim_timer > 0.0);
        assert!(anim.land_anim_hard);
    }

    /// The body-generic overlay arming (shared by the player tick AND the actor
    /// tick) sets the wall-jump push-off pose timer on a `WallJump` op and leaves
    /// it untouched otherwise. Pins that an actor which wall-jumps arms the SAME
    /// pose the player does (§A9 follow-up) — a future edit can't silently make
    /// this player-only again.
    #[test]
    fn arm_movement_anim_overlays_arms_wall_jump_pose_on_wall_jump_op() {
        let mut anim = BodyAnimFacts::default();

        let mut plain = ae::FrameEvents::default();
        plain.operations.push(ae::MovementOp::Jump);
        arm_movement_anim_overlays(&mut anim, &plain);
        assert_eq!(
            anim.wall_jump_anim_timer, 0.0,
            "a plain Jump op does NOT arm the wall-jump pose"
        );

        let mut wall = ae::FrameEvents::default();
        wall.operations.push(ae::MovementOp::WallJump);
        arm_movement_anim_overlays(&mut anim, &wall);
        assert_eq!(
            anim.wall_jump_anim_timer, WALL_JUMP_ANIM_HOLD_SECS,
            "a WallJump op arms the push-off pose for the hold duration"
        );
    }
}
