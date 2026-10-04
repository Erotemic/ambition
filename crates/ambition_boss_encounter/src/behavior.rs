//! Boss behavior-profile vocabulary (data-driven).
//!
//! `BossBehaviorProfile` / `BarkAnchorSpec` / `BossRewardProfile` /
//! `ActorSpriteMetrics` are the schemas every boss is authored into. The named
//! rows live in provider `boss_profiles.ron` fragments assembled in the
//! App-local [`super::BossCatalog`]. They own movement/attacks/damage/hitbox
//! tuning (the engine `BossEncounterSpec` owns phase progression and HP).
//! `BossBehaviorProfile::from_data(catalog, "id")` clones an App-local row;
//! the named constructors (`clockwork_warden()` etc.) are thin lookups. This
//! module also has `boss_animation_keys_for_profile` (attack profile →
//! sprite-row keys) and `canonical_boss_id_from` (the boss kind from LDtk
//! name and brain).

//! The profile types live in `ambition_characters` and are re-exported below.
//! What needs this crate stays here: the `BossCatalog` lookups
//! ([`BossBehaviorProfileExt`]), [`ActorSpriteMetrics`], and the animation-key
//! table.

pub use crate::pattern::profile::{
    BarkAnchorSpec, BossBehaviorProfile, BossProfileRegistry, BossRewardProfile, LimbMotion,
    LimbRoute, StrikeRect,
};

/// The `BossCatalog` lookups for a [`BossBehaviorProfile`].
///
/// An extension trait, not inherent methods, because of the orphan rule: the
/// profile type lives in `ambition_characters`, and `BossCatalog`, which these
/// lookups read, lives here.
///
/// Call sites use `BossBehaviorProfile::from_data(catalog, id)` with this
/// trait in scope.
pub trait BossBehaviorProfileExt {
    /// Look up a boss profile by canonical id, cloning the parsed row from the
    /// App-local boss catalog. Panics if the id is not present; call sites
    /// that need a fallback use `for_authored_boss`.
    fn from_data(catalog: &super::BossCatalog, id: &str) -> Self;
    /// Fallback profile for authored bosses whose canonical id is not in
    /// `boss_profiles.ron`. Clones the default boss's tuning and sets the id,
    /// so the encounter pipeline does not fail.
    fn generic(catalog: &super::BossCatalog, id: impl Into<String>) -> Self;
    /// Resolve a boss profile from an authored display name or canonical id.
    ///
    /// An unknown slug falls back to `generic(slug)` and draws the fallback
    /// body, which looks the same as a boss that is generic by design. So an
    /// unknown slug is logged (see `warn_once_unregistered_boss`).
    fn for_authored_boss(catalog: &super::BossCatalog, id_or_name: &str) -> Self;

    /// Clockwork Warden / Gradient Sentinel: the multi-phase Scripted
    /// reference boss. A thin `from_data` alias, so a test reads a name
    /// instead of an id. The engine ships no named bosses; production
    /// resolves every boss by id.
    #[cfg(any(test, feature = "test-support"))]
    fn clockwork_warden() -> Self;
    /// Mockingbird — airborne ship/bird-like Cycle boss.
    #[cfg(any(test, feature = "test-support"))]
    fn mockingbird() -> Self;
    /// GNU-ton's scholar rider — the boss half of the ADR-0020 linked pair.
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
        // A generic boss draws from its own id's sheet, not the warden's
        // `"boss"` sheet — reset the cloned sprite target to identity.
        profile.sprite_target = None;
        profile
    }

    fn for_authored_boss(catalog: &super::BossCatalog, id_or_name: &str) -> Self {
        let key = crate::encounter_id_from_name(id_or_name);
        // A retired id takes its successor's behavior before the catalog is
        // consulted, because the catalog no longer has the old key. The pair
        // is in `ids::renamed_encounter_id`.
        if let Some(current) = crate::renamed_encounter_id(&key) {
            return <Self as BossBehaviorProfileExt>::from_data(catalog, current);
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
            target: "crate::behavior",
            "boss '{key}' is not in boss_profiles.ron — spawning a GENERIC clone of \
             the clockwork warden under that id. It will draw the generic body no \
             matter how its sheet is wired, because `boss_sprites[\"{key}\"]` cannot \
             exist. Fix the placement's `brain: PhaseScript:<id>` to name a real \
             profile, or add the profile.",
        );
    }
}

