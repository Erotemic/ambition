"""Ambition LDtk tools.

Modal CLI for safely editing the Ambition sandbox.ldtk: validation,
repair, schema/round-trip checks, and semantic edits (areas, entities,
defs, intgrid layers, links). Agents should not hand-edit the LDtk JSON;
use this package instead so mutations are repaired + validated before
write.
"""
