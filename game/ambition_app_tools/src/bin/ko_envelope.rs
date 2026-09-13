//! `ko_envelope` — a LAUNCHER-PULSE CENSUS over the prepared moveset graph,
//! and (next phase) a stage-outcome probe over the same launchers.
//!
//! ⭐⭐ **THE ARCHITECTURAL RULE THIS FILE EXISTS TO OBEY:** read authoring from
//! the COMPILED RUNTIME CONTRACT, drive outcomes through the REAL SIMULATION,
//! and reconstruct neither from source text.
//!
//! ⛔⛔ **AND IT IS WRITTEN AGAINST A MEASURED FAILURE.** 2026-09-13, the census
//! this replaces was attempted four times with `grep`/`awk` over
//! `game/ambition_content/src/*_moveset.rs`, and every attempt was wrong:
//!
//! * `paste - -` desynchronised on 300 `knockback:` lines against 295
//!   `knockback_growth:` lines and reported a mean growth/base ratio of **13.7**.
//! * `knockback_growth: TORQUE_GROWTH` (a named constant) and
//!   `knockback_growth: Some(1.35)` were silently skipped by a numeric-only
//!   pattern, desynchronising everything after them.
//! * A search for the literal `forward_throw:` found 10 sites, all in schema and
//!   test code, and concluded **no fighter authors throws**. Every fighter
//!   authors five capture moves. The field's own doc says why the grep failed:
//!   *"this replaces a grep. A goal check read the movesets looking for
//!   `capture: Some`, which is the kind of guard that answers a question the
//!   compiler can answer better."*
//! * A `*_moveset.rs` glob reported 14 movesets where the directory holds 19.
//!
//! ⇒ Every one of those is unreachable from here. A throw is found by asking the
//! contract for a VERB; its numbers are hydrated from the same typed
//! `EffectRef` the engine itself reads. There is no spelling to miss, no
//! shorthand to mis-parse, and no file list to keep current.
//!
//! ```text
//! PreparedCharacterRegistry → SmashRoster::assemble → registry.get(id)
//!   → kit.projectable_moveset() → MovesetContract
//!       strikes: every HitVolume on every WindowTag::Active window
//!       throws:  move_for_verb(CAPTURE_THROW_*_VERB)
//!                  → effect_refs() where key == "smash.capture_throw"
//!                  → hydrate::<CaptureThrowParams>()
//! ```

use std::collections::BTreeMap;

// ⛔ A `#[path]` MODULE, NOT A LIB — this package declares in its own manifest
// that it has no lib on purpose, because a lib relinks every binary in it
// whenever any of them changes. `trap_probe` and `wire_probe` include this same
// file the same way.
//
// ⭐ AND IT IS WORTH REUSING RATHER THAN REWRITING: its header records four
// separate findings that each cost a run measuring nothing — `NoWindow` omits
// the render app entirely, a hand-stepped `update()` does not wait for the wgpu
// device `run()` waits for, the two hosts do not announce the round the same
// way, and a seated fighter's `Brain` overwrites any control frame delivered
// from outside.
#[path = "../probe_stage.rs"]
mod probe_stage;

use ambition_entity_catalog::smash_capture::{CaptureThrowParams, CAPTURE_CARRY, CAPTURE_THROW};
use ambition_entity_catalog::{
    HitVolume, MovesetContract, WindowTag, CAPTURE_THROW_BACK_VERB, CAPTURE_THROW_DOWN_VERB,
    CAPTURE_THROW_FORWARD_VERB, CAPTURE_THROW_UP_VERB,
};

/// The four canonical throw verbs, in report order.
///
/// ⛔ THESE ARE THE CATALOG'S OWN CONSTANTS, not strings spelled here.
/// `smash_capture::verbs` re-exports these very items — *"re-exports, not a
/// second definition … Spelling them again here would be two places for a typo
/// to become a press that does nothing"* — so a lookup through them cannot
/// disagree with what `SmashCaptureRepertoire::bound` installed.
const THROW_VERBS: [(&str, &str); 4] = [
    ("forward_throw", CAPTURE_THROW_FORWARD_VERB),
    ("back_throw", CAPTURE_THROW_BACK_VERB),
    ("up_throw", CAPTURE_THROW_UP_VERB),
    ("down_throw", CAPTURE_THROW_DOWN_VERB),
];

/// ONE AUTHORED PULSE THAT CAN SEND A BODY, in the two forms this game authors.
///
/// ⛔ **NOT A NEW GAMEPLAY ABSTRACTION.** This is the balance instrument's
/// projection of two existing authoring forms onto the one question it asks —
/// "what does this pulse do to a victim at percent p". Nothing in the engine
/// needs to know it exists, and nothing should be taught to.
#[derive(Debug, Clone)]
enum Launcher {
    /// One `HitVolume` of one `Active` window.
    ///
    /// ⚠ ONE VOLUME IS A PULSE, NOT A MOVE. A tipper and a sourspot are
    /// separate balance facts with different envelopes, and an earlier hit of a
    /// multi-hit move adds damage before a later finisher lands — so a
    /// threshold measured here is a PULSE threshold and is reported as one.
    Strike {
        fighter: String,
        move_id: String,
        /// Every input verb that names this move, from the contract's own map.
        /// Roles come from authored bindings, never from guessing at ids like
        /// `fsmash` or `jab`.
        verbs: Vec<String>,
        window: usize,
        volume: usize,
        hit: HitVolume,
    },
    /// One authored throw, hydrated from the effect the engine reads.
    ///
    /// A throw is naturally one release pulse, so its threshold IS meaningful at
    /// the move level.
    Throw {
        fighter: String,
        move_id: String,
        verb: String,
        /// Which of the four slots, for report grouping.
        slot: &'static str,
        params: CaptureThrowParams,
    },
}

impl Launcher {
    fn fighter(&self) -> &str {
        match self {
            Launcher::Strike { fighter, .. } | Launcher::Throw { fighter, .. } => fighter,
        }
    }

    fn move_id(&self) -> &str {
        match self {
            Launcher::Strike { move_id, .. } | Launcher::Throw { move_id, .. } => move_id,
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            Launcher::Strike { .. } => "strike",
            Launcher::Throw { .. } => "throw",
        }
    }

    /// Stable identity: what a later KO row joins back to.
    fn key(&self) -> String {
        match self {
            Launcher::Strike {
                fighter,
                move_id,
                window,
                volume,
                ..
            } => format!("{fighter}/{move_id}/w{window}/v{volume}"),
            Launcher::Throw {
                fighter, move_id, ..
            } => format!("{fighter}/{move_id}/throw"),
        }
    }

    fn damage(&self) -> i32 {
        match self {
            Launcher::Strike { hit, .. } => hit.damage,
            Launcher::Throw { params, .. } => params.damage,
        }
    }

    fn base_knockback(&self) -> f32 {
        match self {
            Launcher::Strike { hit, .. } => hit.knockback,
            Launcher::Throw { params, .. } => params.knockback,
        }
    }

    /// What the AUTHOR wrote. `None` on a strike means "the volume did not
    /// decide"; a throw always states one.
    fn authored_growth(&self) -> Option<f32> {
        match self {
            Launcher::Strike { hit, .. } => hit.knockback_growth,
            Launcher::Throw { params, .. } => Some(params.knockback_growth),
        }
    }

    /// What the ENGINE will actually use.
    ///
    /// ⛔ `None` IS NOT ZERO. `resolved_hitbox_knockback_magnitude` does
    /// `growth.unwrap_or_else(|| base * ruleset_growth.max(0.0))`, so an
    /// unauthored growth resolves to `base × the ruleset's own growth` — and
    /// `Some(0.0)` is the documented FIXED-knockback case, which is a different
    /// thing entirely. Reading a bare `0.0` as unspecified once made the
    /// fixed-knockback case the one value nobody could author.
    fn effective_growth(&self, ruleset_growth: f32) -> f32 {
        self.authored_growth()
            .unwrap_or_else(|| self.base_knockback() * ruleset_growth.max(0.0))
    }

    /// Fixed knockback: the launch ignores the victim's percent AND weight,
    /// because `scaled_knockback` short-circuits a zero growth and returns
    /// `base` untouched.
    fn is_fixed(&self) -> bool {
        self.authored_growth() == Some(0.0)
    }

    fn launch_dir(&self) -> Option<(f32, f32)> {
        match self {
            Launcher::Strike { hit, .. } => hit.launch_dir,
            Launcher::Throw { params, .. } => Some(params.launch_dir),
        }
    }

    /// The role this pulse answers to, from AUTHORED VERB BINDINGS.
    fn role(&self) -> String {
        match self {
            Launcher::Throw { slot, .. } => (*slot).to_string(),
            Launcher::Strike { verbs, move_id, .. } => {
                if verbs.is_empty() {
                    return format!("unbound:{move_id}");
                }
                // The most specific binding wins; ties broken by the contract's
                // own deterministic (BTreeMap) order.
                let mut best = verbs[0].as_str();
                for v in verbs {
                    if v.len() > best.len() {
                        best = v.as_str();
                    }
                }
                best.to_string()
            }
        }
    }
}

/// Why one fighter, verb or volume could not become a launcher.
///
/// ⭐ MALFORMED ROWS ARE REPORTED, NOT DROPPED. A census that silently skips
/// what it cannot read is the same instrument as a grep that finds nothing and
/// calls it absence.
#[derive(Debug)]
struct Malformed {
    fighter: String,
    what: String,
    why: String,
}

/// A capture slot that resolves, is authored on purpose, and DOES NOT LAUNCH.
///
/// ⭐ RECORDED RATHER THAN DROPPED, so its absence from the KO table reads as
/// authored intent instead of a hole somebody should fill. The goblin's down
/// press hoists its captive (`CAPTURE_CARRY`) — *"A CARRY, NOT A THROW, AND NOT
/// BOTH"* — and a census that reported it as a missing down-throw would invite
/// exactly the repair that guard exists to forbid.
#[derive(Debug)]
struct NonLaunching {
    fighter: String,
    slot: &'static str,
    move_id: String,
}

