//! Boss encounter id helper: `encounter_id_from_name` slugs an authored boss
//! name into a stable id (`"Clockwork Warden"` -> `"clockwork_warden"`). The
//! engine names no boss — every boss's chest reward is authored data
//! (`BossRewardProfile::DropChest` in `boss_profiles.ron`), resolved through
//! the generic `encounter_chest_<id>` naming.

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
