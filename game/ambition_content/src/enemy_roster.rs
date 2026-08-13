//! THE Ambition hostile-archetype roster — named, authored game data.
//!
//! The machinery crate (`ambition_platformer2d_actor_monolith`) owns the generic schema and spawn
//! pipeline. This module contributes Ambition's immutable fragment to the
//! current Bevy [`App`](bevy::prelude::App); no process-global install order is
//! involved.

use ambition_platformer2d_actor_monolith::features::{
    CharacterRosterAppExt, CharacterRosterFragment,
};
use bevy::prelude::App;

/// Provider identity used by every Ambition-authored catalog fragment.
pub const PROVIDER_ID: &str = "ambition";

/// The authored hostile roster, embedded at compile time. Top-level keys are
/// the spawn brain keys a `LoadingZone` / encounter authors as
/// `Brain::Custom("…")`; `"combatant"` is Ambition's fallback row.
pub const CHARACTER_ROSTER_RON: &str = include_str!("../assets/data/character_archetypes.ron");

/// **AMBITION'S OWN UNRESOLVED CASTING, DECLARED WHERE IT IS OWNED** —
/// `(identifier, row it borrows meanwhile, why)`.
///
/// ⛔⛔ these three identifiers are authored in shipped content — a boss summon,
/// an encounter wave, a mite's split — and name no character and no row, so
/// construction would refuse them. Until 2026-08-12 the ENGINE carried them as
/// `IDENTIFIERS_AWAITING_A_CASTING_DECISION`, a const list of three Ambition
/// creature names compiled into a reusable platformer: any other game linking it
/// inherited the waiver, and a typo matching one of the three entered the
/// temporary road silently (GPT 5.6 review, priority 3).
///
/// ⚠ **each line is a content decision Jon has not made yet, not an engineering
/// gap.** Deleting one is what closes it; deleting all three is D102's
/// acceptance signal. What they ultimately ARE is not decided here — `combatant`
/// is what they BORROW meanwhile, and the warning says so on every spawn.
///
/// ⚠ **a const rather than three inline calls** so the buildability guard
/// (`worlds::tests::every_shipped_enemy_placement_can_be_built`) asks the same
/// list construction was given, instead of a second copy that could disagree.
pub(crate) const OPEN_CASTING: &[(&str, &str, &str)] = &[
    (
        "small_lurker",
        "combatant",
        "the Gradient Sentinel's gradient-cascade summon (ledger D93/D96). A \
         minion of the wrong body beats a boss that casts nothing mid-fight.",
    ),
    (
        "large_brute",
        "combatant",
        "the goblin encounter's heavy, three waves of it (ledger D93). Which \
         creature the heavy IS is content.",
    ),
    (
        "SmallSkitter",
        "combatant",
        "what a `DividingMite` splits into (`damage_drops`, ledger D96), and the \
         last CamelCase archetype name left in shipped content. No skitter \
         character exists yet.",
    ),
];

