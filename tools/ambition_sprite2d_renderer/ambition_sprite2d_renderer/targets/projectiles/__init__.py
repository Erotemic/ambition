"""Projectile sprite targets.

Projectiles are their own category (not props/entities): unlike a static
entity sprite they are ANIMATED — each one ships a spritesheet whose
frames are the projectile's successive visual states. The glider, for
instance, is a genuine Conway's Game of Life spaceship cycling through
its four phases. Each module here registers as a tack-on target with a
``render()`` that goes through ``authoring.tackon_sheet.build_sheet``, so
the runtime ``SheetRegistry`` consumes the emitted ``*_spritesheet.ron``
exactly like a character sheet.
"""
