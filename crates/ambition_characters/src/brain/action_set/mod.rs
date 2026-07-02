//! `ActionSet` — per-entity capability.
//!
//! A brain emits abstract intent into [`crate::actor::control::ActorControlFrame`]
//! (`melee_pressed = true`, `fire = Some(dir)`). The actor's
//! `ActionSet` translates that intent into a concrete effect
//! (`spawn a Swipe hitbox`, `launch a Rock projectile`). Two actors
//! can share the same brain template and look completely different
//! because their ActionSets resolve differently.
//!
//! The same data structure works for players, NPCs, enemies, and
//! bosses. A player possessing a goblin keeps the goblin's
//! ActionSet — pressing Attack still resolves to "leap" because that
//! is the goblin's `melee_attack` spec.
//!
//! Telegraphs aren't a separate concept; each attack spec owns its
//! full windup → active → recover animation timing.

use ambition_engine_core as ae;
use bevy::ecs::component::Component;

/// Per-entity capability set. Resolves abstract brain intent
/// (control-frame fields) into concrete effect requests
/// ([`ActionRequest`]) that the EFFECTS-stage spawn systems consume.
///
/// Construct via [`ActionSet::peaceful`] for a "no attacks" baseline
/// and override only the slots that exist for this actor.
#[derive(Component, Clone, Debug, Default)]
pub struct ActionSet {
    /// What `frame.melee_pressed = true` resolves to. `None` means
    /// the actor has no melee at all (peaceful patroller, puppy slug,
    /// etc.); the brain may still set `melee_pressed = true` but the
    /// EFFECTS stage spawns nothing.
    pub melee: Option<MeleeActionSpec>,
    /// What `frame.fire = Some(dir)` resolves to. `None` = no ranged
    /// capability.
    pub ranged: Option<RangedActionSpec>,
    /// How locomotion looks. Walk is the conservative default; brain
    /// templates that emit `desired_vel` get their motion drawn via
    /// this style.
    pub move_style: MoveStyleSpec,
    /// What `frame.special_pressed = true` resolves to. Per-entity
    /// signature move — boss spotlight pattern, player-only ability,
    /// etc.
    pub special: Option<SpecialActionSpec>,
}

impl ActionSet {
    /// "I don't attack" baseline. Used for peaceful NPCs, puppy
    /// slugs, and other passive actors.
    pub fn peaceful() -> Self {
        Self::default()
    }

    /// True iff this ActionSet has at least one offensive capability
    /// (melee or ranged). Daytime HUD / faction logic uses this to
    /// distinguish "passive observer" actors from "could attack
    /// if asked" actors without re-checking three Option<>s.
    #[allow(dead_code, reason = "diagnostic + daytime-consumer helper")]
    pub fn can_attack(&self) -> bool {
        self.melee.is_some() || self.ranged.is_some()
    }
}

/// What a plain `Attack` does to / with a held item — authored on the spec
/// instead of a hardcoded id-chain in `item_pickup::throw_held_item_system`
/// (Refactor 5). The narrow vocabulary the "Pick-up / throw held items" item
/// named; **not** a generic plugin system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
pub enum HeldUseBehavior {
    /// Derive from the verbs: an item WITH a melee/ranged verb keeps on use
    /// (swing/fire); a verb-LESS item throws on use (the legacy
    /// `is_pure_throwable` rule). The default, so existing RON item rows need
    /// no new field.
    #[default]
    Auto,
    /// Keep the item; its verb fires on `Attack` (explicit; `Auto` already
    /// covers a verb-bearing weapon).
    KeepOnUse,
    /// Using it (a plain `Attack`) THROWS it — the javelin's classic
    /// thrown-item feel.
    ThrowOnUse,
    /// A bespoke `*_system` consumes the plain `Attack` (blink / grapple /
    /// mark / summon / shockwave / volley); the item is KEPT and only thrown
    /// on the explicit `Shield + Attack`.
    UseSystem,
}

