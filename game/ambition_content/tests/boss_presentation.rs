//! What the boss catalog says about sheets and special telegraphs, pinned
//! exactly — so moving it out of Rust cannot change it.
//!
//! It asserts the VALUES, not the mechanism, so it says nothing about where they live and
//! everything about what they are.
//!
//! it deliberately pins the exact sets, not just counts. A count survives
//! a typo'd key — and a typo'd key here is silent by construction: a boss whose
//! sheet filename is misspelled draws a placeholder, and a special whose name is
//! misspelled telegraphs nothing, because `special_animation_keys` returns `&[]`
//! for anything unregistered (`boss_animation_keys_for_profile`: *"Unregistered
//! → no special row"*).
//!
//! `apple_rain` is absent from the telegraph map ON PURPOSE and this test records that rather
//! than fixing it. It is the eleventh declared `Special` in `boss_profiles.ron` and the only one
//! with no rows, which is the single line of content blocking the `BossAnim`→`CharacterAnim` fold.
//! A sibling test — `apple_rain_claims_no_animation_rows_which_is_why_the_fold_is_blocked` — fails
//! on EITHER answer so neither happens quietly; this one would too.

use std::collections::{BTreeMap, BTreeSet};

/// The seven boss sheets Ambition ships, and the file each one draws from.
const EXPECTED_SHEETS: [(&str, &str); 7] = [
    (
        "flying_spaghetti_monster_boss",
        "flying_spaghetti_monster_boss_spritesheet.png",
    ),
    ("giant_gnu", "gnu_ton_boss/giant_gnu_spritesheet.png"),
    (
        "gnu_ton_rider",
        "gnu_ton_boss/gnu_ton_rider_spritesheet.png",
    ),
    ("gradient_sentinel", "boss_spritesheet.png"),
    (
        "mockingbird",
        "mockingbird_boss/mockingbird_boss_spritesheet.png",
    ),
    (
        "smirking_behemoth_boss",
        "smirking_behemoth_boss_spritesheet.png",
    ),
    ("trex_boss", "trex_enemy_spritesheet.png"),
];

/// The nineteen content specials that telegraph, and the sprite rows each one
/// asks for. The first key is the canonical runtime key.
const EXPECTED_TELEGRAPHS: [(&str, &[&str]); 19] = [
    ("echo_fan", &["spike_halo", "eye_beam"]),
    ("eye_beam", &["eye_beam", "spike_halo"]),
    ("gradient_cascade", &["spike_halo"]),
    ("gradient_nova", &["spike_halo"]),
    ("minima_trap", &["spike_halo"]),
    ("mode_collapse_converge", &["spike_halo"]),
    ("overfit_volley", &["spike_halo", "eye_beam"]),
    ("overflow_flood", &["spike_halo"]),
    ("saddle_point", &["spike_halo"]),
    ("trex_bite", &["bite"]),
    ("trex_call", &["roar"]),
    ("trex_charge", &["roar"]),
    ("trex_double_bite", &["bite"]),
    ("trex_jaw_grab", &["grab_reach"]),
    ("trex_leap", &["leap"]),
    ("trex_roar", &["roar"]),
    ("trex_snap_up", &["snap_up"]),
    ("trex_stomp", &["stomp"]),
    ("trex_tail_whip", &["tail_swipe"]),
];

/// The sheet rows each geometry strike asks for, for each of the engine's
/// eleven strike keys. The first key is the canonical runtime key; the others
/// are row-name aliases. Three strikes ask for none and keep their static
/// boxes.
///
/// These are the values of the table that was in the boss crate's Rust
/// (`boss_animation_keys_for_profile`) before the rows were authored in
/// `boss_art_keys.ron`. This test passed on that table first, and then on the
/// authored rows, so the move changed no value.
const EXPECTED_STRIKE_ROWS: [(&str, &[&str]); 11] = [
    ("broadside", &[]),
    ("converging_shockwave", &["gnu_shockwave", "hand_slam"]),
    ("dive_lane", &[]),
    ("floor_slam", &["floor_slam", "mouth_open"]),
    ("full_body_pulse", &["spike_halo", "eye_beam"]),
    ("hand_slam", &["gnu_hand_slam", "hand_slam"]),
    ("hand_sweep", &["gnu_hand_sweep", "hand_sweep"]),
    ("hazard_column", &["dash_echo", "eye_beam"]),
    ("head_descent", &["gnu_head_descent", "head_down"]),
    ("side_sweep", &["side_sweep"]),
    ("wing_sweep", &[]),
];

#[test]
fn each_geometry_strike_asks_for_exactly_these_rows() {
    use ambition_characters::brain::boss_pattern::BUILTIN_STRIKE_KEYS;
    use ambition_characters::brain::BossAttackProfile;
    let catalog = ambition_content::bosses::authored_boss_catalog();
    let got: BTreeMap<&str, Vec<String>> = BUILTIN_STRIKE_KEYS
        .iter()
        .map(|key| {
            let profile = BossAttackProfile::Strike((*key).to_string());
            (
                *key,
                ambition_boss_encounter::behavior::boss_animation_keys_for_profile(&catalog, &profile),
            )
        })
        .collect();
    let want: BTreeMap<&str, Vec<String>> = EXPECTED_STRIKE_ROWS
        .iter()
        .map(|(key, rows)| (*key, rows.iter().map(|row| (*row).to_string()).collect()))
        .collect();
    assert_eq!(
        got, want,
        "the sheet rows of a geometry strike changed. `boss_art_keys.ron` is their one authored \
         home; a change there must be deliberate"
    );
}