/// Build every launcher one fighter authors.
fn launchers_of(
    fighter: &str,
    contract: &MovesetContract,
    bad: &mut Vec<Malformed>,
    non_launching: &mut Vec<NonLaunching>,
) -> Vec<Launcher> {
    let mut out = Vec::new();

    // Reverse the contract's own verb map: move id → every verb naming it.
    let mut verbs_of: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for (verb, move_id) in &contract.verbs {
        verbs_of.entry(move_id.as_str()).or_default().push(verb.clone());
    }

    // STRIKES — every volume of every Active window, never collapsed.
    for spec in &contract.moves {
        for (w, window) in spec.windows.iter().enumerate() {
            if !matches!(window.tag, WindowTag::Active) {
                continue;
            }
            for (v, hit) in window.volumes.iter().enumerate() {
                out.push(Launcher::Strike {
                    fighter: fighter.to_string(),
                    move_id: spec.id.clone(),
                    verbs: verbs_of.get(spec.id.as_str()).cloned().unwrap_or_default(),
                    window: w,
                    volume: v,
                    hit: hit.clone(),
                });
            }
        }
    }

    // THROWS — asked for by VERB, hydrated from the engine's own effect.
    //
    // ⛔⛔ A RESOLVED CAPTURE VERB IS NOT NECESSARILY A LAUNCHER, and assuming it
    // was is a defect this instrument shipped with for exactly one run. A slot
    // may carry `CAPTURE_CARRY` instead: the goblin's down press HOISTS its
    // captive rather than launching it, authored on purpose and guarded by
    // `the_goblins_down_throw_hauls_instead_of_launching`, whose own words are
    // *"A CARRY, NOT A THROW, AND NOT BOTH"*. The first version of this loop
    // called that malformed — which would have meant breaking a correct fighter
    // to green an instrument.
    //
    // ⇒ THREE OUTCOMES, not two: a throw (launches, measured here), a carry (a
    // capture slot that deliberately does not launch, recorded so its absence
    // from the KO table reads as intent rather than a hole), and absent.
    for (slot, verb) in THROW_VERBS {
        let Some(spec) = contract.move_for_verb(verb) else {
            // Not a defect: an unauthored throw is the authored way to say
            // "this fighter has none".
            continue;
        };
        let throws: Vec<_> = spec
            .effect_refs()
            .into_iter()
            .filter(|(_, effect)| effect.key == CAPTURE_THROW)
            .collect();
        let carries = spec
            .effect_refs()
            .into_iter()
            .filter(|(_, effect)| effect.key == CAPTURE_CARRY)
            .count();
        if throws.is_empty() && carries > 0 {
            non_launching.push(NonLaunching {
                fighter: fighter.to_string(),
                slot,
                move_id: spec.id.clone(),
            });
            continue;
        }
        if throws.len() != 1 {
            bad.push(Malformed {
                fighter: fighter.to_string(),
                what: format!("{slot} ({verb}) → {}", spec.id),
                why: format!(
                    "resolves to a move carrying {} `{CAPTURE_THROW}` and {carries} \
                     `{CAPTURE_CARRY}` effects; expected exactly 1 throw, or a carry",
                    throws.len()
                ),
            });
            continue;
        }
        match throws[0].1.params.hydrate::<CaptureThrowParams>() {
            Ok(params) => out.push(Launcher::Throw {
                fighter: fighter.to_string(),
                move_id: spec.id.clone(),
                verb: verb.to_string(),
                slot,
                params,
            }),
            Err(err) => bad.push(Malformed {
                fighter: fighter.to_string(),
                what: format!("{slot} ({verb}) → {}", spec.id),
                why: format!("installed throw params did not hydrate: {err}"),
            }),
        }
    }

    out
}

/// `count / median / p25 / p75 / min / max` over one column of one role.
#[derive(Debug, Default)]
struct Dist {
    n: usize,
    min: f32,
    p25: f32,
    median: f32,
    p75: f32,
    max: f32,
}

