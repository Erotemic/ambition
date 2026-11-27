# The Elemental Mine

The mine is west of Eureka's Test Range in the Science Dimension. The ore yard stays on the Science map. Its pit stair enters **The Elemental Mine** on the dungeon map.

The yard still sends ore to Eureka. Inez suspended the lower shift after the crusher and the cavern started answering each other. The upper assay desk and the settling basin are in use. The old sample stope gives the player a way to see the rock the crew worked before it reached the natural cavern.

## Routes and encounters

- The main shaft cage serves the pithead, ventilation level, and lower landing. A fixed ladder gives a second route. The assay stairs and ventilation drift meet at opposite ends of the settling basin.
- The basin shuttle runs in both directions. The silt banks give a way out of the pool. The marked seam in the old sample stope hides a 24-coin locker.
- Argon has vertical space at the intake. Bromine attacks low at the exhaust. Bismuth guards the assay shelves. Mercury holds the basin's sediment island. Uranium guards the lower cart spur.
- The vent relief lever opens the lower screen. The service stair release opens a return route from the lower workings to the assay gallery. The last refuge bench is above the Listening Cavern.

## Mother Lode

Watch the lifted arm for the close hammer, jump the floor fracture, and move off the marked spots before the crystals fall. The cavern shelves give a way over the fracture. The fight opens the seam beyond the arena and leaves 65 coins in loose ore. Mother Lode can speak after her shell breaks.

Inez can recognize the encounter result. The exact boundary of future work, the fate of the offending crusher, and the relationship of the mine to the wider Science story stay open. A dungeon visit needs no new traversal ability. The yard, optional locker, and loose ore give the visit a local purpose and result.

## Sources

The map generators are `game/ambition_map_assets/ambition_content/generators/elemental_mine/make_specs.py` and the Eureka generator's `test_range`. Use the latter with `--rooms=eureka_test_range` to update the west connection. The boss profile, encounter settings, and conversations are Ambition content. `game/ambition_content/rhai/mother_lode.rhai` defines the fracture and falling crystal attacks. The backdrop, equipment, and character art are Python sources in the sprite renderer.
