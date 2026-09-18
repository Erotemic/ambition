"""Unit tests for `scripts/multi_writer_resource_census.py` — hand-built corpus."""

from __future__ import annotations

import importlib.util
import pathlib

_SCRIPT = pathlib.Path(__file__).resolve().parents[1] / "multi_writer_resource_census.py"
_spec = importlib.util.spec_from_file_location("multi_writer_resource_census", _SCRIPT)
mod = importlib.util.module_from_spec(_spec)
assert _spec.loader is not None
_spec.loader.exec_module(mod)


def _write(tmp_path, name: str, body: str) -> str:
    p = tmp_path / name
    p.write_text(body, encoding="utf-8")
    return str(p)


def test_two_files_writing_one_resource_are_reported(tmp_path):
    a = _write(tmp_path, "a.rs", "fn s(mut r: ResMut<Foo>) {}")
    b = _write(tmp_path, "b.rs", "fn t(mut r: ResMut<Foo>) {}")
    found = mod.writers([a, b])
    assert found["Foo"] == {a, b}


def test_a_qualified_path_is_the_same_resource_as_its_short_name(tmp_path):
    """⚠ Otherwise one crate's `ResMut<crate::Foo>` and another's `ResMut<Foo>`
    read as two different types and the duplication hides."""
    a = _write(tmp_path, "a.rs", "fn s(mut r: ResMut<ambition_x::y::Foo>) {}")
    b = _write(tmp_path, "b.rs", "fn t(mut r: ResMut<Foo>) {}")
    assert len(mod.writers([a, b])["Foo"]) == 2


def test_a_test_module_is_not_a_second_authority(tmp_path):
    """⛔ A fixture building a resource by hand is not a writer. Counting them is
    how a census manufactures findings nobody can act on."""
    a = _write(tmp_path, "a.rs", "fn s(mut r: ResMut<Foo>) {}")
    b = _write(
        tmp_path,
        "b.rs",
        "fn unrelated() {}\n#[cfg(test)]\nmod t { fn f(mut r: ResMut<Foo>) {} }\n",
    )
    found = mod.writers([a, b])
    assert found["Foo"] == {a}, "the test-module write must not count"


def test_a_single_writer_is_not_in_the_shortlist(tmp_path):
    a = _write(tmp_path, "a.rs", "fn s(mut r: ResMut<Solo>) {}")
    found = mod.writers([a])
    assert {t: fs for t, fs in found.items() if len(fs) > 1} == {}


def test_a_file_with_no_resmut_contributes_nothing(tmp_path):
    """⚠ Anti-vacuity for the parser: if the regex matched loosely, every file
    would contribute and the shortlist would be the whole workspace."""
    a = _write(tmp_path, "a.rs", "fn s(r: Res<Foo>, q: Query<&Foo>) {}")
    assert mod.writers([a]) == {}


def test_the_two_test_predicates_are_the_SHARED_ones():
    """⛔⛤ NOT A RESPELLING, AND THIS CENSUS BRIEFLY HAD ONE OF EACH.

    `scripts/lib/test_paths.py` exists because five scripts spelled *"is this
    file test-only?"* five ways and gave five answers; the inline-module half has
    its own keeper in the rollback-mutator guard. A copy here would be a sixth
    and a second, and it missed `test.rs`, `test_support.rs` and the four files
    whose first attribute is an inner `#![cfg(test)]`.

    ⇒ This arm pins the IDENTITY of the functions rather than their behaviour,
    because behaviour is what drifts while both copies keep passing.
    """
    import sys

    scripts = pathlib.Path(mod.__file__).resolve().parent
    sys.path.insert(0, str(scripts / "lib"))
    sys.path.insert(0, str(scripts))
    import check_rollback_mutators_run_in_sim as mutator_guard
    import test_paths

    # ⚠ THE CACHED MODULE, NOT A FRESH `exec_module`. A second execution of the
    # same file defines new function objects, so an identity check against it
    # fails whether or not the census respelled anything — the arm would be
    # measuring `importlib`.
    assert mod.is_test_path is test_paths.is_test_path
    assert mod.strip_test_modules is mutator_guard.strip_test_modules


