//! Boss behavior-profile vocabulary (data-driven).
//!
//! `BossBehaviorProfile` / `BarkAnchorSpec` / `BossRewardProfile` /
//! `ActorSpriteMetrics` are the schemas every boss instance is authored INTO:
//! the named rows live in `boss_profiles.ron`, parsed by `BossProfileRegistry`
//! and installed via `install_boss_profiles`. Owns movement/attacks/damage/
//! hitbox tuning (the engine `BossEncounterSpec` owns phase progression + HP).
//! `BossBehaviorProfile::from_data("id")` clones an installed row; the named
//! constructors (`clockwork_warden()` etc.) are thin lookups. Also holds
//! `boss_animation_keys_for_profile` (attack-profile -> sprite-row keys) and
//! `canonical_boss_id_from` (resolves the boss kind from LDtk name + brain).

use crate::brain::boss_pattern::{BossAttackPattern, BossAttackProfile, BossMovementProfile};
use crate::engine_core as ae;

/// Live sandbox-side behavior tuning for a boss. This is deliberately separate
/// from `crate::boss_encounter::BossEncounterSpec`: the engine spec owns phase progression and HP
/// thresholds, while this profile owns sandbox movement, contact size, damage,
/// and hitbox shapes.
///
/// Every field here is authored in `ambition_content`'s `boss_profiles.ron`,
/// parsed into the installable [`BossProfileRegistry`] below (content installs
/// it at plugin-build time). Adding a new boss is a single new key + row in
/// that file when it needs custom behavior; unknown authored bosses fall back
/// to the generic profile.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct BossBehaviorProfile {
    pub id: String,
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
    /// Macro state machine tuning — when enabled, the boss runs an
    /// Engage / Approach / Retreat dance on top of the scripted
    /// attack schedule. See [`crate::brain::BossMacroTuning`].
    /// Use `BossMacroTuning::disabled()` for legacy "stand and
    /// fight" behavior.
    pub macro_tuning: crate::brain::BossMacroTuning,
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
    use crate::engine_core as ae;
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
    use crate::engine_core as ae;
    use serde::Deserialize;

    pub fn deserialize<'de, D>(de: D) -> Result<ae::Vec2, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (x, y) = <(f32, f32)>::deserialize(de)?;
        Ok(ae::Vec2::new(x, y))
    }
}

/// The installed boss-behavior registry (canonical id → profile). The named
/// boss DATA (`boss_profiles.ron`) is content, owned and installed by
/// `ambition_content` at plugin-build time; the lib owns only this generic
/// holder + the `BossBehaviorProfile` schema. Held as an installable global
/// (not a Bevy `Resource`) because `from_data` is called from many non-system
/// contexts (spawn sites, profile clones) — the same rationale as the enemy
/// `EnemyRoster`.
#[derive(Clone, Debug, Default)]
pub struct BossProfileRegistry {
    by_id: std::collections::HashMap<String, BossBehaviorProfile>,
}

impl BossProfileRegistry {
    /// Parse a boss-profile RON document (`HashMap<id, BossBehaviorProfile>`) —
    /// the content layer's install entry point.
    pub fn from_ron(ron: &str) -> Self {
        let by_id = ron::from_str(ron).unwrap_or_else(|err| {
            panic!("boss_profiles.ron failed to deserialize as HashMap<String, BossBehaviorProfile>: {err}")
        });
        Self { by_id }
    }

    fn get(&self, id: &str) -> Option<&BossBehaviorProfile> {
        self.by_id.get(id)
    }
}

/// Content-installed boss-profile registry. Set once at plugin-build time;
/// production resolution REQUIRES it (there is no production embedded default).
static BOSS_PROFILE_OVERRIDE: std::sync::OnceLock<BossProfileRegistry> = std::sync::OnceLock::new();

