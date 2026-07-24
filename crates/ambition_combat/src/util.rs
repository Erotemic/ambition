//! Grab-bag of small feature-side helpers — not a cohesive subsystem.
//!
//! Collision predicates (`player_is_standing_on`, `approximately_same_aabb`),
//! plain math (`approach` toward-target clamp, `midpoint`, the zero-safe
//! `SignumOr`), keyword-based hazard SFX lookup (`hazard_sfx_id`), and
//! keyword-based hazard SFX lookup. Grep here when
//! a feature system needs a one-liner; no shared theme beyond that.
//! (`room_spec_paths` moved to `features::ecs::spawn` — its only consumer;
//! RoomSpec is world-IR vocabulary combat must not name.)

use super::*;

pub fn player_is_standing_on(player: ae::Aabb, platform: ae::Aabb) -> bool {
    let horizontally_overlaps =
        player.right() > platform.left() + 2.0 && player.left() < platform.right() - 2.0;
    let near_top = (player.bottom() - platform.top()).abs() <= 8.0;
    horizontally_overlaps && near_top
}

// Note: the older `blocked` / `blocked_y` predicates lived here.
// They were ad-hoc collision tests used by enemy / NPC sweep code,
// and their OneWay handling did not differentiate above-vs-below
// approaches — a hostile NPC chasing the player could not drop
// through a one-way platform, breaking the chase. Both paths now
// route through `ambition_engine_core::step_kinematic`, which mirrors
// the player's sweep semantics exactly. Don't reintroduce the
// old helpers; if a new caller needs collision-aware motion, add
// it through `KinematicBody`.

pub fn approximately_same_aabb(a: ae::Aabb, b: ae::Aabb) -> bool {
    // Pogo-bounce routing matches an engine-reported orb AABB against
    // sandbox-side breakable AABBs. The two are derived from the same
    // entity placement so the values agree to floating-point tolerance,
    // but a tiny epsilon avoids spurious mismatches if a future codepath
    // recomputes one of the AABBs from rounded coordinates.
    let eps = 0.5;
    (a.center() - b.center()).length() <= eps && (a.half_size() - b.half_size()).length() <= eps
}

pub fn midpoint(a: ae::Vec2, b: ae::Vec2) -> ae::Vec2 {
    ae::Vec2::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5)
}

/// Pick the SFX bank entry for a hazard contact based on the hazard's
/// authored name. Substring match keeps this resilient to naming
/// drift (e.g. `Lava Pit` and `lava_pool` both resolve to lava splash)
/// without coupling the engine to an SFX-asset enum. Falls back to a
/// generic player-damage clip when no keyword matches.
///
/// Long-term, a typed `HazardKind` field on the engine-side
/// `DamageVolume` would let this dispatch happen on a real enum;
/// until then the substring set is short enough to grep.
pub fn hazard_sfx_id(name: &str) -> ambition_sfx::SfxId {
    let n = name.to_ascii_lowercase();
    if n.contains("lava") {
        ambition_sfx::ids::HAZARD_LAVA_SPLASH
    } else if n.contains("acid") {
        ambition_sfx::ids::HAZARD_ACID_SPLASH
    } else if n.contains("electric") || n.contains("shock") {
        ambition_sfx::ids::HAZARD_ELECTRIC_ARC
    } else if n.contains("saw") {
        ambition_sfx::ids::HAZARD_SAW_HIT
    } else if n.contains("spike") || n.contains("thorn") {
        ambition_sfx::ids::HAZARD_SPIKE_HIT
    } else {
        ambition_sfx::ids::PLAYER_DAMAGE
    }
}

pub trait SignumOr {
    fn signum_or(self, fallback: f32) -> f32;
}

impl SignumOr for f32 {
    fn signum_or(self, fallback: f32) -> f32 {
        if self.abs() <= 0.001 {
            fallback
        } else {
            self.signum()
        }
    }
}

