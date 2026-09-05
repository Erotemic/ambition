//! Move authoring — the build-time half of the Smash model: the functions that
//! turn authored specs (`MeleeActionSpec`/`RangedActionSpec`), tunable params
//! (`Simple{Melee,Ranged,Charge}Params`), and the `MovePrefabRegistry` into
//! `MoveSpec`s, plus `build_actor_moveset` which assembles an
//! actor's full `MovesetContract` from its catalog + worn equipment.
//  EXPLICIT, and the point is the MEASUREMENT.
// This was `use super::*`, which is how a module's real coupling stays unknown:
// the bulk move this row needs cannot be planned against a glob. Made explicit,
// what `prefabs.rs` actually needs is three groups, and only one of them is a
// problem.
//
//  the six SFX/VFX constants are the whole remaining coupling to this crate
// — plain `&str` presentation ids, the same class as `POGO_BOUNCE_KEY` before it
// was lowered, so they travel with the builders whenever the builders move.
// Everything else is `ambition_entity_catalog` and `ambition_characters`, both
// of which sit at or below the destination.
use crate::brain::action_set::{MeleeActionSpec, RangedActionSpec, RangedStyle, SpecialActionSpec};
use ambition_entity_catalog::{
    ClipBinding, EffectRef, HitVolume, MoveEvent, MoveEventKind, MoveSpec, MoveWindow,
    MovesetContract, VolumeShape, WindowTag, ATTACK_VERB, RANGED_VERB, SMASH_VERB, SPECIAL_VERB,
};

/// [`HitVolume::vfx`] tags the move runtime knows (§7.2): the sweeping slash
/// arc and the grounded down-tilt's horizontal poke. Unknown tags draw the arc
/// (never a silent drop — a tagged volume asked for presentation).
pub const SLASH_ARC_VFX: &str = "slash_arc";
pub const SLASH_POKE_VFX: &str = "slash_poke";

/// The SFX cue a plain swing fires. Names the engine's procedural `slash` cue
/// (`ambition_sfx::ids::PLAYER_SLASH` = `"player.slash"`) so the audio runtime
/// resolves it to the guaranteed procedural sound.
pub const SWING_SFX_CUE: &str = "player.slash";
// The test is does character PREPARATION call it, not *was it next to something preparation
// calls*: `prepare_character` never reaches the overlay, whose only production caller is
// `avatar/starting_character.rs`, the protagonist road. One character's private sound policy is not
// the character DOMAIN.  they are back in `ambition_platformer2d::combat::moveset`, beside the compile-time hash
// pins that correctly never left it.

/// Cue-id hash pins live in `ambition_combat`, which owns the SFX id table.
/// This crate owns authored cue text without depending on the audio registry.
const _: () = ();

/// Convert an authored [`MeleeActionSpec`] into the base `"attack"` move.
/// The timeline maps to startup/active/recovery windows and one body-local hit
/// volume. Directional and aerial variants derive from this move so all melee
/// executes through the shared moveset runtime.
pub fn attack_move_from_melee(spec: &MeleeActionSpec) -> MoveSpec {
    let (windup, active, recover, damage, reach) = spec.timeline();
    // The authored-melee path is now a thin adapter over the `simple_melee`
    // engine prefab (A2): the MeleeActionSpec timeline becomes prefab params.
    // Byte-identical — the clamps + volume shape live in the prefab core.
    simple_melee(&SimpleMeleeParams {
        windup_s: windup,
        active_s: active,
        recover_s: recover,
        damage,
        reach_px: reach,
        knockback: 120.0,
        // The authored-melee adapter keeps the engine-default swing presentation
        // (byte-parity with the pre-CM5 path); per-move sfx/vfx is authored on
        // the prefab RON rows, not synthesized here.
        swing_sfx: None,
        swing_vfx: None,
        hit_sfx: None,
    })
}

