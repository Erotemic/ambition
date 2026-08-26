//! Small feature-side helpers that do not own a subsystem.

use super::*;

pub fn player_is_standing_on(player: ae::Aabb, platform: ae::Aabb) -> bool {
    let horizontally_overlaps =
        player.right() > platform.left() + 2.0 && player.left() < platform.right() - 2.0;
    let near_top = (player.bottom() - platform.top()).abs() <= 8.0;
    horizontally_overlaps && near_top
}

// Collision-aware body motion belongs in `ambition_platformer2d_core::step_motion`;
// do not add feature-side collision kernels here.

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

/// Pick a hazard-contact SFX from keywords in the authored hazard name.
/// Falls back to generic player damage when no keyword matches.
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
use ambition_characters::actor::Invulnerability;
use ambition_platformer2d_core::BodyShieldState;
use ambition_vfx::vfx::{SlashKind, SlashPose, VfxMessage};
use bevy::prelude::MessageWriter;

/// THE one "can this body take a hit right now?" rule, shared by every damage EMITTER that needs an
/// early-out (hazards, enemy hitboxes, boss volumes, body-contact, enemy projectiles).
pub fn body_vulnerable(
    invulnerable: Invulnerability,
    evading: bool,
    shield: &BodyShieldState,
    combat: &BodyCombat,
) -> bool {
    !invulnerable.any() && !evading && !shield.parrying() && combat.vulnerable()
}

/// This is the SINGLE tangibility gate the damage *detection* layer consults,
/// so an untouchable thing is filtered ONCE at every hit boundary rather than
/// relying on each consume-time resolver to re-check (those checks remain as
/// last-line defense). A body with no `BodyHealth` — a pure prop — is not
/// governed here; a breakable owns its own `broken()` gate.
///
/// ⭐⭐ THE DOC ABOVE PROMISED THIS AND THEN THE PROMISE WAS COLLECTED ON:
/// *"extend THIS function for any future intangibility cause (phasing, spawn-in
/// grace) and every combat boundary inherits it at once."* `OutOfPlay` is that
/// cause, and D201 made it a body-generic fact rather than a player-only one.
///
/// ⛔⛔ AND A BODY WAITING OUT ITS DEATH BEAT IS NOT A CORPSE, which is why the
/// health check alone could not see it. `spend_fighter_stocks` calls
/// `health.reset()` the moment the stock is spent — a fighter comes back FRESH —
/// so for the whole interlude the body reads ALIVE while `OutOfPlay` says the
/// world's hands are off it. It could be struck, could shield, could be counted
/// as a connect, and could accumulate percent it then carried into its next
/// stock.
///
/// ⛔ THE ARGUMENT IS SEPARATE RATHER THAN LOOKED UP because this runs inside
/// detection loops that already hold the victim's components; a lookup here
/// would need a `World` this has no business taking. Changing the signature is
/// deliberate: it makes every existing boundary a compile error until it answers
/// the new question, which is the only way a gate that claims to be THE gate
/// stays one.
/// ⭐ AND IT IS A PARTICIPATION QUESTION, NOT A DEFENSIVE ONE, which is why
/// `OutOfPlay` belongs here and NOT in [`ambition_characters::actor::Invulnerability`].
/// This decides whether the world may reach a body at all — it gates strikes,
/// captures, footstools, contact harm, interaction prompts and TARGET SELECTION.
/// [`body_vulnerable`] answers the later, narrower question: this body was
/// reached, may the hit hurt it? Folding out-of-play into the invulnerability
/// bitset would answer the second question in the first one's place, and a
/// hunter would go on chasing a body it merely could not damage.
pub fn body_is_untouchable(
    health: Option<&ambition_characters::actor::BodyHealth>,
    out_of_play: bool,
) -> bool {
    out_of_play || health.is_some_and(|h| !h.alive())
}

