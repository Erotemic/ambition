use super::*;

#[test]
fn spatial_selection_matches_canonical_scan_across_relationships_and_rebuilds() {
    let mut app = App::new();
    let entities: Vec<_> = (0..129)
        .map(|_| app.world_mut().spawn_empty().id())
        .collect();
    let teams: Vec<_> = (0..9)
        .map(|i| MatchTeam::new(format!("team-{i}")))
        .collect();
    let ids: Vec<_> = (0..129)
        .map(|i| SimId::placement(&format!("body-{i:04}")))
        .collect();
    let mut index = TargetSelectionIndex::default();
    let mut relations = FactionRelations::default();
    relations.set_hostile(ActorFaction::Npc, ActorFaction::Boss, true);
    relations.set_allied(ActorFaction::Boss, ActorFaction::Player, true);
    for tick in 0..16 {
        let room = Some(LiveRoomInstance::ACTIVATION);
        let other_room = Some(LiveRoomInstance::ACTIVATION.next());
        let mut candidates: Vec<_> = entities
            .iter()
            .enumerate()
            .take(129 - tick * 3)
            .map(|(i, &e)| {
                (
                    e,
                    ae::Vec2::new(
                        ((i * 71 + tick * 29) % 101) as f32 - 50.0,
                        ((i * 31 + tick * 11) % 97) as f32 - 48.0,
                    ),
                    FACTIONS[i % 5],
                    Some(&ids[i]),
                    if i % 3 == 0 {
                        None
                    } else {
                        Some(&teams[i % teams.len()])
                    },
                    if i % 4 == 0 { other_room } else { room },
                )
            })
            .collect();
        // A body can occur in both candidate queries, with the same identity.
        let mut duplicate = candidates[1];
        duplicate.1 += ae::Vec2::ONE;
        candidates.insert(2, duplicate);
        if tick == 5 {
            candidates[1].1.x = f32::NAN;
        }
        let tree_teams = index.rebuild(&candidates);
        for observer in 0..40 {
            let entity = entities[observer];
            let position = if tick == 9 {
                ae::Vec2::splat(f32::INFINITY)
            } else {
                ae::Vec2::new(observer as f32 - 17.0, tick as f32 - 5.0)
            };
            for faction in FACTIONS {
                for team in [None, Some(&teams[observer % teams.len()])] {
                    for grudge in [
                        None,
                        Some(crate::components::Grudge::Body(entities[1])),
                        Some(crate::components::Grudge::Faction(ActorFaction::Npc)),
                    ] {
                        for relations in [None, Some(&relations)] {
                            let actor = Combatant {
                                faction,
                                driver: None,
                                team,
                            };
                            let mut expected = None;
                            for candidate in &candidates {
                                if candidate.0 == entity || candidate.5 != room {
                                    continue;
                                }
                                if !combat_relation(
                                    relations,
                                    actor,
                                    grudge,
                                    candidate.0,
                                    Combatant {
                                        faction: candidate.2,
                                        driver: None,
                                        team: candidate.4,
                                    },
                                )
                                .is_target()
                                {
                                    continue;
                                }
                                let d = distance_squared(candidate.1, position);
                                if expected.is_none_or(|(_, _, before)| d < before) {
                                    expected = Some((candidate.0, candidate.1, d));
                                }
                            }
                            let actual = index.nearest(
                                &candidates,
                                &tree_teams,
                                entity,
                                position,
                                room,
                                relations,
                                actor,
                                grudge,
                            );
                            let bits = |result: Option<(Entity, ae::Vec2, f32)>| {
                                result
                                    .map(|(e, p, d)| (e, p.x.to_bits(), p.y.to_bits(), d.to_bits()))
                            };
                            assert_eq!(bits(actual),bits(expected),"tick {tick}, observer {observer}, {faction:?}, {team:?}, {grudge:?}");
                        }
                    }
                }
            }
        }
    }
    assert!(index.rebuild(&[]).is_empty());
    assert!(index.rooms.is_empty());
}