/// Params for the [`simple_melee`] engine prefab (A2 / R2.3) — a forward swing
/// as authored DATA. Every field defaults, so a roster prefab row omits what it
/// doesn't tune (`prefab: "simple_melee"` with empty params = a default jab).
/// `sword_slash` is literally this prefab + params, zero new code.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SimpleMeleeParams {
    #[serde(default = "smp_windup")]
    pub windup_s: f32,
    #[serde(default = "smp_active")]
    pub active_s: f32,
    #[serde(default = "smp_recover")]
    pub recover_s: f32,
    #[serde(default = "smp_damage")]
    pub damage: i32,
    #[serde(default = "smp_reach")]
    pub reach_px: f32,
    #[serde(default = "smp_knockback")]
    pub knockback: f32,
    /// CM5: the SFX cue this swing fires at its Active edge. `None` = the engine
    /// default (`SWING_SFX_CUE`), so an unauthored row is byte-parity; an
    /// authored row makes the move sound distinct (a heavy smash thuds, a jab
    /// snaps) with zero code.
    #[serde(default)]
    pub swing_sfx: Option<String>,
    /// CM5: an OPTIONAL cosmetic effect id — the NAME of a row on one of the
    /// shipped FX spritesheets (`ambition_sprite_sheet::fx`) — emitted at the
    /// Active edge on top of the slash arc. `None` = no extra burst (parity).
    /// Lets a launcher `"starburst"`, a smash `"shockwave"`, a signature
    /// `"sonic_boom"`. A typo is a validation error, never silent.
    #[serde(default)]
    pub swing_vfx: Option<String>,
    /// CM8: the CONTACT sound this swing makes when it LANDS on a body (an
    /// `SfxId` name, e.g. `"player.slash"`), distinct from `swing_sfx` (the
    /// whoosh at the Active edge). This is how a sword and a goblin claw are
    /// heard apart — it rides the volume to the ONE victim-side reaction and
    /// overrides the victim's default hurt sound. `None` = the victim's own
    /// `HurtFeedback` sound (parity).
    #[serde(default)]
    pub hit_sfx: Option<String>,
}

fn smp_windup() -> f32 {
    0.12
}
fn smp_active() -> f32 {
    0.10
}
fn smp_recover() -> f32 {
    0.18
}
fn smp_damage() -> i32 {
    1
}
fn smp_reach() -> f32 {
    36.0
}
fn smp_knockback() -> f32 {
    120.0
}

impl Default for SimpleMeleeParams {
    fn default() -> Self {
        Self {
            windup_s: smp_windup(),
            active_s: smp_active(),
            recover_s: smp_recover(),
            damage: smp_damage(),
            reach_px: smp_reach(),
            knockback: smp_knockback(),
            swing_sfx: None,
            swing_vfx: None,
            hit_sfx: None,
        }
    }
}

