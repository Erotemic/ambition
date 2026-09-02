//! Verify the loading cover on the large `hall_of_characters` transition.
//!
//! The destination cast must be demanded up front, the foreground must become
//! visible after its configured reveal delay, and the cover must remain visible
//! while assets are outstanding. This headless test does not measure GPU upload
//! cost.

use bevy::prelude::*;

use ambition_app::app::{VisibleRenderMode, build_visible_app, shell_host};
use ambition_platformer2d::game_shell::ShellCommand;

/// The authored door from the hub. Named rather than "any zone" so this test
/// fails loudly if the Hall stops being reachable, instead of quietly covering
/// some other room's cheap transition.
const HALL_DOOR_ZONE: &str = "hall_of_characters_door";

/// Minimum staged cast that demonstrates the Hall is demanded up front.
const MINIMUM_HALL_CAST: usize = 100;

fn staged_cast_len(app: &App) -> usize {
    app.world()
        .get_resource::<ambition_platformer2d::actors::character_runtime::CharacterLoadStates>()
        .map(|states| states.cast().len())
        .unwrap_or(0)
}

fn step(app: &mut App) {
    app.update();
    std::thread::sleep(std::time::Duration::from_millis(8));
}

fn settle_cast(app: &mut App, secs: u64) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(secs);
    let mut last = usize::MAX;
    let mut quiet = 0;
    while std::time::Instant::now() < deadline {
        step(app);
        let now = staged_cast_len(app);
        if now == last {
            quiet += 1;
            if quiet >= 25 {
                break;
            }
        } else {
            last = now;
            quiet = 0;
        }
    }
}

/// Boot the no-window shipping host, walk the launcher into gameplay, and
/// record the hub → Hall transition through the room graph (not synthesised).
/// Returns the staged-cast size BEFORE the transition was recorded.
fn boot_and_record_the_hall_transition() -> (App, usize) {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));
    settle_cast(&mut app, 10);

    app.world_mut().write_message(ShellCommand::GoTo(
        shell_host::AMBITION_GAMEPLAY_ROUTE.into(),
    ));
    settle_cast(&mut app, 20);
    let before = staged_cast_len(&app);

    // The REAL transition, resolved through the room graph rather than
    // synthesised: stand in the Hall door and press interact.
    let (target_room, arrival) = {
        let mut query = app
            .world_mut()
            .query::<&ambition_platformer2d::world::rooms::RoomSet>();
        let room_set = query.iter(app.world()).next().expect("a session room set");
        let zone = room_set
            .active_loading_zones()
            .iter()
            .find(|zone| zone.id == HALL_DOOR_ZONE || zone.name == HALL_DOOR_ZONE)
            .unwrap_or_else(|| {
                panic!(
                    "the active room '{}' has no `{HALL_DOOR_ZONE}`, so this test is \
                     measuring nothing",
                    room_set.active_spec().id
                )
            })
            .clone();
        let transition = room_set
            .transition_for_player(
                zone.aabb,
                ambition_platformer2d::engine_core::Vec2::ZERO,
                true,
            )
            .expect("the hall door resolves to a transition");
        (
            room_set.rooms[transition.target_room].id.clone(),
            transition.arrival,
        )
    };

    // This one is recorded rather than walked, so the test names the avatar the same way
    // detection would.
    let subject = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<
            &ambition_platformer2d::platformer::sim_id::SimId,
            bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
        >();
        q.iter(world)
            .next()
            .expect("the hall has a primary avatar to send through its door")
            .clone()
    };
    let _ = app.world_mut()
        .resource_mut::<ambition_platformer2d::actors::session::lifecycle_commit::PendingLifecycleCommit>()
        .record(
            0,
            ambition_platformer2d::actors::session::lifecycle_commit::LifecycleIntent::Transition(
                ambition_platformer2d::actors::session::lifecycle_commit::RoomTransitionIntent {
                    subject,
                    target_room,
                    arrival,
                    edge_exit: false,
                    zone_sfx: None,
                },
            ),
        );
    (app, before)
}