/// Authored item carried by an actor. Held items are gameplay capabilities,
/// not just visuals: they can grant melee and/or ranged actions to the
/// actor's `ActionSet`. The item id is intentionally data-authored so future
/// item rows (axe, sword, thrown bomb, bow, etc.) can be added to RON without
/// adding a Rust enum variant for each item.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
pub struct HeldItemSpec {
    /// Stable authored id used by visuals / projectile routing / future drops.
    pub id: String,
    /// Optional melee verb granted by the held item.
    #[serde(default)]
    pub melee: Option<MeleeActionSpec>,
    /// Optional ranged verb granted by the held item.
    #[serde(default)]
    pub ranged: Option<RangedActionSpec>,
    /// What a plain `Attack` does to/with this item (Refactor 5). `#[serde(default)]`
    /// keeps older RON rows loadable: missing → `Auto`.
    #[serde(default)]
    pub use_behavior: HeldUseBehavior,
}

impl HeldItemSpec {
    /// Whether a plain (non-shield) `Attack` throws this item, per its
    /// [`HeldUseBehavior`]. The single source the throw system reads instead of
    /// a hardcoded id-chain.
    pub fn throws_on_plain_attack(&self) -> bool {
        match self.use_behavior {
            HeldUseBehavior::Auto => self.melee.is_none() && self.ranged.is_none(),
            HeldUseBehavior::ThrowOnUse => true,
            HeldUseBehavior::KeepOnUse | HeldUseBehavior::UseSystem => false,
        }
    }
}

impl HeldItemSpec {
    /// Overlay the item's abilities on top of an archetype action set. The
    /// item wins because weapons are the thing the actor is actually holding;
    /// archetype rows remain useful for body-contact and fallback tuning.
    pub fn apply_to_action_set(&self, actions: &mut ActionSet) {
        if let Some(melee) = self.melee {
            actions.melee = Some(melee);
        }
        if let Some(ranged) = self.ranged {
            actions.ranged = Some(ranged);
        }
    }

    pub fn grants_ranged(&self) -> bool {
        self.ranged.is_some()
    }
}

