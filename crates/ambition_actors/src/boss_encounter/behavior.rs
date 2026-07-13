//! Boss behavior-profile vocabulary (data-driven).
//!
//! `BossBehaviorProfile` / `BarkAnchorSpec` / `BossRewardProfile` /
//! `ActorSpriteMetrics` are the schemas every boss instance is authored INTO:
//! the named rows live in provider `boss_profiles.ron` fragments assembled in
//! the App-local [`super::BossCatalog`]. Owns movement/attacks/damage/hitbox
//! tuning (the engine `BossEncounterSpec` owns phase progression + HP).
//! `BossBehaviorProfile::from_data(catalog, "id")` clones an App-local row; the named
//! constructors (`clockwork_warden()` etc.) are thin lookups. Also holds
//! `boss_animation_keys_for_profile` (attack-profile -> sprite-row keys) and
//! `canonical_boss_id_from` (resolves the boss kind from LDtk name + brain).

use ambition_characters::brain::boss_pattern::{
    BossAttackPattern, BossAttackProfile, BossMovementProfile,
};
use ambition_engine_core as ae;

/// Live sandbox-side behavior tuning for a boss. This is deliberately separate
/// from `crate::boss_encounter::BossEncounterSpec`: the engine spec owns phase progression and HP
/// thresholds, while this profile owns sandbox movement, contact size, damage,
/// and hitbox shapes.
///
/// Every field here is authored in a provider's `boss_profiles.ron` fragment.
/// [`BossProfileRegistry`] remains a pure parser for focused tests and tools;
/// production providers assemble those rows into the App-local
/// [`super::BossCatalog`]. Adding a new boss is a new key + row in provider
/// data when it needs custom behavior; unknown authored bosses fall back
/// to the generic profile.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct BossBehaviorProfile {
    pub id: String,
    /// Sprite-registry target id: which sheet the boss draws from and keys its
    /// per-animation hit/hurtboxes against. `None` (the default) = the boss's
    /// own `id` IS the target (the common case). Author it only when the sheet
    /// target diverges from the id — the gradient sentinel / clockwork warden
    /// share the generic `"boss"` sheet, GNU-ton draws `"gnu_ton_boss"`, the
    /// mockingbird `"mockingbird_boss"`. Data-driven so the engine's sprite
    /// lookup names no boss (was a hardcoded id->target match in `sync.rs`).
    #[serde(default)]
    pub sprite_target: Option<String>,
    #[serde(default, with = "boss_vec2_option")]
    pub combat_size: Option<ae::Vec2>,
    pub movement: BossMovementProfile,
    /// Optional per-phase movement overrides. `None` means "use
    /// `movement` during this phase." Lets a boss escalate its
    /// movement personality across phases without changing the
    /// profile enum itself.
    #[serde(default)]
    pub movement_phase2: Option<BossMovementProfile>,
    #[serde(default)]
    pub movement_enrage: Option<BossMovementProfile>,
    /// Multiplier applied to movement speed while an active special
    /// strike is committed. `< 1.0` keeps the boss roughly anchored
    /// so World-space special hitboxes (saddle cross, minima pit)
    /// don't slide out from under the visual telegraph. `1.0` keeps
    /// pre-Gradient-Sentinel behavior.
    pub strike_speed_scale: f32,
    /// Optional self-dodge: while a strike is committed, the boss side-steps
    /// with the authored `(amplitude_px, frequency_hz)` so it can weave out of
    /// its OWN attack — GNU-ton dodges its apple rain. `None` (the default when
    /// the RON row omits it) = the boss holds its ground during strikes.
    /// Data-driven by design: the engine spawn path reads this instead of
    /// naming a specific boss, so the dependency points content -> core.
    #[serde(default)]
    pub self_dodge: Option<(f32, f32)>,
    /// Macro state machine tuning — when enabled, the boss runs an
    /// Engage / Approach / Retreat dance on top of the scripted
    /// attack schedule. See [`ambition_characters::brain::BossMacroTuning`].
    /// Use `BossMacroTuning::disabled()` for legacy "stand and
    /// fight" behavior.
    pub macro_tuning: ambition_characters::brain::BossMacroTuning,
    pub attacks: Vec<BossAttackProfile>,
    pub attack_cooldown: f32,
    pub attack_windup: f32,
    pub attack_active: f32,
    pub attack_damage: i32,
    pub body_damage: i32,
    /// How attack hitboxes are selected. `Cycle` (default for legacy bosses)
    /// rotates through `attacks` using the flat durations above. `Scripted`
    /// runs an authored phase-keyed timeline of telegraph / strike / rest
    /// beats and ignores `attacks` / `attack_cooldown` / `attack_windup` /
    /// `attack_active`.
    pub attack_pattern: BossAttackPattern,
    /// World-space anchor offset (in pixels) from the boss center where
    /// "hand"-class attacks should originate. For body-centered giants
    /// (GNU-ton) the entity transform sits at the scholar on the shoulder,
    /// not the giant's body — without this offset, hand hitboxes would
    /// hover near the scholar instead of where the giant's arms are. Y is
    /// world-space positive-down; leave at `Vec2::ZERO` for ordinary bosses.
    #[serde(default, with = "boss_vec2_required")]
    pub attack_origin_offset: ae::Vec2,
    /// World-space anchor offset (in pixels) from the boss center for
    /// projectile-like specials. Smirking Behemoth uses this to fire
    /// MemorizedVolley eye beams from its eye instead of its body center.
    #[serde(default, with = "boss_vec2_required")]
    pub projectile_origin_offset: ae::Vec2,
    /// Authored post-defeat reward. `None` (the default when the RON
    /// row omits `reward:`) means the boss drops nothing; `DropChest`
    /// spawns a reward chest at the given offset/size on defeat. This
    /// rides on the behavior profile so the whole boss is authored in
    /// one RON row.
    #[serde(default)]
    pub reward: BossRewardProfile,
    /// Catalog ability this boss grants on defeat (`"blink"`, `"fireball"`, …),
    /// dropped as a collectible pickup — "every boss a failed objective
    /// function; defeating it teaches its theorem." `None` (the default) = no
    /// ability drop. Content data: each boss embodies an ability, authored in
    /// `boss_profiles.ron`, so the engine's drop logic names no boss.
    #[serde(default)]
    pub reward_ability: Option<String>,
    /// The boss's signature wielded gauntlet (a held-item id like `"shockwave"`
    /// / `"meteor"`), dropped as a ground item the player picks up to wield the
    /// boss's own attack. `None` (the default) = no gauntlet drop. Content data
    /// (authored in `boss_profiles.ron`), distinct from [`Self::reward_ability`]
    /// which grants a catalog ability.
    #[serde(default)]
    pub signature_gauntlet: Option<String>,
    /// When `true`, ordinary player hits (slash / projectile) never damage
    /// this boss — they only give honest local impact feedback when they
    /// overlap the body hurtbox. The only kill condition is an environmental
    /// rule authored elsewhere (e.g. the Smirking Behemoth's LDtk-authored
    /// rope/anvil trap in `crate::ambition_content::bosses::cut_rope`).
    ///
    /// Data-driven by design: core asks the boss's *data* whether it is
    /// invulnerable to ordinary hits, instead of naming a specific boss, so the
    /// dependency points content -> core, never the reverse.
    #[serde(default)]
    pub environmental_kill_only: bool,
    /// Where a combat-banter speech bubble anchors relative to the
    /// boss: `pos + (dx_px, dy_half_h * combat_half_height + dy_px)`.
    /// The default hangs the bubble just above the body; multi-part
    /// bosses (GNU-ton's shoulder scholar) author their own.
    #[serde(default)]
    pub bark_anchor: BarkAnchorSpec,
    /// Authored strike-geometry OVERRIDES, keyed by the attack's `move_id`
    /// (`"floor_slam"`, `"hand_sweep"`, …, or a `Special` key). When a profile's
    /// move_id is present here, its [`StrikeRect`] list REPLACES the built-in
    /// [`strike_geometry`](super::attack_geometry::strike_geometry) table for that
    /// strike — so a boss (a second game's especially) authors its OWN strike rects
    /// in `boss_profiles.ron` with NO edit to core's geometry table. Empty (the
    /// `#[serde(default)]`) = use the built-in per-profile geometry, unchanged. The
    /// "second game adds a boss without editing core" oracle, for strike shapes.
    #[serde(default)]
    pub strike_geometry: std::collections::HashMap<String, Vec<super::attack_geometry::StrikeRect>>,
    /// ADR 0020: mount classes a boss authored as a would-be RIDER may pilot. A
    /// boss that rides a mount (GNU-ton the scholar aboard the `giant_gnu` mount)
    /// authors e.g. `["giant"]`; `spawn_boss` then attaches a [`CanPilot`] tag —
    /// the SAME mount-role the enemy path attaches in `attach_mount_role`, so the
    /// boss and enemy spawn paths stay symmetric. Empty (the default) ⇒ the boss
    /// pilots nothing (every boss today). The `RidingOn`/`MountSlot` link itself is
    /// installed later from the room's authored `mounted_on` refs.
    ///
    /// [`CanPilot`]: crate::features::CanPilot
    #[serde(default)]
    pub pilotable_mount_classes: Vec<String>,
    /// Q18 (G3): the profile→limb routing seam. Keyed by the strike's move id
    /// (`"hand_slam"`, `"hand_sweep"`, …), each entry names which of the mount's
    /// limb slots a strike drives and how ([`LimbRoute`]). When this boss is a
    /// RIDER whose linked mount carries a [`crate::features::LimbRig`],
    /// `route_boss_strikes_to_limbs` turns the ACTIVE strike's route into per-limb
    /// `velocity_target` arcs + a `melee_pressed` edge, written onto the mount's
    /// [`crate::features::LimbIntents`]. A strike move id NOT present here stays a
    /// host-body strike (no limb intent) — exactly as today. Empty (the
    /// `#[serde(default)]`) ⇒ no strike drives limbs (every boss but the gnu-ton
    /// rider). Authored in `boss_profiles.ron`, so a second game's mounted boss
    /// wires its own limbs with no edit to core.
    #[serde(default)]
    pub limb_routing: Vec<(String, LimbRoute)>,
    /// G5 (R10.6): the POSSESSED-VERB map — controller verb → this boss's move
    /// key, consulted when a human possesses the boss (`Brain::Player`). The
    /// possession arm reduces the controller's aim to a directional attack verb
    /// through the SAME chain every actor melee resolves
    /// ([`ambition_entity_catalog::directional_verb_chain`]: `attack_down` →
    /// `attack`), then looks the winning verb up HERE; `"special"` maps the
    /// special button. Combined with [`Self::limb_routing`], this is the
    /// controller→limb map: possess GNU-ton, aim down + attack → `hand_slam` →
    /// both giant hands slam. Empty (the `#[serde(default)]`) ⇒ the legacy
    /// possession mapping (primary strike / signature special) — every boss
    /// today except the gnu-ton rider. Authored in `boss_profiles.ron`; verbs
    /// are data, so a second game's possessable boss maps its own controls with
    /// no edit to core.
    #[serde(default)]
    pub possessed_verbs: Vec<(String, String)>,
}

