//! The primitives a character's move table is written with, shared so that a
//! new character does not start by copying another.
//!
//! This crate is a dependency of every demo, so demo characters (Mary-O,
//! Sanic) use the same helpers as the main content.
//!
//! There is one `strike`. A move-building fact is shared here; a game's policy
//! about moves (for example the `Feel` vocabulary, the repertoire and the
//! cancel conventions of `ambition_demo_smash`) is not.
//!
//! `strike` is startup, one active window with one volume, then recovery: the
//! shape of nearly every move in the genre.
//!
//! A move states what it is, never what a mode does with it. Startup, active
//! frames, recovery, hitbox geometry, damage, base launch and growth belong to
//! the swing. Percent, stocks, blast zones and DI belong to the ruleset. So one
//! table can read as Hollow-Knight combat in one game and a platform fighter in
//! another.

use crate::{
    CancelCondition, ClipBinding, EffectRef, HitVolume, ImpulseMode, MoveEvent, MoveEventKind,
    MoveGates, MoveSpec, MoveWindow, VolumeShape, WindowTag,
};

/// The sweep an arcing slash draws.
///
/// A move-building fact, so it lives with the builders: `strike` uses it by
/// default. `moveset_prefabs` imports it.
pub const SLASH_ARC_VFX: &str = "slash_arc";
/// The sweep a straight poke draws.
pub const SLASH_POKE_VFX: &str = "slash_poke";


fn event(mut m: MoveSpec, at_s: f32, kind: MoveEventKind) -> MoveSpec {
    m.events.push(MoveEvent { at_s, kind });
    m
}

/// A timed self-displacement.
///
/// [`ImpulseMode::Set`] commands a velocity; [`ImpulseMode::Add`] contributes
/// to one. A body falling at terminal velocity gets the same result from a
/// `Set` as a standing body, and the worst result from an `Add`. The catalog's
/// `lift_speed` / `lift_side` derivation counts only `Set`, because an `Add`
/// states no speed.
///
/// `local` is body-local: `+x` toward facing, `+y` toward the feet, so a rise
/// has a negative second component.
pub fn impulse(m: MoveSpec, at_s: f32, local: (f32, f32), mode: ImpulseMode) -> MoveSpec {
    event(m, at_s, MoveEventKind::Impulse { local, mode })
}

/// A timed gravity regime: the owner falls at `scale` times its usual gravity
/// for `seconds`, starting at this beat.
///
/// The move asks and the movement domain owns the regime, so the author gives
/// a duration, not an on/off pair. A move that owed an "off" would leak the
/// regime when interrupted, canceled or rolled back.
///
/// The regime outlives the move on purpose. `seconds` runs from this beat, so
/// a parasol opened during a 0.3s animation can hold its owner up for two
/// seconds. A window tag cannot outlast its move.
///
/// `scale` is a multiplier: `1.0` is a no-op and `0.0` is a hover. A
/// non-positive `seconds` clears the body's current modifier, so a second beat
/// in the same move can end the float early.
pub fn gravity_modifier(m: MoveSpec, at_s: f32, scale: f32, seconds: f32) -> MoveSpec {
    event(m, at_s, MoveEventKind::GravityModifier { scale, seconds })
}

/// A sound cue at a moment on the move's own timeline.
///
/// A [`vfx`] burst already plays its own sound; writing
/// `sfx(m, t, "vfx.<family>.<row>")` beside one plays it twice. For a sound
/// that is not the row's default, use [`vfx_cued`].
pub fn sfx(m: MoveSpec, at_s: f32, cue: &str) -> MoveSpec {
    event(
        m,
        at_s,
        MoveEventKind::Sfx {
            cue: cue.to_string(),
        },
    )
}

/// A visual burst at a moment, with its sound.
///
/// `effect` is the name of a row on one of the shipped FX spritesheets
/// (`ambition_sprite_sheet::fx`). `MoveSpec::presentation_problems` refuses a
/// name no sheet carries, and the renderer counts it as a miss.
///
/// The bank ships one `vfx.<family>.<row>` cue per authored row, so the name
/// that finds the clip also finds the sound: `dispatch_move_events` asks for
/// the pair. Do not add an [`sfx`] with the same cue, or the burst plays
/// twice. Use [`vfx_cued`] when the sound is not the row's default.
pub fn vfx(m: MoveSpec, at_s: f32, effect: &str) -> MoveSpec {
    event(
        m,
        at_s,
        MoveEventKind::Vfx {
            effect: effect.to_string(),
            at: (0.0, 0.0),
            scale: 1.0,
            sfx: None,
        },
    )
}

/// A burst that says where and how big, in the same body-local numbers the
/// move's strike volumes use.
///
/// Pass a volume's own `offset` as `at`, so the two cannot disagree.
pub fn vfx_at(m: MoveSpec, at_s: f32, effect: &str, at: (f32, f32), scale: f32) -> MoveSpec {
    event(
        m,
        at_s,
        MoveEventKind::Vfx {
            effect: effect.to_string(),
            at,
            scale,
            sfx: None,
        },
    )
}

/// A placed burst whose sound is not its own row's default.
///
/// `cue` is a bank cue name, not an effect row name. An id that neither the
/// registry nor the packed bank authorizes is counted and dropped, so a typo
/// here is silence, as for [`sfx`].
pub fn vfx_cued(
    m: MoveSpec,
    at_s: f32,
    effect: &str,
    at: (f32, f32),
    scale: f32,
    cue: &str,
) -> MoveSpec {
    event(
        m,
        at_s,
        MoveEventKind::Vfx {
            effect: effect.to_string(),
            at,
            scale,
            sfx: Some(cue.to_string()),
        },
    )
}

/// The contact sound for this move, applied to every volume it throws.
/// Contact feedback belongs to the volume because only the volume knows it
/// connected.
pub fn on_contact(mut m: MoveSpec, cue: &str) -> MoveSpec {
    for volume in m.windows.iter_mut().flat_map(|w| w.volumes.iter_mut()) {
        volume.hit_sfx = Some(cue.to_string());
    }
    m
}

/// How the swing itself is drawn: the strike-presentation tag on every volume
/// this move throws.
///
/// This is not an FX-sheet row name. `HitVolume::vfx` is a two-word
/// vocabulary ([`SLASH_ARC_VFX`] / [`SLASH_POKE_VFX`]) that the move runtime
/// reads twice: it picks the arc or jab shape drawn from the spawned volume,
/// and it makes a volume prefer the sprite manifest's authored hit polygon
/// over the synthetic box. A sheet row name here would silently disable both.
/// Per-move art is a [`vfx`] event.
///
/// [`strike`] tags every volume `slash_arc`, which is wrong for a poke; use
/// this for pokes.
pub fn strike_tag(mut m: MoveSpec, tag: &str) -> MoveSpec {
    for volume in m.windows.iter_mut().flat_map(|w| w.volumes.iter_mut()) {
        volume.vfx = Some(tag.to_string());
    }
    m
}

/// The proper-time instant a move's first hit becomes live. Where its feedback
/// belongs, and where a self-displacement usually does.
pub fn active_start(m: &MoveSpec) -> f32 {
    m.windows
        .iter()
        .find(|w| matches!(w.tag, WindowTag::Active))
        .map_or(0.0, |w| w.start_s)
}