/// Install the authored boss-behavior registry — `ambition_content` calls this
/// at plugin-build time (before any boss spawn / profile clone runs).
pub fn install_boss_profiles(registry: BossProfileRegistry) {
    let _ = BOSS_PROFILE_OVERRIDE.set(registry);
}

/// Content-installed telegraph-animation hints per boss `Special(key)`. The
/// engine ships no anim row for content specials; content registers which
/// sprite rows telegraph each one. Visual only — an unregistered key simply
/// shows no special telegraph row, the strike still fires. This is what keeps
/// the engine from naming `overfit_volley`/`minima_trap`/etc.: the key→rows
/// mapping is content data, not a lib `match`.
static BOSS_SPECIAL_ANIM_KEYS: std::sync::OnceLock<
    std::collections::HashMap<String, &'static [&'static str]>,
> = std::sync::OnceLock::new();

/// Install per-special telegraph anim hints — `ambition_content` calls this at
/// plugin-build time alongside [`install_boss_profiles`].
pub fn install_boss_special_anim_keys(
    map: std::collections::HashMap<String, &'static [&'static str]>,
) {
    let _ = BOSS_SPECIAL_ANIM_KEYS.set(map);
}

/// Telegraph anim rows for a content special key (empty if unregistered).
fn special_anim_keys(key: &str) -> &'static [&'static str] {
    BOSS_SPECIAL_ANIM_KEYS
        .get()
        .and_then(|m| m.get(key))
        .copied()
        .unwrap_or(&[])
}

/// Test fixture: the lib's own unit tests read content's authoritative
/// `boss_profiles.ron` at compile time (cfg(test) only — production embeds no
/// boss data and requires the content install).
#[cfg(test)]
static BOSS_PROFILE_FIXTURE: std::sync::LazyLock<BossProfileRegistry> =
    std::sync::LazyLock::new(|| {
        BossProfileRegistry::from_ron(include_str!(
            "../../../ambition_content/assets/data/boss_profiles.ron"
        ))
    });

#[cfg(test)]
fn boss_profiles() -> &'static BossProfileRegistry {
    BOSS_PROFILE_OVERRIDE.get().unwrap_or(&BOSS_PROFILE_FIXTURE)
}

#[cfg(not(test))]
fn boss_profiles() -> &'static BossProfileRegistry {
    BOSS_PROFILE_OVERRIDE.get().unwrap_or_else(|| {
        panic!(
            "boss profiles not installed — AmbitionContentPlugin must call \
             install_boss_profiles() at build time before any boss spawns"
        )
    })
}

impl BossBehaviorProfile {
    /// Look up a boss profile by canonical id, cloning the parsed row from the
    /// installed registry. Panics if the id isn't present — call sites that
    /// need a fallback should route through `for_authored_boss` instead.
    pub fn from_data(id: &str) -> Self {
        boss_profiles()
            .get(id)
            .cloned()
            .unwrap_or_else(|| panic!("boss profile '{id}' not in boss_profiles.ron"))
    }

    /// Clockwork Warden / Gradient Sentinel — polished multi-phase
    /// Scripted boss. Authored in `boss_profiles.ron`; design notes
    /// live at `dev/journals/gradient-sentinel-boss-design-2026-05-25.md`.
    pub fn clockwork_warden() -> Self {
        Self::from_data("clockwork_warden")
    }

    /// Mockingbird — airborne ship/bird-like Cycle boss.
    /// Authored in `boss_profiles.ron`.
    pub fn mockingbird() -> Self {
        Self::from_data("mockingbird")
    }

    /// GNU-ton — stationary giant with wide-ranging hand attacks.
    /// Authored in `boss_profiles.ron`.
    pub fn gnu_ton() -> Self {
        Self::from_data("gnu_ton")
    }

    /// Smirking Behemoth — cut-rope environmental boss.
    /// Authored in `boss_profiles.ron`.
    pub fn smirking_behemoth_boss() -> Self {
        Self::from_data("smirking_behemoth_boss")
    }