def test_a_whole_test_file_is_not_a_second_authority():
    """⛔⛤ THE HALF THE `#[cfg(test)]` CUT CANNOT DO.

    An integration test under `tests/` and a `mod tests` carried in its own
    `tests.rs` have no `#[cfg(test)]` line to cut at — the parent module carries
    it — so every `ResMut<T>` in them counted as a writer while the module
    docstring said the census reads non-test code. MEASURED 2026-09-17: 1,866
    tracked files became 1,294 production ones and eight types left the
    shortlist.
    """
    assert mod.is_test_path(pathlib.Path("game/ambition_app/tests/a_thing.rs"))
    assert mod.is_test_path(pathlib.Path("crates/x/src/y/tests.rs"))
    assert mod.is_test_path(pathlib.Path("crates/x/src/y/hurtbox_damage_tests.rs"))
    assert mod.is_test_path(pathlib.Path("crates/x/src/projectile/tests/collision.rs"))
    # ⚠ AND THE SHARED PREDICATE DISAGREES WITH WHAT I WOULD HAVE WRITTEN:
    # `test_support.rs` IS test-only here, unioned across the five copies. That
    # is the keeper's call to make, and a second opinion in this file would be
    # the whole defect again.
    assert mod.is_test_path(pathlib.Path("crates/x/src/test_support.rs"))
    assert not mod.is_test_path(
        pathlib.Path("game/ambition_app/src/headless.rs"),
        source="fn main() {}\n",
    )


def test_production_code_after_a_test_module_is_still_read():
    """⛔⛔ THE UNDERCOUNT, AND IT IS THE ONE A POISON FOUND BY NOT FIRING.

    The cut was `src.split("#[cfg(test)]")[0]`, and a module declares its tests
    near the TOP: `#[cfg(test)] mod tests;` at `platformer2d_runtime/src/lib.rs:25`.
    MEASURED 2026-09-17: 777 of 1,302 production files carry the attribute and 40
    of them held 77 `ResMut<T>` occurrences below the first one. A poison appended
    to a file's end proved it by changing nothing.
    """
    src = (
        "fn a(mut r: ResMut<Before>) {}\n"
        "#[cfg(test)]\nmod tests;\n"
        "fn b(mut r: ResMut<After>) {}\n"
        "#[cfg(test)]\nmod inline {\n    fn t(mut r: ResMut<Fixture>) {}\n"
        "    mod deeper { fn u(mut r: ResMut<AlsoFixture>) {} }\n}\n"
        "fn c(mut r: ResMut<Last>) {}\n"
        "fn d(mut r: ResMut<Final>) {}\n"
    )
    kept = set(mod.RESMUT.findall(mod.strip_test_modules(src)))
    assert kept == {"Before", "After", "Last", "Final"}, kept


def test_the_stripper_leaves_no_inline_test_module_behind():
    src = "fn a(mut r: ResMut<X>) {}\n#[cfg(test)]\nmod t { fn f() {} }\n"
    assert "#[cfg(test)]" not in mod.strip_test_modules(src)


def test_a_cfg_test_fn_in_a_production_file_is_a_KNOWN_residual():
    """⚠ THE COST OF USING THE KEEPER, STATED RATHER THAN DISCOVERED.

    The shared stripper removes inline `#[cfg(test)] mod X { }` blocks only, so a
    `#[cfg(test)] fn` in a production file still reads as a writer. MEASURED
    2026-09-17: there are none in this tree — every combination of the two test
    rules gives 333 types and 85 multi-writer — and widening that function
    reaches all of its consumers, which its own docstring forbids doing without
    reading each one's counts.
    """
    src = "#[cfg(test)]\nfn helper(mut r: ResMut<HelperOnly>) {}\n"
    assert mod.RESMUT.findall(mod.strip_test_modules(src)) == ["HelperOnly"]