#[cfg(test)]
mod util_tests {
    //! Small pure feature helpers: the toward-target clamp, the
    //! standing-on-platform predicate (horizontal overlap + top contact),
    //! keyword hazard-SFX dispatch, AABB epsilon-equality, midpoint, and
    //! the zero-safe signum.
    use super::*;

    fn aabb(cx: f32, cy: f32, hx: f32, hy: f32) -> ae::Aabb {
        ae::Aabb::new(ae::Vec2::new(cx, cy), ae::Vec2::new(hx, hy))
    }

    #[test]
    fn player_is_standing_on_requires_overlap_and_top_contact() {
        let platform = aabb(50.0, 100.0, 50.0, 10.0); // top = 90
        assert!(player_is_standing_on(
            aabb(50.0, 67.0, 14.0, 23.0),
            platform
        )); // bottom = 90
            // Far above -> no top contact.
        assert!(!player_is_standing_on(
            aabb(50.0, 0.0, 14.0, 23.0),
            platform
        ));
        // Off to the side -> no horizontal overlap.
        assert!(!player_is_standing_on(
            aabb(300.0, 67.0, 14.0, 23.0),
            platform
        ));
    }

    #[test]
    fn hazard_sfx_id_dispatches_by_keyword_case_insensitively() {
        use ambition_sfx::ids;
        assert_eq!(hazard_sfx_id("Lava Pit"), ids::HAZARD_LAVA_SPLASH);
        assert_eq!(hazard_sfx_id("acid_pool"), ids::HAZARD_ACID_SPLASH);
        assert_eq!(hazard_sfx_id("SHOCK coil"), ids::HAZARD_ELECTRIC_ARC);
        assert_eq!(hazard_sfx_id("buzz saw"), ids::HAZARD_SAW_HIT);
        assert_eq!(hazard_sfx_id("thorn bush"), ids::HAZARD_SPIKE_HIT);
        assert_eq!(hazard_sfx_id("mystery goo"), ids::PLAYER_DAMAGE); // fallback
    }

    #[test]
    fn approximately_same_aabb_tolerates_small_epsilon() {
        let a = aabb(10.0, 10.0, 5.0, 5.0);
        assert!(approximately_same_aabb(a, aabb(10.2, 9.9, 5.0, 5.1)));
        assert!(!approximately_same_aabb(a, aabb(20.0, 10.0, 5.0, 5.0)));
    }

    #[test]
    fn midpoint_averages_the_two_points() {
        assert_eq!(
            midpoint(ae::Vec2::new(0.0, 0.0), ae::Vec2::new(10.0, 4.0)),
            ae::Vec2::new(5.0, 2.0)
        );
    }

    #[test]
    fn signum_or_falls_back_inside_the_deadband() {
        assert_eq!(0.0_f32.signum_or(1.0), 1.0);
        assert_eq!(0.0005_f32.signum_or(-1.0), -1.0);
        assert_eq!(5.0_f32.signum_or(9.0), 1.0);
        assert_eq!((-5.0_f32).signum_or(9.0), -1.0);
    }
}

use ambition_characters::actor::BodyCombat;
use ambition_engine_core::{BodyOffense, BodyShieldState};
use ambition_vfx::vfx::{SlashKind, SlashPose, VfxMessage};
use bevy::prelude::MessageWriter;

/// THE one "can this body take a hit right now?" rule, shared by every damage
/// EMITTER that needs an early-out (hazards, enemy hitboxes, boss volumes,
/// body-contact, enemy projectiles). Fable review 2026-07-02 §A5: this
/// predicate was copy-pasted at five emit sites and had already drifted
/// (the projectile site dropped the parry term). i-frames / dodge-roll /
/// parry / invincibility gate a PLAYER-side victim; the actor-side victim
/// consumer applies its own (shield-directional) rule at consume time.
/// `dodge_rolling` is the semantic fact (`BodyMotionFacts::dodge_rolling`) —
/// the roll timer itself is policy-private (ADR 0024).
pub fn body_vulnerable(
    offense: &BodyOffense,
    dodge_rolling: bool,
    shield: &BodyShieldState,
    combat: &BodyCombat,
) -> bool {
    !offense.invincible && !dodge_rolling && !shield.parrying() && combat.vulnerable()
}

