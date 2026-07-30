#!/usr/bin/env python3
"""Check the architectural absences this repo depends on — claims that nothing exists.

``scripts/check_roadmap_evidence.py`` verifies that a claim's CITATIONS still
exist. It cannot verify the other kind of claim, the kind whose whole content is
that a thing does *not* exist:

* "`register_character` no longer demands art" (queue A1),
* "the String-keyed sheet-row lookup is deleted" (binding-resolution boundary),
* "the rollback exit oracle is not quarantined" (queue A6),
* "the fight test drives the real damage path" (queue A2).

An absence has no citation to rot, so nothing re-reads it. The queue found this
the expensive way: a row said `with_moveset` had NO production caller, C4 gave it
two, and the row went on saying it for as long as it took somebody to notice
(queue W1). Being right when written is not a property a document keeps.

**The mechanism is a predicate, not a cleverer parser.** Do not teach the
evidence checker to read "used to" / "no longer" / "not yet" — that is prose
interpretation, and ``check_roadmap_evidence.py``'s own docstring explains why it
refuses to go there. An absence that MATTERS belongs in the table below, where it
reddens the day somebody reintroduces the thing.

⚠ **Why this is not a bare `git grep`, which is what the queue first proposed.**
Three times a goal-guard check grepped for the absence of an identifier and three
times it went red on PROSE — the phrase appeared in a doc comment *explaining the
removal*. Documenting a removal must never break the guard that verified it. So
every contract here:

* searches **production source only** — the paths are explicit, and this file,
  the planning tree and the test-support scaffolding are outside them;
* **strips comments before matching**, which is the fix for the recurrence above:
  ``//``, ``///``, ``//!``, ``#`` and block-comment bodies are not code, and a
  paragraph explaining why a symbol is gone is the opposite of evidence that it
  is back;
* uses an **exact symbol or a narrowly scoped pattern**. A broad negative grep
  generates noise, and a noisy guard gets waived, which is worse than no guard;
* carries an **id and a reason**, so a red line says what architectural property
  broke rather than just which regex matched.

A contract is meant to be DELETED or INVERTED when the architecture deliberately
changes. That is not a failure of the guard — a red line here is a conversation
about whether the absence is still wanted, and answering "no, we want the thing
now" is a legitimate answer that ends with this row removed in the same commit.

## The second table: dependency edges

``DEPENDENCY_CONTRACTS`` guards the half a grep CANNOT express. "Crate A must not
depend on crate B" is a fact about the manifest graph, not about any line of
text: a grep can find the ``use`` that proves an edge exists and miss the one
added through a re-export tomorrow, and it cannot see an edge introduced through
an intermediary at all. So that table is checked against ``cargo metadata``, and
checked TRANSITIVELY — the claim is that a foundation cannot REACH gameplay, and
a layering inversion almost never arrives as a direct dependency line.

Both tables are RED-PROBED in ``scripts/tests/test_absence_contracts.py``. Every
contract here is green against the live tree, which is the whole point of it and
also the reason running it proves nothing about whether it works.

Usage:
    python3 scripts/check_absence_contracts.py            # report every contract
    python3 scripts/check_absence_contracts.py --check    # exit 1 on a violation
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from bisect import bisect_right
from pathlib import Path

# Every entry is one architectural absence. `paths` are git pathspecs limited to
# production source; `patterns` are Python regexes matched against comment-
# stripped lines. Keep patterns narrow — see the module docstring.
#
# ⚠ CONFINEMENT, NOT SINGULARITY — and the names now say so.
#
# A contract that excludes the one allowed file proves "no reference exists
# OUTSIDE this file". It does not prove that exactly one call exists inside it,
# that the intended call still exists, or that a second resolver was not added
# beside the first. `provider_of_character` already has TWO calls inside
# `presentation.rs` and satisfies its guard.
#
# These were originally named `one-caller-of-*` / `one-reader-of-*`, and
# Campaign 1 then cited them as evidence of exact single authority — which they
# never were (GPT 5.6, 2026-07-28). Confinement is genuinely valuable: it is
# what stops a SECOND file growing its own opinion, which is how every
# split-authority bug in this campaign started. It is just not the stronger
# claim, so the ids no longer imply it.
#
# If exact singularity ever matters for one of these, add a positive count
# assertion INSIDE the allowed file rather than renaming it back.
ABSENCE_CONTRACTS: list[dict] = [
    {
        "id": "registration-does-not-demand-art",
        "paths": ["crates/ambition_actors/src/character_runtime/definition.rs"],
        "patterns": [r"CharacterLoadDemand::request", r"\bdemand\.request\("],
        "reason": (
            "Registering a character DECLARES it; it does not ask for its art. "
            "Demanding at registration defeats the room/match/worn projection "
            "model — every registered character would decode whether or not "
            "anything staged it. Staging is the only demand source (queue A1). "
            "`CharacterLoadDemand` is legitimately named in this file's prose, "
            "which is why this matches code only."
        ),
    },
    {
        "id": "the-worlds-path-is-confined-to-ldtk-paths",
        # The subject includes TEST modules: five of them spelled the path too,
        # and a contract that guards only the package would have watched half a
        # migration.
        "include_tests": True,
        "paths": [
            "tools/ambition_ldtk_tools/",
            # The one legitimate home, and the test that pins what it returns.
            ":(exclude)tools/ambition_ldtk_tools/ambition_ldtk_tools/ldtk/paths.py",
            ":(exclude)tools/ambition_ldtk_tools/tests/test_ldtk_core_helpers.py",
        ],
        "patterns": [r'/\s*"worlds"'],
        "reason": (
            "The LDtk worlds directory is built in ONE place, `ldtk/paths.py`, "
            "whose own docstring says it exists 'so individual commands do not "
            "recreate stale repository-layout assumptions'. Fifteen commands and "
            "five test modules then recreated them anyway, in three different "
            "spellings, and when the worlds moved out of "
            "`crates/ambition_actors/assets` every one of them broke: 11 of 149 "
            "tests red on a clean tree, and a bare `ldtk edit measure` dying with "
            "FileNotFoundError because its `--ldtk` default pointed at a "
            "directory that had not existed for weeks (2026-07-28). A second "
            "copy of a correct path is a copy that has not gone stale YET."
        ),
    },
    {
        "id": "no-string-keyed-sheet-row-lookup",
        "paths": ["crates/", "game/", "fixtures/", "tools/"],
        "patterns": [r"\.row_index_of\(", r"\bfn row_index_of\b"],
        "reason": (
            "A sheet row addressed by an arbitrary &str returned None when the "
            "sprite called the animation `death` and the policy asked for "
            "`dead`, and NOTHING DREW — an absence rendered as an absence. The "
            "lookup was deleted in favour of resolved bindings, and the story "
            "survives only as prose in two `binding.rs` module docs "
            "(binding-resolution boundary, 2026-07-25/26)."
        ),
    },
    {
        # Campaign A4's GUARD leg. A slice ends with one path, not two — and
        # "migrated" is a claim about what no longer exists, which is exactly the
        # kind of claim this file is for.
        "id": "outlander-does-not-hand-order-its-own-composition",
        # Bounded to the external fixture for the same reason the allowlist row
        # is: in-repo apps composing by hand is a MEASUREMENT question the
        # campaign defers, not a rule, and widening this would answer it by
        # accident in a row whose subject is one consumer.
        "paths": ["fixtures/external_consumer/"],
        # The fixture's tests ARE the consumer — a third party exercising the
        # public API — so a test that rebuilds the composition by hand is exactly
        # the second path this forbids, not an exemption from it.
        "include_tests": True,
        "patterns": [
            # The engine ordering rules `ambition::app` now owns. Each of these
            # names is one rule a consumer used to have to know and get right:
            # engine foundation before the groups, engine before host before
            # shell, assets after the content that registers the catalogs and
            # before the presentation that draws them. Between them the fixture's
            # three hand-rolled builders encoded EIGHT such rules, four of which
            # failed SILENTLY when wrong (see `lib.rs`'s builder docs).
            r"\badd_headless_foundation\b",
            r"\binit_engine_states\b",
            r"\bPlatformerEnginePlugins\b",
            r"\bPlatformerHostPlugins\b",
            r"\bPlatformerAssetsPlugin\b",
            r"\bPlatformerPresentationPlugin\b",
            r"\bMinimalShellPlugins\b",
            # Bevy's own group: `PlatformerApp` decides windowed-vs-headless and
            # which five plugins a GPU-less window disables. A consumer adding
            # `DefaultPlugins` itself has taken that decision back.
            r"\bDefaultPlugins\b",
        ],
        "reason": (
            "The fixture had three hand-ordered builders totalling ~110 lines "
            "(`build_outlander_app`, `build_outlander_rollback_app`, "
            "`build_windowed_app`) plus two shared helpers, and between them they "
            "restated eight engine ordering rules — four of which failed "
            "silently: an asset source registered after `AssetPlugin` sealed its "
            "sources, a GPU-less window missing one of five disables, engine "
            "groups before `init_engine_states`, a host naming no initial route "
            "and therefore preparing nothing. All three now go through "
            "`ambition::app::PlatformerApp`, and `src/bin/dump.rs` — the last "
            "hand-ordered path, which also installed the WINDOWED host in a "
            "headless dump — was retired with it. Reintroducing any of these "
            "names in the consumer means a second composition exists, which is "
            "the state slice A4 is defined as ending. Retiring them is also what "
            "closed `ambition::engine` and `ambition::windowed_host` on the A1 "
            "ratchet, so a regression here shows up in two places."
        ),
    },
    {
        "id": "rollback-exit-oracle-is-not-quarantined",
        # The one contract whose SUBJECT is a test file. Everything else is about
        # production code, so tests are excluded by default — see `violations`.
        "include_tests": True,
        "paths": ["game/ambition_app/tests/rollback_exit_oracle.rs"],
        "patterns": [
            # An `#[ignore]` here is either opt-in TOOLING (a bisection you run
            # when the oracle is red) or a DISABLED GUARD. Only the second is
            # the absence this contract is about, and the reason string is what
            # tells them apart — so the contract requires one, which also makes
            # "why is this off?" answerable without reading the test body.
            {
                "grep": r"#\[ignore",
                "match": r"#\[ignore(?!\s*=\s*\"(?:diagnostic|audit|measurement)\b)",
            },
            "known GGRS divergence",
        ],
        "reason": (
            "This oracle guards a KNOWN determinism failure, so an ordinary "
            "green run has to include it. It was `#[ignore]`d and red for a "
            "long time; the file now explains that history in prose, and the "
            "prose must not be mistaken for the attribute coming back "
            "(queue A6). Release blocker for rollback multiplayer. A new "
            "`#[ignore]` is allowed only as `diagnostic`/`audit`/`measurement` "
            "tooling — anything else is a guard being switched off, which is "
            "queue E1's rule applied where it was learned."
        ),
    },
    {
        "id": "fight-tests-do-not-hand-roll-damage",
        "paths": ["crates/", "game/", "fixtures/"],
        "patterns": [r"\b\w*_hp\s*-=", r"\bhp\s*-=\s*\d"],
        "reason": (
            "The two-provider fight test used to compute an AABB overlap by "
            "hand and subtract integers from local variables: no entities, no "
            "`MovePlayback`, no hit events, no `BodyHealth`. It proved the test "
            "could do arithmetic. Damage is asserted through the production "
            "path or not at all (queue A2)."
        ),
    },
    {
        # C3.7 / campaign X14. The catalog's `default_action_set` is the value a
        # `CharacterDefinition`'s authored action set OUTRANKS, and precedence
        # between them is decided in exactly one function. A second reader is a
        # second answer to "what can this body reach for", which is the identity
        # split Campaign 1 exists to close.
        "id": "the-catalog-default-action-set-is-confined-to-one-file",
        "paths": [
            "crates/",
            "game/",
            "fixtures/",
            # The FOLD, the wear-time path for ids nothing registered, and the
            # catalog method being read.
            ":!crates/ambition_actors/src/character_runtime/definition.rs",
            ":!crates/ambition_actors/src/avatar/starting_character.rs",
            ":!crates/ambition_characters/src/actor/character_catalog/mod.rs",
        ],
        "patterns": [r"\bbuild_default_action_set\b"],
        "reason": (
            "`apply_worn_character_kit` is the ONE place the catalog's default "
            "action set is read and weighed against the definition's authored "
            "one (queue C3, 2026-07-28). A second reader forks the precedence, "
            "and the failure is silent: the two answers agree for every "
            "character that authored nothing, which is most of them, so it "
            "presents only on the one character somebody bothered to author — "
            "exactly how the seated-fighter gap hid behind two duelists whose "
            "authored set happened to equal the default. "
            "⚠ 2026-07-29: the fold MOVED to `definition.rs`, where "
            "`finalize_character` performs it once per character at the "
            "preparation barrier. `starting_character.rs` keeps a caller for "
            "the ids nothing REGISTERED — most of the legacy cast — which have "
            "no prepared value to weigh against. Two files, still one decision "
            "per character; the day a registered character is resolved in both "
            "is the day this contract has stopped meaning anything, so read "
            "them together before adding a third."
        ),
    },
    {
        # C3.6 / campaign X12. Every remaining prepared-vs-catalog decision is a
        # NAMED resolver with a documented precedence rule, each called from one
        # file. That is what "no runtime arbitration" means in a tree that still
        # has a legacy catalog as a preparation input: not zero decisions, but no
        # decision made in a second place.
        "id": "the-provider-resolver-is-confined-to-one-file",
        "paths": [
            "crates/",
            "game/",
            "fixtures/",
            ":!crates/ambition_actors/src/character_runtime/presentation.rs",
        ],
        "patterns": [r"\bprovider_of_character\("],
        "reason": (
            "Which provider owns a character — registry first, catalog owners "
            "second — is decided in `presentation.rs` and consumed everywhere "
            "else. A second caller is a second answer, and the failure is a "
            "body emitting in the wrong provider's voice: audible, "
            "attributable to nothing, and only on a crossover stage where two "
            "providers are live at once."
        ),
    },
    {
        "id": "the-catalog-axis-tuning-is-confined-to-one-file",
        # ENGINE crates only, and that is the claim rather than a concession.
        # What must stay singular is the place the ENGINE weighs a catalog
        # tuning against a definition's. A game reading its own catalog is a
        # game reading its own content — Mary-O's inline tests assert her
        # classic-physics numbers that way, and forbidding it would be
        # forbidding a provider to know what it authored.
        "paths": [
            "crates/",
            ":!crates/ambition_actors/src/character_runtime/definition.rs",
            ":!crates/ambition_actors/src/avatar/starting_character.rs",
            ":!crates/ambition_characters/src/actor/character_catalog/mod.rs",
        ],
        "patterns": [r"\baxis_tuning\("],
        "reason": (
            "The catalog's movement feel is read in one place and weighed "
            "against the definition's there. This one nearly broke on the "
            "commit that introduced it: the seated projection first read the "
            "prepared value DIRECTLY, so for a character with catalog tuning "
            "and no authored tuning the worn path inserted the marker and the "
            "projection removed it on the same tick. Two paths answering one "
            "question, reintroduced by the commit closing that exact failure. "
            "⚠ 2026-07-29: `definition.rs` joined the allowed set because the "
            "fold moved to the barrier — and the projection's half of that old "
            "bug is GONE with it, because there is no longer a prepared value "
            "the projection could read `None` from while the worn path read a "
            "row."
        ),
    },
    {
        "id": "the-movement-tuning-resolver-is-confined-to-one-file",
        "paths": [
            "crates/",
            "game/",
            "fixtures/",
            # The ONE production caller, and the re-export that lets it be one.
            ":!crates/ambition_actors/src/avatar/starting_character.rs",
            ":!crates/ambition_actors/src/avatar/mod.rs",
        ],
        "patterns": [r"\bmovement_tuning_for_character\("],
        "reason": (
            "`movement_tuning_for_character` resolves prepared-vs-catalog for a "
            "body's movement tuning, and it is one of the four surviving "
            "character resolvers Campaign 1 decided to KEEP rather than collapse "
            "(X12: they answer different fields on different cadences, and one "
            "universal resolver is the premature abstraction the review warns "
            "against). Keeping four is only safe while each has exactly one "
            "caller — the campaign's content is a guard that no FIFTH appears. "
            "Found 2026-07-28 by verifying the claim that every resolver was "
            "pinned: three were, this one was not, and an unguarded resolver is "
            "the one a second caller grows on."
        ),
    },
    {
        "id": "the-motion-model-resolver-is-confined-to-one-file",
        "paths": [
            "crates/",
            "game/",
            "fixtures/",
            ":!crates/ambition_actors/src/avatar/starting_character.rs",
        ],
        "patterns": [r"\bmotion_model_spec_for_character\("],
        "reason": (
            "The definition-first movement policy resolver (R-a, 2026-07-28). "
            "The catalog-only `motion_model_spec_for_character_id` is "
            "deliberately NOT covered — it is the fallback this one calls, and "
            "two tests plus a from-scratch bundle legitimately have no registry "
            "to consult. What must stay singular is the place that WEIGHS them."
        ),
    },
]

# Files whose content is ABOUT the contracts rather than governed by them.
SELF_REFERENTIAL = {"scripts/check_absence_contracts.py"}

# Dependency-edge contracts, read from `cargo metadata` rather than from text.
#
# A grep cannot express "crate A must not depend on crate B". It can find the
# `use` that proves it, and miss the one added through a re-export tomorrow; it
# cannot see a dependency introduced through an intermediary at all. The manifest
# graph is the fact, so this asks the manifest (GPT 5.6's W1 note, 2026-07-28:
# "use Cargo metadata for dependency edges").
#
# `forbidden` is checked TRANSITIVELY. The claim being guarded is never "no
# direct dependency line" — it is that a foundation crate cannot REACH gameplay,
# and reaching it through one intermediary is the same architectural failure with
# an extra hop. A layering inversion almost never arrives as a direct edge.
DEPENDENCY_CONTRACTS: list[dict] = [
    {
        "id": "engine-core-is-the-floor",
        "crate": "ambition_engine_core",
        "forbidden": "*",
        "reason": (
            "The geometry, movement and body vocabulary every other crate is "
            "written in terms of. It depends on NO workspace crate, and that is "
            "what makes it the layer everything else can agree on rather than "
            "one more participant in a cycle. A single edge out of here makes "
            "the whole graph a suggestion."
        ),
    },
    {
        "id": "platformer-primitives-stays-a-foundation",
        "crate": "ambition_platformer_primitives",
        "forbidden": [
            "ambition_actors",
            "ambition_characters",
            "ambition_combat",
            "ambition_runtime",
            "ambition_content",
            "ambition",
        ],
        "reason": (
            "Session scope, binding resolution and stable ids — the vocabulary "
            "gameplay crates USE. It sits directly above `engine_core` and "
            "below everything that has opinions about actors, so a dependency "
            "on one of those inverts the layering the refactor timeline is "
            "built on (foundations < gameplay_core < content)."
        ),
    },
    {
        "id": "characters-do-not-depend-on-the-actor-integration-layer",
        "crate": "ambition_characters",
        "forbidden": ["ambition_actors", "ambition_runtime", "ambition"],
        "reason": (
            "`ambition_actors` depends on `ambition_characters`, which makes "
            "the reverse edge a cycle waiting to be discovered by the compiler "
            "at the worst moment. It also matters for the deferred "
            "`ambition_actors` decomposition: if a coherent actor kernel exists "
            "at all, `ambition_characters` is below it."
        ),
    },
    {
        "id": "engine-crates-do-not-consume-the-umbrella-facade",
        "crate": "ambition_actors",
        "forbidden": ["ambition"],
        "reason": (
            "`ambition` is the facade a CONSUMER builds a game against; it "
            "re-exports `ambition_actors` among thirty-odd others. An engine "
            "crate reaching back through it is circular by construction, and it "
            "is how a headless consumer ends up compiling the render stack. "
            "⚠ deliberately scoped to engine crates: `ambition_content` DOES "
            "depend on the facade today, and whether that should stop is a "
            "MEASUREMENT question the campaign defers rather than a rule."
        ),
    },
]

# The third table: what a CONSUMER is allowed to name through the facade.
#
# The other two tables forbid specific things. This one permits specific things
# and forbids the rest, and the difference is not stylistic — it is the whole
# reason the row exists. ADR 0031: `ambition` is a namespace mirror today, so
# **a denylist always lags it.** The first draft of the campaign forbade six
# modules. Outlander names eighteen. That contract would have gone green with
# twelve leaks still open, which is worse than no contract, because a green
# contract is believed.
#
# ⚠ TWO invariants, and the second is what makes this a RATCHET rather than a
# count. api-growth-method.md §5: *"a ratchet on a COUNT is not one, because a
# count permits deleting one entry and adding another. Freeze the SET."*
#
#   1. `named ⊆ allowed ∪ baseline` — the consumer may not name a NEW module.
#   2. `baseline ⊆ named`           — the baseline may not keep an entry the
#                                     consumer has stopped naming.
#
# Invariant 2 is the one people leave out. Without it the baseline is a budget:
# migrate `time` away, leave `time` in the list, and the eighteenth slot is now
# free for something else to occupy silently. With it, a migration MUST prune
# its entry in the same commit — and a pruned entry can never come back, because
# invariant 1 then rejects it. That is the ratchet: monotone, per-member, and
# it closes at zero.
#
# `allowed` is EMPTY on purpose. It holds the reviewed public SDK surface, and
# campaign §A1 says the exact public module names stay provisional until A2 is
# accepted. An allowlist populated with guesses before the call sites are
# written would be the campaign designing the API from the module list, which is
# precisely the sequencing ADR 0031 rejects.
MODULE_ALLOWLISTS: list[dict] = [
    {
        "id": "outlander-names-only-the-public-sdk",
        # Slice A is BOUNDED to the external fixture (Jon, 2026-07-30).
        # `game/ambition_content` also depends on the facade, and ADR 0031's
        # dependency contract deliberately records that as a MEASUREMENT
        # question the campaign defers rather than a rule. Widening these paths
        # would answer it by accident, in a row whose subject is host
        # composition.
        "paths": ["fixtures/external_consumer/"],
        # The fixture's `tests/` ARE the consumer. Elsewhere a test calling a
        # resolver is not a second authority, so `is_test_path` excludes it; here
        # the tests are a third party exercising the public API, which is exactly
        # the population being measured. It happens not to change the SET today —
        # every module the tests name, `src/` names too — but it changes the
        # counts, and the counts are §2a's cost proxy.
        "include_tests": True,
        "facade": "ambition",
        # THE PUBLIC SDK, as of slice A. `ambition::app` is the host-composition
        # facade `docs/sdk/api-prototype.md` §5 specifies (`PlatformerApp`,
        # `SessionMode`, `AssetSource`, `GameModule`, `ModuleManifest`,
        # `ModuleDraft`, and `app::prelude`). It is the one name a consumer may
        # reach for that is a PROMISE rather than a mirror of our crate list —
        # which is the whole distinction this contract measures.
        #
        # ⚠ Adding a name here is a compatibility commitment, not a way to make
        # the ratchet green. The test that reads this table cannot tell the two
        # apart, so the review is the gate: a module belongs here only once
        # `docs/sdk/api-prototype.md` names it as SDK surface.
        #
        # §5 also lists `ambition::experience`; the implementation put
        # `GameModule`/`ModuleManifest`/`ModuleDraft` in `ambition::app` beside
        # `PlatformerApp` instead of splitting them, so `experience` does not
        # exist and is deliberately NOT pre-registered here. An allowlist entry
        # for a module nothing names is exactly the stale entry invariant 2
        # forbids.
        # The reviewed SDK surface, kept IDENTICAL across consumers — a module
        # that is a promise to one game is a promise to all of them, and a
        # per-consumer allowlist would let the same name be public here and a
        # leak there.
        #
        # `app` (slice A), `world` (slice C, once the facade stopped mirroring
        # it), and `actor`/`sim`/`view` (slice C) — each a CLOSED list, not a
        # crate re-export. `bevy` is the facade's documented re-export.
        "allowed": {"actor", "app", "bevy", "character", "sim", "view", "world"},
        # Measured 2026-07-30 by this script, not transcribed from the campaign.
        # ⚠ The campaign and ADR 0031 both said NINETEEN while listing eighteen
        # names. There are eighteen. Both documents were corrected in the commit
        # that added this table; the instrument is the authority for its own
        # baseline, because a baseline copied out of prose is a ratchet nobody
        # measured.
        # ⚠ MEASURED after each migration, never edited to make a run green.
        # Slice A4 retired exactly the four `docs/sdk/api-prototype.md` §5
        # predicted — `engine`, `game_assets`, `presentation`, `windowed_host` —
        # taking 18 to 14. The prediction was written down BEFORE A4 ran and the
        # instrument reported the same number, which is the only version of that
        # exercise worth anything: 14 against a remembered guess of 12 would have
        # taught nothing.
        #
        # Six of `asset_manager`'s eight uses closed and the module STAYS, because
        # module granularity is a coarse unit that reports progress late. That is
        # the right direction for a gate to err in; §2a of the growth method
        # carries the per-path counts beside it for the finer picture.
        # ⚠ 14 -> 11, slice C. `engine_core` and `platformer` RETIRED into the
        # curated `actor`/`sim`/`view` modules; `world` moved to `allowed` once
        # the facade stopped mirroring the crate. Pruned in the migrating
        # commit, because invariant 2 went STALE-red and named both.
        #
        # What is left is not composition. `runtime` is 13 uses of
        # `rollback::*` — the session knob ADR 0031 defers to its own slice —
        # and the rest is content and gameplay vocabulary that needs its own
        # derivation rather than the same treatment applied eleven times.
        # ⚠ 11 -> 7. `actors`, `characters`, `sprite_sheet` and `entity_catalog`
        # RETIRED into the curated `character`/`actor`/`view` modules. Authoring
        # ONE character used to mean naming four mirrored crates — the catalog
        # in one, its runtime load state in another, its art in a third, its
        # brain in a fourth — because those are the engine's internal
        # boundaries and the facade published them.
        # ⚠ 18 -> 1 across slices A-C. What is left is `ambition::runtime`, and
        # every one of its ten uses is `rollback::*`.
        #
        # It stays. ADR 0031's Deferred section is explicit that rollback as a
        # public knob is "a far larger promise than a clock — frozen schema,
        # complete authoritative baseline, stable participants, deterministic
        # activation, lifecycle rebasing, confirmation boundaries", with its own
        # slice and its own acceptance tests. Curating it into
        # `ambition::rollback` would make exactly that promise through the back
        # door, and the ratchet reaching zero is not a good enough reason to
        # make a promise the campaign deliberately deferred.
        #
        # This is the one entry that must NOT be closed by the technique that
        # closed the other seventeen.
        "baseline": {"runtime"},
        "reason": (
            "A game depends on `ambition`, and `ambition` is currently the list "
            "of crates the engine happens to be built from — so a consumer's "
            "imports encode our implementation topology and we cannot move an "
            "implementation without breaking them (ADR 0031). Outlander reaches "
            "through the facade for `ambition::runtime::rollback::put_f32`: a "
            "third party building a game is naming an internal serialisation "
            "helper. Each name in `baseline` is one leak still open. The set may "
            "not GAIN a member, and it may not KEEP one the consumer has stopped "
            "naming — see the two invariants above. Zero means consumers name "
            "only the SDK."
        ),
    },
    {
        "id": "minimal-game-names-only-the-public-sdk",
        # Consumer-matrix row 2, added by slice B. The SECOND consumer gets its
        # own ratchet from the start, because a second consumer that may name
        # whatever it likes is not a proof of anything — it is a second way to
        # be shaped like one game.
        "paths": ["fixtures/minimal_game/"],
        "include_tests": True,
        "facade": "ambition",
        # `app` is the SDK. `bevy` is the facade's deliberate re-export — its
        # own doc comment commits to it ("so a game can name bevy TYPES through
        # `ambition::bevy`"), and this game proves the commitment is worth
        # something: it needs NO `bevy` entry in its manifest at all, because it
        # derives nothing. Outlander does. That difference is only visible with
        # two consumers.
        # `world` JOINED 2026-07-30, and only after the facade stopped mirroring
        # it. `pub use ambition_world as world` published every submodule the
        # crate happened to have; `pub mod world { ... }` publishes a CLOSED
        # list, so a new submodule is an internal change until somebody adds it
        # on purpose. That is the difference between a promise and an accident,
        # and it is what makes this entry honest rather than a way to make the
        # number smaller.
        "allowed": {"actor", "app", "bevy", "character", "sim", "view", "world"},
        # Measured 2026-07-30 against the crate as first written. FOUR, against
        # Outlander's fourteen — and the four are not a smaller sample of the
        # same problem, they are one specific hole: `PlatformerExperienceAuthoring`
        # + `PreparedPlatformerSource` + `RoomSpec` + the `engine_core` geometry
        # vocabulary. A minimal game can now COMPOSE through the SDK and still
        # cannot DECLARE a room or an experience through it. That is a measured
        # leak with a named boundary, which is exactly what §3 wants for
        # selecting the next slice.
        # ⚠ FIVE, not the four first recorded — and the correction is the finding.
        #
        # The first baseline was measured against a game that COMPILED. It did
        # not RUN: the host sat in `HostStatus::Activating` for 600 ticks and
        # never started, because preparation validation refuses an experience
        # whose provider registered no explicit audio fragment. A movement-only
        # game with no sound must still DECLARE its silence, so `ambition::audio`
        # is a fifth module every game names no matter how small.
        #
        # The lesson is about the instrument, not the number: a consumer's
        # baseline must be measured against a WORKING consumer. Measured against
        # a compiling one it reads low, and reads low in the flattering
        # direction. The ratchet caught the growth on its first live use, which
        # is the only reason this is a corrected number rather than a wrong one.
        # 5 -> 3. `provider` and `runtime` retired when `ModuleDraft::playable`
        # absorbed the experience declaration: the engine assembles the
        # `PreparedPlatformerSource` and installs the authoring, so a game no
        # longer writes `ambition::runtime::demo_fixture` into its own imports.
        # (A module literally named `demo_fixture` in a shipped game's
        # dependency list is the namespace mirror confessing.)
        #
        # PRUNED IN THE MIGRATING COMMIT, which is invariant 2's whole point —
        # it went STALE-red and named both, so the slots cannot be reoccupied
        # silently.
        # 3 -> 2. `engine_core` retired when `ambition::world::prelude` landed:
        # a game describing a floor no longer reaches into an implementation
        # crate named `engine_core` for `Vec2`/`Block`.
        #
        # ⚠ `world` is deliberately STILL BASELINE, not promoted to `allowed`.
        # ADR 0031's public module list does name `ambition::world`, and this
        # game now touches only its curated prelude — but the facade still
        # re-exports the WHOLE crate (`pub use ambition_world as world`), so
        # blessing the name would commit us to every path under it, which is
        # exactly the namespace mirror the campaign exists to end. It moves to
        # `allowed` when the facade turns it into a curated module. Making a
        # number smaller is not a reason to promise something.
        # 2 -> 1. `audio` retired with `ModuleDraft::no_audio()`: declaring
        # silence is a word on the draft now, not a hand-registered fragment.
        #
        # ⚠ EMPTY. The movement-only minimal game names ONLY reviewed SDK
        # surface: `ambition::app`, `ambition::world`, and the facade's
        # documented `bevy` re-export.
        #
        # This is §4's first terminal condition reached for ONE consumer. It is
        # not the campaign's terminal condition — Outlander still names 14, and
        # four consumer-matrix categories are unproven — but it is the first
        # evidence that "consumers name only the SDK" is a reachable state
        # rather than an aspiration.
        #
        # Zero here means the ratchet now guards a PROPERTY instead of tracking
        # a migration: any new `ambition::` module this game names is a
        # regression, full stop.
        "baseline": set(),
        "reason": (
            "The movement-only minimal game is the consumer-matrix row Outlander "
            "structurally cannot fill: it asks for almost nothing, so whatever it "
            "still has to name is a floor on what EVERY game must know. Four "
            "modules, all of them the room/experience declaration path — since "
            "reduced to ZERO by slices B and C. The set "
            "may not GAIN a member, and it may not KEEP one this game stops "
            "naming."
        ),
    },
]

# ── The campaign's SECOND ratchet: central rollback ownership ────────────────
#
# api-1.0-campaign.md §Ratchets specified this when the campaign was written and
# it was never built. Slice F needs it, because federating rollback ownership
# without a ratchet is a migration with nothing watching it.
#
# Same two invariants as the module allowlist, for the same reason:
#
#   1. current ⊆ frozen  — no NEW stable name or codec may enter the CENTRAL
#                          registration. New domain state registers itself.
#   2. frozen ⊆ current  — a name that has left must be PRUNED, or the baseline
#                          is a budget and the vacated slot fills silently.
#
# ⚠ Frozen as a SET, never as a count. The campaign's own words: "Freezing only
# the NUMBER of central rollback registrations permits deleting one and adding
# another." Zero means `ambition_runtime` is no longer the implementation owner
# of every domain's snapshot — the state
# `impl SnapshotState for ambition_actors::…::MatchSeat` describes today.
ROLLBACK_SCHEMA_BASELINE = (
    "docs/planning/engine/slice-evidence/rollback-schema-baseline.json"
)


def rollback_schema_usage(root: Path) -> dict[str, list[str]]:
    """The stable schema names and codecs `ambition_runtime` owns today."""
    registration = (
        root / "crates/ambition_runtime/src/rollback/mod.rs"
    ).read_text(errors="replace")
    codecs = (
        root / "crates/ambition_runtime/src/rollback/codecs.rs"
    ).read_text(errors="replace")
    return {
        "stable_schema_names": sorted(
            set(re.findall(r'"([a-z_]+\.[a-z_.]+)"', registration))
        ),
        "central_codecs": sorted(
            set(re.findall(r"impl SnapshotState for ([A-Za-z0-9_:<>]+)", codecs))
        ),
    }


def rollback_schema_violations(root: Path) -> tuple[list[str], list[str]]:
    """`new, stale` — invariant 1's breaches and invariant 2's."""
    baseline = json.loads((root / ROLLBACK_SCHEMA_BASELINE).read_text())
    current = rollback_schema_usage(root)
    new: list[str] = []
    stale: list[str] = []
    for key in ("stable_schema_names", "central_codecs"):
        frozen = set(baseline[key])
        live = set(current[key])
        new.extend(f"{key}: {item}" for item in sorted(live - frozen))
        stale.extend(f"{key}: {item}" for item in sorted(frozen - live))
    return new, stale


_LINE_COMMENT = re.compile(r"//.*$")
_HASH_COMMENT = re.compile(r"#(?!\[).*$")
_BLOCK_COMMENT = re.compile(r"/\*.*?\*/", re.DOTALL)


def is_test_path(path: str) -> bool:
    """Whether `path` is test code rather than production code.

    The module docstring has always claimed these contracts "search production
    source only", and the path lists did not enforce it — they happened not to
    match test code, which is not the same thing. The first contract that named a
    function a test legitimately calls proved the gap by flagging its own test
    (2026-07-28).

    A test calling a resolver is not a second authority; it is a test. What the
    contracts are about is who DECIDES in the shipped binary.

    Opt back in with `"include_tests": True` — exactly one contract needs it,
    because its subject IS a test file, and defaulting the other way would have
    silently disabled it.

    ⚠ **This is a PATH test, and this repo also writes tests inline.** A
    `#[cfg(test)] mod` inside a production `lib.rs` is invisible here — found
    2026-07-28 when a contract flagged Mary-O's own inline physics assertions.
    Line-local comment stripping cannot see a module boundary, so the honest
    statement is "production PATHS only", not "production code only". A contract
    whose symbol is legitimately named by inline tests has to narrow its paths
    instead, and say why.
    """
    return (
        "/tests/" in path
        or path.endswith("/tests.rs")
        or path.endswith("_tests.rs")
        or "/tests/" in path
        or path.endswith("_test.rs")
    )


def strip_comments_for(path: str, line: str) -> str:
    """Return `line` with comment text removed, so prose cannot match a pattern.

    Deliberately line-local and deliberately crude. A multi-line `/* */` body
    survives only if it contains no `//`, and the one shape that matters — a
    `///` or `//!` paragraph naming the thing that was removed — is removed
    exactly. Being conservative in the other direction (treating code as
    comment) would HIDE a real violation, so nothing here strips code.

    `#` is a comment in shell and Python and not in Rust, where it opens an
    attribute — and `#[ignore]` is precisely what one contract looks for. So the
    hash rule is applied by file type rather than universally.
    """
    stripped = _BLOCK_COMMENT.sub(" ", line)
    stripped = _LINE_COMMENT.sub("", stripped)
    if not path.endswith((".rs", ".toml")):
        stripped = _HASH_COMMENT.sub("", stripped)
    return stripped


def git_grep(pattern: str, paths: list[str], root: Path) -> list[tuple[str, int, str]]:
    """Every `path, line number, text` match for `pattern` under `paths`."""
    command = ["git", "grep", "-n", "-I", "-E", pattern, "--", *paths]
    result = subprocess.run(
        command, cwd=root, capture_output=True, text=True, check=False
    )
    # git grep exits 1 for "no matches", which is the outcome this file wants.
    if result.returncode not in (0, 1):
        raise RuntimeError(f"git grep failed: {result.stderr.strip()}")
    hits = []
    for raw in result.stdout.splitlines():
        parts = raw.split(":", 2)
        if len(parts) != 3:
            continue
        path, number, text = parts
        try:
            hits.append((path, int(number), text))
        except ValueError:
            continue
    return hits


def violations(contract: dict, root: Path) -> list[tuple[str, int, str]]:
    """Every production line that violates `contract`, comments excluded.

    A pattern is either a string (git grep and the confirming match are the same
    expression) or a `{"grep": ..., "match": ...}` pair. The pair exists because
    `git grep -E` is POSIX ERE and has no lookaround: the coarse expression finds
    candidate lines cheaply and the precise Python one decides. Splitting them is
    what lets a contract say "an ignore WITHOUT a diagnostic reason" instead of
    settling for "an ignore", which is the difference between a contract that
    survives and one that gets waived.
    """
    found: list[tuple[str, int, str]] = []
    include_tests = contract.get("include_tests", False)
    for pattern in contract["patterns"]:
        if isinstance(pattern, str):
            grep, confirm = pattern, pattern
        else:
            grep, confirm = pattern["grep"], pattern["match"]
        compiled = re.compile(confirm)
        for path, number, text in git_grep(grep, contract["paths"], root):
            if path in SELF_REFERENTIAL:
                continue
            if not include_tests and is_test_path(path):
                continue
            if compiled.search(strip_comments_for(path, text)):
                found.append((path, number, text.strip()))
    found.sort()
    return found


def _leading_identifier(text: str) -> list[str]:
    """The identifier a use-tree item starts with, or nothing."""
    match = re.match(r"\s*([A-Za-z_][A-Za-z0-9_]*)", text)
    # `self` re-exports the facade root, which names no module.
    if not match or match.group(1) == "self":
        return []
    return [match.group(1)]


def use_tree_heads(text: str, index: int) -> list[str]:
    """The top-level module names a use tree introduces at `index`.

    ⚠ **This exists because a line regex is EVADABLE, and silently.**
    `ambition::time` is easy to match. `use ambition::{time::Foo, audio::Bar};`
    is the same two leaks written in idiomatic Rust, and a
    `\\bambition::([a-z_]+)` pattern matches neither of them — it sees `{` and
    stops. The fixture happens to contain no brace-grouped facade import today,
    so a line regex would have been green, correct, and wrong the first time
    somebody wrote ordinary Rust.

    Forbidding the braced form was the cheaper fix and it was rejected: a
    contract that outlaws standard syntax to keep its own parser simple is a
    contract that gets waived. So the tree is parsed. Only the depth-1 heads
    matter — `{a::{b, c}, d}` names `a` and `d` — which is why this needs no
    recursion into nested groups.
    """
    while index < len(text) and text[index].isspace():
        index += 1
    if index >= len(text):
        return []
    if text[index] != "{":
        return _leading_identifier(text[index:])

    heads: list[str] = []
    depth = 0
    item: list[str] = []
    for position in range(index, len(text)):
        character = text[position]
        if character == "{":
            depth += 1
            if depth == 1:
                item = []
                continue
        elif character == "}":
            depth -= 1
            if depth == 0:
                heads.extend(_leading_identifier("".join(item)))
                return heads
        if depth == 1 and character == ",":
            heads.extend(_leading_identifier("".join(item)))
            item = []
            continue
        item.append(character)
    # An unterminated group is malformed Rust; take what was readable rather
    # than reporting the file as clean.
    heads.extend(_leading_identifier("".join(item)))
    return heads


def facade_modules(text: str, facade: str) -> list[tuple[int, str]]:
    """Every `offset, top-level module` named through `facade::` in `text`."""
    found: list[tuple[int, str]] = []
    for match in re.finditer(rf"\b{re.escape(facade)}::", text):
        for head in use_tree_heads(text, match.end()):
            found.append((match.start(), head))
    return found


def allowlist_usage(contract: dict, root: Path) -> dict[str, list[tuple[str, int]]]:
    """Every facade module the consumer names, mapped to where it names it.

    Whole-file rather than line-by-line, because a use tree spans lines and the
    heads have to be attributed to the `ambition::` that introduced them.
    Comments are stripped with the same line-local helper the other tables use,
    so the prose recurrence this module exists to survive is survived here too:
    a doc comment naming `ambition::runtime` is not a consumer naming it.
    """
    listed = subprocess.run(
        ["git", "ls-files", "--", *contract["paths"]],
        cwd=root,
        capture_output=True,
        text=True,
        check=True,
    ).stdout.split()

    usage: dict[str, list[tuple[str, int]]] = {}
    for relative in listed:
        if not relative.endswith(".rs"):
            continue
        if not contract.get("include_tests", False) and is_test_path(relative):
            continue
        try:
            raw = (root / relative).read_text(errors="replace")
        except OSError:
            continue
        stripped = "\n".join(
            strip_comments_for(relative, line) for line in raw.splitlines()
        )
        starts = [0]
        for line in stripped.splitlines():
            starts.append(starts[-1] + len(line) + 1)
        for offset, module in facade_modules(stripped, contract["facade"]):
            number = max(1, bisect_right(starts, offset))
            usage.setdefault(module, []).append((relative, number))
    return {module: sorted(where) for module, where in sorted(usage.items())}


def allowlist_violations(
    contract: dict, usage: dict[str, list[tuple[str, int]]]
) -> tuple[list[str], list[str]]:
    """`new, stale` — invariant 1's breaches and invariant 2's."""
    named = set(usage)
    allowed = set(contract["allowed"])
    baseline = set(contract["baseline"])
    return sorted(named - allowed - baseline), sorted(baseline - named)


def cargo_binary() -> str:
    """`cargo`, found even when PATH does not have it.

    This runs from the goal-guard hook, and the hook's PATH has no cargo — a
    lesson this repo paid for twice, because a check that can only ever report
    "command not found" can never pass and wedges the run it was supposed to
    guard. So the rustup location is tried before giving up on PATH.
    """
    rustup = Path.home() / ".cargo" / "bin" / "cargo"
    return str(rustup) if rustup.exists() else "cargo"


def workspace_graph(root: Path) -> dict[str, set[str]]:
    """Every workspace crate's DIRECT workspace dependencies, from the manifests.

    `--no-deps` keeps this to the workspace: registry crates are somebody else's
    layering problem. Dev-dependencies are included deliberately — a test that
    reaches upward compiles the upward edge, and "only in tests" is exactly the
    excuse under which a layering inversion first arrives.
    """
    raw = subprocess.run(
        [cargo_binary(), "metadata", "--no-deps", "--format-version", "1"],
        cwd=root,
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    metadata = json.loads(raw)
    members = {package["name"] for package in metadata["packages"]}
    return {
        package["name"]: {
            dependency["name"]
            for dependency in package["dependencies"]
            if dependency["name"] in members
        }
        for package in metadata["packages"]
    }


def reachable(graph: dict[str, set[str]], start: str) -> dict[str, list[str]]:
    """Every crate `start` can reach, mapped to the path that gets there.

    The path is the whole value of reporting this: "A depends on B" is arguable,
    "A -> C -> B" is a fix.
    """
    found: dict[str, list[str]] = {}
    queue = [(start, [start])]
    while queue:
        crate, path = queue.pop(0)
        for dependency in sorted(graph.get(crate, ())):
            if dependency in found or dependency == start:
                continue
            found[dependency] = path + [dependency]
            queue.append((dependency, path + [dependency]))
    return found


def dependency_violations(contract: dict, graph: dict[str, set[str]]) -> list[str]:
    crate = contract["crate"]
    if crate not in graph:
        return [f"contract names `{crate}`, which is not a workspace member"]
    reached = reachable(graph, crate)
    forbidden = contract["forbidden"]
    targets = sorted(reached) if forbidden == "*" else forbidden
    return [
        " -> ".join(reached[target]) for target in targets if target in reached
    ]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check", action="store_true", help="exit 1 when a contract is violated"
    )
    parser.add_argument(
        "--allowlist-open-count",
        action="store_true",
        help=(
            "print the number of baseline modules a consumer still names, and "
            "nothing else — the campaign's progress metric, for a goal check"
        ),
    )
    args = parser.parse_args()

    root = Path(
        subprocess.run(
            ["git", "rev-parse", "--show-toplevel"],
            capture_output=True,
            text=True,
            check=True,
        ).stdout.strip()
    )

    if args.allowlist_open_count:
        # Deliberately the count of BASELINE modules still named, not of every
        # module named: a new module is invariant 1's problem and this number
        # must not be able to rise when one appears. It only ever falls.
        still_open = sum(
            len(set(allowlist_usage(contract, root)) & set(contract["baseline"]))
            for contract in MODULE_ALLOWLISTS
        )
        print(still_open)
        return 0

    broken = 0
    for contract in ABSENCE_CONTRACTS:
        found = violations(contract, root)
        if not found:
            print(f"  ok   {contract['id']}")
            continue
        broken += 1
        print(f"  RED  {contract['id']}")
        print(f"       {contract['reason']}")
        for path, number, text in found:
            print(f"       {path}:{number}: {text}")

    graph = workspace_graph(root)
    for contract in DEPENDENCY_CONTRACTS:
        found = dependency_violations(contract, graph)
        if not found:
            print(f"  ok   {contract['id']}")
            continue
        broken += 1
        print(f"  RED  {contract['id']}")
        print(f"       {contract['reason']}")
        for path in found:
            print(f"       {path}")

    for contract in MODULE_ALLOWLISTS:
        usage = allowlist_usage(contract, root)
        new, stale = allowlist_violations(contract, usage)
        still_open = sorted(set(usage) & set(contract["baseline"]))
        facade = contract["facade"]
        if not new and not stale:
            print(
                f"  ok   {contract['id']}"
                f"  ({len(still_open)} of {len(contract['baseline'])} baseline "
                f"modules still named)"
            )
        else:
            broken += 1
            print(f"  RED  {contract['id']}")
            print(f"       {contract['reason']}")
            for module in new:
                sites = usage[module]
                print(
                    f"       NEW  {facade}::{module} "
                    f"({len(sites)} uses) is in neither the reviewed public "
                    f"surface nor the frozen baseline"
                )
                for path, number in sites:
                    print(f"            {path}:{number}")
            for module in stale:
                print(
                    f"       STALE  {facade}::{module} is in the baseline but "
                    f"the consumer no longer names it — PRUNE it in this "
                    f"commit, or the baseline is a budget and the slot it "
                    f"leaves behind can be filled silently"
                )
        # §2a of the growth method: which paths does the consumer still name,
        # and how many times. Frequency is the crude cost proxy and it is
        # usually right, so the instrument prints it rather than making the
        # next slice go and count by hand.
        if still_open:
            ranked = sorted(
                ((len(usage[module]), module) for module in still_open),
                reverse=True,
            )
            summary = "  ".join(f"{module}:{count}" for count, module in ranked)
            print(f"       still named — {summary}")

    new, stale = rollback_schema_violations(root)
    if not new and not stale:
        baseline = json.loads((root / ROLLBACK_SCHEMA_BASELINE).read_text())
        print(
            f"  ok   central-rollback-ownership-may-not-grow  "
            f"({baseline['stable_schema_name_count']} stable names, "
            f"{baseline['central_codec_count']} codecs still centrally owned)"
        )
    else:
        broken += 1
        print("  RED  central-rollback-ownership-may-not-grow")
        print(
            "       `ambition_runtime` is the implementation owner of every "
            "domain's snapshot, and the campaign's second ratchet freezes that "
            "SET so it can only shrink as ownership federates outward."
        )
        for item in new:
            print(f"       NEW    {item} entered the CENTRAL registration")
        for item in stale:
            print(
                f"       STALE  {item} left the central registration but is "
                f"still in the baseline — PRUNE it in this commit"
            )

    total = (
        len(ABSENCE_CONTRACTS)
        + len(DEPENDENCY_CONTRACTS)
        + len(MODULE_ALLOWLISTS)
        + 1
    )
    if broken:
        print(f"\n{broken} of {total} absence contracts are violated.")
        print(
            "Either the reintroduction is a mistake, or the architecture changed "
            "on purpose — in which case DELETE or INVERT the contract in the same "
            "commit rather than waiving it."
        )
        return 1 if args.check else 0
    print(f"\n{total} of {total} absence contracts hold.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