/// The `simple_melee` prefab core: a forward Startup/Active(one Rect hit)/Recovery
/// swing on the owner's proper-time clock. Shared by the authored-melee adapter
/// ([`attack_move_from_melee`]) and the prefab registry.
pub fn simple_melee(p: &SimpleMeleeParams) -> MoveSpec {
    let windup = p.windup_s.max(0.0);
    let active = p.active_s.max(0.02);
    let recover = p.recover_s.max(0.0);
    let duration = windup + active + recover;
    // Forward rect: centered just past the body, extending to `reach`, with a
    // torso-height band. Authored body-local (x = side/forward); the runtime
    // mirrors it by facing and rotates it into the gravity frame at spawn.
    let half_x = (p.reach_px * 0.5).max(8.0);
    let volume = HitVolume {
        // An ordinary swing: it hurts. A gust is authored, never inherited.
        // CM8: the authored contact sound rides the volume to the victim-side
        // reaction (a sword vs a claw); unauthored swings fall back to the
        // victim's own hurt sound.
        hit_sfx: p.hit_sfx.clone(),
        shape: VolumeShape::Rect {
            offset: (p.reach_px * 0.6, 0.0),
            half_extents: (half_x, 16.0),
        },
        damage: p.damage.max(1),
        knockback: p.knockback,
        // Prefab swings are flat-knockback; percent growth is authored on
        // explicit RON volumes (CM1) — a prefab growth param can follow.
        knockback_growth: None,
        launch_dir: None,
        // A plain swing lands no on-hit technique; directional variants (a
        // down-air pogo) author `on_hit` per-move (R2.5 player-melee fold).
        on_hit: None,
        // A bladed swing (§7.1/§7.2): draws the slash arc from the spawned
        // volume, and — when the owner's sprite manifest authors a hit polygon
        // for this move's clip — swings THAT authored blade instead of this
        // synthetic rect (the rect is the fallback for unmanifested bodies).
        vfx: Some(SLASH_ARC_VFX.to_string()),
        reaction: None,
    };
    MoveSpec {
        display_name: None,
        landing_lag_s: None,
        autocancel_after_s: None,
        sprite_spin_hz: None,
        equips: None,
        flow: None,
        id: ATTACK_VERB.to_string(),
        clip: ClipBinding {
            clip: "attack_side".to_string(),
            fallbacks: vec!["slash".to_string(), "idle".to_string()],
        },
        duration_s: duration,
        windows: vec![
            MoveWindow {
                start_s: 0.0,
                end_s: windup,
                tag: WindowTag::Startup,
                volumes: vec![],
                sustain_effect: None,
                motion_scale: 1.0,
            },
            MoveWindow {
                start_s: windup,
                end_s: windup + active,
                tag: WindowTag::Active,
                volumes: vec![volume],
                sustain_effect: None,
                motion_scale: 1.0,
            },
            MoveWindow {
                start_s: windup + active,
                end_s: duration,
                tag: WindowTag::Recovery,
                volumes: vec![],
                sustain_effect: None,
                motion_scale: 1.0,
            },
        ],
        events: {
            // The swing SFX at the Active edge (authored cue or the engine
            // default), plus — if the row authored one — a cosmetic burst at the
            // same edge (CM5 per-move presentation).
            let mut events = vec![MoveEvent {
                at_s: windup,
                kind: MoveEventKind::Sfx {
                    cue: p
                        .swing_sfx
                        .clone()
                        .unwrap_or_else(|| SWING_SFX_CUE.to_string()),
                },
            }];
            if let Some(effect) = &p.swing_vfx {
                events.push(MoveEvent {
                    at_s: windup,
                    kind: MoveEventKind::Vfx {
                        effect: effect.clone(),
                        at: (0.0, 0.0),
                        scale: 1.0,
                        sfx: None,
                    },
                });
            }
            events
        },
        gates: Default::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
        smash_charge: None,
        charge_gesture: ambition_entity_catalog::ChargeGesture::default(),
        repeat: None,
    }
}

/// The projectile IS the damage (spawned by the event through the shared projectile-request
/// consumer), so the move carries NO hit volume. Windup/recovery are authored defaults per spec
/// kind (deferred-tuning); the body-side refire cooldown remains the hard rate floor, the move
/// duration an additional cadence gate.
pub fn fire_move_from_ranged(spec: &RangedActionSpec) -> MoveSpec {
    // Draw/settle timing per weapon kind — Arrow winds up slowest (per its doc),
    // Pistol snappiest. New authored defaults (ranged had none); tune later.
    let (windup, recover) = match spec.style {
        RangedStyle::Pistol => (0.08, 0.15),
        RangedStyle::Rock => (0.12, 0.18),
        RangedStyle::Bolt => (0.18, 0.20),
        RangedStyle::Arrow => (0.28, 0.22),
    };
    // Thin adapter over the `simple_ranged` engine prefab (A2). The projectile
    // still comes from the owner's live ActionSet.ranged at the fire event.
    simple_ranged(&SimpleRangedParams {
        windup_s: windup,
        recover_s: recover,
    })
}

/// Params for the [`simple_ranged`] engine prefab (A2). The move carries NO
/// projectile spec — the fire event samples the owner's live `ActionSet.ranged`
/// — so its only knobs are the draw/settle timings.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct SimpleRangedParams {
    #[serde(default = "srp_windup")]
    pub windup_s: f32,
    #[serde(default = "srp_recover")]
    pub recover_s: f32,
}