/// Resolve a boss's canonical encounter id from its authored LDtk name and
/// parsed brain payload.
///
/// A room author may give a flavorful display name such as "System Boss"
/// while the brain names the boss kind via `PhaseScript:clockwork_warden`.
/// Deriving the id from the display name alone
/// (`encounter_id_from_name("System Boss")` = `"system_boss"`) would fall back
/// to a generic profile (no music, default behavior). Use this whenever you
/// need the boss kind for behavior, profile or music lookup; use
/// `boss.behavior.id` when you already have a live boss.
///
/// Resolution order:
/// 1. `BossBrain::PhaseScript { script_id }` with non-empty `script_id`: the
///    brain names the boss kind.
/// 2. `BossBrain::Custom(label)` with a non-empty label: same intent, weaker
///    contract.
/// 3. `encounter_id_from_name(authored_name)`: fallback.
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

/// The behaviour that an authored boss placement gets.
///
/// This is the one resolution from a placement's display name and brain to
/// its behaviour: the canonical id ([`canonical_boss_id_from`]), then the
/// catalog, with a renamed id and the generic profile as that lookup decides.
/// The id of the result is the id that the boss reports when it is defeated
/// (`QuestAdvanceEvent::BossDefeated`), so a reader that must know that id
/// (a quest validator) asks here. It must not slug the display name.
pub fn authored_boss_behavior(
    catalog: &super::BossCatalog,
    name: &str,
    brain: &ambition_entity_catalog::placements::BossBrain,
) -> BossBehaviorProfile {
    <BossBehaviorProfile as BossBehaviorProfileExt>::for_authored_boss(
        catalog,
        &canonical_boss_id_from(name, brain),
    )
}


/// Ordered sprite-metadata keys that may describe a boss attack profile's
/// gameplay geometry. The first key is the canonical runtime key; later keys
/// are row-name aliases used by generated sheets and visual review tools.
///
/// The rows are authored data in the App-local boss catalog, for a geometry
/// strike and for a content special, so this function names no sheet row. A move
/// that no provider gives rows to has none: a strike keeps its static boxes,
/// and a special telegraphs nothing.
pub fn boss_animation_keys_for_profile(
    catalog: &super::BossCatalog,
    profile: &ambition_characters::brain::BossAttackProfile,
) -> Vec<String> {
    use ambition_characters::brain::BossAttackProfile;
    match profile {
        BossAttackProfile::Special(key) => catalog.special_animation_keys(key),
        BossAttackProfile::Strike(key) => catalog.strike_animation_keys(key),
    }
    .to_vec()
}

#[cfg(test)]
mod pilotable_mount_tests {
    use super::*;

    /// A boss authors no `pilotable_mount_classes` unless it rides something,
    /// so the serde default is empty. The GNU-ton rider names exactly the
    /// mount class the `giant_gnu` archetype declares; a typo would leave the
    /// scholar on foot.
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

    /// `possessed_verbs` defaults to empty (the fallback possession mapping)
    /// for every profile that does not author it. The gnu-ton rider's map is
    /// typo-guarded: every verb's move key must name a move in the profile's
    /// own `attacks`, or the verb could never fire (the trigger looks the move
    /// up by id in the boss's moveset).
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
        // Possessing him conducts his fists: every verb names one of his
        // content-performed `Special`s.
        for (verb, move_key) in &rider.possessed_verbs {
            assert!(
                rider
                    .attacks
                    .iter()
                    .any(|attack| attack.is_special() && &attack.move_id() == move_key),
                "possessed verb '{verb}' should command a conducted Special, got '{move_key}'",
            );
        }
    }
}
