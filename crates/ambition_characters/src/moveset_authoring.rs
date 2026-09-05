//! The primitives a character's move table is written with — shared, because
//! the second character to author one must not begin by copying the first.
//!
//! Mary-O and Sanic are registered by their own demos, both were 0/16 on the smash grid, and
//! neither demo depends on Ambition's content crate — so completing their kits meant either a
//! fourth copy of these helpers or moving the one copy to a crate everybody already has.
//! `ambition_characters` is where the character model lives and where `moveset_prefabs` already
//! derives a table from an action set; authoring one by hand belongs beside it.
//!
//! There is ONE `strike` now. `ambition_demo_smash` carried a fork of it for a
//! while, differing only in the clip fallback chain; the platform-fighter half
//! that is genuinely its own — the `Feel` vocabulary, the repertoire, the cancel
//! conventions — stayed there, which is the line: a move-BUILDING fact is
//! shared, a game's POLICY about moves is not.
//!
//! They were never robot-specific — `strike` is *startup, one active window carrying one volume,
//! recovery*, which is the shape of nearly every move in the genre.
//!
//!  a move states what it IS, never what a mode does with it. Startup,
//! active frames, recovery, hitbox geometry, damage, base launch and growth are
//! properties of the swing; percent, stocks, blast zones and DI are the
//! RULESET's. That is what lets one table read as Hollow-Knight combat in one
//! game and a platform fighter in another.

use crate::moveset_prefabs::{SLASH_ARC_VFX, SLASH_POKE_VFX};
use ambition_entity_catalog::{
    CancelCondition, ClipBinding, EffectRef, HitVolume, ImpulseMode, MoveEvent, MoveEventKind,
    MoveGates, MoveSpec, MoveWindow, VolumeShape, WindowTag,
};

// The posture now follows from the slot in [`crate::smash_repertoire::SmashRepertoire`], which is
// the only place in the repo that knows what a tilt or an aerial IS. Leaving the helpers here would
// have left the per-move override available to a fighter that did not mean to take it.

fn event(mut m: MoveSpec, at_s: f32, kind: MoveEventKind) -> MoveSpec {
    m.events.push(MoveEvent { at_s, kind });
    m
}

/// A TIMED SELF-DISPLACEMENT.
///
///  [`ImpulseMode::Set`] COMMANDS a velocity; [`ImpulseMode::Add`] contributes
/// to one. The difference is the difference between a recovery and a hop: a body
/// falling at terminal velocity gets exactly the same result from a `Set` as a
/// standing one does, and the worst possible result from an `Add`. It is also
/// what the catalog's `lift_speed` / `lift_side` derivation keys on — an `Add`
/// states no speed, so no static reader may claim one for it.
///
/// `local` is body-local: `+x` toward facing, `+y` toward the feet, so a rise is
/// a NEGATIVE second component.
pub fn impulse(m: MoveSpec, at_s: f32, local: (f32, f32), mode: ImpulseMode) -> MoveSpec {
    event(m, at_s, MoveEventKind::Impulse { local, mode })
}

/// A CUE AT A MOMENT. The move's own timeline is where its sound lives, so a
/// windup you can hear and a swing you can hear are two events and not two
/// systems.
///
/// A [`vfx`] burst is heard on its own; writing `sfx(m, t, "vfx.<family>.<row>")` beside one
/// plays it TWICE. If the sound is genuinely not the row's default, say so ON the burst with
/// [`vfx_cued`].
pub fn sfx(m: MoveSpec, at_s: f32, cue: &str) -> MoveSpec {
    event(
        m,
        at_s,
        MoveEventKind::Sfx {
            cue: cue.to_string(),
        },
    )
}