fn srp_windup() -> f32 {
    0.12
}
fn srp_recover() -> f32 {
    0.18
}

impl Default for SimpleRangedParams {
    fn default() -> Self {
        Self {
            windup_s: srp_windup(),
            recover_s: srp_recover(),
        }
    }
}

/// The `simple_ranged` prefab core: a Startup(draw)/Recovery(settle) timeline
/// whose single [`MoveEventKind::Ranged`] fire event spawns the owner's shot.
pub fn simple_ranged(p: &SimpleRangedParams) -> MoveSpec {
    let windup = p.windup_s.max(0.0);
    let recover = p.recover_s.max(0.0);
    let duration = windup + recover;
    MoveSpec {
        display_name: None,
        landing_lag_s: None,
        autocancel_after_s: None,
        sprite_spin_hz: None,
        equips: None,
        flow: None,
        id: RANGED_VERB.to_string(),
        clip: ClipBinding {
            clip: "shoot".to_string(),
            fallbacks: vec!["attack_side".to_string(), "idle".to_string()],
        },
        duration_s: duration,
        windows: vec![
            MoveWindow {
                start_s: 0.0,
                end_s: windup,
                tag: WindowTag::Startup,
                volumes: vec![],
                sustain_effect: None,
                motion_scale: 1.0,
            },
            // No Active hit volume — the projectile spawned by the fire event is the
            // damage. The Recovery window just holds the post-shot settle.
            MoveWindow {
                start_s: windup,
                end_s: duration,
                tag: WindowTag::Recovery,
                volumes: vec![],
                sustain_effect: None,
                motion_scale: 1.0,
            },
            // THE POKE MUST BE CANCELLABLE INTO THE MELEE FINISH.
            //
            // The fighter brain's own comment states the intent: *"Fire WHILE
            // closing, not instead of closing: a ranged poke advances toward the
            // target (throwing the poke on the way in to the melee finish)
            // rather than camping at range."* This prefab authored NO cancelable
            // window, so nothing could ever interrupt it — and a move "plays to
            // completion before another starts". A brain that pokes on the way in
            // therefore starves its own melee: the press arrives, the trigger is
            // reached, the gesture is armed, and `cancel_permits` says no.
            //
            // The window covers the SETTLE only, never the draw — a shot still
            // commits once it is started, so this cannot cancel the fire event
            // away. `OnWhiff` would be wrong here: the poke's projectile lands
            // (or not) long after the settle, so its hit state says nothing about
            // whether closing is the right follow-up.
            MoveWindow {
                start_s: windup,
                end_s: duration,
                tag: WindowTag::Cancelable {
                    into: vec![
                        ATTACK_VERB.to_string(),
                        SMASH_VERB.to_string(),
                        "any_attack".to_string(),
                    ],
                    condition: ambition_entity_catalog::CancelCondition::Always,
                },
                volumes: vec![],
                sustain_effect: None,
                motion_scale: 1.0,
            },
        ],
        events: vec![MoveEvent {
            at_s: windup,
            kind: MoveEventKind::Ranged,
        }],
        gates: Default::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
        smash_charge: None,
        charge_gesture: ambition_entity_catalog::ChargeGesture::default(),
        repeat: None,
    }
}

