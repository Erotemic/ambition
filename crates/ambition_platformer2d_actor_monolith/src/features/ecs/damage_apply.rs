//! Victim-side combat damage resolution: shield blocks, knockback, hitstun,
//! safe-respawn, and death-respawn for the controlled body, plus the
//! `apply_player_hit_events` system that drives them off the victim-side
//! `HitEvent` channel.
//!
//! This is runtime (sim) logic — it emits `SfxMessage`/`VfxMessage` *facts* but
//! holds no render dependency (`VfxMessage` is `ambition_vfx`, not
//! `ambition_render`). It lived in `ambition_app` only because it was authored
//! beside the room/attack glue; it belongs here in the combat runtime, where the
//! state it mutates already lives.
//!
//! Player-centrism note: the bodies still name the controlled actor "player"
//! because the component vocabulary (`BodyCombat`, `ae::BodyClustersMut`)
//! does. The relativity-principle fix is the actor-unification rename of those
//! types, tracked separately; this drain only relocates and de-render-couples.

use bevy::prelude::{Entity, MessageReader, MessageWriter, Query, Res, ResMut};

use ambition_platformer2d_core as ae;
use ambition_platformer2d_world::collision::MovingPlatformSet;
use ambition_vfx::vfx::{DebrisBurstMessage, VfxMessage};

use crate::actor::BodyAnimFacts;
use crate::actor::PrimaryPlayerOnly;
use crate::avatar::PlayerSafetyState;
use crate::combat::events::{GameplayBannerRequested, HitEvent as FeatureHitEvent, HitTarget};
use crate::time::feel::Platformer2dFeelTuningMonolith;
use crate::time::time_control::{ClockRequester, ClockResetRequest};
use crate::{
    remember_safe_player_position, ActorDiedMessage, RoomTransitionCooldown, SafePositionContext,
};
use ambition_characters::actor::{BodyCombat, BodyHealth, BodyWallet, BodyWalletShield};
use ambition_characters::equipment::WornEquipment;
use ambition_platformer2d_core::RoomGeometry;
use ambition_sfx::{SfxMessage, SfxWriter};

// `body_vulnerable` / `shield_blocks_hit` moved to `crate::combat::util`
// (E2): they are the shared victim-gate predicates every damage EMITTER
// reads — combat vocabulary, not victim-side application code.
pub use crate::combat::util::{body_vulnerable, shield_blocks_hit};

// `scaled_knockback` moved to `crate::combat::util` (E2): the CM1
// knockback-scaling LAW is combat model vocabulary.
pub use crate::combat::util::scaled_knockback;

// THE directional-influence law (CM2) moved to the floor with the rest of the
// hit-response kernel (`ambition_platformer2d_core::hit_response`, FB6b) so the
// fighter brain's shadow rollout applies the SAME rotation the victim path
// does. Re-exported here because this is where its consumers learned to name
// it.
pub use ae::hit_response::di_adjust;

/// The kernel-tuning row for one struck body: the boss/enemy feel SELECTION is
/// this crate's business, the response math is the floor's
/// (`ae::hit_response`). One constructor, so the velocity and hitstun calls
/// cannot pick different rows for the same hit.
/// ⚠ **`feel.di_max_angle` must already be the RESOLVED match value** (AE6).
/// Directional influence is a rule of the match being played, not world tuning,
/// so the system that reads `Res<Platformer2dFeelTuningMonolith>` folds the resolved rules
/// into its LOCAL copy before the hit path sees it — see `resolved_feel`. The
/// row travels as one struct so the launch and the hitstun cannot disagree.
pub(crate) fn hit_response_tuning(
    feel: &Platformer2dFeelTuningMonolith,
    boss_hit: bool,
) -> ae::hit_response::HitResponseTuning {
    ae::hit_response::HitResponseTuning {
        knockback_x: if boss_hit {
            feel.boss_knockback_x
        } else {
            feel.enemy_knockback_x
        },
        knockback_y: if boss_hit {
            feel.boss_knockback_y
        } else {
            feel.enemy_knockback_y
        },
        hitstun_time: if boss_hit {
            feel.boss_hitstun_time
        } else {
            feel.enemy_hitstun_time
        },
        di_max_angle: feel.di_max_angle,
    }
}

/// Per-body feel values for [`resolve_body_hit`] — how hard the hit reads on
/// THIS body (blink length, i-frame window), not whether it lands. The player
/// and actors pass different numbers; the rule is one.
#[derive(Clone, Copy, Debug)]
pub struct BodyHitFeel {
    /// Damage-blink armed on a damaging hit.
    pub hit_flash: f32,
    /// Post-hit i-frame window armed on a damaging hit.
    pub damage_invuln_time: f32,
    /// Damage-blink armed on a BLOCKED hit (0.0 leaves the flash untouched).
    pub block_hit_flash: f32,
    /// Guard i-frame on a blocked hit: the timer is raised to at least this.
    pub block_invuln_floor: f32,
    /// Hitstop when a worn ARMOR row absorbs the hit instead of HP taking it.
    ///
    /// ⚠ absorbing a hit is not a quiet event — losing a form is the most
    /// consequential thing short of dying — and this branch returns before the
    /// damaging path's beat, so without a number here it had none. Jon reported
    /// exactly that: *"there needs to be a bit of hitstun when maryo gets hit."*
    pub armor_hitstop_time: f32,
}

/// What [`resolve_body_hit`] decided about one hit on one body.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BodyHitResolution {
    /// The hit doesn't register at all: the body is inside its post-hit
    /// i-frame window, or already dead. No state was touched — the caller
    /// plays no feedback and records no landed hit.
    Ignored,
    /// The body's raised shield consumed the hit: no damage, but the hit DID
    /// register (guard i-frame armed; the caller plays block feedback).
    Blocked,
    /// A worn armor equipment row absorbed the hit (A3
    /// `OnHit::ConsumeAsArmor`): no HP damage, the row was spent (removed or
    /// downgraded), and the SAME brief i-frames a damaging hit arms are armed.
    /// The hit registered, but it never reaches HP or the death path.
    Armored,
    /// A wallet-backed shield absorbed the hit before HP/death resolution.
    /// `spent` is the entire positive balance consumed by the resolver.
    WalletShielded { spent: i32 },
    /// The hit landed. `damage` is the post-multiplier amount applied; `died`
    /// is whether it killed the body.
    Damaged { damage: i32, died: bool },
}

/// **What the shared resolver DECIDED about one hit**, announced so an observer
/// can explain it.
///
/// The resolution already exists — `Ignored` / `Blocked` / `Armored` /
/// `WalletShielded` / `Damaged` is exactly the "why was this hit accepted or
/// rejected" vocabulary — and it was reaching nobody but the caller. This is the
/// smallest way to let it out.
///
/// ⚠ **written only under the `causal` feature.** A build with no inspector
/// pays nothing: no message, no write, no registration. The alternative was
/// instrumenting the resolver itself, which would have put an observer's
/// concern inside the one function every body's damage goes through.
///
/// ⚠ **nothing in the simulation reads this.** It is a fact ABOUT a decision
/// already made, so a reader cannot change the decision — the same property
/// that makes the stock-lifecycle observer safe.
#[cfg(feature = "causal")]
// Not `Copy`: `HitSource` carries data. Cloned at the one write site, which
// happens once per landed hit rather than per frame.
#[derive(bevy::prelude::Message, Clone, Debug, PartialEq)]
pub struct BodyHitResolved {
    pub body: bevy::prelude::Entity,
    pub resolution: BodyHitResolution,
    pub source: crate::combat::HitSource,
    /// The raw damage the hit carried, BEFORE the multiplier and before any
    /// defence. Kept beside the outcome because "asked for 30 and dealt 0" and
    /// "asked for 0" are different findings.
    pub raw_damage: i32,
}

