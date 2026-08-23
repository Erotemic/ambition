//! The primitives a character's move table is written with — shared, because
//! the second character to author one must not begin by copying the first.
//!
//! Mary-O and Sanic are registered by their own demos, both were 0/16 on the smash grid, and
//! neither demo depends on Ambition's content crate — so completing their kits meant either a
//! fourth copy of these helpers or moving the one copy to a crate everybody already has.
//! `ambition_characters` is where the character model lives and where `moveset_prefabs` already
//! derives a table from an action set; authoring one by hand belongs beside it.
//!
//!  `ambition_demo_smash` still carries its OWN fork of these (`crate::moveset`
//! in that crate, with a `Feel` tag this one has no concept of). Unifying it is
//! its own change and would expose what the fork hides; it is not this one.
//!
//! They were never robot-specific — `strike` is *startup, one active window carrying one volume,
//! recovery*, which is the shape of nearly every move in the genre.
//!
//!  a move states what it IS, never what a mode does with it. Startup,
//! active frames, recovery, hitbox geometry, damage, base launch and growth are
//! properties of the swing; percent, stocks, blast zones and DI are the
//! RULESET's. That is what lets one table read as Hollow-Knight combat in one
//! game and a platform fighter in another.

use crate::moveset_prefabs::SLASH_ARC_VFX;
use ambition_entity_catalog::{
    ClipBinding, EffectRef, HitVolume, ImpulseMode, MoveEvent, MoveEventKind, MoveGates, MoveSpec,
    MoveWindow, VolumeShape, WindowTag,
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
/// [`SLASH_POKE_VFX`](ambition_characters::moveset_prefabs::SLASH_POKE_VFX))
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
        landing_lag_s: None,
        autocancel_after_s: None,
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
                    knockback_growth,
                    launch_dir,
                    on_hit,
                    // The blade tag: the move runtime draws the slash from the
                    // SAME spawned volume, so the hitbox and the arc can never
                    // point different ways.  a POKE wants the other tag — see
                    // [`strike_tag`].
                    vfx: Some(SLASH_ARC_VFX.to_string()),
                    hit_sfx: None,
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
        landing_lag_s: None,
        autocancel_after_s: None,
    }
}