/// The refusal shared by the window-pushing verbs ([`armor`],
/// [`armor_under`], [`invuln`], [`cancelable`]).
///
/// Each of those verbs pushes a window with no volumes, whose content is only
/// its tag and bounds. A zero-width window is invisible in the source and on
/// screen, so each verb must refuse it. One shared check keeps them equal.
///
/// # Panics
///
/// If the window would never be open.
// Public because every author owes this constraint; `moveset_prefabs` in
// `ambition_characters` calls it.
pub fn refuse_a_window_that_never_opens(
    id: &str,
    start_s: f32,
    end_s: f32,
    what: &str,
    consequence: &str,
) {
    assert!(
        end_s > start_s,
        "{what} on `{id}` runs {start_s}s..{end_s}s, which is never open — and a \
         window that never opens is invisible in play: {consequence}",
    );
}

/// A cancel window. The timeline is the cancel table, so a combo route is
/// authored here.
pub fn cancelable(
    mut m: MoveSpec,
    start_s: f32,
    end_s: f32,
    into: &[&str],
    condition: CancelCondition,
) -> MoveSpec {
    refuse_a_window_that_never_opens(
        &m.id,
        start_s,
        end_s,
        "a cancel window",
        "the follow-up the move advertises can never actually be taken",
    );
    m.windows.push(MoveWindow {
        start_s,
        end_s,
        tag: WindowTag::Cancelable {
            into: into.iter().map(|s| (*s).to_string()).collect(),
            condition,
        },
        volumes: Vec::new(),
        motion_scale: 1.0,
        sustain_effect: None,
    });
    m
}

/// An authored intangibility window: the owner cannot be hit between these
/// beats. `project_move_defense_windows` consumes the tag.
pub fn invuln(mut m: MoveSpec, start_s: f32, end_s: f32) -> MoveSpec {
    refuse_a_window_that_never_opens(
        &m.id,
        start_s,
        end_s,
        "an invulnerable window",
        "the move simply takes hits it looked like it should pass through",
    );
    m.windows.push(MoveWindow {
        start_s,
        end_s,
        tag: WindowTag::Invuln,
        volumes: Vec::new(),
        motion_scale: 1.0,
        sustain_effect: None,
    });
    m
}

/// Super armor: through this window the body is hit and does not react.
///
/// `MovePlayback` republishes `BodyCombat::armored` from the live window
/// every tick, and `hit_reaction` gates the launch on `!combat.armored`. <!-- cite-ok: records the deleted boolean `ArmorPolicy` replaced -->
///
/// Not invulnerability. An armored body takes the damage, but not the launch,
/// the hitstun or the recoil lock. So armor loses to chip damage and grabs
/// and wins the trade against one big hit; i-frames do the opposite.
pub fn armor(mut m: MoveSpec, start_s: f32, end_s: f32) -> MoveSpec {
    refuse_a_window_that_never_opens(
        &m.id,
        start_s,
        end_s,
        "an armour window",
        "the move simply loses trades it looked like it should win",
    );
    m.windows.push(MoveWindow {
        start_s,
        end_s,
        tag: WindowTag::Armor,
        volumes: Vec::new(),
        motion_scale: 1.0,
        sustain_effect: None,
    });
    m
}

/// Threshold armor: through this window the body is held through small hits
/// and reacts normally to a big one.
///
/// [`armor`] is all or nothing. `breaks_at` is the damage at which a hit gets
/// through. The comparison is `>=`, so to survive 9 and break on 10, author
/// `10`.
///
/// Nothing accumulates: each hit is judged alone, so two 6s never break it.
/// This also needs no rollback state for remaining armor.
///
/// Armor does not change the damage taken. The threshold decides only who
/// keeps trajectory and control.
pub fn armor_under(mut m: MoveSpec, start_s: f32, end_s: f32, breaks_at: i32) -> MoveSpec {
    refuse_a_window_that_never_opens(
        &m.id,
        start_s,
        end_s,
        "a threshold-armour window",
        "the move simply loses trades it looked like it should win",
    );
    m.windows.push(MoveWindow {
        start_s,
        end_s,
        tag: WindowTag::ArmorUnder { damage: breaks_at },
        volumes: Vec::new(),
        motion_scale: 1.0,
        sustain_effect: None,
    });
    m
}

/// Fixed knockback: this hit launches the same at 0% and at 200%.
///
/// [`strike`] takes one `f32` and treats zero as "the stage decides", so it
/// cannot author `Some(0.0)`. Use this in place of editing
/// `windows[..].volumes[..]` by hand.
///
/// Use it for moves that must land the same every time: a multi-hit pulse
/// whose carry must hold at high percent, or a hold that holds everyone
/// equally.
pub fn fixed_knockback(mut m: MoveSpec) -> MoveSpec {
    for volume in m.windows.iter_mut().flat_map(|w| w.volumes.iter_mut()) {
        volume.knockback_growth = Some(0.0);
    }
    m
}

/// A conditional technique on contact: the engine's `on_hit` seam, applied to
/// every volume the move lands.
///
/// It states what the landing can do, not what a game does with it: the
/// down-air says it can rebound its attacker, and the ruleset decides whether
/// to use that or read the swing as a spike. Compare [`on_contact`] (a sound)
/// and [`strike_tag`] (how the swing draws).
pub fn on_hit(mut m: MoveSpec, key: &str) -> MoveSpec {
    for volume in m.windows.iter_mut().flat_map(|w| w.volumes.iter_mut()) {
        volume.on_hit = Some(EffectRef::new(key));
    }
    m
}

/// A tail the body cannot steer out of. Extends the move to `to_s` with a
/// Recovery window whose `motion_scale` damps the owner's steering. It is
/// enforced body-side, so it binds a CPU and a human the same way.
pub fn committed_tail(mut m: MoveSpec, to_s: f32, motion_scale: f32) -> MoveSpec {
    let from = m.duration_s;
    if to_s <= from {
        return m;
    }
    m.windows.push(MoveWindow {
        start_s: from,
        end_s: to_s,
        tag: WindowTag::Recovery,
        volumes: Vec::new(),
        motion_scale,
        sustain_effect: None,
    });
    m.duration_s = to_s;
    m
}

/// A taunt: the one authored move that threatens nobody.
///
/// No volume, no impulse, one rooted recovery window: the owner cannot act for
/// `duration_s`. Compose `sfx` / `vfx` onto the result like any other move.
pub fn taunt(id: &str, duration_s: f32) -> MoveSpec {
    MoveSpec {
        display_name: None,
        id: id.to_string(),
        clip: ClipBinding {
            clip: "taunt".to_string(),
            // A sheet with no taunt row stands still, which suits a taunt.
            fallbacks: vec!["idle".to_string()],
        },
        duration_s,
        windows: vec![MoveWindow {
            start_s: 0.0,
            end_s: duration_s,
            tag: WindowTag::Recovery,
            volumes: Vec::new(),
            // Rooted: a taunt you can walk out of is not a commitment.
            motion_scale: 0.0,
            sustain_effect: None,
        }],
        events: Vec::new(),
        gates: MoveGates::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
        smash_charge: None,
        charge_gesture: crate::ChargeGesture::default(),
        repeat: None,
        landing_lag_s: None,
        autocancel_after_s: None,
        sprite_spin_hz: None,
        equips: None,
        flow: None,
    }
}

