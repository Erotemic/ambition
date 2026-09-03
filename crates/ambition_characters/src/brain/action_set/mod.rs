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

use ambition_platformer2d_core as ae;
use bevy::ecs::component::Component;

/// Per-entity capability set. Resolves abstract brain intent
/// (control-frame fields) into concrete effect requests
/// ([`ActionRequest`]) that the EFFECTS-stage spawn systems consume.
///
/// Construct via [`ActionSet::peaceful`] for a "no attacks" baseline
/// and override only the slots that exist for this actor.
#[derive(Component, Clone, Debug, Default, PartialEq)]
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
    /// CAPABILITY marker: `Some(Special(move_id))` means this body HAS a signature special —
    /// the brain reads `special.is_some()` to decide whether to press `special_pressed`.
    /// Sourced from the archetype's `signature_move` at spawn. `None` = no special.
    pub special: Option<SpecialActionSpec>,
}

/// A body's kit before equipment — what its IDENTITY alone grants.
///
/// Worn equipment may grant action verbs, and a grant has to be revocable: a row
/// that is consumed, downgraded, or unequipped must take its verb with it. That is
/// only possible against a known un-granted baseline, because "the live
/// `ActionSet` minus this row's grants" is not recoverable once two rows have
/// overlaid the same slot.
///
/// So the identity derivation writes what it produced HERE, and the live
/// [`ActionSet`] / `ActorMoveset` become a pure function of
/// `identity + worn equipment`, recomputed whenever either side changes. Any
/// equipment mutation — from a pickup, a menu, or a hit that spends armor and
/// splices in a downgrade — reconciles through that one derivation, for any body
/// and any controller.
#[derive(Component, Clone, Debug, Default)]
pub struct IdentityKit {
    /// The action set the body's identity derived, before any grant overlay.
    pub action_set: ActionSet,
    /// The moveset the body's identity derived, before any granted verb overlay.
    /// Held as the derivation base so a REVOKED verb's move disappears with it
    /// rather than lingering in an overlay-only rebuild.
    pub moveset: ambition_entity_catalog::MovesetContract,
}

impl IdentityKit {
    /// Publish what identity alone derived, from the pair it derived.
    ///
    /// Both construction paths built this struct literally — the spawn bundle and the persona
    /// derive — which is two places deciding what "the baseline" contains.
    ///
    /// One constructor, so the pair that defines a baseline is stated once.
    pub fn of(action_set: ActionSet, moveset: ambition_entity_catalog::MovesetContract) -> Self {
        Self {
            action_set,
            moveset,
        }
    }
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

    /// This repertoire, narrowed to what the body may currently do.
    ///
    /// Splitting them is what lets a character AUTHOR its canonical repertoire
    /// and progression filter it: the definition says the robot has a swipe, a
    /// bolt and a bubble shield; this says today it has two of them.
    ///
    /// `ranged` is deliberately ungated, and that is inherited rather than
    /// chosen: there is no projectile ability in `AbilitySet`, so gating it here
    /// would silently disarm every ranged character. When such a flag exists this
    /// is the one place that changes.
    ///
    /// `move_style` is a BODY fact and is never filtered — a body that may not
    /// attack still walks the way it walks.
    pub fn gated_by(&self, abilities: ambition_platformer2d_core::AbilitySet) -> Self {
        Self {
            melee: abilities.attack.then(|| self.melee.clone()).flatten(),
            special: abilities.shield.then(|| self.special.clone()).flatten(),
            ranged: self.ranged.clone(),
            move_style: self.move_style,
        }
    }
}

