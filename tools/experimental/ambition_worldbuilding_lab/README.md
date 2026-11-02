# Ambition Worldbuilding Lab

This is an offline authoring tool for exploring dimension concepts. It reads a
versioned inventory and computes theme patterns. It also compares possible
placements of concept families into concrete worlds.

**It does not change world canon, LDtk maps, or the gate network.**

The original 17-dimension brainstorm is one input. The tool also includes
later world plans, recent user decisions, and unapproved LLM suggestions. The
catalog keeps these sources separate. The data is an initial hand-coded
inventory, not a fully audited transcript of every brainstorm.

## Requirements

Python 3.11 or newer and NumPy 2.x. The tool has no engine or game dependencies.

Run it with `uv`, from the repository root:

```sh
uv run --project tools/experimental/ambition_worldbuilding_lab \
  python tools/experimental/ambition_worldbuilding_lab/worldlab.py validate
```

If NumPy is already installed, `python3 tools/experimental/ambition_worldbuilding_lab/worldlab.py`
works without an environment setup step.

## Commands

These examples use the script path directly. Use the `uv run --project ...`
prefix if NumPy is not installed in the current Python environment.

```sh
TOOL=tools/experimental/ambition_worldbuilding_lab/worldlab.py

# Check the data contract and scenario references.
python3 "$TOOL" validate

# Study user-origin ideas, plans, and direct user revisions only.
python3 "$TOOL" --scope core analyze \
  --out-dir /tmp/ambition-worldlab-core

# Reproduce the earlier 144-item analysis scope, including unapproved ideas.
python3 "$TOOL" --scope all analyze \
  --out-dir /tmp/ambition-worldlab-all --scenario consolidated

# Compare the original 17-slot arrangement with two new arrangements.
python3 "$TOOL" compare

# Find related material, combinations, and alternative homes for ideas.
python3 "$TOOL" neighbors 'Creative culture'
python3 "$TOOL" fuse --anchor 'Creative culture' --limit 12
python3 "$TOOL" placements --scenario consolidated --world creativity

# Add new, explicitly provisional concepts without editing the base catalog.
python3 "$TOOL" --scope all \
  --supplement tools/experimental/ambition_worldbuilding_lab/examples/example_supplement.json \
  analyze --out-dir /tmp/ambition-worldlab-examples

# Focus on families not assigned in the chosen scenario.
python3 "$TOOL" placements --scenario historical-17 --only-unassigned

# Check the numerical code.
python3 -m unittest discover \
  -s tools/experimental/ambition_worldbuilding_lab/tests -p 'test_*.py' -v
```

The `--catalog`, `--scenarios`, `--supplement`, and `--scope` options go
before the command. Each command has its own `--help` text.

The `analyze` command writes these files to the given output directory:

| File | Use |
| --- | --- |
| `report.md` | PCA, NMF, and resampling notes |
| `analysis.json` | Machine-readable parameters and full results |
| `pca.svg` | Labeled, scale-independent two-axis view |
| `family_coords.csv` | Coordinates and feature values per family |
| `fusions.md` | Pairs of ideas with shared and distinct features |
| `placements.md` | Possible theme transfers with current test assignments |
| `world_scenarios.md` | Scores for alternative concrete dimension groupings |

The tool writes only to `--out-dir` during analysis. It does not edit the
source catalog when it runs.

## Input data

- `data/catalog.json` defines 15 features and 144 initial ideas.
- Each record has a stable `id`, a `family`, an `origin`, binary feature `tags`,
  and a `source_scope` that points to places to check.
- `data/world_scenarios.json` defines several test arrangements. Each world
  has an explicit physical setting and associated theme families.
- `examples/example_supplement.json` is **not canon**. Its three records show
  the supported extension format.

Use `--scope core` by default. It includes `original`, `built`, and
`conversation_user` records. Here `built` means that a concept was represented
in a recent repository plan or implementation inventory. It does **not**
verify that every location is playable.

`expanded` also includes later brainstorm material. `all` also includes LLM
suggestions, assistant proposals, and analysis-only hypotheses. All scopes
exclude the unassigned catch-all and world-topology structural note from the
PCA matrix. The original records stay in the catalog.

The initial feature tags were assigned by an agent in one exploratory analysis.
They need review. `source_scope` is a research lead, not a verified quotation
for each concept. Do not treat those tags as observed facts or user approvals.

## Numerical methods

1. For each selected concept, turn its feature tags into a binary vector.
2. Average these vectors within each manually defined family.
3. Give each family one row with equal weight in the PCA matrix.
4. Center the rows and use NumPy SVD for PCA. Report the component directions
   and the proportion of coded variance they explain.
5. Use seeded multiplicative updates for nonnegative matrix factorization
   (NMF). Report the factor-feature weights and reconstruction residual.
6. Resample the family rows for a descriptive stability check. This is not a
   statistical confidence interval for the game's world design.
7. Compute cosine similarities for nearest neighbors, possible transfers,
   and within-world theme cohesion.
8. Rank pairs of theme families with a shared/distinct feature heuristic.
   The score is a way to order questions for exploration, not a quality score.

A scenario comparison reports **coverage**, **within-world similarity**, and
**between-world difference** as separate measures. The tool does not combine
them into one total score and does not select a world count. A low cohesion
score can mean a deliberate cultural mix, not a failure. A high score does not
mean a good level or story.

## Design constraints

- A dimension is a **concrete physical world**. A theme is not a dimension.
- Local travel connects regions of one world. Gates connect worlds.
- The Math world keeps a distinct formal/Platonic identity.
- Do not add a philosophy dimension or an abstract between-world hub.
- It is valid for two worlds to share a culture, idea, or historical influence.
- A tentative candidate remains tentative until the maintainer makes a choice.
- The tool must not write into `docs/brainstorms/`. That directory preserves
  authored brainstorming history.

See [the planning owner document](../../../docs/planning/game/computational-dimension-design-lab.md)
for the design workflow and future work.