#[test]
fn the_halls_transition_bills_its_whole_cast_and_covers_the_wait() {
    let (mut app, before) = boot_and_record_the_hall_transition();
    // the loop must not be the assertion. It waits for the cast to grow AT
    // ALL and then measures that same frame, so a Hall that trickled its
    // characters in ten at a time still fails the bill below. A loop that waited
    // for `>= MINIMUM_HALL_CAST` would pass by waiting, which is the shape this
    // whole file exists to catch.
    let mut arrived = false;
    for _ in 0..120 {
        step(&mut app);
        if staged_cast_len(&app) > before {
            arrived = true;
            break;
        }
    }
    assert!(
        arrived,
        "the Hall transition never demanded a single new character in 120 frames, \
         so this test measured no bill at all"
    );

    // ── The bill arrives at once ────────────────────────────────────────────
    let staged = staged_cast_len(&app);
    assert!(
        staged - before >= MINIMUM_HALL_CAST,
        "the Hall transition staged only {} new character(s) on its first frame \
         (from {before} to {staged}). The room authors 129 NpcSpawn placements, \
         so the rest are \
         being demanded later — after their actors spawn, in frame, uncovered, \
         which is the defect this file exists to keep closed",
        staged - before,
    );

    // ── And the wait is covered ─────────────────────────────────────────────
    //
    // The transition is COVERED (`cover_required`), and it releases only once
    // every character it billed is realized: the barrier holds the reveal, the
    // cover holds the screen. Images decode for real in this composition (the
    // no-window builder finishes its plugins), so the barrier SETTLES here —
    // in about ten frames of game time on this machine, which is under the
    // 250 ms `loading_reveal_after` grace, so the explicit loading foreground
    // is correctly never shown for it. The grace is a presentation contract
    // with its own tests; what this test owns is that nothing was revealed
    // while the cast was still arriving.
    let hall_ids = hall_character_ids(&mut app);
    let mut released_at = None;
    let mut ready_when_released = 0;
    let mut cover_required = false;
    for frame in 0..600 {
        let state = app
            .world()
            .resource::<ambition_platformer2d::runtime::room_transition::RoomTransitionLoadState>(
        );
        match state.active.as_ref() {
            Some(active) => {
                cover_required |= active.cover_required;
                if active.asset_readiness_complete && released_at.is_none() {
                    let assets = app
                        .world()
                        .resource::<ambition_platformer2d::sprite_sheet::game_assets::GameAssets>();
                    ready_when_released = hall_ids
                        .iter()
                        .filter(|id| assets.characters.sheet(id).is_some())
                        .count();
                    released_at = Some(frame);
                }
            }
            None if released_at.is_some() => break,
            None => {}
        }
        step(&mut app);
    }
    assert!(cover_required, "the Hall transition ran without a cover");
    let released_at = released_at.expect(
        "the Hall's asset barrier never reached readiness in 600 frames: the reveal is \
         held forever, which presents to a player as a loading screen that never ends",
    );
    assert!(
        ready_when_released >= hall_ids.len().min(MINIMUM_HALL_CAST),
        "the barrier released at frame {released_at} with {ready_when_released} of the \
         hall's {} characters realized — the reveal is not waiting for the cast it billed",
        hall_ids.len()
    );
    println!(
        "[hall-transition] barrier released after {released_at} frames with \
         {ready_when_released}/{} realized",
        hall_ids.len()
    );
}

