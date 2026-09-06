//! D199's deferral condition, as a CHECK rather than a sentence.
//!
//! ⭐⭐ THE SWEPT-VERSUS-HURT-VOLUME HALF OF D199 IS DEFERRED ON A MEASUREMENT,
//! not on taste. The projectile victim test is an ENDPOINT overlap, so a shot
//! fast enough to cross a body inside one 60Hz tick passes through it. That is
//! only a defect if content can produce such a shot, and on 2026-08-29 it could
//! not: the fastest authored projectile was 640 px/s = 10.67px per tick, while
//! tunnelling a 28px body needs **>1680 px/s — 2.6x anything authored**.
//!
//! ⛔⛔ A DEFERRAL NOBODY CHECKS BECOMES THE REAL RULE. The row says *"re-open
//! when a projectile exceeds ~1700 px/s"*, and until this file existed that
//! sentence was enforced by nothing: the day somebody authors a 2000 px/s shot,
//! the shot silently passes through its victims and every test stays green.
//! This is that sentence, executable.
//!
//! ⚠ **IT GUARDS THE SPEED HALF ONLY, AND SAYING SO IS THE POINT.** The other
//! input to the same inequality is the smallest thing a shot must not step over,
//! and that is the victim's published damage volume — which falls back to the
//! coarse body box, and the body box is `BodySource::SpriteAuthored`
//! (per-pose, resolved from the sheet) for most of the cast. It is not readable
//! from authored data, so a census of it here would be a number about the three
//! characters that author `hurtboxes` wearing the authority of a census over the
//! whole roster. ⇒ if that side ever moves — a hurt volume authored below ~11px
//! — this guard will NOT catch it, and D199's row is where that is written down.

use ambition_platformer2d::characters::brain::action_set::RangedActionSpec;

/// Per-tick displacement a shot may reach before the endpoint victim test can
/// step over an ordinary body.
///
/// ⭐ DERIVED, not chosen: `THRESHOLD_PX_PER_TICK` is the ~28px body D199
/// measured against, and the speed is what that is at the sim's own rate — so
/// changing `SIM_TICK_HZ` moves the ceiling with it instead of leaving a
/// hard-coded 1700 behind.
const SMALLEST_ORDINARY_BODY_PX: f32 = 28.0;

fn ceiling_px_per_s() -> f32 {
    SMALLEST_ORDINARY_BODY_PX * ambition_platformer2d::runtime::SIM_TICK_HZ as f32
}

/// The fastest a spec can launch, charge included.
///
/// ⛔ THE CHARGE MULTIPLIER IS PART OF THE AUTHORED SPEED. A 600 px/s shot with
/// `speed_mult: 3.0` is an 1800 px/s shot the moment somebody holds the button,
/// and reading `speed` alone would report it as comfortably inside the ceiling.
fn fastest_launch(spec: &RangedActionSpec) -> f32 {
    let charged = spec
        .charge
        .as_ref()
        .map(|charge| spec.speed * charge.speed_mult.max(1.0))
        .unwrap_or(spec.speed);
    spec.speed.max(charged)
}

/// Every named kind in the basic kit, by EXHAUSTIVE DESTRUCTURE.
///
/// ⛔ A hand-kept list here would silently stop covering the kit the day a
/// fourth variant lands. The `match` below has no wildcard, so adding one is a
/// compile error in this file.
fn every_named_kind() -> Vec<(&'static str, f32)> {
    use ambition_platformer2d::projectiles::ProjectileKind::*;
    [Fireball, Hadouken, HadoukenSuper]
        .into_iter()
        .map(|kind| {
            let name = match kind {
                Fireball => "Fireball",
                Hadouken => "Hadouken",
                HadoukenSuper => "HadoukenSuper",
            };
            (name, kind.speed())
        })
        .collect()
}

/// ⭐⭐ NO AUTHORED SHOT MAY OUTRUN THE ENDPOINT VICTIM TEST.
///
/// The population is the whole prepared registry, not a hand-listed cast: a
/// fast shot authored on a character nobody put on the Smash grid is still a
/// fast shot, and the row's condition is about what CONTENT can produce.
#[test]
fn no_authored_projectile_can_step_over_a_body_in_one_tick() {
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    app.update();
    let registry = app
        .world()
        .resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>();

    let ceiling = ceiling_px_per_s();
    let mut fastest: Vec<(String, f32)> = Vec::new();

    for id in registry.ids() {
        let Some(prepared) = registry.get(id) else {
            continue;
        };
        let Some(ranged) = prepared.kit.action_set().and_then(|set| set.ranged.as_ref()) else {
            continue;
        };
        fastest.push((id.to_string(), fastest_launch(ranged)));
    }
    for (name, speed) in every_named_kind() {
        fastest.push((format!("ProjectileKind::{name}"), speed));
    }

    // ⚠ THE POPULATION BESIDE THE FINDING. "Nothing exceeds the ceiling" means
    // nothing at all if the sweep found no shots — which is exactly what a
    // renamed accessor or an empty registry would produce, silently.
    assert!(
        fastest.len() >= 4,
        "the sweep found only {} authored shot(s), which is too few to be a \
         census of the cast — the registry or the ranged accessor moved: {fastest:?}",
        fastest.len()
    );

    let offenders: Vec<String> = fastest
        .iter()
        .filter(|(_, speed)| *speed > ceiling)
        .map(|(who, speed)| {
            format!(
                "{who} launches at {speed} px/s = {:.2}px per tick",
                speed / ambition_platformer2d::runtime::SIM_TICK_HZ as f32
            )
        })
        .collect();

    let peak = fastest
        .iter()
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .expect("the sweep is non-empty");

    assert!(
        offenders.is_empty(),
        "D199's swept-projectile deferral has EXPIRED. A shot that moves more \
         than {SMALLEST_ORDINARY_BODY_PX}px in one {} Hz tick can cross a body \
         between two endpoint tests and deal no damage, and the projectile \
         victim test is still an endpoint overlap:\n  {}\n\n⇒ reopen D199's \
         swept-versus-hurt-volume half. The primitive it needs is already on \
         disk and tested: `ambition_geometry::CombatVolume::swept_aabb`. ⛔ Do \
         NOT buy the fix by testing the UNION of the start and end boxes — that \
         is exact only for axis-aligned travel and invents hits on bodies a \
         diagonal shot visually missed.",
        ambition_platformer2d::runtime::SIM_TICK_HZ,
        offenders.join("\n  "),
    );

    eprintln!(
        "[d199] {} authored shot(s) swept; fastest is {} at {} px/s ({:.2}px/tick), \
         ceiling {ceiling} px/s ({SMALLEST_ORDINARY_BODY_PX}px/tick)",
        fastest.len(),
        peak.0,
        peak.1,
        peak.1 / ambition_platformer2d::runtime::SIM_TICK_HZ as f32,
    );
}
