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
    let (ggrs_in_set, ggrs_orphan, _) = set_membership(
        &mut app,
        ambition_platformer2d::rollback::GgrsSchedule,
    );

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