/// THE one "is this body an intangible corpse?" rule (Jon 2026-07-22: "prevent
/// intangible things from interacting or presenting in any way"). A body that
/// carries [`BodyHealth`] and has dropped to zero HP is dead: combat treats it
/// as absent until it revives or despawns. No swing lands on it, no impact
/// plays at it, no bark answers it.
///
/// This is the SINGLE tangibility gate the damage *detection* layer consults,
/// so a dead thing is filtered ONCE at every hit boundary rather than relying on
/// each consume-time resolver to re-check `alive()` (those checks remain as
/// last-line defense). A body with no `BodyHealth` — a pure prop — is not
/// governed here; a breakable owns its own `broken()` gate. Extend THIS function
/// for any future intangibility cause (phasing, spawn-in grace) and every combat
/// boundary inherits it at once.
pub fn body_is_corpse(health: Option<&ambition_characters::actor::BodyHealth>) -> bool {
    health.is_some_and(|h| !h.alive())
}

/// Whether a held shield blocks a hit coming from `hit_pos`: you can only guard
/// the local side you face (a hit from behind still lands). A facing of exactly
/// 0 (neutral) guards either side. Pure so the directional rule is unit-tested
/// directly.
pub fn shield_blocks_hit(
    shield_held: bool,
    facing: f32,
    player_pos: ae::Vec2,
    hit_pos: ae::Vec2,
    gravity_dir: ae::Vec2,
) -> bool {
    if !shield_held {
        return false;
    }
    if facing == 0.0 {
        return true;
    }
    let frame = ae::AccelerationFrame::new(gravity_dir);
    let local_side_delta = frame.to_local(hit_pos - player_pos).x;
    // Same local-side sign => the hit is on the side the controlled body faces.
    local_side_delta.signum() == facing.signum()
}

/// On-screen size for the slash effect: a flourish a bit larger than the
/// hitbox so the swing reads beyond the exact damage box. Takes the world
/// hitbox half-extent. Tunable.
fn slash_effect_size(hitbox_half_size: ae::Vec2) -> f32 {
    const SLASH_EFFECT_SCALE: f32 = 2.0;
    ((hitbox_half_size * 2.0).max_element() * SLASH_EFFECT_SCALE).max(24.0)
}

/// THE single melee-slash effect emit. EVERY body's melee — the player AND any
/// brain-driven actor — draws its swing through this one function, so the slash
/// visual has exactly ONE definition (size curve + message shape). `center` is the
/// world hitbox center, `half_size` its half-extent, `dir` the gravity-relative
/// body→strike offset (the renderer rotates the art along it).
///
/// ONE BODY, ONE PATH: do NOT add another `VfxMessage::Slash` site — call this.
/// The former two-state-machine fork (the flat `MeleeSwing`/`BodyMelee` driver
/// vs the `MovePlayback` moveset) is collapsed: melee is a `"attack"`-verb
/// moveset move for every body, and `advance_move_playback` is the sole caller
/// on the strike path.
pub fn emit_melee_slash(
    vfx: &mut MessageWriter<VfxMessage>,
    center: ae::Vec2,
    half_size: ae::Vec2,
    kind: SlashKind,
    pose: SlashPose,
    dir: ae::Vec2,
) {
    vfx.write(VfxMessage::Slash {
        center,
        size: slash_effect_size(half_size),
        kind,
        pose,
        dir,
    });
}

