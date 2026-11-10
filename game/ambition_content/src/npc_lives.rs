//! Ambition's NPC lives (`data/npc_lives.ron`), published to the engine's
//! executor as its [`LifeBook`]. A content reload publishes the book again,
//! so an edited domain is the next plan's domain.

use std::sync::Arc;

use ambition_content_schemas::npc_lives::{lowered_npc_lives, NpcLives, Observation as Authored};
use ambition_platformer2d::actors::body::life::{LifeBook, LifeSpec, Observation};
use bevy::prelude::App;

/// The book of `lives`. Absent means no lives: no body plans.
pub fn life_book(lives: Option<&NpcLives>) -> LifeBook {
    let Some(lives) = lives else {
        return LifeBook::default();
    };
    LifeBook {
        domains: lives.domains.iter().map(|(name, domain)| (name.clone(), Arc::new(domain.clone()))).collect(),
        lives: lives
            .lives
            .iter()
            .map(|(character, row)| {
                let observes = row
                    .observes
                    .iter()
                    .map(|(name, observation)| {
                        let observation = match observation {
                            Authored::Free(thing) => Observation::Free(thing.clone()),
                            Authored::Present(character) => Observation::Present(character.clone()),
                        };
                        (name.clone(), observation)
                    })
                    .collect();
                (character.clone(), LifeSpec {
                        domain: row.domain.clone(),
                        facts: row.facts.clone(),
                        authored: row.authored.clone(),
                        daily: row.daily.clone(),
                        observes,
                    })
            })
            .collect(),
        holds: lives.holds.clone(),
    }
}

pub(super) fn register(app: &mut App) {
    let pack = crate::pack::select(app.world_mut());
    app.insert_resource(life_book(lowered_npc_lives(&pack)));
}

/// The reload half: the candidate pack's lives replace the book.
pub(crate) fn publish(world: &mut bevy::ecs::world::World, pack: &ambition_content_pack::PreparedContentPack) {
    world.insert_resource(life_book(lowered_npc_lives(pack)));
}
