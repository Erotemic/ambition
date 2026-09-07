//! How much of `Update` is even addressable by set-level gating.
//!
//! 311 counted CALL SITES, and one `add_systems` registers many systems.
//! The runtime population is what the executor pays for, so that is what this
//! measures.
//!
//! ## Why "systems in no set" is the number worth having first
//!
//! The proposed fix is set-level: gate a SET by session phase. A system that belongs to no set
//! cannot be gated that way — it would need its own condition, one at a time, which is a
//! different (and much larger) piece of work.
//!
//! Set MEMBERSHIP can be, and it bounds the answer from above: an unsetted system is certainly not
//! set-gated.

use bevy::ecs::schedule::graph::Direction;
use bevy::ecs::schedule::{NodeId, ScheduleLabel, Schedules};
use bevy::prelude::*;

/// Systems in the schedule, split into (in at least one AUTHORED set, in none),
/// plus a per-crate tally of the UNSETTED ones.
fn set_membership(
    app: &mut App,
    label: impl ScheduleLabel,
) -> (usize, usize, Vec<(String, usize)>) {
    let label = label.intern();
    app.world_mut()
        .resource_scope(|world, mut schedules: Mut<Schedules>| {
            let schedule = schedules.get_mut(label).expect("the schedule exists");
            // The graph is built lazily on first run; a schedule that has never
            // run reports no structure at all.
            let _ = schedule.initialize(world);
            let graph = schedule.graph();
            let hierarchy = graph.hierarchy().graph();
            let mut in_a_set = 0usize;
            let mut orphan = 0usize;
            let mut by_crate: std::collections::BTreeMap<String, usize> =
                std::collections::BTreeMap::new();
            for (key, system) in schedule
                .systems()
                .expect("initialized above, so the systems are enumerable")
            {
                // Exclude Bevy's automatic `SystemTypeSet`; this census measures
                // authored schedule structure, not per-system bookkeeping sets.
                // for row after row.
                //
                // and this is done STRUCTURALLY, without naming anything.
                // `get_node_name` on a set walks its members to render an
                // anonymous set's name, and PANICS if the hierarchy still
                // references a system the schedule no longer holds — which this
                // app's graph does. A `SystemTypeSet` is exactly the set whose
                // only member is its own system, so "a parent with more than one
                // member" identifies an authored grouping without asking any
                // node what it is called.
                //
                // the known error: a genuinely authored set with exactly ONE
                // member is counted as unsetted. That biases the number DOWN, so
                // the conclusion it supports (how much is addressable) is
                // conservative rather than flattering.
                let authored_parents = hierarchy
                    .neighbors_directed(NodeId::System(key), Direction::Incoming)
                    .filter(|parent| matches!(parent, NodeId::Set(_)))
                    .filter(|parent| {
                        hierarchy
                            .neighbors_directed(*parent, Direction::Outgoing)
                            .count()
                            > 1
                    })
                    .count();
                if authored_parents > 0 {
                    in_a_set += 1;
                } else {
                    orphan += 1;
                    // the SYSTEM's own name, not `get_node_name` — see above
                    // for why naming a node can panic on this graph.
                    let name = format!("{}", system.name());
                    let owner = name
                        .split("::")
                        .next()
                        .unwrap_or("<unknown>")
                        .rsplit(' ')
                        .next()
                        .unwrap_or("<unknown>")
                        .to_string();
                    *by_crate.entry(owner).or_default() += 1;
                }
            }
            let mut tally: Vec<(String, usize)> = by_crate.into_iter().collect();
            tally.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            (in_a_set, orphan, tally)
        })
}