/// A special whose content is only its effect: a timeline, a clip, and no
/// volume.
///
/// A summon, a transformation, a counter-stance or a teleport has a technique
/// as its payload, not a box. Through [`strike`] it would need an active
/// window with an empty volume list, which reads as "this hits, for nothing".
///
/// The timeline is `Startup` up to `commits_at_s` and `Recovery` after: the
/// windup you can be punished during, then the tail you owe. Rooted
/// throughout.
pub fn hitless_special(id: &str, clip: &str, commits_at_s: f32, duration_s: f32) -> MoveSpec {
    assert!(
        commits_at_s <= duration_s,
        "special `{id}` commits at {commits_at_s}s but lasts {duration_s}s"
    );
    MoveSpec {
        display_name: None,
        id: id.to_string(),
        clip: ClipBinding {
            clip: clip.to_string(),
            fallbacks: vec!["idle".to_string()],
        },
        duration_s,
        windows: vec![
            MoveWindow {
                start_s: 0.0,
                end_s: commits_at_s,
                tag: WindowTag::Startup,
                volumes: Vec::new(),
                motion_scale: 0.0,
                sustain_effect: None,
            },
            MoveWindow {
                start_s: commits_at_s,
                end_s: duration_s,
                tag: WindowTag::Recovery,
                volumes: Vec::new(),
                motion_scale: 0.0,
                sustain_effect: None,
            },
        ],
        events: Vec::new(),
        gates: MoveGates::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
        smash_charge: None,
        charge_gesture: crate::ChargeGesture::default(),
        repeat: None,
        landing_lag_s: None,
        autocancel_after_s: None,
        sprite_spin_hz: None,
        equips: None,
        flow: None,
    }
}

#[allow(clippy::too_many_arguments)]
/// The dash attack's shape, with the fighter supplying only what it hits for.
///
/// The helper owns the shape (fast startup, long recovery, forward carry) and
/// the fighter owns the numbers, as in [`taunt`].
///
/// The frame shape is the mechanic: it starts faster than a tilt because the
/// dash is already a commitment, and it recovers longer because that is the
/// price. With a tilt's recovery it would be a strictly better tilt.
pub fn dash_attack(id: &str, shape: DashAttackShape, damage: i32, knockback: f32) -> MoveSpec {
    // The impulse is forward and lands at the swing, so the move carries the
    // dash's momentum.
    impulse(
        strike(Strike {
            id,
            clip: "dash_attack",
            startup_s: shape.startup_s,
            active_s: shape.active_s,
            recover_s: shape.recover_s,
            // `reach_px` is what `reach_of` measures: offset plus half-extent,
            // not the offset alone. A fighter test can then pin a reach (for
            // example Carl's `NEAREST_REACH`) with the same number.
            offset: (shape.reach_px * 0.6, -2.0),
            half_extents: (shape.reach_px * 0.4, 20.0),
            damage,
            knockback,
            knockback_growth: 1.5,
            launch_dir: Some((0.92, -0.39)),
            on_hit: None,
        }),
        shape.startup_s,
        (260.0, 0.0),
        ImpulseMode::Add,
    )
}

/// The frames a dash attack occupies, so a fighter with its own timing
/// invariant can keep it.
///
/// [`DashAttackShape::GENRE`] is still the standard dash attack, and most
/// fighters use it. A fighter whose own tests assert a property the genre
/// numbers break (for example a tolerance band, a recovery ratio, a
/// reach-monotonic line) authors its own shape. A shared default must not
/// override a character's own invariant.
#[derive(Clone, Copy, Debug)]
pub struct DashAttackShape {
    pub startup_s: f32,
    pub active_s: f32,
    pub recover_s: f32,
    pub reach_px: f32,
}

impl DashAttackShape {
    /// The genre's dash attack: faster than a tilt because the dash is already
    /// a commitment, and a longer recovery because that is the price.
    pub const GENRE: Self = Self {
        startup_s: 0.05,
        active_s: 0.09,
        recover_s: 0.26,
        reach_px: 40.0,
    };
}

/// One strike, as named values.
///
/// Named fields, not positional arguments. Several timings and knockback
/// values are all `f32`, and `offset` and `half_extents` are both
/// `(f32, f32)`, so a transposition would silently change a fighter's feel.
///
/// It is not `Serialize`: a move file states the `MoveSpec` a strike builds,
/// so this record has no wire shape to freeze.
#[derive(Debug, Clone, PartialEq)]
pub struct Strike<'a> {
    /// The move id. Unique within the kit.
    pub id: &'a str,
    /// The animation row. Falls back through `attack_side` → `attack` → `slash`
    /// → `idle`, so a missing clip never costs the move its gameplay.
    pub clip: &'a str,
    /// The tell, before anything is dangerous.
    pub startup_s: f32,
    /// How long the volume is live.
    pub active_s: f32,
    /// The tail after it, during which the body is committed.
    pub recover_s: f32,
    /// Volume centre, body-local. Mirrors with facing.
    pub offset: (f32, f32),
    /// Volume half-extents. Do not transpose with `offset`.
    pub half_extents: (f32, f32),
    pub damage: i32,
    /// Base launch speed.
    pub knockback: f32,
    /// How much the launch grows with the victim's damage.
    pub knockback_growth: f32,
    /// `None` lets the shared rule derive it from the geometry.
    pub launch_dir: Option<(f32, f32)>,
    /// What landing this hit can do beyond damage. The down-air uses it to say
    /// it can rebound its attacker; the ruleset
    /// (`DeclaredCombatRules::downward_hit`) decides whether to use that or
    /// read the swing as a spike.
    pub on_hit: Option<EffectRef>,
}

impl<'a> Strike<'a> {
    /// A named, zero-damage placeholder. Call sites still state every real
    /// field; this exists so a new field does not require editing every
    /// literal.
    pub fn new(id: &'a str, clip: &'a str) -> Self {
        Self {
            id,
            clip,
            startup_s: 0.0,
            active_s: 0.0,
            recover_s: 0.0,
            offset: (0.0, 0.0),
            half_extents: (0.0, 0.0),
            damage: 0,
            knockback: 0.0,
            knockback_growth: 1.0,
            launch_dir: None,
            on_hit: None,
        }
    }
}

/// One holding pulse of a multi-hit, authored once and repeated.
///
/// Small on purpose: only what a pulse needs that the finisher does not
/// already state. A pulse is a weak hit that holds; the finisher launches.
#[derive(Clone, Copy, Debug)]
pub struct Pulse {
    /// Where the pulse's box sits, body-local, and how big it is.
    pub offset: (f32, f32),
    pub half_extents: (f32, f32),
    /// Chip damage. Pulses are cheap on purpose: the finisher pays for the
    /// move, and pulses that hurt would be better than the ending.
    pub damage: i32,
    /// How long one pulse is live, and the gap before the next one.
    ///
    /// The gap is required. The move runtime lets separated Active windows hit
    /// the same victim again and refuses a re-hit across a contiguous track, so
    /// a multi-hit with one long window, or windows that touch, lands once.
    pub active_s: f32,
    pub gap_s: f32,
    /// The hold itself.
    pub autolink: crate::AutolinkVolume,
}

