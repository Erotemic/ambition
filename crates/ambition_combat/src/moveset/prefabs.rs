//! **Move authoring** — the build-time half of the Smash model: the functions that
//! turn authored specs (`MeleeActionSpec`/`RangedActionSpec`), tunable params
//! (`Simple{Melee,Ranged,Charge}Params`), and the `MovePrefabRegistry` into
//! `MoveSpec`s, plus `build_actor_moveset` which assembles an
//! actor's full `MovesetContract` from its catalog + worn equipment.
//!
//! Split out of the former `moveset.rs` for the D-B module-size gate. The runtime
//! (`MovePlayback`, `advance_move_playback`, the systems) stays in `mod.rs`; this
//! module shares its constants, imports, and component types via `use super::*`.
use super::*;
use ambition_characters::brain::action_set::RangedStyle;

/// Convert an authored [`MeleeActionSpec`] into a data-driven `"attack"`
/// [`MoveSpec`] — the melee subsumption (fable review §A1 / §3a). The swing's
/// windup/active/recover timeline becomes Startup / Active(one forward Rect hit
/// volume) / Recovery windows on the owner's proper-time clock, so a plain melee
/// runs through the SAME moveset runtime as the body's specials. Re-binding a
/// swing onto another body is now a data edit, and the swing composes with
/// dilation / pause for free.
///
/// The forward hit volume is a body-local rect sized from the spec's `reach_px`
/// (offset a bit past the body, half-extent covering the reach + a torso-height
/// band). Knockback is a sensible default. This base swing is the `"attack"`
/// verb; the up/down-tilt, the four aerials, and the pogo down-air are DERIVED
/// from it by [`directional_attack_variants`] — every direction runs through the
/// moveset (there is no flat player melee path anymore).
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
    /// CM5: an OPTIONAL cosmetic burst id (`ambition_vfx::move_vfx_kind`
    /// vocabulary) emitted at the Active edge on top of the slash arc — `None` =
    /// no extra burst (parity). Lets a launcher `"starburst"`, a smash
    /// `"shockwave"`. A typo is a startup validation error, never silent.
    #[serde(default)]
    pub swing_vfx: Option<String>,
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
        shape: VolumeShape::Rect {
            offset: (p.reach_px * 0.6, 0.0),
            half_extents: (half_x, 16.0),
        },
        damage: p.damage.max(1),
        knockback: p.knockback,
        // Prefab swings are flat-knockback; percent growth is authored on
        // explicit RON volumes (CM1) — a prefab growth param can follow.
        kb_growth: 0.0,
        launch_dir: None,
        // A plain swing lands no on-hit technique; directional variants (a
        // down-air pogo) author `on_hit` per-move (R2.5 player-melee fold).
        on_hit: None,
        // A bladed swing (§7.1/§7.2): draws the slash arc from the spawned
        // volume, and — when the owner's sprite manifest authors a hit polygon
        // for this move's clip — swings THAT authored blade instead of this
        // synthetic rect (the rect is the fallback for unmanifested bodies).
        vfx: Some(SLASH_ARC_VFX.to_string()),
    };
    MoveSpec {
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
                    },
                });
            }
            events
        },
        gates: Default::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
    }
}

/// Convert an authored [`RangedActionSpec`] into a data-driven `"ranged"`
/// [`MoveSpec`] — the ranged subsumption (fable review, option A). Ranged had NO
/// windup/recovery before (it fired instantly on a body-side cooldown); giving it a
/// move timeline is the expressivity win: a `Startup` draw/aim window, a single
/// [`MoveEventKind::Ranged`] fire event at release (which spawns the projectile,
/// sampling LIVE aim), and a `Recovery` settle window — all on the owner's
/// proper-time clock, so a dilated shooter's draw slows with it. The projectile IS
/// the damage (spawned by the event through the shared enemy-projectile consumer),
/// so the move carries NO hit volume. Windup/recovery are authored defaults per
/// spec kind (deferred-tuning); the body-side refire cooldown remains the hard rate
/// floor, the move duration an additional cadence gate.
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
        ],
        events: vec![MoveEvent {
            at_s: windup,
            kind: MoveEventKind::Ranged,
        }],
        gates: Default::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
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
        shape: VolumeShape::Rect {
            offset: (p.reach_px * 0.6, 0.0),
            half_extents: (half_x, 18.0),
        },
        damage: p.damage.max(1),
        knockback: p.knockback,
        kb_growth: 0.0,
        launch_dir: None,
        on_hit: None,
        // A charge is a bladed strike too — same slash + authored-blade rules.
        vfx: Some(SLASH_ARC_VFX.to_string()),
    };
    MoveSpec {
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
                    },
                });
            }
            events
        },
        gates: Default::default(),
        start_impulse: None,
        // CM3: the charge move's payoff — the authored release multiplier.
        smash_charge_mult: p.smash_charge_mult,
    }
}

/// A prefab builder: hydrate an authored [`ParamValue`] into the prefab's own
/// params and expand it into a [`MoveSpec`]. `fn`-pointer shaped so the registry
/// stays a plain data table.
pub type MovePrefabBuilder = fn(&ambition_entity_catalog::ParamValue) -> Result<MoveSpec, String>;

/// String-keyed prefab registry (A2 / R2.3): `key + params -> MoveSpec`, expanded
/// at roster install. The engine ships `simple_melee` / `simple_ranged` /
/// `simple_charge`; a content roster names a prefab + params to mint a move with
/// ZERO new code (`sword_slash = simple_melee` + sword params). Content may
/// register its own prefabs for richer shapes.
pub struct MovePrefabRegistry {
    builders: std::collections::BTreeMap<String, MovePrefabBuilder>,
}