/// Q18 (G3): one motion verb the limb router turns into a `velocity_target` arc
/// across a strike's Startup/Active phases. A tiny closed set — anything richer
/// is authored later as per-limb `MoveSpec`s. Data-driven on the boss profile;
/// the router (`route_boss_strikes_to_limbs`) owns the arc math.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
pub enum LimbMotion {
    /// Lift the limb toward `-gravity` (wind up / hold high).
    Raise,
    /// Sweep the limb laterally along the host's facing.
    SweepAcross,
    /// Arc the limb down along `+gravity` to a strike depth (an overhead slam).
    SlamDown,
    /// Station-keep — the limb holds its home pose while others strike.
    Hold,
}

/// Q18 (G3): a strike's limb ROUTE — which of the mount's limb slots it drives
/// (`"hand_left"` / `"hand_right"`), and the [`LimbMotion`] each performs. Keyed
/// by move id inside [`BossBehaviorProfile::limb_routing`]. Authored in RON:
/// `("hand_slam", (slots: ["hand_left", "hand_right"], motion: SlamDown))`.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct LimbRoute {
    /// Slot names the strike drives. `"hand_left"` / `"hand_right"` map to
    /// [`crate::features::LimbSlot`] via `LimbSlot::from_route_str`; unknown names
    /// are ignored (a route to a slot the rig doesn't carry is simply inert).
    pub slots: Vec<String>,
    /// How each named slot moves during the strike.
    pub motion: LimbMotion,
}

