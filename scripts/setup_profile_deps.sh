#!/usr/bin/env bash
# Preflight for `scripts/profile_desktop.sh` on a machine that has never
# profiled Ambition.
#
# ⭐ WHY THIS IS SEPARATE FROM `run_developer_setup.sh --profile`. That script
# INSTALLS the profiling toolchain. This one asks whether the toolchain actually
# WORKS, which is a different question and the one that has cost real time:
# `g++` was installed on `calculex` and the profiling build still died at the
# link, because clang resolves `-lstdc++` against exactly one gcc version
# directory and the installed one was not it. A package list cannot express
# that. A question to the compiler can.
#
# Every check answers one thing that, left alone, fails LATE — at the end of a
# twenty-minute build, or after a play session, when re-running is expensive.
#
# Usage:
#   scripts/setup_profile_deps.sh            # check, and fix what apt can fix
#   scripts/setup_profile_deps.sh --check    # report only, change nothing
#
# Exit status is 0 when everything a profiling run needs is present, 1 when
# something still is not, so CI or another script can gate on it.
set -uo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
apply=1
[[ "${1:-}" == "--check" ]] && apply=0

pass=0
fail=0
warned=0
declare -a todo=()

ok()   { printf '  \033[32m✓\033[0m %s\n' "$*"; pass=$((pass + 1)); }
bad()  { printf '  \033[31m✗\033[0m %s\n' "$*"; fail=$((fail + 1)); }
warn() { printf '  \033[33m!\033[0m %s\n' "$*"; warned=$((warned + 1)); }
note() { printf '      %s\n' "$*"; }
head2() { printf '\n\033[1m%s\033[0m\n' "$*"; }

apt_install() {
    if ((apply == 0)); then
        todo+=("sudo apt-get install -y $*")
        return 1
    fi
    printf '      installing: %s\n' "$*"
    sudo apt-get install -y "$@" >/dev/null 2>&1
}

# ── 1. The C++ standard library, as CLANG sees it ────────────────────────────
# ⛔ THE ONE THAT ACTUALLY BIT. Tracy's client is C++, so `tracy-client-sys`
# emits `cargo:rustc-link-lib=stdc++` and every `--features profile` link ends
# in `-lstdc++`. clang resolves that against the NEWEST complete gcc install
# under /usr/lib/gcc/<triple>/ and passes only that one as `-L`. If that
# directory has no `libstdc++.so`, the link dies with
# `mold: library not found: stdc++` while `g++` sits installed and innocent.
#
# ⚠ A machine in this state builds the game, RUNS the game and passes tests. It
# fails only under `--features profile`, at the very end of a cold build.
check_cxx_stdlib() {
    head2 "C++ standard library (Tracy links -lstdc++)"
    if ! command -v clang >/dev/null 2>&1; then
        bad "clang is not installed (.cargo/config.toml pins it as the linker driver)"
        apt_install clang || note "run: sudo apt-get install -y clang"
        return
    fi
    local resolved
    resolved="$(clang -print-file-name=libstdc++.so 2>/dev/null)"
    if [[ -e "$resolved" ]]; then
        ok "clang resolves libstdc++.so -> $resolved"
        return
    fi
    # clang echoes the bare name back when it cannot find it.
    local selected version
    selected="$(clang -v -x c++ /dev/null -o /dev/null 2>&1 |
        sed -n 's/^Selected GCC installation: //p' | head -1)"
    if [[ -z "$selected" ]]; then
        bad "clang cannot resolve libstdc++.so and would not say which gcc it selected"
        note "run by hand: clang -v -x c++ /dev/null -o /dev/null 2>&1 | grep 'GCC installation'"
        return
    fi
    version="$(basename "$selected")"
    bad "clang selected gcc $version, which has no libstdc++.so — the profiling link will fail"
    note "the versions that DO have it:"
    local d found=0
    for d in /usr/lib/gcc/*/*/; do
        [[ -e "$d/libstdc++.so" ]] && { note "  $d"; found=1; }
    done
    ((found)) || note "  (none — no libstdc++-*-dev is installed at all)"
    note "installing the dev package for the version clang actually picked:"
    if apt_install "libstdc++-${version}-dev"; then
        resolved="$(clang -print-file-name=libstdc++.so 2>/dev/null)"
        if [[ -e "$resolved" ]]; then
            ok "fixed: clang now resolves libstdc++.so -> $resolved"
            fail=$((fail - 1))
        else
            note "still unresolved after install; check what owns $selected"
        fi
    else
        note "run: sudo apt-get install -y libstdc++-${version}-dev"
    fi
}

