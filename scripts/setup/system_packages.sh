#!/usr/bin/env bash
# Install the host packages a desktop build and the asset pipeline need.
#
# Usage:
#   scripts/setup/system_packages.sh
#   scripts/setup/system_packages.sh --profile   # + the Tracy GUI's apt packages
#   scripts/setup/system_packages.sh --help
#
# Idempotent: a checkout that is already provisioned never invokes sudo.
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
AMBITION_SETUP_LABEL=system-packages
# shellcheck source=../lib/setup_common.sh
. "$repo_root/scripts/lib/setup_common.sh"
APT_ENSURE_LOG_PREFIX='[system-packages]'
# shellcheck source=../lib/apt_ensure.sh
. "$repo_root/scripts/lib/apt_ensure.sh"

skip_system_packages=0
# The Tracy GUI's apt packages only; the profiling TOOLCHAIN is
# `scripts/setup/profiling_tools.sh`.
want_profiling=0
while [ "$#" -gt 0 ]; do
    case "$1" in
        --profile) want_profiling=1 ;;
        -h|--help) setup_usage "$0"; exit 0 ;;
        *) fatal "unknown option: $1" ;;
    esac
    shift
done

install_system_packages() {
    if [ "$skip_system_packages" -eq 1 ]; then
        log "skipping host package install"
        return 0
    fi
    if ! apt_ensure_supported; then
        warn "apt-get/dpkg-query not found; skipping Debian/Ubuntu package install"
        return 0
    fi

    local -a required_pkgs=(
        build-essential
        ca-certificates
        clang
        curl
        ffmpeg
        fluid-soundfont-gm
        fluid-soundfont-gs
        fluidsynth
        git
        libasound2-dev
        libfontconfig1-dev
        libsndfile1
        libudev-dev
        libvulkan1
        libwayland-dev
        libx11-dev
        libxcb-shape0-dev
        libxcb-xfixes0-dev
        libxcursor-dev
        libxi-dev
        libxinerama-dev
        libxkbcommon-dev
        libxkbcommon-x11-dev
        libxrandr-dev
        mesa-vulkan-drivers
        mold
        # `download_ambition_audio_tools.sh` unpacks the FreePats Upright Piano
        # from a .7z; without this it warns and that library silently does not
        # install. It is a required package because the instrument libraries are
        # a default part of setup.
        p7zip-full
        pkg-config
        python3-dev
        python3-venv
        rubberband-cli
        sox
        timgm6mb-soundfont
    )
    # The profiling toolchain. `scripts/profile_desktop.sh` already requires
    # perf and strace and reads vulkaninfo/glxinfo for its host-environment
    # report; nothing installed them, so a fresh clone could not profile at
    # all. cmake builds Tracy below. hotspot/heaptrack are the GUI companions
    # for perf.data and allocation traces.
    #
    # ⭐ `g++` IS HERE FOR THE C++ STANDARD LIBRARY'S *DEV* PACKAGE, NOT FOR A
    # COMPILER. Tracy's client is C++: `tracy-client-sys` is the ONLY crate in
    # the whole graph that emits `cargo:rustc-link-lib=stdc++`, and it makes
    # every `--features profile` link end in `-lstdc++`. The runtime
    # `libstdc++6` that ships with any desktop is NOT enough — the linker needs
    # the `libstdc++.so` symlink out of `libstdc++-N-dev`.
    #
    # ⚠⚠ **INSTALLING `g++` IS NOT PROOF THAT THIS IS SATISFIED, and this array
    # cannot fully guarantee it.** clang picks the NEWEST gcc version directory
    # it finds under `/usr/lib/gcc/<triple>/` and passes only that one as `-L`.
    # If some other package has pulled in a newer gcc's runtime bits without its
    # `libstdc++-N-dev`, clang points the linker at a directory that has no
    # `libstdc++.so` and the link fails with `g++` sitting installed and
    # innocent. The check that actually answers it is one line:
    #
    #     clang -print-file-name=libstdc++.so     # a bare name back = not found
    #     ls /usr/lib/gcc/*/*/libstdc++.so        # which versions really have it
    #
    # and the fix is `libstdc++-<the version clang picked>-dev`.
    #
    # ⚠ The trap, met on `calculex` 2026-08-29: every crate compiled, all 537
    # rlibs landed, and the run died on the last link with `mold: library not
    # found: stdc++`. A machine in this state builds the game, RUNS the game and
    # passes tests — it fails only under `--features profile`. The message reads
    # like a stale incremental cache or a full disk and is neither; there is
    # nothing to delete.
    local -a profiling_pkgs=(
        cmake
        g++
        heaptrack
        hotspot
        linux-tools-common
        linux-tools-generic
        mesa-utils
        strace
        vulkan-tools
    )
    # Tracy's GUI server. Its CLI tools need none of this; these are only so
    # the interactive profiler can build for a developer sitting at a desktop.
    local -a tracy_gui_pkgs=(
        libcapstone-dev
        libcurl4-openssl-dev
        libdbus-1-dev
        libegl1-mesa-dev
        libfreetype-dev
        libglfw3-dev
        libpugixml-dev
        libwayland-bin
        wayland-protocols
    )
    local -a optional_pkgs=(musescore-general-soundfont)
    if [ "$want_profiling" -eq 1 ]; then
        required_pkgs+=("${profiling_pkgs[@]}")
        # Optional, not required: a headless box has no use for the Tracy GUI,
        # and its absence must not fail setup or block the CLI tools.
        optional_pkgs+=("${tracy_gui_pkgs[@]}")
    else
        log "profiling packages not requested (--profile adds them)"
    fi

    # Required first, and fatal on failure: nothing downstream builds without
    # these. Optional packages are attempted separately and never escalate on
    # their own, so a headless box that will never carry the Tracy GUI
    # libraries does not prompt for a password on every single setup run.
    apt_ensure "${required_pkgs[@]}" \
        || fatal "required host packages could not be installed (see the apt output above)"
    apt_ensure_optional "${optional_pkgs[@]}"
}

install_system_packages