def test_a_target_two_writers_both_reach_is_reported(tmp_path):
    a = _write(tmp_path, "a.rs", "fn s(mut save: ResMut<Foo>) { save.data_mut().x = 1; }")
    b = _write(tmp_path, "b.rs", "fn t(mut save: ResMut<Foo>) { save.data_mut(); }")
    found = mod.shared_targets("Foo", {a, b})
    touching, writing = found["data_mut"]
    assert touching == {a, b}
    # ⚠ `_mut` is what makes these CERTAIN; neither call site has an `=` on it.
    assert writing == {a, b}


def test_a_target_only_one_writer_reaches_is_not_a_shared_target(tmp_path):
    """⛔ THE WHOLE POINT OF THE NARROWING. A field one writer owns alone is the
    shape this census is looking FOR, not against, and reporting it would put 300
    lines of noise in front of the reader."""
    a = _write(tmp_path, "a.rs", "fn s(mut r: ResMut<Foo>) { r.mine = 1; r.both = 2; }")
    b = _write(tmp_path, "b.rs", "fn t(mut r: ResMut<Foo>) { r.both = 3; }")
    assert set(mod.shared_targets("Foo", {a, b})) == {"both"}


def test_a_read_is_touched_but_not_certain(tmp_path):
    """⛔⛤ THE SPLIT THIS MODE EXISTS FOR, AND THE REASON THE OLD HAND TABLE
    OVERSTATED. `save.data()` through a `ResMut` binding may be a read or the
    first half of a write, and no regex knows which. Counting it as a write is
    how `AmbitionGameSave 14 writers` ended up beside a baseline of 17."""
    a = _write(tmp_path, "a.rs", "fn s(mut save: ResMut<Foo>) { let _ = save.data(); }")
    b = _write(tmp_path, "b.rs", "fn t(mut save: ResMut<Foo>) { if save.data() {} }")
    touching, writing = mod.shared_targets("Foo", {a, b})["data"]
    assert touching == {a, b}
    assert writing == set(), "a bare call is not evidence of a write"


def test_a_whole_resource_replacement_is_its_own_target(tmp_path):
    """⭐ `*state = Default::default()` replaces every field at once, which is a
    different kind of authority from touching one of them — and it is the shape
    the hand table recorded as *nothing shared* for `VersusMatch`."""
    a = _write(tmp_path, "a.rs", "fn s(mut st: ResMut<Foo>) { *st = Foo::default(); }")
    b = _write(tmp_path, "b.rs", "fn t(mut st: ResMut<Foo>) { *st = Foo::new(); }")
    found = mod.shared_targets("Foo", {a, b})
    assert found[mod.WHOLE] == ({a, b}, {a, b})


def test_an_equality_test_is_not_an_assignment(tmp_path):
    """⛔ `==` and `=>` both follow a field with an `=`. Treating either as a
    write would make every match arm and every comparison read as certain, and
    "certain" is the number a reader trusts."""
    a = _write(tmp_path, "a.rs", "fn s(r: ResMut<Foo>) { if r.mode == 1 {} }")
    b = _write(tmp_path, "b.rs", "fn t(r: ResMut<Foo>) { match r.mode { _ => () } }")
    touching, writing = mod.shared_targets("Foo", {a, b})["mode"]
    assert touching == {a, b}
    assert writing == set()


def test_a_compound_assignment_is_certain(tmp_path):
    a = _write(tmp_path, "a.rs", "fn s(mut r: ResMut<Foo>) { r.n += 1; }")
    b = _write(tmp_path, "b.rs", "fn t(mut r: ResMut<Foo>) { r.n |= 2; }")
    assert mod.shared_targets("Foo", {a, b})["n"][1] == {a, b}