/// Params for the [`simple_charge`] engine prefab (A2) — a hold-then-release
/// heavy hit the demos need (SMB1's crouch-charge, a wind-up smash). The
/// `charge_s` Startup window is the hold; the `active_s` Active window lands one
/// forward Rect hit sized from `reach_px`, then `recover_s` settle.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SimpleChargeParams {
    #[serde(default = "scp_charge")]
    pub charge_s: f32,
    #[serde(default = "scp_active")]
    pub active_s: f32,
    #[serde(default = "scp_recover")]
    pub recover_s: f32,
    #[serde(default = "scp_damage")]
    pub damage: i32,
    #[serde(default = "scp_reach")]
    pub reach_px: f32,
    #[serde(default = "scp_knockback")]
    pub knockback: f32,
    /// CM3 smash-charge payoff: the multiplier a fully-charged release applies to
    /// damage + knockback (`1.0 → smash_charge_mult` by charge fraction). DEFAULT
    /// `1.0` = no scaling (parity); a smash roster authors e.g. `2.0`.
    #[serde(default = "scp_charge_mult")]
    pub smash_charge_mult: f32,
    /// CM5: the release SFX cue (`None` = engine default). See
    /// [`SimpleMeleeParams::swing_sfx`].
    #[serde(default)]
    pub swing_sfx: Option<String>,
    /// CM5: an optional cosmetic burst at the Active edge (`None` = parity). See
    /// [`SimpleMeleeParams::swing_vfx`].
    #[serde(default)]
    pub swing_vfx: Option<String>,
    /// CM8: the CONTACT sound when the charged strike lands. See
    /// [`SimpleMeleeParams::hit_sfx`].
    #[serde(default)]
    pub hit_sfx: Option<String>,
}

fn scp_charge() -> f32 {
    0.45
}
fn scp_active() -> f32 {
    0.12
}
fn scp_recover() -> f32 {
    0.30
}
fn scp_damage() -> i32 {
    3
}
fn scp_reach() -> f32 {
    44.0
}
fn scp_knockback() -> f32 {
    260.0
}
fn scp_charge_mult() -> f32 {
    1.0
}

impl Default for SimpleChargeParams {
    fn default() -> Self {
        Self {
            charge_s: scp_charge(),
            active_s: scp_active(),
            recover_s: scp_recover(),
            damage: scp_damage(),
            reach_px: scp_reach(),
            knockback: scp_knockback(),
            smash_charge_mult: scp_charge_mult(),
            swing_sfx: None,
            swing_vfx: None,
            hit_sfx: None,
        }
    }
}

/// The `simple_charge` prefab core.
pub fn simple_charge(p: &SimpleChargeParams) -> MoveSpec {
    let charge = p.charge_s.max(0.0);
    let active = p.active_s.max(0.02);
    let recover = p.recover_s.max(0.0);
    let duration = charge + active + recover;
    let half_x = (p.reach_px * 0.5).max(8.0);
    let volume = HitVolume {
        // An ordinary swing: it hurts.
        // CM8: authored contact sound for the charged strike (see simple_melee).
        hit_sfx: p.hit_sfx.clone(),
        shape: VolumeShape::Rect {
            offset: (p.reach_px * 0.6, 0.0),
            half_extents: (half_x, 18.0),
        },
        damage: p.damage.max(1),
        knockback: p.knockback,
        knockback_growth: None,
        launch_dir: None,
        on_hit: None,
        // A charge is a bladed strike too — same slash + authored-blade rules.
        vfx: Some(SLASH_ARC_VFX.to_string()),
        reaction: None,
    };
    MoveSpec {
        display_name: None,
        landing_lag_s: None,
        autocancel_after_s: None,
        sprite_spin_hz: None,
        equips: None,
        flow: None,
        id: "charge".to_string(),
        clip: ClipBinding {
            clip: "attack_side".to_string(),
            fallbacks: vec!["slash".to_string(), "idle".to_string()],
        },
        duration_s: duration,
        windows: vec![
            MoveWindow {
                start_s: 0.0,
                end_s: charge,
                tag: WindowTag::Startup,
                volumes: vec![],
                sustain_effect: None,
                motion_scale: 1.0,
            },
            MoveWindow {
                start_s: charge,
                end_s: charge + active,
                tag: WindowTag::Active,
                volumes: vec![volume],
                sustain_effect: None,
                motion_scale: 1.0,
            },
            MoveWindow {
                start_s: charge + active,
                end_s: duration,
                tag: WindowTag::Recovery,
                volumes: vec![],
                sustain_effect: None,
                motion_scale: 1.0,
            },
        ],
        events: {
            let mut events = vec![MoveEvent {
                at_s: charge,
                kind: MoveEventKind::Sfx {
                    cue: p
                        .swing_sfx
                        .clone()
                        .unwrap_or_else(|| SWING_SFX_CUE.to_string()),
                },
            }];
            if let Some(effect) = &p.swing_vfx {
                events.push(MoveEvent {
                    at_s: charge,
                    kind: MoveEventKind::Vfx {
                        effect: effect.clone(),
                        at: (0.0, 0.0),
                        scale: 1.0,
                        sfx: None,
                    },
                });
            }
            events
        },
        gates: Default::default(),
        start_impulse: None,
        // CM3: the charge move's payoff — the authored release multiplier.
        smash_charge_mult: p.smash_charge_mult,
        smash_charge: None,
        charge_gesture: ambition_entity_catalog::ChargeGesture::default(),
        repeat: None,
    }
}