/// A multi-hit: `pulses` holding hits, then the strike you pass in as the
/// finisher.
///
/// A combinator over [`strike`], not a second builder. The finisher is an
/// ordinary strike; this inserts the lead-in before it. The intermediate hits
/// keep the victim inside the next box, and only the last one launches.
///
/// Not a capture and not a per-character mechanism: `HitVolume::autolink` on
/// the pulse volumes holds the victim, and any move may author it.
pub fn multihit(m: MoveSpec, pulses: usize, pulse: Pulse) -> MoveSpec {
    if pulses == 0 {
        return m;
    }
    let mut m = m;
    // The lead-in fills the finisher's Startup, which is stretched, so the
    // finisher keeps the startup its author chose.
    let lead_in = pulses as f32 * (pulse.active_s + pulse.gap_s);
    let shift = |t: f32| t + lead_in;
    let finish_start = active_start(&m);
    for window in &mut m.windows {
        // Everything from the finisher's Active onward moves back; its Startup
        // stretches to cover the lead-in and is not duplicated.
        if window.start_s >= finish_start {
            window.start_s = shift(window.start_s);
            window.end_s = shift(window.end_s);
        } else {
            window.end_s = shift(window.end_s);
        }
    }
    m.duration_s = shift(m.duration_s);
    let mut pulse_windows: Vec<MoveWindow> = Vec::with_capacity(pulses);
    for index in 0..pulses {
        let start = finish_start + index as f32 * (pulse.active_s + pulse.gap_s);
        pulse_windows.push(MoveWindow {
            start_s: start,
            end_s: start + pulse.active_s,
            tag: WindowTag::Active,
            volumes: vec![HitVolume {
                shape: VolumeShape::Rect {
                    offset: pulse.offset,
                    half_extents: pulse.half_extents,
                },
                damage: pulse.damage,
                // A pulse has no launch of its own: the autolink sets the
                // victim's velocity. The knockback value still feeds the
                // hitstun the pulse owes.
                knockback: 1.0,
                knockback_growth: Some(0.0),
                launch_dir: None,
                reaction: Some(crate::VolumeReaction::Autolink(
                    pulse.autolink,
                )),
                on_hit: None,
                vfx: Some(SLASH_POKE_VFX.to_string()),
                hit_sfx: None,
            }],
            motion_scale: 1.0,
            sustain_effect: None,
        });
    }
    m.windows.append(&mut pulse_windows);
    // The runtime reads windows in authored order for the sweetspot rule, so
    // the lead-in must sort before the finisher, not only start earlier.
    m.windows.sort_by(|a, b| {
        a.start_s
            .total_cmp(&b.start_s)
            .then(a.end_s.total_cmp(&b.end_s))
    });
    m
}

/// A gust: a volume that pushes and does not hurt.
///
/// It uses `VolumeReaction::Windbox`, and `hit_reaction` sets `flinchless`
/// from it. This builder holds three invariants, each silent when broken:
///
/// - Damage must be zero, or the catalog rejects the move
///   (`WindboxWithDamage`).
/// - Knockback growth must be fixed, or a gust pushes a damaged fighter
///   further than a fresh one.
/// - No slash arc: `strike` draws one from the spawned volume, so a gust built
///   on it would show a blade that does no damage.
pub struct Gust<'a> {
    /// The move id. Unique within the kit.
    pub id: &'a str,
    /// The animation row, with `strike`'s fallbacks.
    pub clip: &'a str,
    /// The tell, before the air moves.
    pub startup_s: f32,
    /// How long the gust blows.
    pub active_s: f32,
    /// The tail after it.
    pub recover_s: f32,
    /// Volume centre, body-local. Mirrors with facing.
    pub offset: (f32, f32),
    /// Volume half-extents, body-local.
    pub half_extents: (f32, f32),
    /// How hard it pushes, in the units of a strike's `knockback`. This is the
    /// only thing the move does to its target.
    pub push: f32,
    /// Which way it shoves, body-local: `+x` toward facing, `+y` gravity-down.
    ///
    /// Authored, not derived (unlike a strike's `None`). The shared rule
    /// derives a launch from where the victim stood relative to the volume,
    /// which is wrong for wind: a gust blows one way.
    pub push_dir: (f32, f32),
    /// Does it keep pushing while they stand in it?
    ///
    /// `true` opts out of the hit-once set: right for a sustained wind, wrong
    /// for a one-shot push (see `WindboxVolume::repeating`). A `false` gust is
    /// a single hard blast.
    pub sustained: bool,
}

/// Author a gust: [`strike`]'s timeline, with the three windbox invariants held.
///
/// # Panics
///
/// If `push` is not positive. Such a gust spends a startup and a recovery to
/// do nothing, and its volume is invisible, so a player cannot see the fault.
pub fn gust(spec: Gust<'_>) -> MoveSpec {
    assert!(
        spec.push > 0.0,
        "gust `{}` shoves with {}, so it spends its whole timeline doing nothing \
         visible to anybody",
        spec.id,
        spec.push,
    );
    let mut m = strike(Strike {
        id: spec.id,
        clip: spec.clip,
        startup_s: spec.startup_s,
        active_s: spec.active_s,
        recover_s: spec.recover_s,
        offset: spec.offset,
        half_extents: spec.half_extents,
        // Zero: `WindboxWithDamage` is a validation error.
        damage: 0,
        knockback: spec.push,
        // Set on the volumes below: the builder reads its own zero as "the
        // stage decides", which wind must not do.
        knockback_growth: 0.0,
        launch_dir: Some(spec.push_dir),
        on_hit: None,
    });
    for volume in m.windows.iter_mut().flat_map(|w| w.volumes.iter_mut()) {
        volume.reaction = Some(crate::VolumeReaction::Windbox(
            crate::WindboxVolume {
                repeating: spec.sustained,
            },
        ));
        // No slash: `strike` draws its arc from the spawned volume, so without
        // this the fighter would show a blade. There is no wind art yet, so a
        // gust draws nothing; this is a known gap.
        volume.vfx = None;
    }
    fixed_knockback(m)
}

