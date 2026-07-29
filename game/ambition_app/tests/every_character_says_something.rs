//! **Nobody in the shipped cast stands mute.** (Jon, 2026-07-29)
//!
//! Jon walked the Hall of Characters and found pedestals whose occupants said
//! nothing. The measurement behind this file: 4 of 144 pedestals staged a
//! character with no catalog row at all — every one registered by a demo
//! provider through `register_character`, which until that day could carry art
//! and a name but not a voice.
//!
//! # Why silence is the failure and not merely a gap
//!
//! The ambient bark ticker SKIPS an actor it can find no line for. So a
//! character with nothing authored is not "quiet", it is invisible to the one
//! system whose whole job is making a room feel inhabited — and the Hall is a
//! room whose entire purpose is that its inhabitants introduce themselves.
//!
//! It also degrades in exactly the wrong direction for an engine: the
//! population that cannot speak is *characters another game brought*, because
//! those are the ones with no row in Ambition's catalog. A stranger's character
//! stands silent while ours chatter.
//!
//! # What this checks that the unit tests cannot
//!
//! `player_robot_lineage` guards its own three, and each provider's registration
//! is a few lines somebody can read. Neither can see the COMPOSED cast — the
//! shipped host's catalog plus every provider's registrations, which is the only
//! place the real population exists. This walks that, and it fails naming the
//! silent characters rather than a count.

use ambition::characters::actor::character_catalog::{BarkSituation, CharacterCatalog};
use ambition_app::app::{build_visible_app, VisibleRenderMode};

/// Every character the shipped host composes can produce at least one line.
///
/// "At least one" is deliberately the bar. This is not asking for a dialogue
/// graph, a situation pool, or good writing — it is asking that the floor be a
/// sentence instead of silence, which is the difference between a pedestal that
/// is part of the game and one that reads as unfinished.
#[test]
fn every_composed_character_can_say_at_least_one_line() {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    // ⚠ **THE BARRIER, or the registry is not the cast.** `register_character`
    // queues a definition; the prepared registry is PUBLISHED once, whole, at
    // `Plugin::finish`. Reading it off a freshly built App finds a partial cast
    // — which is how the first version of this test reported three characters
    // mute that had voices, and would equally have reported a real one silent
    // as fine.
    ambition::platformer::app_finalization::finalize(&mut app);
    let catalog = app.world().resource::<CharacterCatalog>();
    let registry = app
        .world()
        .get_resource::<ambition::actors::character_runtime::PreparedCharacterRegistry>();

    // The composed population: catalog rows AND registered-only characters. The
    // second half is the one that was mute, so a sweep over the catalog alone
    // would have passed while the defect was live.
    let mut ids: Vec<String> = catalog.iter().map(|(id, _)| id.clone()).collect();
    if let Some(registry) = registry {
        ids.extend(registry.ids().map(str::to_string));
    }
    ids.sort();
    ids.dedup();

    assert!(
        ids.len() > 50,
        "the shipped host composed only {} characters, so this is probing a \
         population too small to be the real cast and would pass forever",
        ids.len()
    );

    let silent: Vec<&String> = ids
        .iter()
        .filter(|id| {
            // The same resolution order the bark authority uses: the catalog's
            // pool for the situation, then its fallback pool, then whatever the
            // character's own definition brought.
            let from_catalog = catalog.bark_line(id, BarkSituation::Hall, 0).is_some();
            let from_definition = registry
                .and_then(|registry| registry.get(id))
                .and_then(|prepared| prepared.voice_line(0))
                .is_some();
            !from_catalog && !from_definition
        })
        .collect();

    assert!(
        silent.is_empty(),
        "{} composed character(s) can produce no line at all, so the ambient \
         ticker skips them and they stand mute wherever they are staged — most \
         visibly on a Hall pedestal. Give each one `barks.hall` /\n\
         `fallback_dialogue` in its catalog row, or `CharacterDefinition::\
         with_voice` if it is registered-only:\n\n  {}\n",
        silent.len(),
        silent
            .iter()
            .map(|id| id.as_str())
            .collect::<Vec<_>>()
            .join("\n  "),
    );
}