/// Authored speech-bubble anchor for a boss (see
/// [`BossBehaviorProfile::bark_anchor`]).
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
pub struct BarkAnchorSpec {
    pub dx_px: f32,
    pub dy_half_h: f32,
    pub dy_px: f32,
}

impl Default for BarkAnchorSpec {
    fn default() -> Self {
        Self {
            dx_px: 0.0,
            dy_half_h: -1.0,
            dy_px: -20.0,
        }
    }
}

/// Authored post-defeat reward for a boss. Parsed from the optional
/// `reward:` field of each row in `assets/data/boss_profiles.ron`
/// (defaults to `None` when the field is absent). The drop-chest
/// geometry (`offset`, `size`) is in world pixels; `pickup` names the
/// `PickupKind` granted on open.
///
/// Lives in the RON alongside the rest of the boss's behavior tuning, so
/// adding/retuning a reward is a content edit, not a code change.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub enum BossRewardProfile {
    None,
    DropChest {
        pickup: ambition_interaction::PickupKind,
        #[serde(with = "boss_vec2_required")]
        offset: ae::Vec2,
        #[serde(with = "boss_vec2_required")]
        size: ae::Vec2,
    },
}

impl Default for BossRewardProfile {
    fn default() -> Self {
        Self::None
    }
}