/// One strike on one timeline: startup, one active window with one volume,
/// recovery.
///
/// Nearly every move has this shape, so the authored differences are the ones
/// that matter: commitment time, reach, launch, and how much of the launch
/// scales with the victim's damage.
pub fn strike(spec: Strike<'_>) -> MoveSpec {
    let Strike {
        id,
        clip,
        startup_s,
        active_s,
        recover_s,
        offset,
        half_extents,
        damage,
        knockback,
        knockback_growth,
        launch_dir,
        on_hit,
    } = spec;
    let active_start = startup_s;
    let active_end = startup_s + active_s;
    MoveSpec {
        display_name: None,
        id: id.to_string(),
        clip: ClipBinding {
            clip: clip.to_string(),
            // The fallback chain. A move names the exact row it wants
            // (`smash_forward`, `air_back`); this is what it uses when a sheet
            // lacks that row.
            //
            // The fallbacks are directional first: an up-tilt without
            // `attack_up` should look like a side swing, and every fighter
            // sheet has `attack_side`.
            //
            // A missing clip must never cost the move its gameplay: the
            // timeline runs whatever draws.
            fallbacks: vec![
                "attack_side".to_string(),
                "attack".to_string(),
                "slash".to_string(),
                "idle".to_string(),
            ],
        },
        duration_s: active_end + recover_s,
        windows: vec![
            MoveWindow {
                start_s: 0.0,
                end_s: active_start,
                tag: WindowTag::Startup,
                volumes: Vec::new(),
                motion_scale: 1.0,
                sustain_effect: None,
            },
            MoveWindow {
                start_s: active_start,
                end_s: active_end,
                tag: WindowTag::Active,
                volumes: vec![HitVolume {
                    shape: VolumeShape::Rect {
                        offset,
                        half_extents,
                    },
                    damage,
                    knockback,
                    // The builder's zero means "the stage decides". A move that
                    // wants fixed knockback says so on the volume (see
                    // `HitVolume::knockback_growth` and [`fixed_knockback`]).
                    knockback_growth: (knockback_growth > 0.0).then_some(knockback_growth),
                    launch_dir,
                    on_hit,
                    // The blade tag: the move runtime draws the slash from the
                    // same spawned volume, so the hitbox and the arc agree. A
                    // poke wants the other tag (see [`strike_tag`]).
                    vfx: Some(SLASH_ARC_VFX.to_string()),
                    hit_sfx: None,
                    reaction: None,
                }],
                motion_scale: 1.0,
                sustain_effect: None,
            },
            MoveWindow {
                start_s: active_end,
                end_s: active_end + recover_s,
                tag: WindowTag::Recovery,
                volumes: Vec::new(),
                motion_scale: 1.0,
                sustain_effect: None,
            },
        ],
        events: Vec::new(),
        gates: MoveGates::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
        smash_charge: None,
        charge_gesture: crate::ChargeGesture::default(),
        repeat: None,
        landing_lag_s: None,
        autocancel_after_s: None,
        sprite_spin_hz: None,
        equips: None,
        flow: None,
    }
}

#[cfg(test)]
mod multihit_tests {
    use super::*;

    fn finisher() -> MoveSpec {
        strike(Strike {
            id: "test_multihit",
            clip: "attack_up",
            startup_s: 0.09,
            active_s: 0.10,
            recover_s: 0.20,
            offset: (5.0, -19.0),
            half_extents: (22.0, 27.0),
            damage: 7,
            knockback: 88.0,
            knockback_growth: 1.65,
            launch_dir: Some((0.1, -1.0)),
            on_hit: None,
        })
    }

    fn pulse() -> Pulse {
        Pulse {
            offset: (2.0, -12.0),
            half_extents: (26.0, 30.0),
            damage: 2,
            active_s: 0.035,
            gap_s: 0.030,
            autolink: crate::AutolinkVolume {
                anchor: (14.0, 6.0),
                carry: 1.0,
                pull: 22.0,
                max_speed: 900.0,
            },
        }
    }

    fn actives(m: &MoveSpec) -> Vec<&MoveWindow> {
        m.windows
            .iter()
            .filter(|w| matches!(w.tag, WindowTag::Active))
            .collect()
    }

    /// The gaps are required. The move runtime lets separated Active windows
    /// hit the same victim again and refuses a re-hit across a contiguous
    /// track, so one long window, or windows that touch, would land once.
    #[test]
    fn every_pulse_is_a_separated_window_so_each_one_can_re_hit() {
        let m = multihit(finisher(), 4, pulse());
        let live = actives(&m);
        assert_eq!(live.len(), 5, "four pulses and one finisher: {live:?}");
        for pair in live.windows(2) {
            assert!(
                pair[1].start_s > pair[0].end_s + 1e-6,
                "two Active windows touch, so the second cannot re-hit: {:?} then {:?}",
                pair[0],
                pair[1]
            );
        }
    }

    /// The pulses hold and the finisher launches.
    #[test]
    fn the_pulses_hold_and_only_the_last_hit_launches() {
        let m = multihit(finisher(), 4, pulse());
        let live = actives(&m);
        let (last, leading) = live.split_last().expect("five windows");
        for window in leading {
            let volume = &window.volumes[0];
            assert!(
                volume.autolink().is_some(),
                "an intermediate pulse authored no hold, so the victim leaves"
            );
            assert!(
                volume.launch_dir.is_none(),
                "a pulse authored a launch beside its hold — two answers to one \
                 question"
            );
        }
        let finish = &last.volumes[0];
        assert!(
            finish.autolink().is_none(),
            "the FINISHER holds instead of launching, so the move never ends"
        );
        assert_eq!(finish.launch_dir, Some((0.1, -1.0)));
        assert_eq!(finish.damage, 7, "the finisher kept its authored payload");
    }

    /// The finisher moves back by the lead-in and is not overwritten, and the
    /// move grows by exactly that much.
    #[test]
    fn the_finisher_is_pushed_back_and_the_move_grows_by_the_lead_in() {
        let base = finisher();
        let m = multihit(base.clone(), 4, pulse());
        let lead_in = 4.0 * (0.035 + 0.030);
        assert!(
            (m.duration_s - (base.duration_s + lead_in)).abs() < 1e-5,
            "duration {} against {} + {lead_in}",
            m.duration_s,
            base.duration_s
        );
        let live = actives(&m);
        let finish = live.last().expect("a finisher");
        assert!(
            (finish.start_s - (active_start(&base) + lead_in)).abs() < 1e-5,
            "the finisher did not move back by the lead-in: {finish:?}"
        );
        let recovery: Vec<_> = m
            .windows
            .iter()
            .filter(|w| matches!(w.tag, WindowTag::Recovery))
            .collect();
        assert_eq!(recovery.len(), 1, "recovery was duplicated or lost");
        assert!(
            (recovery[0].end_s - m.duration_s).abs() < 1e-5,
            "recovery no longer reaches the end of the move: {:?}",
            recovery[0]
        );
    }

    /// Negative control: zero pulses gives the plain strike. Without this, a
    /// combinator that always inserted something would pass the tests above.
    #[test]
    fn a_multihit_of_zero_pulses_is_the_plain_strike() {
        let base = finisher();
        let m = multihit(base.clone(), 0, pulse());
        assert_eq!(m.duration_s, base.duration_s);
        assert_eq!(m.windows.len(), base.windows.len());
        assert_eq!(actives(&m).len(), 1);
    }
}

/// A verb's invariant is tested in the crate that defines the verb, not only
/// through an authored fighter in another crate.
#[cfg(test)]
mod refusal_tests {
    //! Tests for the documented `# Panics` of each verb.
    //!
    //! Each window verb has its own refusal test, because a shared helper
    //! called from several sites must be guarded on each path.
    //! `all_three_accept_a_window_that_opens` is the positive control: without
    //! it, `should_panic` tests also pass against verbs that always panic.
    use super::*;

    #[test]
    #[should_panic(expected = "never open")]
    fn armour_that_never_opens_is_refused() {
        let _ = armor(taunt("test_armor_window", 0.5), 0.20, 0.20);
    }

    /// The same refusal on `invuln`.
    #[test]
    #[should_panic(expected = "never open")]
    fn i_frames_that_never_open_are_refused() {
        let _ = invuln(taunt("test_invuln_window", 0.5), 0.30, 0.10);
    }

    /// The same refusal on `cancelable`.
    #[test]
    #[should_panic(expected = "never open")]
    fn a_cancel_window_that_never_opens_is_refused() {
        let _ = cancelable(
            taunt("test_cancel_window", 0.5),
            0.20,
            0.20,
            &["special"],
            CancelCondition::OnHit,
        );
    }