/// Registry of authored held items, keyed by stable id.
///
/// Archetypes (and future drop tables / pickups) reference an item by id —
/// `held_item: Some("gun_sword")` — instead of embedding the full spec, so a
/// weapon is defined in exactly one place and can be shared. New weapons are
/// added here (or, later, an item RON the loader merges in) rather than
/// duplicated per archetype. The schema is deliberately the current
/// id/melee/ranged shape; richer fields (muzzle offset, ammo, projectile arc)
/// land when the item pass that needs them does.
static HELD_ITEMS: std::sync::LazyLock<std::collections::HashMap<&'static str, HeldItemSpec>> =
    std::sync::LazyLock::new(|| {
        let mut items = std::collections::HashMap::new();
        items.insert(
            "gun_sword",
            HeldItemSpec {
                id: "gun_sword".into(),
                melee: None,
                ranged: Some(RangedActionSpec::Bolt {
                    speed: 500.0,
                    damage: 2,
                }),
                use_behavior: HeldUseBehavior::Auto,
            },
        );
        items.insert(
            "gun_sword_heavy",
            HeldItemSpec {
                id: "gun_sword_heavy".into(),
                melee: None,
                ranged: Some(RangedActionSpec::Bolt {
                    speed: 500.0,
                    damage: 3,
                }),
                use_behavior: HeldUseBehavior::Auto,
            },
        );
        // The puppy-slug gun has no melee/ranged verb of its own — `Attack` is
        // intercepted by `puppy_slug_gun::fire_puppy_slug_gun_system`, which
        // summons a player-allied puppy slug instead.
        items.insert(
            "puppy_slug_gun",
            HeldItemSpec {
                id: "puppy_slug_gun".into(),
                melee: None,
                ranged: None,
                use_behavior: HeldUseBehavior::UseSystem,
            },
        );
        // The shockwave gauntlet has no melee/ranged verb — `Attack` is
        // intercepted by `shockwave::fire_shockwave_system`, which emits a
        // generic `DamageBox` effect so `apply_effects` spawns a player-faction
        // AOE (the player wielding a boss-style attack).
        items.insert(
            "shockwave",
            HeldItemSpec {
                id: "shockwave".into(),
                melee: None,
                ranged: None,
                use_behavior: HeldUseBehavior::UseSystem,
            },
        );
        // The volley gauntlet has no melee/ranged verb — `Attack` is intercepted
        // by `volley::fire_volley_system`, which fires a fan of player-faction
        // bolts through the faction-aware projectile pool.
        items.insert(
            "volley",
            HeldItemSpec {
                id: "volley".into(),
                melee: None,
                ranged: None,
                use_behavior: HeldUseBehavior::UseSystem,
            },
        );
        // The focus-beam gauntlet has no melee/ranged verb — `Attack` is
        // intercepted by `beam::fire_beam_system`, which spawns an aimed line
        // `Hitbox` of Player faction (the smirking_behemoth eye-beam, wielded).
        items.insert(
            "beam",
            HeldItemSpec {
                id: "beam".into(),
                melee: None,
                ranged: None,
                use_behavior: HeldUseBehavior::UseSystem,
            },
        );
        // The vortex gauntlet has no melee/ranged verb — `Attack` is intercepted
        // by `vortex::fire_vortex_system`, which spawns a point attractor that
        // gathers enemies (crowd-control; no damage — pull-then-slam).
        items.insert(
            "vortex",
            HeldItemSpec {
                id: "vortex".into(),
                melee: None,
                ranged: None,
                use_behavior: HeldUseBehavior::UseSystem,
            },
        );
        // The sentry gauntlet has no melee/ranged verb — `Attack` is intercepted
        // by `sentry::fire_sentry_system`, which deploys an auto-firing turret.
        items.insert(
            "sentry",
            HeldItemSpec {
                id: "sentry".into(),
                melee: None,
                ranged: None,
                use_behavior: HeldUseBehavior::UseSystem,
            },
        );
        // The dive gauntlet has no melee/ranged verb — `Attack` is intercepted
        // by `dive::fire_dive_system`, which lunges the player along the aim and
        // cuts a damage corridor (the overflow boss's crash, wielded).
        items.insert(
            "dive",
            HeldItemSpec {
                id: "dive".into(),
                melee: None,
                ranged: None,
                use_behavior: HeldUseBehavior::UseSystem,
            },
        );
        // The meteor gauntlet has no melee/ranged verb — `Attack` is intercepted
        // by `meteor::fire_meteor_system`, which rains falling player-faction
        // projectiles onto a zone ahead (GNU-ton's apple-rain, wielded).
        items.insert(
            "meteor",
            HeldItemSpec {
                id: "meteor".into(),
                melee: None,
                ranged: None,
                use_behavior: HeldUseBehavior::UseSystem,
            },
        );
        // The bomb is a pure throwable (no melee/ranged verb): a plain Attack
        // throws it, and `bomb::tick_bomb_fuses` detonates it on a fuse.
        items.insert(
            "bomb",
            HeldItemSpec {
                id: "bomb".into(),
                melee: None,
                ranged: None,
                use_behavior: HeldUseBehavior::Auto,
            },
        );
        // The Mark/Recall ability has no melee/ranged verb either — its plain
        // `Attack` is intercepted by `mark_recall::mark_recall_system` (drop a
        // teleport mark) and `Blink` recalls to it. Like the puppy-slug gun it
        // opts out of throw-on-attack via `throw_held_item_system`.
        items.insert(
            "mark_recall",
            HeldItemSpec {
                id: "mark_recall".into(),
                melee: None,
                ranged: None,
                use_behavior: HeldUseBehavior::UseSystem,
            },
        );
        // The Fireball ability fires a ranged bolt that *explodes on contact*
        // (`item_pickup::fire_held_ranged_system` tags the shot by this id, and
        // `held_projectile_step` detonates it). The Bolt damage is the splash
        // damage; the AOE box is what makes it distinct from the gun-sword.
        items.insert(
            "fireball",
            HeldItemSpec {
                id: "fireball".into(),
                melee: None,
                ranged: Some(RangedActionSpec::Bolt {
                    speed: 440.0,
                    damage: 3,
                }),
                use_behavior: HeldUseBehavior::Auto,
            },
        );
        // Blink has no melee/ranged verb — its plain `Attack` is intercepted by
        // `blink::blink_system` (a short collision-clamped teleport along aim),
        // so it opts out of throw-on-attack like the other pure-use abilities.
        items.insert(
            "blink",
            HeldItemSpec {
                id: "blink".into(),
                melee: None,
                ranged: None,
                use_behavior: HeldUseBehavior::UseSystem,
            },
        );
        // Grapple has no melee/ranged verb either — `grapple::grapple_system`
        // intercepts its `Attack` (yank toward a grappled surface).
        items.insert(
            "grapple",
            HeldItemSpec {
                id: "grapple".into(),
                melee: None,
                ranged: None,
                use_behavior: HeldUseBehavior::UseSystem,
            },
        );
        // The gravity grenade is a pure throwable like the bomb (plain Attack
        // throws it); `gravity_grenade::tick_gravity_grenade_fuses` opens an
        // up-gravity well on its fuse instead of exploding.
        items.insert(
            "gravity_grenade",
            HeldItemSpec {
                id: "gravity_grenade".into(),
                melee: None,
                ranged: None,
                use_behavior: HeldUseBehavior::Auto,
            },
        );
        items
    });

