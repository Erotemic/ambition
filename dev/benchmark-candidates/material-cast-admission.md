# Validate a cast without constructing discarded geometry

Collision casts need numeric admission for their source and endpoint maps.
They do not need a new projected shape for each admission check. Keep the
validation separate from geometry construction and share the same bounds and
resolution rules. Validate each distinct endpoint map; an admitted start does
not prove that the endpoint keeps enough coordinate resolution.

A translation of an admitted similarity map keeps its scale and orthogonal
basis. Check the new translation and its inverse instead of checking the
basis again. Keep the inverse check: a finite translation can still overflow
when mapped through the inverse scale.

The desktop capture `desktop-autoplay-native-20261118T153023Z` placed
`map_volume` and similarity-map construction in the navigation rollout hot
path. A VM probe with the production core at optimization level 3 measured
1.30–2.60 times the cast preparation rate after these changes. Moving
axis-aligned boxes measured 1.35–1.54 times the rate. Bounds checksums matched.
This is a kernel result, not a whole-frame performance result.

The probe covers boxes, rotated boxes, circles, and convex polygons in world
and mapped charts, with zero and nonzero displacement. It uses nine samples
per case and version, with 100,000 casts per sample. The core behavior suite
also checks numeric refusal and contact behavior.