    /// Positive control: all three still accept an ordinary window, so the
    /// refusals above test the bound and not a broken verb.
    #[test]
    fn all_three_accept_a_window_that_opens() {
        use crate::WindowTag;
        let armoured = armor(taunt("test_armor_ok", 0.5), 0.05, 0.20);
        let invulnerable = invuln(taunt("test_invuln_ok", 0.5), 0.05, 0.20);
        let cancelling = cancelable(
            taunt("test_cancel_ok", 0.5),
            0.05,
            0.20,
            &["special"],
            CancelCondition::OnHit,
        );
        assert!(armoured.windows.iter().any(|w| w.tag == WindowTag::Armor));
        assert!(invulnerable.windows.iter().any(|w| w.tag == WindowTag::Invuln));
        assert!(cancelling
            .windows
            .iter()
            .any(|w| matches!(w.tag, WindowTag::Cancelable { .. })));
    }

    #[test]
    #[should_panic(expected = "commits at")]
    fn a_special_that_commits_after_it_ends_is_refused() {
        let _ = hitless_special("test_commit", "attack", 0.9, 0.4);
    }

    #[test]
    #[should_panic(expected = "nothing visible to anybody")]
    fn a_gust_that_does_not_shove_is_refused() {
        let _ = gust(Gust {
            id: "test_gust",
            clip: "attack_side",
            startup_s: 0.20,
            active_s: 0.30,
            recover_s: 0.30,
            offset: (30.0, 0.0),
            half_extents: (30.0, 22.0),
            push: 0.0,
            push_dir: (1.0, -0.2),
            sustained: true,
        });
    }
}

#[cfg(test)]
mod charge_tests {
    //! Each guarantee of [`charge`] has its own test: the hold inside the
    //! move, a positive max hold, the multiplier floor, and the gesture.
    use super::*;

    fn swing() -> MoveSpec {
        strike(Strike {
            id: "test_charge",
            clip: "attack",
            startup_s: 0.16,
            active_s: 0.08,
            recover_s: 0.30,
            offset: (30.0, 0.0),
            half_extents: (18.0, 14.0),
            damage: 13,
            knockback: 142.0,
            knockback_growth: 2.75,
            launch_dir: Some((1.0, -0.28)),
            on_hit: None,
        })
    }

    fn held() -> Charge {
        Charge {
            hold_at_s: 0.06,
            max_hold_s: 1.2,
            stores: false,
            roots: true,
            sustain: crate::ChargeSustain::WhileHeld,
            gesture: crate::ChargeGesture::Special,
            multiplier: 1.6,
        }
    }

    /// One call sets the spec, the gesture and the multiplier, so a caller
    /// cannot forget one of them.
    #[test]
    fn a_charge_sets_its_spec_its_gesture_and_its_payoff() {
        let m = charge(swing(), held());
        let spec = m.smash_charge.as_ref().expect("the charge is authored");
        assert_eq!(spec.max_hold_s, 1.2);
        assert!(spec.roots, "the hold roots him");
        assert!(!spec.stores, "and does not bank");
        assert_eq!(m.charge_gesture, crate::ChargeGesture::Special);
        assert_eq!(m.smash_charge_mult, 1.6);
    }

    /// The move's own timeline is untouched — a charge is a hold ON a move, not
    /// a different move.
    #[test]
    fn a_charge_moves_no_window() {
        let base = swing();
        let m = charge(base.clone(), held());
        assert_eq!(m.windows.len(), base.windows.len());
        assert_eq!(m.duration_s, base.duration_s);
    }

    #[test]
    #[should_panic(expected = "never arrives")]
    fn a_hold_after_the_move_ends_is_refused() {
        let mut late = held();
        late.hold_at_s = 99.0;
        let _ = charge(swing(), late);
    }

    #[test]
    #[should_panic(expected = "cannot be held")]
    fn a_zero_length_hold_is_refused() {
        let mut instant = held();
        instant.max_hold_s = 0.0;
        let _ = charge(swing(), instant);
    }

    /// A multiplier of exactly 1.0 is allowed. A charge's reward may be outside
    /// the move (the Projectile Polygon's charge shot pays through
    /// `RangedCharge`'s tier ladder).
    #[test]
    fn a_multiplier_of_one_is_allowed_because_the_payoff_may_be_elsewhere() {
        let mut flat = held();
        flat.multiplier = 1.0;
        let m = charge(swing(), flat);
        assert_eq!(m.smash_charge_mult, 1.0);
        assert!(m.smash_charge.is_some(), "and it is still a charge");
    }

    /// What cannot be right: a hold that makes the move weaker.
    #[test]
    #[should_panic(expected = "WEAKER")]
    fn a_charge_that_weakens_the_move_is_refused() {
        let mut worse = held();
        worse.multiplier = 0.8;
        let _ = charge(swing(), worse);
    }
}

#[cfg(test)]
mod wake_tests {
    use super::*;

    fn kick() -> MoveSpec {
        strike(Strike {
            id: "test_wake",
            clip: "attack_down",
            startup_s: 0.12,
            active_s: 0.08,
            recover_s: 0.24,
            offset: (18.0, 16.0),
            half_extents: (30.0, 10.0),
            damage: 6,
            knockback: 70.0,
            knockback_growth: 1.35,
            launch_dir: Some((0.7, -0.6)),
            on_hit: None,
        })
    }

    fn dust() -> Wake {
        Wake {
            offset: (58.0, 14.0),
            half_extents: (24.0, 8.0),
            push: 60.0,
            push_dir: (1.0, -0.15),
            repeating: false,
        }
    }

    /// The hit stays ahead of the push (the inverse of [`tipper`]). A wake
    /// ranked first would push people the move was about to hit.
    #[test]
    fn the_wake_is_ranked_behind_the_hit() {
        let m = wake(kick(), dust());
        let window = m
            .windows
            .iter()
            .find(|w| w.tag == WindowTag::Active && !w.volumes.is_empty())
            .expect("an active window");
        assert_eq!(window.volumes.len(), 2);
        assert!(window.volumes[0].damage > 0, "index 0 is the HIT");
        assert_eq!(window.volumes[1].damage, 0, "index 1 is the wind");
        assert!(
            window.volumes[1].shape.leading_edge_x()
                > window.volumes[0].shape.leading_edge_x(),
            "the wake trails BEYOND the hit"
        );
    }

    /// It is a windbox, not a weak second hitbox: zero damage, flat push, and the
    /// reaction that makes `hit_reaction` read it as flinchless.
    #[test]
    fn the_wake_pushes_and_does_not_hurt() {
        let m = wake(kick(), dust());
        let wind = m
            .windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .find(|v| v.damage == 0)
            .expect("a wind volume");
        assert!(matches!(
            wind.reaction,
            Some(crate::VolumeReaction::Windbox(_))
        ));
        assert_eq!(
            wind.knockback_growth,
            Some(0.0),
            "a shove that grew with damage is a hit's rule in wind's costume"
        );
    }

    #[test]
    fn a_wake_adds_no_window() {
        let base = kick();
        let m = wake(base.clone(), dust());
        assert_eq!(m.windows.len(), base.windows.len());
        assert_eq!(m.duration_s, base.duration_s);
    }