def test_a_qualified_binding_is_still_found(tmp_path):
    """⚠ The census collapses `ResMut<a::b::Foo>` onto `Foo`, so the target scan
    has to find the binding under the qualified spelling too — otherwise the
    three `ResMut<ambition_characters::control::SlotInteractionState>` writers in
    this tree would report no shared target at all."""
    a = _write(tmp_path, "a.rs", "fn s(mut r: ResMut<ambition_x::y::Foo>) { r.f = 1; }")
    b = _write(tmp_path, "b.rs", "fn t(mut r: ResMut<Foo>) { r.f = 2; }")
    assert mod.shared_targets("Foo", {a, b})["f"][0] == {a, b}


def test_a_fixture_access_is_not_a_shared_target(tmp_path):
    """⛔ The same `#[cfg(test)]` rule as the writer scan, for the same reason: a
    fixture reaching for a field is not a second authority over it."""
    a = _write(tmp_path, "a.rs", "fn s(mut r: ResMut<Foo>) { r.f = 1; }")
    b = _write(
        tmp_path,
        "b.rs",
        "fn t(mut r: ResMut<Foo>) {}\n#[cfg(test)]\nmod x { fn u(mut r: ResMut<Foo>) { r.f = 2; } }\n",
    )
    assert mod.shared_targets("Foo", {a, b}) == {}


def test_a_wrapped_resmut_with_a_trailing_comma_is_a_writer(tmp_path):
    """⛔⛤ TEN SITES, AND THE ONE THAT MATTERED MOST WAS THE SLOT'S CONSUMER.

    A long qualified path wraps, and the wrapped form carries a trailing comma:

        ResMut<
            ambition_..::session::lifecycle_commit::PendingLifecycleCommit,
        >,

    `\\s*>` cannot cross that comma. `room_transition/commit.rs` — the system that
    SPENDS the rollback-registered lifecycle slot — was therefore not a writer of
    it as far as this census was concerned, so the type read as 3 writer files
    when it has 6. MEASURED 2026-09-17: 10 sites, `FeatureEcsWorldOverlay` 8->9,
    `DeveloperRuntimeState` 5->6, `RoomTransitionCooldown` 2->3, and
    `MovingPlatformSet` arriving on the shortlist for the first time.

    ⚠ **AND THE BLIND SPOT WAS NOT RANDOM**, which is what makes it worth an arm
    rather than a one-line fix: the wrapped sites are exactly the ones a
    formatter chose to wrap, wrapping follows PATH LENGTH, and a long path means
    the type came from another crate — which is where a second authority is most
    likely to be.
    """
    a = _write(tmp_path, "a.rs", "fn s(mut r: ResMut<Foo>) {}")
    b = _write(
        tmp_path,
        "b.rs",
        "fn t(\n    mut r: ResMut<\n        some::very::long::path::Foo,\n    >,\n) {}\n",
    )
    assert mod.writers([a, b])["Foo"] == {a, b}


def test_a_generic_resource_is_still_not_matched(tmp_path):
    """⭐ THE CONTROL FOR THE WIDENING ABOVE. Tolerating `,?` must not turn the
    pattern into "anything up to the next `>`": `ResMut<Assets<Image>>` has an
    inner generic, and collapsing it onto `Assets` would put a Bevy asset store
    on a duplicate-authority shortlist in company with every other user of it.
    The census has never claimed to read generic resources, and this arm is why
    the widening did not quietly start."""
    a = _write(tmp_path, "a.rs", "fn s(mut r: ResMut<Assets<Image>>) {}")
    b = _write(tmp_path, "b.rs", "fn t(mut r: ResMut<Assets<Image>>) {}")
    assert mod.writers([a, b]) == {}