/// The census, on the shipped composition rather than a fixture.
///
/// Printed rather than pinned to an exact number: this is a MEASUREMENT that
/// should move, and a test that fails whenever a system is added would be
/// deleted within a week. The assertion is only that the schedule is still large
/// enough for the question to matter, so the print is not silently measuring an
/// empty app.
#[test]
fn census_of_how_much_of_update_is_inside_a_set() {
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    for _ in 0..4 {
        app.update();
    }

    let (update_in_set, update_orphan, update_tally) = set_membership(&mut app, Update);
    let (ggrs_in_set, ggrs_orphan, _) =
        set_membership(&mut app, ambition_platformer2d::rollback::GgrsSchedule);

    let update_total = update_in_set + update_orphan;
    let ggrs_total = ggrs_in_set + ggrs_orphan;
    eprintln!(
        "[update-census] Update: {update_total} systems — {update_in_set} in a set, \
         {update_orphan} in NONE ({:.0}% unsetted)",
        100.0 * update_orphan as f32 / update_total.max(1) as f32
    );
    eprintln!(
        "[update-census] GgrsSchedule: {ggrs_total} systems — {ggrs_in_set} in a set, \
         {ggrs_orphan} in NONE ({:.0}% unsetted)",
        100.0 * ggrs_orphan as f32 / ggrs_total.max(1) as f32
    );

    for (owner, count) in update_tally.iter().take(15) {
        eprintln!("[update-census]   unsetted in `Update`: {count:>4}  {owner}");
    }

    assert!(
        update_total > 100,
        "the shipped app's `Update` should carry hundreds of systems; {update_total} \
         means this measured something other than the real composition"
    );
}

/// Every set that reads `MenuControlFrame` must have members in the same schedule.
/// A `.before` edge to a set in another schedule is vacuous, so set existence
/// alone is insufficient.
#[test]
fn the_menu_frame_reader_sets_are_co_scheduled() {
    use ambition_platformer2d::actors::schedule::{
        MenuFrameConsume, MenuFrameCutsceneSkip, MenuNavConsume,
    };

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    for _ in 0..4 {
        app.update();
    }

    let labels: Vec<bevy::ecs::schedule::InternedScheduleLabel> = app
        .world()
        .resource::<Schedules>()
        .iter()
        .map(|(_, schedule)| schedule.label())
        .collect();

    // Schedules in which each set has AT LEAST ONE member system.
    let mut cutscene_in: Vec<String> = Vec::new();
    let mut nav_in: Vec<String> = Vec::new();
    let mut umbrella_in: Vec<String> = Vec::new();

    /// How many members a set has in this graph. ZERO for a set that was never
    /// registered here AND for one registered as an empty node — the two are
    /// the same fact for a `.before` pinned against it.
    fn members<S: bevy::ecs::schedule::SystemSet>(
        graph: &bevy::ecs::schedule::ScheduleGraph,
        set: S,
    ) -> usize {
        let Some(key) = graph.system_sets.get_key(set.intern()) else {
            return 0;
        };
        graph
            .hierarchy()
            .graph()
            .neighbors_directed(NodeId::Set(key), Direction::Outgoing)
            .count()
    }

    for label in labels {
        app.world_mut()
            .resource_scope(|world, mut schedules: Mut<Schedules>| {
                let schedule = schedules.get_mut(label).expect("label came from the map");
                let _ = schedule.initialize(world);
                let graph = schedule.graph();
                let name = format!("{label:?}");
                if members(graph, MenuFrameCutsceneSkip) > 0 {
                    cutscene_in.push(name.clone());
                }
                if members(graph, MenuNavConsume) > 0 {
                    nav_in.push(name.clone());
                }
                if members(graph, MenuFrameConsume) > 0 {
                    umbrella_in.push(name);
                }
            });
    }
    cutscene_in.sort();
    nav_in.sort();
    umbrella_in.sort();
    println!("cutscene-skip readers in: {cutscene_in:?}");
    println!("nav readers in:           {nav_in:?}");
    println!("MenuFrameConsume in:      {umbrella_in:?}");

    assert!(
        !cutscene_in.is_empty() && !nav_in.is_empty(),
        "a menu-frame reader set has no members in ANY schedule, so every \
         `.before` pinned against it is already vacuous — cutscene: \
         {cutscene_in:?}, nav: {nav_in:?}",
    );
    assert_eq!(
        cutscene_in, nav_in,
        "the two sets that read `MenuControlFrame` are populated in DIFFERENT \
         schedules, so a writer cannot land before both by pinning both: one of \
         the two `.before`s is silently doing nothing. cutscene-skip in \
         {cutscene_in:?}, nav in {nav_in:?}",
    );
    assert_eq!(
        umbrella_in, nav_in,
        "`MenuFrameConsume` — the ONE name a frame writer pins against — is not \
         populated in the same schedule as the readers it is supposed to \
         contain. That is not a loud failure anywhere else: the pin still \
         compiles, the node is still created, and it constrains nothing. \
         umbrella in {umbrella_in:?}, readers in {nav_in:?}",
    );
}

