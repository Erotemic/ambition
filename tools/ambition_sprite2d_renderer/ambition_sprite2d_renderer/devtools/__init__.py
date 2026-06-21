"""Developer tools for the sprite renderer.

Author-facing utilities that inspect or verify rendered output rather
than produce runtime assets. Kept out of the render path so the
PIL-only ``core`` stays dependency-light.

- ``debug_hitboxes`` — overlay per-animation hurt/hit boxes on a
  rendered spritesheet to verify they line up with the visible pose.
"""
