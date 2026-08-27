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

//! Nothing here needed this crate; the coupling was locational. They are re-exported below, so
//! every existing path still resolves.
//!
//! What genuinely needs this crate stayed: the `BossCatalog` lookups (now
//! [`BossBehaviorProfileExt`], because the orphan rule does not let an inherent
//! `impl` follow a type across a crate boundary), [`ActorSpriteMetrics`], and
//! the animation-key table.

use ambition_platformer2d_core as ae;

pub use crate::pattern::profile::{
    BarkAnchorSpec, BossBehaviorProfile, BossProfileRegistry, BossRewardProfile, LimbMotion,
    LimbRoute, StrikeRect,
};

/// The `BossCatalog` lookups for a [`BossBehaviorProfile`].
///
/// an extension TRAIT, not inherent methods, and that is the orphan rule
/// speaking rather than a style choice. The profile type lives in
/// `ambition_characters` now; an inherent `impl` for it can only be written
/// there, and `BossCatalog` — which these need — lives HERE. The trait is the
/// only shape that lets the lookup stay next to the catalog it reads.
///
/// Call sites are unchanged (`BossBehaviorProfile::from_data(catalog, id)`);
/// they need this trait in scope.
pub trait BossBehaviorProfileExt {
    /// Look up a boss profile by canonical id, cloning the parsed row from the
    /// App-local boss catalog. Panics if the id isn't present — call sites that
    /// need a fallback should route through `for_authored_boss` instead.
    fn from_data(catalog: &super::BossCatalog, id: &str) -> Self;
    /// Fallback profile for authored bosses whose canonical id isn't in
    /// `boss_profiles.ron`. Clones the shipped default boss's tuning and
    /// overrides the id so the encounter pipeline doesn't fault.
    fn generic(catalog: &super::BossCatalog, id: impl Into<String>) -> Self;
    /// Resolve a boss profile from an authored display name or canonical id.
    ///
    /// The slug matched nothing, `generic(slug)` cloned the warden's tuning under a bogus id, and
    /// the render then looked up `boss_sprites["tri_slam_sweep_halo"]`, missed, and drew the
    /// fallback body. Nothing anywhere said a word. A boss that is generic BY ACCIDENT looks
    /// exactly like a boss that is generic by design.
    fn for_authored_boss(catalog: &super::BossCatalog, id_or_name: &str) -> Self;

    /// Clockwork Warden / Gradient Sentinel — the polished multi-phase Scripted
    /// reference boss. A thin `from_data` alias so a test reads the name instead
    /// of the stringly id; the engine ships NO named bosses, and production
    /// resolves every boss by id.
    #[cfg(any(test, feature = "test-support"))]
    fn clockwork_warden() -> Self;
    /// Mockingbird — airborne ship/bird-like Cycle boss.
    #[cfg(any(test, feature = "test-support"))]
    fn mockingbird() -> Self;
    /// GNU-ton's scholar RIDER — the boss half of the ADR-0020 linked pair.
    #[cfg(any(test, feature = "test-support"))]
    fn gnu_ton_rider() -> Self;
}

impl BossBehaviorProfileExt for BossBehaviorProfile {
    fn from_data(catalog: &super::BossCatalog, id: &str) -> Self {
        catalog
            .behavior(id)
            .cloned()
            .unwrap_or_else(|| panic!("boss profile '{id}' not in boss_profiles.ron"))
    }

    fn generic(catalog: &super::BossCatalog, id: impl Into<String>) -> Self {
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

    fn for_authored_boss(catalog: &super::BossCatalog, id_or_name: &str) -> Self {
        let key = crate::encounter_id_from_name(id_or_name);
        if key == "gradient_sentinel" {
            return <Self as BossBehaviorProfileExt>::from_data(catalog, "clockwork_warden");
        }
        match catalog.behavior(&key) {
            Some(profile) => profile.clone(),
            None => {
                warn_once_unregistered_boss(&key);
                <Self as BossBehaviorProfileExt>::generic(catalog, key)
            }
        }
    }

    #[cfg(any(test, feature = "test-support"))]
    fn clockwork_warden() -> Self {
        <Self as BossBehaviorProfileExt>::from_data(
            super::catalog::test_boss_catalog(),
            "clockwork_warden",
        )
    }

    #[cfg(any(test, feature = "test-support"))]
    fn mockingbird() -> Self {
        <Self as BossBehaviorProfileExt>::from_data(
            super::catalog::test_boss_catalog(),
            "mockingbird",
        )
    }

    #[cfg(any(test, feature = "test-support"))]
    fn gnu_ton_rider() -> Self {
        <Self as BossBehaviorProfileExt>::from_data(
            super::catalog::test_boss_catalog(),
            "gnu_ton_rider",
        )
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
            target: "ambition_boss_encounter::behavior",
            "boss '{key}' is not in boss_profiles.ron — spawning a GENERIC clone of \
             the clockwork warden under that id. It will draw the generic body no \
             matter how its sheet is wired, because `boss_sprites[\"{key}\"]` cannot \
             exist. Fix the placement's `brain: PhaseScript:<id>` to name a real \
             profile, or add the profile.",
        );
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
            crate::encounter_id_from_name(label)
        }
        _ => crate::encounter_id_from_name(name),
    }
}

/// Live boss state owned by the simulation: body, HP, alive flag,
/// encounter-phase mirror, and a few cosmetic-timer scalars.
/// Attack policy and attack execution state live elsewhere: the
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

        let rider = BossBehaviorProfile::from_data(crate::test_boss_catalog(), "gnu_ton_rider");
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