    /// Flying Spaghetti Monster — false-god boss (canon: Jon).
    /// Authored in `boss_profiles.ron`.
    pub fn flying_spaghetti_monster_boss() -> Self {
        Self::from_data("flying_spaghetti_monster_boss")
    }

    /// T-rex — grounded, melee-centric bipedal boss (canon: Jon).
    /// Authored in `boss_profiles.ron`.
    pub fn trex_boss() -> Self {
        Self::from_data("trex_boss")
    }

    /// Mode Collapse — a floating summoner boss. Authored in `boss_profiles.ron`.
    pub fn mode_collapse_boss() -> Self {
        Self::from_data("mode_collapse_boss")
    }

    /// Exploding Gradient — a floating ranged-pressure boss. Authored in
    /// `boss_profiles.ron`.
    pub fn exploding_gradient_boss() -> Self {
        Self::from_data("exploding_gradient_boss")
    }

    /// Overflow — an aerial melee dive-bomber. Authored in `boss_profiles.ron`.
    pub fn overflow_boss() -> Self {
        Self::from_data("overflow_boss")
    }

    /// Fallback profile for authored bosses whose canonical id isn't
    /// in `boss_profiles.ron`. Clones the Clockwork Warden's tuning
    /// and overrides the id so the encounter pipeline doesn't fault
    /// when an unknown boss spawns.
    pub fn generic(id: impl Into<String>) -> Self {
        let mut profile = Self::clockwork_warden();
        profile.id = id.into();
        profile
    }

    /// Resolve a boss profile from an authored display name or
    /// canonical id. Matches the encounter-id slug against the
    /// known bosses in `boss_profiles.ron`; falls back to a generic
    /// clone if the slug isn't a registered boss.
    pub fn for_authored_boss(id_or_name: &str) -> Self {
        let key = crate::boss_encounter::encounter_id_from_name(id_or_name);
        if key == "gradient_sentinel" {
            return Self::clockwork_warden();
        }
        boss_profiles()
            .get(&key)
            .cloned()
            .unwrap_or_else(|| Self::generic(key))
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
pub fn canonical_boss_id_from(name: &str, brain: &crate::actor::BossBrain) -> String {
    match brain {
        crate::actor::BossBrain::PhaseScript { script_id } if !script_id.is_empty() => {
            script_id.clone()
        }
        crate::actor::BossBrain::Custom(label) if !label.is_empty() => {
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
    profile: &crate::brain::BossAttackProfile,
) -> &'static [&'static str] {
    use crate::brain::BossAttackProfile;
    match profile {
        BossAttackProfile::FloorSlam => &["floor_slam", "mouth_open"],
        BossAttackProfile::SideSweep => &["side_sweep"],
        BossAttackProfile::FullBodyPulse => &["spike_halo", "eye_beam"],
        BossAttackProfile::HazardColumn => &["dash_echo", "eye_beam"],
        // Content specials carry their telegraph rows as installed data
        // (see `install_boss_special_anim_keys`), so the engine names no
        // specific special here. Unregistered → no special row.
        BossAttackProfile::Special(key) => special_anim_keys(key),
        // GNU-ton profiles use gameplay-specific canonical keys in
        // the runtime RON so one visual row can expose multiple
        // boxes (e.g. hand_slam vs shockwave). Accept the visual row
        // names too, so regenerated manifests and review images can
        // stay row-oriented without disconnecting the in-game boxes.
        BossAttackProfile::HandSlam => &["gnu_hand_slam", "hand_slam"],
        BossAttackProfile::ConvergingShockwave => &["gnu_shockwave", "hand_slam"],
        BossAttackProfile::HandSweep => &["gnu_hand_sweep", "hand_sweep"],
        BossAttackProfile::HeadDescent => &["gnu_head_descent", "head_down"],
        // Remaining profiles (WingSweep / DiveLane / Broadside) belong to
        // the legacy aerial bosses that still rely on
        // `volumes_for_profile`'s fallback math.
        _ => &[],
    }
}