    #[test]
    #[should_panic(expected = "dead code")]
    fn a_wake_inside_the_hit_is_refused() {
        let mut enclosed = dust();
        enclosed.offset = (10.0, 16.0);
        let _ = wake(kick(), enclosed);
    }

    #[test]
    #[should_panic(expected = "no push")]
    fn a_wake_that_does_not_push_is_refused() {
        let mut still = dust();
        still.push = 0.0;
        let _ = wake(kick(), still);
    }
}

#[cfg(test)]
mod tipper_tests {
    use super::*;

    fn poke() -> MoveSpec {
        strike(Strike {
            id: "test_tipper",
            clip: "attack",
            startup_s: 0.10,
            active_s: 0.08,
            recover_s: 0.20,
            offset: (30.0, 0.0),
            half_extents: (14.0, 10.0),
            damage: 6,
            knockback: 80.0,
            knockback_growth: 1.40,
            launch_dir: Some((0.8, -0.5)),
            on_hit: None,
        })
    }

    fn far_tip() -> Tip {
        Tip {
            offset: (52.0, 0.0),
            half_extents: (6.0, 6.0),
            damage: 13,
            knockback: 130.0,
            knockback_growth: Some(2.0),
            launch_dir: Some((0.8, -0.5)),
        }
    }

    /// Tests rank, not presence. An appended tip reads correctly in the
    /// source but plays backwards, and a volume count would not catch that.
    #[test]
    fn the_tip_is_ranked_ahead_of_the_base() {
        let m = tipper(poke(), far_tip());
        let window = m
            .windows
            .iter()
            .find(|w| w.tag == WindowTag::Active && !w.volumes.is_empty())
            .expect("an active window");
        assert_eq!(window.volumes.len(), 2, "a tipper is base plus tip");
        assert!(
            window.volumes[0].shape.leading_edge_x()
                > window.volumes[1].shape.leading_edge_x(),
            "index 0 must be the FAR volume: {} vs {}",
            window.volumes[0].shape.leading_edge_x(),
            window.volumes[1].shape.leading_edge_x(),
        );
        assert!(window.volumes[0].damage > window.volumes[1].damage);
    }

    /// The move keeps its own timeline: a sweetspot is a second volume on a
    /// window, never a second window.
    #[test]
    fn a_tip_adds_no_window_and_moves_no_timing() {
        let base = poke();
        let m = tipper(base.clone(), far_tip());
        assert_eq!(m.windows.len(), base.windows.len());
        assert_eq!(m.duration_s, base.duration_s);
    }

    #[test]
    #[should_panic(expected = "reach")]
    fn a_tip_that_does_not_outreach_the_base_is_refused() {
        let mut near = far_tip();
        near.offset = (10.0, 0.0);
        let _ = tipper(poke(), near);
    }

    /// "Stronger" is an or: the guard reads
    /// `tip.damage > base.damage || tip.knockback > base.knockback`. A tip that
    /// trades damage for launch is a valid sword design. Only a tip weaker on
    /// both counts (a sourspot at the far end) is refused.
    #[test]
    #[should_panic(expected = "sourspot")]
    fn a_tip_weaker_on_both_counts_is_refused() {
        let mut weak = far_tip();
        weak.damage = 1;
        weak.knockback = 10.0;
        let _ = tipper(poke(), weak);
    }

    /// The builder's zero is not fixed knockback.
    ///
    /// `strike` stores `(knockback_growth > 0.0).then_some(knockback_growth)`,
    /// so `knockback_growth: 0.0` in a [`Strike`] gives `None` on the volume,
    /// which means "the stage decides" (the ruleset's growth). A move that
    /// wants a flat launch says so on the volume via [`fixed_knockback`].
    #[test]
    fn a_zero_growth_in_the_builder_means_the_stage_decides() {
        let m = strike(Strike {
            id: "test_zero_growth",
            clip: "attack",
            startup_s: 0.10,
            active_s: 0.08,
            recover_s: 0.20,
            offset: (30.0, 0.0),
            half_extents: (14.0, 10.0),
            damage: 6,
            knockback: 80.0,
            knockback_growth: 0.0,
            launch_dir: Some((0.8, -0.5)),
            on_hit: None,
        });
        let volume = m
            .windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .next()
            .expect("a volume");
        assert_eq!(
            volume.knockback_growth, None,
            "the builder's zero is 'stage decides', NOT flat"
        );
        let flat = fixed_knockback(m);
        let volume = flat
            .windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .next()
            .expect("a volume");
        assert_eq!(
            volume.knockback_growth,
            Some(0.0),
            "`fixed_knockback` is the only thing that authors a flat launch"
        );
    }

    /// The other side of the or: a tip may hit for less and launch for more.
    #[test]
    fn a_tip_may_trade_damage_for_launch() {
        let mut trade = far_tip();
        trade.damage = 3;
        let m = tipper(poke(), trade);
        let window = m
            .windows
            .iter()
            .find(|w| w.tag == WindowTag::Active && !w.volumes.is_empty())
            .expect("an active window");
        assert_eq!(window.volumes.len(), 2);
    }
}

/// A smash charge: the hold, what it buys, and which button drives it.
///
/// A `SmashChargeSpec` travels with two companions, `charge_gesture` and
/// `smash_charge_mult`. As fields here, they are set where the charge is
/// decided, and a caller cannot forget one.
///
/// `projectile_polygon_moveset.rs` builds its charge shot as a whole
/// `MoveSpec` literal, so it does not use this `MoveSpec -> MoveSpec` verb.
pub struct Charge {
    /// When the hold begins, in move-seconds. Keep it inside the startup: the
    /// windup should read before the freeze.
    pub hold_at_s: f32,
    /// The longest hold, in seconds.
    pub max_hold_s: f32,
    /// May the charge be banked for a later press?
    ///
    /// A stored charge is a threat carried into the next exchange. It changes
    /// the fighter's character a lot, so it has no default here.
    pub stores: bool,
    /// Is the fighter rooted while holding?
    pub roots: bool,
    /// What sustains the hold; see [`ChargeSustain`].
    pub sustain: crate::ChargeSustain,
    /// Which press drives it. `Smash` is the genre's default; `Special` is for a
    /// charge that lives on a special button.
    pub gesture: crate::ChargeGesture,
    /// What a full hold multiplies this move by.
    ///
    /// `1.0` is valid: a charge's payoff may be outside the move (the
    /// Projectile Polygon's charge shot pays through `RangedCharge`'s tier
    /// ladder). So `1.0` means "the reward is elsewhere". A value below `1.0`
    /// is refused, because a hold that makes the move weaker cannot be
    /// correct.
    pub multiplier: f32,
}