/// A `.before` pinned against an umbrella orders EVERY member.
///
/// The property `MenuFrameConsume` rests on, proven on a three-system app rather than assumed from
/// Bevy's docs.
///
/// deliberately NOT the shipped app: a behavioural claim about a Bevy
/// mechanism is answered by exercising the mechanism, and a full composition
/// would let a hundred other constraints produce the same order by accident.
#[test]
fn a_before_pinned_against_an_umbrella_orders_every_member() {
    #[derive(bevy::ecs::schedule::SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
    struct Umbrella;
    #[derive(bevy::ecs::schedule::SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
    struct MemberOne;
    #[derive(bevy::ecs::schedule::SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
    struct MemberTwo;

    #[derive(Resource, Default)]
    struct Order(Vec<&'static str>);

    let mut app = App::new();
    app.init_resource::<Order>();
    app.configure_sets(Update, (MemberOne, MemberTwo).in_set(Umbrella));
    app.add_systems(
        Update,
        (
            (|mut order: ResMut<Order>| order.0.push("reader_one")).in_set(MemberOne),
            (|mut order: ResMut<Order>| order.0.push("reader_two")).in_set(MemberTwo),
            // The writer names the umbrella ONLY.
            (|mut order: ResMut<Order>| order.0.push("writer")).before(Umbrella),
        ),
    );
    app.update();

    let order = &app.world().resource::<Order>().0;
    assert_eq!(
        order.first(),
        Some(&"writer"),
        "a system pinned `.before` an umbrella did not run before its members — \
         every `.before(MenuFrameConsume)` in the tree is then decorative. \
         Observed order: {order:?}"
    );
    assert_eq!(order.len(), 3, "all three systems must have run: {order:?}");
}

/// Readers of `MenuControlFrame` must be ordered AGAINST EACH OTHER, not just
/// after the writer.
///
/// `MenuFrameConsume` orders WRITER → readers, and
/// [`a_before_pinned_against_an_umbrella_orders_every_member`] proves that edge
/// is real. Neither says anything about reader vs READER — and one reader
/// mutates: `MenuControlFrame::consume_nav_edges()` clears
/// `up`/`down`/`left`/`right`/`select`/`back`/`page_*` for every later reader in
/// ANY crate, and two open-routing systems additionally set `menu.back = false`.
/// Two readers with no edge between them run in an order Bevy does not fix, so
/// which surface eats a press is emergent rather than stated.
///
/// This asks Bevy's own detector instead of grepping registration sites. A grep
/// over `add_systems` lines cannot see an `.in_set` / `configure_sets` edge
/// declared in another crate — that is exactly the half this file already had to
/// learn once — and `conflicting_systems()` is computed unconditionally in
/// `build_schedule` (before the `ambiguity_detection` setting is consulted), so
/// it is available without turning warnings on.
///
/// ⚠ Reported by RESOURCE, never by system name: `get_node_name` panics on this
/// app's graph, and Bevy strips system names without its `debug` feature.
/// The member systems of one authored set in this graph, as keys.
///
/// The inverse of the `members` count in
/// [`the_menu_frame_reader_sets_are_co_scheduled`]: that asks whether a set is
/// populated at all, this asks WHICH systems are in it, so a conflicting pair can
/// be attributed without naming anything.
fn set_members<S: bevy::ecs::schedule::SystemSet>(
    graph: &bevy::ecs::schedule::ScheduleGraph,
    set: S,
) -> std::collections::HashSet<bevy::ecs::schedule::SystemKey> {
    let Some(key) = graph.system_sets.get_key(set.intern()) else {
        return std::collections::HashSet::new();
    };
    graph
        .hierarchy()
        .graph()
        .neighbors_directed(NodeId::Set(key), Direction::Outgoing)
        .filter_map(|n| match n {
            NodeId::System(k) => Some(k),
            NodeId::Set(_) => None,
        })
        .collect()
}

type ConflictCensus = (
    usize,
    usize,
    usize,
    std::collections::BTreeMap<String, usize>,
);

fn menu_frame_conflicts(app: &mut App) -> ConflictCensus {
    use ambition_platformer2d::actors::schedule::{
        MenuFrameConsume, MenuFrameCutsceneSkip, MenuNavConsume,
    };
    let menu_id = app
        .world()
        .components()
        .component_id::<ambition_platformer2d::input::MenuControlFrame>()
        .expect("the shipped app registers MenuControlFrame");

    let labels: Vec<bevy::ecs::schedule::InternedScheduleLabel> = app
        .world()
        .resource::<Schedules>()
        .iter()
        .map(|(_, schedule)| schedule.label())
        .collect();

    let mut on_menu_frame = 0usize;
    let mut total = 0usize;
    // Which authored menu set each side of a conflicting pair belongs to.
    // System NAMES are unavailable (stripped without bevy's `debug` feature, and
    // `get_node_name` panics on this graph), but SET MEMBERSHIP is structural and
    // says the actionable thing: a side in no menu set is one that never joined
    // the umbrella, which is the fix.
    let mut by_membership: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    // An EXCLUSIVE system's conflict is recorded with an EMPTY id list
    // (bevy_ecs schedule/node.rs: `is_exclusive()` pushes `Box::new([])`), so a
    // filter on "the ids contain MenuControlFrame" cannot see it. Counted
    // separately rather than silently dropped — this is the blind half of the
    // question, not a zero.
    let mut unattributed = 0usize;

    for label in labels {
        app.world_mut()
            .resource_scope(|world, mut schedules: Mut<Schedules>| {
                let schedule = schedules.get_mut(label).expect("label came from the map");
                let _ = schedule.initialize(world);
                let conflicts = schedule.graph().conflicting_systems();
                let graph = schedule.graph();
                let nav = set_members(graph, MenuNavConsume);
                let cutscene = set_members(graph, MenuFrameCutsceneSkip);
                let umbrella = set_members(graph, MenuFrameConsume);
                let where_is = |k: &bevy::ecs::schedule::SystemKey| -> &'static str {
                    if nav.contains(k) {
                        "MenuNavConsume"
                    } else if cutscene.contains(k) {
                        "MenuFrameCutsceneSkip"
                    } else if umbrella.contains(k) {
                        "MenuFrameConsume(direct)"
                    } else {
                        "NO MENU SET"
                    }
                };
                for (a, b, ids) in &conflicts.0 {
                    total += 1;
                    if ids.is_empty() {
                        unattributed += 1;
                    } else if ids.contains(&menu_id) {
                        on_menu_frame += 1;
                        let (mut x, mut y) = (where_is(a), where_is(b));
                        if x > y {
                            std::mem::swap(&mut x, &mut y);
                        }
                        *by_membership.entry(format!("{x}  ×  {y}")).or_default() += 1;
                    }
                }
            });
    }
    (on_menu_frame, total, unattributed, by_membership)
}