/// Resolve a held-item id to its authored spec, or `None` for an unknown id.
pub fn held_item_by_id(id: &str) -> Option<HeldItemSpec> {
    HELD_ITEMS.get(id).cloned()
}

/// Concrete melee actions an actor can perform. Each variant carries
/// its **own** animation timing (windup → active → recover) — there
/// is no separate `TelegraphSpec`.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[allow(
    dead_code,
    reason = "spec variants surface to per-actor EFFECTS consumers"
)]
pub enum MeleeActionSpec {
    /// Generic short swing. Used by Striker / standard goblin melee.
    Swipe(SwipeSpec),
    /// Heavy lunging step + strike. Used by Brute / large mob melee.
    Lunge(LungeSpec),
    /// Pounce + slam. Used by FastFall and the puppy-slug aerial dive
    /// (when applicable). Today no actor uses this; reserved for
    /// future Wanderer-with-aggression archetypes.
    Slam(SlamSpec),
    /// Jaw-snap bite. Used by puppy slug aggressive variants and
    /// sharks if/when they get melee.
    Bite(BiteSpec),
    /// Light reactive punch — a quick jab thrown back when struck (for reactive
    /// strikers; a passive practice target does NOT use this).
    PunchWeak(PunchSpec),
}

/// Concrete ranged actions an actor can perform.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
pub enum RangedActionSpec {
    /// Throws a rock-shaped projectile (used by skirmishers /
    /// peaceful-turned-hostile NPCs).
    Rock { speed: f32, damage: i32 },
    /// Fires an arrow. Slower windup than Rock, more damage.
    Arrow { speed: f32, damage: i32 },
    /// Fires a pistol shot (used by pirate skirmishers).
    Pistol { speed: f32, damage: i32 },
    /// Fires a magical bolt (used by bosses).
    Bolt { speed: f32, damage: i32 },
}

impl RangedActionSpec {
    /// Effective launch speed. The brain emits `frame.fire =
    /// Some(dir)`; the EFFECTS stage reads this to set the
    /// projectile speed.
    pub fn speed(self) -> f32 {
        match self {
            Self::Rock { speed, .. }
            | Self::Arrow { speed, .. }
            | Self::Pistol { speed, .. }
            | Self::Bolt { speed, .. } => speed,
        }
    }

    /// Damage on hit.
    pub fn damage(self) -> i32 {
        match self {
            Self::Rock { damage, .. }
            | Self::Arrow { damage, .. }
            | Self::Pistol { damage, .. }
            | Self::Bolt { damage, .. } => damage,
        }
    }
}

impl MeleeActionSpec {
    /// Total swing duration (windup + active + recover) in seconds.
    /// Cooldown systems / animation pickers use this to gate the
    /// "can swing again" question.
    #[allow(dead_code, reason = "diagnostic helper for EFFECTS consumers")]
    pub fn total_duration_s(self) -> f32 {
        match self {
            Self::Swipe(s) => s.windup_s + s.active_s + s.recover_s,
            Self::Lunge(s) => s.windup_s + s.active_s + s.recover_s,
            Self::Slam(s) => s.windup_s + s.active_s + s.recover_s,
            Self::Bite(s) => s.windup_s + s.active_s + s.recover_s,
            Self::PunchWeak(s) => s.windup_s + s.active_s + s.recover_s,
        }
    }

