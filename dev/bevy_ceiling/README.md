# bevy_ceiling

The fastest a Bevy 0.19.1 window can run on the machine in front of you, with
nothing in it. Run it before believing any game FPS number: a game frame can
only be judged against what an EMPTY Bevy frame costs on the same GPU, driver,
compositor, and monitor.

```sh
cd dev/bevy_ceiling
cargo run --release                          # 1600x900, vsync (Fifo) — what the game asks for
cargo run --release -- --no-vsync            # the real ceiling: PresentMode::Immediate
cargo run --release -- --no-vsync --sprites 500
cargo run --release -- --no-vsync --seconds 20   # exits itself and prints a summary
```

The on-screen counter is Bevy's own `FpsOverlayPlugin`. The same numbers are
printed to stdout every two seconds, from the app's own frame-time ring
buffer, so a run can be logged and compared.

What the flags are for:

- `--no-vsync` / `--present-mode fifo|mailbox|immediate|auto-no-vsync`. The game
  never sets a present mode, so it gets Bevy's default, `Fifo`: frames are
  quantised to the monitor's refresh interval and the FPS can never exceed the
  refresh rate. If the empty window under `fifo` shows the monitor's rate and
  under `immediate` shows thousands, the driver honours vsync and every
  sub-refresh game FPS is a frame that missed a refresh, not a slow frame.
- `--sprites N` puts N moving sprites of one generated texture on screen. It
  answers "what does the stock 2d pipeline cost per sprite here" with nothing
  else in the frame.
- `--no-overlay` drops the FPS overlay so its own cost (a text update every
  100 ms and a small material) can be measured.
- `--width W --height H` — the game opens at 1600x900, the default here.
- `--seconds S` — quit after S seconds of measurement and print one summary
  line, for scripted runs.

This crate is its own workspace on purpose: none of the engine's crates,
features, or dependency table can reach it, so what it measures is Bevy.
