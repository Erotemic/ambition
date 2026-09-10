//! A9 AXIS TWO: which of the linked crates actually INSTALL anything here?
//!
//! ⭐⭐ THE ROW NAMES THREE AXES AND ONLY ONE HAS EVER BEEN MEASURED:
//! *"Separate compiler reachability, runtime installation and public-import
//! ergonomics."* The closure contract and `cargo tree` answer the first — 49
//! workspace crates including the facade, 48 besides it. **Neither can say
//! whether a linked crate does any work in this profile.**
//!
//! ⇒ A crate that is LINKED but INSTALLS NOTHING is a materially different
//! finding from one doing work: the first is dead weight a consumer pays for in
//! compile time and dependency surface and receives nothing for; the second is
//! the profile actually being made of something.
//!
//! ⛔ THIS REPORTS; IT DOES NOT RATCHET. A crate installing zero systems is not
//! automatically wrong — it may contribute types, assets, or a resource rather
//! than a system, and `install`-shaped work can be a `Plugin::build` that only
//! inserts resources. ⇒ The output is a subject list for the row, and calling it
//! a defect list would be the same over-reach as reading a closure number as a
//! carve mandate.

use ambition_platformer2d::app::prelude::*;
use ambition_platformer2d::bevy::ecs::schedule::Schedules;
use ambition_platformer2d::bevy::prelude::*;
use headless_profile::HeadlessModule as HeadlessModuleRef;

/// `ambition_combat::foo::bar` -> `ambition_combat`; anything else -> `<other>`.
fn owning_crate(name: &str) -> String {
    let head = name.split('<').next().unwrap_or(name);
    let head = head.trim_start_matches('(');
    match head.split("::").next() {
        Some(c) if c.starts_with("ambition_") => c.to_string(),
        _ => "<non-ambition>".to_string(),
    }
}

