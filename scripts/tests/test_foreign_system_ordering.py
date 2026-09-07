"""Prerequisite C: cross-domain ordering must not name foreign system identities.

⭐⭐ THE RULE, from the architecture program: *"Every load-bearing cross-capability
ordering relationship must be expressible using public phase/set vocabulary rather
than foreign system identities."* A crate that fixes the relative order of two
OTHER crates' private systems has taken authority over domains it does not own,
and moving those domains into separately composable crates does not make them
separately composable — the edge is in the code, not in the packaging.

⛔ A RATCHET, NOT A GATE — AND ONE HALF OF IT HAS REACHED ZERO. Eighty-seven
existed when this was written and the capability-written half is now **0**: every
row a ruleset or capability wrote has been converted to published-set vocabulary.
The composition-layer half (72) has not, so the total ceiling is still a ratchet
rather than a gate.

⚠ AND THE ANTI-VACUITY FLOOR HAD TO MOVE WHEN THAT HAPPENED. It floored the
CAPABILITY bucket — "zero would be excellent news and is not what this tree
contains" — which stopped being true. Deleting it would have left the classifier
free to collapse unnoticed; it now floors the population that is still non-empty,
and the classifier stays pinned separately. **A floor protects the population it
names, and the population it names can be fixed out from under it.**

⚠ THE MEASURE ITSELF TOOK FOUR CORRECTIONS AND EACH ONE MATTERED, recorded in
`scripts/measure_foreign_system_ordering.py` so the next reader inherits them
rather than the conclusions:

* a bare module path (`actors::sync_visuals`) is INTRA-crate, not foreign — the
  first run said 28 violations and most were a crate ordering itself;
* being the runtime is not an exemption, and the version that made it one
  excused the architecture note's own named example;
* `.before(...)` is not the only spelling — that example is a CHAINED TUPLE and
  a regex over `.before(` scored it as absent;
* but a chain is only the defect when it spans TWO DIFFERENT foreign crates. A
  composition layer sequencing systems it installs is what a runtime is for; the
  version without that discriminator reported 174 and most of it was a runtime
  doing its job.
"""

from __future__ import annotations

import importlib.util
import pathlib
import tempfile

REPO = pathlib.Path(__file__).resolve().parents[2]
MEASURE = REPO / "scripts" / "measure_foreign_system_ordering.py"

#  measured 2026-09-06. Both may fall; neither may rise.
#
# ⛔⛔ AND THE TOTAL WENT 72 -> 78 WITHOUT THE TREE GETTING WORSE, which is the
# one thing a ratchet cannot express and must therefore say in prose. The measure
# began resolving a system to the crate that DEFINES it rather than to the crate
# the caller spelled — and that moved rows in BOTH directions: it dropped 31
# self-installs (the runtime naming its own systems through the `ambition_platformer2d`
# umbrella, counted as reaching into somebody else's crate) and it caught more
# whose head looked local. Net +6.
#
# ⇒ A RATCHET IS ONLY MEANINGFUL WHILE THE INSTRUMENT IS FIXED. Re-baselining after
# an instrument change is not laundering a regression — but it is indistinguishable
# from one unless the reason is written next to the number, so it is.
# ⭐ The invariant that matters held through the change: capability-written is
# still 0, measured the new way.
#
# ⭐ 15 -> 10 AND 87 -> 82 IN THREE STEPS, AND ONLY TWO OF THEM WERE FIXES:
#   · the dismount worked example (below) — 15 -> 13;
#   · `apply_summon_effects` joining `EffectExecutionSet` — 13 -> 12;
#   · a peer's dormancy fix — the set was already published and unused;
#   · the two `scripted_input.rs` edges, redundant with a set edge already
#     beside them — 9 -> 7;
#   · ⛔ and a MEASUREMENT correction, 12 -> 10: `#![cfg(test)]` is a FILE-level
#     gate, invisible both to a filename heuristic and to the inline-`mod`
#     stripper, so `features/ecs/fighter_harness.rs` was counted as production
#     and contributed two false violations. Exactly one file in the tree carries
#     that attribute, which is why it hid — a rule with a single instance is one
#     nobody trips over until it matters.
#
# ⭐ 15 -> 13 AND 87 -> 85 BY THE WORKED EXAMPLE: `ambition_mount` now publishes
# `DismountRequestsApplied`, the runtime installs `apply_dismount_requests` into
# it, and `ambition_demo_smash`'s two shark-ride orderings name the SET. The
# ruleset was naming the function because there was nothing else to name — the
# system belonged to no published set — which is the shape of most of what is
# left here.
CAPABILITY_ORDERING_CEILING = 0
# ⭐⭐ 78 -> 75 BY THE FIRST CAPABILITY-OWNED PLUGIN (prerequisite C2).
# `ambition_mount` now ships `MountPlugin` + `install_mount_simulation_systems`,
# so the runtime adds a PLUGIN and says which schedule and which phase instead
# of naming four of that crate's private systems. Mount rows: 7 -> 2.
TOTAL_ORDERING_CEILING = 75