/// POSITIVE CONTROL: the detector actually fires on the shape being asked about.
///
/// Two systems that both hold `ResMut<MenuControlFrame>` with no ordering edge —
/// the literal defect — on a hand-built app, so a zero on the shipped app is a
/// fact about the shipped app and not about a detector that never reports
/// anything.
#[test]
fn the_menu_frame_conflict_detector_reports_an_unordered_pair() {
    let mut app = App::new();
    app.init_resource::<ambition_platformer2d::input::MenuControlFrame>();
    app.add_systems(
        Update,
        (
            |mut f: ResMut<ambition_platformer2d::input::MenuControlFrame>| f.back = false,
            |mut f: ResMut<ambition_platformer2d::input::MenuControlFrame>| f.start = false,
        ),
    );
    app.update();

    let (on_menu_frame, _, _, _) = menu_frame_conflicts(&mut app);
    assert!(
        on_menu_frame >= 1,
        "two unordered `ResMut<MenuControlFrame>` systems must be reported as \
         conflicting; got {on_menu_frame}. The detector is not answering the \
         question this file asks it, so every zero it reports is vacuous."
    );
}

/// NEGATIVE CONTROL: an ordering edge is what clears it.
///
/// The same two systems, chained. Pins that the detector is reporting ORDER and
/// not merely "two systems touch one resource" — without this, the positive
/// control above would pass for a detector that flags every shared resource.
#[test]
fn ordering_the_pair_clears_the_menu_frame_conflict() {
    let mut app = App::new();
    app.init_resource::<ambition_platformer2d::input::MenuControlFrame>();
    app.add_systems(
        Update,
        (
            (|mut f: ResMut<ambition_platformer2d::input::MenuControlFrame>| f.back = false),
            (|mut f: ResMut<ambition_platformer2d::input::MenuControlFrame>| f.start = false),
        )
            .chain(),
    );
    app.update();

    let (on_menu_frame, _, _, _) = menu_frame_conflicts(&mut app);
    assert_eq!(
        on_menu_frame, 0,
        "chaining the pair must clear the conflict; got {on_menu_frame}"
    );
}