def test_an_exclusive_world_write_is_a_writer(tmp_path):
    """⛔⛔ THE OTHER WAY BEVY HANDS OUT `&mut T`, AND THIS CENSUS KNEW ONLY ONE.

    MEASURED 2026-09-17: 83 multi-writer types became 102 when
    `world.resource_mut::<T>()` counted — 19 types were not on the shortlist at
    all and 21 gained writers.

    ⭐ **AND THE MISSING FILES WERE NOT A RANDOM SAMPLE**, which is the part worth
    keeping: an exclusive-world system is what a COMMIT EXECUTOR is, so the road
    this shape hid was the destructive one.
    `rollback_ggrs/lifecycle_commit.rs` clears `PendingLifecycleCommit` and
    `RoomTransitionLoadState`; `session/reset/mod.rs` reaches `AmbitionGameSave`,
    `AuthoredOccurrences`, `QuestRegistry` and `GameplayBanner`. The census was
    blind to the systems that SPEND the state it was auditing.
    """
    a = _write(tmp_path, "a.rs", "fn s(mut r: ResMut<Foo>) {}")
    b = _write(
        tmp_path,
        "b.rs",
        "fn t(world: &mut World) {\n"
        "    if let Some(mut f) = world.get_resource_mut::<some::path::Foo>() { f.x = 1; }\n"
        "    let mut g = world.resource_mut::<Foo>();\n"
        "}\n",
    )
    assert mod.writers([a, b])["Foo"] == {a, b}


def test_installing_a_resource_is_not_counted_as_writing_it(tmp_path):
    """⭐ THE CONTROL, AND IT IS A DELIBERATE SCOPE LINE RATHER THAN AN OVERSIGHT.

    `insert_resource` / `init_resource` / `get_resource_or_insert_with` INSTALL a
    value instead of mutating a live one. That is a different question — who owns
    CONSTRUCTION — with a different right answer, and counting it here would put
    every plugin's `build` on a duplicate-authority shortlist. If this arm ever
    goes red, the widening that did it needs its own measured population before
    it lands, not after.
    """
    a = _write(tmp_path, "a.rs", "fn s(app: &mut App) { app.insert_resource(Foo::default()); }")
    b = _write(tmp_path, "b.rs", "fn t(app: &mut App) { app.init_resource::<Foo>(); }")
    c = _write(
        tmp_path,
        "c.rs",
        "fn u(world: &mut World) { world.get_resource_or_insert_with::<Foo>(Foo::default); }",
    )
    assert mod.writers([a, b, c]) == {}


def test_a_target_reached_from_an_exclusive_world_binding_joins_the_table(tmp_path):
    """⛔⛤ THE FILE COUNTED AND ITS TARGETS DID NOT, WHICH IS THE WORST OF BOTH.

    `_BINDING` reads the binding name off a `ResMut<T>` PARAMETER; an
    exclusive-world write has no parameter to read. So when
    `world.resource_mut::<T>()` started counting as a writer, those files raised
    the FILE total while contributing nothing to the join — and the narrowing
    table, which is what tells the next reader where to spend an adjudication,
    silently got less representative as the census got more complete.

    ⇒ MEASURED on `PendingLifecycleCommit`: "record, 2 of 7 files" before, and
    after this both `record` (2) and `take` (2) — the armers AND the clearers,
    which is the shape its verdict actually turns on.
    """
    a = _write(tmp_path, "a.rs", "fn s(mut p: ResMut<Foo>) { let _ = p.record(1); }")
    b = _write(
        tmp_path,
        "b.rs",
        "fn t(world: &mut World) {\n"
        "    if let Some(mut p) = world.get_resource_mut::<a::b::Foo>() { p.record(2); }\n"
        "}\n",
    )
    c = _write(
        tmp_path,
        "c.rs",
        "fn u(world: &mut World) { let mut p = world.resource_mut::<Foo>(); p.record(3); }\n",
    )
    touching, _ = mod.shared_targets("Foo", {a, b, c})["record"]
    assert touching == {a, b, c}