/// Give a move a smash charge, its gesture and its payoff in one call.
///
/// # Panics
///
/// If the hold begins outside the move (a freeze that never arrives, or one
/// after the move has ended); if `max_hold_s` is not positive; or if the
/// multiplier is below `1.0`. A multiplier of exactly `1.0` is allowed (see
/// [`Charge::multiplier`]).
pub fn charge(mut m: MoveSpec, charge: Charge) -> MoveSpec {
    let id = m.id.clone();
    assert!(
        charge.hold_at_s >= 0.0 && charge.hold_at_s < m.duration_s,
        "move `{id}` freezes at {}s in a move lasting {}s — a hold outside the \
         move never arrives",
        charge.hold_at_s,
        m.duration_s,
    );
    assert!(
        charge.max_hold_s > 0.0,
        "move `{id}` authors a {}s maximum hold, so the charge cannot be held",
        charge.max_hold_s,
    );
    assert!(
        charge.multiplier >= 1.0,
        "move `{id}` authors a charge multiplier of {} — a hold that makes the \
         move WEAKER is the one reading of this number that cannot be right",
        charge.multiplier,
    );
    m.smash_charge = Some(crate::SmashChargeSpec {
        hold_at_s: charge.hold_at_s,
        max_hold_s: charge.max_hold_s,
        stores: charge.stores,
        roots: charge.roots,
        sustain: charge.sustain,
    });
    m.charge_gesture = charge.gesture;
    m.smash_charge_mult = charge.multiplier;
    m
}

/// The push a move leaves beyond its hit: the dust, the wake, the displaced
/// air.
///
/// The inverse of a [`Tip`]. A tip goes in first, so the far volume wins
/// wherever both reach. A wake is appended, so it applies only when the hit
/// misses: a body in both is hit, and a body out of hit range is pushed.
///
/// It cannot carry damage: `WindboxWithDamage` is a validation error. It moves
/// you and does not hurt you.
pub struct Wake {
    /// Where the push sits, body-local, and how big it is.
    pub offset: (f32, f32),
    pub half_extents: (f32, f32),
    /// How hard it pushes, in the units of a volume's `knockback`.
    pub push: f32,
    /// Which way it pushes, body-local. A ground wake pushes along the floor;
    /// a gust pushes away from the chest.
    pub push_dir: (f32, f32),
    /// May it move the same body again while that body stands in it?
    ///
    /// `true` for a sustained wind you cannot walk through; `false` for a
    /// one-shot push. This opts out of the hit-once set (see
    /// [`WindboxVolume::repeating`]).
    pub repeating: bool,
}

/// Give a strike a wake: a pushing volume beyond its hit that does no damage.
///
/// One call for "it hits, and what it misses gets pushed". [`gust`] builds a
/// whole move of wind; this adds a wake to an existing strike.
///
/// # Panics
///
/// If the move has no Active volume for a wake to trail; if the wake does not
/// reach further than the hit (the damaging volume is ranked first and wins
/// where both reach, so an enclosed wake is dead code); or if the push is not
/// positive.
pub fn wake(mut m: MoveSpec, wake: Wake) -> MoveSpec {
    let id = m.id.clone();
    assert!(
        wake.push > 0.0,
        "move `{id}` authors a wake with no push, which is a volume that costs a \
         window and does nothing"
    );
    let window = m
        .windows
        .iter_mut()
        .find(|w| w.tag == WindowTag::Active && !w.volumes.is_empty())
        .unwrap_or_else(|| panic!("move `{id}` has no active volume for a wake to trail"));
    let hit_edge = window.volumes[0].shape.leading_edge_x();
    let wake_edge = wake.offset.0 + wake.half_extents.0;
    assert!(
        wake_edge > hit_edge,
        "move `{id}` authors a wake reaching {wake_edge} inside a hit reaching \
         {hit_edge} — the damaging volume is ranked first and wins wherever both \
         reach, so an enclosed wake is authored dead code"
    );
    // Appended: first-authored wins, so the hit must stay ahead of the push.
    // See [`Wake`].
    window.volumes.push(HitVolume {
        shape: VolumeShape::Rect {
            offset: wake.offset,
            half_extents: wake.half_extents,
        },
        // Zero: `WindboxWithDamage` is a validation error.
        damage: 0,
        knockback: wake.push,
        // Fixed: a push that grew with the victim's damage would follow a
        // hit's rule, not wind's (as in `gust`).
        knockback_growth: Some(0.0),
        launch_dir: Some(wake.push_dir),
        reaction: Some(crate::VolumeReaction::Windbox(
            crate::WindboxVolume {
                repeating: wake.repeating,
            },
        )),
        on_hit: None,
        vfx: None,
        // A wake is silent: the move's own cues already mark the kick, and a
        // sound here would play on a body that was not hit.
        hit_sfx: None,
    });
    m
}

/// The far half of a swing, authored to outrank the near half.
///
/// A thrust whose tip hits harder than its base rewards spacing: the same
/// button is a poke up close and a kill at range.
///
/// Compare [`Wake`], the inverse shape: a tip is inserted at rank 0 so the far
/// volume wins wherever both reach; a wake is appended so the hit stays ahead
/// of the push.
pub struct Tip {
    /// Where the tip sits, body-local, and how big it is.
    pub offset: (f32, f32),
    pub half_extents: (f32, f32),
    pub damage: i32,
    pub knockback: f32,
    pub knockback_growth: Option<f32>,
    pub launch_dir: Option<(f32, f32)>,
}

/// Give a strike a sweetspot: a stronger volume at its far end.
///
/// `StrikeRank { window, volume }` is the move's reading order, and the strike
/// seam arbitrates on it: the victim takes the first-authored volume that
/// reaches it and no other. So a tipper is one Active window with two volumes,
/// with the tip first.
///
/// The tip is inserted at index 0. Appended, it would rank below the base, and
/// the sourspot would win wherever both reach.
///
/// # Panics
///
/// If the move has no Active volume to be the base; if the tip does not reach
/// further than the base; or if it is weaker on both damage and knockback.
///
/// The strength rule is an or: a tip may hit for less damage and launch for
/// more. That is a normal sword design.
///
/// The reach rule is about the name, not reachability. A tip that the base
/// contains still wins wherever it reaches (rank 0 is never outranked), but
/// it is not a tip. Author a hilt sweetspot on its own terms.
pub fn tipper(mut m: MoveSpec, tip: Tip) -> MoveSpec {
    let id = m.id.clone();
    let window = m
        .windows
        .iter_mut()
        .find(|w| w.tag == WindowTag::Active && !w.volumes.is_empty())
        .unwrap_or_else(|| {
            panic!("move `{id}` has no active volume for a tip to outrank")
        });
    let base = &window.volumes[0];
    let base_edge = base.shape.leading_edge_x();
    let tip_edge = tip.offset.0 + tip.half_extents.0;
    assert!(
        tip_edge > base_edge,
        "move `{id}` authors a tip reaching {tip_edge}px and a base reaching \
         {base_edge}px. The tip WOULD still be felt — rank 0 is never outranked \
         — but it would not be a TIP, and this helper's name promises the far \
         end. Author a sweetspot elsewhere on its own terms.",
    );
    assert!(
        tip.damage > base.damage || tip.knockback > base.knockback,
        "move `{id}` authors a tip that is no stronger than its base ({} damage \
         / {} knockback against {} / {}) — that is a sourspot with extra steps, \
         and the spacing it asks the player to learn buys them nothing",
        tip.damage,
        tip.knockback,
        base.damage,
        base.knockback,
    );
    // Insert, do not push: rank is authored order.
    window.volumes.insert(
        0,
        HitVolume {
            shape: VolumeShape::Rect {
                offset: tip.offset,
                half_extents: tip.half_extents,
            },
            damage: tip.damage,
            knockback: tip.knockback,
            knockback_growth: tip.knockback_growth,
            launch_dir: tip.launch_dir,
            reaction: None,
            on_hit: None,
            vfx: None,
            hit_sfx: None,
        },
    );
    m
}
