//! A boss's voice: the barks of the character row its encounter names.

use super::*;
use ambition_characters::actor::character_catalog::{
    BarkSituation, CharacterCatalog, CharacterCatalogData,
};

fn shipped_characters() -> CharacterCatalog {
    CharacterCatalog::from_data(
        ron::from_str::<CharacterCatalogData>(include_str!(
            "../../../../game/ambition_content/assets/data/character_catalog.ron"
        ))
        .expect("the shipped character catalog parses"),
    )
}

fn gnu_ton() -> BossClusterScratch {
    BossClusterScratch::new(
        crate::test_boss_catalog(),
        "gnu_ton",
        "GNU-ton",
        ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(64.0, 96.0)),
        ambition_entity_catalog::placements::BossBrain::PhaseScript {
            script_id: "gnu_ton_rider".to_string(),
        },
    )
}

/// GNU-ton speaks the lines of the character his encounter names, in the
/// row's order, and the same boss with no voice says nothing.
#[test]
fn a_boss_speaks_the_barks_of_the_character_its_encounter_names() {
    let characters = shipped_characters();
    let boss = gnu_ton();
    let voice = boss
        .config
        .seed
        .as_ref()
        .and_then(|seed| seed.encounter.voice.clone())
        .expect("gnu_ton_rider.ron names a voice");
    for situation in [BarkSituation::OnHit, BarkSituation::Idle] {
        let pool = characters.get(&voice).expect("the voice is a row").barks.pool(situation);
        assert!(!pool.is_empty(), "{voice} has {situation:?} lines");
        for rotation in 0..pool.len() as u32 + 1 {
            assert_eq!(
                boss.config.bark(&characters, situation, rotation),
                characters.bark_line(&voice, situation, rotation),
                "{situation:?} line {rotation}"
            );
        }
    }
    assert_eq!(boss.config.bark(&characters, BarkSituation::OnHit, 0), Some("Counterfeit."));

    let mut silent = gnu_ton();
    silent.config.seed.as_mut().expect("seeded").encounter.voice = None;
    assert_eq!(silent.config.bark(&characters, BarkSituation::OnHit, 0), None);
}