def test_a_system_param_bundle_field_is_a_writer(tmp_path):
    """⛔⛔ THE BIGGEST OF THE THREE REGEX HOLES, AND THE WORST-TARGETED ONE.

    `ResMut<'w, Foo>` is how every `#[derive(SystemParam)]` bundle spells it, and
    a bundle is exactly the shape a resource takes when SEVERAL SYSTEMS SHARE ONE
    ACCESSOR — `ActingParticipant` answering *"which controller wants to
    interact"*, `DialogueDispatch` opening a conversation. So the census was
    blind to the most deliberate form of shared access in the tree, which is the
    thing it exists to find.

    MEASURED 2026-09-17: 80 sites over 13 files and 69 types, shortlist 102 ->
    121, with 19 types arriving that had never been on it and 31 gaining writers
    — `SlotInteractionState` 4 -> 6, `ActiveConversation` 2 -> 5. Two verdicts
    banked earlier that day had been written against incomplete populations.
    """
    a = _write(tmp_path, "a.rs", "fn s(mut r: ResMut<Foo>) {}")
    b = _write(
        tmp_path,
        "b.rs",
        "#[derive(SystemParam)]\npub struct P<'w, 's> {\n"
        "    state: ResMut<'w, Foo>,\n"
        "    q: Query<'w, 's, &'static Bar>,\n}\n",
    )
    c = _write(
        tmp_path,
        "c.rs",
        "#[derive(SystemParam)]\nstruct Q<'world> {\n    f: ResMut<'world, a::b::Foo>,\n}\n",
    )
    assert mod.writers([a, b, c])["Foo"] == {a, b, c}


def test_a_system_param_field_contributes_its_targets_too(tmp_path):
    r"""A bundle's field must reach the narrowing table, not just the file total.

    ⛔⛤ **AND THIS ARM WAS FIRST WRITTEN TO CHECK `shared_targets`' RESULT, WHERE
    IT PASSED FOR THE WRONG REASON — CAUGHT BY POISONING.** Deleting the optional
    lifetime from `_BINDING` left the whole file green, because `shared_targets`
    falls back to a LOOSE pattern (`ResMut<[^>]*\bFoo\s*>`) whenever the strict
    one finds no binding, and that fallback happens to span `'w, `. So the
    end-to-end reading could not tell the strict path from the fallback.

    ⇒ It now asserts `_BINDING` itself, which is the thing the fix changed. The
    fallback still exists and still covers this shape; what this pins is that the
    STRICT path does too, so the loose one stays a fallback rather than becoming
    the road every `SystemParam` bundle travels.
    """
    import re

    strict = re.compile(mod._BINDING % "Foo")
    assert strict.findall("    gestures: ResMut<'w, Foo>,") == ["gestures"]
    assert strict.findall("    mut plain: ResMut<Foo>,") == ["plain"]

    a = _write(tmp_path, "a.rs", "fn s(mut r: ResMut<Foo>) { r.clear(); }")
    b = _write(
        tmp_path,
        "b.rs",
        "#[derive(SystemParam)]\nstruct P<'w> {\n    gestures: ResMut<'w, Foo>,\n}\n"
        "impl P<'_> { fn go(&mut self) { self.gestures.clear(); } }\n",
    )
    touching, _ = mod.shared_targets("Foo", {a, b})["clear"]
    assert touching == {a, b}


def test_a_lifetime_is_not_mistaken_for_the_type(tmp_path):
    """⭐ THE CONTROL. `'w` must be consumed as a lifetime, not collapsed onto a
    type called `w` — and a `ResMut<'w, Assets<Image>>` must still be skipped,
    because the generic rule has to survive the lifetime widening."""
    a = _write(tmp_path, "a.rs", "fn s(mut r: ResMut<'w, Foo>) {}")
    found = mod.writers([a])
    assert set(found) == {"Foo"}, found
    b = _write(tmp_path, "b.rs", "struct P<'w> { a: ResMut<'w, Assets<Image>> }")
    c = _write(tmp_path, "c.rs", "struct Q<'w> { a: ResMut<'w, Assets<Image>> }")
    assert mod.writers([b, c]) == {}