    /// Damage dealt on a clean hit.
    #[allow(dead_code, reason = "diagnostic helper for EFFECTS consumers")]
    pub fn damage(self) -> i32 {
        match self {
            Self::Swipe(s) => s.damage,
            Self::Lunge(s) => s.damage,
            Self::Slam(s) => s.damage,
            Self::Bite(s) => s.damage,
            Self::PunchWeak(s) => s.damage,
        }
    }

    /// Reach (hitbox forward extent) in px from the actor's anchor.
    #[allow(dead_code, reason = "diagnostic helper for EFFECTS consumers")]
    pub fn reach_px(self) -> f32 {
        match self {
            Self::Swipe(s) => s.reach_px,
            Self::Lunge(s) => s.reach_px,
            Self::Slam(s) => s.reach_px,
            Self::Bite(s) => s.reach_px,
            Self::PunchWeak(s) => s.reach_px,
        }
    }
}

/// How an actor's locomotion looks.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Deserialize)]
pub enum MoveStyleSpec {
    /// Two-legged walk (default for humanoids).
    #[default]
    Walk,
    /// Heavy slow walk — used by Brute.
    WalkHeavy,
    /// Hop forward in arcs (used by FastFall).
    Hop,
    /// Strafing sideways motion (used by Skirmisher).
    Strafe,
    /// Crawls along surfaces (used by puppy slug). The actor's
    /// `surface_normal` rotates the rendered motion.
    Slither,
    /// Floats / hovers (used by aerial bosses, sharks).
    Float,
}

/// Per-entity signature move.
///
/// Specials are content-defined string keys. A content-owned *Technique*
/// recognizes the key and owns the params + behavior. Not `Copy` — the key is
/// an owned `String`.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SpecialActionSpec {
    /// An open, content-defined special. The `String` is the special
    /// **key** (snake_case, e.g. `"overfit_volley"`); the matching
    /// content-owned *Technique* reads its own params + emits the
    /// effects. The brain emits this when a `BossAttackProfile::Special`
    /// beat strikes (see `BossAttackProfile::special_key`). The old
    /// per-boss variants (`DebrisRain`, `MemorizedVolley`, `LockOnBeam`,
    /// `PitTrap`, `RotatingCross`, `MinionCascade`) collapsed here — the
    /// engine names no boss special.
    Special(String),
    // `ShockwaveSlam` moved off this enum onto the generic effect seam
    // (`ambition_vfx::Effect::DamageBox`): an actor-generic
    // ground-slam is now an emitted effect, not a Special variant. It was the
    // first actor-generic special; the rest migrate the same way.
}

// --- Concrete attack spec timings ---
//
// Each spec carries (windup, active, recover) in seconds, plus
// damage + a hitbox half-extent. Today these values mirror the
// pre-refactor enemy archetype constants so Chunk 3's migration is
// a one-for-one move. Chunk 4 / data-table work shrinks duplication.

/// Light melee swing. Striker default.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
pub struct SwipeSpec {
    pub windup_s: f32,
    pub active_s: f32,
    pub recover_s: f32,
    pub damage: i32,
    pub reach_px: f32,
}

impl SwipeSpec {
    pub const STRIKER_DEFAULT: Self = Self {
        windup_s: 0.28,
        active_s: 0.08,
        recover_s: 0.32,
        damage: 1,
        reach_px: 28.0,
    };
}

/// Heavy lunging strike. Brute default.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
pub struct LungeSpec {
    pub windup_s: f32,
    pub active_s: f32,
    pub recover_s: f32,
    pub damage: i32,
    pub reach_px: f32,
    /// Forward step (px) the actor takes during windup.
    pub step_px: f32,
}

impl LungeSpec {
    pub const BRUTE_DEFAULT: Self = Self {
        windup_s: 0.45,
        active_s: 0.12,
        recover_s: 0.45,
        damage: 2,
        reach_px: 40.0,
        step_px: 18.0,
    };
}