/// The generic animation row each attack plays, and the sheet row its
/// damageable box is sampled from, for the engine's eleven strike keys and
/// for three specials: `apple_rain` (it claims no row and reads the head
/// row), one that telegraphs, and one no boss declares.
///
/// These are the values the two Rust tables of the boss crate gave
/// (`boss_anim_for_attack_profile`, `boss_animation_key_for_sample`) before
/// the rows were authored in `boss_art_keys.ron`. The authored tables were
/// compared with the Rust tables for 20 profiles before the Rust tables were
/// deleted.
const EXPECTED_ATTACK_ART: [(&str, bool, Option<&str>, Option<&str>); 14] = [
    // (move id, it is a special, the animation row, the hurtbox sample row)
    ("broadside", false, Some("SideSweep"), None),
    ("converging_shockwave", false, Some("FloorSlam"), Some("hand_slam")),
    ("dive_lane", false, Some("DashEcho"), None),
    ("floor_slam", false, Some("FloorSlam"), Some("floor_slam")),
    ("full_body_pulse", false, Some("SpikeHalo"), Some("spike_halo")),
    ("hand_slam", false, Some("FloorSlam"), Some("hand_slam")),
    ("hand_sweep", false, Some("SideSweep"), Some("hand_sweep")),
    ("hazard_column", false, Some("DashEcho"), Some("dash_echo")),
    ("head_descent", false, Some("SpikeHalo"), Some("head_down")),
    ("side_sweep", false, Some("SideSweep"), Some("side_sweep")),
    ("wing_sweep", false, None, None),
    ("apple_rain", true, Some("SpikeHalo"), Some("head_down")),
    ("overfit_volley", true, Some("SpikeHalo"), Some("spike_halo")),
    ("no_such_special", true, Some("SpikeHalo"), None),
];

#[test]
fn each_attack_plays_exactly_this_row_and_samples_exactly_this_row() {
    use ambition_characters::brain::BossAttackProfile;
    let catalog = ambition_content::bosses::authored_boss_catalog();
    let got: Vec<(&str, bool, Option<String>, Option<String>)> = EXPECTED_ATTACK_ART
        .iter()
        .map(|(move_id, special, _, _)| {
            let profile = if *special {
                BossAttackProfile::Special((*move_id).to_string())
            } else {
                BossAttackProfile::Strike((*move_id).to_string())
            };
            (
                *move_id,
                *special,
                catalog.attack_animation(&profile).map(|anim| format!("{anim:?}")),
                catalog.hurtbox_sample_row(&profile),
            )
        })
        .collect();
    let want: Vec<(&str, bool, Option<String>, Option<String>)> = EXPECTED_ATTACK_ART
        .iter()
        .map(|(move_id, special, anim, row)| {
            (*move_id, *special, anim.map(str::to_string), row.map(str::to_string))
        })
        .collect();
    assert_eq!(
        got, want,
        "the animation row or the hurtbox sample row of a boss attack changed. \
         `boss_art_keys.ron` is their one authored home; a change there must be deliberate"
    );
}

#[test]
fn the_boss_sheet_filenames_are_exactly_these_seven() {
    let catalog = ambition_content::bosses::authored_boss_catalog();
    let got: BTreeMap<&str, &str> = catalog.sprite_filenames().collect();
    let want: BTreeMap<&str, &str> = EXPECTED_SHEETS.iter().copied().collect();
    assert_eq!(
        got, want,
        "the boss sheet filenames changed. `boss_art_keys.ron` is their one \
         authored home; a change there must be deliberate"
    );
}

#[test]
fn the_ten_telegraphing_specials_ask_for_exactly_these_rows() {
    let catalog = ambition_content::bosses::authored_boss_catalog();
    for (special, want) in EXPECTED_TELEGRAPHS {
        let got: Vec<&str> = catalog
            .special_animation_keys(special)
            .iter()
            .map(String::as_str)
            .collect();
        assert_eq!(
            got, want,
            "`{special}`'s telegraph rows changed. ⚠ an EMPTY list here is not a \
             loud failure at runtime: `special_animation_keys` returns `&[]` for \
             anything unregistered, so a renamed special silently stops \
             telegraphing"
        );
    }
}

/// the invariant the schema will inherit: a telegraph row for a special
/// nobody declares is dead content, and the current map has no way to say so.
#[test]
fn every_telegraphing_special_is_one_the_profiles_actually_declare() {
    use ambition_characters::brain::BossAttackProfile;
    let catalog = ambition_content::bosses::authored_boss_catalog();
    // Every encounter has a behavior and vice versa — `BossCatalogFragment`
    // validates both directions — so walking the encounters reaches every boss.
    let declared: BTreeSet<String> = catalog
        .encounter_specs()
        .filter_map(|spec| catalog.behavior(&spec.id))
        .flat_map(|behavior| behavior.attacks.iter())
        .filter_map(|profile| match profile {
            BossAttackProfile::Special(key) => Some(key.clone()),
            BossAttackProfile::Strike(_) => None,
        })
        .collect();
    let telegraphing: BTreeSet<String> = EXPECTED_TELEGRAPHS
        .iter()
        .map(|(special, _)| (*special).to_string())
        .collect();

    // Anti-vacuity: if the walk finds nothing, the subset check below is free.
    assert!(
        !declared.is_empty(),
        "no `Special` was found in any encounter, so this test cannot tell a \
         valid telegraph key from a typo — the walk is broken, not the content"
    );

    let orphans: Vec<&String> = telegraphing.difference(&declared).collect();
    assert!(
        orphans.is_empty(),
        "these specials have telegraph rows but are declared by no boss: \
         {orphans:?}. Either the special was renamed and the map was not, or the \
         rows outlived the attack"
    );
}
