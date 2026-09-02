#!/usr/bin/env bash
# Prove `capture_scene --press-during` photographs a DIFFERENT moment than the
# ordinary shutter, by pixel hash.
#
# WHY A SCRIPT AND NOT A `#[test]`. The claim is about pixels, so answering it
# needs the real render stack: two full app boots, a swapchain, a readback. On a
# machine with no GPU that is llvmpipe at a couple of frames a second, which is
# minutes per capture — a cost the workspace suite cannot carry on every run for
# a developer tool. The unit tests in `capture_scene.rs` pin the SCHEDULE (which
# press-driving frame the shutter opens on, and that the flag's absence changes
# nothing); this pins the CONSEQUENCE, on demand, and prints what it measured.
#
# THE PAIR IS CHOSEN SO "DIFFERENT" IS NOT A COIN FLIP. `Enter` on the launcher
# starts a route change, so the two shutters are on opposite sides of it:
#   --press-during 1  the launcher, with Enter still held (the release edge has
#                     not been delivered — a tap is two frames by design)
#   (no flag)         whatever the confirmation reached, after the new route
#                     loaded and became ready
# Two images of the same screen would make an equal hash the interesting result;
# these are two different screens, so an equal hash means the flag did nothing.
#
# USAGE: scripts/verify_press_during_capture.sh [OUT_DIR]
set -euo pipefail

out_dir="${1:-/tmp/press_during_check}"
mkdir -p "$out_dir"
cd "$(git rev-parse --show-toplevel)"

# ⛔ `ultra` IS NOT A PREFERENCE HERE. The Potato tier a software rasteriser gets
# seeded to scales screen shaders and the parallax to nothing, so two captures
# could match by both being empty. capture_scene's own --help says the same.
export AMBITION_QUALITY_PROFILE=ultra

size="${PRESS_DURING_SIZE:-320x180}"
route="${PRESS_DURING_ROUTE:-ambition_launcher}"
press="${PRESS_DURING_PRESS:-Enter}"

echo "building capture_scene ..."
cargo build -p ambition_app_tools --bin capture_scene
bin=target/debug/capture_scene

shoot() {
    local label="$1" out="$2"
    shift 2
    echo "=== $label ==="
    if ! timeout 900 "$bin" --route "$route" "$out" "$size" --press "$press" "$@" \
        >"$out_dir/$label.log" 2>&1; then
        echo "FAIL: capture '$label' exited $? — see $out_dir/$label.log"
        tail -20 "$out_dir/$label.log"
        exit 1
    fi
    grep -E 'capture_scene: (pressed|texture readback|NO SUBJECT)' "$out_dir/$label.log" || true
}

shoot during "$out_dir/during.png" --press-during 1
shoot after "$out_dir/after.png"

during_hash=$(sha256sum "$out_dir/during.png" | cut -d' ' -f1)
after_hash=$(sha256sum "$out_dir/after.png" | cut -d' ' -f1)

echo
echo "during (--press-during 1): $during_hash  $(stat -c%s "$out_dir/during.png") bytes"
echo "after  (ordinary shutter): $after_hash  $(stat -c%s "$out_dir/after.png") bytes"

if [[ "$during_hash" == "$after_hash" ]]; then
    echo
    echo "FAIL: the two captures are byte-identical. --press-during opened the shutter"
    echo "      at the same moment the ordinary one does, so the flag bought nothing."
    exit 1
fi

# A DIFFERENT HASH IS NOT YET A DIFFERENT PICTURE. A blank PNG and a slightly
# less blank one also differ, and this tool's standing failure mode is writing an
# untouched capture texture and calling it success. So require that BOTH frames
# carry something: an all-one-colour PNG compresses to a few hundred bytes at
# this size, and neither of these should be near that.
floor="${PRESS_DURING_MIN_BYTES:-2000}"
for f in during after; do
    bytes=$(stat -c%s "$out_dir/$f.png")
    if ((bytes < floor)); then
        echo
        echo "FAIL: $f.png is $bytes bytes (< $floor) — that is a blank or near-blank"
        echo "      frame, and two blanks differing proves nothing about the shutter."
        exit 1
    fi
done

echo
echo "PASS: the shutters photographed different, non-blank moments."