/// Vec2 (de)serialization shims for `BossBehaviorProfile`. `bevy_math::Vec2`
/// doesn't implement `Deserialize` under the features the sandbox compiles
/// with, so we route through tuple shims.
mod boss_vec2_option {
    use ambition_engine_core as ae;
    use serde::Deserialize;

    pub fn deserialize<'de, D>(de: D) -> Result<Option<ae::Vec2>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw: Option<(f32, f32)> = Option::deserialize(de)?;
        Ok(raw.map(|(x, y)| ae::Vec2::new(x, y)))
    }
}

mod boss_vec2_required {
    use ambition_engine_core as ae;
    use serde::Deserialize;

    pub fn deserialize<'de, D>(de: D) -> Result<ae::Vec2, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (x, y) = <(f32, f32)>::deserialize(de)?;
        Ok(ae::Vec2::new(x, y))
    }
}

/// Parsed boss-behavior rows used by content tooling and focused tests. Runtime
/// authority lives in the App-local [`super::BossCatalog`].
#[derive(Clone, Debug, Default)]
pub struct BossProfileRegistry {
    by_id: std::collections::HashMap<String, BossBehaviorProfile>,
}

impl BossProfileRegistry {
    /// Parse a boss-profile RON document (`HashMap<id, BossBehaviorProfile>`).
    pub fn from_ron(ron: &str) -> Self {
        let by_id = ron::from_str(ron).unwrap_or_else(|err| {
            panic!("boss_profiles.ron failed to deserialize as HashMap<String, BossBehaviorProfile>: {err}")
        });
        Self { by_id }
    }

    pub fn get(&self, id: &str) -> Option<&BossBehaviorProfile> {
        self.by_id.get(id)
    }
}

impl BossBehaviorProfile {
    /// Look up a boss profile by canonical id, cloning the parsed row from the
    /// App-local boss catalog. Panics if the id isn't present — call sites that
    /// need a fallback should route through `for_authored_boss` instead.
    pub fn from_data(catalog: &super::BossCatalog, id: &str) -> Self {
        catalog
            .behavior(id)
            .cloned()
            .unwrap_or_else(|| panic!("boss profile '{id}' not in boss_profiles.ron"))
    }