/// Fold a body's authored melee (`ActionSet.melee`) and ranged (`ActionSet.ranged`)
/// into its moveset as the `"attack"` and `"ranged"` moves, merging with any
/// signature-move repertoire. The single seam every actor-spawn path calls so a
/// body's basic swing, its shot, and its specials live in ONE `MovesetContract`.
/// Returns `None` when the body has none of them.
/// A body-local direction to transform the base swing's reach into.
#[derive(Clone, Copy)]
enum Dir {
    Fwd,
    Up,
    Down,
    Back,
}

/// Derive a body's DIRECTIONAL melee variants from its base `"attack"` move by
/// transforming that move's hit volume: the forward reach rotates up / down
/// (its dimensions swap) or mirrors behind, so a character's ONE authored swing
/// yields up-/down-tilt + the four aerials + the pogo down-air, all scaled by
/// ITS own reach and body. Presentation clip and grounded gate change per
/// variant; timing/damage/knockback are inherited. Each entry is
/// `(verb id, MoveSpec)` — the verb the directional trigger resolves to.
fn directional_attack_variants(base: &MoveSpec) -> Vec<(String, MoveSpec)> {
    fn xf(dir: Dir, offset: (f32, f32), half: (f32, f32)) -> ((f32, f32), (f32, f32)) {
        let ((ox, oy), (hx, hy)) = (offset, half);
        match dir {
            Dir::Fwd => ((ox, oy), (hx, hy)),
            // +x reach → up (-y); box dimensions swap under the quarter turn.
            Dir::Up => ((0.0, -ox), (hy, hx)),
            Dir::Down => ((0.0, ox), (hy, hx)),
            Dir::Back => ((-ox, oy), (hx, hy)),
        }
    }
    // `label` is the genre-facing name for the directional variant; other moves
    // keep the title-cased id fallback.
    let variant =
        |id: &str, label: &str, clip: &str, grounded: bool, dir: Dir, pogo: bool| -> MoveSpec {
            let mut m = base.clone();
            m.id = id.to_string();
            m.display_name = Some(label.to_string());
            m.clip.clip = clip.to_string();
            m.gates.grounded = Some(grounded);
            for w in &mut m.windows {
                if !matches!(w.tag, WindowTag::Active) {
                    continue;
                }
                for v in &mut w.volumes {
                    if let VolumeShape::Rect {
                        offset,
                        half_extents,
                    } = v.shape
                    {
                        let (o, h) = xf(dir, offset, half_extents);
                        v.shape = VolumeShape::Rect {
                            offset: o,
                            half_extents: h,
                        };
                    }
                    if pogo {
                        // The down-air's landing pogo — an engine on-hit technique
                        // Body contacts consume the resolved victim hit; genuine world pogo surfaces use the separate world-contact path.
                        v.on_hit = Some(EffectRef::new(crate::technique::POGO_BOUNCE_KEY));
                    }
                    // The grounded down-tilt reads as a kneeling forward poke, not a
                    // sweep (mirrors the bespoke path's `slash_kind`: Down → Poke);
                    // every other direction keeps the base swing's arc.
                    if matches!(dir, Dir::Down) && grounded && v.vfx.is_some() {
                        v.vfx = Some(SLASH_POKE_VFX.to_string());
                    }
                }
            }
            m
        };
    // NB: the second `variant(...)` arg is the CLIP the strike resolves its
    // AUTHORED hitbox polygon from the App-local resolver → a plain manifest
    // animation-name lookup). It MUST match a real authored row or the strike
    // silently falls back to the tiny default Rect. The manifest authors the
    // aerials as `air_up` / `air_down` / `air_back` / `air_forward` (NOT
    // `attack_air*`), so the aerials bind those — otherwise the down-air's big
    // authored blade was lost and the dair read as a tiny box. The sprite is
    // driven by the swing's intent (`directional_attack_anim`), not this clip, so
    // the clip name only steers the hitbox lookup.
    vec![
        (
            "attack_up".to_string(),
            variant("attack_up", "Up Tilt", "attack_up", true, Dir::Up, false),
        ),
        (
            "attack_down".to_string(),
            variant(
                "attack_down",
                "Down Tilt",
                "attack_down",
                true,
                Dir::Down,
                false,
            ),
        ),
        (
            "attack_air".to_string(),
            variant(
                "attack_air",
                "Forward Air",
                "air_forward",
                false,
                Dir::Fwd,
                false,
            ),
        ),
        (
            "attack_air_up".to_string(),
            variant("attack_air_up", "Up Air", "air_up", false, Dir::Up, false),
        ),
        (
            "attack_air_back".to_string(),
            variant(
                "attack_air_back",
                "Back Air",
                "air_back",
                false,
                Dir::Back,
                false,
            ),
        ),
        (
            "attack_air_down".to_string(),
            variant(
                "attack_air_down",
                "Down Air",
                "air_down",
                false,
                Dir::Down,
                true,
            ),
        ),
    ]
}

