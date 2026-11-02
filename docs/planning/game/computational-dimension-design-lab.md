# Computational dimension design lab

**State:** PROTOTYPE. The first offline inventory and analysis tool is included.
This document is a focused design owner, not an additional execution queue.
No dimension placement in this document is an approved canon change.

## Goal

Use computational methods to find useful creative combinations in Ambition's
worldbuilding notes. Keep human judgment responsible for dimension identities,
physical geography, narrative, traversal, and gate topology.

The main question is not "How many principal components are there?" It is:

> Which combinations of cultures, environments, skills, institutions, and
> physical laws produce concrete dimensions worth exploring?

The world graph is a separate design object from the theme vector space.
A family can belong to several worlds. A dimension can contain several biomes,
societies, and historical periods. The player must be able to explore each
world as connected geography rather than as isolated portal booths.

## Included prototype

Implementation: [`tools/experimental/ambition_worldbuilding_lab/`](../../../tools/experimental/ambition_worldbuilding_lab/README.md).

The prototype contains:

- a catalog of 144 normalized concepts with 15 manually assigned features;
- source categories that keep user ideas separate from unapproved proposals;
- a reproducible PCA and a seeded NMF fit;
- a seeded family-level resampling check;
- a similarity-based neighbor search;
- a family-mixing heuristic for creative prompts;
- world comparison profiles and possible theme-transfer suggestions;
- three exploratory scenario definitions;
- deterministic JSON, CSV, Markdown, and SVG outputs;
- input validation and standalone tests.

The original hand-coded inventory excluded the unassigned catch-all and one
world-topology note from its numerical sample. It retained them in the record
catalog. Therefore the baseline PCA uses 142 concepts in 31 families when the
`all` scope is selected. The `core` scope has fewer entries and excludes the
assistant-generated suggestions.

The original 17-slot roster is preserved as `historical-17`. It includes
separate Pirate and Ninja ideas. A later proposal combines them. This is a
comparison, not a claim that the original topology remains current.

## Hard constraints

1. Preserve **Math** as a distinct Platonic/formal world.
2. Place philosophers where their concrete roles fit. Do not add a dedicated
   philosophy dimension to solve a classification problem.
3. Do not invent travel corridors or metaphysical locations between worlds.
4. Each candidate dimension must have terrain, traversal routes, inhabitants
   or a reason for being sparsely inhabited, and a physical extent.
5. Treat its culture, genre, biome, and physics as separate properties.
6. Do not infer a gate connection from theme-vector proximity.
7. Keep source provenance and approval status visible. An LLM proposal is not
   a maintainer decision. A plan entry is not proof of shipped content.
8. Do not edit `docs/brainstorms/`, current canon, gate addresses, or
   `docs/planning/queue.md` as part of numerical exploration.

## How to use the tool for one design cycle

**1. Add evidence.** Read a focused source. Add new normalized records with
origin, source scope, family, and feature tags. Use a separate supplement file
for experiments. Keep the exact evidence text and file line when available.

**2. Inspect variation.** Run the core PCA and NMF analysis. Use the `all`
scope only to look for useful counterfactual ideas, not to increase confidence
in a user choice. Compare results across scopes and coding revisions.

**3. Create combinations.** Ask for nearest families and balanced mixtures.
The pair score does not invent the world. Write a concrete sketch with a town,
landscape, ordinary routines, local tensions, gameplay systems, and one
interconnected traversable region.

**4. Compare placements.** Put the candidate in one or more test scenarios.
Compare coverage, cohesion, separation, and protected constraints. Check where
it could be a district of another world. A separate dimension is justified by
its physical identity, not by a cluster assignment.

**5. Review by play.** Pick promising regions for LDtk mockups, gameplay
prototypes, and narrative scenes. Use actual play and story coherence to judge
them. Only an explicit design decision can promote a result into canon.

## Current research questions

### Creative culture

- Could the Art world become a world of creative expression and mastery?
- Can music, literature, theater, dance, philosophy, persuasion, cooking, and
  skilled craft share one physical civilization without losing local variety?