    /// Fallback profile for authored bosses whose canonical id isn't
    /// in `boss_profiles.ron`. Clones the shipped default boss's tuning
    /// (`clockwork_warden`, the polished multi-phase reference) and
    /// overrides the id so the encounter pipeline doesn't fault when an
    /// unknown boss spawns.
    pub fn generic(catalog: &super::BossCatalog, id: impl Into<String>) -> Self {
        let mut profile = catalog
            .fallback_behavior()
            .cloned()
            .unwrap_or_else(|| {
                panic!(
                    "boss catalog has no unambiguous fallback behavior; select a provider default through active-session authority"
                )
            });
        profile.id = id.into();
        // A generic boss draws from ITS OWN id's sheet, not the warden's
        // `"boss"` sheet — reset the cloned sprite target to identity.
        profile.sprite_target = None;
        profile
    }

    /// Resolve a boss profile from an authored display name or
    /// canonical id. Matches the encounter-id slug against the
    /// known bosses in `boss_profiles.ron`; falls back to a generic
    /// clone if the slug isn't a registered boss.
    ///
    /// **The fallback is LOUD.** It used to be silent, and that is how
    /// `sandbox.ldtk`'s `basement_boss` shipped for months carrying
    /// `PhaseScript:tri_slam_sweep_halo` — a pattern name, not a boss id. The
    /// slug matched nothing, `generic(slug)` cloned the warden's tuning under a
    /// bogus id, and the render then looked up `boss_sprites["tri_slam_sweep_halo"]`,
    /// missed, and drew the provider-selected fallback body. Nothing anywhere said a
    /// word. A boss that is generic BY ACCIDENT looks exactly like a boss that is
    /// generic by design, which is why this warns.
    pub fn for_authored_boss(catalog: &super::BossCatalog, id_or_name: &str) -> Self {
        let key = crate::boss_encounter::encounter_id_from_name(id_or_name);
        if key == "gradient_sentinel" {
            return Self::from_data(catalog, "clockwork_warden");
        }
        match catalog.behavior(&key) {
            Some(profile) => profile.clone(),
            None => {
                warn_once_unregistered_boss(&key);
                Self::generic(catalog, key)
            }
        }
    }
}

/// Warn once per unknown slug. A boss placement resolves every time its room
/// loads, so an unconditional warning would drown the log on a room the player
/// re-enters.
fn warn_once_unregistered_boss(key: &str) {
    use std::collections::BTreeSet;
    use std::sync::{LazyLock, Mutex};
    static WARNED: LazyLock<Mutex<BTreeSet<String>>> =
        LazyLock::new(|| Mutex::new(BTreeSet::new()));
    let fresh = WARNED
        .lock()
        .map(|mut seen| seen.insert(key.to_string()))
        .unwrap_or(false);
    if fresh {
        bevy::log::warn!(
            target: "ambition::bosses",
            "boss '{key}' is not in boss_profiles.ron — spawning a GENERIC clone of \
             the clockwork warden under that id. It will draw the generic body no \
             matter how its sheet is wired, because `boss_sprites[\"{key}\"]` cannot \
             exist. Fix the placement's `brain: PhaseScript:<id>` to name a real \
             profile, or add the profile.",
        );
    }
}

/// Named-boss test conveniences: thin `from_data` aliases so a test can read
/// `BossBehaviorProfile::mockingbird()` instead of the stringly id. The engine
/// ships NO named bosses — production resolves every boss by id through
/// `from_data` / `for_authored_boss`, and the DATA lives in
/// `boss_profiles.ron`. These aliases exist only for the boss test suites.
#[cfg(test)]
impl BossBehaviorProfile {
    /// Clockwork Warden / Gradient Sentinel — the polished multi-phase
    /// Scripted reference boss (design notes:
    /// `dev/journals/gradient-sentinel-boss-design-2026-05-25.md`).
    pub fn clockwork_warden() -> Self {
        Self::from_data(super::catalog::test_boss_catalog(), "clockwork_warden")
    }