/// Convert an authored [`SpecialActionSpec`] into a data-driven `"special"` [`MoveSpec`] — the
/// special subsumption, the mirror of [`attack_move_from_melee`] / [`fire_move_from_ranged`].
///
/// The move is a short Startup/Active/Recovery timeline with NO hit volumes: a
/// signature special is not necessarily a strike, and its concrete GAMEPLAY
/// consequence is content-defined by the key. The move `id` IS the key, so the
/// consequence resolves by identity (e.g. `"bubble_shield"` raises the guard
/// while the move plays, via `sustain_bubble_shield`), and the on-screen Special
/// button reads the key's title-cased label ("Bubble Shield").
pub fn special_move_from_spec(spec: &SpecialActionSpec) -> MoveSpec {
    let SpecialActionSpec::Special(key) = spec;
    let (windup, active, recover) = (0.08, 0.24, 0.13);
    MoveSpec {
        display_name: None,
        landing_lag_s: None,
        autocancel_after_s: None,
        sprite_spin_hz: None,
        equips: None,
        flow: None,
        id: key.clone(),
        clip: ClipBinding {
            clip: "special".to_string(),
            fallbacks: vec!["idle".to_string()],
        },
        duration_s: windup + active + recover,
        windows: vec![
            MoveWindow {
                start_s: 0.0,
                end_s: windup,
                tag: WindowTag::Startup,
                volumes: vec![],
                sustain_effect: None,
                motion_scale: 1.0,
            },
            MoveWindow {
                start_s: windup,
                end_s: windup + active,
                tag: WindowTag::Active,
                volumes: vec![],
                sustain_effect: None,
                motion_scale: 1.0,
            },
            MoveWindow {
                start_s: windup + active,
                end_s: windup + active + recover,
                tag: WindowTag::Recovery,
                volumes: vec![],
                sustain_effect: None,
                motion_scale: 1.0,
            },
        ],
        events: vec![],
        gates: Default::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
        smash_charge: None,
        charge_gesture: ambition_entity_catalog::ChargeGesture::default(),
        repeat: None,
    }
}