impl MovePrefabRegistry {
    /// A registry pre-seeded with the engine-shipped prefabs.
    pub fn with_engine_prefabs() -> Self {
        let mut reg = Self {
            builders: std::collections::BTreeMap::new(),
        };
        reg.register("simple_melee", |p| {
            Ok(simple_melee(&p.hydrate().map_err(|e| e.to_string())?))
        });
        reg.register("simple_ranged", |p| {
            Ok(simple_ranged(&p.hydrate().map_err(|e| e.to_string())?))
        });
        reg.register("simple_charge", |p| {
            Ok(simple_charge(&p.hydrate().map_err(|e| e.to_string())?))
        });
        reg
    }

    /// Register (or override) a prefab builder under `key`.
    pub fn register(&mut self, key: impl Into<String>, builder: MovePrefabBuilder) {
        self.builders.insert(key.into(), builder);
    }

    /// Expand a prefab row into a move named `move_id`. Errors if the key is
    /// unknown (a roster typo) or the authored params don't hydrate.
    pub fn expand(
        &self,
        key: &str,
        params: &ambition_entity_catalog::ParamValue,
        move_id: &str,
    ) -> Result<MoveSpec, String> {
        let builder = self
            .builders
            .get(key)
            .ok_or_else(|| format!("unknown move prefab '{key}'"))?;
        let mut spec = builder(params)?;
        spec.id = move_id.to_string();
        // CM5: reject an unresolvable presentation id (a `Vfx`/`Sfx` typo) at
        // expansion time — the SAME startup-validation gate a bad prefab key or
        // param hits, so authored sound/vfx typos never survive to a silent
        // missing effect. The cosmetic-vfx vocabulary lives in `ambition_vfx`;
        // inject it (entity_catalog can't depend on the render-adjacent crate).
        let problems = spec.presentation_problems(|id| ambition_vfx::move_vfx_kind(id).is_some());
        if !problems.is_empty() {
            return Err(problems.join("; "));
        }
        Ok(spec)
    }

    /// True iff no prefab is registered.
    pub fn is_empty(&self) -> bool {
        self.builders.is_empty()
    }
}

impl Default for MovePrefabRegistry {
    fn default() -> Self {
        Self::with_engine_prefabs()
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
    let variant = |id: &str, clip: &str, grounded: bool, dir: Dir, pogo: bool| -> MoveSpec {
        let mut m = base.clone();
        m.id = id.to_string();
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
                    // (fable review AJ1). Fires off any `PogoTarget` (enemy or orb).
                    v.on_hit = Some(EffectRef::new(crate::on_hit::POGO_BOUNCE_KEY));
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
            variant("attack_up", "attack_up", true, Dir::Up, false),
        ),
        (
            "attack_down".to_string(),
            variant("attack_down", "attack_down", true, Dir::Down, false),
        ),
        (
            "attack_air".to_string(),
            variant("attack_air", "air_forward", false, Dir::Fwd, false),
        ),
        (
            "attack_air_up".to_string(),
            variant("attack_air_up", "air_up", false, Dir::Up, false),
        ),
        (
            "attack_air_back".to_string(),
            variant("attack_air_back", "air_back", false, Dir::Back, false),
        ),
        (
            "attack_air_down".to_string(),
            variant("attack_air_down", "air_down", false, Dir::Down, true),
        ),
    ]
}

/// Convert an authored [`SpecialActionSpec`] into a data-driven `"special"`
/// [`MoveSpec`] — the special subsumption, the mirror of
/// [`attack_move_from_melee`] / [`fire_move_from_ranged`]. `ActionSet.special`
/// stopped being an executable path (the flat resolver arm was retired); it is a
/// pure capability marker, and the concrete move must live in the body's moveset
/// on the `"special"` verb for [`super::trigger_moveset_moves`] to fire it on
/// `special_pressed`. Without this the canonical player's `Special("bubble_shield")`
/// marker had no move — pressing Special did nothing.
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

/// Equip one [`EquipmentRow`] onto a body, returning the rebuilt
/// [`MovesetContract`] when — and only when — the row actually changed what the
/// body can do.
///
/// This is the A3 equip contract, and the split it encodes is the point: an
/// equipment row is either
///
/// - **read-time only** (no [`EquipmentGrant`]s — a grow-cap, an armor plate, a
///   damage-scaling flower): it lands in [`WornEquipment`] and nothing else moves.
///   Its effect is folded at the moment it matters, by
///   [`resolved_ranged`](ambition_characters::equipment::resolved_ranged) and
///   friends. Rebuilding a moveset for it would be pure churn, so this returns
///   `None` and the caller keeps the contract it already has; or
/// - **grant-bearing** (a spark blossom that confers a ranged verb): the whole
///   worn set's grants are re-applied to `actions` and the moveset is rebuilt
///   from the result over `signature`, so the body gains the granted verb while
///   keeping every verb its own authored signature declared.
///
/// Re-applying the FULL worn set rather than just this row's grants is what makes
/// unequip work as the plain inverse: drop the row from `worn` and call this
/// again with the remaining set.
pub fn equip_equipment_row(
    actions: &mut ambition_characters::brain::action_set::ActionSet,
    worn: &mut ambition_characters::equipment::WornEquipment,
    signature: Option<&MovesetContract>,
    row: ambition_characters::equipment::EquipmentRow,
) -> Option<MovesetContract> {
    let confers_capability = !row.grants.is_empty();
    worn.equip(row);
    if !confers_capability {
        return None;
    }
    ambition_characters::equipment::apply_equipment_grants(actions, worn);
    build_actor_moveset(
        signature,
        actions.melee.as_ref(),
        actions.ranged.as_ref(),
        actions.special.as_ref(),
    )
}
