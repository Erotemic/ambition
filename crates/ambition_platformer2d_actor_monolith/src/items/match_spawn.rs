//! ITEMS THE MATCH DROPS — the genre's other combatant.
//!
//! ⭐⭐ AND IT IS THE FIRST CUSTOMER OF `sim_random`, which is why it exists in
//! this shape. A spawner is the smallest honest consumer of a rollback-safe
//! random source: it draws twice a tick (which item, which point), it must draw
//! the SAME two on a resimulated tick, and it must not correlate with anything
//! else drawing that tick.
//!
//! ⛔⛔ NO SCHEDULE STATE OF ITS OWN, and that is still the whole design. There
//! is no countdown resource here and no "last spawned" tick: a drop happens on
//! the ticks where `elapsed % every_ticks == 0`, a pure function of the match
//! clock the same way the opening ceremony's phase is.
//!
//! ⚠ WHAT CHANGED, 2026-08-24: `elapsed` is now the LIVE match clock
//! ([`LiveMatchTicks`](crate::character_runtime::live_match_clock::LiveMatchTicks))
//! rather than `ActiveMatch::ticks_since_activation`. That clock IS counted, and
//! it IS registered rollback state — the cost this paragraph used to say the
//! spawner would not pay. It is paid once, by the clock, for the two consumers
//! that need it, instead of here: the alternative was each consumer patching
//! around the ceremony its own way, which is what the hand-written `elapsed == 0`
//! below used to be doing.
//!
//! ⛔ AND THE IDENTITY IS DERIVED, not sequenced. `SimId::match_spawn(activation,
//! tick)` — the pickup road mints under the THROWER and takes a `SimIdCounter`
//! from it, and a match-level spawner has no thrower. Deriving is not a
//! workaround for that: `(match, tick)` determines the object completely and at
//! most one spawn per tick exists, so a counter would be a second authority on a
//! fact the tick already settles. ⚠ the tick in that pair is the LIVE one now,
//! which is still strictly increasing within a match and so still unique.

use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::SpawnScopedExt;
use bevy::prelude::*;

/// Which of the two draws a tick makes. Distinct salts, because
/// `sim_random(domain, tick, salt)` with one salt answers the same number twice
/// — the footgun its own docs name.
const SALT_WHICH_ITEM: u64 = 0;
const SALT_WHICH_POINT: u64 = 1;

/// Drop one item on the ticks the match's rules say to.
pub fn spawn_match_items(
    mut commands: Commands,
    prepared: Option<Res<crate::character_runtime::PreparedMatch>>,
    active: Option<Res<crate::character_runtime::ActiveMatch>>,
    // HOW LONG THIS MATCH HAS BEEN FOUGHT — the same reading the timeout uses,
    // which is the whole point of the row this closes.
    live: Res<crate::character_runtime::live_match_clock::LiveMatchTicks>,
) {
    let (Some(prepared), Some(active)) = (prepared, active) else {
        return;
    };
    let Some(rules) = prepared.rules().item_spawns.as_ref() else {
        return;
    };
    if !rules.active() {
        return;
    }
    // ⭐ THE LIVE CLOCK — ticks this match has actually been FOUGHT, with the
    // opening ceremony and every pause already excluded by the one system that
    // owns the question. This used to be `ticks_since_activation` plus a
    // hand-written `elapsed == 0`, which stood in for "not during the countdown"
    // and stopped being true the moment an interval was shorter than one.
    let elapsed = live.of(&active);
    // ⛔ AND ZERO IS STILL SKIPPED, for what is now its real reason: live tick
    // zero is the RELEASE tick, and an item landing on it is an item nobody had
    // a frame to contest. The first drop is one full interval in.
    if elapsed == 0 || elapsed % u64::from(rules.every_ticks) != 0 {
        return;
    }

    let weights: Vec<u32> = rules.table.iter().map(|(_, weight)| *weight).collect();
    let Some(chosen) = ae::sim_random::sim_random_weighted(
        ae::sim_random::DOMAIN_ITEM_SPAWN,
        elapsed,
        SALT_WHICH_ITEM,
        &weights,
    ) else {
        return;
    };
    let Some(point) = ae::sim_random::sim_random_index(
        ae::sim_random::DOMAIN_ITEM_SPAWN,
        elapsed,
        SALT_WHICH_POINT,
        rules.points.len(),
    )
    .map(|index| rules.points[index]) else {
        return;
    };
    let id = &rules.table[chosen].0;
    // `held_spec_by_id`, the WIDE lookup — the same one the checkpoint rebuild
    // uses, and for the same reason: an id that came from the item catalog
    // rather than the brain's registry answers `None` to the narrow one.
    let Some(spec) = crate::items::pickup::held_spec_by_id(id) else {
        // ⛔ A ROW NAMING AN ITEM THAT DOES NOT EXIST DROPS NOTHING, and says so
        // once rather than every interval: an authored table is content, and
        // content can name something that has been removed.
        bevy::log::warn_once!(
            target: "ambition_platformer2d::items",
            "this match's spawn table names `{id}`, and no item spec answers to it"
        );
        return;
    };
    let sim_id = active.instance().parts().1.map(|activated_on| {
        ambition_platformer2d_shared_tangle::sim_id::SimId::match_spawn(activated_on, elapsed)
    });
    let mut spawned = commands.spawn_room_scoped((
        crate::items::pickup::GroundItem {
            spec,
            // AT REST. A dropped item falls under `ground_item_physics` from
            // wherever the stage put its point; giving it a velocity here would
            // be this system having an opinion about how items arrive, which is
            // presentation the stage owns.
            vel: ae::Vec2::ZERO,
            pos: point,
            half_extent: crate::items::pickup::MINTED_ITEM_HALF_EXTENT,
        },
        Name::new(format!("Match item: {id}")),
    ));
    if let Some(sim_id) = sim_id {
        spawned.insert(sim_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character_runtime::MatchItemSpawns;

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
                tick,
                SALT_WHICH_ITEM,
                &weights,
            )
            .expect("the table has weight");
            let point = ae::sim_random::sim_random_index(
                ae::sim_random::DOMAIN_ITEM_SPAWN,
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
                    tick,
                    SALT_WHICH_ITEM
                ),
                ae::sim_random::sim_random(
                    ae::sim_random::DOMAIN_ITEM_SPAWN,
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

    /// ⭐ AND THE SCHEDULE IS A PURE FUNCTION OF THE MATCH CLOCK.
    ///
    /// ⛔⛔ NO COUNTDOWN RESOURCE, which is the point: a ticking timer here would
    /// be authoritative mutable state inside the rollback window, the trap
    /// `prepared_match` documents having paid for once already. And tick ZERO
    /// never drops — the fighters are still held by the opening countdown, and an
    /// item nobody can contest is one somebody walks into.
    #[test]
    fn the_drop_schedule_is_the_clock_and_the_opening_tick_is_never_one() {
        let every = 480u64;
        let drops = |elapsed: u64| elapsed != 0 && elapsed % every == 0;

        assert!(!drops(0), "a match dropped an item on the tick it opened");
        assert!(!drops(479));
        assert!(drops(480), "the first interval did not drop");
        assert!(drops(960), "the second interval did not drop");
        assert!(!drops(961));
        // …and re-asking any tick answers the same, because there is nothing to
        // advance.
        assert_eq!(drops(480), drops(480));
    }
}
