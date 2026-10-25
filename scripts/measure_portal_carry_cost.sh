#!/usr/bin/env bash
# The cost of the portal carry: frame time and memory with the carry on and
# off, per quality tier, while a body goes through a pair again and again.
#
#   scripts/measure_portal_carry_cost.sh [TIERS] [FRAMES]
#
# The scene: `portal_lab`'s D pair (a floor face into a wall face); the
# player, in a space helmet, falls into the floor face and through. The
# windows are on (the game's default). `capture_scene` runs FRAMES frames
# under xvfb and writes one shot at the end, so no frame but the last reads
# an image back. It prints, per tier and carry: the median and 95th
# percentile of the frame census windows after the first 10 s, the most
# resident memory of the process, and the textures it made with no file
# (`PROCEDURAL`, where the carry's targets are counted).
#
# On a machine with no GPU the renderer is llvmpipe: each frame time
# includes the rasterizing a GPU does, and a texture is in the process's
# memory. Compare the two columns of one run; the milliseconds are not a
# phone's. Other work on the machine moves them: run it on a quiet machine.
set -euo pipefail
tiers="${1:-low,high}"
frames="${2:-900}"
repo_root="$(cd "$(dirname "$0")/.." && pwd)"
bin="${BIN:-$repo_root/target/debug/capture_scene}"
out="$(mktemp -d)"
trap 'rm -rf "$out"' EXIT
printf '%-8s %-5s %10s %10s %10s %s\n' tier carry p50_ms p95_ms max_rss procedural
for tier in ${tiers//,/ }; do
    for carry in on off; do
        log="$out/$tier-$carry.log"
        AMBITION_QUALITY_PROFILE="$tier" /usr/bin/time -v xvfb-run -a "$bin" portal_lab 1190,606 "$out/shot.png" 1280x720 \
            --warmup "$frames" --camera-zoom debug --portal-effect cones --carry "$carry" \
            --gear space_helmet@40 --player-at 2200,876@30 --player-at 1450.0,720.0@90:0.0,0.0 >"$log" 2>&1 || true
        awk -v tier="$tier" -v carry="$carry" '
            /\[frame-census\]/ {
                split($2, span, "-"); sub("s", "", span[2]);
                if (span[1] + 0 >= 10) { for (i = 1; i <= NF; i++) { split($i, kv, "="); if (kv[1] == "p50") { sub("ms", "", kv[2]); p50[n] = kv[2] } if (kv[1] == "p95") { sub("ms", "", kv[2]); p95[n] = kv[2] } } n++ }
            }
            /Maximum resident set size/ { rss = $NF }
            /\[image-census\]/ { line = $0 }
            END {
                asort(p50); asort(p95);
                match(line, /PROCEDURAL\(no file\) [0-9]+×[0-9.]+MP/);
                printf "%-8s %-5s %10s %10s %8.0fMB %s\n", tier, carry, p50[int((n + 1) / 2)], p95[int((n + 1) / 2)], rss / 1024, substr(line, RSTART, RLENGTH)
            }' "$log"
    done
done