# ── 2. perf, and whether the kernel will let it record ───────────────────────
# `perf record` needs perf_event_paranoid <= 1, and kernel symbols need
# kptr_restrict = 0 or every kernel frame in the report reads 0xffffffff…
check_perf() {
    head2 "perf"
    if ! command -v perf >/dev/null 2>&1; then
        bad "perf is not installed"
        apt_install linux-tools-common "linux-tools-$(uname -r)" linux-tools-generic ||
            note "run: sudo apt-get install -y linux-tools-common linux-tools-\$(uname -r)"
    elif ! perf stat true >/dev/null 2>&1; then
        bad "perf is installed but cannot run (often a kernel/tools version mismatch)"
        note "perf reports: $(perf --version 2>&1 | head -1)"
        note "kernel is: $(uname -r)"
    else
        ok "perf works ($(perf --version 2>&1 | head -1))"
    fi

    local paranoid
    paranoid="$(cat /proc/sys/kernel/perf_event_paranoid 2>/dev/null || echo unknown)"
    if [[ "$paranoid" =~ ^-?[0-9]+$ ]] && ((paranoid <= 1)); then
        ok "perf_event_paranoid=$paranoid (recording allowed)"
    else
        warn "perf_event_paranoid=$paranoid — 'perf record' will be refused"
        note "for this boot:  sudo sysctl -w kernel.perf_event_paranoid=1"
        note "profile_desktop.sh sets this itself when it can; this is only a heads-up"
    fi

    local kptr
    kptr="$(cat /proc/sys/kernel/kptr_restrict 2>/dev/null || echo unknown)"
    if [[ "$kptr" == "0" ]]; then
        ok "kptr_restrict=0 (kernel symbols will resolve)"
    else
        warn "kptr_restrict=$kptr — kernel frames will report as 0xffffffff…"
        note "for this boot:  sudo sysctl -w kernel.kptr_restrict=0"
    fi
}

# ── 3. Tracy, both halves and the version between them ───────────────────────
# ⚠ THE CLIENT AND THE SERVER MUST BE THE SAME TRACY. The game links whatever
# `tracy-client-sys` vendors; `tracy-capture` is built separately. A mismatch
# does not error — the capture simply never connects and the bundle records
# "the game never connected", which reads like a game bug.
check_tracy() {
    head2 "Tracy (per-system attribution)"
    local have_capture=1
    for tool in tracy-capture tracy-csvexport; do
        if command -v "$tool" >/dev/null 2>&1; then
            ok "$tool on PATH ($(command -v "$tool"))"
        else
            have_capture=0
            if [[ -x "$HOME/.local/bin/$tool" ]]; then
                bad "$tool is installed at ~/.local/bin but is NOT on PATH"
                note "add to your shell rc:  export PATH=\"\$HOME/.local/bin:\$PATH\""
            else
                bad "$tool is not installed"
                note "run: ./run_developer_setup.sh --profile   (builds Tracy from source via cmake)"
            fi
        fi
    done
    ((have_capture)) || return

    # The version the game will actually speak, read from the vendored header.
    local header client_version server_version
    header="$(find "${CARGO_HOME:-$HOME/.cargo}/registry/src" -maxdepth 6 \
        -path '*tracy-client-sys-*/tracy/common/TracyVersion.hpp' -print -quit 2>/dev/null)"
    if [[ -z "$header" ]]; then
        warn "could not find the vendored TracyVersion.hpp; skipping the version match"
        note "it appears once the profiling build has fetched tracy-client-sys"
        return
    fi
    client_version="$(awk -F'[ =}]+' '/Major/ {maj=$(NF-1)} /Minor/ {min=$(NF-1)} /Patch/ {pat=$(NF-1)} END {print maj"."min"."pat}' "$header")"
    # ⚠ `tracy-capture` prints no version, at all, under any flag. The only
    # local record of what was BUILT is the source tree
    # `run_developer_setup.sh` cloned into the cache, so that is what we
    # compare against — inferred, and said to be inferred.
    local cache_dir built
    cache_dir="${XDG_CACHE_HOME:-$HOME/.cache}/ambition"
    built="$(ls -d "$cache_dir"/tracy-* 2>/dev/null | sed 's#.*/tracy-##' | sort -V | tail -1)"
    if [[ -z "$built" ]]; then
        warn "no Tracy source tree in $cache_dir; cannot tell which version was built"
        note "the client the game links is $client_version"
    elif [[ "$client_version" == "$built" ]]; then
        ok "Tracy versions agree on $client_version (client vendored, server built from cache)"
    else
        bad "Tracy MISMATCH: the game links $client_version, the built server is $built"
        note "⚠ a mismatch does NOT error. tracy-capture simply never connects, and the"
        note "  bundle records 'the game never connected' — which reads like a game bug."
        note "run: ./run_developer_setup.sh --profile   (it builds the version the crate vendors)"
    fi
}