    /// Mockingbird — airborne ship/bird-like Cycle boss.
    pub fn mockingbird() -> Self {
        Self::from_data(super::catalog::test_boss_catalog(), "mockingbird")
    }

    /// GNU-ton's scholar RIDER — the boss half of the ADR-0020 linked pair. He
    /// rides the `giant_gnu` mount and his strikes drive its hand limbs. This
    /// replaced the fused single `gnu_ton` profile in the E6 teardown; the
    /// giant's body geometry now lives on the MOUNT's sheet, not the boss's.
    pub fn gnu_ton_rider() -> Self {
        Self::from_data(super::catalog::test_boss_catalog(), "gnu_ton_rider")
    }
}

/// Resolve a boss's *canonical encounter id* from its authored
/// LDtk name + parsed brain payload.
///
/// The room author may set the display name to something flavorful
/// like "System Boss" while the brain points at the canonical
/// boss kind via `PhaseScript:clockwork_warden`. Without this
/// helper the encounter pipeline derives the id from the display
/// name only — `encounter_id_from_name("System Boss")` =
/// `"system_boss"` — and falls back to a generic boss profile
/// (empty music tracks, default behavior). Use this helper any
/// time you need the boss kind for behavior / profile / music
/// lookup; prefer `boss.behavior.id` when you already have a live
/// `BossRuntime`.
///
/// Resolution order:
/// 1. `BossBrain::PhaseScript { script_id }` with non-empty
///    `script_id` — the brain explicitly names the boss kind.
/// 2. `BossBrain::Custom(label)` with a non-empty label — same
///    intent, weaker contract.
/// 3. `encounter_id_from_name(authored_name)` — legacy fallback.
pub fn canonical_boss_id_from(
    name: &str,
    brain: &ambition_entity_catalog::placements::BossBrain,
) -> String {
    match brain {
        ambition_entity_catalog::placements::BossBrain::PhaseScript { script_id }
            if !script_id.is_empty() =>
        {
            script_id.clone()
        }
        ambition_entity_catalog::placements::BossBrain::Custom(label) if !label.is_empty() => {
            crate::boss_encounter::encounter_id_from_name(label)
        }
        _ => crate::boss_encounter::encounter_id_from_name(name),
    }
}