/// **The launch, announced.** Carries [`BodyReaction`]'s three inputs beside the
/// velocity they produced, because the velocity alone cannot say whether a short
/// launch was a weak hit or a well-DI'd one.
///
/// Feature-gated and `Option`-written, exactly like [`BodyHitResolved`]: read by
/// an instrument and nothing else, so a composition that never registers it must
/// publish nothing rather than panic.
#[cfg(feature = "causal")]
#[derive(bevy::prelude::Message, Clone, Copy, Debug, PartialEq)]
pub struct BodyReactionApplied {
    pub body: bevy::prelude::Entity,
    pub reaction: BodyReaction,
}

/// Deterministic victim-side fact emitted when a [`BodyWalletShield`] spends a
/// wallet balance. The generic resolver owns survival; game content owns how
/// the spent currency is presented (for example, as scattered rings).
#[derive(bevy::prelude::Message, Clone, Copy, Debug, PartialEq)]
pub struct WalletShieldSpent {
    pub victim: Entity,
    pub amount: i32,
    pub pos: ae::Vec2,
}

/// A wallet the resolver is permitted to spend as armor.
///
/// The capability is [`BodyWalletShield`] — owning currency is NOT the same fact
/// as currency that absorbs damage, and the difference must survive into the
/// resolver's signature. Passing a bare `&mut BodyWallet` would let a caller
/// forget the marker query and silently turn every wallet into a shield; the
/// constructor takes the marker by reference so the capability check cannot be
/// skipped, and the resolver never has to carry a field it does not read.
pub struct WalletArmor<'a>(&'a mut BodyWallet);

impl<'a> WalletArmor<'a> {
    /// Pair a wallet with the marker that makes it defensive. The marker is
    /// carried only to make the capability a precondition of construction.
    pub fn new(wallet: &'a mut BodyWallet, _shield: &BodyWalletShield) -> Self {
        Self(wallet)
    }

    /// Spend the whole positive balance, or `None` when there is nothing to
    /// spend and the hit must fall through to HP.
    fn spend_all(self) -> Option<i32> {
        (self.0.balance > 0).then(|| std::mem::replace(&mut self.0.balance, 0))
    }
}

/// THE one victim-side hit resolver every body shares (fable review 2026-07-02
/// §A2): consume-time i-frame gate → directional shield block → scaled damage →
/// death flag → hit-flash/i-frame arming. The player and actor consumers both
/// call this; what stays outside is genuine per-body POLICY (difficulty
/// multiplier choice, death→respawn vs death→drops, peaceful-actor barks,
/// knockback style — the last is step 6 of the §A2 plan).
///
/// MECHANICS contract:
/// - i-frames are consumed HERE, at consume time, for every body — no emitter
///   decides them (an emitter may still read `body_vulnerable` to early-out or
///   mute feedback, but the event must flow).
/// - `damage_multiplier` is a policy hook (player: difficulty × assist;
///   actors: 1.0). A landed hit always deals at least 1 damage.
/// - `never_dies` bodies (training dummies) take no health damage at all.
/// - `health: None` (headless test bodies) resolves as damaged-but-undying.
#[allow(clippy::too_many_arguments)]
pub fn resolve_body_hit(
    combat: &mut BodyCombat,
    mut health: Option<&mut BodyHealth>,
    armor: Option<&mut WornEquipment>,
    wallet_shield: Option<WalletArmor<'_>>,
    shield_active: bool,
    facing: f32,
    body_pos: ae::Vec2,
    impact_pos: ae::Vec2,
    gravity_dir: ae::Vec2,
    raw_damage: i32,
    damage_multiplier: f32,
    never_dies: bool,
    feel: BodyHitFeel,
    // Whether this hit is UNSTOPPABLE: no i-frame gate, no shield, no armor,
    // no wallet, and not even `never_dies`.
    //
    // Exactly one source is — the blast zone. Every defence above is a defence
    // against BEING HIT, and nothing hit this body: the world stopped. You
    // cannot be invulnerable to the edge of the world, you cannot block it,
    // and a training dummy that has left the stage is not training.
    //
    // It matters by the frame. The launch that throws a body off the stage is
    // the same event that arms its knockback i-frames — 0.2s on an actor, 0.75s
    // on the player — so gating the blast zone on vulnerability makes the case
    // it is FOR the case it handles worst: the victim crosses the line and
    // falls for up to 45 frames with nothing happening.
    //
    // The one gate it does NOT bypass is `!alive()`. The blast gate is a
    // position test that re-fires every tick, so a corpse outside the world
    // must stop being killed.
    unstoppable: bool,
) -> BodyHitResolution {
    if !unstoppable && !combat.vulnerable() {
        return BodyHitResolution::Ignored;
    }
    if let Some(health) = health.as_deref() {
        if !health.alive() {
            return BodyHitResolution::Ignored;
        }
    }
    if !unstoppable && shield_blocks_hit(shield_active, facing, body_pos, impact_pos, gravity_dir) {
        if feel.block_hit_flash > 0.0 {
            combat.hit_flash = feel.block_hit_flash;
        }
        combat.damage_invuln_timer = combat.damage_invuln_timer.max(feel.block_invuln_floor);
        return BodyHitResolution::Blocked;
    }
    // A3 armor-on-hit (shield beats armor beats damage): a worn armor row spends
    // itself BEFORE the hit reaches HP. The wearer takes zero HP damage and gets
    // the same brief i-frames a damaging hit arms. Generic — any body carrying a
    // `WornEquipment` with a `ConsumeAsArmor` row gets this; the player is the
    // only wirer today. `never_dies` bodies still spend armor (a downgrade is a
    // state change worth honoring), then reach the no-death path below anyway.
    if let Some(armor) = armor.filter(|_| !unstoppable) {
        if armor.consume_armor().is_some() {
            combat.hit_flash = feel.hit_flash;
            combat.damage_invuln_timer = feel.damage_invuln_time;
            // ⛔ **AND THE BEAT, which this branch used to skip.** Jon, from play:
            // *"there needs to be a bit of hitstun when maryo gets hit."* Spending
            // armor returns EARLY, so it never reached the hitstop the damaging
            // path arms below — and losing a form is the most consequential thing
            // that happens to her short of dying. She shrank mid-stride with no
            // beat, which reads as the hit not landing at all.
            //
            // ⚠ the hitstop only; NOT the recoil lock or the carried launch. Those
            // belong to being THROWN, and armor absorbs the throw — that is what
            // absorbing means. What is owed is the pause that says it happened.
            combat.hitstop_timer = combat.hitstop_timer.max(feel.armor_hitstop_time);
            return BodyHitResolution::Armored;
        }
    }
    // Wallet-backed armor is resolved before HP and death. A lethal hit against
    // a one-health body carrying currency therefore spends the balance rather
    // than entering the respawn path.
    if let Some(spent) = wallet_shield
        .filter(|_| !unstoppable)
        .and_then(WalletArmor::spend_all)
    {
        combat.hit_flash = feel.hit_flash;
        combat.damage_invuln_timer = feel.damage_invuln_time;
        // ⛔ **AND THE BEAT — the same one the armor branch above was fixed for,
        // and this branch went on skipping.** Jon, from play: *"When SANIC is hit
        // … he should also have some hitstun."* Losing the purse is the same
        // event as losing a form: the most consequential thing short of dying,
        // and it happened with no pause at all, which reads as the hit not
        // landing. The rings burst out of a body that never flinched.
        //
        // ⚠ the hitstop ONLY, exactly as for armor. The recoil lock and the
        // carried launch belong to being THROWN, and the caller already keeps
        // the physical reaction — a `HitMode::Knockback` still knocks the body
        // off its ledge. What is owed here is the pause that says it happened.
        combat.hitstop_timer = combat.hitstop_timer.max(feel.armor_hitstop_time);
        return BodyHitResolution::WalletShielded { spent };
    }
    combat.hit_flash = feel.hit_flash;
    combat.damage_invuln_timer = feel.damage_invuln_time;
    let damage = ((raw_damage as f32) * damage_multiplier).round() as i32;
    let damage = damage.max(1);
    let died = if never_dies && !unstoppable {
        false
    } else {
        health
            .as_deref_mut()
            .map(|health| health.damage(damage))
            .unwrap_or(false)
    };
    BodyHitResolution::Damaged { damage, died }
}

