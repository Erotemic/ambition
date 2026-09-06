# Parallax minimal-App test: AssetServer resource validation

Occurrence count: Bevy required-resource validation in minimal test apps — hit #2.

The parallax room-background test added `refresh_parallax_background` to a tiny `App`
without asset plugins. Even though the test set `ActiveParallaxProfile` so the system
should return before touching assets, Bevy validates all `Res<T>` parameters before the
system body runs. A required `Res<AssetServer>` therefore panicked before the early return.

Fix: make the asset server parameter optional in parallax spawn/refresh systems and return
without spawning when it is absent. Production presentation apps still have `AssetServer`;
minimal unit tests can exercise metadata/profile logic without constructing the full asset
stack.