/// Pounce + slam. Reserved for future hostile aerial archetypes.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
pub struct SlamSpec {
    pub windup_s: f32,
    pub active_s: f32,
    pub recover_s: f32,
    pub damage: i32,
    pub reach_px: f32,
    pub hop_height_px: f32,
}

/// Jaw bite — short reach, fast.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
pub struct BiteSpec {
    pub windup_s: f32,
    pub active_s: f32,
    pub recover_s: f32,
    pub damage: i32,
    pub reach_px: f32,
}

/// Light reactive punch — a reactive counter-jab (not used by passive targets).
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
pub struct PunchSpec {
    pub windup_s: f32,
    pub active_s: f32,
    pub recover_s: f32,
    pub damage: i32,
    pub reach_px: f32,
}

impl PunchSpec {
    pub const SANDBAG_DEFAULT: Self = Self {
        windup_s: 0.15,
        active_s: 0.08,
        recover_s: 0.40,
        damage: 1,
        reach_px: 22.0,
    };
}

/// Concrete effect a brain's abstract intent resolves to, after
/// reading the actor's `ActionSet`. The EFFECTS-stage spawn systems
/// consume this list per actor per tick and translate each into a
/// real hitbox / projectile / FX.
///
/// In Chunk 2 this is the *shape* of the resolver output — actual
/// resolver wiring (writing hitbox AABBs into a feature-output
/// channel) lands in Chunk 3 when an actor first uses a brain. For
/// now the helpers are testable in isolation.
// Not `Copy`: the `Special` variant carries an owned `SpecialActionSpec`
// (open `String` key). Cloned at the few emit sites; cheap (specials are rare).
#[derive(Clone, Debug, PartialEq)]
pub enum ActionRequest {
    /// Spawn a melee hitbox in front of the actor.
    Melee {
        spec: MeleeActionSpec,
        origin: ae::Vec2,
        facing: f32,
        attack_axis: ae::Vec2,
    },
    /// Spawn a projectile traveling in `dir`. Used by NPC / enemy /
    /// boss ranged: a single "fire now" edge resolved from
    /// `frame.fire = Some(...)` by [`resolve`]. Player ranged uses
    /// `PlayerProjectileTick` instead so the EFFECTS consumer can
    /// drive the charge state machine + motion-recognition buffer.
    Ranged {
        spec: RangedActionSpec,
        origin: ae::Vec2,
        /// Direction in the frame named by `dir_policy`.
        dir: ae::Vec2,
        /// Frame policy for `dir`; consumers convert at their own simulation
        /// seam, where the actor's current acceleration frame is known.
        dir_policy: ae::GameplayFramePolicy,
    },
    /// Trigger the actor's special. Resolved by the per-actor
    /// special handler (player ability system, boss encounter
    /// driver, etc.).
    Special { spec: SpecialActionSpec },
    /// Per-tick player projectile signal — drives the player
    /// projectile EFFECTS consumer's charge state machine + motion
    /// recognition buffer. Emitted by a dedicated player-projectile
    /// emit system (not by [`resolve`]) because the per-player
    /// charge tiers / projectile kinds live in the projectile
    /// system's own config rather than as a per-actor `ActionSet`
    /// capability. Keeps the player's combat verbs flowing through
    /// the same `ActorActionMessage` channel as melee and NPC
    /// ranged.
    ///
    /// The variant intentionally omits `origin` and `facing` — the
    /// projectile EFFECTS consumer reads those from its
    /// `BodyKinematics` query (the authoritative source of player
    /// body position / facing), so dragging them through the
    /// message would just duplicate state and force the emit side
    /// to query Transform too.
    PlayerProjectileTick {
        /// Movement axis sample (mirrors `ActorControlFrame::
        /// desired_vel`). Pushed into the motion-recognition buffer
        /// every tick so QCF / half-circle detection survives the
        /// migration.
        axis: ae::Vec2,
        /// Aim direction in the controlled actor's local frame. Zero = use facing.
        aim: ae::Vec2,
        /// Rising edge: projectile button pressed this tick.
        press: bool,
        /// Sustain: projectile button held this tick.
        held: bool,
        /// Falling edge: projectile button released this tick.
        released: bool,
    },
}