def test_a_session_world_component_is_its_own_population(tmp_path):
    """⛔⛤ THE POPULATION A10 CREATED AND THE INSTRUMENT DID NOT FOLLOW.

    A session world component lives on the session root and is reached through
    `SessionWorldMut<T>`, not `ResMut<T>`. MEASURED 2026-09-17: none of the eight
    types carrying that accessor has `#[derive(Resource)]`, so the resource
    census is silent about them BY CONSTRUCTION — while four have more than one
    production writer and `EncounterMusicRequest` has eight.

    ⭐ The spelling came from `check_rollback_mutators_run_in_sim.py`, whose
    docstring had already recorded both lessons this census needed: that
    `SessionWorldMut<T>` is a mutable param, and that *"a guard keyed on how a
    write is SPELLED goes blind when a refactor respells it, and the direction is
    the dangerous one — it reports no offenders."*
    """
    a = _write(tmp_path, "a.rs", "fn s(mut r: SessionWorldMut<RoomSet>) {}")
    b = _write(
        tmp_path,
        "b.rs",
        "#[derive(SystemParam)]\nstruct P<'w> {\n"
        "    rooms: SessionWorldMut<'w, a::b::RoomSet>,\n}\n",
    )
    assert mod.session_world_writers([a, b])["RoomSet"] == {a, b}
    # ⛔ AND THE TWO POPULATIONS MUST NOT LEAK INTO EACH OTHER. Folding them
    # would make "two writers" mean two different things in one number: a
    # resource's lifetime is the App's, a session world component's is the
    # session's, and a session boundary reclaims the second.
    assert mod.writers([a, b]) == {}
    c = _write(tmp_path, "c.rs", "fn t(mut r: ResMut<RoomSet>) {}")
    assert mod.session_world_writers([c]) == {}


def test_a_session_world_fixture_is_not_a_writer(tmp_path):
    """⚠ Same rule as the resource side, and worth its own arm because this
    population is small: with only eight types, one counted fixture is a
    12% error."""
    a = _write(tmp_path, "a.rs", "fn s(mut r: SessionWorldMut<RoomSet>) {}")
    b = _write(
        tmp_path,
        "b.rs",
        "fn t() {}\n#[cfg(all(test, feature = \"x\"))]\nmod fix {\n"
        "    fn u(mut r: SessionWorldMut<RoomSet>) {}\n}\n"
        "// a comment about `SessionWorldMut<RoomSet>` is not a writer either\n",
    )
    assert mod.session_world_writers([a, b])["RoomSet"] == {a}


def test_a_write_site_is_attributed_to_its_enclosing_item(tmp_path):
    """⭐ `write_sites` answers *"how many ITEMS in this file write it"*, which is
    the question a verdict saying "one owner" is actually making."""
    f = tmp_path / "two_systems.rs"
    f.write_text(
        "fn one(mut a: ResMut<Shared>) { a.x = 1; }\n"
        "fn two(mut b: ResMut<Shared>) { b.x = 2; }\n"
        "fn reads_only(c: Res<Shared>) {}\n",
        encoding="utf-8",
    )
    sites = mod.write_sites("Shared", [str(f)])
    assert sites[str(f)] == ["one", "two"]