/// THE one victim-side hit-feedback reaction (CM8). Every place a strike LANDS
/// on a body — the player-hurt path, the actor-hurt path, the boss-hurt path —
/// calls this ONE rule at the moment its `resolve_body_hit` reports the hit
/// registered, so the payload is never chosen by an `is_player` branch again.
///
/// The split it enforces: the ATTACK owns its `strike_sfx` (a sword and a claw
/// sound apart), which overrides only the SOUND; the VICTIM owns the spray and
/// debris through its [`HurtFeedback`] (an enemy struck by anything uses
/// [`HurtFeedback::ENEMY`] and so never throws the player's red hurt burst,
/// while the player always keeps its own). Exactly one hit sound plays: the
/// attack's if it authored one, else the victim's default. The universal impact
/// spark always fires; the richer spray/debris only if the victim's profile
/// carries them.
///
/// Muting for dodged / parried / i-framed hits needs no gate here: the callers
/// invoke this only on the LANDED branch, after `resolve_body_hit` already
/// dropped an ignored hit.
pub fn emit_hit_feedback(
    sfx: &mut ambition_sfx::SfxWriter,
    vfx: &mut MessageWriter<VfxMessage>,
    debris: &mut MessageWriter<DebrisBurstMessage>,
    hurt: ambition_vfx::HurtFeedback,
    strike_sfx: Option<ambition_sfx::SfxId>,
    pos: ae::Vec2,
) {
    sfx.write(ambition_sfx::SfxMessage::Play {
        id: strike_sfx.unwrap_or(hurt.sfx),
        pos,
    });
    vfx.write(VfxMessage::Impact { pos });
    if let Some(burst) = hurt.burst {
        vfx.write(burst.message(pos));
    }
    if let Some(cue) = hurt.debris {
        debris.write(DebrisBurstMessage { pos, cue });
    }
}

/// THE knockback-scaling law (CM1): the smash-percent growth term folded onto a
/// hit's base knockback. A body that has accumulated more damage launches
/// farther under the same hit, scaled down by its weight. Pure and
/// frame-agnostic so it is unit-tested directly and reused by every hit path.
///
/// `base` is the volume's flat knockback; `growth` is the authored `kb_growth`;
/// `victim_damage_taken` is `BodyHealth::damage_taken()`; `victim_weight` is the
/// archetype weight (reference `1.0`). PARITY: `growth == 0.0` returns `base`
/// exactly, so every un-authored volume is byte-identical to today.
pub fn scaled_knockback(
    base: f32,
    growth: f32,
    victim_damage_taken: i32,
    victim_weight: f32,
) -> f32 {
    if growth == 0.0 {
        return base;
    }
    let weight = if victim_weight > 0.0 {
        victim_weight
    } else {
        1.0
    };
    base + growth * victim_damage_taken.max(0) as f32 / weight
}

#[cfg(test)]
mod hit_feedback_tests {
    use super::*;
    use ambition_sfx::{ids, OwnedSfxMessage, SfxId};
    use ambition_vfx::vfx::DebrisBurstMessage;
    use ambition_vfx::{HurtFeedback, VfxMessage};
    use bevy::ecs::message::Messages;
    use bevy::prelude::*;

    #[derive(Resource, Clone, Copy)]
    struct Input {
        hurt: HurtFeedback,
        strike: Option<SfxId>,
    }

    struct Emitted {
        sfx: Vec<SfxId>,
        impacts: usize,
        bursts: usize,
        debris: usize,
    }

    fn emit_system(
        input: Res<Input>,
        mut sfx: ambition_sfx::SfxWriter,
        mut vfx: MessageWriter<VfxMessage>,
        mut debris: MessageWriter<DebrisBurstMessage>,
    ) {
        emit_hit_feedback(
            &mut sfx,
            &mut vfx,
            &mut debris,
            input.hurt,
            input.strike,
            ae::Vec2::ZERO,
        );
    }