/// A BURST AT A MOMENT — picture and sound, because they are one thing.
///
///  `effect` is the NAME of a row on one of the shipped FX spritesheets
/// (`ambition_sprite_sheet::fx` — 189 of them). `MoveSpec::presentation_problems`
/// refuses a name no sheet carries, and the renderer counts it as a miss rather
/// than playing nothing quietly.
///
///  it is heard as well as seen, and the author writes nothing for that.
/// The bank ships one `vfx.<family>.<row>` cue per authored row, so the name
/// that finds the clip finds the sound; `dispatch_move_events` asks for the pair
/// and presentation resolves it.  do NOT follow this with an [`sfx`] naming
/// that same cue — the burst would be heard twice. [`vfx_cued`] is for the
/// exception where the sound is genuinely not the row's default.
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

/// A burst that says WHERE and HOW BIG, in the same body-local numbers the
/// move's strike volumes use.
///
///  pass a volume's own `offset` as `at` and the two cannot disagree.
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

/// A PLACED BURST THAT DOES NOT SOUND LIKE ITS OWN ROW.
///
///  `cue` is a bank cue name, not an effect row name. An id neither the
/// registry nor the packed bank authorizes is counted and dropped, not heard —
/// so a typo here is silence, exactly as it is for [`sfx`].
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

/// WHAT LANDING THIS MOVE SOUNDS LIKE, applied to every volume it throws.
/// Contact feedback belongs to the volume because only the volume knows it
/// connected.
pub fn on_contact(mut m: MoveSpec, cue: &str) -> MoveSpec {
    for volume in m.windows.iter_mut().flat_map(|w| w.volumes.iter_mut()) {
        volume.hit_sfx = Some(cue.to_string());
    }
    m
}

/// HOW THE SWING ITSELF IS DRAWN — the strike-presentation tag every volume
/// this move throws carries.
///
///  this is NOT an FX-sheet row name. `HitVolume::vfx` is a two-word
/// vocabulary ([`SLASH_ARC_VFX`] /
/// [`SLASH_POKE_VFX`](crate::moveset_prefabs::SLASH_POKE_VFX))
/// that the move runtime
/// reads twice: it picks the arc-vs-jab shape drawn out of the spawned volume,
/// and it is the flag that makes a volume prefer the sprite manifest's authored
/// hit polygon for this move's clip over the synthetic box. A sheet row name put
/// here would silently take a move off both paths. Per-move ART is a
/// [`vfx`] EVENT; this is how the SWING reads.
///
/// [`strike`] tags every volume `slash_arc`, which is right for a committed
/// swing and wrong for a poke — so this exists for the pokes.
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