- Does learned dexterity fit as an important local culture within this world,
  while martial traditions remain prominent in Pirate/Ninja regions?

### Discovery and technology

- Does Science become a wider civilization of laboratories, engineering,
  planets, space travel, and industry?
- Or does Cyberpunk/Industry have enough social and physical identity to stand
  apart from experimental science?
- Should technology also appear in Home, Evil, and Pirate/Ninja societies?

### Ontology and topology

- Which worlds use ordinary planetary geography, and which use bounded or
  topologically nontrivial manifolds?
- Are large and small worlds different in physical extent and transit costs?
- How do migration, trade, and political control of gates change these worlds?
- Can Math contain a literal Plato's Cave while still remaining the world of
  ideal mathematical objects?

## Next tool improvements, in bounded units

These tasks are research directions. They are not selected queue work.

### A. Evidence-level catalog audit

For each concept, find its exact source passage. Record `source_path`, line
range or commit, speaker, and whether it is a user idea, a later confirmed
choice, or an assistant suggestion. Preserve each distinct claim. Detect
near-duplicate entries without deleting the original evidence. Keep the raw
text outside the numerical model so an agent can check the abstraction.

**Check:** Sample at least 20 records from different source categories. Verify
that each stored citation points to the claimed text. Preserve the source
identity in the report output.

### B. Feature coding review

Test the current 15-feature vocabulary. Consider adding separately measured
axes for physical scale, ecological constraints, cultural complexity,
level traversal, sound/music, philosophical questions, and narrative form.
Have at least two independent coding passes. Measure disagreement instead of
assuming the first pass is correct. Keep alternate labels as versioned data.

**Check:** The report names feature revisions and compares changed components.
An agent can explain why the results moved between revisions.

### C. Multiobjective world construction

Represent each candidate world as a physical-world description plus a set of
families and region examples. Search for partitions or overlapping covers with
separate objectives: source coverage, cultural overlap, mechanical variety,
production cost, and maintainable gate topology. Apply hard constraints before
ranking. Show a Pareto set instead of declaring one best scenario.

**Check:** Protected Math never merges. A philosophy-only world is not
introduced by an automatic operation. Every proposed dimension includes a
physical geography sketch and a route concept.

### D. Topological and geographic model

Store a *different* graph for gate addresses, dimension nodes, and local
regions. Test connectivity, return routes, travel costs, geographic loops,
logistical bottlenecks, and sequence breaks. Do not assume vector similarity
is a gate edge. Keep this graph outside LDtk until it has a stable data model.

**Check:** A world grouping can change without forcing a gate rename. A
geographic graph can have several regions within one dimension.

### E. Human-facing exploration view

Build an optional local HTML report that filters by origin, feature, world,
and scenario. Show why records are neighbors or why a mixture scores well.
Link back to exact evidence. Add a way to stage alternate tags, compare runs,
and export a proposal for review. Do not turn the UI into a second source of
canon.

**Check:** Any report can be regenerated from versioned input plus a recorded
random seed. No network service or game build is needed for the first version.

## Validation

From the repository root:

```sh
TOOL=tools/experimental/ambition_worldbuilding_lab/worldlab.py
python3 "$TOOL" validate
python3 -m unittest discover \
  -s tools/experimental/ambition_worldbuilding_lab/tests -p 'test_*.py' -v
python3 "$TOOL" --scope all analyze --out-dir /tmp/ambition-worldlab-review
```

The first PCA ratios in the `all` scope are about 22.5%, 16.1%, and 14.1%.
These are regression values for the current hand-coded dataset, not design
targets. If an approved annotation changes, update the regression expectation
and record why.

Related material:

- [`docs/brainstorms/original_brainstorm.md`](../../brainstorms/original_brainstorm.md)
- [`docs/brainstorms/good_llm_ideas/original_notes.md`](../../brainstorms/good_llm_ideas/original_notes.md)
- [`docs/planning/game/world-composition.md`](world-composition.md)
- [`docs/planning/game/open-world-roadmap.md`](open-world-roadmap.md)
- [`docs/concepts/brainstorms-design-incubation.md`](../../concepts/brainstorms-design-incubation.md)
