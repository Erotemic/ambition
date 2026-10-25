//! Deterministic match item spawns.
//!
//! Spawn at gameplay-clock interval crossings, not sampled tick
//! divisibility. Scaled time can repeat a tick. The drop ordinal
//! determines its stable identity; match context and distinct salts
//! determine the two rollback-safe random draws. No independent
//! spawn countdown or identity counter is needed.

use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::{SessionCommands, SpawnSessionScopedExt};
use bevy::prelude::*;

/// Which of the two draws a tick makes. Distinct salts, because
/// `sim_random(domain, tick, salt)` with one salt answers the same number twice
/// — the footgun its own docs name.
const SALT_WHICH_ITEM: u64 = 0;
const SALT_WHICH_POINT: u64 = 1;

/// Drop one item on the ticks the match's rules say to.
pub fn spawn_match_items(
    mut commands: SessionCommands,
    prepared: Option<Res<ambition_match::PreparedMatch>>,
    active: Option<Res<ambition_match::ActiveMatch>>,
    // HOW LONG THIS MATCH HAS BEEN FOUGHT — the same reading the timeout uses,
    // which is the whole point of the row this closes.
    live: Res<ambition_match::live_match_clock::LiveMatchTicks>,
) {
    let (Some(prepared), Some(active)) = (prepared, active) else {
        return;
    };
    // The session that owns the item. With no session, no match is fought.
    let Some(scope) = commands.spawn_scope() else {
        return;
    };
    let Some(rules) = prepared.rules().item_spawns.as_ref() else {
        return;
    };
    if !rules.active() {
        return;
    }
    // `crossed` reports each interval once even when scaled gameplay
    // time holds a projected tick across multiple simulation steps.
    // The first spawn is after one complete interval, not at release.
    let Some(ordinal) = live.crossed(&active, rules.every_ticks) else {
        return;
    };

    // ⭐⭐ WHICH MATCH IS DRAWING. Without it every match is the same match: the
    // clock above restarts at zero, so match two drew match one's items, in
    // order, from its first drop. The stamp is already canonical simulation
    // state, so a resimulated tick draws with the context it drew with before.
    let context = active.random_context();
    let weights: Vec<u32> = rules.table.iter().map(|(_, weight)| *weight).collect();
    let Some(chosen) = ae::sim_random::sim_random_weighted(
        ae::sim_random::DOMAIN_ITEM_SPAWN,
        context,
        ordinal,
        SALT_WHICH_ITEM,
        &weights,
    ) else {
        return;
    };
    let Some(point) = ae::sim_random::sim_random_index(
        ae::sim_random::DOMAIN_ITEM_SPAWN,
        context,
        ordinal,
        SALT_WHICH_POINT,
        rules.points.len(),
    )
    .map(|index| rules.points[index]) else {
        return;
    };
    let id = &rules.table[chosen].0;
    // `held_spec_by_id`, the same lookup the checkpoint rebuild uses. ⚠ It was
    // once the WIDE one of two, for ids the brain's registry did not hold; there
    // is one registry since 2026-09-05 and this is now a pass-through to it.
    let Some(spec) = ambition_held_items::held_spec_by_id(id) else {
        // ⛔ A ROW NAMING AN ITEM THAT DOES NOT EXIST DROPS NOTHING, and says so
        // once rather than every interval: an authored table is content, and
        // content can name something that has been removed.
        bevy::log::warn_once!(
            target: "ambition_platformer2d::items",
            "this match's spawn table names `{id}`, and no item spec answers to it"
        );
        return;
    };
    // ⛔⛤ THIS USED TO NAME THE ACTIVATION TICK, AND THE ID IS CANONICAL.
    // `entity.sim_id` is `component-canonical`, so the whole string is in the
    // peer checksum, and the tick counts every sim step the App has run —
    // menus included. Two peers minted DIFFERENT identities for the same item.
    // The match's ordinal within its session is the term they agree on.
    let sim_id = active.ordinal().map(|match_ordinal| {
        ambition_platformer2d_shared_tangle::sim_id::SimId::match_spawn(match_ordinal, ordinal)
    });
    let mut spawned = commands.spawn_room_in_session(
        scope,
        (
            // AT REST, and the constructor is now what says so. A dropped item falls
            // under `ground_item_physics` from wherever the stage put its point;
            // giving it a velocity here would be this system having an opinion about
            // how items arrive, which is presentation the stage owns.
            ambition_held_items::GroundItem::at_rest(
                spec,
                point,
                ambition_held_items::MINTED_ITEM_HALF_EXTENT,
            ),
            Name::new(format!("Match item: {id}")),
        ),
    );
    if let Some(sim_id) = sim_id {
        spawned.insert(sim_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_match::MatchItemSpawns;

    /// One match's context, for the draws these tests inspect directly. The
    /// value is arbitrary — what matters is that it is the SAME one across a
    /// comparison, since the point of those assertions is the salt.
    const CONTEXT: ae::sim_random::RandomContext = 0x5A11_0000_1234_0001;

    fn rules(every_ticks: u32) -> MatchItemSpawns {
        MatchItemSpawns {
            every_ticks,
            table: vec![("bomb".to_string(), 3), ("gravity_grenade".to_string(), 1)],
            points: vec![
                ae::Vec2::new(100.0, 40.0),
                ae::Vec2::new(200.0, 40.0),
                ae::Vec2::new(300.0, 40.0),
            ],
        }
    }

    /// ⭐⭐ FOUR WAYS TO BE OFF, AND `active()` IS THE ONE PLACE THAT KNOWS THEM.
    ///
    /// ⛔ A caller that checked two of the three and missed the third would drop
    /// nothing while believing items were on — or worse, panic indexing an empty
    /// point list. Asked in one place for exactly that reason.
    #[test]
    fn a_declaration_is_off_unless_it_has_an_interval_a_point_and_a_weight() {
        assert!(rules(480).active(), "a complete declaration reads as off");
        assert!(
            !rules(0).active(),
            "a zero interval reads as ON, so a rules screen cannot switch items \
             off without deleting the table"
        );
        assert!(
            !MatchItemSpawns {
                points: vec![],
                ..rules(480)
            }
            .active(),
            "a declaration with nowhere to drop reads as on"
        );
        assert!(
            !MatchItemSpawns {
                table: vec![("bomb".to_string(), 0)],
                ..rules(480)
            }
            .active(),
            "a table whose every row is switched off reads as on"
        );
    }

    /// ⭐⭐ THE SAME TICK DROPS THE SAME ITEM AT THE SAME PLACE — which is the
    /// whole reason this draws from `sim_random` instead of a stream.
    ///
    /// ⛔⛔ AND THE TWO DRAWS MUST NOT AGREE WITH EACH OTHER. They share a tick
    /// and a domain and differ only in their salt; a spawner that passed one salt
    /// twice would pick item `n` and point `n` forever, which looks random for
    /// one draw and is visibly wrong the moment somebody watches the stage.
    #[test]
    fn a_tick_decides_the_item_and_the_point_and_they_do_not_track_each_other() {
        let weights = [3u32, 1];
        let mut same_index = 0;
        let mut both = Vec::new();
        for tick in 1..200u64 {
            let item = ae::sim_random::sim_random_weighted(
                ae::sim_random::DOMAIN_ITEM_SPAWN,
                CONTEXT,
                tick,
                SALT_WHICH_ITEM,
                &weights,
            )
            .expect("the table has weight");
            let point = ae::sim_random::sim_random_index(
                ae::sim_random::DOMAIN_ITEM_SPAWN,
                CONTEXT,
                tick,
                SALT_WHICH_POINT,
                3,
            )
            .expect("three points");
            // Reproducible: the same tick, asked again.
            assert_eq!(
                item,
                ae::sim_random::sim_random_weighted(
                    ae::sim_random::DOMAIN_ITEM_SPAWN,
                    CONTEXT,
                    tick,
                    SALT_WHICH_ITEM,
                    &weights,
                )
                .unwrap(),
                "tick {tick} chose a different item the second time it was asked, \
                 so a resimulated tick drops something else and the peers diverge"
            );
            // ⛔⛔ THE CORRELATION TEST IS ON THE RAW DRAWS, not on the reduced
            // indices. `item` is a weighted pick over 2 rows and `point` is an
            // index over 3, so those two differ under ANY salt — an assertion on
            // them cannot fail and would report a shared salt as healthy.
            // Measured: it passed with both salts set to 0.
            assert_ne!(
                ae::sim_random::sim_random(
                    ae::sim_random::DOMAIN_ITEM_SPAWN,
                    CONTEXT,
                    tick,
                    SALT_WHICH_ITEM
                ),
                ae::sim_random::sim_random(
                    ae::sim_random::DOMAIN_ITEM_SPAWN,
                    CONTEXT,
                    tick,
                    SALT_WHICH_POINT
                ),
                "the two draws on tick {tick} come from the same number, so the \
                 spawner is asking one question twice and calling the answers \
                 `which item` and `which point`"
            );
            if item == point {
                same_index += 1;
            }
            both.push((item, point));
        }
        // Kept as a weak sanity reading only — see the assertion above for why
        // it cannot be the correlation test.
        assert!(same_index < 199, "every drop put item n at point n");
        assert!(
            both.iter().any(|(i, _)| *i == 1),
            "the lighter-weighted item never came up in 199 drops, so the table \
             is not being read as a table"
        );
    }

    // ⭐ THE SCHEDULE IS A PURE FUNCTION OF THE MATCH CLOCK, and the rule now
    // lives entirely in `LiveMatchTicks::crossed` — so it is pinned where the
    // real clock can be driven, by
    // `live_match_clock::tests::each_interval_is_crossed_exactly_once_at_any_time_scale`.
    // What used to stand here re-implemented `elapsed % every == 0` locally and
    // agreed with itself; it could not have seen the repeat that a scaled clock
    // produces, because it never asked the clock.
    //
    // ⛔⛔ NO COUNTDOWN RESOURCE, which is still the point: a ticking timer here
    // would be authoritative mutable state inside the rollback window, the trap
    // `prepared_match` documents having paid for once already. And ordinal ZERO
    // never drops — the fighters are still held by the opening countdown, and an
    // item nobody can contest is one somebody walks into.
}
