//! A boss's voice: the barks of the character it is an instance of.

use super::*;
use ambition_characters::actor::character_catalog::{BarkSituation, CharacterCatalog, CharacterCatalogData};

fn shipped_characters() -> CharacterCatalog {
    CharacterCatalog::from_data(
        ron::from_str::<CharacterCatalogData>(include_str!(
            "../../../../game/ambition_content/assets/data/character_catalog.ron"
        ))
        .expect("the shipped character catalog parses"),
    )
}

fn gnu_ton(sprite_content: &ambition_sprite_sheet::SpriteContent) -> BossClusterScratch {
    BossClusterScratch::new(&sprite_content,
        crate::test_boss_catalog(),
        "gnu_ton",
        "GNU-ton",
        ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(64.0, 96.0)),
        ambition_entity_catalog::placements::BossBrain::PhaseScript {
            script_id: "gnu_ton_rider".to_string(),
        },
    )
}

/// GNU-ton speaks the lines of his character, in the row's order, and a boss
/// whose character has no row says nothing.
#[test]
fn a_boss_speaks_the_barks_of_its_character() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let characters = shipped_characters();
    let boss = gnu_ton(&sprite_content);
    let voice = boss.config.behavior.character.clone();
    assert_eq!(voice, "npc_gnu_ton_boss");
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

    let mut silent = gnu_ton(&sprite_content);
    silent.config.behavior.character = "npc_nobody".to_string();
    assert_eq!(silent.config.bark(&characters, BarkSituation::OnHit, 0), None);
}
