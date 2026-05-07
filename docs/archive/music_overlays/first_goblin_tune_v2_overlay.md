# first_goblin_tune_v2 overlay

Adds a simpler adaptive goblin cue source under the v8 renderer tree and an installer that wires its rendered OGG assets into the Rust generated-music player.

This version is intentionally conservative for the demo:

- same Rust section/stem structure: `intro`, `wave1`, `wave2`, `wave3`, `recap_loop`, `outro`
- same stem names: `strings`, `brass`, `winds`, `choir_pad`, `mallets`, `percussion`
- simple drums only; no busy late fills
- lower brass/choir/percussion gains
- runtime crossfades remain active, but the underlying tune is closer to the short goblin cue