/// Register Ambition's hostile archetypes into this Bevy App.
///
/// ⛔ **THROUGH THE COMPILER, not beside it.** This used to call
/// `CharacterRosterFragment::from_ron(…, CHARACTER_ROSTER_RON)`, re-parsing bytes
/// the pack had already read and judged — the two-readers split the content pack
/// exists to close.
///
/// The `character_archetypes` schema additionally refuses what `from_ron`
/// accepted: an `inherits` naming a row this roster does not define (which used
/// to fall back to the baseline silently, reading as "my inheritance did
/// nothing"), a self-inheriting row, and a `patrol_effort` / `chase_effort`
/// outside `0.0..=1.0` — effort is a FRACTION of `run_speed` (§4.7) and the seam
/// clamps it, so an out-of-range value reads as tuned and behaves identically.
pub fn register(app: &mut App) {
    let by_brain =
        ambition_combat::content_schema::lowered_character_archetypes(crate::pack::prepared())
            .expect("the archetype schema lowers its roster for every pack that compiles")
            .clone();
    let mut fragment =
        CharacterRosterFragment::from_prepared_specs(PROVIDER_ID, by_brain, CHARACTER_ROSTER_RON)
            .expect("Ambition character_archetypes.ron should be a valid roster fragment");
    for (identifier, temporary_row, reason) in OPEN_CASTING {
        fragment = fragment.with_open_casting_decision(*identifier, *temporary_row, *reason);
    }
    app.register_character_roster_fragment(fragment);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_roster_parses_and_registers() {
        let mut app = App::new();
        register(&mut app);
        assert!(app
            .world()
            .contains_resource::<ambition_platformer2d_actor_monolith::features::CharacterRoster>(
            ));
    }

    /// **A PRACTICE TARGET DOES NOT STRIKE BACK** — asked of the CHARACTERS that
    /// are practice targets, which is where they live.
    ///
    /// ⛔⛔ **this ran over the archetype ROSTER and had gone vacuous** (repaired
    /// 2026-08-13). It asserted `sandbags_are_passive()`, which reads
    /// `all(|spec| !spec.is_sandbag || spec.melee.is_none())` — and Ambition's
    /// shipped roster is down to `combatant` and `medium_striker`, neither of
    /// which sets `is_sandbag`. An `all` over zero matching rows is `true`, so
    /// the test passed by having nothing to check, which is the D94 shape: a
    /// green guard whose subject migrated out from under it.
    ///
    /// ⛔⛔ **AND MIGRATING THE OLD CLAIM LITERALLY WOULD HAVE BEEN A FALSE
    /// RULE.** The roster invariant was *"a sandbag row has `melee: None`"*, and
    /// asked of the characters it fails immediately: both sandbags name the
    /// `sandbag_punch` action set, which authors a real `PunchWeak`. That is
    /// deliberate content, not a defect — an archetype row fused kit and policy,
    /// so the only way to say "never strikes back" was to remove the fist. A
    /// character says it with its POLICY, and `sandbag.rs`'s own note records
    /// exactly that: *"Notices nobody and swings at nobody; the old row's
    /// `attack_range: 150.0` sat beside `melee: None`"*.
    ///
    /// ⇒ so this asserts the MECHANISM that actually holds — a practice target's
    /// autonomous profile notices nobody and reaches nobody — rather than the
    /// old rule's proxy. Asserting the proxy would have pushed content to strip
    /// a fist for a reason that was never the real one.
    ///
    /// ⚠ the count assertion is what stops THIS test going vacuous the same way:
    /// a cast with no practice targets means they moved again.
    #[test]
    fn practice_target_characters_do_not_strike_back() {
        let mut app = App::new();
        crate::character_catalog::register(&mut app);
        crate::player_robot_lineage::register_declared_cast(&mut app);
        ambition_platformer2d_shared_tangle::app_finalization::finalize(&mut app);
        let prepared = app
            .world()
            .resource::<ambition_platformer2d_actor_monolith::character_runtime::PreparedCharacterRegistry>();

        let mut targets = 0;
        for id in prepared.ids() {
            let character = prepared
                .get(id)
                .unwrap_or_else(|| panic!("`{id}` is in the registry's own id list"));
            if !character.practice_target {
                continue;
            }
            targets += 1;
            let policy = character
                .autonomous_profile
                .unwrap_or_else(|| panic!("`{id}` is a practice target that states no policy, so what it does when hit is whatever a default happens to say"));
            assert_eq!(
                (policy.aggro_radius, policy.attack_range),
                (0.0, 0.0),
                "`{id}` is authored as a practice target and its policy notices \
                 targets at {}px and reaches them at {}px — a dummy that \
                 counter-attacks is not a dummy. ⚠ its KIT is not the thing to \
                 fix: both sandbags carry `sandbag_punch` on purpose, and the \
                 policy is what keeps the fist unused",
                policy.aggro_radius,
                policy.attack_range
            );
        }
        assert!(
            targets >= 2,
            "this cast holds {targets} practice targets and Ambition ships two \
             (`sandbag`, `sandbag_infinite`), so the guard above checked nothing \
             — which is exactly how its roster-side ancestor went quietly vacuous"
        );
    }
}