pub fn build_actor_moveset(
    signature: Option<&MovesetContract>,
    melee: Option<&MeleeActionSpec>,
    ranged: Option<&RangedActionSpec>,
    special: Option<&SpecialActionSpec>,
) -> Option<MovesetContract> {
    let mut contract = signature.cloned().unwrap_or_default();
    if let Some(melee) = melee {
        let attack = attack_move_from_melee(melee);
        contract
            .verbs
            .insert(ATTACK_VERB.to_string(), attack.id.clone());
        // Directional variants DERIVED from the base swing (fable review R2.5):
        // the character's ONE authored melee becomes up-/down-tilt + the four
        // aerials + the pogo down-air, scaled by ITS reach — not a hardcoded
        // per-character table. Every controlled body (human / brain / RL) resolves
        // these through the SAME directional trigger; a neutral, grounded attacker
        // (every enemy today, since brains don't aim) still resolves `"attack"`,
        // so the swing that lands is byte-identical.
        for (verb, mv) in directional_attack_variants(&attack) {
            contract.verbs.insert(verb, mv.id.clone());
            contract.moves.retain(|m| m.id != mv.id);
            contract.moves.push(mv);
        }
        // Replace any existing attack move (idempotent) then push.
        contract.moves.retain(|m| m.id != attack.id);
        contract.moves.push(attack);
    }
    if let Some(ranged) = ranged {
        let fire = fire_move_from_ranged(ranged);
        contract
            .verbs
            .insert(RANGED_VERB.to_string(), fire.id.clone());
        contract.moves.retain(|m| m.id != fire.id);
        contract.moves.push(fire);
    }
    if let Some(special) = special {
        // The special folds LAST so a body's authored signature (`signature`
        // arg) is the base and the ActionSet marker overlays it — idempotent by
        // move id, so re-deriving on an equip/kit swap is stable.
        let mv = special_move_from_spec(special);
        contract
            .verbs
            .insert(SPECIAL_VERB.to_string(), mv.id.clone());
        contract.moves.retain(|m| m.id != mv.id);
        contract.moves.push(mv);
    }
    if contract.moves.is_empty() {
        None
    } else {
        Some(contract)
    }
}

/// Equip one [`crate::equipment::EquipmentRow`] onto a body, returning the rebuilt
/// [`MovesetContract`] when — and only when — the row actually changed what the
/// body can do.
///
/// This is the A3 equip contract, and the split it encodes is the point: an
/// equipment row is either
///
/// - read-time only (no [`crate::equipment::EquipmentGrant`]s — a grow-cap, an armor plate, a
///   damage-scaling flower): it lands in [`crate::equipment::WornEquipment`] and nothing else moves.
///   Its effect is folded at the moment it matters, by
///   [`resolved_ranged`](crate::equipment::resolved_ranged) and
///   friends. Rebuilding a moveset for it would be pure churn, so this returns
///   `None` and the caller keeps the contract it already has; or
/// - grant-bearing (a spark blossom that confers a ranged verb): the whole
///   worn set's grants are re-applied to `actions` and the moveset is rebuilt
///   from the result over `signature`, so the body gains the granted verb while
///   keeping every verb its own authored signature declared.
///
/// Re-applying the FULL worn set rather than just this row's grants is what makes
/// unequip work as the plain inverse: drop the row from `worn` and call this
/// again with the remaining set.
pub fn equip_equipment_row(
    actions: &mut crate::brain::action_set::ActionSet,
    worn: &mut crate::equipment::WornEquipment,
    signature: Option<&MovesetContract>,
    row: crate::equipment::EquipmentRow,
) -> Option<MovesetContract> {
    let confers_capability = !row.grants.is_empty();
    worn.equip(row);
    if !confers_capability {
        return None;
    }
    crate::equipment::apply_equipment_grants(actions, worn);
    build_actor_moveset(
        signature,
        actions.melee.as_ref(),
        actions.ranged.as_ref(),
        actions.special.as_ref(),
    )
}