/// The shipped composition, measured with the controlled instrument above.
#[test]
fn menu_frame_readers_are_ordered_against_each_other_in_the_shipped_app() {
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    for _ in 0..4 {
        app.update();
    }

    let (on_menu_frame, total, unattributed, by_membership) = menu_frame_conflicts(&mut app);
    eprintln!(
        "[menu-frame] {on_menu_frame} unordered pairs conflict on MenuControlFrame \
         ({total} conflicting pairs in all schedules, {unattributed} unattributable \
         because an exclusive system reports no ids)"
    );

    // ANTI-VACUITY: a zero is only a fact about the app if the detector saw the
    // app at all. A composition missing the menu plugins, or a graph that never
    // got built, reports zero for the wrong reason.
    assert!(
        total > 0,
        "no schedule reported ANY conflicting pair — the graph was not built or \
         this measured an empty composition, so the MenuControlFrame zero below \
         would be vacuous"
    );

    for (where_, count) in &by_membership {
        eprintln!("[menu-frame]   {count:>3}  {where_}");
    }

    // ⚠ The per-bucket breakdown is a HINT, not an accounting. Ordering the two
    // backends' nav dropped the total 11 -> 9 (the `MenuNavConsume x
    // MenuNavConsume` row, poison-verified by removing the one `configure_sets`
    // and watching 11 come back), but two of the REMAINING pairs also changed
    // bucket, which adding an ordering edge cannot cause. Membership is resolved
    // per schedule, so a system can label differently in two of them. The TOTAL is
    // exact and reproducible; trust that and re-derive a bucket before acting on it.
    //
    // A RATCHET, not a pin at zero: the eleven that exist are a real finding and
    // fixing them is per-surface judgement (which context each menu should
    // declare), but a TWELFTH must not be able to land silently. Lower this number
    // when you order a pair; never raise it.
    //
    // Deliberately not `assert_eq!(.., 0)`: that would leave the suite red while
    // the audit is open, and a permanently-red guard stops being read.
    // ⭐ THE NINE ARE FULLY ACCOUNTED FOR, 2026-09-07, and the arithmetic closes:
    // eight are `MenuNavConsume x NO MENU SET` and the ninth is
    // `MenuFrameCutsceneSkip x MenuNavConsume`, which the host declares
    // deliberate. FOUR readers of `MenuControlFrame` declare no menu set at all
    // -- `ambition_dialog::dialog_input`, `ambition_menu::map::handle_map_menu_hotkeys`,
    // and `ambition_game_shell`'s `basic_shell_menu_intent` and
    // `drive_shell_pause_menu` -- against two of the nav consumers. 4 x 2 = 8.
    //
    // ⚠ THE DIAGNOSTIC BELOW CAN ONLY NAME TWO OF THE FOUR. The `game_shell` pair
    // are private `fn` in a crate with no `pub fn` in either file, so they have no
    // nameable `SystemTypeSet` path and print as `<other> x <other>`. They were
    // found by grepping every `MenuControlFrame` parameter in the tree instead --
    // including the `Option<Res<..>>` shape a `Res<MenuControlFrame>` grep misses.
    // ⇒ Do NOT read four unnamed pairs as four unknown systems: the population is
    // enumerable from source even where it is not nameable from a test.
    //
    // Whether these matter is `awaiting-maintainer-decision.md` Q75 -- it turns on
    // whether two of those surfaces can be live in one frame, which is Jon's to say.
    const KNOWN_UNORDERED_MENU_PAIRS: usize = 9;
    assert!(
        on_menu_frame <= KNOWN_UNORDERED_MENU_PAIRS,
        "{on_menu_frame} pairs of systems touch `MenuControlFrame` with no ordering \
         edge between them, up from the known {KNOWN_UNORDERED_MENU_PAIRS}. A new \
         menu-frame system landed without joining `MenuFrameConsume` (or chaining \
         against the readers). One reader calls `consume_nav_edges()`, which clears \
         the nav edges for every later reader in ANY crate, so which surface acts on \
         a press is then decided by an order Bevy does not fix. Membership of the \
         conflicting pairs: {by_membership:?}"
    );
}