// ⛔ **`death_respawn_player` IS DELETED** (ADR 0033). It was the engine's death
// handler and it was a RESPAWN handler: teleport to `world.spawn`, refill mana,
// re-anchor safety, reset the sim clock, `anim.reset()`, `combat.reset()`,
// `health.reset()` to full, i-frames, a banner — and then, last, announce that a
// death had happened. Every ruleset that wanted a death to MEAN something had to
// claw the body back from that, which is what Mary-O's death beat was: six
// counter-measures, none of them about dying.
//
// The respawn is a CONSEQUENCE now. It happens through the one shared road
// (`reset_sandbox`, reached by `RoomReplayRequested`) when the game's authored
// `DeathRules` say so, or inside a ruleset that owns its own — never as a reflex
// in the frame the body died.

/// **The two things a death ANNOUNCES**, bundled because Bevy caps a system at
/// 16 parameters and the player-damage system was at the cap.
///
/// Grouping by meaning rather than to make a number fit: both are written at the
/// same moment for the same reason, and a future consumer of "this body died"
/// belongs in here beside them rather than as a 17th parameter.
#[derive(bevy::ecs::system::SystemParam)]
pub struct BodyDeathWriters<'w> {
    /// The world's own death record — position and attribution.
    pub died: MessageWriter<'w, ActorDiedMessage>,
    /// S4: a KO of a body whose death a RULESET owns, for the stocks loop. A
    /// separate message rather than a flag on `ActorDiedMessage` because the two
    /// do not fire together: an `Unbounded` fighter is knocked out without dying
    /// in the world's sense at all.
    pub knockouts: MessageWriter<'w, crate::combat::stocks::BodyKnockedOut>,
    /// The resolver's DECISION about each hit, for the causal inspector. Rides
    /// this bundle rather than a new parameter because it is written from the
    /// same two places, at the same moment, from the same value.
    ///
    /// ⚠ **`Option`, and that is the difference between this and
    /// `BodyKnockedOut` above.** A knockout is a message the SIMULATION acts on,
    /// so a composition that fails to register it is broken and should say so
    /// loudly. This one is read by an INSTRUMENT and by nothing else, so a
    /// composition that never registers it — a hand-built test fixture, a game
    /// with no inspector — should publish nothing rather than panic. An
    /// instrument that can take down a composition is worse than no instrument.
    #[cfg(feature = "causal")]
    pub resolutions: Option<MessageWriter<'w, BodyHitResolved>>,
    /// The LAUNCH the reaction produced. Same `Option` rule as above.
    #[cfg(feature = "causal")]
    pub reactions: Option<MessageWriter<'w, BodyReactionApplied>>,
}