    /// Drive the REAL `emit_hit_feedback` (through the real `SfxWriter`) once and
    /// collect exactly what reached each bus.
    fn run(hurt: HurtFeedback, strike: Option<SfxId>) -> Emitted {
        let mut world = World::new();
        world.init_resource::<Messages<OwnedSfxMessage>>();
        world.init_resource::<Messages<VfxMessage>>();
        world.init_resource::<Messages<DebrisBurstMessage>>();
        world.insert_resource(Input { hurt, strike });
        let mut schedule = Schedule::default();
        schedule.add_systems(emit_system);
        schedule.run(&mut world);

        let sfx_msgs = world.resource::<Messages<OwnedSfxMessage>>();
        let mut cursor = sfx_msgs.get_cursor();
        let sfx: Vec<SfxId> = cursor
            .read(sfx_msgs)
            .filter_map(|m| match m.request {
                ambition_sfx::SfxMessage::Play { id, .. } => Some(id),
                _ => None,
            })
            .collect();

        let vfx_msgs = world.resource::<Messages<VfxMessage>>();
        let mut vcursor = vfx_msgs.get_cursor();
        let mut impacts = 0;
        let mut bursts = 0;
        for m in vcursor.read(vfx_msgs) {
            match m {
                VfxMessage::Impact { .. } => impacts += 1,
                VfxMessage::Burst { .. } => bursts += 1,
                _ => {}
            }
        }

        let debris_msgs = world.resource::<Messages<DebrisBurstMessage>>();
        let mut dcursor = debris_msgs.get_cursor();
        let debris = dcursor.read(debris_msgs).count();

        Emitted {
            sfx,
            impacts,
            bursts,
            debris,
        }
    }

    /// The exact CM8 property: a sword and a claw are heard apart. Two different
    /// authored strike sounds over the SAME victim profile produce two different
    /// hit sounds — no `is_player` branch anywhere selects the payload.
    #[test]
    fn a_sword_and_a_claw_are_heard_apart() {
        let sword = SfxId::new("weapon.sword");
        let claw = SfxId::new("creature.claw");
        let with_sword = run(HurtFeedback::ENEMY, Some(sword));
        let with_claw = run(HurtFeedback::ENEMY, Some(claw));
        assert_eq!(with_sword.sfx, vec![sword], "the sword plays its own sound");
        assert_eq!(with_claw.sfx, vec![claw], "the claw plays its own sound");
        assert_ne!(
            with_sword.sfx, with_claw.sfx,
            "different attacks on the same body sound different"
        );
    }

    /// The CM8 bug: an enemy struck by another enemy used to play the player's
    /// `PLAYER_DAMAGE` sound and the red "you got hurt" burst. The ENEMY profile
    /// carries neither, so no `is_player`-flavored payload can leak onto a
    /// non-player victim.
    #[test]
    fn an_enemy_victim_never_throws_the_player_hurt_burst() {
        // Unauthored enemy-vs-enemy hit (no strike sound → the victim's default).
        let e = run(HurtFeedback::ENEMY, None);
        assert_eq!(
            e.sfx,
            vec![ids::PLAYER_HIT],
            "plain hit tick, not PLAYER_DAMAGE"
        );
        assert_ne!(
            e.sfx,
            vec![ids::PLAYER_DAMAGE],
            "the enemy must NOT play the player's hurt grunt"
        );
        assert_eq!(e.bursts, 0, "no red hurt burst on an enemy victim");
        assert_eq!(e.debris, 0, "no hurt debris on an enemy victim");
        assert_eq!(e.impacts, 1, "still a plain impact spark");
    }

    /// The player keeps its rich hurt reaction — its red burst + debris + grunt —
    /// because that reaction is the VICTIM's profile, not an attacker-side
    /// `is_player` branch. And an authored strike sound overrides only the SOUND,
    /// never the player's spray (so a sword still makes the player flash red).
    #[test]
    fn the_player_keeps_its_hurt_burst_under_any_attack() {
        let unauthored = run(HurtFeedback::PLAYER, None);
        assert_eq!(unauthored.sfx, vec![ids::PLAYER_DAMAGE]);
        assert_eq!(unauthored.bursts, 1, "player throws its red hurt burst");
        assert_eq!(unauthored.debris, 1, "player throws hurt debris");

        let sword = SfxId::new("weapon.sword");
        let by_sword = run(HurtFeedback::PLAYER, Some(sword));
        assert_eq!(
            by_sword.sfx,
            vec![sword],
            "the sword's sound overrides the grunt"
        );
        assert_eq!(
            by_sword.bursts, 1,
            "but the player STILL throws its own red hurt burst"
        );
        assert_eq!(by_sword.debris, 1);
    }
}