/// The characters the hall PLACES, read from the room spec so a placement whose
/// actor failed to spawn cannot hide.
fn hall_character_ids(app: &mut App) -> Vec<String> {
    use ambition_platformer2d::entity_catalog::placements::{InteractionKindSpec, PlacementSchema};
    let mut query = app
        .world_mut()
        .query::<&ambition_platformer2d::world::rooms::RoomSet>();
    let room_set = query.iter(app.world()).next().expect("a session room set");
    let hall = room_set
        .rooms
        .iter()
        .find(|room| room.id == "hall_of_characters")
        .expect("the hall is in the room set");
    let mut ids: Vec<String> = hall
        .placements
        .iter()
        .filter_map(|placement| match &placement.schema {
            PlacementSchema::Interactable(spec) => match &spec.kind {
                InteractionKindSpec::Npc {
                    character_id: Some(id),
                    ..
                } => Some(id.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

/// ⛔⛔ THE BARRIER RELEASED WHILE 111 OF THE HALL'S SHEETS WERE ONLY DECLARED.
///
/// Measured on the host 2026-09-02 (`desktop-timeline-run-20260902T015909Z`):
/// the cover retired 66 ms after the door (`asset_wait_ms=3`), 111 actors drew
/// the placeholder rectangle with the engine's own warning — *"declared as
/// 'npc_busy_beaver' but not materialized"* — and 434 MP of art arrived in the
/// open over three seconds as nine frames of 89-355 ms. Loads are rationed to
/// one character per frame; the manifest held only the realized sheets' pages;
/// the test above counted STAGED characters and stayed green.
///
/// The rule, frame by frame: as long as any placed character's sheet is still
/// `Declared` with no terminal outcome, the transition's asset readiness is NOT
/// complete. The barrier settles here (images decode in this composition), so
/// the release is checked too: legal only with nothing left declared — and the
/// sheets must REALIZE along the way, which is the progress that proves the
/// loop was not idle.
#[test]
fn the_reveal_waits_for_every_placed_character_not_just_the_realized_ones() {
    use ambition_platformer2d::sprite_sheet::character::CharacterSheetState;
    use ambition_platformer2d::sprite_sheet::game_assets::GameAssets;

    let (mut app, _before) = boot_and_record_the_hall_transition();
    let ids = hall_character_ids(&mut app);
    assert!(
        ids.len() > 50,
        "the hall places only {} characters?",
        ids.len()
    );

    let mut realized_first = None;
    let mut realized_last = 0;
    let mut frames_with_declared = 0;
    for _frame in 0..400 {
        step(&mut app);
        let (declared, realized) = {
            let assets = app.world().resource::<GameAssets>();
            let states = app
                .world()
                .resource::<ambition_platformer2d::actors::character_runtime::CharacterLoadStates>(
            );
            let mut declared = 0;
            let mut realized = 0;
            for id in &ids {
                match assets.characters.sheet_state(id) {
                    CharacterSheetState::Ready(_) => realized += 1,
                    CharacterSheetState::Declared { character_id }
                        if states.outcome(character_id).is_none() =>
                    {
                        declared += 1
                    }
                    _ => {}
                }
            }
            (declared, realized)
        };
        if realized_first.is_none() && realized > 0 {
            realized_first = Some(realized);
        }
        realized_last = realized;
        let state = app
            .world()
            .resource::<ambition_platformer2d::runtime::room_transition::RoomTransitionLoadState>(
        );
        let Some(active) = state.active.as_ref() else {
            // The transition finished. Legal only with nothing left declared.
            assert_eq!(
                declared,
                0,
                "the transition released with {declared} of the hall's {} characters still                  only DECLARED — their art arrives after the reveal, in the open",
                ids.len()
            );
            break;
        };
        if declared > 0 {
            frames_with_declared += 1;
            assert!(
                !active.asset_readiness_complete,
                "asset readiness reported COMPLETE while {declared} of the hall's {} placed                  characters were still only declared (realized so far: {realized}). The                  barrier is waiting on the realized sheets' pages and not on the characters                  it demanded.",
                ids.len()
            );
        }
    }
    assert!(
        frames_with_declared > 0,
        "no frame ever had a declared-but-unrealized character; the ration this test          exists to cover was not exercised"
    );
    // One per frame, behind the cover, until the whole cast is realized. Before
    // the remainder was forwarded to the global demand this stalled at the
    // ration's worth (6 of 129) and the other 123 loaded after their actors
    // spawned — after the reveal.
    assert!(
        realized_last >= ids.len(),
        "only {realized_last} of the hall's {} characters realized in 400 frames (first \
         sample {:?}); the ration's remainder is not reaching the global demand, so the \
         rest will load after the reveal",
        ids.len(),
        realized_first
    );
}

/// The hall is a gallery (`RoomMetadata::gallery`): its pedestals are drawn
/// 132 px tall, so the transition realizes its cast at Quarter — 11.5x fewer
/// megapixels than the Full frames the same walk loaded on 2026-09-02 (434 MP,
/// 2.15 GB resident). Under the default (Full) setting, every hall sheet the
/// transition realizes asks for Quarter; an ordinary room would ask for Full.
#[test]
fn the_halls_cast_is_realized_at_the_gallery_tier_not_the_setting() {
    use ambition_platformer2d::persistence::settings::TextureResolutionScale;
    use ambition_platformer2d::sprite_sheet::game_assets::GameAssets;

    let (mut app, _before) = boot_and_record_the_hall_transition();
    let ids = hall_character_ids(&mut app);
    // Characters the hub already realized at the setting's tier stay as they
    // are — a Full sheet in a gallery is merely oversampled, and retiring it on
    // every entry to re-decode it on every exit would be churn for nothing.
    let already_realized: std::collections::BTreeSet<String> = {
        let assets = app.world().resource::<GameAssets>();
        ids.iter()
            .filter(|id| assets.characters.sheet(id).is_some())
            .cloned()
            .collect()
    };
    let setting = app
        .world()
        .resource::<ambition_platformer2d::persistence::settings::UserSettings>()
        .video
        .quality
        .resolved_budget()
        .sprites
        .effective_scale();
    assert!(
        setting > TextureResolutionScale::Quarter,
        "this test needs a setting ABOVE the gallery cap to tell the two apart; got {setting:?}"
    );
    // Let the ration realize a good part of the cast behind the cover.
    for _ in 0..200 {
        step(&mut app);
    }
    let assets = app.world().resource::<GameAssets>();
    let mut tiers: std::collections::BTreeMap<TextureResolutionScale, usize> = Default::default();
    let mut kept_as_they_were = 0;
    for id in &ids {
        let Some(sheet) = assets.characters.sheet(id) else {
            continue;
        };
        if already_realized.contains(id) {
            assert_eq!(
                sheet.requested_tier, setting,
                "'{id}' was realized at {setting:?} before the door and must be KEPT there, \
                 not retired and re-decoded because a gallery would have asked for less"
            );
            kept_as_they_were += 1;
            continue;
        }
        *tiers.entry(sheet.requested_tier).or_default() += 1;
    }
    let realized: usize = tiers.values().sum();
    assert!(
        realized >= 100,
        "only {realized} of {} realized in 200 frames",
        ids.len()
    );
    assert_eq!(
        tiers.keys().copied().collect::<Vec<_>>(),
        vec![TextureResolutionScale::Quarter],
        "the hall's newly realized sheets asked for {tiers:?}; a gallery caps its cast at \
         Quarter under a {setting:?} setting"
    );
    assert_eq!(kept_as_they_were, already_realized.len());
}

/// THE OTHER DIRECTION (asset open work 6): a character realized at the
/// gallery's Quarter and then needed in an uncapped room is re-tiered UP.
///
/// Boot straight into the hall (so its cast — including the characters the hub
/// also places — realizes at Quarter with nothing pre-realized at Full), record
/// the hall → hub transition through the room graph, and watch the shared
/// characters' sheets ask for the setting's tier while the cover is still down:
/// `PendingRoomTierFloor` names the destination's floor, and convergence
/// retires a Quarter sheet as too small for the room being loaded.
#[test]
fn leaving_the_gallery_re_tiers_the_shared_cast_up_to_the_setting() {
    use ambition_platformer2d::entity_catalog::placements::{InteractionKindSpec, PlacementSchema};
    use ambition_platformer2d::persistence::settings::TextureResolutionScale;
    use ambition_platformer2d::sprite_sheet::game_assets::GameAssets;

    const HUB: &str = "central_hub_complex";
    const HALL_EXIT_ZONE: &str = "hall_of_characters_entry";

    let mut app =
        ambition_app::app::build_visible_app_with(VisibleRenderMode::NoWindow, true, |app| {
            app.insert_resource(ambition_app::app::StartRoomOverride(
                "hall_of_characters".to_string(),
            ));
            app.insert_resource(ambition_app::app::StartRoomMustResolve);
        });
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));
    settle_cast(&mut app, 10);
    app.world_mut().write_message(ShellCommand::GoTo(
        shell_host::AMBITION_GAMEPLAY_ROUTE.into(),
    ));
    settle_cast(&mut app, 20);
    {
        let mut query = app
            .world_mut()
            .query::<&ambition_platformer2d::world::rooms::RoomSet>();
        let room_set = query.iter(app.world()).next().expect("a session room set");
        assert_eq!(
            room_set.active_spec().id,
            "hall_of_characters",
            "premise: the override started the session in the hall"
        );
    }
    let setting = app
        .world()
        .resource::<ambition_platformer2d::persistence::settings::UserSettings>()
        .video
        .quality
        .resolved_budget()
        .sprites
        .effective_scale();
    assert!(
        setting > TextureResolutionScale::Quarter,
        "premise: setting {setting:?} is above the cap"
    );
    for _ in 0..200 {
        step(&mut app);
    }

    // The characters the HUB places that the hall realized at Quarter.
    let hub_ids: std::collections::BTreeSet<String> = {
        let mut query = app
            .world_mut()
            .query::<&ambition_platformer2d::world::rooms::RoomSet>();
        let room_set = query.iter(app.world()).next().expect("a session room set");
        let hub = room_set
            .rooms
            .iter()
            .find(|room| room.id == HUB)
            .expect("the hub is in the room set");
        hub.placements
            .iter()
            .filter_map(|placement| match &placement.schema {
                PlacementSchema::Interactable(spec) => match &spec.kind {
                    InteractionKindSpec::Npc {
                        character_id: Some(id),
                        ..
                    } => Some(id.clone()),
                    _ => None,
                },
                _ => None,
            })
            .collect()
    };
    // The WORN character goes everywhere the player goes, so it is shared with
    // every room by construction; the hub's own placed cast joins it.
    let worn: String = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<
            &ambition_platformer2d::characters::actor::WornCharacter,
            bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
        >();
        q.iter(world)
            .next()
            .expect("the primary avatar wears a character")
            .0
            .as_str()
            .to_string()
    };
    let mut hall_ids = hall_character_ids(&mut app);
    hall_ids.push(worn.clone());
    hall_ids.sort();
    hall_ids.dedup();
    // The hall's cast the hub does NOT place: retired with the room, and never
    // re-decoded at Full for a room that has no use for it.
    let hall_only: Vec<String> = hall_ids
        .iter()
        .filter(|id| !hub_ids.contains(*id) && **id != worn)
        .cloned()
        .collect();
    assert!(
        hall_only.len() >= 50,
        "premise: the hall places a cast the hub does not"
    );
    let shared_at_quarter: Vec<String> = {
        let assets = app.world().resource::<GameAssets>();
        hall_ids
            .iter()
            .cloned()
            .filter(|id| hub_ids.contains(id) || *id == worn)
            .filter(|id| {
                assets
                    .characters
                    .sheet(id)
                    .is_some_and(|sheet| sheet.requested_tier == TextureResolutionScale::Quarter)
            })
            .collect()
    };
    assert!(
        !shared_at_quarter.is_empty(),
        "premise: no character the hub places was realized at Quarter in the hall, so \
         there is nothing to re-tier (hub cast {} ids)",
        hub_ids.len()
    );

    // Record the hall -> hub transition through the room graph.
    let (target_room, arrival) = {
        let mut query = app
            .world_mut()
            .query::<&ambition_platformer2d::world::rooms::RoomSet>();
        let room_set = query.iter(app.world()).next().expect("a session room set");
        let zone = room_set
            .active_loading_zones()
            .iter()
            .find(|zone| zone.id == HALL_EXIT_ZONE)
            .unwrap_or_else(|| panic!("the hall has no `{HALL_EXIT_ZONE}`"))
            .clone();
        let transition = room_set
            .transition_for_player(
                zone.aabb,
                ambition_platformer2d::engine_core::Vec2::ZERO,
                true,
            )
            .expect("the hall's exit resolves to a transition");
        (
            room_set.rooms[transition.target_room].id.clone(),
            transition.arrival,
        )
    };
    assert_eq!(
        target_room, HUB,
        "premise: the hall's exit leads to the hub"
    );
    let subject = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<
            &ambition_platformer2d::platformer::sim_id::SimId,
            bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
        >();
        q.iter(world).next().expect("a primary avatar").clone()
    };
    let _ = app.world_mut()
        .resource_mut::<ambition_platformer2d::actors::session::lifecycle_commit::PendingLifecycleCommit>()
        .record(
            0,
            ambition_platformer2d::actors::session::lifecycle_commit::LifecycleIntent::Transition(
                ambition_platformer2d::actors::session::lifecycle_commit::RoomTransitionIntent {
                    subject,
                    target_room,
                    arrival,
                    edge_exit: false,
                    zone_sfx: None,
                },
            ),
        );
    for _ in 0..300 {
        step(&mut app);
    }

    let assets = app.world().resource::<GameAssets>();
    let still_quarter: Vec<String> = shared_at_quarter
        .iter()
        .filter(|id| {
            assets
                .characters
                .sheet(id)
                .is_none_or(|sheet| sheet.requested_tier != setting)
        })
        .cloned()
        .collect();
    assert!(
        still_quarter.is_empty(),
        "{} of {} hub characters the hall realized at Quarter did not re-tier to {setting:?} \
         while the hub was being loaded: {still_quarter:?}",
        still_quarter.len(),
        shared_at_quarter.len()
    );
    // ⛔ AND THE GALLERY'S OWN CAST WAS NOT RE-DECODED AT FULL. Leaving retires
    // every Quarter sheet; only the hub's are re-demanded. Before the fix the
    // whole hall came back at Full — the entry hitch in reverse, and bigger.
    let promoted_for_nobody: Vec<String> = hall_only
        .iter()
        .filter(|id| {
            assets
                .characters
                .sheet(id)
                .is_some_and(|sheet| sheet.requested_tier == setting)
        })
        .cloned()
        .collect();
    // ⛔ MINUS THE HUB'S ONE-HOP NEIGHBOURS. The neighbour prefetch
    // (`prefetch_neighbor_room_preparation_system`) demands what the rooms
    // next door place, at THEIR tier, in the open, by design — so a gallery
    // character the basement also spawns comes back at Full for the basement,
    // not for nobody. Found the first time this fixture decoded real images:
    // `basement_enemies` spawns an "Ai Slop".
    let neighbour_ids: std::collections::BTreeSet<String> = {
        let neighbour_tokens: Vec<String> = {
            let mut query = app
                .world_mut()
                .query::<&ambition_platformer2d::world::rooms::RoomSet>();
            let room_set = query.iter(app.world()).next().expect("a session room set");
            assert_eq!(room_set.active_spec().id, HUB, "premise: the hub is active");
            room_set
                .neighboring_room_indices()
                .into_iter()
                .flat_map(|index| room_placed_character_tokens(&room_set.rooms[index]))
                .collect()
        };
        let registry = app
            .world()
            .resource::<ambition_platformer2d::character::PreparedCharacterRegistry>();
        let catalog = app
            .world()
            .resource::<ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog>();
        neighbour_tokens
            .iter()
            .map(|token| {
                ambition_platformer2d::actors::character_runtime::canonical_character_id(
                    registry, catalog, token,
                )
                .to_string()
            })
            .collect()
    };
    let promoted_for_nobody: Vec<String> = promoted_for_nobody
        .into_iter()
        .filter(|id| !neighbour_ids.contains(id))
        .collect();
    assert!(
        promoted_for_nobody.is_empty(),
        "{} hall-only characters were re-decoded at {setting:?} for a room that does not \
         place them: {:?}…",
        promoted_for_nobody.len(),
        promoted_for_nobody.iter().take(5).collect::<Vec<_>>()
    );

    // ⭐ AND THE RETIRED GALLERY CAST HAS LEFT MEMORY (asset open work 4: a
    // retained image must have an owner). The plugins are finished in this
    // composition, so images really decode and really drop: a hall-only
    // character's page still in `Assets<Image>` here would be held by
    // something other than its realization — the leak this exit exists to
    // catch. Pages the hub or its neighbours want are theirs to keep.
    // The rule: a resident character page belongs to a realization in the
    // table. Whatever demanded it — this room, a neighbour prefetch, a worn
    // identity — did so by realizing a sheet, and retiring the sheet drops
    // the page's last handle. A page resident with no realization is held by
    // something else, and that something is the leak.
    let owned_pages: std::collections::BTreeSet<String> = {
        let assets = app.world().resource::<GameAssets>();
        assets
            .characters
            .resident_sheets()
            .map(|(_, sheet)| sheet)
            .chain(assets.characters.props.values())
            .flat_map(|sheet| sheet.pages.iter())
            .filter_map(|page| page.texture.path().map(|path| path.to_string()))
            .collect()
    };
    let ledger = ambition_platformer2d::sprite_sheet::game_assets::image_stages::ledger();
    let leaked: Vec<String> = ledger
        .resident_rows()
        .filter(|row| row.source == Some("character-sheet"))
        .filter_map(|row| row.path.clone())
        .filter(|path| !owned_pages.contains(path))
        .collect();
    let resident_character_pages = ledger
        .resident_rows()
        .filter(|row| row.source == Some("character-sheet"))
        .count();
    assert!(
        resident_character_pages > 0,
        "premise: character pages decoded in this composition (the plugins are finished)"
    );
    assert!(
        leaked.is_empty(),
        "{} character page(s) are resident in the hub with no realization owning them: {:?}…",
        leaked.len(),
        leaked.iter().take(40).collect::<Vec<_>>()
    );
}

/// The characters a room places by name: NPC interactables and authored enemy
/// spawns, the two roads `room_character_tokens` demands from.
fn room_placed_character_tokens(
    room: &ambition_platformer2d::world::rooms::RoomSpec,
) -> Vec<String> {
    use ambition_platformer2d::entity_catalog::placements::{InteractionKindSpec, PlacementSchema};
    let mut tokens: Vec<String> = room
        .placements
        .iter()
        .filter_map(|placement| match &placement.schema {
            PlacementSchema::Interactable(spec) => match &spec.kind {
                InteractionKindSpec::Npc {
                    character_id: Some(id),
                    ..
                } => Some(id.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect();
    tokens.extend(room.enemy_spawns.iter().map(|enemy| enemy.name.clone()));
    tokens
}

/// PREFETCH SCOPE: does the transition's demand reach every character the room
/// PLACES, or only most of them?
///
/// The third of the three candidate causes for the reveal's 111 placeholder
/// warnings — the other two being retired realizations and re-decodes, both now
/// instrumented elsewhere. This one is the cheapest to answer and had never been
/// asked directly: the sibling tests count STAGED characters and assert a
/// minimum, which cannot distinguish "all 129 were demanded" from "126 were, and
/// three were never asked for at all".
///
/// ⛔ `outcome(id).is_none()` IS THE QUESTION. A character with any outcome —
/// Ready, pending, even failed — was reached by the demand. `None` means nothing
/// ever asked, which is a scope hole rather than a slow load, and no amount of
/// waiting fixes it.
#[test]
fn every_character_the_hall_places_is_reached_by_its_demand() {
    use ambition_platformer2d::actors::character_runtime::CharacterLoadStates;

    let (mut app, _before) = boot_and_record_the_hall_transition();
    let placed = hall_character_ids(&mut app);
    // The world authors 129 NpcSpawn placements with 129 DISTINCT character_ids
    // and no duplicates (counted from hall_of_characters.ldtk, 2026-09-02), so a
    // shortfall here is a missing character rather than a deduplicated one.
    assert!(
        placed.len() > 50,
        "the hall places only {} characters, so this test is measuring nothing",
        placed.len()
    );

    // Settle: the loader is rationed to one character per frame, so reaching a
    // 129-character cast needs at least that many frames even when nothing is
    // wrong. Waiting generously is correct HERE precisely because the assertion
    // is about scope rather than speed.
    for _ in 0..600 {
        step(&mut app);
    }

    let unreached: Vec<&String> = {
        let states = app.world().resource::<CharacterLoadStates>();
        placed
            .iter()
            .filter(|id| states.outcome(id).is_none())
            .collect()
    };
    assert!(
        unreached.is_empty(),
        "{} of the hall's {} placed characters were never reached by any demand \
         after 600 frames — nothing asked for them, so they cannot resolve by \
         waiting: {:?}",
        unreached.len(),
        placed.len(),
        unreached
    );
}

/// Record a transition through `zone` of the active room and step until the
/// target room is active (or `max` frames pass). Returns the frames it took.
fn transit_through(app: &mut App, zone_id: &str, max: usize) -> usize {
    let (target_room, arrival) = {
        let mut query = app
            .world_mut()
            .query::<&ambition_platformer2d::world::rooms::RoomSet>();
        let room_set = query.iter(app.world()).next().expect("a session room set");
        let zone = room_set
            .active_loading_zones()
            .iter()
            .find(|zone| zone.id == zone_id || zone.name == zone_id)
            .unwrap_or_else(|| {
                panic!(
                    "the active room '{}' has no `{zone_id}`",
                    room_set.active_spec().id
                )
            })
            .clone();
        let transition = room_set
            .transition_for_player(
                zone.aabb,
                ambition_platformer2d::engine_core::Vec2::ZERO,
                true,
            )
            .expect("the zone resolves to a transition");
        (
            room_set.rooms[transition.target_room].id.clone(),
            transition.arrival,
        )
    };
    let subject = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<
            &ambition_platformer2d::platformer::sim_id::SimId,
            bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
        >();
        q.iter(world).next().expect("a primary avatar").clone()
    };
    let _ = app.world_mut()
        .resource_mut::<ambition_platformer2d::actors::session::lifecycle_commit::PendingLifecycleCommit>()
        .record(
            0,
            ambition_platformer2d::actors::session::lifecycle_commit::LifecycleIntent::Transition(
                ambition_platformer2d::actors::session::lifecycle_commit::RoomTransitionIntent {
                    subject,
                    target_room: target_room.clone(),
                    arrival,
                    edge_exit: false,
                    zone_sfx: None,
                },
            ),
        );
    for frame in 0..max {
        step(app);
        let mut query = app
            .world_mut()
            .query::<&ambition_platformer2d::world::rooms::RoomSet>();
        let active = query
            .iter(app.world())
            .next()
            .map(|set| set.active_spec().id.clone());
        let state = app
            .world()
            .resource::<ambition_platformer2d::runtime::room_transition::RoomTransitionLoadState>(
        );
        if active.as_deref() == Some(target_room.as_str()) && state.active.is_none() {
            return frame;
        }
    }
    panic!("the transition to '{target_room}' did not complete in {max} frames");
}

/// What the character tables and the image ledger hold right now.
fn residency_snapshot(app: &App) -> (usize, usize, f64) {
    use ambition_platformer2d::sprite_sheet::game_assets::GameAssets;
    let assets = app.world().resource::<GameAssets>();
    let realizations = assets.characters.resident_sheets().count();
    let ledger = ambition_platformer2d::sprite_sheet::game_assets::image_stages::ledger();
    let (pages, mp) = ledger
        .resident_rows()
        .filter(|row| row.source == Some("character-sheet"))
        .fold((0usize, 0.0f64), |(n, mp), row| (n + 1, mp + row.megapixels));
    (realizations, pages, mp)
}

/// RESIDENCY GROWTH (asset open work 4, the "measure working-set growth"
/// half): two hub → hall → hub round trips, and the working set on each
/// return to the hub must be the SAME — realizations, character pages and
/// megapixels. A set that grows on the second lap is a retention nobody
/// owns: a realization the table keeps for a character no room places, or a
/// page held past its realization (the exit guard above covers the latter).
#[test]
fn two_round_trips_through_the_gallery_return_the_same_working_set() {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));
    settle_cast(&mut app, 10);
    app.world_mut().write_message(ShellCommand::GoTo(
        shell_host::AMBITION_GAMEPLAY_ROUTE.into(),
    ));
    settle_cast(&mut app, 20);
    for _ in 0..120 {
        step(&mut app);
    }
    let start = residency_snapshot(&app);
    let mut laps = Vec::new();
    for lap in 0..2 {
        let to_hall = transit_through(&mut app, HALL_DOOR_ZONE, 900);
        for _ in 0..120 {
            step(&mut app);
        }
        let in_hall = residency_snapshot(&app);
        let to_hub = transit_through(&mut app, "hall_of_characters_entry", 900);
        for _ in 0..300 {
            step(&mut app);
        }
        let back = residency_snapshot(&app);
        eprintln!(
            "[residency-lap {lap}] to hall in {to_hall} frames: {in_hall:?}; back in {to_hub} \
             frames: {back:?}"
        );
        laps.push(back);
    }
    eprintln!("[residency] at start {start:?}; after each lap {laps:?}");
    let (first, second) = (laps[0], laps[1]);
    assert_eq!(
        first.0, second.0,
        "the hub's resident REALIZATIONS grew between the first and second return \
         ({} → {}): the table keeps a character no room places",
        first.0, second.0
    );
    assert_eq!(
        first.1, second.1,
        "the hub's resident character PAGES grew between laps ({} → {})",
        first.1, second.1
    );
    assert!(
        (first.2 - second.2).abs() < 0.05,
        "resident character megapixels moved between laps ({:.1} → {:.1})",
        first.2, second.2
    );
}
