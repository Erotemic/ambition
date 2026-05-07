# first_goblin_tune_v2 demo polish overlay

This overlay updates the first_goblin_tune_v2 proof-of-concept after in-game listening:

- lowers the rendered preview peak and reduces limiter/reverb/stereo processing to reduce popping/fuzz;
- slightly lowers source instrument velocities so the renderer has more headroom;
- raises the Rust in-game generated-music relative volume so the demo is not too soft;
- keeps wave 1 simple and makes wave 2 / brute / wave 3 noticeably stronger;
- adds a short 0.85 second delay before next-wave enemies spawn after a wave clears.

Apply by unzipping at the repository root, render with the v8 renderer, then run the installer.