impl std::fmt::Display for ActionRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Melee { origin, facing, .. } => {
                write!(f, "{}(at {:?} facing {:+.0})", self.label(), origin, facing,)
            }
            Self::Ranged {
                origin,
                dir,
                dir_policy,
                ..
            } => {
                write!(
                    f,
                    "{}(from {:?} dir {:?} {:?})",
                    self.label(),
                    origin,
                    dir,
                    dir_policy,
                )
            }
            Self::Special { .. } => write!(f, "{}", self.label()),
            Self::PlayerProjectileTick {
                press,
                held,
                released,
                ..
            } => {
                let edge = if *press {
                    "press"
                } else if *released {
                    "release"
                } else if *held {
                    "held"
                } else {
                    "sample"
                };
                write!(f, "{}({})", self.label(), edge)
            }
        }
    }
}

impl ActionRequest {
    /// Short label naming the request kind ("melee_swipe",
    /// "ranged_bolt", "special", …). Useful for
    /// trace logs, debug overlays, and grep-friendly diagnostics
    /// without the verbose Debug rendering.
    #[allow(dead_code, reason = "diagnostic helper for the EFFECTS-flip migration")]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Melee { spec, .. } => match spec {
                MeleeActionSpec::Swipe(_) => "melee_swipe",
                MeleeActionSpec::Lunge(_) => "melee_lunge",
                MeleeActionSpec::Slam(_) => "melee_slam",
                MeleeActionSpec::Bite(_) => "melee_bite",
                MeleeActionSpec::PunchWeak(_) => "melee_punch_weak",
            },
            Self::Ranged { spec, .. } => match spec {
                RangedActionSpec::Rock { .. } => "ranged_rock",
                RangedActionSpec::Arrow { .. } => "ranged_arrow",
                RangedActionSpec::Pistol { .. } => "ranged_pistol",
                RangedActionSpec::Bolt { .. } => "ranged_bolt",
            },
            Self::Special { spec } => match spec {
                // Open content special — the key carries the specific
                // identity (e.g. `overfit_volley`); this static label is
                // just the kind.
                SpecialActionSpec::Special(_) => "special",
            },
            Self::PlayerProjectileTick { .. } => "player_projectile_tick",
        }
    }
}

/// Resolve a brain's abstract control frame into 0..N concrete
/// action requests using the actor's `ActionSet`. Pure function;
/// no Bevy, no side effects. Most ticks emit zero or one request;
/// multi-request ticks are the boss-pattern case (e.g. a phase that
/// simultaneously fires and lunges).
pub fn resolve(
    actions: &ActionSet,
    frame: &crate::actor::control::ActorControlFrame,
    origin: ae::Vec2,
) -> Vec<ActionRequest> {
    let mut out = Vec::with_capacity(2);
    // A melee swing is triggered by the attack button OR the DEDICATED POGO button:
    // pogo is the air-down variant of the same swing (its intent is resolved to
    // `AirDown`/`can_pogo` downstream in `start_attack` from `frame.pogo_pressed`).
    // Without the pogo trigger here the dedicated pogo button would emit no melee
    // message, so `start_body_melee` would never start the swing that carries the
    // bounce — the pogo-owned-by-the-sandbox-hitbox contract. AI brains never set
    // `pogo_pressed`, so this only ever fires for a player-controlled body.
    if frame.melee_pressed || frame.pogo_pressed {
        if let Some(spec) = actions.melee {
            out.push(ActionRequest::Melee {
                spec,
                origin,
                facing: frame.facing,
                attack_axis: frame.attack_axis,
            });
        }
    }
    if let Some(req) = frame.fire {
        if let Some(spec) = actions.ranged {
            // Today extracts `dir` off the engine's
            // `ActorFireRequest` for compat with the existing
            // enemy/boss callers. When `frame.fire` is narrowed to
            // `Option<Vec2>` (speed in ActionSet), this becomes
            // `dir: req`.
            out.push(ActionRequest::Ranged {
                spec,
                origin,
                dir: req.dir,
                dir_policy: req.dir_policy,
            });
        }
    }
    if frame.special_pressed {
        if let Some(spec) = &actions.special {
            out.push(ActionRequest::Special { spec: spec.clone() });
        }
    }
    out
}

#[cfg(test)]
mod tests;