def test_a_bundle_field_is_not_attributed_to_the_function_above_it(tmp_path):
    """⛔⛤ THE CORRECTION THAT MADE THIS FUNCTION USABLE, 2026-09-18.

    With only `fn` headers, a `ResMut` FIELD of a `SystemParam` bundle was
    attributed to the nearest `fn` above it — and in the real tree that named
    `capture_armed_rebind` as the writer of `NewGameResetRequested`, a function
    that does not touch it. ⇒ **A site-level instrument that names the WRONG
    system is worse than a file-level one that names none**, because a verdict
    quoting it reads as measured. The honest answer for a bundle is the bundle.
    """
    f = tmp_path / "bundle.rs"
    f.write_text(
        "fn something_else(q: Query<&T>) {}\n"
        "\n"
        "#[derive(SystemParam)]\n"
        "pub struct MenuParams<'w> {\n"
        "    reset: ResMut<'w, Shared>,\n"
        "}\n",
        encoding="utf-8",
    )
    sites = mod.write_sites("Shared", [str(f)])
    assert sites[str(f)] == ["<param bundle: MenuParams>"]
    assert "something_else" not in sites[str(f)]


def test_write_sites_and_writers_agree_about_what_counts_as_code(tmp_path):
    """⚠ TWO INSTRUMENTS OVER ONE POPULATION. A `write_sites` that saw a test
    module or a comment that `writers` does not would make a per-system verdict
    disagree with the per-file census it is written against."""
    f = tmp_path / "mixed.rs"
    f.write_text(
        "fn live(mut a: ResMut<Shared>) {}\n"
        "// fn commented(mut a: ResMut<Shared>) {}\n"
        "#[cfg(test)]\n"
        "mod tests {\n"
        "    fn fixture(mut a: ResMut<Shared>) {}\n"
        "}\n",
        encoding="utf-8",
    )
    assert mod.write_sites("Shared", [str(f)])[str(f)] == ["live"]
    assert mod.writers([str(f)])["Shared"] == {str(f)}


def test_write_sites_sees_every_spelling_writers_does(tmp_path):
    """⛔⛤ THE ARM THAT WOULD HAVE CAUGHT A REAL MISS, ADDED AFTER IT DID NOT.

    `write_sites` first spelled its own turbofish regex and left out the optional
    trailing comma, so this shape — real, in
    `world/gated_lock_walls.rs` — was a writer to `writers` and invisible to
    `write_sites`. The per-file census said 10 files for
    `FeatureEcsWorldOverlay`; the per-system view showed 9, and NOTHING
    complained. ⇒ Two instruments over one population must SHARE the pattern.
    """
    f = tmp_path / "spellings.rs"
    f.write_text(
        "fn a(mut r: ResMut<Shared>) {}\n"
        "fn b(mut r: ResMut<'w, Shared>) {}\n"
        "fn c(mut r: ResMut<some::path::Shared>) {}\n"
        "fn d(world: &mut World) {\n"
        "    let _ = world.resource_mut::<Shared>();\n"
        "}\n"
        "fn e(world: &mut World) {\n"
        "    let _ = world.get_resource_mut::<\n"
        "        some::long::path::Shared,\n"
        "    >();\n"
        "}\n",
        encoding="utf-8",
    )
    assert mod.writers([str(f)])["Shared"] == {str(f)}
    assert mod.write_sites("Shared", [str(f)])[str(f)] == ["a", "b", "c", "d", "e"]


def test_the_two_instruments_agree_across_the_whole_tree():
    """⭐ NOT A UNIT TEST, AND DELIBERATELY: the hand-built corpus above cannot
    enumerate the spellings this repository actually uses. For every multi-writer
    type, the FILES `write_sites` finds must be exactly the files `writers`
    found — a disagreement means one of the two is reading a different
    population, and a per-system verdict written against the smaller one names
    the wrong owner."""
    files = mod.production_files()
    found = mod.writers(files)
    multi = {t: fs for t, fs in found.items() if len(fs) > 1}
    assert len(multi) > 50, "the corpus collapsed; this arm would pass over nothing"
    disagree = {
        t: (sorted(fs), sorted(mod.write_sites(t, fs)))
        for t, fs in multi.items()
        if set(mod.write_sites(t, fs)) != set(fs)
    }
    assert not disagree, disagree