/// DIAGNOSTIC: name the systems in each conflicting pair.
///
/// Bevy gives every system its own `SystemTypeSet`, so a NAMED function can be
/// looked up as a set and its `SystemKey` recovered — which is how a report can
/// name who without the `debug` feature and without `get_node_name` (which panics
/// on this graph).
#[test]
fn name_the_menu_frame_conflicts() {
    use bevy::ecs::schedule::IntoSystemSet;
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    for _ in 0..4 {
        app.update();
    }
    let menu_id = app
        .world()
        .components()
        .component_id::<ambition_platformer2d::input::MenuControlFrame>()
        .expect("registered");

    let labels: Vec<bevy::ecs::schedule::InternedScheduleLabel> = app
        .world()
        .resource::<Schedules>()
        .iter()
        .map(|(_, s)| s.label())
        .collect();

    let mut resolved = 0usize;
    for label in labels {
        app.world_mut()
            .resource_scope(|world, mut schedules: Mut<Schedules>| {
                let schedule = schedules.get_mut(label).expect("exists");
                let _ = schedule.initialize(world);
                let graph = schedule.graph();
                let mut named: Vec<(&str, std::collections::HashSet<bevy::ecs::schedule::SystemKey>)> =
                    Vec::new();
                macro_rules! name {
                    ($n:expr, $s:expr) => {
                        named.push(($n, set_members(graph, $s.into_system_set())));
                    };
                }
                name!("dialog_input", ambition_platformer2d::dialog::dialog_input);
                name!("fold_touch_gestures", ambition_platformer2d::touch_input::menu_bridge::fold_touch_gestures);
                name!("handle_map_menu_hotkeys", ambition_platformer2d::menu::map::handle_map_menu_hotkeys);
                name!("apply_menu_frame_to_cutscene_request", ambition_platformer2d::actors::schedule::apply_menu_frame_to_cutscene_request);
                name!("populate_menu_control_frame_from_actions", ambition_platformer2d::actors::schedule::populate_menu_control_frame_from_actions);

                let who = |k: &bevy::ecs::schedule::SystemKey| -> String {
                    for (n, keys) in &named {
                        if keys.contains(k) {
                            return (*n).to_string();
                        }
                    }
                    "<other>".to_string()
                };
                for (a, b, ids) in &graph.conflicting_systems().0 {
                    if ids.contains(&menu_id) {
                        let (x, y) = (who(a), who(b));
                        if x != "<other>" || y != "<other>" {
                            resolved += 1;
                        }
                        eprintln!("[named] {label:?}: {x} x {y}");
                    }
                }
            });
    }

    // ANTI-VACUITY: if `into_system_set()` ever stopped resolving, every pair
    // would print `<other> x <other>` and this diagnostic would be silently
    // useless while still passing. At least one side must be named.
    //
    // Not pinned to WHICH systems: that would freeze today's shape, which is the
    // thing the ratchet above is meant to let us change.
    assert!(
        resolved > 0,
        "no conflicting pair could be attributed to a named system — the \
         SystemTypeSet lookup stopped working, so this diagnostic is reporting \
         nothing while appearing to pass"
    );
}

/// Set MEMBERSHIP is not an ORDER between siblings.
///
/// This decides whether "put the remaining readers in `MenuFrameConsume`" is a
/// fix or a no-op, so it is answered by exercising the mechanism rather than
/// assumed from the umbrella's name. `MenuFrameConsume` gives a frame WRITER one
/// pin that covers every reader; it does not arrange the readers among
/// themselves, and two direct members still race.
///
/// ⇒ The remaining pairs therefore need an explicit order (a chain, or nested
/// sets), which is the per-surface judgement D-MENU-CHANNEL describes — not
/// merely membership.
#[test]
fn two_direct_members_of_one_set_are_still_unordered() {
    #[derive(bevy::ecs::schedule::SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
    struct Umbrella;

    let mut app = App::new();
    app.init_resource::<ambition_platformer2d::input::MenuControlFrame>();
    app.add_systems(
        Update,
        (
            (|mut f: ResMut<ambition_platformer2d::input::MenuControlFrame>| f.back = false)
                .in_set(Umbrella),
            (|mut f: ResMut<ambition_platformer2d::input::MenuControlFrame>| f.start = false)
                .in_set(Umbrella),
        ),
    );
    app.update();

    let (on_menu_frame, _, _, _) = menu_frame_conflicts(&mut app);
    assert!(
        on_menu_frame >= 1,
        "two systems in the SAME set reported no conflict, so membership alone \
         orders them — if that were true, joining `MenuFrameConsume` would be \
         enough for the remaining readers and this file's advice is wrong"
    );
}