/// Resolve this frame's hits against one body. Returns **true when the body was
/// Class-B remapped** (`collision-and-ccd.md` §3.2) — a death respawn or a
/// hazard safe-respawn both teleport it. Knockback does not: it writes velocity,
/// which is Class-A's business. The caller owns the entity id, so the caller
/// records into `ClassBRemapLog`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_player_damage_events(
    player_entity: Entity,
    // True when this body carries `RulesetOwnsDeath` — a match, not the world,
    // decides what a zero-health fighter means.
    ruleset_owns_death: bool,
    // A13: the attacker's bank and this body's own, resolved by the caller which
    // holds the queries.
    attacker_source: Option<&ambition_sfx::PresentationSourceId>,
    victim_source: Option<&ambition_sfx::PresentationSourceId>,
    // Whether the striker hits heavy, resolved by the caller which holds the
    // queries — the same shape `attacker_source` above already uses.
    heavy_attacker: bool,
    _world: &ae::World,
    sfx: &mut SfxWriter,
    vfx: &mut MessageWriter<VfxMessage>,
    debris: &mut MessageWriter<DebrisBurstMessage>,
    death_writers: &mut BodyDeathWriters<'_>,
    clusters: &mut ae::BodyClustersMut<'_>,
    _sim_state: &mut RoomTransitionCooldown,
    clock_resets: &mut MessageWriter<ClockResetRequest>,
    safety: &mut PlayerSafetyState,
    banner_requests: &mut MessageWriter<GameplayBannerRequested>,
    mut player_health: Option<&mut BodyHealth>,
    // A3: the player's worn equipment, so an armor row can absorb this hit before
    // it reaches HP. `None` for a player wearing nothing.
    armor: Option<&mut WornEquipment>,
    wallet_shield: Option<WalletArmor<'_>>,
    wallet_shield_spent: &mut MessageWriter<WalletShieldSpent>,
    damage_events: &[FeatureHitEvent],
    tuning: ae::MovementTuning,
    // The body's frame down direction, resolved by the environment.
    gravity_dir: ae::Vec2,
    feel: Platformer2dFeelTuningMonolith,
    difficulty_multiplier: f32,
    // The controlled body's held locomotion (local frame) for DI (CM2).
    di_input_local: ae::Vec2,
    _anim: &mut BodyAnimFacts,
    combat: &mut BodyCombat,
    motion_model: &mut ae::MotionModel,
    // The published maneuver facts (ADR 0024): dodge-roll i-frames gate at
    // consume time from the semantic projection, not the policy's timer.
    facts: &ae::BodyMotionFacts,
) -> bool {
    let Some(damage) = damage_events.first().cloned() else {
        return false;
    };
    // Consume-time vulnerability (§A2): any held invulnerability REASON,
    // dodge-roll i-frames, and an active parry drop the event before any state
    // mutates. The post-hit i-frame window is consumed inside the resolver —
    // the SAME rule for every body; emitters no longer decide it.
    if !body_vulnerable(
        player_health
            .as_deref()
            .map(|health| health.health.invulnerable)
            .unwrap_or_default(),
        facts.dodge_rolling,
        clusters.shield,
        combat,
    ) {
        return false;
    }
    let impact_pos = damage
        .knockback
        .as_ref()
        .map(|k| k.impact_pos)
        .unwrap_or_else(|| damage.volume.center());
    // THE shared mechanics: shield block (the body's RESOLVED guard —
    // `resolve_shield` is ability-gated and dash-blocked, invariant I3),
    // difficulty-scaled damage, death flag, hit-flash + i-frame arming.
    // Difficulty is player POLICY: easy halves incoming damage, hard doubles
    // it, plus the fine-grained gameplay multiplier and assist factor.
    let resolution = resolve_body_hit(
        combat,
        player_health.as_deref_mut(),
        armor,
        wallet_shield,
        clusters.shield.active,
        clusters.kinematics.facing,
        clusters.kinematics.pos,
        impact_pos,
        gravity_dir,
        damage.damage,
        difficulty_multiplier,
        false,
        BodyHitFeel {
            hit_flash: 0.20,
            damage_invuln_time: feel.knockback_invulnerability_time,
            block_hit_flash: 0.0,
            block_invuln_floor: 0.12,
            armor_hitstop_time: 0.070,
        },
        // The player's knockback i-frames are 0.75s — the longest window in the
        // game, armed by the very launch that throws them off the stage.
        matches!(damage.source, crate::combat::HitSource::LeftTheWorld),
    );
    // The resolver's decision, announced for the inspector. Gated: a build with
    // no `causal` feature writes nothing here.
    #[cfg(feature = "causal")]
    if let Some(resolutions) = death_writers.resolutions.as_mut() {
        resolutions.write(BodyHitResolved {
            body: player_entity,
            resolution,
            source: damage.source.clone(),
            raw_damage: damage.damage,
        });
    }
    // **THE KO, announced.** (S4) Two things count as one for a body whose death
    // a ruleset owns: the meter emptying its pool, and the world throwing it out.
    // An `Unbounded` fighter only ever reaches the second — its pool is full at
    // the moment it is knocked off the stage — so a signal derived from health
    // alone would never fire for exactly the bodies stocks exist for.
    if ruleset_owns_death {
        let knocked_out = match resolution {
            BodyHitResolution::Damaged { died, .. } => {
                died || matches!(damage.source, crate::combat::HitSource::LeftTheWorld)
            }
            // An i-framed or already-refused hit is not a KO. `LeftTheWorld` is
            // passed as unstoppable above, so a ring-out cannot land here.
            _ => false,
        };
        if knocked_out {
            death_writers
                .knockouts
                .write(crate::combat::stocks::BodyKnockedOut {
                    body: player_entity,
                    cause: damage.source.clone(),
                });
        }
    }
    match resolution {
        BodyHitResolution::Ignored => false,
        BodyHitResolution::Blocked => {
            // The guard is this body's (G1).
            sfx.write_for_body(
                victim_source,
                SfxMessage::Play {
                    id: ambition_sfx::ids::WORLD_ROCK_HIT,
                    pos: clusters.kinematics.pos,
                },
            );
            banner_requests.write(GameplayBannerRequested::new("blocked", 1.0));
            false
        }
        // A3: a worn armor row (mushroom-analog) absorbed the hit. No HP change,
        // no respawn/teleport (so no Class-B remap), just the spent powerup and
        // the brief i-frames the resolver already armed.
        BodyHitResolution::Armored => {
            // The armor was WORN by this body, so losing it is its cue (G1).
            sfx.write_for_body(
                victim_source,
                SfxMessage::Play {
                    id: ambition_sfx::ids::PLAYER_DAMAGE,
                    pos: impact_pos,
                },
            );
            banner_requests.write(GameplayBannerRequested::new("POWERUP LOST", 1.4));
            false
        }
        BodyHitResolution::WalletShielded { spent } => {
            wallet_shield_spent.write(WalletShieldSpent {
                victim: player_entity,
                amount: spent,
                pos: clusters.kinematics.pos,
            });
            // The shield prevents HP loss and death, not the ordinary physical
            // reaction. This preserves hazard reset and knockback feel without
            // making positive currency a no-recoil invincibility exploit.
            match damage.mode {
                crate::combat::HitMode::SafeRespawn => {
                    safe_respawn_player(
                        sfx,
                        victim_source,
                        vfx,
                        clusters,
                        clock_resets,
                        safety,
                        combat,
                        tuning,
                        feel,
                        impact_pos,
                        motion_model,
                    );
                    true
                }
                crate::combat::HitMode::Knockback => {
                    ae::movement::knock_off_ledge(motion_model, clusters.ledge);
                    let reaction = apply_player_knockback(
                        sfx,
                        vfx,
                        debris,
                        clusters,
                        combat,
                        tuning,
                        gravity_dir,
                        feel,
                        &damage,
                        di_input_local,
                        attacker_source,
                        victim_source,
                        heavy_attacker,
                    );
                    publish_reaction(death_writers, player_entity, reaction);
                    false
                }
            }
        }
        BodyHitResolution::Damaged { died: true, .. } => {
            // ⭐ **ONE DEATH ARM** (ADR 0033). This used to be two: a
            // `RulesetOwnsDeath` arm that voiced the death and left the body
            // alone, and an exploration arm that teleported the body to spawn,
            // full-healed it, printed *"PLAYER DOWN: respawned at room start
            // with full HP"*, and only THEN announced the death. The second was
            // an undo racing everything that wanted to react — and it was
            // unreachable for a match, which is how seat 0 came to be unable to
            // lose (GPT 5.6, 2026-07-27).
            //
            // Both arms want the same thing, and the ruleset arm was already
            // it: the body DIES OUT LOUD and stays where it is. Health stays at
            // zero for whoever owns the consequence to see. Where the body goes
            // next, what it costs, and who scores it are all authored — a level
            // reset, a stock, a bubble — and none of that is the sound of
            // losing.
            //
            // `ruleset_owns_death` no longer selects an arm. It is still read to
            // decide whether the world's OTHER consequences apply (the enemy
            // death economy), which is the question it actually answers.
            sfx.write_for_body(
                victim_source,
                SfxMessage::Death {
                    pos: clusters.kinematics.pos,
                },
            );
            // Attribution for the death fact: the killing hit's source category
            // plus its attacker entity when the source carries one.
            death_writers.died.write(ActorDiedMessage {
                victim: player_entity,
                pos: impact_pos,
                cause: crate::DeathCause {
                    source: damage.source.clone(),
                    attacker: damage.attacker,
                },
            });
            // The caller's `bool` is "did this reaction relocate the body". It
            // never does now — nothing here moves anything.
            false
        }
        BodyHitResolution::Damaged { died: false, .. } => match damage.mode {
            crate::combat::HitMode::SafeRespawn => {
                // CM8: a safe-respawn is a "reset" reaction, not a hurt, so it
                // keeps its own Reset cue — but the striking attack's sound
                // (e.g. the spike) still plays, since the body did touch it.
                if let Some(id) = damage.strike_sfx {
                    // The ATTACK's sound, so the attacker's source — the same
                    // split `emit_hit_feedback` makes (A13).
                    sfx.write_for_body(
                        attacker_source,
                        SfxMessage::Play {
                            id,
                            pos: impact_pos,
                        },
                    );
                }
                safe_respawn_player(
                    sfx,
                    victim_source,
                    vfx,
                    clusters,
                    clock_resets,
                    safety,
                    combat,
                    tuning,
                    feel,
                    impact_pos,
                    motion_model,
                );
                true
            }
            crate::combat::HitMode::Knockback => {
                // Getting hit knocks you off a ledge grab — you fall with the
                // knockback instead of hanging there immune. The hang state is
                // policy-private, so the typed op goes through the model.
                ae::movement::knock_off_ledge(motion_model, clusters.ledge);
                let reaction = apply_player_knockback(
                    sfx,
                    vfx,
                    debris,
                    clusters,
                    combat,
                    tuning,
                    gravity_dir,
                    feel,
                    &damage,
                    di_input_local,
                    attacker_source,
                    victim_source,
                    heavy_attacker,
                );
                publish_reaction(death_writers, player_entity, reaction);
                false
            }
        },
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn safe_respawn_player(
    sfx: &mut SfxWriter,
    // G1: the reset chime is this body's, for the same reason its death is.
    victim_source: Option<&ambition_sfx::PresentationSourceId>,
    vfx: &mut MessageWriter<VfxMessage>,
    clusters: &mut ae::BodyClustersMut<'_>,
    clock_resets: &mut MessageWriter<ClockResetRequest>,
    safety: &PlayerSafetyState,
    combat: &mut BodyCombat,
    tuning: ae::MovementTuning,
    feel: Platformer2dFeelTuningMonolith,
    from: ae::Vec2,
    motion_model: &mut ae::MotionModel,
) {
    let to = safety.last_safe_pos;
    // The follow-up `refresh_movement_resources_clusters` that used to sit here is
    // GONE: the reset takes the live air-jump count now and already restores dash
    // charges from the same `BodyAbilities`, so the second call restated its
    // answer. Four of five call sites performed that ritual and one did not,
    // which is the whole argument for folding it in.
    ae::reset_body_clusters(motion_model, clusters, to, tuning.air_jumps);
    combat.damage_invuln_timer = feel.hazard_respawn_invulnerability_time;
    combat.hitstun_timer = 0.0;
    combat.recoil_lock_timer = 0.0;
    combat.hitstop_timer = 0.0;
    combat.hit_flash = feel.reset_flash_time;
    clock_resets.write(ClockResetRequest::sim_clock(
        ClockRequester::Engine,
        "safe_respawn",
    ));
    sfx.write_for_body(victim_source, SfxMessage::Reset { pos: to });
    vfx.write(VfxMessage::ResetEffects { from, to });
}

/// THE frame-agnostic knockback velocity for ANY struck body (§A2 step 6):
/// side away from the hit's source (falling back to the stored event dir, then
/// away from facing), launched with a rise against the body's gravity.
///
/// `FeelScale` magnitudes preserve the standard per-source feel vector used by
/// contact damage, hazards, and projectiles. `LaunchSpeed` magnitudes preserve
/// the absolute engine-unit speed authored by melee move volumes. The latter is
/// deliberately NOT multiplied by the feel vector: doing so was the goblin-hit
/// teleport regression (`120 px/s` was interpreted as a `120x` multiplier).
/// THE frame-agnostic knockback velocity for ANY struck body (§A2 step 6).
/// The math is the floor's (`ae::hit_response::knockback_velocity` — FB6b, so
/// the shadow rollout predicts with the same formula); this wrapper owns only
/// the boss/enemy feel selection, which is a fact about THIS game's tuning
/// rows and not about knockback.
/// ⚠ **TEST-ONLY, and that is the finding.** The 07-30 handoff flagged this and
/// `knockback_reaction_scale` as two `dead_code` warnings that might be orphans
/// of the saturating-meter problem (S4). They are not: their production callers
/// moved into `ae::hit_response` with the kernel carve (FB6b), and what is left
/// is the historical local NAME that this module's tests read against. Scoped to
/// `test` rather than deleted, because a test asserting the wrapper's feel
/// selection is asserting something the kernel's own tests do not.
#[cfg(test)]
pub(crate) fn resolved_body_knockback_velocity(
    victim_pos: ae::Vec2,
    victim_facing: f32,
    gravity_dir: ae::Vec2,
    boss_hit: bool,
    knockback: Option<&crate::combat::HitKnockback>,
    di_input_local: ae::Vec2,
    feel: Platformer2dFeelTuningMonolith,
) -> ae::Vec2 {
    ae::hit_response::knockback_velocity(
        victim_pos,
        victim_facing,
        gravity_dir,
        knockback,
        di_input_local,
        &hit_response_tuning(&feel, boss_hit),
    )
}

/// See [`ae::hit_response::reaction_scale`] — kept under its historical local
/// name for this module's tests. Test-only for the same reason as
/// [`resolved_body_knockback_velocity`] above.
#[cfg(test)]
fn knockback_reaction_scale(knockback: Option<&crate::combat::HitKnockback>) -> f32 {
    ae::hit_response::reaction_scale(knockback)
}

/// The ONE post-hit launch + stagger arming for ANY struck body (§A2 steps
/// 6–7), called by the player's knockback path and the actor damage consumer:
/// SET the resolved knockback velocity, then arm hitstun (feel-scaled and
/// source-selected), the fixed recoil throw, and the hitstop beat. hit_flash +
/// damage_invuln_timer are armed by `resolve_body_hit` before this runs —
/// this owns only the launch + control-lock timers.
/// **What the reaction DECIDED**, so a caller can explain it.
///
/// The magnitude and direction of a launch are the product of the authored
/// knockback, the victim's own DI, and the feel tuning — three inputs whose
/// individual contributions are invisible in the resulting velocity. "Why did
/// knockback have this magnitude and direction" is the inspector's question,
/// and it cannot be answered from the velocity alone.
///
/// Returned rather than published from in here: this function takes a `&mut
/// Vec2` and no writer, and threading one in would put an observer's parameter
/// through the reaction every body shares. The caller has the writer.
#[cfg(feature = "causal")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyReaction {
    /// The velocity the launch wrote.
    pub velocity: ae::Vec2,
    /// The DI the victim held, in its local frame. `ZERO` means it did not
    /// steer — which is a different finding from steering and being overruled.
    pub di_input_local: ae::Vec2,
    pub hitstun: f32,
    /// `false` when the hit carried no knockback at all: the body was hurt and
    /// not launched, and a reader looking for a launch should stop here.
    pub had_knockback: bool,
}

pub(crate) fn apply_body_hit_reaction(
    vel: &mut ae::Vec2,
    // **The carried-momentum channel.** The launch is momentum the WORLD
    // imparted, and the hands-off air stop assist is required not to bleed that
    // away — `carried_run`'s own doc names knockback as one of the two cases it
    // exists for. Only the portal adapter ever wrote it, so a launched body's
    // floor was zero and `AIR_STOP_ASSIST` erased a 420px/s smash in seven
    // frames: 15.5px of travel out of the ~210px the launch describes
    // (measured, queue F0e).
    flight: &mut ae::BodyFlightState,
    combat: &mut BodyCombat,
    body_pos: ae::Vec2,
    body_facing: f32,
    gravity_dir: ae::Vec2,
    boss_hit: bool,
    knockback: Option<&crate::combat::HitKnockback>,
    // The struck body's held control (local frame) for DI (CM2). `ZERO` = none.
    di_input_local: ae::Vec2,
    feel: Platformer2dFeelTuningMonolith,
) -> BodyReactionOutcome {
    // ONE tuning row for the whole reaction, so the launch and the hitstun
    // cannot disagree about which feel numbers this hit uses (FB6b).
    let response = hit_response_tuning(&feel, boss_hit);
    let launch = ae::hit_response::knockback_velocity(
        body_pos,
        body_facing,
        gravity_dir,
        knockback,
        di_input_local,
        &response,
    );
    *vel = launch;
    // ⭐ **and PUBLISH it, because the write above is not authoritative for every
    // body.** `BodyKinematics::vel` is the authority for an axis-swept body and a
    // MIRROR for a riding surface-momentum one, whose velocity is derived from
    // the scalar `v_t` along its tangent and republished on the next step. Sanic
    // rides. So every knockback he took was applied faithfully to a field nothing
    // read, with hitstun 0.24s and knockback 360/260 all non-zero and no reaction
    // visible — reported as "no knockback", diagnosed for a long time as feel.
    //
    // `step_motion` drains this and hands it to the model, which is the only
    // thing that knows whether a launch means *leave the surface* or *override
    // the run*. Written here rather than applied here for the same reason: this
    // function has a `&mut Vec2` and no world and no `MotionModel`.
    flight.pending_launch = launch;
    combat.hitstun_timer = ae::hit_response::hitstun_duration(knockback, &response);
    // Brief hard control-lock at the front of the hitstun window: the body is
    // thrown with no authority, then regains the attack verb the instant it
    // clears (while still in hitstun + i-frames). Fixed-length — the recoil is a
    // readable beat, not something that scales with how hard the hit was.
    combat.recoil_lock_timer = feel.knockback_recoil_lock_time;
    combat.hitstop_timer = feel.player_damage_hitstop_time;
    // CARRY THE LAUNCH, for exactly as long as the body cannot answer for it.
    //
    // The floor is the run-axis component of the velocity just written, in the
    // same frame the launch was resolved in. The window is HITSTUN, not the
    // recoil lock: the recoil lock is the short hard beat at the front, while
    // hitstun is the whole span in which the body has no authority over its own
    // trajectory — and "momentum you were given while you could not act" is a
    // statement about authority. Deliberately an existing number rather than a
    // new tuning knob; the carry is owed for exactly as long as the reaction it
    // belongs to.
    let side = ae::AccelerationFrame::new(gravity_dir).side;
    flight.carried_run = vel.dot(side);
    flight.carried_hold = combat.hitstun_timer;

    #[cfg(feature = "causal")]
    return BodyReaction {
        velocity: *vel,
        di_input_local,
        hitstun: combat.hitstun_timer,
        had_knockback: knockback.is_some(),
    };
    #[cfg(not(feature = "causal"))]
    {
        let _ = di_input_local;
    }
}

/// What [`apply_body_hit_reaction`] returns. A unit under no instrument, so a
/// build without `causal` pays nothing for a value nobody reads.
#[cfg(feature = "causal")]
pub(crate) type BodyReactionOutcome = BodyReaction;
#[cfg(not(feature = "causal"))]
pub(crate) type BodyReactionOutcome = ();

/// Announce a player-side launch, if anybody is listening.
///
/// A free function rather than repeated at both call sites, because the two
/// knockback arms are already near-identical and a third near-identical block
/// is how they drift.
///
/// ⚠ **it exists in BOTH feature configurations** — a no-op without `causal` —
/// so the call sites carry no `cfg` and the returned reaction is never an
/// unused binding. Gating the function instead meant two conditional blocks at
/// the call sites and a warning in the build that has no inspector, which is
/// exactly the kind of noise that gets a warning ignored.
#[cfg(feature = "causal")]
fn publish_reaction(
    writers: &mut BodyDeathWriters<'_>,
    body: bevy::prelude::Entity,
    reaction: BodyReaction,
) {
    if let Some(reactions) = writers.reactions.as_mut() {
        reactions.write(BodyReactionApplied { body, reaction });
    }
}

#[cfg(not(feature = "causal"))]
fn publish_reaction(
    _writers: &mut BodyDeathWriters<'_>,
    _body: bevy::prelude::Entity,
    _reaction: BodyReactionOutcome,
) {
}

pub(crate) fn apply_player_knockback(
    sfx: &mut SfxWriter,
    vfx: &mut MessageWriter<VfxMessage>,
    debris: &mut MessageWriter<DebrisBurstMessage>,
    clusters: &mut ae::BodyClustersMut<'_>,
    combat: &mut BodyCombat,
    tuning: ae::MovementTuning,
    // The body's frame down direction, resolved by the environment.
    gravity_dir: ae::Vec2,
    feel: Platformer2dFeelTuningMonolith,
    damage: &FeatureHitEvent,
    // The controlled body's held locomotion (local frame) for DI (CM2).
    di_input_local: ae::Vec2,
    // A13: the two banks this hit's sound could belong to. The attacker's if the
    // strike authored a cue, the struck body's otherwise.
    attacker_source: Option<&ambition_sfx::PresentationSourceId>,
    victim_source: Option<&ambition_sfx::PresentationSourceId>,
    // **Does this hit come off a HEAVY attacker?** Asked of the attacker entity
    // by the system that holds the queries, not pattern-matched out of the cause
    // vocabulary here — see the twin on `apply_actor_hit`.
    heavy_attacker: bool,
) -> BodyReactionOutcome {
    let boss_hit = heavy_attacker;
    let knockback = damage.knockback.as_ref();
    let impact_pos = knockback
        .map(|k| k.impact_pos)
        .unwrap_or_else(|| damage.volume.center());
    let pos = clusters.kinematics.pos;
    let facing = clusters.kinematics.facing;
    let reaction = apply_body_hit_reaction(
        &mut clusters.kinematics.vel,
        &mut clusters.flight,
        combat,
        pos,
        facing,
        gravity_dir,
        boss_hit,
        knockback,
        di_input_local,
        feel,
    );
    ae::refresh_movement_resources_clusters(
        clusters.abilities,
        &mut *clusters.dash,
        &mut *clusters.jump,
        tuning.air_jumps,
    );
    // CM8: THE one victim-side reaction. The player carries the rich hurt
    // profile (its red "you got hurt" burst + debris) that used to be emitted
    // attacker-side keyed on `is_player`; the striking attack's `strike_sfx`
    // overrides the sound so an enemy sword and a hazard are heard apart.
    crate::combat::util::emit_hit_feedback(
        sfx,
        vfx,
        debris,
        ambition_vfx::HurtFeedback::PLAYER,
        damage.strike_sfx,
        damage.damage,
        impact_pos,
        attacker_source,
        victim_source,
    );
    reaction
}

/// Resolve this tick's victim-side `HitEvent`s and remember the last safe-spawn
/// position.
///
/// Reads `MessageReader<HitEvent>` and filters to victim-side sources (hazard /
/// enemy / boss); attacker-side hits (player slash, player projectile, pogo) are
/// consumed by `apply_feature_hit_events` separately. Routes the first event
/// through `handle_player_damage_events` — which can knock back, hitstun,
/// hazard-respawn, or fully kill the player — and writes resulting sfx / vfx /
/// died messages directly to their `MessageWriter`s. Then runs
/// `remember_safe_player_position` to update `safety.last_safe_pos` when the
/// player wasn't damaged this frame, isn't blinking, isn't in hitstun, and isn't
/// mid-room-transition.
#[allow(clippy::too_many_arguments)]
/// The controlled body's INCOMING damage policy: exactly what
/// [`resolve_body_hit`]'s contract states — difficulty × assist.
///
/// Deliberately NOT `player_damage_multiplier`: that setting is the OUTGOING
/// scale ("the firer's outgoing-damage scaling",
/// `ambition_projectiles::kind::ProjectileKind::spec`), and folding it in here
/// turned the power slider into a self-punishment knob — raising your damage
/// also raised the damage you took.
pub fn incoming_player_damage_multiplier(
    gameplay: &ambition_persistence::settings::GameplaySettings,
) -> f32 {
    let assist_factor = match gameplay.assist {
        ambition_persistence::settings::AssistMode::Off => 1.0,
        ambition_persistence::settings::AssistMode::On => 0.5,
    };
    gameplay.difficulty.damage_taken_multiplier() * assist_factor
}

/// Publish the authoritative death fact for a body the MOVEMENT KERNEL reset.
///
/// A pit fall, a drown, or a tile-grid `HazardBlock` never reaches
/// [`resolve_body_hit`]: the kernel flags `FrameEvents::reset`,
/// `integrate_home_body` teleports the body to spawn, and **no health is ever
/// touched** — `hazard_runtime` says so outright ("tile-grid hazards run through
/// the engine's reset-to-spawn path and never reach `HazardRuntime`"). So the
/// most common death in a platformer emitted no death signal at all, and the one
/// consumer that wanted it — Mary-O's lives — had to infer death from
/// `BodyLifetime.resets` instead.
///
/// That inference is the bug this exists to remove. Six unrelated callers bump
/// `resets`: two real deaths, a room load, an avatar rebuild, a sandbox reset,
/// and **a room replay's own reset**. Mary-O read the replay's bump as a fresh
/// death, spent another life, and requested another replay — an unbounded loop
/// that drained the whole lives counter many times a second in the hosted app.
/// A counter cannot carry a reason; a message can.
///
/// Scoped to the primary player because "died" here means "the local player's
/// attempt ended". An actor's own hazard reaction is its business — it never
/// teleports to the player spawn, so it never sets this flag.
pub fn publish_kernel_reset_death(
    mut died: MessageWriter<ActorDiedMessage>,
    bodies: Query<(Entity, &crate::avatar::PlayerBodyFrameOutput), PrimaryPlayerOnly>,
) {
    for (victim, frame_out) in &bodies {
        let Some(reset) = frame_out.reset else {
            continue;
        };
        died.write(ActorDiedMessage {
            victim,
            pos: reset.origin,
            // The kernel gate now says WHICH world killed her, so this reports
            // it instead of apologizing for not knowing. No entity claims any
            // of these kills — `attacker` stays `None` for all of them.
            cause: crate::DeathCause {
                source: death_source_of(reset.cause),
                attacker: None,
            },
        });
    }
}

/// The killing category a kernel reset belongs to.
///
/// A voluntary reset is still a death: the run ended, the lives counter should
/// spend one, and a player who presses the restart verb in a platformer expects
/// the attempt to be over. It is charged to `Hazard` — the same anonymous
/// world-killed-you category the spikes use — because no vocabulary exists for
/// "you asked", and inventing one would only be honest if something read it.
fn death_source_of(cause: ae::ResetCause) -> crate::combat::HitSource {
    match cause {
        ae::ResetCause::LeftTheWorld => crate::combat::HitSource::LeftTheWorld,
        ae::ResetCause::Hazard | ae::ResetCause::Drowned | ae::ResetCause::Requested => {
            crate::combat::HitSource::Hazard
        }
    }
}

/// Stage this frame's hits that belong to the controlled-body victim resolver
/// into its rollback-registered FIFO.
///
/// A `HitTarget::Body(entity)` is already victim-resolved and wins over the
/// legacy source-direction partition. That is what lets body-owned melee stay
/// controller-independent: a `PlayerSlash` produced by one fighter can target
/// another human-controlled body without being thrown away as an attacker-side
/// broadcast. Untargeted/broadcast events keep the old source filter until that
/// separate event-vocabulary cleanup lands.
///
/// ⭐ **whether a resolved victim belongs to THIS resolver is asked of the
/// world, not read off the event.** The target used to be stamped
/// `HitTarget::Player(e)` by whichever producer resolved it, and this system
/// trusted the stamp. It is one variant now, so the question is answered where
/// the answer actually lives: is that entity in the human-controlled
/// population? ⛔ without the check the FIFO would stage every body's hit —
/// including hits the actor consumer owns — into rollback-registered state that
/// then has to be snapshotted and checksummed every frame.
///
/// Runs at the END of the Combat phase, after every `HitEvent` writer, so the
/// hand-off from message channel to snapshot state happens same-frame; only
/// the FIFO — restored exactly by GGRS — crosses the frame boundary.
pub fn stage_player_victim_hit_events(
    mut hit_events: MessageReader<FeatureHitEvent>,
    mut pending: ResMut<crate::combat::events::PendingPlayerHitEvents>,
    controlled_bodies: Query<
        (),
        bevy::prelude::With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
    >,
) {
    for event in hit_events.read() {
        let mine = match event.target {
            HitTarget::Body(victim) => controlled_bodies.contains(victim),
            _ => !event.source.seeks_victims(),
        };
        if mine {
            pending.0.push(event.clone());
        }
    }
}

/// Void the staged victim-hit FIFO when this frame crossed a room-lifecycle
/// boundary, so a hit staged by the OLD population can never land on the
/// player in the NEW one.
///
/// The window is real: the FIFO's drain (`apply_player_hit_events`) is gated
/// on `gameplay_allowed`, and every boundary suspends gameplay first — so a
/// hit staged the frame before a transition sits in the queue through the
/// whole covered load and would land on the first resumed frame, thrown by an
/// enemy that no longer exists in a room the player already left.
///
/// Two messages cover every boundary: [`ResetRoomFeaturesEvent`] is the
/// same-room reset (player reset input, room replay), and
/// [`crate::rooms::RoomLoaded`] is the staging fact fired whenever a room's
/// contents finish (re)spawning — transitions, session resets, and snapshot
/// restores. Runs after `ResetProcessing` (the last boundary processor in the
/// frame), before the next frame's drain.
pub fn void_pending_player_hits_at_lifecycle_boundaries(
    mut room_resets: MessageReader<crate::combat::ResetRoomFeaturesEvent>,
    mut rooms_loaded: MessageReader<crate::rooms::RoomLoaded>,
    mut pending: ResMut<crate::combat::events::PendingPlayerHitEvents>,
) {
    let crossed = room_resets.read().count() > 0 || rooms_loaded.read().count() > 0;
    if crossed && !pending.0.is_empty() {
        pending.0.clear();
    }
}

/// **The set `apply_player_hit_events` runs in.**
///
/// ⛔ four GAME call sites order against this function by name — one in
/// `ambition_demo_mary_o`, three in `ambition_demo_sanic`, all reaching through
/// the facade for an engine leaf (`actors::features::ecs::damage_apply::…`).
/// That path is itself the tell: a consumer that has to spell four module levels
/// to place its own system is depending on engine internals, which is what the
/// facade exists to prevent.
///
/// ⚠ ONE member, for the same reason as
/// [`crate::world::overlay::FeatureWorldOverlaySet`]: the system beside it in the
/// tuple (`publish_kernel_reset_death`) is deliberately NOT gated on
/// `gameplay_allowed` while this one is, so a set spanning both would hand
/// consumers an ordering against a system with different run conditions. One
/// member keeps the swap exactly equivalent.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlayerHitResolutionSet;

pub fn apply_player_hit_events(
    // Bundled into one tuple param to stay under Bevy's 16-system-param ceiling
    // (S3e's relational `relations` + `attacker_factions` pushed this to 17).
    // `class_b` is the §3.2 transit ledger — death and hazard respawn are both
    // Class-B remaps, and this system is where the victim's entity id is known.
    // CM8: `debris_writer` is bundled here rather than added as a top-level
    // param — this system already sits at Bevy's 16-param ceiling (hence the
    // tuple). It carries the player's hurt-debris puff into the ONE victim-side
    // reaction, so the player keeps the impact debris that used to fire
    // attacker-side.
    (
        world,
        moving_platforms,
        mut class_b,
        mut debris_writer,
        mut wallet_shield_spent,
        body_sources,
        heavy_attackers,
    ): (
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<RoomGeometry>,
        Res<MovingPlatformSet>,
        Option<ResMut<ambition_platformer2d_shared_tangle::class_b::ClassBRemapLog>>,
        MessageWriter<DebrisBurstMessage>,
        MessageWriter<WalletShieldSpent>,
        // A13: whose cues each body emits. Bundled here for the same reason the
        // writers are — this system is at Bevy's param ceiling.
        Query<&ambition_sfx::BodyPresentationSource>,
        // **Which bodies hit HEAVY.** Filter-only, so it reads no components and
        // conflicts with nothing; bundled for the same ceiling reason.
        Query<(), bevy::prelude::With<super::boss_clusters::BossConfig>>,
    ),
    active_tuning: Res<ae::ActiveMovementTuning>,
    feel_tuning: Res<Platformer2dFeelTuningMonolith>,
    user_settings: Res<ambition_persistence::settings::UserSettings>,
    feature_ecs_overlay: Res<crate::world::overlay::FeatureEcsWorldOverlay>,
    mut sim_state: ResMut<RoomTransitionCooldown>,
    mut clock_resets: MessageWriter<ClockResetRequest>,
    mut banner_requests: MessageWriter<GameplayBannerRequested>,
    // The rollback-registered FIFO `stage_player_victim_hit_events` filled at
    // the END of the previous frame's Combat phase — NOT a `MessageReader`:
    // this system runs before Combat, so the victim stream is consumed one
    // frame after it is produced, and cross-frame combat truth must be
    // snapshot state (see `PendingPlayerHitEvents`).
    mut pending_hits: ResMut<crate::combat::events::PendingPlayerHitEvents>,
    mut death_writers: BodyDeathWriters,
    mut sfx_writer: SfxWriter,
    mut vfx_writer: MessageWriter<VfxMessage>,
    // SLOT-0 BY DESIGN: the safe-position memory this feeds is slot 0's respawn
    // point. Damage ROUTING itself is body-generic (it runs off factions and the
    // grudge); only "where does the local player wake up" is primary-scoped.
    primary_q: Query<Entity, crate::actor::PrimaryPlayerOnly>,
    // Friendly-fire policy (the DAMAGE side) + a faction lookup for the hit's
    // attacker: damage is physical, so any DIFFERENT-faction attacker's hit lands
    // on the player — including a duel's stray that the observer walked into. Only
    // a same-faction attacker (co-op ally) is spared, unless friendly fire is on.
    // Whether an actor AIMS at the player is the separate targeting concern.
    // AE6: the rules THIS MATCH plays under, resolved from its declaration
    // folded over the world's baseline. Never `FriendlyFire` directly — a
    // reader that consults the baseline is how a stage's rules and the
    // world's rules got to disagree.
    combat_rules: Option<Res<crate::combat::rules::ResolvedCombatTuning>>,
    attacker_factions: Query<&crate::combat::components::ActorFaction>,
    mut player_q: Query<
        (
            Entity,
            ae::BodyClusterQueryData,
            Option<&mut BodyHealth>,
            // A3: the player's worn equipment (mushroom/flower). `Option` so a
            // player wearing nothing still resolves — the common case.
            Option<&mut WornEquipment>,
            // Optional generic wallet-backed armor. The marker determines
            // whether a positive wallet balance is defensive currency.
            Option<&mut BodyWallet>,
            Option<&BodyWalletShield>,
            &mut BodyAnimFacts,
            &mut BodyCombat,
            &mut PlayerSafetyState,
            // Does a RULESET own this body's death? A match fighter's zero
            // health is the match's business; the exploration respawn would
            // teleport and full-heal it before any rules layer could look.
            bevy::prelude::Has<crate::combat::components::RulesetOwnsDeath>,
            // The controlled body's held input, for directional influence (CM2).
            // `Option` so a headless player with no brain still resolves (→ ZERO,
            // no DI). Inert unless `feel.di_max_angle` is authored nonzero.
            Option<&ambition_characters::brain::ActorControl>,
            // The victim's per-tick resolved frame (shield side + knockback
            // launch are frame-relative facts of the VICTIM's body).
            &crate::physics::ResolvedMotionFrame,
            // The body's movement policy: a death/safe respawn is a discrete
            // TRANSIT and must reconcile model-private attachment.
            &mut crate::features::MotionModel,
            // The published maneuver projection (dodge i-frames, blink grace).
            &ae::BodyMotionFacts,
        ),
        // SLOT-0 BY DESIGN: this is the PLAYER-VICTIM path — hitstop, the death
        // banner, the safe-position rewind. Actor-vs-actor damage runs through
        // `apply_actor_hit_events` on the same `HitEvent` stream; the two differ
        // only in the feel/save consequences the local human is owed.
        //
        // ⛔ **AND A BODY THAT HAS ALREADY LOST IS NOT A TARGET** (ADR 0033).
        // Jon, from play: *"when maryo is in her death animation, she still gets
        // hit by enemies."* Mary-O answered that by granting herself
        // `Invulnerability::SCRIPTED` for the beat — a game holding a grant open
        // for a state the engine could see all along. "Out of play" IS "no
        // hurtbox", so the filter says it once, here, for every game.
        (
            PrimaryPlayerOnly,
            bevy::prelude::Without<ambition_combat::death_rules::OutOfPlay>,
        ),
    >,
) {
    let primary = primary_q.single().ok();
    // Drain the staged victim-side hits — attacker-side hits flow to
    // `apply_feature_hit_events` off the message channel directly (same-frame).
    let combat_rules = combat_rules.map(|r| *r).unwrap_or_default();
    let friendly_fire = combat_rules.friendly_fire();
    let events: Vec<FeatureHitEvent> = std::mem::take(&mut pending_hits.0)
        .into_iter()
        // Friendly-fire gate: a same-faction attacker (co-op ally) doesn't damage
        // the player unless friendly fire is on; any different-faction hit lands
        // (the observer takes a duel's strays). Hits with no entity attacker
        // (hazards, string-owned enemy projectiles) are environmental and always apply.
        .filter(
            |e| match e.attacker.and_then(|a| attacker_factions.get(a).ok()) {
                Some(faction) => crate::combat::targeting::can_damage(
                    *faction,
                    crate::combat::components::ActorFaction::Player,
                    friendly_fire,
                ),
                None => true,
            },
        )
        .collect();

    let difficulty_multiplier = incoming_player_damage_multiplier(&user_settings.gameplay);
    let tuning = active_tuning.0;
    // AE6: DIRECTIONAL INFLUENCE IS A MATCH RULE, folded into this system's
    // local copy of the world's feel. The versus stage used to get the same
    // effect by WRITING `Platformer2dFeelTuningMonolith` on route entry and putting it back
    // on exit — correct by discipline, and one crash between the two away from
    // leaving the borrow outstanding. Nothing is written now, so there is
    // nothing to restore.
    let mut feel = *feel_tuning;
    feel.di_max_angle = combat_rules.di_max_angle;
    let safe_world = ambition_platformer2d_world::collision::world_with_sandbox_solids(
        &world.0,
        &moving_platforms.0,
        &feature_ecs_overlay,
    );

    // Resolve every event to a concrete target entity once: events
    // with `HitTarget::Body(e)` route to that player; events with
    // `HitTarget::Volume` (legacy "iterates-and-takes-primary") fall
    // back to the primary player. Events that never resolve (no
    // primary, e.g. headless pre-spawn) are silently dropped.
    let resolved: Vec<(Entity, FeatureHitEvent)> = events
        .into_iter()
        .filter_map(|e| {
            let target = match e.target {
                HitTarget::Body(entity) => Some(entity),
                HitTarget::Volume => primary,
                // Pre-resolved non-player actor victim + orb-match are not player
                // hits — the actor / breakable consumers own them.
                //
                // `UnresolvedFeatures` is not one either, and for a stronger
                // reason: it is the half of a strike whose targets are NOT bodies.
                // Falling back to the primary player the way `Volume` does would
                // hand a player the hit its own swing failed to resolve.
                HitTarget::OrbMatch | HitTarget::UnresolvedFeatures => None,
            };
            target.map(|t| (t, e))
        })
        .collect();

    for (
        player_entity,
        mut cluster_item,
        player_health,
        worn,
        wallet,
        wallet_shield,
        mut anim,
        mut combat,
        mut safety,
        ruleset_owns_death,
        control,
        resolved_frame,
        mut motion_model,
        facts,
    ) in &mut player_q
    {
        let target_events: Vec<FeatureHitEvent> = resolved
            .iter()
            .filter(|(t, _)| *t == player_entity)
            .map(|(_, e)| e.clone())
            .collect();
        let damaged_this_frame = !target_events.is_empty();
        // The victim's held locomotion (local frame) drives DI (CM2).
        let di_input_local = control
            .map(|c| c.0.locomotion.vec())
            .unwrap_or(ae::Vec2::ZERO);

        let mut clusters = cluster_item.as_clusters_mut();
        // The victim's per-tick resolved frame direction (shield side +
        // knockback launch are frame-relative) — the same value its own
        // movement integrated under this tick.
        let victim_gravity_dir = resolved_frame.down();
        // A13: resolved BEFORE the writers are borrowed. The attacker is whoever
        // the first pending event names; the victim is this body.
        let attacker_source = target_events
            .first()
            .and_then(|event| event.attacker)
            .and_then(|attacker| body_sources.get(attacker).ok())
            .map(|source| source.id().clone());
        let victim_source = body_sources
            .get(player_entity)
            .ok()
            .map(|source| source.id().clone());
        // Resolved here for the same reason `attacker_source` is: the attacker is
        // an ENTITY the event names, and this is where the queries are.
        let heavy_attacker = target_events
            .first()
            .and_then(|event| event.attacker)
            .is_some_and(|attacker| heavy_attackers.contains(attacker));
        let remapped = handle_player_damage_events(
            player_entity,
            ruleset_owns_death,
            attacker_source.as_ref(),
            victim_source.as_ref(),
            heavy_attacker,
            &world.0,
            &mut sfx_writer,
            &mut vfx_writer,
            &mut debris_writer,
            &mut death_writers,
            &mut clusters,
            &mut sim_state,
            &mut clock_resets,
            &mut safety,
            &mut banner_requests,
            player_health.map(|h| h.into_inner()),
            worn.map(|w| w.into_inner()),
            wallet
                .map(|wallet| wallet.into_inner())
                .zip(wallet_shield)
                .map(|(wallet, shield)| WalletArmor::new(wallet, shield)),
            &mut wallet_shield_spent,
            &target_events,
            tuning,
            victim_gravity_dir,
            feel,
            difficulty_multiplier,
            di_input_local,
            &mut anim,
            &mut combat,
            &mut motion_model,
            facts,
        );
        // Class-B transit authority (`collision-and-ccd.md` §3.2). Death and the
        // hazard safe-respawn both teleport the victim; recorded here because
        // this is where the entity id lives.
        if remapped {
            if let Some(log) = class_b.as_mut() {
                log.record(
                    player_entity,
                    ambition_platformer2d_shared_tangle::class_b::ClassBRemap::DeathOrReset,
                );
            }
        }

        let ctx = SafePositionContext {
            damaged_this_frame,
            in_hitstun: combat.hitstun_timer > 0.0,
            feature_requested_reset: false,
            blink_grace_active: facts.blink_grace,
            room_transitioning: sim_state.remaining > 0.0,
        };
        remember_safe_player_position(&mut safety, &clusters, &safe_world, ctx);
    }
}

#[cfg(test)]
mod tests;