/// Does a SPENT guard still reach this hit? — the poke rule.
///
/// A shield sinks as it is spent, and a body behind a small one is exposed at
/// the head and the feet. `coverage` is the fraction of the body's half-height
/// the guard still covers ([`ambition_platformer2d_core::ShieldTuning::coverage_at`]);
/// `1.0` covers everything and can never poke.
///
/// `tilt` shifts the covered BAND rather than widening it
/// ([`ambition_platformer2d_core::ShieldTuning::tilt_bias`]) — a signed
/// fraction of half-height, positive toward the feet. Shifting toward one end
/// exposes the other by the same amount, which is the trade that makes tilting
/// a decision. It does nothing to a guard that already covers everything: a
/// full shield has no exposed end to choose between, and only a SPENT one gives
/// the player that question.
///
/// measured along the body's own gravity axis, not screen Y, so a wall-walker
/// is poked at the ends of the axis it actually stands on.
///
/// separate from [`shield_blocks_hit`] rather than folded into it. That
/// function answers *which SIDE can this guard face*, which is a different
/// question from *how much of me does it still cover* — one is about facing and
/// the other about the resource. A single predicate would make a poke look like
/// a facing failure in every trace that reads it.
pub fn guard_covers_hit(
    coverage: f32,
    tilt: f32,
    body_pos: ae::Vec2,
    body_size: ae::Vec2,
    hit_pos: ae::Vec2,
    gravity_dir: ae::Vec2,
) -> bool {
    if coverage >= 1.0 {
        return true;
    }
    let frame = ae::AccelerationFrame::new(gravity_dir);
    let half_height = frame.to_world_half(body_size * 0.5).dot(gravity_dir).abs();
    let along = (hit_pos - body_pos).dot(gravity_dir);
    let centre = half_height * tilt;
    (along - centre).abs() <= half_height * coverage.max(0.0)
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

/// Swing art uses the exact authored combat volume; it must not overshoot damage geometry.
const SLASH_ART_MARGIN: f32 = 1.0;

/// Emit the shared melee-slash visual for any body.
///
/// Melee presentation has one path; callers should route slash effects through this function.
pub fn emit_melee_slash(
    vfx: &mut MessageWriter<VfxMessage>,
    volume: &ae::CombatVolume,
    from: ae::Vec2,
    owner: bevy::prelude::Entity,
    kind: SlashKind,
    pose: SlashPose,
) {
    // BODY-LOCAL, because the effect travels with the body. Resolving the swing
    // here and then subtracting the attacker is not a detour: the volume is
    // already placed in the world by `FollowOwner`, and the projection needs it
    // there to know which end is the handle.
    let shape = volume
        .swing_shape(from)
        .scaled(SLASH_ART_MARGIN)
        .translated(-from);
    vfx.write(VfxMessage::Slash {
        shape,
        owner,
        kind,
        pose,
    });
}

/// Damage at or above this value reads as a committed/heavy contact for the
/// canonical robot blade. Ordinary jabs and default slashes remain light.
///
/// NOT REACHABLE BY ANY SHIPPED ATTACK TODAY. The robot player's only slash comes from
/// `default_player_action_set` at `damage: 1` on a prefab whose `smash_charge_mult` is `1.0`,
/// and no equipment path scales melee damage — so every strike resolves LIGHT and the rendered
/// `…flesh.deep` / `…metal.gong` samples cannot currently play.
const PLAYER_ROBOT_DEEP_IMPACT_DAMAGE: i32 = 3;

/// Resolve an attack-owned strike id against the victim's material profile and
/// the already-authored damage strength. Only the canonical robot-player
/// selector is special; every concrete authored strike id remains byte-for-byte
/// attack-owned behavior.
pub fn resolve_strike_sfx(
    hurt: ambition_vfx::HurtFeedback,
    strike_sfx: Option<ambition_sfx::SfxId>,
    damage: i32,
) -> ambition_sfx::SfxId {
    let Some(strike) = strike_sfx else {
        return hurt.sfx;
    };
    if strike != ambition_sfx::ids::PLAYER_ROBOT_SLASH_IMPACT {
        return strike;
    }
    let deep = damage >= PLAYER_ROBOT_DEEP_IMPACT_DAMAGE;
    match hurt.material {
        ambition_vfx::ImpactMaterial::Flesh if deep => {
            ambition_sfx::ids::PLAYER_ROBOT_SLASH_IMPACT_FLESH_DEEP
        }
        ambition_vfx::ImpactMaterial::Flesh => {
            ambition_sfx::ids::PLAYER_ROBOT_SLASH_IMPACT_FLESH_LIGHT
        }
        ambition_vfx::ImpactMaterial::Robot => ambition_sfx::ids::PLAYER_ROBOT_SLASH_IMPACT_ROBOT,
        ambition_vfx::ImpactMaterial::Metal if deep => {
            ambition_sfx::ids::PLAYER_ROBOT_SLASH_IMPACT_METAL_GONG
        }
        ambition_vfx::ImpactMaterial::Metal => {
            ambition_sfx::ids::PLAYER_ROBOT_SLASH_IMPACT_METAL_CHINK
        }
    }
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
/// attack's if it authored one, else the victim's default — except for the one
/// attack-owned MATERIAL SELECTOR, which asks to be resolved against the
/// victim's [`ambition_vfx::ImpactMaterial`] (see [`resolve_strike_sfx`]); the
/// attack still chooses whether to ask. The universal impact spark always fires;
/// the richer spray/debris only if the victim's profile carries them.
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
    damage: i32,
    pos: ae::Vec2,
    // BOTH sides' presentation sources, because this function emits ONE sound
    // whose provenance depends on which cue won.
    //
    // The split this function already enforces — the ATTACK owns the sound, the
    // VICTIM owns the spray — is exactly the split attribution needs. An authored
    // `strike_sfx` is the attacker's cue and must resolve in the ATTACKER's bank;
    // the fallback is the victim's `HurtFeedback::sfx` and must resolve in the
    // VICTIM's. Passing one source for both would mis-attribute half the hits in
    // any fight between two providers, which is the only kind of fight this
    // machinery exists for.
    //
    // `None` on either side falls back to the session provider, which is the right
    // answer for a hazard, a breakable, or a body wearing no character.
    attacker_source: Option<&ambition_sfx::PresentationSourceId>,
    victim_source: Option<&ambition_sfx::PresentationSourceId>,
) {
    // Which bank the resolved sound belongs to follows `resolve_strike_sfx`'s own
    // precedence: an authored strike sound wins (attacker), otherwise the victim's
    // hurt profile answers.
    let sound_source = if strike_sfx.is_some() {
        attacker_source
    } else {
        victim_source
    };
    sfx.write_for_body(
        sound_source,
        ambition_sfx::SfxMessage::Play {
            id: resolve_strike_sfx(hurt, strike_sfx, damage),
            pos,
        },
    );
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
/// `base` is the volume's flat knockback; `growth` is the authored `knockback_growth`;
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
    use ambition_sfx::{OwnedSfxMessage, SfxId, ids};
    use ambition_vfx::vfx::DebrisBurstMessage;
    use ambition_vfx::{HurtFeedback, VfxMessage};
    use bevy::ecs::message::Messages;
    use bevy::prelude::*;

    #[derive(Resource, Clone, Copy)]
    struct Input {
        hurt: HurtFeedback,
        strike: Option<SfxId>,
        damage: i32,
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
            input.damage,
            ae::Vec2::ZERO,
            None,
            None,
        );
    }

    /// Drive the REAL `emit_hit_feedback` (through the real `SfxWriter`) once and
    /// collect exactly what reached each bus.
    fn run_at_damage(hurt: HurtFeedback, strike: Option<SfxId>, damage: i32) -> Emitted {
        let mut world = World::new();
        world.init_resource::<Messages<OwnedSfxMessage>>();
        world.init_resource::<Messages<VfxMessage>>();
        world.init_resource::<Messages<DebrisBurstMessage>>();
        world.insert_resource(Input {
            hurt,
            strike,
            damage,
        });
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

    fn run(hurt: HurtFeedback, strike: Option<SfxId>) -> Emitted {
        run_at_damage(hurt, strike, 1)
    }

    /// The player-robot selector is the only strike id that consults victim
    /// material and authored damage strength; all concrete ids remain authored
    /// attack sounds.
    #[test]
    fn robot_player_selector_resolves_material_and_contact_depth() {
        assert_eq!(
            resolve_strike_sfx(HurtFeedback::ENEMY, Some(ids::PLAYER_ROBOT_SLASH_IMPACT), 1),
            ids::PLAYER_ROBOT_SLASH_IMPACT_FLESH_LIGHT,
        );
        assert_eq!(
            resolve_strike_sfx(
                HurtFeedback::ENEMY,
                Some(ids::PLAYER_ROBOT_SLASH_IMPACT),
                PLAYER_ROBOT_DEEP_IMPACT_DAMAGE
            ),
            ids::PLAYER_ROBOT_SLASH_IMPACT_FLESH_DEEP,
        );
        assert_eq!(
            resolve_strike_sfx(HurtFeedback::ROBOT, Some(ids::PLAYER_ROBOT_SLASH_IMPACT), 8),
            ids::PLAYER_ROBOT_SLASH_IMPACT_ROBOT,
        );
        assert_eq!(
            resolve_strike_sfx(HurtFeedback::METAL, Some(ids::PLAYER_ROBOT_SLASH_IMPACT), 1),
            ids::PLAYER_ROBOT_SLASH_IMPACT_METAL_CHINK,
        );
        assert_eq!(
            resolve_strike_sfx(
                HurtFeedback::METAL,
                Some(ids::PLAYER_ROBOT_SLASH_IMPACT),
                PLAYER_ROBOT_DEEP_IMPACT_DAMAGE
            ),
            ids::PLAYER_ROBOT_SLASH_IMPACT_METAL_GONG,
        );
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

/// THE FOUR REASONS A BODY CANNOT BE HIT, each one alone.
///
/// `body_vulnerable` had SIX production callers and no test. It is the
/// single gate every damage boundary consults — hazards, empowerment, the actor
/// update, the damage resolver, the avatar's body integration — and the one that
/// decides whether a PARRY works at all, because a parry is not a block: a
/// parried hit never registers, where a blocked one registers and arms a guard
/// i-frame. That distinction is the whole of *"does the parry window read"*, and
/// nothing asserted it.
///
/// each term is toggled ALONE against an otherwise-vulnerable body, which
/// is what makes this a test of the GATE rather than of one scenario: a gate
/// that had lost any single term would still pass a test that raised two.
///
/// `parrying()` is `active && parry_window_timer > 0.0`, so the last case is
/// the one that separates a parry from a held shield — a shield that is up but
/// past its window leaves the body vulnerable HERE and is caught downstream by
/// `shield_blocks_hit` instead. Two defences, two gates, different outcomes.
#[cfg(test)]
mod vulnerability_gate_tests {
    use super::body_vulnerable;
    use ambition_characters::actor::{BodyCombat, Invulnerability};
    use ambition_platformer2d_core::BodyShieldState;

    fn open() -> (Invulnerability, bool, BodyShieldState, BodyCombat) {
        (
            Invulnerability::none(),
            false,
            BodyShieldState::default(),
            BodyCombat::default(),
        )
    }

    #[test]
    fn a_body_with_nothing_holding_it_is_vulnerable() {
        let (inv, evading, shield, combat) = open();
        assert!(
            body_vulnerable(inv, evading, &shield, &combat),
            "an unguarded, un-invulnerable, non-evading body outside its i-frames \
             cannot be hit — every clause below is measured against this one, so \
             a false here makes them all vacuous"
        );
    }

    /// ⭐ THE M3 CONTRACT. A move's authored `Invuln` window is one more REASON
    /// in the same bitset, so the gate answers it with nothing new taught — and
    /// so does `unhittable`, which is this expression inverted. A move grant
    /// gated anywhere else would give presentation a second answer to read.
    #[test]
    fn a_move_invuln_window_is_answered_by_the_one_gate() {
        let shield = BodyShieldState::default();
        let combat = BodyCombat::default();
        let mut invulnerable = Invulnerability::none();
        assert!(body_vulnerable(invulnerable, false, &shield, &combat));

        invulnerable.set(Invulnerability::MOVE, true);
        assert!(
            !body_vulnerable(invulnerable, false, &shield, &combat),
            "a body inside its move's authored Invuln window was struck — the \
             grant is being gated somewhere this rule cannot see"
        );

        // And it is ONE reason among several: releasing it must not release a
        // grant somebody else is holding.
        invulnerable.set(Invulnerability::SCRIPTED, true);
        invulnerable.set(Invulnerability::MOVE, false);
        assert!(
            !body_vulnerable(invulnerable, false, &shield, &combat),
            "the move window's release dropped a scripted grant it does not own"
        );
    }

    #[test]
    fn parrying_alone_makes_a_body_untouchable() {
        let (inv, evading, mut shield, combat) = open();
        shield.active = true;
        shield.parry_window_timer = 0.05;
        assert!(
            !body_vulnerable(inv, evading, &shield, &combat),
            "a body inside its parry window took the hit — the parry is a \
             DIFFERENT defence from the block, and this gate is the only thing \
             that makes it one"
        );
    }

    /// THE POISON that separates the two defences: a shield held past its
    /// parry window is NOT invulnerable here. It may still block, which is
    /// `shield_blocks_hit`'s business downstream and a different outcome
    /// (`Blocked`, with a guard i-frame) from never being hit at all.
    #[test]
    fn a_shield_past_its_parry_window_leaves_the_body_vulnerable_to_this_gate() {
        let (inv, evading, mut shield, combat) = open();
        shield.active = true;
        shield.parry_window_timer = 0.0;
        assert!(
            body_vulnerable(inv, evading, &shield, &combat),
            "merely holding a shield made the body untouchable, so a parry is \
             indistinguishable from a guard and the timing means nothing"
        );
    }

    #[test]
    fn evading_alone_makes_a_body_untouchable() {
        let (inv, _, shield, combat) = open();
        assert!(!body_vulnerable(inv, true, &shield, &combat));
    }

    #[test]
    fn any_single_invulnerability_reason_makes_a_body_untouchable() {
        for reason in [
            Invulnerability::TRANSFORMING,
            Invulnerability::EMPOWERED,
            Invulnerability::SCRIPTED,
        ] {
            let (mut inv, evading, shield, combat) = open();
            inv.set(reason, true);
            assert!(
                !body_vulnerable(inv, evading, &shield, &combat),
                "one reason held ({reason}) left the body hittable, so the gate \
                 is reading something narrower than `any()`"
            );
        }
    }
}

#[cfg(test)]
mod poke_tests {
    use super::*;

    const SIZE: ae::Vec2 = ae::Vec2::new(24.0, 40.0);
    const CARDINALS: [ae::Vec2; 4] = [
        ae::Vec2::new(0.0, 1.0),
        ae::Vec2::new(0.0, -1.0),
        ae::Vec2::new(1.0, 0.0),
        ae::Vec2::new(-1.0, 0.0),
    ];

    /// A FULL GUARD COVERS EVERYTHING, A SPENT ONE EXPOSES THE ENDS.
    ///
    /// the property that makes chip pressure end in a hit rather than in a
    /// stalemate: hold a shield long enough and the head and the feet come out
    /// from behind it. `coverage >= 1.0` can never poke, which is what keeps
    /// every exploration body — none of which authors a shield resource —
    /// exactly as protected as it was.
    #[test]
    fn a_spent_guard_stops_reaching_the_head_and_the_feet() {
        for dir in CARDINALS {
            let frame = ae::AccelerationFrame::new(dir);
            let half = frame.to_world_half(SIZE * 0.5).dot(dir).abs();
            let body = ae::Vec2::ZERO;
            // A hit near the far end of the body's own gravity axis.
            let at_the_end = dir * (half * 0.9);

            assert!(
                guard_covers_hit(1.0, 0.0, body, SIZE, at_the_end, dir),
                "a WHOLE guard failed to cover its own body under gravity {dir:?}"
            );
            assert!(
                !guard_covers_hit(0.45, 0.0, body, SIZE, at_the_end, dir),
                "a guard at 45% still covered the end of the body under gravity {dir:?}"
            );
            // ...while the middle stays covered at the same coverage.
            assert!(
                guard_covers_hit(0.45, 0.0, body, SIZE, dir * (half * 0.2), dir),
                "a guard at 45% failed to cover the body's own middle"
            );
        }
    }

    /// TILTING A SPENT GUARD BUYS ONE END BY SELLING THE OTHER.
    ///
    /// The three arms are the whole mechanic, and the middle one alone would
    /// not be: a rule that only ever COVERS MORE is a free upgrade every player
    /// would hold the stick for permanently, and it would be indistinguishable
    /// from simply raising `min_coverage`. The cost arm is what makes the tilt
    /// a decision, so it is asserted beside the benefit rather than trusted.
    #[test]
    fn a_tilt_trades_one_end_of_the_body_for_the_other() {
        for dir in CARDINALS {
            let frame = ae::AccelerationFrame::new(dir);
            let half = frame.to_world_half(SIZE * 0.5).dot(dir).abs();
            let body = ae::Vec2::ZERO;
            // `dir` points along gravity, so this end is the FEET and the
            // opposite one is the head.
            let feet = dir * (half * 0.9);
            let toward_head = dir * (half * -0.3);
            let lean_to_the_feet = 0.5;

            // BASELINE, asserted first: untilted, the feet are out and a point
            // just off centre is still behind the guard. Without this arm the
            // two below could both pass on a rule that covers nothing at all.
            assert!(
                !guard_covers_hit(0.45, 0.0, body, SIZE, feet, dir),
                "the untilted baseline changed: a 45% guard covered the feet under {dir:?}"
            );
            assert!(
                guard_covers_hit(0.45, 0.0, body, SIZE, toward_head, dir),
                "the untilted baseline changed: a 45% guard missed its own middle under {dir:?}"
            );

            // THE BENEFIT: lean toward the feet and the poke that was getting
            // through no longer does.
            assert!(
                guard_covers_hit(0.45, lean_to_the_feet, body, SIZE, feet, dir),
                "leaning toward the feet did not cover a hit on them under {dir:?}"
            );
            // THE COST, and the arm that separates a shift from a widening:
            // the same lean hands over ground the guard used to hold.
            assert!(
                !guard_covers_hit(0.45, lean_to_the_feet, body, SIZE, toward_head, dir),
                "leaning toward the feet cost the head nothing under {dir:?} — \
                 the band WIDENED instead of moving"
            );

            // A WHOLE GUARD HAS NOTHING TO TRADE. Every exploration body is
            // here, and a tilt must not be able to open a hole in one.
            assert!(
                guard_covers_hit(1.0, lean_to_the_feet, body, SIZE, toward_head, dir),
                "a tilt opened a gap in an unlimited guard under {dir:?}"
            );
        }
    }

    /// WHICH WAY THE STICK LEANS THE GUARD, and that a body with no declared
    /// range cannot lean at all.
    #[test]
    fn the_stick_leans_the_guard_toward_the_end_it_points_at() {
        let fighter = ae::ShieldTuning::PLATFORM_FIGHTER;
        // `LocalAxes` is +y TOWARD-FEET, so DOWN is the positive stick.
        assert!(
            fighter.tilt_bias(1.0) > 0.0,
            "holding DOWN must lean the guard toward the feet (+gravity)"
        );
        assert!(
            fighter.tilt_bias(-1.0) < 0.0,
            "holding UP must lean the guard toward the head"
        );
        assert_eq!(fighter.tilt_bias(0.0), 0.0, "a centred stick must not lean");
        // The stick cannot buy more lean than the body declares.
        assert_eq!(fighter.tilt_bias(4.0), fighter.tilt_bias(1.0));
        assert_eq!(
            ae::ShieldTuning::OFF.tilt_bias(1.0),
            0.0,
            "a body that declares no tilt range must not lean"
        );
    }

    /// COVERAGE FALLS WITH INTEGRITY, AND A NON-RESOURCE NEVER SHRINKS.
    #[test]
    fn coverage_tracks_integrity_and_an_unlimited_guard_is_whole() {
        let fighter = ae::ShieldTuning::PLATFORM_FIGHTER;
        assert_eq!(fighter.coverage_at(1.0), 1.0);
        assert_eq!(fighter.coverage_at(0.0), fighter.min_coverage);
        assert!(fighter.coverage_at(0.5) < 1.0);
        assert!(fighter.coverage_at(0.5) > fighter.min_coverage);

        // the floor: a body with no shield resource is never poked, whatever
        // its integrity reads.
        assert_eq!(ae::ShieldTuning::OFF.coverage_at(0.0), 1.0);
    }
}