/// A CANCEL WINDOW. The timeline IS the cancel table, so a combo route is
/// authored here and nowhere else.
pub fn cancelable(
    mut m: MoveSpec,
    start_s: f32,
    end_s: f32,
    into: &[&str],
    condition: CancelCondition,
) -> MoveSpec {
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

/// AN AUTHORED INTANGIBILITY WINDOW: the owner cannot be hit between these beats.
///
/// ⭐ `WindowTag::Invuln` HAS BEEN AUTHORING VOCABULARY WITH NO WAY TO SAY IT
/// from this module — every helper here builds `Startup`, `Active`, `Recovery`
/// or `Cancelable`, so a move that wanted i-frames had to push a `MoveWindow`
/// by hand. `project_move_defense_windows` has consumed the tag for a while;
/// this is the other end of it.
pub fn invuln(mut m: MoveSpec, start_s: f32, end_s: f32) -> MoveSpec {
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

/// SUPER ARMOR: through this window the body IS hit and does not answer for it.
///
/// ⭐⭐ THE OTHER HALF OF [`invuln`]'S STORY, one variant over and still missing.
/// `WindowTag::Armor` is consumed end to end — `MovePlayback` republishes
/// `BodyCombat::armored` from the live window every tick, and `hit_reaction`
/// gates the launch on `!combat.armored` with tests either side of it — and
/// **no authored move in the tree has ever opened one.** Measured 2026-09-05.
/// The engine has had super armor for a while and the roster had no way to ask.
///
/// ⛔ NOT INVULNERABILITY, AND THE DIFFERENCE IS THE WHOLE MOVE. An armoured
/// body takes the damage; what it does not take is the launch, the hitstun and
/// the recoil lock. ⇒ So armour LOSES to chip and to grabs and wins the trade
/// against one big hit, where i-frames do the opposite — which is why a fighter
/// wants both words and not a switch between them.
pub fn armor(mut m: MoveSpec, start_s: f32, end_s: f32) -> MoveSpec {
    assert!(
        end_s > start_s,
        "armor window on `{}` runs {start_s}s..{end_s}s, which is never open — \
         and an armour window that never opens is invisible in play: the move \
         simply loses trades it looked like it should win",
        m.id,
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

/// FIXED KNOCKBACK: this hit launches the same at 0% and at 200%.
///
/// ⭐⭐ THE THING [`strike`] CANNOT SAY. Its builder takes one `f32` and reads
/// zero as *"this stage decides"* — which is what every caller has always meant
/// — so `Some(0.0)`, a hit whose launch does NOT grow with its victim's damage,
/// was unreachable through it. Its own comment says as much and points here:
/// *"a move that wants FIXED knockback says so on the volume"*. This is that
/// sentence, made callable, so the next move that wants it does not reach into
/// `windows[..].volumes[..]` by hand.
///
/// The customers are moves whose whole identity is landing the same every time:
/// a multi-hit pulse whose carry must not dissolve at high percent, and a hold
/// that is supposed to hold everyone equally.
pub fn fixed_knockback(mut m: MoveSpec) -> MoveSpec {
    for volume in m.windows.iter_mut().flat_map(|w| w.volumes.iter_mut()) {
        volume.knockback_growth = Some(0.0);
    }
    m
}

/// A CONDITIONAL TECHNIQUE ON CONTACT — the engine's `on_hit` seam, applied to
/// every volume the move lands.
///
/// What the landing is CAPABLE of, never what a game does with it: the down-air
/// says it can rebound its attacker and the RULESET decides whether to take it
/// up on that or read the swing as a spike. Compare [`on_contact`], which is a
/// SOUND, and [`strike_tag`], which is how the swing draws.
pub fn on_hit(mut m: MoveSpec, key: &str) -> MoveSpec {
    for volume in m.windows.iter_mut().flat_map(|w| w.volumes.iter_mut()) {
        volume.on_hit = Some(EffectRef::new(key));
    }
    m
}

/// A TAIL THE BODY CANNOT STEER OUT OF. Extends the move to `to_s` with a
/// Recovery window whose `motion_scale` damps the owner's steering — the genre's
/// "you are committed now", authored rather than hardcoded, and enforced
/// body-side so it binds a CPU and a human identically.
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

/// A TAUNT — the one authored move that threatens nobody.
///
/// No volume, no impulse, one committed recovery window: everything a taunt IS
/// is that you cannot act for `duration_s`, which is what makes it a statement.
/// Compose `sfx` / `vfx` onto the result the way every other move does.
pub fn taunt(id: &str, duration_s: f32) -> MoveSpec {
    MoveSpec {
        display_name: None,
        id: id.to_string(),
        clip: ClipBinding {
            clip: "taunt".to_string(),
            // A sheet with no taunt row stands still, which is the right thing
            // for a move whose whole content is standing still.
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
        charge_gesture: ambition_entity_catalog::ChargeGesture::default(),
        repeat: None,
        landing_lag_s: None,
        autocancel_after_s: None,
        sprite_spin_hz: None,
        equips: None,
        flow: None,
    }
}

/// A special whose whole content is its EFFECT: a timeline, a clip, and no
/// volume anywhere on it.
///
/// ⭐ NOT EVERY SPECIAL HITS. A summon, a transformation, a counter-stance and a
/// teleport are all moves whose payload is a technique rather than a box, and
/// [`strike`] cannot express one — its shape is startup / one active volume /
/// recovery, so authoring a hitless move through it means an active window
/// carrying an empty volume list, which reads as *"this hits, for nothing"*.
///
/// The whole timeline is `Startup` up to `commits_at_s` and `Recovery` after,
/// because that IS the shape: everything before the effect is the wind-up you
/// can be punished during, and everything after is the tail you owe for it.
/// Rooted throughout — a special you can stroll out of is not a commitment.
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
        charge_gesture: ambition_entity_catalog::ChargeGesture::default(),
        repeat: None,
        landing_lag_s: None,
        autocancel_after_s: None,
        sprite_spin_hz: None,
        equips: None,
        flow: None,
    }
}

/// One strike on one timeline: startup, one active window carrying one volume,
/// recovery.
///
/// Every move here is that shape, so the authored differences are the ones that
/// MATTER — how long you are committed, how far it reaches, how hard it throws,
/// and how much of the throw scales with the victim's damage.
#[allow(clippy::too_many_arguments)]
/// THE DASH ATTACK's shape, with the fighter supplying only what it hits for.
///
///  the same split [`taunt`] uses: the helper owns the SHAPE — the genre's fast
/// startup, long recovery and forward carry — and the fighter owns the numbers.
/// A dash attack that each of fourteen fighters designed from scratch would be
/// fourteen chances to author something that is not a dash attack.
///
///  the frame shape is the mechanic: it starts faster than a tilt because
/// you already committed to the dash, and it recovers longer because that
/// commitment is what you are paying for. A version with a tilt's recovery would
/// be a strictly better tilt.
pub fn dash_attack(id: &str, shape: DashAttackShape, damage: i32, knockback: f32) -> MoveSpec {
    //  the impulse is FORWARD and lands at the swing, so the move carries the
    // dash's own momentum rather than stopping the body to hit.
    impulse(
        strike(Strike {
            id,
            clip: "dash_attack",
            startup_s: shape.startup_s,
            active_s: shape.active_s,
            recover_s: shape.recover_s,
            //  `reach_px` IS what `reach_of` measures — offset plus
            // half-extent, not the offset alone. A fighter whose tests pin a
            // reach (Carl's `NEAREST_REACH`) has to be able to say the number
            // its own doc says, and a helper that meant something else by the
            // same word made that impossible to write down.
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

/// The frames a dash attack occupies, so a fighter that authored a LAW about
/// its own timings can honour it.
///
///  this is not "the shape became optional". [`DashAttackShape::GENRE`] is
/// still the one statement of what a dash attack is, and nine of the fourteen
/// fighters take it unchanged. The five that do not are the five whose own
/// tests assert a property the genre's numbers break — Oiler's tolerance band,
/// the Oni's 3x recovery law, Carl's reach-monotonic line, George's poke/commit
/// gap — and each of those is a fact about that CHARACTER that a shared default
/// has no standing to overrule.
///
///  the guards found every one of them. A generic move stamped over
/// fourteen fighters violated five authored invariants, and five character
/// censuses said so on the first run.
#[derive(Clone, Copy, Debug)]
pub struct DashAttackShape {
    pub startup_s: f32,
    pub active_s: f32,
    pub recover_s: f32,
    pub reach_px: f32,
}

impl DashAttackShape {
    /// The genre's dash attack. Faster than a tilt because the dash is
    /// already committed, and recovering longer because that commitment is what
    /// is being paid for — a version with a tilt's recovery is a better tilt.
    pub const GENRE: Self = Self {
        startup_s: 0.05,
        active_s: 0.09,
        recover_s: 0.26,
        reach_px: 40.0,
    };
}

/// ONE STRIKE, AS VALUES — the shape 294 of this repertoire's moves already had.
///
/// ⛔⛔ **NAMED FIELDS, and that is the point of the type.** This was twelve
/// POSITIONAL arguments; Alice's jab read `"challenge", "jab", 0.05, 0.05,
/// 0.13, (24.0, 0.0), (17.0, 13.0), 3, 48.0, 1.05, None, None`. Three of those
/// numbers are timings and two are
/// knockback, all `f32`, and two are `(f32, f32)` geometry — so a transposition
/// inside either group is a SILENT change to a fighter's feel: the compiler
/// cannot see it, and no test asserts any individual fighter's numbers.
///
/// ⭐ it is also what makes the authored `smash_fighter` facet a derive rather
/// than a redesign — a named-field record of pure values maps one-to-one onto
/// serde, which `CaptureKitAuthoring` already demonstrated for the capture kit.
/// ⛔ this type is NOT `Serialize` yet, deliberately: adding the derive is the
/// facet's slice, and doing it here would freeze a wire shape before a customer
/// has asked for one.
#[derive(Debug, Clone, PartialEq)]
pub struct Strike<'a> {
    /// The move id. Unique within the kit.
    pub id: &'a str,
    /// The animation row. Falls back through `attack_side` → `attack` → `slash`
    /// → `idle`, so a missing clip never costs the move its gameplay.
    pub clip: &'a str,
    /// The tell, before anything is dangerous.
    pub startup_s: f32,
    /// How long the volume is LIVE.
    pub active_s: f32,
    /// The tail after it, during which the body is committed.
    pub recover_s: f32,
    /// Volume centre, body-local. Mirrors with facing.
    pub offset: (f32, f32),
    /// Volume half-extents. ⚠ the OTHER `(f32, f32)`, and the one a
    /// transposition with `offset` used to hide in.
    pub half_extents: (f32, f32),
    pub damage: i32,
    /// Base launch speed.
    pub knockback: f32,
    /// How much the launch grows with the victim's damage.
    pub knockback_growth: f32,
    /// `None` lets the shared rule derive it from the geometry.
    pub launch_dir: Option<(f32, f32)>,
    /// What LANDING this hit can do beyond damage. Today exactly one move uses
    /// it: the down-air says it is capable of rebounding its attacker, and the
    /// RULESET (`DeclaredCombatRules::downward_hit`) decides whether this game
    /// takes it up on that or reads the swing as a spike instead.
    pub on_hit: Option<EffectRef>,
}

impl<'a> Strike<'a> {
    /// A named, zero-damage placeholder. Every real field is still stated at
    /// the call site — this exists so a future field does not edit 294 literals.
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
/// Deliberately small: everything a pulse needs that the finisher does not
/// already state. A pulse is a WEAK hit that holds — the launch is the
/// finisher's job.
#[derive(Clone, Copy, Debug)]
pub struct Pulse {
    /// Where the pulse's box sits, body-local, and how big it is.
    pub offset: (f32, f32),
    pub half_extents: (f32, f32),
    /// Chip damage. Intermediate pulses are cheap by design — the move is paid
    /// for by its finisher, and a multi-hit whose pulses hurt is a better move
    /// than its own ending.
    pub damage: i32,
    /// How long one pulse is live, and the GAP before the next one.
    ///
    /// ⛔⛔ THE GAP IS LOAD-BEARING, not spacing. The move runtime's re-hit rule
    /// lets SEPARATED Active windows hit the same victim again and refuses it
    /// across a contiguous track — so a multi-hit that authored one long window,
    /// or windows that touch, lands exactly once.
    pub active_s: f32,
    pub gap_s: f32,
    /// The hold itself.
    pub autolink: ambition_entity_catalog::AutolinkVolume,
}

/// A MULTI-HIT: `pulses` holding hits, then the strike you pass in as the
/// finisher.
///
/// ⭐ A COMBINATOR over [`strike`], not a second builder — the finisher is an
/// ordinary strike with an ordinary launch, and this inserts the lead-in in
/// front of it. That is the genre's shape stated directly: the intermediate hits
/// keep the victim inside the next box and only the LAST one sends it anywhere.
///
/// ⛔ NOT a capture and not a per-character mechanism: what holds the victim is
/// `HitVolume::autolink` on the pulse volumes, which any move may author.
pub fn multihit(m: MoveSpec, pulses: usize, pulse: Pulse) -> MoveSpec {
    if pulses == 0 {
        return m;
    }
    let mut m = m;
    // The lead-in occupies the gap the finisher's Startup already reserves, so a
    // multi-hit does not silently become slower than the strike it was built
    // from — the AUTHOR chose that startup and the finisher still owns it.
    let lead_in = pulses as f32 * (pulse.active_s + pulse.gap_s);
    let shift = |t: f32| t + lead_in;
    let finish_start = active_start(&m);
    for window in &mut m.windows {
        // Everything from the finisher's Active onward moves back; its Startup
        // stretches to cover the lead-in instead of being duplicated.
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
                // A pulse authors NO launch of its own: the autolink decides the
                // victim's velocity outright, and a knockback beside it would be
                // a second answer to one question. The number still feeds the
                // hitstun the pulse owes.
                knockback: 1.0,
                knockback_growth: Some(0.0),
                launch_dir: None,
                reaction: Some(ambition_entity_catalog::VolumeReaction::Autolink(
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
    // The runtime reads windows in authored order for the sweetspot rule, so the
    // lead-in must sort before the finisher rather than merely start earlier.
    m.windows.sort_by(|a, b| {
        a.start_s
            .total_cmp(&b.start_s)
            .then(a.end_s.total_cmp(&b.end_s))
    });
    m
}

/// A GUST: a volume that SHOVES and does not hurt.
///
/// ⭐⭐ EVERYTHING THIS NEEDS WAS ALREADY IN THE ENGINE AND NO FIGHTER USED IT.
/// `VolumeReaction::Windbox` ships with a validation error for a windbox that
/// carries damage, and `hit_reaction` already sets `flinchless` from it — *"this
/// is a push, not a hit"*. What was missing was a way to SAY it: authoring a
/// gust meant hand-building a `MoveWindow` and remembering three separate
/// invariants, and nobody did. Measured 2026-09-05: zero authored windboxes on
/// the entire roster.
///
/// ⛔ THE THREE INVARIANTS THIS EXISTS TO HOLD, because each one is silent when
/// broken. Damage must be ZERO or the catalog rejects the move
/// (`WindboxWithDamage`). Knockback growth must be FIXED, or a gust shoves a
/// damaged fighter further than a fresh one — which is a hit's rule, not wind's.
/// And the slash arc must go: `strike` draws one from the spawned volume, so a
/// gust built on it swings a visible blade that does no damage.
pub struct Gust<'a> {
    /// The move id. Unique within the kit.
    pub id: &'a str,
    /// The animation row, with `strike`'s fallbacks.
    pub clip: &'a str,
    /// The tell, before the air moves.
    pub startup_s: f32,
    /// How long the gust BLOWS.
    pub active_s: f32,
    /// The tail after it.
    pub recover_s: f32,
    /// Volume centre, body-local. Mirrors with facing.
    pub offset: (f32, f32),
    /// Volume half-extents, body-local.
    pub half_extents: (f32, f32),
    /// How hard it shoves. The same units as a strike's `knockback`, and it is
    /// the ONLY thing this move does to whoever it catches.
    pub push: f32,
    /// Which way it shoves, body-local: `+x` toward facing, `+y` gravity-down.
    ///
    /// ⚠ AUTHORED RATHER THAN DERIVED, unlike a strike's `None`. The shared rule
    /// derives a launch from where the victim stood relative to the volume, which
    /// is right for a blow and wrong for wind: a gust blows ONE WAY regardless of
    /// who walked into which side of it.
    pub push_dir: (f32, f32),
    /// Does it keep pushing while they stand in it?
    ///
    /// ⭐ `true` opts out of the hit-once set — correct for a sustained wind and
    /// wrong for a one-shot shove, which is why `WindboxVolume::repeating` is
    /// authored rather than assumed. A `false` gust is a single hard blast.
    pub sustained: bool,
}

/// Author a gust: [`strike`]'s timeline, with the three windbox invariants held.
///
/// # Panics
///
/// If `push` is not positive. A gust that shoves nowhere is a move that spends a
/// startup and a recovery to do nothing, and the volume it spawns is invisible —
/// so there is no frame at which a player could see that it had failed.
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
        // ⛔ ZERO, AND THE CATALOG ENFORCES IT: `WindboxWithDamage` is a
        // validation error, so a gust that chipped would not load.
        damage: 0,
        knockback: spec.push,
        // Set on the volumes below rather than here: the builder reads its own
        // zero as "this stage decides", which is the opposite of what wind wants.
        knockback_growth: 0.0,
        launch_dir: Some(spec.push_dir),
        on_hit: None,
    });
    for volume in m.windows.iter_mut().flat_map(|w| w.volumes.iter_mut()) {
        volume.reaction = Some(ambition_entity_catalog::VolumeReaction::Windbox(
            ambition_entity_catalog::WindboxVolume {
                repeating: spec.sustained,
            },
        ));
        // ⛔ NO SLASH. `strike` draws its arc from the spawned volume, so without
        // this the fighter swings a blade that does no damage and the player is
        // told the wrong thing about a move whose whole point is that it is not
        // a hit. ⚠ There is no wind art yet, so a gust currently draws NOTHING —
        // which is honest and is a known gap rather than a choice.
        volume.vfx = None;
    }
    fixed_knockback(m)
}

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
            //  THE AUTHORED FALLBACK CHAIN (sprite redirect P0/P1,
            // ). A move names the exact row it wants — `smash_forward`,
            // `air_back` — and this is what it settles for when a sheet does not
            // have it. Robot v3's new sheet has 132 rows and draws the exact
            // clip; a lean sheet with `attack` draws that; one with only `slash`
            // and `idle` still plays.
            //
            //  the structural fallbacks are DIRECTIONAL first — an up-tilt
            // that cannot find `attack_up` should look like a side swing before
            // it looks like nothing, and `attack_side` is the row every fighter
            // sheet in the repo has had for a year.
            //
            //  a missing clip must never cost the move its GAMEPLAY: the
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
                    // The BUILDER's zero still means "this stage decides", which
                    // is what every caller of it has always meant. A move that
                    // wants FIXED knockback says so on the volume — see
                    // `HitVolume::knockback_growth` — because a builder that
                    // took one number could not express both.
                    knockback_growth: (knockback_growth > 0.0).then_some(knockback_growth),
                    launch_dir,
                    on_hit,
                    // The blade tag: the move runtime draws the slash from the
                    // SAME spawned volume, so the hitbox and the arc can never
                    // point different ways.  a POKE wants the other tag — see
                    // [`strike_tag`].
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
        charge_gesture: ambition_entity_catalog::ChargeGesture::default(),
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
            autolink: ambition_entity_catalog::AutolinkVolume {
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

    /// ⛔⛔ THE GAPS ARE LOAD-BEARING, NOT SPACING. The move runtime's re-hit
    /// rule lets SEPARATED Active windows strike the same victim again and
    /// refuses it across a contiguous track — so a multi-hit authored as one long
    /// window, or as windows that touch, lands exactly ONCE and the whole
    /// mechanic silently does not exist.
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

    /// The pulses HOLD and the finisher LAUNCHES — which is the whole shape, and
    /// the thing a careless edit inverts.
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

    /// The finisher moves BACK by the lead-in rather than being overwritten, and
    /// the move gets longer by exactly that much — a multihit must not silently
    /// eat its own ending or its recovery.
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

    /// ⭐ THE POISON: zero pulses is the strike it was built from, untouched.
    /// Without this, a combinator that always inserted something would pass every
    /// assertion above.
    #[test]
    fn a_multihit_of_zero_pulses_is_the_plain_strike() {
        let base = finisher();
        let m = multihit(base.clone(), 0, pulse());
        assert_eq!(m.duration_s, base.duration_s);
        assert_eq!(m.windows.len(), base.windows.len());
        assert_eq!(actives(&m).len(), 1);
    }
}