/// Live boss state owned by the simulation: body, HP, alive flag,
/// encounter-phase mirror, and a few cosmetic-timer scalars.
/// **Attack policy and attack execution state live elsewhere:** the
/// brain layer's `BossPatternState` owns the cursor / clocks and the
/// `BossAttackState` component owns the live telegraph/active
/// profile. `BossRuntime` carries body fields only.
/// Snapshot of the sprite generator's `body_metrics` for a boss,
/// captured once at sprite-registry lookup time so per-tick
/// damage/hurtbox math doesn't re-query the SheetRegistry resource.
///
/// `body_pixel_bbox` is the single overall body bbox (legacy /
/// single-piece bosses). `body_pixel_parts` is the multi-rect
/// representation for disjointed-piece bosses (head + body + arms).
/// Either one or both may be populated; the consumer picks parts
/// when present and falls back to bbox otherwise.
///
/// `frame_width` / `frame_height` are the sprite-frame dimensions
/// (e.g. 128×128 for clockwork_warden) used to scale pixel-space
/// coordinates into world-space via the boss's render size.
///
/// `sprite_render_size` is the world-space extent of the rendered
/// sprite quad — i.e. `BossSheetSpec::render_size(boss.size)`. The
/// hurtbox / hitbox math uses this (NOT `boss.size`) as the world
/// scale so the cyan / red / yellow boxes line up with the visible
/// sprite. Without this distinction, the boss spawns at LDtk size
/// (e.g. 128×160) but renders 1.6× bigger (~256×256), and the boxes
/// end up half the size of the visible body.
#[derive(Clone, Debug, Default)]
pub struct ActorSpriteMetrics {
    pub frame_width: u32,
    pub frame_height: u32,
    pub body_pixel_bbox: Option<ambition_sprite_sheet::PixelRect>,
    pub body_pixel_parts: Vec<ambition_sprite_sheet::NamedPixelRect>,
    /// World-space extent of the rendered sprite quad. Equal to
    /// `BossSheetSpec::render_size(boss.size)` at derivation time.
    /// Falls back to `(boss.size, boss.size)` when the sprite spec
    /// isn't known (test fixtures); consumers treat zero as
    /// "no render size yet, use ctx.size".
    pub sprite_render_size: ae::Vec2,
    /// World-space offset from `boss.pos` to the body's bounding
    /// AABB center. Captures the fact that the body bbox inside the
    /// sprite frame isn't necessarily at the frame center —
    /// the gradient sentinel's body sits a few pixels left of center
    /// and ~17 px above frame center, which scales to ~(-6, -35) in
    /// world space at 256×256 render. Without this offset,
    /// `boss.aabb()` is centered on `boss.pos` but the visible body
    /// is centered ~41 px above, so the pogo zone / orange debug
    /// box / body-contact zone all sit "below" the visible body
    /// and pogo doesn't register where the player aims.
    pub combat_offset: ae::Vec2,
    /// Per-animation `{hurtbox, hitbox}` data keyed by animation
    /// name (matches the spritesheet rows: `"rest"`,
    /// `"floor_slam"`, `"side_sweep"`, …). The renderer fills
    /// `hurtbox` from each animation's union alpha-bbox; the
    /// adapter declares `hitbox` rects for attack animations.
    /// Consumers (`damageable_volumes`, `volumes_for_profile`)
    /// look up by current animation name to scale hurtboxes /
    /// hitboxes with the on-screen sprite pose.
    pub animations: std::collections::HashMap<String, ambition_sprite_sheet::AnimationMetrics>,
}

impl ActorSpriteMetrics {
    /// True iff this snapshot carries at least one rectangle the
    /// derivation can use.
    pub fn has_body(&self) -> bool {
        !self.body_pixel_parts.is_empty() || self.body_pixel_bbox.is_some()
    }

    /// Per-animation hurtbox lookup. Used by `damageable_volumes`
    /// to size the hurtbox to the *currently-playing* animation
    /// (so attack frames with extended arms get a wider hurtbox
    /// than the rest pose). Returns `None` if the animation has
    /// no per-animation override; the caller falls back to
    /// `body_pixel_parts` / `body_pixel_bbox`.
    pub fn hurtbox_for_animation(
        &self,
        animation: &str,
    ) -> Option<&ambition_sprite_sheet::AnimationBox> {
        self.animations.get(animation)?.hurtbox.as_ref()
    }

    /// Per-animation hitbox lookup. Used by `volumes_for_profile`
    /// to read the sprite-author-declared damage geometry for an
    /// attack animation (so a side-sweep's hitbox covers both
    /// extended arms, not the generic bounding rect). Returns
    /// `None` if the animation has no authored hitbox; the
    /// caller falls back to its hardcoded volume math.
    pub fn hitbox_for_animation(
        &self,
        animation: &str,
    ) -> Option<&ambition_sprite_sheet::AnimationBox> {
        self.animations.get(animation)?.hitbox.as_ref()
    }
}

