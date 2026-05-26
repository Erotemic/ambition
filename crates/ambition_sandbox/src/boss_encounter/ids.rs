/// Encounter id of the pirate-cove boss. The chest dropped on its
/// defeat reuses the standard `encounter_chest_<id>` naming so the
/// existing open / looted-flag plumbing
/// (`crate::encounter::encounter_reward_looted_flag`) handles
/// persistence with no special case.
///
/// GENERALIZATION PLAN: this id + the chest constants below are the
/// only mockingbird-specific knobs in `sync_mockingbird_treasure_chest`.
/// When a second boss needs an on-defeat drop, lift these into a small
/// data-driven table — e.g. a `BossDeathReward { encounter_id,
/// chest_size, drop_offset, reward: PickupKind }` map registered next
/// to `default_boss_specs()`. The sync function then iterates the
/// table instead of hard-coding `MOCKINGBIRD_ENCOUNTER_ID`. We're
/// intentionally NOT building the abstraction yet (one example isn't
/// a pattern), but the named-after-the-thing function is the smell
/// that points to the future refactor.
#[allow(
    dead_code,
    reason = "referenced by boss_encounter::tests + planned data-driven boss-reward table"
)]
pub const MOCKINGBIRD_ENCOUNTER_ID: &str = "mockingbird";

/// Sanitize an authored boss `name` into a stable encounter id. Lowercases,
/// strips non-alphanumeric characters, replaces spaces with underscores.
/// `"Clockwork Warden"` → `"clockwork_warden"`.
pub fn encounter_id_from_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut prev_was_underscore = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_was_underscore = false;
        } else if !prev_was_underscore && !out.is_empty() {
            out.push('_');
            prev_was_underscore = true;
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    if out.is_empty() {
        "boss".to_string()
    } else {
        out
    }
}