def _module():
    spec = importlib.util.spec_from_file_location("measure_foreign_ordering", MEASURE)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _rows():
    module = _module()
    foreign = [r for r in module.findings(include_local=False) if r[3] == "foreign"]
    ordering = [r for r in foreign if r[4] == "ordering"]
    capability = [r for r in ordering if not module.is_composition_layer(r[0])]
    return ordering, capability


def test_a_capability_does_not_order_another_crates_systems() -> None:
    """⛔ THE SHARP HALF. A ruleset or capability crate reaching into another
    crate's schedule is the case with no defence at all: it is not composing
    anything, it is asserting that somebody else's system runs at a particular
    moment relative to its own."""
    _, capability = _rows()
    assert len(capability) <= CAPABILITY_ORDERING_CEILING, (
        f"{len(capability)} capability-written foreign orderings "
        f"(ceiling {CAPABILITY_ORDERING_CEILING}). Each one is a crate that "
        "cannot be composed away from the crate it names:\n  "
        + "\n  ".join(f"{c} -> {t}  ({w})" for c, t, w, _, _ in sorted(set(capability)))
    )


def test_foreign_ordering_overall_only_shrinks() -> None:
    """The whole population, composition layers included — because the runtime
    owning PHASES is not the same as the runtime owning the pairwise order of two
    capabilities' private systems, and only the second is counted here."""
    ordering, _ = _rows()
    assert len(ordering) <= TOTAL_ORDERING_CEILING, (
        f"{len(ordering)} foreign orderings (ceiling {TOTAL_ORDERING_CEILING}); "
        "the prerequisite is that this number reaches zero, so it may not rise"
    )


def test_the_capability_split_is_not_degenerate() -> None:
    """⛔⛔ THE POISON THAT CAUGHT THIS ONE PASSED, WHICH IS WHY IT EXISTS.

    Making `is_composition_layer` return `True` for everything — the exact
    regression an earlier version of the measure shipped, excusing the runtime —
    empties the capability bucket, so a ceiling of 15 is satisfied by zero and
    both other tests stay green. ⇒ A ceiling can only see the number growing; it
    cannot see the CLASSIFIER collapsing, and a collapsed classifier reports
    perfect compliance.

    ⭐ So this pins the split itself against a case with no ambiguity:
    `ambition_demo_smash` is a RULESET. If it is ever counted as a composition
    layer, the split has stopped meaning anything."""
    module = _module()
    assert not module.is_composition_layer("ambition_demo_smash"), (
        "a ruleset crate is being classified as a composition layer, so every "
        "capability-written ordering is being excused as 'the runtime owns global "
        "order' — which is the exemption the architecture note's own example "
        "disproves"
    )
    assert module.is_composition_layer("ambition_platformer2d_runtime"), (
        "the runtime is no longer recognised as a composition layer, so the "
        "split has collapsed the other way and every row reads as a capability"
    )
    # ⛔⛔ THIS ARM USED TO SAY "zero would be excellent news and is not what this
    # tree contains", floored on the CAPABILITY bucket. It is zero now — somebody
    # did fix them — so the floor had to move rather than be deleted, or the
    # classifier could collapse and nothing would notice. ⇒ Floor the population
    # that is still non-empty, and keep the classifier pinned above.
    ordering, _ = _rows()
    assert ordering, (
        "zero foreign orderings ANYWHERE, capability and composition alike. That "
        "would be a finished prerequisite and is not what this tree contains: a "
        "zero here means the measure stopped finding them"
    )


