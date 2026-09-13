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

fn main() {
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