fn dist(mut xs: Vec<f32>) -> Dist {
    if xs.is_empty() {
        return Dist::default();
    }
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let at = |q: f32| -> f32 {
        let i = ((xs.len() as f32 - 1.0) * q).round() as usize;
        xs[i.min(xs.len() - 1)]
    };
    Dist {
        n: xs.len(),
        min: xs[0],
        p25: at(0.25),
        median: at(0.5),
        p75: at(0.75),
        max: xs[xs.len() - 1],
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PHASE 2: THE STAGE-OUTCOME PROBE
//
// ⛔⛔ **A KO IS NOT A RESET.** `measure_cell` in `smash_in_the_host.rs` re-seats
// an entire match per cell, which is correct and unaffordable here: the matrix
// is thousands of trials. Reusing one match is possible, but ONLY behind an
// explicit reset contract, because a knockout spends a stock, opens
// `DeathInterlude`, adds `PendingRespawn`/`OutOfPlay`, grants `RespawnGrace`
// (which publishes `Invulnerability::RESPAWN`), teleports the body, and leaves
// `BodyKnockedOut` messages in the buffer that a fresh cursor would re-read as
// the NEXT trial's result.
//
// ⇒ Every clause below is an assertion, not a hope, and each is tied to a fact
// read out of the engine rather than assumed.
// ─────────────────────────────────────────────────────────────────────────────

/// What one pulse did to one victim at one percent.
#[derive(Debug, Clone)]
struct Trial {
    /// Read off `HitEvent.knockback` — the engine's own resolved number, never
    /// recomputed here. `None` means no `LaunchSpeed` reached the victim, which
    /// is a fixture failure rather than a measurement of zero.
    resolved_launch: Option<f32>,
    ko: bool,
    /// Did the launch reach this body's own tumble threshold, read live off its
    /// motion model rather than restated as `500.0`.
    tumbled: bool,
    ticks_to_ko: Option<usize>,
    /// Victim state at the moment the pulse was fired, for the determinism
    /// probe. Two trials of the same pulse at the same percent must agree here.
    at_strike: String,
    /// The meter the victim actually entered the pulse carrying.
    entry_percent: i32,
    /// What the launch arithmetic saw. For a throw this is `entry + damage`,
    /// because `apply_capture_throws` damages BEFORE reading the meter.
    effective_percent: i32,
}

/// The outcome of a threshold search, including the ones that are not a number.
#[derive(Debug)]
enum Threshold {
    /// Lowest entry percent that KOs, verified: `at-1` survives, `at` kills.
    At(i32),
    /// Never killed anywhere in the tested range.
    Above(i32),
    /// KO then survive as percent RISES. Samples preserved rather than
    /// collapsed into a scalar that would be a fabrication.
    NonMonotonic(Vec<(i32, bool)>),
    /// The pulse never connected — nothing measured.
    NoContact,
    /// The probe could not obtain a clean trial at this percent.
    ///
    /// ⛔ THIS IS NOT A SURVIVAL, AND THE PREVIOUS FORM SAID IT WAS. A refused
    /// reset pushed `(p, false)`, so a percent the INSTRUMENT failed to measure
    /// was recorded as a percent the VICTIM lived through — and the threshold
    /// walked straight past it. Run 2 refused 8 of 64 rows this way.
    Refused(i32),
}

impl Threshold {
    fn cell(&self) -> String {
        match self {
            Threshold::At(p) => p.to_string(),
            Threshold::Above(p) => format!(">{p}"),
            Threshold::NonMonotonic(s) => format!("NON_MONOTONIC{s:?}"),
            Threshold::NoContact => "NO_CONTACT".into(),
            Threshold::Refused(p) => format!("REFUSED@{p}"),
        }
    }
}

struct KoProbe {
    app: bevy::prelude::App,
    attacker: bevy::prelude::Entity,
    victim: bevy::prelude::Entity,
    centre: f32,
    tumble_speed: f32,
    victim_weight: f32,
}

impl KoProbe {
    fn new(attacker_id: &str, victim_id: &str) -> Self {
        use ambition_platformer2d::characters::brain::Brain;

        let staged = probe_stage::stage(probe_stage::StageRequest {
            cast: [attacker_id, victim_id],
            demo_host: false,
            rendered: false,
        });
        let probe_stage::Staged {
            mut app,
            seat0,
            seat1,
        } = staged;

        // ⛔ STOCKS FIRST, AND IT IS A CORRECTNESS PRECONDITION rather than a
        // speed trick: `STARTING_STOCKS` is 3, and Smash's
        // `take_eliminated_fighters_out_of_play` DESPAWNS an eliminated body —
        // so the fourth probe KO would query a destroyed entity.
        for body in [seat0, seat1] {
            if let Some(mut stocks) = app
                .world_mut()
                .get_mut::<ambition_platformer2d::actor::FighterStocks>(body)
            {
                stocks.remaining = 9_999;
                stocks.started_with = 9_999;
            }
            // ⛔ BOTH brains, not just seat 0. A live CPU victim is what broke
            // the first version of `ring_out`: it walked during the window and
            // something reset its meter mid-reading.
            app.world_mut().entity_mut(body).insert(Brain::stand_still());
        }
        app.update();

        // Read off the REAL stage, the way `ring_out::stage_centre_and_reach`
        // does — never restated as a literal, because `STAGE_SIZE` and the blast
        // margins are the demo's own constants and a copy here would be a second
        // stage that silently stops matching the first.
        let room = ambition_demo_smash::smash_stage();
        let centre = room.world.size.x / 2.0;

        // The victim's OWN tumble threshold, off its live motion model.
        let tumble_speed = app
            .world()
            .get::<ambition_platformer2d::actor::MotionModel>(seat1)
            .and_then(|model| match model {
                ambition_platformer2d::actor::MotionModel::AxisSwept(axis) => {
                    Some(axis.params.abilities.tumble_speed)
                }
                _ => None,
            })
            .unwrap_or(0.0);

        // The divisor the knockback law actually uses, read not restated.
        let victim_weight = app
            .world()
            .get::<ambition_platformer2d::combat::components::CombatTuning>(seat1)
            .map(|t| t.weight)
            .unwrap_or(1.0);

        Self {
            app,
            attacker: seat0,
            victim: seat1,
            centre,
            tumble_speed,
            victim_weight,
        }
    }

    fn pos(&self, body: bevy::prelude::Entity) -> ambition_platformer2d::engine_core::Vec2 {
        self.app
            .world()
            .get::<ambition_platformer2d::platformer::body::BodyKinematics>(body)
            .map(|k| k.pos)
            .unwrap_or_default()
    }

    fn park(&mut self, body: bevy::prelude::Entity, x: f32) {
        probe_stage::pin_grounded_at_rest(
            &mut self.app,
            body,
            ambition_platformer2d::engine_core::Vec2::new(x, 200.0),
        );
    }

    /// Every victim fact a trial could INHERIT, in one tab-free field.
    ///
    /// ⛔ NOT A HEALTH CHECK. `reset_trial` already asserts the conditions it
    /// knows to assert; this exists precisely for the fact it does NOT know
    /// about, so it reports state rather than judging it. The differing column
    /// between a pre-KO trial and a post-KO trial is the answer.
    fn snapshot(&self) -> String {
        use ambition_platformer2d::characters::actor::{BodyCombat, BodyHealth};
        let w = self.app.world();
        let v = self.victim;
        let ground = w
            .get::<ambition_platformer2d::engine_core::BodyGroundState>(v)
            .map(|g| g.on_ground)
            .unwrap_or(false);
        let (pos, vel) = w
            .get::<ambition_platformer2d::platformer::body::BodyKinematics>(v)
            .map(|k| (k.pos, k.vel))
            .unwrap_or_default();
        let (hitlag, hitstun, hitstop) = w
            .get::<BodyCombat>(v)
            .map(|c| (c.is_in_hitlag(), c.hitstun_timer, c.hitstop_timer))
            .unwrap_or((false, 0.0, 0.0));
        let dmg = w
            .get::<BodyHealth>(v)
            .map(|h| h.damage_taken())
            .unwrap_or(0);
        let mut flags = Vec::new();
        if w.get::<ambition_platformer2d::combat::stocks::RespawnGrace>(v)
            .is_some()
        {
            flags.push("grace");
        }
        if w.get::<ambition_platformer2d::combat::death_rules::DeathInterlude>(v)
            .is_some()
        {
            flags.push("interlude");
        }
        if w.get::<ambition_platformer2d::combat::stocks::PendingRespawn>(v)
            .is_some()
        {
            flags.push("pending");
        }
        if w.get::<ambition_platformer2d::combat::death_rules::OutOfPlay>(v)
            .is_some()
        {
            flags.push("outofplay");
        }
        if w.get::<ambition_platformer2d::combat::capture::CapturedBy>(v)
            .is_some()
        {
            flags.push("captured");
        }
        let flags = if flags.is_empty() {
            "none".to_string()
        } else {
            flags.join("+")
        };
        format!(
            "ground={ground} pos=({:.1},{:.1}) vel=({:.1},{:.1}) hitlag={hitlag} \
             hitstun={hitstun:.3} hitstop={hitstop:.3} dmg={dmg} flags={flags}",
            pos.x, pos.y, vel.x, vel.y
        )
    }

    /// ⛔ THE CONTRACT. A trial may not begin until every one of these is true,
    /// and each is CHECKED rather than waited-out by tick count.
    fn reset_trial(&mut self, entry_percent: i32, victim_x: f32) -> bool {
        use ambition_platformer2d::characters::actor::{BodyHealth, Invulnerability};

        // 1. Let the real respawn lifecycle finish. `respawn_when_the_interlude_
        //    closes` gates on `!DeathInterlude.open()` and then hands back every
        //    fact the spend took, so waiting on the MARKERS is waiting on the
        //    engine's own answer.
        let mut settled = false;
        for _ in 0..600 {
            let w = self.app.world();
            let between_lives = w
                .get::<ambition_platformer2d::combat::death_rules::DeathInterlude>(self.victim)
                .is_some()
                || w.get::<ambition_platformer2d::combat::stocks::PendingRespawn>(self.victim)
                    .is_some()
                || w.get::<ambition_platformer2d::combat::death_rules::OutOfPlay>(self.victim)
                    .is_some();
            // 2. BOTH grace witnesses. `stocks.rs` clears the bit when the clock
            //    expires AND retracts it when the component is removed, so the
            //    two agreeing is a measurement; either alone is a claim. A trial
            //    begun under live grace hits an untouchable victim and reports a
            //    FALSE SURVIVAL — the worst failure this probe can have.
            let protected = w
                .get::<ambition_platformer2d::combat::stocks::RespawnGrace>(self.victim)
                .is_some()
                || w.get::<BodyHealth>(self.victim)
                    .is_some_and(|h| h.health.invulnerable.holds(Invulnerability::RESPAWN));
            let frozen = w
                .get::<ambition_platformer2d::characters::actor::BodyCombat>(self.victim)
                .is_some_and(|c| c.is_in_hitlag() || c.hitstun_timer > 0.0);
            let captured = w
                .get::<ambition_platformer2d::combat::capture::CapturedBy>(self.victim)
                .is_some();
            if !between_lives && !protected && !frozen && !captured {
                settled = true;
                break;
            }
            self.app.update();
        }
        if !settled {
            // ⭐ SAY WHICH CONDITION BLOCKED IT. A refusal that reports only
            // "refused" sends the next reader guessing, and run 3 refused 16 of
            // 60 cells — every one of them a LEDGE cell, clustered where
            // thresholds are low and therefore where nearly every trial ends in
            // a KO. That points at the respawn cycle rather than the position,
            // but pointing is not knowing, so the probe names the marker still
            // standing when its budget ran out.
            let w = self.app.world();
            let mut blocked = Vec::new();
            if w.get::<ambition_platformer2d::combat::death_rules::DeathInterlude>(self.victim)
                .is_some()
            {
                blocked.push("interlude");
            }
            if w.get::<ambition_platformer2d::combat::stocks::PendingRespawn>(self.victim)
                .is_some()
            {
                blocked.push("pending");
            }
            if w.get::<ambition_platformer2d::combat::death_rules::OutOfPlay>(self.victim)
                .is_some()
            {
                blocked.push("outofplay");
            }
            if w.get::<ambition_platformer2d::combat::stocks::RespawnGrace>(self.victim)
                .is_some()
            {
                blocked.push("grace");
            }
            if w.get::<ambition_platformer2d::combat::capture::CapturedBy>(self.victim)
                .is_some()
            {
                blocked.push("captured");
            }
            if w.get::<ambition_platformer2d::characters::actor::BodyCombat>(self.victim)
                .is_some_and(|c| c.is_in_hitlag() || c.hitstun_timer > 0.0)
            {
                blocked.push("frozen");
            }
            eprintln!(
                "KO_REFUSE: stage=settle x={victim_x:.0} pct={entry_percent} blocked=[{}]",
                if blocked.is_empty() {
                    "NONE-none-of-the-tracked-markers".to_string()
                } else {
                    blocked.join("+")
                }
            );
            return false;
        }

        // 3. Place, still, and metered.
        self.park(self.victim, victim_x);
        self.park(self.attacker, victim_x - 240.0);
        for _ in 0..40 {
            self.park(self.attacker, victim_x - 240.0);
            self.app.update();
            let grounded = self
                .app
                .world()
                .get::<ambition_platformer2d::engine_core::BodyGroundState>(self.victim)
                .is_some_and(|g| g.on_ground);
            if grounded {
                break;
            }
        }
        // ⛔⛔ DO NOT `park` AGAIN HERE — MEASURED 2026-09-13, AND THIS WAS THE
        // DEFECT UNDER RUN 1'S WHOLE TABLE.
        //
        // `park` places a body at y = 200.0, which is ABOVE the platform surface
        // (PLATFORM_TOP is 300). The loop above spends up to forty ticks waiting
        // for the victim to LAND, and the previous form then lifted it straight
        // back into the air and ran one update — so every trial began airborne,
        // and `ground=` varied with where that single tick left it. A body
        // struck in the air takes a different road than one struck standing,
        // which is why four identical trials alternated survive/KO on an
        // IDENTICAL resolved launch of 403.0, and why the oracle row's centre
        // threshold moved 291 -> 283 when a coarser step reshuffled the order.
        //
        // Correct x WITHOUT restoring the height the landing just resolved.
        //
        // ⛔⛤ AND THROUGH THE PIN, DELIBERATELY NOT THROUGH `transit_body`: this
        // body is being slid along a floor it is ALREADY STANDING ON, and the
        // grounded premise asserted just below is the entire point of the
        // reset. The transit authority would clear that contact by design. See
        // `pin` for the measurement that settled it.
        //
        // The `y` is read back from the body the loop above landed, so only the
        // `x` is the trial's to choose — the pin takes a full pose, and handing
        // it anything else would restore the height this comment forbids.
        let landed_y = self.pos(self.victim).y;
        probe_stage::pin_grounded_at_rest(
            &mut self.app,
            self.victim,
            ambition_platformer2d::engine_core::Vec2::new(victim_x, landed_y),
        );
        self.app.update();

        // ⭐ AND THE PREMISE IS NOW CHECKED RATHER THAN HOPED FOR. A trial that
        // cannot start the victim standing still on the floor does not start.
        let grounded = self
            .app
            .world()
            .get::<ambition_platformer2d::engine_core::BodyGroundState>(self.victim)
            .is_some_and(|g| g.on_ground);
        if !grounded {
            return false;
        }

        for (body, meter) in [(self.victim, entry_percent), (self.attacker, 0)] {
            // ⛔ THE ATTACKER IS PINNED TO ZERO because `rage_scale` reads its
            // meter and multiplies EVERY resolved launch. The sweep that picked
            // the shipped percent scale ran with an uncontrolled ~1.25x rage.
            if let Some(mut health) = self.app.world_mut().get_mut::<BodyHealth>(body) {
                health.set_damage_taken(meter);
            }
        }
        self.app.update();
        true
    }

    /// Fire one authored volume at the parked victim and read the engine's own
    /// verdict.
    fn strike(&mut self, hit: &HitVolume, entry_percent: i32, victim_x: f32) -> Option<Trial> {
        use ambition_platformer2d::combat::events::{HitEvent, HitKnockbackMagnitude, HitTarget};
        use ambition_platformer2d::combat::stocks::BodyKnockedOut;
        use ambition_platformer2d::engine_core::Vec2 as EVec2;

        if !self.reset_trial(entry_percent, victim_x) {
            return None;
        }
        // ⭐ THE STATE THE TRIAL ACTUALLY BEGINS IN, captured AFTER the reset.
        // The first version of this diagnostic snapshotted before `reset_trial`
        // and therefore reported the PREVIOUS trial's leftovers — informative
        // about what carries over, and silent about the only thing that decides
        // an outcome, which is the state at the moment of the strike.
        let at_strike = self.snapshot();
        let struck_at = self.pos(self.victim);
        let attacker = self.attacker;

        // ⛔⛔ CURSORS MADE **ONCE, HERE**, AND ADVANCED INSIDE THE LOOP.
        //
        // This is the whole defence against a previous trial's knockout being
        // credited to this one. A cursor created now is already positioned past
        // everything the buffer holds, so the reads below see only what this
        // pulse produces; a cursor created per tick would re-read from the
        // buffer's START every tick and rediscover every earlier trial's KO.
        //
        // ⚠ There is no `get_cursor_from_end` in bevy_ecs 0.19.1 — `get_cursor`
        // is the only constructor, and every call site in this repo uses it.
        // "Made once, before the loop" IS the end-of-stream guarantee; it is a
        // discipline rather than an API, which is exactly why it is written down
        // here instead of assumed.
        let mut hit_cursor = self
            .app
            .world()
            .resource::<bevy::ecs::message::Messages<HitEvent>>()
            .get_cursor();
        let mut ko_cursor = self
            .app
            .world()
            .resource::<bevy::ecs::message::Messages<BodyKnockedOut>>()
            .get_cursor();

        let strike = self
            .app
            .world_mut()
            .spawn((
                ambition_platformer2d::combat::strike::Hitbox {
                    owner: attacker,
                    source: ambition_platformer2d::vfx::HitSide::Enemy,
                    anchor: ambition_platformer2d::combat::strike::HitboxAnchor::World {
                        center: struck_at,
                    },
                    half_extent: EVec2::new(48.0, 48.0),
                    shape: None,
                    facing: 1.0,
                    damage: hit.damage,
                    knockback: ambition_platformer2d::combat::strike::HitboxKnockback::LaunchSpeed {
                        base: hit.knockback,
                        growth: hit.knockback_growth,
                    },
                    launch_dir: hit.launch_dir.map(|(x, y)| EVec2::new(x, y)),
                    frame_down: EVec2::new(0.0, 1.0),
                    strike_sfx: None,
                    reaction: None,
                },
                ambition_platformer2d::combat::strike::HitboxHits::default(),
                ambition_platformer2d::combat::strike::HitboxLifetime { remaining_s: 0.1 },
            ))
            .id();

        let mut resolved_launch = None;
        let mut ko = false;
        let mut ticks_to_ko = None;
        // ⭐ THE TRIAL ENDS WHEN IT IS DECIDED, NOT WHEN THE BUDGET RUNS OUT.
        //
        // Measured 2026-09-13: a SURVIVING trial spent all 150 ticks watching a
        // body that had already come to rest, and at ~8.9ms/frame that budget —
        // not the simulation — was the whole cost of this instrument (~170s per
        // pulse row, ~5.7h for the matrix, against a 90m timeout).
        //
        // ⛔ THE ARMING CONDITIONS ARE THE CORRECTNESS ARGUMENT, and each one
        // excludes a way a body can be still WITHOUT the trial being over:
        //   * `resolved_launch.is_some()` — before contact the victim is parked
        //     and motionless, which looks exactly like "come to rest".
        //   * `!is_in_hitlag()` — impact hitstop FREEZES both bodies. A victim
        //     mid-freeze is perfectly still and has not yet travelled anywhere.
        //   * `hitstun_timer <= 0` — still being carried by the launch.
        //   * grounded and slow, for `SETTLED_TICKS` CONSECUTIVE ticks, so a
        //     single frame of ground contact mid-arc cannot end the trial.
        // A body that satisfies all four has been launched, has finished its
        // launch, and is standing on the floor: it cannot reach a blast line.
        const SETTLED_TICKS: usize = 4;
        const SLOW_PX_S: f32 = 12.0;
        let mut settled_for = 0usize;
        for tick in 0..150 {
            self.app.update();
            if resolved_launch.is_none() {
                let messages = self.app.world().resource::<bevy::ecs::message::Messages<HitEvent>>();
                for ev in hit_cursor.read(messages) {
                    if !matches!(ev.target, HitTarget::Body(e) if e == self.victim) {
                        continue;
                    }
                    if let Some(kb) = ev.knockback.as_ref() {
                        if let HitKnockbackMagnitude::LaunchSpeed(v) = kb.magnitude {
                            resolved_launch = Some(v);
                            break;
                        }
                    }
                }
            }
            if !ko {
                let messages = self
                    .app
                    .world()
                    .resource::<bevy::ecs::message::Messages<BodyKnockedOut>>();
                if ko_cursor.read(messages).any(|k| {
                    k.body == self.victim
                        && matches!(
                            k.cause,
                            ambition_platformer2d::combat::HitSource::LeftTheWorld
                        )
                }) {
                    ko = true;
                    ticks_to_ko = Some(tick);
                }
            }
            if ko {
                break;
            }
            if resolved_launch.is_some() {
                let w = self.app.world();
                let frozen = w
                    .get::<ambition_platformer2d::characters::actor::BodyCombat>(self.victim)
                    .is_some_and(|c| c.is_in_hitlag() || c.hitstun_timer > 0.0);
                let grounded = w
                    .get::<ambition_platformer2d::engine_core::BodyGroundState>(self.victim)
                    .is_some_and(|g| g.on_ground);
                let slow = w
                    .get::<ambition_platformer2d::platformer::body::BodyKinematics>(self.victim)
                    .is_some_and(|k| k.vel.x.abs() < SLOW_PX_S && k.vel.y.abs() < SLOW_PX_S);
                if !frozen && grounded && slow {
                    settled_for += 1;
                    if settled_for >= SETTLED_TICKS {
                        let _ = tick;
                        break;
                    }
                } else {
                    settled_for = 0;
                }
            }
        }
        if self.app.world().get_entity(strike).is_ok() {
            self.app.world_mut().entity_mut(strike).despawn();
        }

        Some(Trial {
            tumbled: resolved_launch.is_some_and(|v| self.tumble_speed > 0.0 && v >= self.tumble_speed),
            resolved_launch,
            ko,
            ticks_to_ko,
            entry_percent,
            effective_percent: entry_percent,
            at_strike,
        })
    }

    /// Coarse sweep → bracket → binary search → VERIFY both sides.
    ///
    /// ⛔ NOT A BARE BINARY SEARCH. The raw launch formula is monotonic in
    /// percent, but a TRAJECTORY is not obliged to be: platforms, ceilings,
    /// downward launches and landings all intervene. A search that assumes
    /// monotonicity would return a confident number for a curve that does not
    /// have one.
    /// The resolved launch at one percent, retrying a refused reset.
    fn launch_at(&mut self, hit: &HitVolume, percent: i32, victim_x: f32) -> Option<f32> {
        for _ in 0..3 {
            if let Some(t) = self.strike(hit, percent, victim_x) {
                return t.resolved_launch;
            }
        }
        None
    }

    /// Does this pulse KO at `percent`? DECIDED BY MAJORITY OF THREE.
    ///
    /// ⛔ ONE TRIAL IS NOT AN ANSWER NEAR A BOUNDARY. Run 2 self-detected 15
    /// disagreements across 100 cells and they were all one shape: the binary
    /// search converged on a percent BECAUSE a trial there killed, and the
    /// verification at that same percent then did not. Far from a threshold this
    /// probe is exactly reproducible — 8 of 8 trials byte-identical at 200% on a
    /// move whose threshold is above 300 — so the flakiness is not general. It
    /// lives within a percent or two of the blast line, where the body crosses
    /// or fails to cross on the strength of a single tick.
    ///
    /// ⭐ THE RIGHT PRECISION IS THE ONE THE QUESTION NEEDS. Tuning how a game
    /// FEELS does not turn on ±1%, so the answer is not a more exact search; it
    /// is a cell that reports the same value twice. Majority of three, short-
    /// circuiting as soon as either side reaches two.
    ///
    /// A refused reset is RETRIED rather than counted, and `None` — the probe
    /// could not obtain a clean trial — is returned so the caller can say so
    /// instead of silently scoring a survival.
    fn kills(&mut self, hit: &HitVolume, percent: i32, victim_x: f32) -> Option<bool> {
        let (mut yes, mut no) = (0, 0);
        while yes < 2 && no < 2 {
            let mut trial = None;
            for _ in 0..3 {
                if let Some(t) = self.strike(hit, percent, victim_x) {
                    trial = Some(t);
                    break;
                }
            }
            match trial {
                Some(t) if t.ko => yes += 1,
                Some(_) => no += 1,
                None => return None,
            }
        }
        Some(yes > no)
    }

    fn ko_threshold(&mut self, hit: &HitVolume, victim_x: f32, max_percent: i32) -> Threshold {
        // ⚠ 50 RATHER THAN 25, AND IT IS A TRADE I AM MAKING ON PURPOSE.
        // Widening the coarse step halves the sweep (13 samples -> 7) and costs
        // the binary search one extra iteration, so the THRESHOLD it converges
        // on is unchanged — but the sweep is also how `NonMonotonic` is spotted,
        // and half as many samples is half as many chances to see a curve double
        // back. The both-sides verification below still runs on every cell, so a
        // non-monotonicity AT the threshold is still caught; one hiding strictly
        // between two coarse samples is not. That is the exposure.
        let step = 50;
        let mut samples: Vec<(i32, bool)> = Vec::new();
        let mut bracket: Option<(i32, i32)> = None;
        let mut p = 0;
        while p <= max_percent {
            let Some(ko) = self.kills(hit, p, victim_x) else {
                return Threshold::Refused(p);
            };
            samples.push((p, ko));
            if ko && bracket.is_none() {
                bracket = Some(((p - step).max(0), p));
            }
            // A KO followed by a survival at a HIGHER percent is a real
            // finding, not noise to be smoothed.
            if bracket.is_some() && !ko {
                return Threshold::NonMonotonic(samples);
            }
            if bracket.is_some() {
                break;
            }
            p += step;
        }
        let Some((mut lo, mut hi)) = bracket else {
            // ⭐ "NEVER KILLED" AND "NEVER CONNECTED" ARE DIFFERENT ANSWERS and
            // only one of them is about the game. One probe at the top of the
            // range separates them.
            let connected = self
                .launch_at(hit, max_percent, victim_x)
                .is_some();
            return if connected {
                Threshold::Above(max_percent)
            } else {
                Threshold::NoContact
            };
        };
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            let Some(killed) = self.kills(hit, mid, victim_x) else {
                return Threshold::Refused(mid);
            };
            if killed {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        // ⛔ ZERO IS THE FLOOR, AND IT HAS NO BELOW — measured 2026-09-13.
        //
        // `hi == 0` means the pulse KOs at zero percent. The previous form still
        // ran the both-sides check at `(hi - 1).max(0)`, which is 0 AGAIN, so
        // `below` was necessarily the same measurement as `at` and the
        // contradiction branch fired every time. It then recorded the sample as
        // `hi - 1` = `-1`, a percent no trial ever ran. Every zero-percent ledge
        // KO in run 1 was reported `NON_MONOTONIC[(0, true), (-1, true), ...]`
        // on the strength of that. A KO at the floor is a finding, not a fault.
        if hi == 0 {
            return Threshold::At(0);
        }
        // VERIFY BOTH SIDES rather than trusting the walk.
        //
        // ⛔ THROUGH `kills`, NOT `strike`. These two lines were the last place a
        // single trial decided anything, and they are precisely where run 2's
        // fifteen disagreements surfaced: the search converged on `hi` because a
        // trial there killed, and then ONE trial at that same `hi` said it did
        // not. `is_some_and` also folded a refused reset into "did not kill",
        // which is the same silent survival the `Refused` variant exists to stop.
        let Some(below) = self.kills(hit, hi - 1, victim_x) else {
            return Threshold::Refused(hi - 1);
        };
        let Some(at) = self.kills(hit, hi, victim_x) else {
            return Threshold::Refused(hi);
        };
        if below || !at {
            samples.push((hi - 1, below));
            samples.push((hi, at));
            return Threshold::NonMonotonic(samples);
        }
        Threshold::At(hi)
    }
}

/// PHASE 2 ENTRY: KO envelopes for reachable strike pulses, reference vs heavy.
///
/// ⛔ THE SELF-TEST RUNS FIRST AND GATES EVERYTHING. Reusing one staged match
/// across trials is only sound if a repeated trial repeats; if it does not, a
/// whole table would be contaminated by the previous stock and look like data.
/// ⭐ AT WHICH PERCENTS DOES THE PULSE FAIL TO CONNECT AT ALL?
///
/// The determinism probe reported `launch=-` — no contact — for a 300% trial
/// whose at-strike state was IDENTICAL to eight trials at 200% that connected
/// cleanly, and it did so in both runs. That is reproducible and tracks the
/// percent rather than the trial's position in the sequence.
///
/// ⛔ THIS IS NOT COSMETIC. `ko_threshold` records a no-contact trial as a
/// SURVIVAL, so a percent band where contact silently fails reads as a band the
/// victim survived, and the threshold walks straight past it.
///
/// Two hypotheses were rejected by reading the code rather than by running this:
/// the emitted magnitude variant is decided by the `HitboxKnockback` this probe
/// constructs (`LaunchSpeed`), so no variant is being dropped by the reader; and
/// `set_damage_taken` is `accumulated = damage.max(0)`, so there is no cap and
/// no state transition at a high meter. Having no hypothesis left is the reason
/// to measure rather than to keep guessing.
fn run_contact() {
    let mut probe = KoProbe::new("npc_pirate_admiral", "player_robot_v3");
    let launchers = {
        let world = probe.app.world();
        let registry = world
            .get_resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>()
            .expect("registry");
        let prepared = registry.get("npc_pirate_admiral").expect("attacker");
        let contract = prepared.kit.projectable_moveset().expect("moveset");
        let mut bad = Vec::new();
        let mut nl = Vec::new();
        launchers_of("npc_pirate_admiral", contract, &mut bad, &mut nl)
    };
    let Some(Launcher::Strike { hit, move_id, .. }) = launchers
        .iter()
        .find(|l| l.role() == "smash_forward" && matches!(l, Launcher::Strike { .. }))
    else {
        println!("# no attack_forward strike found");
        return;
    };
    println!("# contact sweep: npc_pirate_admiral `{move_id}` vs player_robot_v3, centre");
    println!("# a row with contact=false is one `ko_threshold` would have recorded as a SURVIVAL");
    println!("percent\tcontact\tlaunch\tko\tat_strike");
    for p in (0..=300).step_by(25) {
        match probe.strike(hit, p, probe.centre) {
            Some(t) => println!(
                "{p}\t{}\t{}\t{}\t{}",
                t.resolved_launch.is_some(),
                t.resolved_launch
                    .map(|v| format!("{v:.1}"))
                    .unwrap_or_else(|| "-".into()),
                t.ko,
                t.at_strike
            ),
            None => println!("{p}\tRESET_REFUSED\t-\t-\t-"),
        }
    }
}

/// ⭐ DOES A TRIAL INHERIT ANYTHING FROM THE TRIAL BEFORE IT?
///
/// Run 1 said yes and named the shape. Two cells reported a percent that the
/// coarse sweep measured as a SURVIVAL and the verification measured as a KO,
/// and in both the binary search had converged to `lo + 1` — every probe after
/// the first knockout came back a knockout. The oracle row moved with it
/// (centre 291 on the slow path, 283 on the fast one).
///
/// ⛔ THE SELF-TEST IN `run_probe` CANNOT SEE THIS. It fires one pulse at one
/// percent twice with identical history, so it establishes that the probe is
/// deterministic given the same past — which is not the property the threshold
/// search needs. The search needs the answer at a percent to be independent of
/// what ran before it, and that is what this measures.
///
/// This prints state rather than asserting, because the question is WHICH fact
/// carries over, and a guess at that is worth nothing.
fn run_determinism() {
    // ⛔⛔ NEAR A REAL BOUNDARY, AND THE FIRST VERSION OF THIS TEST WAS NOT.
    //
    // It probed 200% on a move whose centre threshold is above 300, so every
    // trial was a comfortable survival and all eight agreed. That established
    // the probe is reproducible where nothing is in doubt — a guard that cannot
    // fail — and it is why run 2 still produced fifteen disagreeing cells after
    // this test had passed. The flakiness lives within a percent or two of the
    // blast line, so the test has to stand there.
    //
    // ⛔ AND THE SECOND ATTEMPT MISSED TOO, FOR A DIFFERENT REASON. It used 50%,
    // taken from a `smash_forward` LEDGE contradiction, but then fired
    // `tilt_forward` at CENTRE — a different move at a different position, where
    // the threshold is above 300. Result: 6 of 6 `kills` and 8 of 8 single
    // trials all agreeing on a comfortable survival. A guard that cannot fail,
    // twice over.
    //
    // ⛔ THE THIRD ATTEMPT MISSED TOO, AND FOR THE MOST AVOIDABLE REASON YET: it
    // took 145 from RUN 2, a table produced by the broken instrument. 8 of 8
    // single trials came back a unanimous KILL at launch 721.9. A percent
    // inherited from void data is not a threshold on this tree.
    //
    // ⭐ SO THE BOUNDARY IS MEASURED ON THIS TREE, NOT INHERITED. Sweeping
    // `smash_forward` at centre with the current binary:
    //     125% -> survive (launch 644.4)
    //     150% -> KO      (launch 741.2)
    // and 145% was already a unanimous kill, so the flip sits below it. 130 is
    // inside the bracket and is where a single trial should waver if it ever
    // does — which the single-trial control below is there to report.
    const NEAR: i32 = 130;
    const LETHAL: i32 = 300;

    let mut probe = KoProbe::new("npc_pirate_admiral", "player_robot_v3");
    let launchers = {
        let world = probe.app.world();
        let registry = world
            .get_resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>()
            .expect("registry");
        let prepared = registry.get("npc_pirate_admiral").expect("attacker");
        let contract = prepared.kit.projectable_moveset().expect("moveset");
        let mut bad = Vec::new();
        let mut nl = Vec::new();
        launchers_of("npc_pirate_admiral", contract, &mut bad, &mut nl)
    };
    // The pulse the run-1 latch actually appeared on: a forward tilt, whose
    // centre cell was one of the two genuine disagreements.
    let Some(Launcher::Strike { hit, move_id, .. }) = launchers
        .iter()
        .find(|l| l.role() == "smash_forward" && matches!(l, Launcher::Strike { .. }))
    else {
        println!("# no attack_forward strike found — cannot reproduce the run-1 latch");
        return;
    };
    println!("# determinism probe on npc_pirate_admiral `{move_id}` vs player_robot_v3");
    println!("# every line below is the SAME pulse at the SAME percent unless marked");
    println!("phase\ttrial\tpercent\tko\tlaunch\tticks\tpre_state");

    let x = probe.centre;

    // ⭐⭐ THE ARM THAT TESTS THE ACTUAL CLAIM.
    //
    // The single-trial rows below may legitimately disagree with each other at
    // this percent — that IS the defect, and reproducing it is half the point.
    // The claim that needs testing is the REMEDY: that `kills`, deciding by
    // majority of three, returns the same verdict every time at a percent where
    // one trial does not. A test that only ever calls `strike` cannot say
    // anything about that, and calling this file's determinism probe "passing"
    // on the strength of single trials is how run 2 shipped fifteen bad cells.
    //
    // ⛔ If these six disagree, majority-of-three is NOT enough and no matrix
    // should be run on it.
    let mut votes = Vec::new();
    for _ in 0..6 {
        votes.push(
            probe
                .kills(hit, NEAR, x)
                .map(|k| k.to_string())
                .unwrap_or_else(|| "REFUSED".into()),
        );
    }
    println!("# kills()@{NEAR}% x6 (majority-of-three each): {}", votes.join(" "));
    // ⛔ THIS LINE MAY NOT CLAIM BOUNDARY-NESS IT HAS NOT ESTABLISHED.
    //
    // It previously printed "the remedy holds at a boundary percent" whenever
    // the six votes agreed — including on percents that were nowhere near a
    // boundary, which is what happened on all three attempts (200% and 50% were
    // comfortable survivals; 145% a comfortable kill, 8 of 8 single trials
    // agreeing at launch 721.9). Agreement at a percent where ONE trial is
    // already unanimous says nothing whatever about majority-of-three.
    //
    // The single-trial arm is the control: only if IT disagrees is this percent
    // a boundary, and only then does agreement among the votes mean anything.
    let votes_agree = votes.iter().all(|v| v == &votes[0]);
    println!(
        "# ⇒ kills() {}",
        if votes_agree {
            "agreed 6/6"
        } else {
            "⛔ DISAGREED — majority-of-three is insufficient; do not run the matrix"
        }
    );
    println!(
        "# ⇒ whether that MEANS anything depends on the single-trial rows below: \
         if they are unanimous too, {NEAR}% is not a boundary and this test is \
         a guard that cannot fail."
    );

    let x = probe.centre;
    for i in 0..4 {
        let t = probe.strike(hit, NEAR, x);
        let pre = t
            .as_ref()
            .map(|t| t.at_strike.clone())
            .unwrap_or_else(|| "RESET_REFUSED".into());
        println!(
            "before_any_ko\t{i}\t{NEAR}\t{}\t{}\t{}\t{pre}",
            t.as_ref().map(|t| t.ko.to_string()).unwrap_or_else(|| "-".into()),
            t.as_ref()
                .and_then(|t| t.resolved_launch)
                .map(|v| format!("{v:.1}"))
                .unwrap_or_else(|| "-".into()),
            t.as_ref()
                .and_then(|t| t.ticks_to_ko)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "-".into()),
        );
    }

    let t = probe.strike(hit, LETHAL, x);
    let pre = t
        .as_ref()
        .map(|t| t.at_strike.clone())
        .unwrap_or_else(|| "RESET_REFUSED".into());
    // ⛔ THE LAUNCH COLUMN IS FORMATTED, NOT HARDCODED — and it was hardcoded.
    //
    // This row printed a literal `-` where every other row prints a measured
    // launch, and I read my own placeholder as a measurement: it became a
    // reported "no contact at 300%", a commit message recording a defect as
    // KNOWN AND NOT YET EXPLAINED, and a contact sweep to chase it. The sweep
    // came back 13 of 13 connected. The trial had been fine the whole time.
    //
    // ⇒ A column that can only ever print one value is not reporting anything.
    println!(
        "INDUCE_KO\t-\t{LETHAL}\t{}\t{}\t{}\t{pre}",
        t.as_ref().map(|t| t.ko.to_string()).unwrap_or_else(|| "-".into()),
        t.as_ref()
            .and_then(|t| t.resolved_launch)
            .map(|v| format!("{v:.1}"))
            .unwrap_or_else(|| "-".into()),
        t.as_ref()
            .and_then(|t| t.ticks_to_ko)
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".into()),
    );

    for i in 0..4 {
        let t = probe.strike(hit, NEAR, x);
        let pre = t
            .as_ref()
            .map(|t| t.at_strike.clone())
            .unwrap_or_else(|| "RESET_REFUSED".into());
        println!(
            "after_a_ko\t{i}\t{NEAR}\t{}\t{}\t{}\t{pre}",
            t.as_ref().map(|t| t.ko.to_string()).unwrap_or_else(|| "-".into()),
            t.as_ref()
                .and_then(|t| t.resolved_launch)
                .map(|v| format!("{v:.1}"))
                .unwrap_or_else(|| "-".into()),
            t.as_ref()
                .and_then(|t| t.ticks_to_ko)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "-".into()),
        );
    }
    println!(
        "# READ IT AS: if `before_any_ko` and `after_a_ko` disagree at {NEAR}%, the trial \
         is NOT independent and the differing column of `pre_state` names what carried over."
    );
}

fn run_probe() {
    const REFERENCE: &str = "player_robot_v3";
    const HEAVY: &str = ambition_demo_smash::SMASH_GEORGE_BOOUL;
    // Attackers spanning the roster rather than all 21: the question is whether
    // ROLES separate, and every fighter authors the same role set.
    let attackers = ["npc_pirate_admiral", "smash_george_booul", "npc_bob"];
    let max_percent = 300;

    // ⭐ ONE MATCHUP PER PROCESS. The six cells of the matrix share nothing —
    // separate Apps, separate worlds — so running them as six processes is pure
    // wall-clock division that cannot touch a single measured number. With no
    // arguments the tool still walks the whole matrix in one process.
    let argv: Vec<String> = std::env::args().collect();
    let after: Vec<&str> = argv
        .iter()
        .skip_while(|a| a.as_str() != "probe")
        .skip(1)
        .map(|s| s.as_str())
        .collect();
    let pairs: Vec<(String, String)> = if after.len() >= 2 {
        vec![(after[0].to_string(), after[1].to_string())]
    } else {
        let mut v = Vec::new();
        for victim in [REFERENCE, HEAVY] {
            for a in attackers {
                v.push((a.to_string(), victim.to_string()));
            }
        }
        v
    };

    println!("# ko_envelope PROBE — stage outcomes, 1.25 frozen, no authored value changed");
    // ⛔⛔ EVERY KO PERCENT BELOW IS A NO-RECOVERY LOWER BOUND, and reading one
    // as a kill percent overstates the game's lethality.
    //
    // Both bodies carry `Brain::stand_still()`. The victim therefore never DIs,
    // never jumps, never air-dodges and never uses a recovery move — it is
    // launched and it travels until something stops it. A real victim fights the
    // launch, so its true threshold is HIGHER than the number in these columns,
    // by an amount this instrument does not measure. What the columns ARE good
    // for is comparison between cells measured the same way: role against role,
    // centre against ledge, reference weight against heavy.
    println!(
        "# victim brain: stand_still — no DI, no jump, no recovery. KO% is a \
         NO-RECOVERY LOWER BOUND, not a kill percent."
    );
    println!("# attacker meter pinned to 0 so rage_scale cannot multiply a resolved launch.");
    // ⛔⛔ AERIAL ROLES ARE MEASURED UNDER CONDITIONS THEY NEVER OCCUR IN, and
    // their rows are NOT comparable with the grounded ones.
    //
    // Every trial parks the victim standing on the platform, so an `attack_air*`
    // pulse is being fired at a grounded body from a grounded attacker. Run 2
    // duly reported `centre_never = n/n` for attack_air, attack_air_up,
    // attack_air_down and attack_dash. That is a statement about the FIXTURE,
    // not about aerials: a real aerial connects with an airborne victim who has
    // no floor to be driven into and no landing to absorb the launch.
    //
    // ⇒ Read the grounded roles (attack*, smash*, tilt*) as measurements and the
    // aerial roles as not-yet-measured. Giving them an airborne victim is a
    // separate fixture, not a tweak to this one.
    println!(
        "# ⛔ AERIAL ROLES (attack_air*, attack_dash) FIRE AT A GROUNDED, PARKED VICTIM — \
         a fixture they never meet in play. Their cells measure the fixture, not the move."
    );

    {
        for (attacker_id, victim_id) in &pairs {
            let (attacker_id, victim_id) = (attacker_id.as_str(), victim_id.as_str());
            let mut probe = KoProbe::new(attacker_id, victim_id);
            println!(
                "\n# {attacker_id} vs {victim_id}  weight={:.2}  tumble_speed={:.1}",
                probe.victim_weight, probe.tumble_speed
            );

            // Build this attacker's bound strike pulses from the runtime contract.
            let launchers = {
                let world = probe.app.world();
                let registry = world
                    .get_resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>()
                    .expect("registry");
                let Some(prepared) = registry.get(attacker_id) else {
                    println!("#   NOT IN REGISTRY — skipped");
                    continue;
                };
                let Some(contract) = prepared.kit.projectable_moveset() else {
                    println!("#   no projectable moveset — skipped");
                    continue;
                };
                let mut bad = Vec::new();
                let mut nl = Vec::new();
                launchers_of(attacker_id, contract, &mut bad, &mut nl)
                    .into_iter()
                    .filter(|l| matches!(l, Launcher::Strike { verbs, .. } if !verbs.is_empty()))
                    .collect::<Vec<_>>()
            };

            // ⭐ THE SELF-TEST, AND RUN 1 PROVED THE OLD ONE INADEQUATE.
            //
            // It fired one pulse at one percent twice and compared only `ko`.
            // That passed — and the table it admitted contained cells where a
            // percent measured as a survival in the coarse sweep came back a KO
            // in verification, because every trial was starting AIRBORNE and the
            // outcome turned on where a single update left the body.
            //
            // ⛔ COMPARE THE WHOLE OUTCOME, NOT THE VERDICT. Two trials can
            // agree on `ko` and still have begun in different states, which is
            // the disagreement that matters; `at_strike` is what catches it.
            // Three repetitions rather than two, because an alternating defect
            // (survive/KO/survive/KO — exactly what run 1 showed) is invisible
            // to any even-numbered comparison of the first two.
            if let Some(Launcher::Strike { hit, .. }) = launchers.first() {
                let mut seen = Vec::new();
                for _ in 0..3 {
                    seen.push(match probe.strike(hit, 100, probe.centre) {
                        Some(t) => format!(
                            "ko={} launch={} {}",
                            t.ko,
                            t.resolved_launch
                                .map(|v| format!("{v:.1}"))
                                .unwrap_or_else(|| "-".into()),
                            t.at_strike
                        ),
                        None => "RESET_REFUSED".to_string(),
                    });
                }
                println!("#   repeatability@100% x3:");
                for s in &seen {
                    println!("#     {s}");
                }
                if seen.iter().any(|s| s != &seen[0]) {
                    println!(
                        "#   ⛔ TRIALS ARE NOT INDEPENDENT — the same pulse at the same percent \
                         gave different results, so every threshold below would be an artefact \
                         of trial order. Table suppressed."
                    );
                    continue;
                }
            }

            println!(
                "role\tmove\tw/v\tbase\tgrowth\tlaunch@100\ttumble%\tcentre_KO%\tledge_KO%\tko_ticks"
            );
            // ⛔⛔ DERIVED FROM THE PLATFORM, AND INBOARD OF ITS EDGE.
            //
            // `centre + 240.0` parked the victim at x=560, which is EXACTLY
            // where the floor ends: `smash_stage` builds one solid at
            // min=(80,300) size=(480,32), so the platform spans 80..560. A body
            // standing on the last pixel decides `on_ground` sub-pixel, and it
            // showed — 15 of run 2's 16 self-contradicting cells were LEDGE
            // cells, and four ledge thresholds collapsed to 0-25%. That is the
            // fixture tipping a body off a cliff it was already teetering on,
            // not a measurement of knockback.
            //
            // Read the real edge off the stage and stand a margin inboard, so the
            // cell measures a launch from NEAR the ledge instead of a coin flip
            // about whether the victim was ever standing.
            let ledge_x = {
                let room = ambition_demo_smash::smash_stage();
                let right = room
                    .world
                    .blocks
                    .iter()
                    .map(|b| b.aabb.max.x)
                    .fold(f32::MIN, f32::max);
                if right > f32::MIN {
                    right - 40.0
                } else {
                    probe.centre + 200.0
                }
            };
            println!("#   ledge_x = {ledge_x:.1} (derived platform edge, 40px inboard)");
            for l in &launchers {
                let Launcher::Strike { hit, move_id, .. } = l else {
                    continue;
                };
                // ⛔ THROUGH `launch_at` LIKE THE OTHER TWO. This value is both
                // the printed column AND the midpoint the linearity check tests
                // the fitted line against, so a single refused reset here does
                // not merely blank a cell — it disarms the check that decides
                // whether the tumble crossing may be solved at all.
                let launch_at_100 = probe.launch_at(hit, 100, probe.centre);
                // ⭐ THE TUMBLE CROSSING IS SOLVED, NOT SWEPT — and the solve
                // CHECKS ITS OWN PREMISE instead of assuming it.
                //
                // `scaled_knockback` is linear in victim percent, so three
                // samples answer what a 13-trial sweep answered: two fix the
                // line, the third must land on it. Measured against the slow
                // path's own jab row (base 55.0, growth 1.10): l(0)=55.0,
                // l(200)=330.0, midpoint 192.5 — and the engine reported exactly
                // 192.5 at 100%, which is what makes the line trustworthy here.
                //
                // ⛔ If the third point misses, this prints NONLINEAR and no
                // crossing, because a fitted line through a curve that is not
                // one is a fabricated number.
                let l0 = probe.launch_at(hit, 0, probe.centre);
                let l200 = probe.launch_at(hit, 200, probe.centre);
                let tumble_cell = match (l0, launch_at_100, l200) {
                    (Some(a), Some(mid), Some(b)) => {
                        let predicted = (a + b) / 2.0;
                        if (predicted - mid).abs() > 0.5 {
                            format!("NONLINEAR({a:.1}/{mid:.1}/{b:.1})")
                        } else if probe.tumble_speed <= 0.0 {
                            "no-tumble-speed".to_string()
                        } else if a >= probe.tumble_speed {
                            "0".to_string()
                        } else {
                            let slope = (b - a) / 200.0;
                            if slope <= 0.0 {
                                format!(">{max_percent}")
                            } else {
                                let cross = ((probe.tumble_speed - a) / slope).ceil() as i32;
                                if cross > max_percent {
                                    format!(">{max_percent}")
                                } else {
                                    cross.to_string()
                                }
                            }
                        }
                    }
                    _ => "-".to_string(),
                };
                let centre_ko = probe.ko_threshold(hit, probe.centre, max_percent);
                let ledge_ko = probe.ko_threshold(hit, ledge_x, max_percent);
                let (w, v) = match l {
                    Launcher::Strike { window, volume, .. } => (*window, *volume),
                    _ => (0, 0),
                };
                // ⭐ LAUNCH-MOTION TIMING, which is a named suspect in this
                // investigation and was being recorded and thrown away — the
                // compiler said so ("field `ticks_to_ko` is never read"). Ticks
                // from contact to blast line at the cell's own threshold is how
                // "the launch feels slow" stops being a matter of opinion.
                // ⛔ BY REFERENCE: `Threshold::NonMonotonic` carries a `Vec`, so
                // this enum is not `Copy` and matching it by value would move
                // `centre_ko` out from under the `centre_ko.cell()` below.
                let ko_ticks = match &centre_ko {
                    Threshold::At(p) => probe
                        .strike(hit, *p, probe.centre)
                        .and_then(|t| t.ticks_to_ko)
                        .map(|t| t.to_string())
                        .unwrap_or_else(|| "-".into()),
                    _ => "-".to_string(),
                };
                println!(
                    "{}\t{move_id}\tw{w}v{v}\t{:.1}\t{}\t{}\t{}\t{}\t{}\t{}",
                    l.role(),
                    hit.knockback,
                    hit.knockback_growth
                        .map(|g| format!("{g:.2}"))
                        .unwrap_or_else(|| "None".into()),
                    launch_at_100
                        .map(|v| format!("{v:.1}"))
                        .unwrap_or_else(|| "-".into()),
                    tumble_cell,
                    centre_ko.cell(),
                    ledge_ko.cell(),
                    ko_ticks,
                );
            }
        }
    }
}

fn main() {
    // ⭐ BEFORE `probe`, because a table measured with a non-independent trial is
    // worth less than no table: run 1 produced one and its oracle row had moved.
    if std::env::args().any(|a| a == "contact") {
        run_contact();
        return;
    }
    if std::env::args().any(|a| a == "determinism") {
        run_determinism();
        return;
    }
    if std::env::args().any(|a| a == "probe") {
        run_probe();
        return;
    }
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    // ⛔ THE REGISTRY IS FILLED BY A `Startup` SYSTEM. A build that has never
    // updated has a catalog and NO REGISTRY AT ALL, so every id would miss and
    // the census would read as an empty roster.
    for _ in 0..4 {
        app.update();
    }

    let world = app.world();
    let registry = world
        .get_resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>()
        .expect("the composed host has a prepared-character registry");
    // ⛔⛤ **NOT THE LIVE RESOURCE — AND THE FIRST RUN OF THIS TOOL PROVED WHY.**
    // `project_combat_rules` folds `DeclaredCombatRules` every tick, but the
    // SMASH declaration only exists once the Smash experience is entered. A
    // census that merely boots the app host and reads the resolved resource gets
    // `Default`: measured 2026-09-13, this printed
    // `victim_percent_knockback_scale=1 knockback_growth=0 rage_max_scale=1
    // stale_floor=1 stale_knockback_influence=1` where Smash declares
    // `1.25 / 0.02 / 1.4 / 0.55 / 0.30`. The `knockback_growth=0` then collapsed
    // every unauthored-growth volume's effective growth to ZERO.
    //
    // ⇒ Ask the DECLARATION, which is the authority and is `pub`, and fold it
    // the same way the engine does. The stage probe additionally asserts this
    // against the live resource once a real Smash match exists — a declaration
    // and a running world agreeing is a measurement; either alone is a claim.
    let declared = ambition_demo_smash::smash_declared_combat_rules();
    let rules = ambition_platformer2d::combat::rules::ResolvedCombatTuning::resolve(
        Some(declared),
        0.0,
        false,
    );
    assert!(
        rules.knockback_growth > 0.0,
        "the folded Smash ruleset reports knockback_growth={} — a zero fallback silently \
         reads every unauthored growth as zero, which is the defect this line replaced",
        rules.knockback_growth
    );

    // ⭐ THE POPULATION IS THE SHIPPED HOST'S OWN ANSWER, printed so the census
    // can never be read as covering a cast it did not cover.
    let roster = ambition_demo_smash::select::SmashRoster::assemble(registry);
    let ids: Vec<String> = roster.ids().map(|s| s.to_string()).collect();
    println!("# ko_envelope — launcher-pulse census");
    println!("# roster (SmashRoster::assemble, the seatable cast): {} fighters", ids.len());
    for id in &ids {
        println!("#   {id}");
    }
    println!("# ruleset: victim_percent_knockback_scale={} knockback_growth(fallback)={} \
              rage_max_scale={} stale_floor={} stale_knockback_influence={}",
        rules.victim_percent_knockback_scale,
        rules.knockback_growth,
        rules.rage_max_scale,
        rules.stale_floor,
        rules.stale_knockback_influence,
    );
    println!("# THROW ROAD ASYMMETRY (current engine facts, not endorsements):");
    println!("#   throws: percent_scale=YES  staling=NO  rage=NO  damage applied BEFORE launch");
    println!("#   strikes: percent_scale=YES staling=YES rage=YES");

    let mut all: Vec<Launcher> = Vec::new();
    let mut bad: Vec<Malformed> = Vec::new();
    let mut non_launching: Vec<NonLaunching> = Vec::new();
    for id in &ids {
        let Some(prepared) = registry.get(id) else {
            bad.push(Malformed {
                fighter: id.clone(),
                what: "registry".into(),
                why: "on the assembled roster but absent from the registry".into(),
            });
            continue;
        };
        let Some(contract) = prepared.kit.projectable_moveset() else {
            bad.push(Malformed {
                fighter: id.clone(),
                what: "moveset".into(),
                why: "prepared with no projectable moveset".into(),
            });
            continue;
        };
        all.extend(launchers_of(id, contract, &mut bad, &mut non_launching));
    }

    println!("\nfighter\tmove_id\tkind\trole\tverbs\twindow\tvolume\tdamage\tbase_kb\tauthored_growth\teffective_growth\tgrowth_over_base\tfixed\tlaunch_dir\tkey");
    for l in &all {
        let (w, v, verbs) = match l {
            Launcher::Strike {
                window,
                volume,
                verbs,
                ..
            } => (window.to_string(), volume.to_string(), verbs.join("|")),
            Launcher::Throw { verb, .. } => ("-".into(), "-".into(), verb.clone()),
        };
        let eg = l.effective_growth(rules.knockback_growth);
        let base = l.base_knockback();
        let ratio = if base > 0.0 { eg / base } else { f32::NAN };
        let dir = l
            .launch_dir()
            .map(|(x, y)| format!("({x:.2},{y:.2})"))
            .unwrap_or_else(|| "default".into());
        println!(
            "{}\t{}\t{}\t{}\t{}\t{w}\t{v}\t{}\t{:.1}\t{}\t{:.4}\t{:.4}\t{}\t{dir}\t{}",
            l.fighter(),
            l.move_id(),
            l.kind(),
            l.role(),
            verbs,
            l.damage(),
            base,
            l.authored_growth()
                .map(|g| format!("{g:.3}"))
                .unwrap_or_else(|| "None".into()),
            eg,
            ratio,
            l.is_fixed(),
            l.key(),
        );
    }

    // ⭐⭐ BOUND ROLES ONLY, SIDE BY SIDE — the one table the whole instrument
    // exists to put in front of a reader.
    //
    // ⛔ UNBOUND PULSES ARE EXCLUDED HERE AND COUNTED SEPARATELY. 115 of 532
    // strike pulses answer to NO verb (`attack`, `attack_up`, `attack_air*` and
    // friends, identical across 15 fighters): a press cannot reach them, so
    // letting them into a role distribution drags every median toward a
    // stand-in's numbers while looking like content.
    //
    // ⛔ AND NO SPREAD RATIO IS PRINTED. Some authored pulses have a genuinely
    // zero growth/base — fixed knockback is a real authored role — so a
    // max/min ratio over this set divides by a legitimate zero. Min, median and
    // max say what a ratio would have, without being undefined.
    {
        let mut bound: BTreeMap<String, Vec<&Launcher>> = BTreeMap::new();
        let mut unbound = 0usize;
        for l in &all {
            match l {
                Launcher::Strike { verbs, .. } if verbs.is_empty() => unbound += 1,
                _ => bound.entry(l.role()).or_default().push(l),
            }
        }
        println!("\n# BOUND-ROLE COMPARISON (reachable pulses only)");
        println!("# excluded: {unbound} unbound strike pulses answering to no verb");
        println!("role\tn\tbase_med\tgrowth_med\tratio_min\tratio_med\tratio_max");
        for (role, ls) in &bound {
            let bases = dist(ls.iter().map(|l| l.base_knockback()).collect());
            let growths = dist(
                ls.iter()
                    .map(|l| l.effective_growth(rules.knockback_growth))
                    .collect(),
            );
            let ratios = dist(
                ls.iter()
                    .filter(|l| l.base_knockback() > 0.0)
                    .map(|l| l.effective_growth(rules.knockback_growth) / l.base_knockback())
                    .collect(),
            );
            println!(
                "{role}\t{}\t{:.1}\t{:.3}\t{:.4}\t{:.4}\t{:.4}",
                ls.len(),
                bases.median,
                growths.median,
                ratios.min,
                ratios.median,
                ratios.max
            );
        }
    }

    // ROLE SUMMARIES — the shape a gameplay question can actually be asked of.
    let mut by_role: BTreeMap<String, Vec<&Launcher>> = BTreeMap::new();
    for l in &all {
        by_role.entry(l.role()).or_default().push(l);
    }
    println!("\n# role summaries (n / min / p25 / median / p75 / max)");
    println!("role\tcol\tn\tmin\tp25\tmedian\tp75\tmax");
    for (role, ls) in &by_role {
        for (col, xs) in [
            ("base_kb", ls.iter().map(|l| l.base_knockback()).collect::<Vec<_>>()),
            (
                "effective_growth",
                ls.iter().map(|l| l.effective_growth(rules.knockback_growth)).collect(),
            ),
            (
                "growth_over_base",
                ls.iter()
                    .filter(|l| l.base_knockback() > 0.0)
                    .map(|l| l.effective_growth(rules.knockback_growth) / l.base_knockback())
                    .collect(),
            ),
            ("damage", ls.iter().map(|l| l.damage() as f32).collect()),
        ] {
            let d = dist(xs);
            println!(
                "{role}\t{col}\t{}\t{:.4}\t{:.4}\t{:.4}\t{:.4}\t{:.4}",
                d.n, d.min, d.p25, d.median, d.p75, d.max
            );
        }
    }

    println!("\n# totals: {} launchers ({} strikes, {} throws) over {} fighters",
        all.len(),
        all.iter().filter(|l| matches!(l, Launcher::Strike { .. })).count(),
        all.iter().filter(|l| matches!(l, Launcher::Throw { .. })).count(),
        ids.len(),
    );
    if non_launching.is_empty() {
        println!("# non-launching capture slots: none");
    } else {
        println!(
            "# NON-LAUNCHING CAPTURE SLOTS: {} (authored carries — NOT gaps, and not to be \
             \"fixed\" into throws)",
            non_launching.len()
        );
        for c in &non_launching {
            println!("#   {} · {} · {} · CAPTURE_CARRY", c.fighter, c.slot, c.move_id);
        }
    }
    if bad.is_empty() {
        println!("# malformed rows: none");
    } else {
        println!("# MALFORMED ROWS: {}", bad.len());
        for m in &bad {
            println!("#   {} · {} · {}", m.fighter, m.what, m.why);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Boot the composed host once and hand back the registry's view.
    fn composed() -> bevy::prelude::App {
        let mut app = ambition_app::app::build_visible_app(
            ambition_app::app::VisibleRenderMode::NoWindow,
            true,
        );
        for _ in 0..4 {
            app.update();
        }
        app
    }

    /// ⛔ THE CENSUS POPULATION IS A CLAIM. An empty roster would make every
    /// downstream distribution vacuously "fine".
    #[test]
    fn the_composed_smash_roster_is_not_empty() {
        let app = composed();
        let registry = app
            .world()
            .get_resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>()
            .expect("registry");
        let roster = ambition_demo_smash::select::SmashRoster::assemble(registry);
        let n = roster.ids().count();
        assert!(
            n > 0,
            "SmashRoster::assemble produced NO seatable fighters, so a census over it \
             would report an empty game rather than a defect"
        );
    }

    /// ⭐⭐ THIS TEST REPLACES EVERY GREP THIS FILE'S HEADER DESCRIBES.
    ///
    /// It asserts the thing four source parsers got wrong: the roster's fighters
    /// really do author throws, reachable by VERB through the runtime contract.
    #[test]
    fn every_resolved_throw_verb_carries_exactly_one_hydratable_throw_effect() {
        let app = composed();
        let registry = app
            .world()
            .get_resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>()
            .expect("registry");
        let roster = ambition_demo_smash::select::SmashRoster::assemble(registry);

        let mut resolved = 0usize;
        let mut carries = 0usize;
        let mut fighters_with_throws = 0usize;
        for id in roster.ids() {
            let Some(prepared) = registry.get(id) else {
                continue;
            };
            let Some(contract) = prepared.kit.projectable_moveset() else {
                continue;
            };
            let mut mine = 0usize;
            for (slot, verb) in THROW_VERBS {
                let Some(spec) = contract.move_for_verb(verb) else {
                    continue;
                };
                let throws: Vec<_> = spec
                    .effect_refs()
                    .into_iter()
                    .filter(|(_, e)| e.key == CAPTURE_THROW)
                    .collect();
                let carried = spec
                    .effect_refs()
                    .into_iter()
                    .filter(|(_, e)| e.key == CAPTURE_CARRY)
                    .count();
                // ⛔ A CARRY IS NOT A DEFECT. The goblin's down press hoists
                // instead of launching, on purpose, guarded by
                // `the_goblins_down_throw_hauls_instead_of_launching`. The first
                // version of this arm asserted `throws.len() == 1` outright and
                // reddened on a CORRECT fighter — a repair from here would have
                // broken the game to green the instrument.
                if throws.is_empty() && carried > 0 {
                    carries += 1;
                    continue;
                }
                assert_eq!(
                    throws.len(),
                    1,
                    "`{id}`'s {slot} resolves to `{}` carrying {} `{CAPTURE_THROW}` and \
                     {carried} `{CAPTURE_CARRY}` effects. A capture slot is a throw (exactly \
                     one throw effect) or a carry (at least one carry effect and no throw); \
                     this is neither.",
                    spec.id,
                    throws.len()
                );
                throws[0]
                    .1
                    .params
                    .hydrate::<CaptureThrowParams>()
                    .unwrap_or_else(|e| {
                        panic!("`{id}`'s {slot} params did not hydrate as CaptureThrowParams: {e}")
                    });
                mine += 1;
                resolved += 1;
            }
            if mine > 0 {
                fighters_with_throws += 1;
            }
        }
        // ⭐ THE FLOOR IS THE POINT. Without it this arm passes on a roster where
        // every lookup silently returns `None` — which is precisely the
        // conclusion four source-parsing attempts reached, and every one of them
        // was wrong. A bare "no malformed rows" cannot tell an empty census from
        // a healthy one.
        assert!(
            resolved >= 20,
            "only {resolved} throws resolved across the whole roster ({carries} carries). \
             Every fighter authors a capture kit, so a number this small is a BROKEN LOOKUP \
             rather than a game without throws."
        );
        println!("{resolved} throws + {carries} carries over {fighters_with_throws} fighters");
    }

    /// `Some(0.0)` is FIXED knockback, and it must not be folded into "the
    /// volume did not decide". Reading a bare `0.0` as unspecified once made the
    /// documented fixed-knockback case the one value nobody could author.
    #[test]
    fn a_zero_growth_stays_fixed_and_an_absent_growth_takes_the_ruleset_fallback() {
        let fixed = Launcher::Throw {
            fighter: "f".into(),
            move_id: "m".into(),
            verb: CAPTURE_THROW_UP_VERB.into(),
            slot: "up_throw",
            params: CaptureThrowParams {
                damage: 6,
                knockback: 145.0,
                knockback_growth: 0.0,
                launch_dir: (0.0, -1.0),
            },
        };
        assert!(fixed.is_fixed(), "an authored 0.0 growth is fixed knockback");
        assert_eq!(
            fixed.effective_growth(0.02),
            0.0,
            "a FIXED throw must not silently acquire the ruleset's fallback growth"
        );

        let unauthored = Launcher::Strike {
            fighter: "f".into(),
            move_id: "m".into(),
            verbs: vec!["attack".into()],
            window: 0,
            volume: 0,
            hit: HitVolume {
                knockback_growth: None,
                ..sample_volume()
            },
        };
        assert!(!unauthored.is_fixed(), "`None` is not fixed knockback");
        assert!(
            (unauthored.effective_growth(0.02) - 100.0 * 0.02).abs() < 1e-6,
            "an unauthored growth resolves to base × the ruleset's growth, not zero"
        );
    }

    /// A role must come from an AUTHORED BINDING, never from guessing at a name.
    #[test]
    fn a_strikes_role_comes_from_its_authored_verb_not_its_id() {
        let l = Launcher::Strike {
            fighter: "f".into(),
            move_id: "looks_like_a_jab".into(),
            verbs: vec!["attack".into(), "attack_forward_strong".into()],
            window: 0,
            volume: 0,
            hit: sample_volume(),
        };
        assert_eq!(
            l.role(),
            "attack_forward_strong",
            "the most specific authored binding names the role"
        );
        let unbound = Launcher::Strike {
            fighter: "f".into(),
            move_id: "orphan".into(),
            verbs: vec![],
            window: 0,
            volume: 0,
            hit: sample_volume(),
        };
        assert_eq!(
            unbound.role(),
            "unbound:orphan",
            "a move no verb names is reported as unbound rather than guessed at"
        );
    }

    #[test]
    fn a_distribution_reports_its_own_count() {
        let d = dist(vec![1.0, 2.0, 3.0, 4.0]);
        assert_eq!(d.n, 4);
        assert_eq!(d.min, 1.0);
        assert_eq!(d.max, 4.0);
        assert_eq!(dist(vec![]).n, 0, "an empty column reports n=0, not a value");
    }

    fn sample_volume() -> HitVolume {
        HitVolume {
            shape: ambition_entity_catalog::VolumeShape::Rect {
                offset: (0.0, 0.0),
                half_extents: (10.0, 10.0),
            },
            damage: 5,
            knockback: 100.0,
            knockback_growth: Some(2.0),
            launch_dir: None,
            reaction: None,
            on_hit: None,
            vfx: None,
            hit_sfx: None,
        }
    }
}