# ── 4. Things the bundle's own reports need ──────────────────────────────────
check_reporting_tools() {
    head2 "Bundle reporting"
    local missing=()
    for tool in strace python3; do
        if command -v "$tool" >/dev/null 2>&1; then
            ok "$tool"
        else
            bad "$tool is missing"
            missing+=("$tool")
        fi
    done
    ((${#missing[@]})) && { apt_install "${missing[@]}" || note "run: sudo apt-get install -y ${missing[*]}"; }

    # `ambition.local.toml` is parsed with tomllib, which is python 3.11+.
    if python3 -c 'import tomllib' >/dev/null 2>&1; then
        ok "python3 has tomllib (ambition.local.toml will be read)"
    else
        warn "python3 is older than 3.11: ambition.local.toml will be IGNORED"
        note "the game still runs; the per-machine quality config just does nothing"
    fi

    if command -v vulkaninfo >/dev/null 2>&1; then
        ok "vulkaninfo (the bundle records which adapter actually rendered)"
    else
        warn "vulkaninfo missing — summary.md cannot name the GPU it measured"
        apt_install vulkan-tools || note "run: sudo apt-get install -y vulkan-tools"
    fi
}

# ── 5. Where the bundles land ────────────────────────────────────────────────
# ⚠ perf.data dominates a bundle and a long run is gigabytes. Finding out the
# disk is full AFTER a play session costs the session.
check_space() {
    head2 "Disk for the bundles"
    local dir="$repo_root/dev/ambition_dev_measurements"
    local avail_kb avail_gb
    avail_kb="$(df -Pk "$dir" 2>/dev/null | awk 'NR==2 {print $4}')"
    if [[ -z "$avail_kb" ]]; then
        warn "could not read free space for $dir"
        return
    fi
    avail_gb=$((avail_kb / 1024 / 1024))
    if ((avail_gb >= 20)); then
        ok "${avail_gb}G free where bundles are written"
    elif ((avail_gb >= 5)); then
        warn "${avail_gb}G free where bundles are written — a long capture may not fit"
        note "bundles accumulate in dev/ambition_dev_measurements/profiles/; prune old ones"
    else
        bad "${avail_gb}G free where bundles are written"
        note "mold fails a link with 'failed to write to an output file. Disk full?'"
        note "⛔ do NOT reclaim by deleting under target/ — see AGENTS.md"
    fi
}

printf '\033[1mAmbition profiling preflight\033[0m — %s\n' "$(hostname)"
((apply)) || printf '(--check: reporting only, nothing will be installed)\n'

check_cxx_stdlib
check_perf
check_tracy
check_reporting_tools
check_space

head2 "Result"
printf '  %d ok, %d problem(s), %d warning(s)\n' "$pass" "$fail" "$warned"
if ((${#todo[@]})); then
    printf '\n  would run:\n'
    printf '    %s\n' "${todo[@]}"
fi
if ((fail == 0)); then
    printf '\n  Ready: ./scripts/profile_desktop.sh\n'
    printf '  Without Tracy (faster, and the arm to size an optimization against):\n'
    printf '    ./scripts/profile_desktop.sh --no-tracy\n'
    exit 0
fi
printf '\n  Fix the ✗ items above, then re-run this script.\n'
exit 1