/// Ordered sprite-metadata keys that may describe a boss attack
/// profile's gameplay geometry. The first key is the canonical
/// runtime key; later keys are row-name aliases used by generated
/// sheets / visual review tools. Keeping the aliases here prevents
/// GNU-ton from silently falling back to rest/static boxes when the
/// generator names the visual row `head_down` but gameplay asks for
/// `HeadDescent`.
pub fn boss_animation_keys_for_profile(
    catalog: &super::BossCatalog,
    profile: &ambition_characters::brain::BossAttackProfile,
) -> Vec<String> {
    use ambition_characters::brain::BossAttackProfile;
    // Content specials carry their telegraph rows in the App-local boss catalog, so the engine names no
    // specific special here. Unregistered → no special row.
    if let BossAttackProfile::Special(key) = profile {
        return catalog
            .special_animation_keys(key)
            .iter()
            .cloned()
            .collect();
    }
    match profile.move_id().as_str() {
        "floor_slam" => vec!["floor_slam".into(), "mouth_open".into()],
        "side_sweep" => vec!["side_sweep".into()],
        "full_body_pulse" => vec!["spike_halo".into(), "eye_beam".into()],
        "hazard_column" => vec!["dash_echo".into(), "eye_beam".into()],
        // GNU-ton profiles use gameplay-specific canonical keys in
        // the runtime RON so one visual row can expose multiple
        // boxes (e.g. hand_slam vs shockwave). Accept the visual row
        // names too, so regenerated manifests and review images can
        // stay row-oriented without disconnecting the in-game boxes.
        "hand_slam" => vec!["gnu_hand_slam".into(), "hand_slam".into()],
        "converging_shockwave" => vec!["gnu_shockwave".into(), "hand_slam".into()],
        "hand_sweep" => vec!["gnu_hand_sweep".into(), "hand_sweep".into()],
        "head_descent" => vec!["gnu_head_descent".into(), "head_down".into()],
        // Remaining strikes (wing_sweep / dive_lane / broadside) belong to
        // the legacy aerial bosses that still rely on `volumes_for_profile`.
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod pilotable_mount_tests {
    use super::*;

    /// ADR 0020 field addition (fork #2): a boss authors NO
    /// `pilotable_mount_classes` unless it really rides something, so the serde
    /// default keeps them empty. The one boss that DOES ride is the GNU-ton
    /// rider, and it names exactly the mount class the `giant_gnu` archetype
    /// declares — a typo there would silently leave the scholar on foot.
    #[test]
    fn only_a_riding_boss_authors_pilotable_classes() {
        for profile in [
            BossBehaviorProfile::clockwork_warden(),
            BossBehaviorProfile::mockingbird(),
        ] {
            assert!(
                profile.pilotable_mount_classes.is_empty(),
                "{} pilots nothing by default",
                profile.id,
            );
        }
        assert_eq!(
            BossBehaviorProfile::gnu_ton_rider().pilotable_mount_classes,
            vec!["giant".to_string()],
            "the rider boards the giant-class mount — that is what makes the pair a pair",
        );
    }

    /// G5 field addition: `possessed_verbs` defaults empty (legacy possession
    /// mapping) for every profile that doesn't author it, and the gnu-ton
    /// rider's authored map is TYPO-GUARDED — every verb's move key must name a
    /// move in the profile's own `attacks` repertoire, or the verb could never
    /// fire (the trigger looks the move up by id in the boss's moveset).
    #[test]
    fn possessed_verbs_default_empty_and_authored_keys_name_real_attacks() {
        assert!(
            BossBehaviorProfile::clockwork_warden()
                .possessed_verbs
                .is_empty(),
            "unauthored profiles keep the legacy possession mapping",
        );

        let rider = BossBehaviorProfile::from_data(
            crate::boss_encounter::test_boss_catalog(),
            "gnu_ton_rider",
        );
        assert!(
            !rider.possessed_verbs.is_empty(),
            "the gnu-ton rider authors the G5 possessed-verb map",
        );
        let move_ids: Vec<String> = rider.attacks.iter().map(|p| p.move_id()).collect();
        for (verb, move_key) in &rider.possessed_verbs {
            assert!(
                move_ids.contains(move_key),
                "possessed verb '{verb}' names '{move_key}', which is not in the \
                 rider's authored attacks {move_ids:?} — the verb could never fire",
            );
        }
        // The two limb verbs land on routed strikes so possession drives the
        // giant's hands (the G5 payoff): both keys appear in limb_routing.
        for key in ["hand_slam", "hand_sweep"] {
            assert!(
                rider
                    .possessed_verbs
                    .iter()
                    .any(|(_, move_key)| move_key == key)
                    && rider.limb_routing.iter().any(|(k, _)| k == key),
                "'{key}' should be reachable by a possessed verb AND limb-routed",
            );
        }
    }
}