def test_the_measure_still_sees_a_cross_crate_chain() -> None:
    """⭐⭐ THE POSITIVE CONTROL — AND IT HAD TO BE REPOINTED, WHICH IS THE WHOLE
    LESSON OF THIS EDIT.

    It used to pin the ONE instance the architecture note named by hand:
    `ambition_mount::enforce_mount_rider_link` chained with
    `actor_monolith::rebuild_dismounted_rider_brains`. Two versions of the measure
    scored that chain as absent — once by excusing the runtime, once by matching
    only `.before(` — so pinning it kept a simplified matcher from quietly losing
    the thing it was built for.

    ⛔⛔ THEN THE DEFECT WAS FIXED AND THE CONTROL WENT RED. That is the correct
    failure and it is worth naming, because the tempting reading is "the guard is
    broken". ⇒ **A positive control pinned to a live defect has a lifetime: it
    dies the day somebody repairs its subject, and the red it throws is a report
    of SUCCESS wearing the costume of a regression.** The wrong response is to
    delete it — that leaves the matcher free to collapse unnoticed, which is the
    exact hazard it was guarding. The right response is to repoint it and write
    down what it used to hold, so the next reader can tell a fixed pin from a
    broken one.

    ⚠ SO THIS ONE IS PINNED ON A SHAPE INSTEAD OF A NAME. It asserts that the
    measure still finds a chained tuple spanning two *different* foreign crates —
    the spelling that defeated the `.before(`-only matcher — without depending on
    any single row surviving. It still dies when the prerequisite is FINISHED, at
    which point the ceiling is 0 and this file has done its job."""
    _assert_chain_detected_on_a_synthetic_tree()


def _write(root: pathlib.Path, rel: str, text: str) -> None:
    path = root / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def _assert_chain_detected_on_a_synthetic_tree() -> None:
    """⛔⛔ THE ARM THAT ACTUALLY CATCHES CHAIN-BLINDNESS, and it exists because
    the obvious version of it PASSED THE POISON.

    The first draft asserted "some ordering row comes from a file containing
    `.chain()`". Ran against a measure with chain detection deliberately disabled:
    **green**. ⇒ It was checking a property of the FILE, and `combat_schedule.rs`
    contains `.chain()` for a dozen local reasons, so the arm was true no matter
    what the measure did with it. The count told the real story — 75 rows clean,
    **9** chain-blind — but flooring the count is not available here: this is a
    ratchet whose whole purpose is to drive that number to zero, and a floor of 40
    goes red the moment the prerequisite makes progress.

    ⇒ So the detection is tested against a TREE BUILT FOR IT: two capability
    crates, one composition crate chaining a system from each. No `.before(`, no
    `.after(` — the spelling that defeated the original matcher and nothing else.
    A measure that sees it finds exactly one ordering row; one that does not finds
    zero. This arm cannot be satisfied by the real tree's incidental punctuation,
    and it stays meaningful on the day the ceiling reaches 0."""
    module = _module()
    with tempfile.TemporaryDirectory() as tmp:
        root = pathlib.Path(tmp)
        for crate, body in (
            ("cap_alpha", "pub fn alpha_system() {}\n"),
            ("cap_beta", "pub fn beta_system() {}\n"),
        ):
            _write(root, f"crates/{crate}/Cargo.toml", f'[package]\nname = "{crate}"\n')
            _write(root, f"crates/{crate}/src/lib.rs", body)
        _write(root, "crates/fake_runtime/Cargo.toml", '[package]\nname = "fake_runtime"\n')
        # The defect, in the one spelling that matters: a composition layer fixing
        # the relative order of two OTHER crates' private systems, via `.chain()`.
        _write(
            root,
            "crates/fake_runtime/src/lib.rs",
            "pub fn build(app: &mut App) {\n"
            "    app.add_systems(\n"
            "        Update,\n"
            "        (cap_alpha::alpha_system, cap_beta::beta_system).chain(),\n"
            "    );\n"
            "}\n",
        )
        original = module.REPO
        try:
            module.REPO = root
            rows = [r for r in module.findings(include_local=False) if r[4] == "ordering"]
        finally:
            module.REPO = original
    assert rows, (
        "the measure found NO ordering row in a tree built to contain exactly one: "
        "a composition crate chaining `cap_alpha::alpha_system` with "
        "`cap_beta::beta_system`. It has stopped recognising the chained-tuple "
        "spelling, which is how it once scored the architecture note's own "
        "reference instance as absent"
    )
    named = {r[1] for r in rows}
    assert any("alpha_system" in n for n in named) and any("beta_system" in n for n in named), (
        f"the chain was detected but attributed to the wrong systems: {sorted(named)}"
    )
