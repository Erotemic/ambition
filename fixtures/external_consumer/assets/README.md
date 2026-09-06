This file exists so the Outlander fixture has ART OF ITS OWN.

Recorded SDK leak #3 said "consumer-owned art still has no home": a third party
could point the AssetServer at the ENGINE tree or at nothing, so its own
sprites had nowhere to live. `ambition_asset_manager::consumer_source::layered_asset_source`
is the answer — this tree first, the engine tree for anything not authored here.

`outlander_marker.txt` is deliberately a TEXT file rather than a PNG: the claim
under test is that the asset SOURCE resolves out of the consumer tree, and a
generated binary would put a build step between the fixture and the thing it
proves. Its twin, `engine_only_probe.txt`, does NOT exist here — it is read to
show the fallback still reaches the engine tree.