/// What a plain `Attack` does to / with a held item — authored on the spec
/// instead of a hardcoded id-chain in `item_pickup::throw_held_item_system`
/// (Refactor 5). The narrow vocabulary the "Pick-up / throw held items" item
/// named; not a generic plugin system.
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
        if let Some(ranged) = &self.ranged {
            actions.ranged = Some(ranged.clone());
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
                ranged: Some(
                    RangedActionSpec::bolt(500.0, 2)
                        .with_visual(LASERSWORD_VISUAL)
                        .with_discharge(gun_sword_discharge())
                        .with_flight(held_shot_flight(500.0)),
                ),
                use_behavior: HeldUseBehavior::Auto,
            },
        );
        // ⭐ THE BOMB SHE LAYS. A held item with no verb of its own — the whole
        // point of it is that it is a THING somebody carries, and its behaviour
        // lives on the object's own fuse rather than on a button.
        //
        // ⛔ IT MUST BE REGISTERED OR THE MOVE IS HALF A MOVE. `pickup_held_item_system`
        // resolves a ground item to the spec it becomes in a hand, and an
        // unregistered id is an object nobody can pick up — which is exactly
        // half of what Jon asked the down-B for.
        items.insert(
            "polygon_bomb",
            HeldItemSpec {
                id: "polygon_bomb".into(),
                melee: None,
                ranged: None,
                // ⛔ `Auto`, NOT `UseSystem`. A pure throwable is released by
                // the ordinary throw road (`throw_held_item_system`); a
                // `UseSystem` item would have Attack intercepted by a fire
                // system that does not exist for it, and pressing Attack with a
                // bomb in hand would do nothing at all.
                use_behavior: HeldUseBehavior::Auto,
            },
        );
        // ⭐ THE POLYGON'S PONYTAIL, taken hold of for one move and let go again.
        //
        // ⛔ A HELD ITEM FOR A THING THAT IS PART OF HER, and that is the point
        // rather than a compromise: the seam `MoveSpec::equips` opens is "while
        // this move plays, the fighter is WIELDING this, and its ranged verb is
        // the shot" — and grabbing your own tail and throwing it is exactly
        // that. The alternative was a second ranged slot on the character, which
        // would have made every fighter's action set carry a field one of them
        // uses.
        items.insert(
            "polygon_ponytail",
            HeldItemSpec {
                id: "polygon_ponytail".into(),
                melee: None,
                ranged: Some(
                    RangedActionSpec::bolt(430.0, 7)
                        // ⭐ IT COMES BACK. `0.34` to the turnaround means it is
                        // home at `0.68` — long enough to cross the spacing she
                        // fights at, short enough that throwing it is a
                        // commitment rather than a zoning wall.
                        .with_flight(ProjectileFlight::boomerang(0.34))
                        .with_visual("polygon_bolt")
                        // The move's own recovery is the cadence: a second
                        // recharge would refuse a shot the move was already
                        // accepted to fire.
                        .with_refire(0.0),
                ),
                use_behavior: HeldUseBehavior::Auto,
            },
        );
        // ⭐ THE ADMIRAL'S OWN GUN-SWORD, drawn for one move and put away again.
        //
        // ⛔ A ROW OF ITS OWN RATHER THAN A LOUDER `gun_sword`, because the
        // shared one is a PICKUP in the adventure game and a raider's sidearm,
        // and a side-special's payoff is not the number either of those should
        // be balanced around. Same art, same discharge, same hand — a different
        // weapon.
        items.insert(
            "admiral_gun_sword",
            HeldItemSpec {
                id: "admiral_gun_sword".into(),
                melee: None,
                ranged: Some(
                    RangedActionSpec::bolt(620.0, 8)
                        // ⭐ THE SAME DISCHARGE THE ROW'S OWN COMMENT CLAIMS —
                        // *"same art, same discharge, same hand"* — and until it
                        // was authored here that sentence was false: the shot's
                        // look, muzzle, cue and kick were decided by a compare
                        // against the string `"gun_sword"`, which this weapon is
                        // not.
                        .with_visual(LASERSWORD_VISUAL)
                        .with_discharge(gun_sword_discharge())
                        // ⭐ THE HALF-PLANE, WHICH IS JON'S RULE VERBATIM: the
                        // player picks a side and the weapon picks the angle
                        // within it. A foe behind the admiral is not a target
                        // for a shot he aimed forwards.
                        //
                        // The range is a little over half the smash stage's
                        // width, so the assist reaches across a normal spacing
                        // exchange and not across the whole room.
                        .with_aim_assist(AimAssist::half_plane(360.0))
                        // ⛔ THE MOVE'S OWN RECOVERY IS THE CADENCE HERE. This
                        // weapon is drawn by a special and put away with it, so
                        // a second recharge on top would refuse a shot the move
                        // had already been accepted to fire — the exact
                        // accept-then-veto `refire_s`'s own doc warns about.
                        .with_refire(0.0),
                ),
                use_behavior: HeldUseBehavior::Auto,
            },
        );
        items.insert(
            "gun_sword_heavy",
            HeldItemSpec {
                id: "gun_sword_heavy".into(),
                melee: None,
                ranged: Some(RangedActionSpec::bolt(500.0, 3)),
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
        // projectiles onto a zone ahead (a player-wielded analogue of the
        // apple-rain technique).
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
        // The Fireball ability fires a bolt that BURSTS where it lands: the same
        // projectile road as every other shot, with a splash authored on its
        // flight. The Bolt damage is the splash damage; the AOE box is what makes
        // it distinct from the gun-sword. Its look is its own sprite, not the
        // tinted energy ball the catalog's "fireball" id draws.
        items.insert(
            "fireball",
            HeldItemSpec {
                id: "fireball".into(),
                melee: None,
                ranged: Some(
                    RangedActionSpec::bolt(440.0, 3)
                        .with_visual(GAUNTLET_FIREBALL_VISUAL)
                        .with_discharge(Discharge {
                            muzzle: Muzzle::default(),
                            fire_sfx: Some("player.dash".into()),
                            recoil: 0.0,
                        })
                        .with_flight(held_shot_flight(440.0).with_splash(FIREBALL_SPLASH_HALF)),
                ),
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

/// Every held-item id this registry answers to, sorted.
///
/// The binding sweep needs the list, not just the lookup: a `GroundItemSpec`
/// naming an unregistered item is (in its own doc's words) "skipped at spawn
/// rather than erroring", so the only way that typo becomes visible is to resolve
/// the room's references against what actually exists, ahead of time.
pub fn held_item_ids() -> Vec<String> {
    let mut ids: Vec<String> = HELD_ITEMS.keys().map(|id| (*id).to_owned()).collect();
    ids.sort();
    ids
}

/// Concrete melee actions an actor can perform. Each variant carries
/// its own animation timing (windup → active → recover) — there
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

/// How a shot FLIES once it leaves the barrel — the authored physics of a
/// projectile, independent of who fired it.
///
/// This is the authoring seam for the rest: content states the arc, the bounce policy, and the
/// lifetime it wants, and the shared projectile body steps exactly that. Nothing here names an
/// ability or a firer.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
pub struct ProjectileFlight {
    /// Downward acceleration along gravity, px/s². `0` flies straight.
    pub gravity: f32,
    /// How many times the shot may bounce off a valid support face before it
    /// expires. `0` with [`Self::bounce_on_world_contact`] false is a shot that
    /// dies on first contact.
    pub bounces: u8,
    /// Whether world contact bounces the shot (vs. expiring it).
    pub bounce_on_world_contact: bool,
    /// Seconds before the shot expires on its own.
    pub max_lifetime: f32,
    /// Half-extent of the shot's body.
    pub half_extent: ae::Vec2,
    /// Seconds until this shot has stopped and starts coming BACK, or `None`
    /// for a shot that flies on — which is every shot in the game but one.
    ///
    /// ⭐ THE BOOMERANG, AS ONE NUMBER. Jon, 2026-08-27: *"I think the
    /// projectile polygon should be able to use her ponytail as a boomarang for
    /// her side-b."* It is resolved into a constant acceleration back along the
    /// launch axis at spawn, so a thrown tail slows, stops, and comes home the
    /// way it went out — and needs no reference to the thrower, which is what
    /// keeps a returning shot inside the projectile stepper's pure signature.
    ///
    /// ⛔ THE TIME TO THE TURNAROUND, NOT THE TIME HOME. The return leg costs
    /// the same as the outbound one, so a shot authored at `0.4` passes back
    /// through the launch point at `0.8` and wants a lifetime a little past
    /// that. [`Self::boomerang`] does that arithmetic.
    pub boomerang_return_s: Option<f32>,
    /// Half-extent of the burst this shot deals where it lands, or `0.0` for a
    /// shot that hits only what it touched. The fireball's splash. Authored
    /// here so a fireball is a bolt with a splash on the ONE projectile road,
    /// not a second projectile simulation keyed on its item id.
    #[serde(default)]
    pub splash_half_extent: f32,
}

impl ProjectileFlight {
    /// A straight, non-bouncing shot — the historical default every ranged pool
    /// hardcoded before flight was authorable.
    pub const STRAIGHT: Self = Self {
        gravity: 0.0,
        bounces: 0,
        bounce_on_world_contact: false,
        max_lifetime: 2.4,
        half_extent: ae::Vec2::new(10.0, 8.0),
        boomerang_return_s: None,
        splash_half_extent: 0.0,
    };

    /// A shot that goes out, stops, and comes back — `out_s` to the turnaround.
    ///
    /// The lifetime is the ROUND TRIP EXACTLY, so the tail expires at the hand
    /// that threw it rather than sailing off behind her.
    ///
    /// ⭐⭐ AND THE ROUND TRIP IS ANALYTIC, not a guess. The return is a constant
    /// acceleration `-v0 / out_s` (resolved at spawn from the launch velocity),
    /// so the shot's displacement is `v0·t − v0·t²/2·out_s` and it is back at
    /// the throw point at exactly `t = 2·out_s`. There is nothing to tune.
    ///
    /// ⛔⛔ IT USED TO CARRY `+ 0.15` FOR "just past the hand", and 0.15s of a
    /// shot that is accelerating backwards is not a little — measured against the
    /// real 60Hz integrator at the ponytail's own 430 px/s and `out_s = 0.34`,
    /// the tail expired **79.2 px BEHIND the launch point travelling 603 px/s
    /// backwards**. A fast rearward projectile, where the
    /// doc said "caught". Deleting the term lands it 1.4 px past the hand, and
    /// the same deletion holds across the range: `out_s` 0.25 → 7.2 px, 0.5 →
    /// 0.0 px, against 82 and 73 before.
    pub const fn boomerang(out_s: f32) -> Self {
        Self {
            boomerang_return_s: Some(out_s),
            max_lifetime: out_s * 2.0,
            ..Self::STRAIGHT
        }
    }

    /// An arcing shot that skips off floors — gravity plus a bounce budget.
    pub const fn arcing(gravity: f32, bounces: u8) -> Self {
        Self {
            gravity,
            bounces,
            bounce_on_world_contact: true,
            ..Self::STRAIGHT
        }
    }

    pub const fn with_lifetime(mut self, seconds: f32) -> Self {
        self.max_lifetime = seconds;
        self
    }

    pub const fn with_half_extent(mut self, half_extent: ae::Vec2) -> Self {
        self.half_extent = half_extent;
        self
    }

    /// A shot that bursts where it lands, over a box of this half-extent.
    pub const fn with_splash(mut self, half_extent: f32) -> Self {
        self.splash_half_extent = half_extent;
        self
    }
}

/// The CADENCE archetype of a ranged action — how long the body takes to draw and
/// recover. Distinct from the shot's flight: a slow-drawn bow and a snap pistol
/// can fire projectiles that behave identically, and one archetype's cadence can
/// launch wildly different shots.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Deserialize)]
pub enum RangedStyle {
    /// Thrown rock (skirmishers / peaceful-turned-hostile NPCs).
    Rock,
    /// Drawn arrow. Slower windup than Rock, more damage.
    Arrow,
    /// Pistol snap-shot (pirate skirmishers).
    Pistol,
    /// Magical bolt (bosses).
    #[default]
    Bolt,
}

/// A concrete ranged action: a cadence, a shot, and optionally how that shot flies
/// and what it looks like.
///
/// `flight` / `visual` are `None` for "whatever the firing pool's default is",
/// which is what every ranged action did before they existed. Authoring either one
/// is how content gives a granted ranged verb its own identity without the
/// projectile stepper learning a single ability name.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct RangedActionSpec {
    pub style: RangedStyle,
    /// Launch speed. The brain emits `frame.fire = Some(dir)`; the EFFECTS stage
    /// reads this to set the projectile speed.
    pub speed: f32,
    /// Damage on hit.
    pub damage: i32,
    /// Authored flight physics. `None` = the pool's straight default.
    #[serde(default)]
    pub flight: Option<ProjectileFlight>,
    /// Authored `ProjectileVisualId`. `None` = the firer's default look.
    #[serde(default)]
    pub visual: Option<String>,
    /// How a CHARGED release of this shot differs from a tap. `None` = a shot
    /// that does not charge, which is every ranged action that has not opted in.
    #[serde(default)]
    pub charge: Option<RangedCharge>,
    /// Seconds the WEAPON needs between shots.
    ///
    /// ⭐⭐ THIS IS THE WEAPON'S RECHARGE, NOT THE MOVE'S RECOVERY, and keeping
    /// them apart is the whole point of the field. A firing move authors how
    /// long the fighter is committed to the animation; this authors how long
    /// the weapon is unavailable. They used to be one number by accident, so
    /// making a shot come out faster also took the fighter's legs away for
    /// longer.
    ///
    /// ⛔⛔ IT IS CHECKED WHERE THE MOVE IS ACCEPTED, never where the shot is
    /// spawned. A move accepted with a hot weapon is guaranteed to fire: the
    /// old arrangement accepted the move and then vetoed its projectile
    /// downstream, which meant the animation played, the sound played, and
    /// nothing came out.
    #[serde(default = "default_ranged_refire_s")]
    pub refire_s: f32,
    /// How far this weapon will BEND a shot toward a target the shooter was
    /// already pointing at. `None` = a shot that goes exactly where it was
    /// aimed, which is every ranged action that has not opted in.
    #[serde(default)]
    pub aim_assist: Option<AimAssist>,
    /// How this shot LEAVES the weapon — muzzle, cue, kick. `None` = the plain
    /// discharge every ranged action had before weapons could state one.
    #[serde(default)]
    pub discharge: Option<Discharge>,
}

/// HOW A SHOT LEAVES THE WEAPON — the choices that are about the discharge
/// rather than about what the shot does on arrival.
///
/// ⭐⭐ AUTHORED, BECAUSE IT WAS A STRING COMPARE. `brain_effects` decided the
/// projectile visual, the muzzle, the fire cue and the recoil from
/// `held_item_id == Some("gun_sword")`, so the Pirate Admiral's side-B — which
/// draws `admiral_gun_sword`, a row whose own comment says *"same art, same
/// discharge, same hand"* — got none of them. A comment stating a rule the code
/// contradicts is the tell.
///
/// ⛔ AND THE ANSWER IS NOT A SECOND STRING COMPARE. Two weapons sharing a
/// discharge share THIS, and keep their own damage, speed and aim assist; a
/// third weapon that wants the look and not the kick authors the difference
/// instead of asking to be added to a list in another crate.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct Discharge {
    /// Where the shot is born.
    pub muzzle: Muzzle,
    /// The cue that plays the moment it fires. `None` = the weapon says nothing
    /// of its own, which is every ranged action that has not opted in.
    pub fire_sfx: Option<String>,
    /// How hard firing shoves the shooter back, in world px/s along the negative
    /// fire direction.
    pub recoil: f32,
}

/// The generic body's recoil. Small on purpose: it is a bit of feedback, not a
/// movement option.
pub const DEFAULT_RANGED_RECOIL: f32 = 60.0;

impl Default for Discharge {
    fn default() -> Self {
        Self {
            muzzle: Muzzle::default(),
            fire_sfx: None,
            recoil: DEFAULT_RANGED_RECOIL,
        }
    }
}

/// Where a shot is born.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Deserialize)]
pub enum Muzzle {
    /// A little above the body's own origin — what a body with no weapon in
    /// view does, and what every ranged action did before this existed.
    #[default]
    BodyOrigin,
    /// The shooter's HAND, pushed `ahead` px along the shot. What a drawn weapon
    /// does, so the shot leaves the barrel the player can see rather than the
    /// fighter's midriff — and it follows the hand whether the pirate is still
    /// mounted or has fallen off the shark.
    Hand { ahead: f32 },
}

/// The gun-sword's discharge: the spinning blade, the hand, the cue and the kick
/// that made the pirate's shot feel like a cannon.
///
/// ⭐ ONE VALUE FOR BOTH GUN-SWORDS. The adventure game's pickup and the
/// admiral's drawn sidearm are different WEAPONS — different damage, speed and
/// assist, and balanced for different games — and the same DISCHARGE. That is
/// exactly the split this type exists to express.
pub fn gun_sword_discharge() -> Discharge {
    Discharge {
        muzzle: Muzzle::Hand { ahead: 18.0 },
        fire_sfx: Some("weapon.lasersword.fire".into()),
        // Visibly knocks the rider and shark back together, which is the whole
        // read on a pirate who fires while mounted.
        recoil: 380.0,
    }
}

/// The gun-sword's shot LOOKS like a spinning blade.
pub const LASERSWORD_VISUAL: &str = "lasersword";

/// The gauntlet fireball's shot: its own glowing sprite, radial, drawn a touch
/// over its contact box. Registered by the game's projectile visual catalog.
pub const GAUNTLET_FIREBALL_VISUAL: &str = "gauntlet_fireball";

/// The splash box a fireball bursts with where it lands.
pub const FIREBALL_SPLASH_HALF: f32 = 56.0;

/// The flight every hand-fired held shot has had since it existed: a 24 x 18 px
/// body that flies straight until it has covered 1600 px. Authored here, once,
/// instead of a range gate inside a second projectile stepper.
pub const fn held_shot_flight(speed: f32) -> ProjectileFlight {
    ProjectileFlight::STRAIGHT
        .with_half_extent(ae::Vec2::new(12.0, 9.0))
        .with_lifetime(HELD_SHOT_MAX_RANGE / speed)
}

/// How far a hand-fired held shot flies before it expires on its own.
pub const HELD_SHOT_MAX_RANGE: f32 = 1600.0;

/// A service pistol's shot LOOKS like a bullet — brass slug, hot tip, short
/// wake, authored travelling +x so `FlipToTravel` mirrors it correctly.
///
/// ⛔ NAMED HERE FOR THE REASON `LASERSWORD_VISUAL` IS: the id is a contract
/// between a weapon that fires and the content catalog that registers the look,
/// and a bare string at each end drifts silently — an unregistered id does not
/// fail, it quietly draws the engine's generic quad.
pub const PISTOL_ROUND_VISUAL: &str = "pistol_round";

/// A weapon's willingness to correct the shooter's aim.
///
/// ⭐ THE COMMANDED DIRECTION IS STILL THE DECISION. Jon, 2026-08-27, on the
/// pirate's gun-sword: *"When the side-b resolves it should locate the nearest
/// opponent and angle the equipped gun and shot so it fires in their direction
/// IF THEY ARE IN THE HALF PLANE the side-b was directed towards."* The player
/// chooses a side; the weapon chooses an angle within it. A fighter behind you
/// is not a target for a shot you aimed forwards, and that is what makes the
/// move a read rather than a homing missile.
///
/// ⛔ IT BENDS THE DIRECTION AND NOTHING ELSE. The shot still travels, can still
/// be shielded, still misses a target that moves — this is a firing ANGLE, not a
/// guarantee of contact.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
pub struct AimAssist {
    /// The widest angle from the commanded direction that still counts as "the
    /// way I was pointing", in radians. `FRAC_PI_2` is Jon's half-plane.
    pub max_angle_rad: f32,
    /// How far a target may be and still attract the shot, in world px. Past
    /// this the shot goes where it was aimed.
    pub max_range: f32,
}

impl AimAssist {
    /// The half-plane the commanded direction faces, out to `max_range`.
    pub const fn half_plane(max_range: f32) -> Self {
        Self {
            max_angle_rad: std::f32::consts::FRAC_PI_2,
            max_range,
        }
    }
}

/// The recharge a ranged action gets when it authors none.
///
/// ⭐ THIS NUMBER IS THE GAME THAT WAS PLAYTESTED. It began life as a generic
/// anti-spam floor on every ranged ATTEMPT, one layer below anything a
/// character could author — but four months of play happened underneath it, so
/// it is now also the de facto cadence of ranged combat. Measured 2026-08-23:
/// 22 of 28 authored ranged events in the duel arena were being refused by it,
/// and removing it made every ranged fighter fire ~3.7x faster and stop
/// meleeing altogether.
///
/// ⛔ SO IT MOVED RATHER THAN DIED. Per-character tuning starts from this
/// baseline, one weapon at a time — not from whatever falls out of deleting it.
pub const DEFAULT_RANGED_REFIRE_S: f32 = 1.1;

fn default_ranged_refire_s() -> f32 {
    DEFAULT_RANGED_REFIRE_S
}

/// The ladder a held shot climbs.
///
/// ⭐ THE SHOT IS THE PAYOFF, which is what makes this its own type rather than
/// another `smash_charge_mult`. A charged melee swing pays in one number: the
/// volume it already spawns hits harder. A charged shot pays in an OBJECT — it
/// leaves bigger, faster, and looking like a different thing — and a player has
/// to be able to read which one is coming at them before it arrives.
///
/// ⛔ THE TIERS ARE THE VISUAL'S, not the damage's. Damage and speed interpolate
/// smoothly from `1.0` at a tap to their multipliers at a full hold, because a
/// charge is continuous; the LOOK steps, because a stepped look is what a player
/// can actually read at a glance on a busy stage. Nothing reconciles the two on
/// purpose.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct RangedCharge {
    /// Damage at a FULL hold, as a multiple of the uncharged shot.
    pub damage_mult: f32,
    /// Launch speed at a full hold, as a multiple.
    pub speed_mult: f32,
    /// Half-extent at a full hold, as a multiple. A charged shot that hits in
    /// the same box it always did reads as a lie the moment it is drawn bigger.
    pub size_mult: f32,
    /// One `ProjectileVisualId` per tier, WEAKEST FIRST. Empty = one look at
    /// every charge, which is the honest answer for a shot whose art does not
    /// change.
    pub visuals: Vec<String>,
}

impl RangedCharge {
    /// Which tier a `0..=1` charge fraction lands in. `None` for a ladder with
    /// no rungs.
    fn tier(&self, fraction: f32) -> Option<&str> {
        let count = self.visuals.len();
        if count == 0 {
            return None;
        }
        // `min(count - 1)` and not a clamp on the fraction: a full charge is
        // exactly `1.0`, and `1.0 * count` indexes one past the end.
        let index = ((fraction.clamp(0.0, 1.0) * count as f32) as usize).min(count - 1);
        Some(&self.visuals[index])
    }
}

impl RangedActionSpec {
    pub fn new(style: RangedStyle, speed: f32, damage: i32) -> Self {
        Self {
            style,
            speed,
            damage,
            flight: None,
            visual: None,
            charge: None,
            refire_s: DEFAULT_RANGED_REFIRE_S,
            aim_assist: None,
            discharge: None,
        }
    }

    pub fn rock(speed: f32, damage: i32) -> Self {
        Self::new(RangedStyle::Rock, speed, damage)
    }
    pub fn arrow(speed: f32, damage: i32) -> Self {
        Self::new(RangedStyle::Arrow, speed, damage)
    }
    pub fn pistol(speed: f32, damage: i32) -> Self {
        Self::new(RangedStyle::Pistol, speed, damage)
    }
    pub fn bolt(speed: f32, damage: i32) -> Self {
        Self::new(RangedStyle::Bolt, speed, damage)
    }

    /// Author how this action's shot flies.
    pub fn with_flight(mut self, flight: ProjectileFlight) -> Self {
        self.flight = Some(flight);
        self
    }

    /// Author the `ProjectileVisualId` this action's shot carries.
    pub fn with_visual(mut self, visual: impl Into<String>) -> Self {
        self.visual = Some(visual.into());
        self
    }

    /// Author how far this weapon will bend a shot toward a target.
    /// State how this shot leaves the weapon. See [`Discharge`].
    pub fn with_discharge(mut self, discharge: Discharge) -> Self {
        self.discharge = Some(discharge);
        self
    }

    pub fn with_aim_assist(mut self, assist: AimAssist) -> Self {
        self.aim_assist = Some(assist);
        self
    }

    /// Author how long this weapon takes to recharge between shots.
    pub fn with_refire(mut self, refire_s: f32) -> Self {
        self.refire_s = refire_s.max(0.0);
        self
    }

    /// Effective launch speed.
    pub fn speed(&self) -> f32 {
        self.speed
    }

    /// Damage on hit.
    pub fn damage(&self) -> i32 {
        self.damage
    }

    /// Author how a held release of this shot differs from a tap.
    pub fn with_charge(mut self, charge: RangedCharge) -> Self {
        self.charge = Some(charge);
        self
    }

    /// This shot as released at `fraction` of a full charge.
    ///
    /// ⭐ THE ONE PLACE A CHARGE BECOMES A SHOT. The fire site asks for the spec
    /// at the fraction the move's playback froze and spawns whatever comes back,
    /// so a charged weapon and an ordinary one leave through identical code.
    ///
    /// A spec with no ladder returns itself unchanged at every fraction, which
    /// is byte-parity for every ranged action that existed before charging did.
    pub fn at_charge(&self, fraction: f32) -> Self {
        let Some(charge) = self.charge.as_ref() else {
            return self.clone();
        };
        let f = fraction.clamp(0.0, 1.0);
        let lerp = |mult: f32| 1.0 + f * (mult - 1.0);
        let mut out = self.clone();
        out.damage = ((self.damage as f32) * lerp(charge.damage_mult)).round() as i32;
        out.speed = self.speed * lerp(charge.speed_mult);
        if let Some(flight) = out.flight.as_mut() {
            flight.half_extent *= lerp(charge.size_mult);
        }
        if let Some(visual) = charge.tier(f) {
            out.visual = Some(visual.to_string());
        }
        out
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

    /// The full authored timeline as `(windup_s, active_s, recover_s, damage, reach_px)`.
    /// The single accessor the melee→moveset subsumption reads to author a body's
    /// basic swing as a data-driven `"attack"` [`MoveSpec`](ambition_entity_catalog::MoveSpec)
    /// — Startup/Active(forward volume)/Recovery on the owner's proper-time clock —
    /// so a plain melee runs through the SAME moveset runtime as its specials
    /// (fable review §A1 / §3a). Variant-specific extras (Lunge `step_px`, Slam
    /// `hop_height_px`) are not carried yet — a self-motion window is a
    /// parameterizable follow-up.
    pub fn timeline(self) -> (f32, f32, f32, i32, f32) {
        match self {
            Self::Swipe(s) => (s.windup_s, s.active_s, s.recover_s, s.damage, s.reach_px),
            Self::Lunge(s) => (s.windup_s, s.active_s, s.recover_s, s.damage, s.reach_px),
            Self::Slam(s) => (s.windup_s, s.active_s, s.recover_s, s.damage, s.reach_px),
            Self::Bite(s) => (s.windup_s, s.active_s, s.recover_s, s.damage, s.reach_px),
            Self::PunchWeak(s) => (s.windup_s, s.active_s, s.recover_s, s.damage, s.reach_px),
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
    /// key (snake_case, e.g. `"overfit_volley"`); the matching
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

/// Whether a ranged request is a controller POLL or a shot a move already
/// committed to.
///
/// ⭐⭐ TWO ROADS REACH ONE CONSUMER, and they owe different things. A brain
/// emits `fire` on every in-band tick and never rate-limits itself, so its
/// request is an ATTEMPT and the weapon's recharge is the only thing standing
/// between it and a stream of projectiles. A moveset `Ranged` event is the
/// other road: the body already accepted a move, paid for it with its
/// recharge, and has been playing the animation for a quarter of a second.
///
/// ⛔ WITHOUT THIS DISTINCTION THE CONSUMER HAS TO GUESS, and it guessed the
/// same way for both — which is how an accepted Charge Shot could play its
/// windup, flash its muzzle, and fire nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RangedCommitment {
    /// A controller poll. The weapon's recharge refuses it silently, which is
    /// correct: nothing was promised and nothing was shown.
    #[default]
    Attempt,
    /// A move the body ACCEPTED authored this shot. The weapon was spent at
    /// acceptance, so the shot is guaranteed here.
    CommittedMove,
}

/// Concrete effect a brain's abstract intent resolves to, after
/// reading the actor's `ActionSet`. The EFFECTS-stage spawn systems
/// consume this list per actor per tick and translate each into a
/// real hitbox / projectile / FX.
///
/// ⭐ THIS IS A LIVE SEAM, NOT A PROPOSED ONE — the comment here said
/// otherwise until 2026-09-03, describing itself as "the *shape* of the
/// resolver output" whose wiring "lands in Chunk 3 when an actor first
/// uses a brain". That wiring landed. Carried by `ActorActionMessage`,
/// this type is consumed in production by the traversal abilities
/// (`abilities/traversal/{flyline,trapdoor,teleport}`), by
/// `features/ecs/brain_effects` and by `ambition_held_items`.
///
/// ⛔ AND THE STALE VERSION COST SOMETHING REAL. A planning page listed
/// "typed action vocabulary rather than free-form mutation" as an unmet
/// requirement of a future agentic-character runtime, because the seam's
/// own comment disclaimed it — a reader who trusts the type would have
/// designed this twice.
// Not `Copy`: the `Special` variant carries an owned `SpecialActionSpec`
// (open `String` key). Cloned at the few emit sites; cheap (specials are rare).
#[derive(Clone, Debug, PartialEq)]
pub enum ActionRequest {
    /// Spawn a melee hitbox in front of the actor.
    Melee {
        spec: MeleeActionSpec,
        origin: ae::Vec2,
        facing: f32,
        /// body-LOCAL, and typed so the request cannot lose the frame the
        /// control frame already established. It was `Vec2`, so the resolver had
        /// to call `.vec()` on a typed field and hand the request a vector that
        /// no longer said which space it was in — a round trip through an
        /// untyped seam for no gain.
        ///
        /// nothing in production READS this today; the directional resolution
        /// the doc on `ActorControlFrame::attack_axis` describes happens in
        /// `ambition_platformer2d::combat::moveset`, from the frame directly. Kept rather than
        /// deleted because that is a design question about where the resolution
        /// belongs, not a cleanup.
        attack_axis: ae::LocalAxes,
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
        /// WHO IS ASKING — and it is the only thing that decides whether the
        /// weapon's recharge may still refuse this shot. See
        /// [`RangedCommitment`].
        commitment: RangedCommitment,
    },
    /// Trigger the actor's special. Resolved by the per-actor
    /// special handler (player ability system, boss encounter
    /// driver, etc.).
    ///
    /// `params` carries the triggering [`EffectRef`](ambition_entity_catalog::EffectRef)'s
    /// opaque payload (A1 / R2.2): a moveset `Effect` event bridges its
    /// `effect.params` in here so the content technique keyed by `spec` can
    /// [`hydrate`](ambition_entity_catalog::ParamValue::hydrate) its own typed
    /// params. Brain-emitted specials (bubble_shield, a boss `Special(key)`
    /// beat) carry the empty default — they name a paramless technique.
    Special {
        spec: SpecialActionSpec,
        params: ambition_entity_catalog::ParamValue,
    },
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
            Self::Ranged { spec, .. } => match spec.style {
                RangedStyle::Rock => "ranged_rock",
                RangedStyle::Arrow => "ranged_arrow",
                RangedStyle::Pistol => "ranged_pistol",
                RangedStyle::Bolt => "ranged_bolt",
            },
            Self::Special { spec, .. } => match spec {
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
    // pogo is the air-down variant of the same swing (the moveset resolves it to the
    // `attack_air_down` move carrying the pogo on-hit technique from `pogo_pressed`).
    // Without the pogo trigger here the dedicated pogo button would emit no melee
    // message, so the moveset `"attack"` trigger would never start the swing that
    // carries the bounce. AI brains never set `pogo_pressed`, so this only ever
    // fires for a player-controlled body.
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
        if let Some(spec) = actions.ranged.clone() {
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
                // The brain polls; the weapon decides. See `RangedCommitment`.
                commitment: RangedCommitment::Attempt,
            });
        }
    }
    // NOTE: `special_pressed` is resolved by the MOVESET, not here. `ActionSet.special` is now a
    // pure CAPABILITY marker the brain reads to decide whether to press special; the move executes
    // through the moveset runtime, and content techniques fire via the `Effect{key}` bridge in
    // `dispatch_move_events`. Bosses already dispatched their multi-special repertoire through
    // `dispatch_boss_special`, never this arm.
    out
}

#[cfg(test)]
mod tests;

/// Derivation-time choice for how a body's ranged input executes. This is the
/// sole switch for folding ranged/special presets, applying ranged presentation,
/// and installing charge-projectile runtime state. `ChargesProjectiles` remains
/// the runtime marker.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RangedExecution {
    /// A chargeable projectile owns the ranged press; do not also fold the
    /// action set's ranged verb into the moveset.
    ChargedProjectile,
    /// A moveset verb derived from the action set's own `ranged` preset.
    ///
    /// What content-authored personas use. They have no charge mechanic and no
    /// shell `special` marker, so their special is authored into their moves
    /// rather than folded from the set.
    MovesetVerb,
}

impl RangedExecution {
    /// Whether a body executing this way carries the charge capability.
    pub fn charges_projectiles(self) -> bool {
        matches!(self, Self::ChargedProjectile)
    }
}