#[test]
fn which_linked_crates_install_systems_in_the_minimum_profile() {
    let mut app = PlatformerApp::headless()
        .mount(HeadlessModuleRef::default())
        .try_build()
        .expect("the A9 minimum profile composes headless");

    // Session activation installs systems over several frames; a census taken at
    // build time would undercount and read as a cleaner result than the truth.
    for _ in 0..8 {
        app.update();
    }

    let mut by_crate: std::collections::BTreeMap<String, usize> = Default::default();
    let mut schedules_seen = 0usize;
    let mut skipped = 0usize;
    let mut samples: Vec<String> = Vec::new();
    let world = app.world_mut();
    world.resource_scope(|_world, mut schedules: Mut<Schedules>| {
        // `Schedule::label()` hands back an interned label; the `&dyn` one from
        // `iter()` cannot be used as a key.
        let labels: Vec<_> = schedules.iter().map(|(_, s)| s.label()).collect();
        for label in labels {
            let Some(schedule) = schedules.get_mut(label) else {
                continue;
            };
            // ⛔ DELIBERATELY NOT `initialize`d HERE. Building the graph inserts a
            // resource, and doing that inside `resource_scope` on `Schedules` is
            // what Bevy panics about — but the honest reason is better than the
            // mechanical one: a schedule that has never RUN has installed nothing
            // that ever executed, and axis two is about runtime installation. So
            // this censuses the graphs the profile actually built by running, and
            // reports how many it could not read rather than forcing them.
            let Ok(systems) = schedule.systems() else {
                skipped += 1;
                continue;
            };
            schedules_seen += 1;
            for (_key, system) in systems {
                let raw = system.name().to_string();
                if samples.len() < 8 && !samples.contains(&raw) {
                    samples.push(raw.clone());
                }
                *by_crate.entry(owning_crate(&raw)).or_default() += 1;
            }
        }
    });

    let total: usize = by_crate.values().sum();
    // ⛔ ANTI-VACUITY FLOOR, FIRST. Zero schedules or zero systems means the
    // census ran against a drained graph and every "installs nothing" below
    // would be an artefact of the instrument rather than a fact about a crate.
    assert!(
        schedules_seen > 0 && total > 0,
        "censused {schedules_seen} schedules and {total} systems — the graph was \
         not readable, so no conclusion about any crate is available"
    );

    for sample in &samples {
        eprintln!("[a9-axis2] raw-name-sample: {sample}");
    }
    // ⛔⛔ THE FLOOR THAT MATTERED WAS NOT THE ONE I WROTE FIRST. The original
    // asserted `schedules > 0 && systems > 0` and then printed
    // "ambition_crates_installing=0 of 48 linked" as though it were a RESULT.
    // 557 systems were found and NONE could be attributed — a broken instrument
    // that reads exactly like a profile where no engine crate installs anything,
    // which is the most alarming possible finding and would have been entirely
    // an artefact. A floor must cover the step that can silently fail, and the
    // step that can silently fail here is ATTRIBUTION, not collection.
    const STRIPPED: &str = "<Enable the debug feature to see the name>";
    let names_stripped = samples.iter().all(|s| s == STRIPPED);
    for sample in samples.iter().take(3) {
        eprintln!("[a9-axis2] raw-name-sample: {sample}");
    }

    if names_stripped {
        // ⛔⛔ THE NAME IS NOT UNREADABLE, IT IS NEVER STORED. Traced 2026-09-10:
        // `bevy_utils::DebugName` has NO FIELD AT ALL without `bevy_utils/debug`
        // (`debug_info.rs`), so the string is discarded at construction and no
        // runtime accessor can recover it. `TypeId` survives and carries no crate.
        // ⇒ Attribution without `System::name()` is not merely hard here; the data
        // does not exist.
        //
        // ⭐ AND WHAT ENABLES IT IN THE FULL BUILD NAMES THE INCONSISTENCY:
        // `bevy_dev_tools` requires `bevy_utils/debug`, and the minimum profile
        // does not link `bevy_dev_tools`. Measured with `cargo tree -e features`:
        // the workspace has `bevy_utils feature "debug"`, the featureless facade
        // does not. Meanwhile OUR `ambition_dev_tools` IS in the minimum profile --
        // facade-direct and non-optional -- and its census attributes systems BY
        // NAME. So a mandatory diagnostics crate sits in a profile that cannot
        // supply the one fact it reads.
        //
        // ⚠ The remedy is a RULING, not a repair: should the facade pull
        // `bevy/debug` because it always links `ambition_dev_tools`? That trades
        // binary size against diagnosability at the profile the SDK offers, and it
        // is not this fixture's to decide. Adding a feature HERE is forbidden --
        // the fixture's whole value is that everything it links arrives implicitly.
        // ⭐⭐ THIS IS THE FINDING, NOT A SKIP. `bevy_ecs`'s `debug` feature is
        // off in the featureless closure, so EVERY system in the minimum profile
        // reports the same placeholder name. The profile links
        // `ambition_dev_tools` (a FACADE-DIRECT, non-optional dependency) and
        // that crate's own census attributes systems BY NAME -- so the minimum
        // profile carries a diagnostics crate that cannot diagnose it.
        //
        // ⇒ Axis two is not answerable by name at this profile. Answering it
        // needs either a feature this fixture is forbidden to add without a
        // planning row, or an attribution that does not go through
        // `System::name()`. Both are decisions, and this records the wall rather
        // than reporting a zero that looks like an answer.
        eprintln!(
            "[a9-axis2] VERDICT unattributable: {total} systems across \
             {schedules_seen} built schedules, every name stripped. The minimum \
             profile is not introspectable by system name."
        );
        assert!(
            total > 100,
            "names are stripped AND only {total} systems exist -- that is not the \
             known debug-feature wall, it is a second failure underneath it"
        );
        return;
    }

    let ambition: Vec<(&String, &usize)> = by_crate
        .iter()
        .filter(|(c, _)| c.starts_with("ambition_"))
        .collect();
    assert!(
        !ambition.is_empty(),
        "names are readable but NOT ONE of {total} systems attributed to an \
         `ambition_*` crate -- the parser is wrong, and a zero here would print \
         as the alarming finding it is not"
    );
    eprintln!(
        "[a9-axis2] schedules_read={schedules_seen} schedules_unbuilt={skipped} \
         systems={total} ambition_crates_installing={} of 48 linked",
        ambition.len()
    );
    for (c, n) in &ambition {
        eprintln!("[a9-axis2]   {n:>5}  {c}");
    }
}
