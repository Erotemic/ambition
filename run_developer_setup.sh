#!/usr/bin/env bash
# Idempotently prepare a fresh checkout for desktop development.
#
# The DEFAULT is the fast path to a runnable game: host libraries, the Rust
# toolchain, submodules, tool-local Python environments, generated assets, and
# a desktop target check. Nothing that only an analysis or authoring specialist
# needs is installed unless asked for, because a first clone's job is to run.
#
# Everything heavier is one extra argument:
#
#   --profile           the profiling/analysis toolchain: perf, strace,
#                       heaptrack, hotspot, vulkan/mesa info tools, Tracy built
#                       from source, cargo-flamegraph, and the cargo analysis
#                       tools (llvm-cov, modules, sweep, mark-sweep, nextest).
#                       Costs ~190 apt packages (hotspot alone pulls the KDE
#                       Frameworks stack) plus several source builds.
#   --full              the above, plus nothing else — the instrument libraries
#                       are already part of a default run.
#
# ⛔ THE SAMPLED INSTRUMENT LIBRARIES ARE NOT OPTIONAL. They were, behind
# `--audio-libraries`, on the theory that without them "every cue still
# renders — a quality difference, not a failure". It is a failure: a default
# clone rendered the ENTIRE catalogue through General MIDI and reported success,
# and nothing downstream can tell that audio from the real thing. Two cues
# (`aether_severance`, `blazingly_fast`) set `render.strict_backends` and could
# not render at all, which is what finally made it visible. Every library is a
# public download; `--skip-audio-libraries` opts out for a machine that only
# needs to compile.
#
# Usage:
#   ./run_developer_setup.sh [--profile] [--full]
#       [--skip-system-packages] [--skip-rust] [--skip-submodules]
#       [--skip-tally] [--skip-python] [--skip-assets] [--skip-cargo-check]
#       [--skip-audio-libraries]
#
# Environment:
#   AMBITION_TOOL_PYTHON=3.12
#   UV_EXCLUDE_NEWER=YYYY-MM-DD
#   AMBITION_AUDIO_TOOLS_ROOT=/data/audio-tools
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$repo_root"

# Host packages go through the shared helper so that a checkout which is
# already provisioned never invokes sudo at all. See scripts/lib/apt_ensure.sh.
APT_ENSURE_LOG_PREFIX='[developer-setup]'
# shellcheck source=scripts/lib/apt_ensure.sh
. "$repo_root/scripts/lib/apt_ensure.sh"

skip_system_packages=0
skip_rust=0
skip_submodules=0
skip_python=0
skip_assets=0
skip_cargo_check=0
skip_tally=0

# Opt-in, not opt-out. These were once on by default and the default run spent
# almost all of its time on them before it ever reached the game.
want_profiling=0
# ⛔ DEFAULT ON. See the block at the top of this file: opt-in meant every fresh
# machine shipped General-MIDI stand-ins for the whole soundtrack.
want_audio_libraries=1

# Tracy speaks a versioned wire protocol and REFUSES to connect to a client
# built against a different version, so the server must match the
# `tracy-client-sys` crate the game links. That version, and the build that
# honours it, live in `scripts/setup/install_profiling_tools.sh` — one
# implementation, because there were two and one of them read the WORD "Major"
# out of the header instead of the number.

usage() {
    awk '
        NR == 1 { next }
        /^set -euo pipefail$/ { exit }
        /^#$/ { print ""; next }
        /^# / { sub(/^# /, ""); print }
    ' "$0"
}

log() {
    printf '[developer-setup] %s\n' "$*"
}

warn() {
    printf '[developer-setup] warning: %s\n' "$*" >&2
}

fatal() {
    printf '[developer-setup] error: %s\n' "$*" >&2
    exit 1
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --skip-system-packages) skip_system_packages=1 ;;
        --skip-rust) skip_rust=1 ;;
        --skip-submodules) skip_submodules=1 ;;
        --skip-python) skip_python=1 ;;
        --skip-assets) skip_assets=1 ;;
        --skip-cargo-check) skip_cargo_check=1 ;;
        --skip-tally) skip_tally=1 ;;
        --profile) want_profiling=1 ;;
        --skip-audio-libraries) want_audio_libraries=0 ;;
        # Accepted so an existing invocation does not break; it is the default.
        --audio-libraries) want_audio_libraries=1 ;;
        --full) want_profiling=1; want_audio_libraries=1 ;;
        -h|--help) usage; exit 0 ;;
        *) fatal "unknown option: $1" ;;
    esac
    shift
done

have() {
    command -v "$1" >/dev/null 2>&1
}

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

ensure_rust() {
    if [ "$skip_rust" -eq 1 ]; then
        log "skipping Rust setup"
        return 0
    fi

    if ! have rustup; then
        have curl || fatal "curl is required to install rustup"
        log "installing rustup and the stable Rust toolchain"
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
            | sh -s -- -y --profile default --default-toolchain stable
    fi

    if [ -f "$HOME/.cargo/env" ]; then
        # shellcheck disable=SC1091
        . "$HOME/.cargo/env"
    fi

    have rustup || fatal "rustup is not on PATH after installation"
    rustup toolchain install stable
    rustup default stable
    rustup component add rustfmt clippy llvm-tools-preview
    have cargo || fatal "cargo is not on PATH after Rust setup"

    # ⛔ NONE of these is on the path from a clone to a running game, and every
    # one is a source build. They were unconditional, and together they were the
    # single largest block of a default run — cargo-modules alone compiles the
    # rust-analyzer crate graph. Each is reachable without them:
    #   cargo-llvm-cov    only `run_game.sh --cov`
    #   cargo-mark-sweep  only `scripts/sweep_cargo_target.sh`, which already
    #                     prints its own install line when it is missing
    #   cargo-modules     no caller in the repo
    #   cargo-sweep       no caller in the repo
    #   cargo-nextest     `scripts/run_tests.py` says so itself, at its
    #                     definition: "OPTIONAL, NOT REQUIRED. A contributor
    #                     without nextest still gets the same" results, because
    #                     the runner falls back to plain `cargo test`.
    if [ "$want_profiling" -eq 1 ]; then
        ensure_cargo_tool cargo-llvm-cov cargo-llvm-cov
        ensure_cargo_tool cargo-modules cargo-modules
        ensure_cargo_tool cargo-sweep cargo-sweep
        ensure_cargo_tool cargo-mark-sweep cargo-mark-sweep
        # Per-test wall times, which stable libtest cannot report:
        # `--report-time` is nightly-only, so ranking the slowest tests in a
        # 1600s suite otherwise means timing each test binary by hand. nextest
        # also runs each test in its own process, so a test that only passes on
        # a sibling's leftover state shows up as a failure instead of hiding.
        ensure_cargo_tool cargo-nextest cargo-nextest
    else
        log "cargo analysis tools not requested (--profile adds them)"
    fi

    log "Rust ready: $(rustc --version)"
}

ensure_cargo_tool() {
    local package="$1"
    local binary="$2"
    if have "$binary"; then
        log "$binary already installed"
    else
        log "installing $package"
        cargo install --locked "$package"
    fi
}

ensure_profiling_tools() {
    if [ "$want_profiling" -eq 0 ]; then
        return 0
    fi

    # A flame graph as a file, for the workflow that does not want a GUI.
    if have cargo; then
        ensure_cargo_tool flamegraph flamegraph
    else
        warn "cargo unavailable; skipping cargo-flamegraph"
    fi

    # ⭐ ONE IMPLEMENTATION, AND IT IS THE STANDALONE ONE. This used to carry its
    # own copy of the Tracy build and its own version parse — and that parse read
    # the WORD "Major" instead of the number, so it asked git for a branch called
    # `vMajor.Minor.Patch`, failed to clone, and left whatever Tracy was already
    # there. That is how a 0.13.1 server survived beside a 0.14.0 client and cost
    # a capture on real hardware its per-system zones.
    #
    # It is also the whole reason the standalone script exists: somebody whose
    # only problem is a mismatched Tracy should not have to run submodule sync,
    # tool venvs and asset regeneration to fix it.
    if ! "$repo_root/scripts/setup/install_profiling_tools.sh"; then
        warn "Tracy tools not installed; perf-based profiling is unaffected"
    fi
}

ensure_resource_tally() {
    if [ "$skip_tally" -eq 1 ]; then
        log "skipping LLM resource-accounting hook install"
        return 0
    fi
    [ -e "$repo_root/.llm_resource_tally/tool" ] || return 0
    have python3 || { warn "python3 not found; skipping resource-accounting hook install"; return 0; }

    # Offline and idempotent (it only points core.hooksPath at the committed
    # hook dir and refreshes the managed AGENTS.md block). Accounting is
    # bookkeeping, not a runtime dependency, so a failure here must never
    # block a checkout from becoming runnable.
    log "arming the LLM resource-accounting git hook"
    if ! python3 "$repo_root/.llm_resource_tally/tool" install; then
        warn "resource-accounting hook install failed; continuing (run it by hand later)"
    fi
}

ensure_submodules() {
    if [ "$skip_submodules" -eq 1 ]; then
        log "skipping git submodule setup"
        return 0
    fi
    have git || fatal "git is required for submodule setup"
    [ -f "$repo_root/.gitmodules" ] || return 0

    log "syncing and initializing git submodules recursively"
    git submodule sync --recursive
    git submodule update --init --recursive

    # Verify against real gitlinks (index mode 160000), not `.gitmodules` entries.
    # A `.gitmodules` block whose gitlink was dropped is a stale declaration that
    # `git submodule update` correctly ignores; treating it as fatal bricks setup
    # for every fresh clone. Only a gitlink git *should* have materialized is an error.
    local path
    while read -r path; do
        [ -n "$path" ] || continue
        [ -d "$repo_root/$path" ] || fatal "submodule path was not initialized: $path"
        if [ -z "$(find "$repo_root/$path" -mindepth 1 -maxdepth 1 -print -quit)" ]; then
            fatal "submodule path is empty after update: $path"
        fi
    done < <(git -C "$repo_root" ls-files --stage | awk '$1 == "160000" { print substr($0, index($0, $4)) }')

    # A declared submodule with no gitlink can never be initialized. Warn so the
    # stale entry gets cleaned up, but do not block the rest of setup.
    local declared
    while read -r _ declared; do
        [ -n "$declared" ] || continue
        if ! git -C "$repo_root" ls-files --stage -- "$declared" | grep -q '^160000'; then
            warn "stale .gitmodules entry with no gitlink (ignored): $declared"
        fi
    done < <(git config -f "$repo_root/.gitmodules" --get-regexp '^submodule\..*\.path$' || true)
}

ensure_uv() {
    if have uv; then
        log "uv already installed: $(uv --version)"
    else
        have curl || fatal "curl is required to install uv"
        log "installing uv"
        curl -LsSf https://astral.sh/uv/install.sh | sh
        if [ -f "$HOME/.local/bin/env" ]; then
            # shellcheck disable=SC1091
            . "$HOME/.local/bin/env"
        fi
        export PATH="$HOME/.local/bin:$PATH"
        have uv || fatal "uv install did not put uv on PATH"
    fi

    if [ -z "${UV_EXCLUDE_NEWER:-}" ]; then
        local cutoff
        if cutoff="$(date -u -d '14 days ago' +%Y-%m-%d 2>/dev/null)"; then
            export UV_EXCLUDE_NEWER="$cutoff"
            log "UV_EXCLUDE_NEWER=$UV_EXCLUDE_NEWER"
        else
            warn "date -d is unavailable; uv will use repository configuration"
        fi
    fi
    export UV_LINK_MODE="${UV_LINK_MODE:-copy}"
}

tool_python_version() {
    printf '%s\n' "${AMBITION_TOOL_PYTHON:-3.12}"
}

venv_major_minor() {
    "$1" - <<'PY'
import sys
print(f"{sys.version_info.major}.{sys.version_info.minor}")
PY
}

# ⛔⛔ A VENV IS MACHINE STATE AND THIS CHECKOUT CAN BE SHARED. `pyvenv.cfg`
# records an absolute interpreter path in one user's home, so a `.venv` inside a
# tree two users mount over virtiofs is real for exactly one of them. The other
# falls through to a bare `python3` without the tool's dependencies — and on
# 2026-08-29 the sprite pipeline then published a character wearing another
# character's art and reported success.
#
# ⭐ The same shape as `scripts/setup/target_bindmount.sh`, which exists because
# `target/` is machine state in a shared tree. `tool_python.sh` resolves this
# store BEFORE any in-repo `.venv`, so both users get their own and neither sees
# the other's.
#
# ⚠ In-repo `.venv` directories are still honoured for checkouts that predate
# this, and `AMBITION_TOOL_VENVS` moves the store.
# The ONE place that knows how a tool's interpreter is resolved. Sourced rather
# than reimplemented so setup, the regen scripts and the game all agree.
# shellcheck disable=SC1091
source "$repo_root/scripts/lib/tool_python.sh"

ambition_tool_venv_dir() {
    local project="$1"
    printf '%s/%s\n' \
        "${AMBITION_TOOL_VENVS:-${XDG_CACHE_HOME:-$HOME/.cache}/ambition-tool-venvs}" \
        "$(basename -- "$project")"
}

ensure_tool_venv() {
    local project="$1"
    local requested_python="$2"
    local venv_dir
    venv_dir="$(ambition_tool_venv_dir "$project")"
    mkdir -p "$(dirname -- "$venv_dir")"
    local label="${venv_dir/#$HOME/~}"

    if [ -x "$venv_dir/bin/python" ]; then
        local current_python
        current_python="$(venv_major_minor "$venv_dir/bin/python")"
        if [ "$current_python" != "$requested_python" ]; then
            log "recreating $label ($current_python -> $requested_python)"
            uv venv --clear --python "$requested_python" "$venv_dir"
        else
            log "reusing $label (Python $current_python)"
        fi
    else
        if [ -e "$venv_dir" ]; then
            warn "replacing incomplete environment: $label"
        fi
        log "creating $label with Python $requested_python"
        uv venv --clear --python "$requested_python" "$venv_dir"
    fi
}

install_tool_project() {
    local relative_project="$1"
    local import_name="$2"
    local editable_target="${3:-.}"
    local project="$repo_root/$relative_project"
    local requested_python
    requested_python="$(tool_python_version)"

    [ -d "$project" ] || fatal "missing tool project: $relative_project (submodule not initialized?)"
    [ -f "$project/pyproject.toml" ] || fatal "missing $relative_project/pyproject.toml"

    ensure_tool_venv "$project" "$requested_python"
    local venv_python
    venv_python="$(ambition_tool_venv_dir "$project")/bin/python"
    log "installing $relative_project into ${venv_python/#$HOME/~}"
    (
        cd "$project"
        uv pip install --python "$venv_python" -e "$editable_target"
        if [ -d "$project/tests" ]; then
            uv pip install --python "$venv_python" pytest
        fi
    )
    "$venv_python" -c "import $import_name" \
        || fatal "$relative_project installed but '$import_name' is not importable"
}

tool_projects() {
    cat <<'EOF'
tools/ambition_sprite2d_renderer ambition_sprite2d_renderer
tools/ambition_music_renderer ambition_music_renderer
tools/ambition_sfx_renderer ambition_sfx_renderer
tools/ambition_background_renderer ambition_background_renderer
tools/ambition_parallax_renderer ambition_parallax_renderer
tools/ambition_ldtk_tools ambition_ldtk_tools
EOF
}

verify_tool_environments() {
    local relative_project import_name project python_bin
    while read -r relative_project import_name; do
        project="$repo_root/$relative_project"
        # ⚠ Ask `tool_python.sh`, do not reconstruct the path: it is the ONE
        # place that knows the resolution order (per-machine store, then an
        # in-repo `.venv` for checkouts that predate the store). A verifier with
        # its own idea of where the interpreter lives will fail a machine that
        # is actually fine.
        python_bin="$(ambition_select_tool_python "$project" "" 0)"
        ambition_python_exists "$python_bin" \
            || fatal "no interpreter for $relative_project; rerun without --skip-python"
        "$python_bin" -c "import $import_name" >/dev/null 2>&1 \
            || fatal "$import_name is not importable from $python_bin; rerun without --skip-python"
    done < <(tool_projects)
}

ensure_python_tools() {
    if [ "$skip_python" -eq 1 ]; then
        log "skipping Python tool installation"
        return 0
    fi

    ensure_uv

    # Keep every authoring project isolated. The SFX renderer intentionally
    # caps Python below 3.13, and the audio/sprite stacks carry native wheels
    # that should not constrain unrelated tools.
    install_tool_project tools/ambition_sprite2d_renderer ambition_sprite2d_renderer
    install_tool_project tools/ambition_music_renderer ambition_music_renderer '.[all]'
    install_tool_project tools/ambition_sfx_renderer ambition_sfx_renderer
    install_tool_project tools/ambition_background_renderer ambition_background_renderer
    install_tool_project tools/ambition_parallax_renderer ambition_parallax_renderer
    install_tool_project tools/ambition_ldtk_tools ambition_ldtk_tools

    install_scripts_env

    log "Python authoring environments are ready"
}

# The repo-root `.venv` used by `scripts/*.py` (as opposed to the per-tool
# authoring environments above).
#
# `scripts/ecs_inventory.py` — which regenerates the `.agent/ecs_inventory`
# packets an agent navigates by — parses Rust with tree-sitter. Nothing
# installed it, so on a fresh clone that regeneration simply failed with
# ModuleNotFoundError and the committed navigation data silently went stale.
# That is the regen-on-a-fresh-clone invariant, so it belongs in setup.
install_scripts_env() {
    local requested_python venv_dir venv_python
    requested_python="$(tool_python_version)"
    # Same creation policy as every tool-local environment: this used to be its
    # own copy of the logic, and being a copy is how it missed both `--clear`
    # and the interpreter-version check the tool venvs have had all along.
    ensure_tool_venv "$repo_root" "$requested_python"
    # ⛔ ASK THE HELPER FOR THE PATH; DO NOT SPELL IT AGAIN. `ensure_tool_venv`
    # creates this environment in the per-machine store, and a second literal
    # `$repo_root/.venv` here named a directory nothing had created — so every
    # fresh clone died on the next line with "No virtual environment ... for
    # path `.venv/bin/python`", before assets or the desktop check ever ran.
    venv_dir="$(ambition_tool_venv_dir "$repo_root")"
    venv_python="$venv_dir/bin/python"
    log "installing scripts/ dependencies into ${venv_python/#$HOME/~}"
    # `pytest` belongs here for the same reason `tree_sitter_rust` does, and its
    # absence was worse. `scripts/run_tests.py` runs the repo's TWO Python suites
    # as `sys.executable -m pytest` — the goal guard, the test runner, the
    # package-asset guard, the architectural absence contracts, and the whole
    # 149-test LDtk authoring toolchain — and nothing installed it. So on any
    # machine set up by THIS script both jobs failed instantly with "No module
    # named pytest", which `run_tests.py` reports as two red jobs at 0.0s among
    # twenty green ones: a suite whose first line is "if the thing that decides
    # whether the suite is honest is broken, that is the answer" could never run
    # the thing that decides it.
    # ⚠ EVERY ONE OF THESE IS A COLLECTION ERROR WHEN ABSENT, NOT A SKIPPED TEST.
    # `scripts/tests` imports the scripts it tests at module scope, so one
    # missing third-party module takes the whole detached-tools job down at
    # import time: `numpy` and `soundfile` are `scripts/audio_levels.py`, and
    # `rich` is the repository's clickable-`file://` output convention, which
    # `scripts/agent_query.py` — step 2 of the AGENTS.md cold start — needs to
    # print anything at all.
    uv pip install --python "$venv_python" \
        pytest tree_sitter tree_sitter_rust numpy soundfile rich
    # The moveset inspector is imported directly out of `tools/` by
    # `scripts/tests/test_moveset_inspector_renderer.py`, so it belongs in THIS
    # environment rather than one of its own. Installed editable so its
    # dependencies stay its own to declare (today: pyyaml).
    uv pip install --python "$venv_python" -e tools/ambition_moveset_inspector
    local module
    for module in tree_sitter_rust pytest numpy soundfile rich yaml; do
        "$venv_python" -c "import $module" \
            || fatal "$venv_dir installed but '$module' is not importable — the repo's own Python suites cannot run"
    done
}

# The sampled instrument libraries, and the sfizz/LV2 hosts that can actually
# play them.
#
# ⛔ THE RENDERER IS NOT BROKEN WITHOUT THESE. `render/group.py` warns per
# instrument and falls back to General MIDI, so a default checkout still renders
# every cue — the GM soundfonts are in the required package list precisely so it
# can. What the libraries buy is quality, at the cost of gigabytes, which is why
# they are opt-in rather than part of the fast path.
#
# The downloader writes `$root/env.sh`, which is what `scripts/regen/music.sh` and
# `render_music.sh` source to expose the SFZ/LV2/VST3/CLAP search paths.
ensure_audio_libraries() {
    local root renderer
    root="${AMBITION_AUDIO_TOOLS_ROOT:-/data/audio-tools}"
    renderer="$repo_root/tools/ambition_music_renderer"

    if [ "$want_audio_libraries" -eq 0 ]; then
        if [ -f "$root/env.sh" ]; then
            log "sampled instrument libraries already present at $root"
        else
            log "sampled instruments not requested (--audio-libraries adds them);"
            log "   music renders through the General-MIDI fallback until then"
        fi
        return 0
    fi

    if [ ! -x "$renderer/setup.sh" ]; then
        warn "$renderer/setup.sh is missing; skipping the audio toolchain"
        return 0
    fi

    # sfizz first: without a player, a downloaded SFZ library is inert.
    #
    # INSTALL_SFIZZ_OBS defaults to 0 in the renderer's setup because it adds a
    # third-party apt source, and Ubuntu does not package sfizz at all — so with
    # the default the SFZ libraries below install and then nothing can play
    # them. Asking for --audio-libraries IS asking for the SFZ path, so opt in
    # here rather than leaving the flag half-effective.
    log "installing the native audio toolchain (sfizz, LV2/VST3 hosts)"
    INSTALL_SFIZZ_OBS=1 "$renderer/setup.sh" \
        || warn "audio toolchain setup reported a failure; continuing"

    # /data is root-owned on a fresh box and the downloader does not escalate,
    # so its first write would fail on a path the developer never chose.
    #
    # Test writability, NOT `mkdir -p`: mkdir -p SUCCEEDS on a directory that
    # already exists no matter who owns it, so gating the escalation on its exit
    # status skipped the chown on exactly the boxes that needed it.
    if [ ! -w "$root" ] && have sudo; then
        log "making $root writable for $(whoami)"
        sudo mkdir -p "$root" && sudo chown "$(id -u):$(id -g)" "$root" || true
    fi
    if [ ! -w "$root" ]; then
        warn "$root is not writable; skipping the library download"
        warn "set AMBITION_AUDIO_TOOLS_ROOT to a path you own and rerun"
        return 0
    fi

    log "downloading sampled instrument libraries into $root (this is large)"
    "$renderer/download_ambition_audio_tools.sh" "$root" \
        || warn "instrument library download reported a failure; cues will use the fallback"
}

regenerate_assets() {
    if [ "$skip_assets" -eq 1 ]; then
        log "skipping generated assets"
        return 0
    fi
    if [ "$skip_python" -eq 1 ]; then
        log "checking existing tool-local environments"
        verify_tool_environments
    fi

    log "regenerating all runtime assets"
    "$repo_root/scripts/regen/assets.sh"
    regenerate_missing_published_sheets
    verify_generated_content_is_current
}

# ⛔ "THE SETUP RAN" IS NOT "THE CONTENT IS CURRENT", and the gap is invisible
# from inside the game. Measured on `calculex` 2026-08-29: 998 sprite sheets
# ~11 days stale and the parallax quality tiers absent entirely, so the game drew
# OLD art at Low/Medium and NO parallax art at all. The tiers meant for weak
# hardware were the ones most likely to be broken, on the machines least able to
# notice — nothing in the game says "this sheet is eleven days behind its source".
#
# ⭐ THE CHECKER ALREADY EXISTED AND NOTHING CALLED IT. That is the whole defect;
# the regeneration itself was already composed correctly by `regen/assets.sh`.
#
# Reports rather than fails: a stale variant is a degraded picture, not a broken
# checkout, and a setup script that exits non-zero over it would block work that
# does not care.
verify_generated_content_is_current() {
    have python3 || return 0
    [ -f "$repo_root/scripts/check_quality_variants_are_fresh.py" ] || return 0
    if python3 "$repo_root/scripts/check_quality_variants_are_fresh.py" >/dev/null 2>&1; then
        log "generated quality variants are current"
        return 0
    fi
    warn "generated quality variants are STALE or MISSING after regeneration"
    log "   the game draws old art at Low/Medium and may draw none at all:"
    log "     python3 scripts/check_quality_variants_are_fresh.py   # what is stale"
    log "     ./scripts/regen/quality_variants.sh                   # rebuild it"
}

# ⛔⛔ FRESHNESS AND PRESENCE ARE TWO DIFFERENT FAILURES, and only one of them
# had an instrument. `verify_generated_content_is_current` above asks *"is the
# published tier art OLDER than its source?"* — which says nothing about art
# that was never published at all, because A FILE THAT DOES NOT EXIST CANNOT BE
# STALE.
#
# ⚠ MEASURED 2026-08-30, the first time `cargo test --workspace --lib` was run
# to completion (D-QTT-1): it failed inside `ambition_render` on
# `a_left_drawn_character_faces_the_way_they_are_going_like_a_right_drawn_one`,
# which uses `goblin_cave_dagger` as its canonical RIGHT-drawn sheet.
# `record_for_sheet_key` returned `None` because no roster line ever published
# it. The panic named handedness; the defect was a missing ASSET, and the whole
# goblin weapon family was absent with it.
#
# ⭐ GENERATED ART IS GITIGNORED, so "it works on my checkout" is the EXPECTED
# symptom of this class rather than a surprising one: whoever rendered the
# target by hand has it and nobody else does. That is exactly why it belongs in
# the bootstrap and not in a person's memory.
#
# GENERATES rather than only reporting, because the whole lesson of the
# quality-variant row above is that a checker nothing acts on is a checker
# nobody runs.
regenerate_missing_published_sheets() {
    have python3 || return 0
    local checker="$repo_root/scripts/check_published_sheets_are_present.py"
    [ -f "$checker" ] || return 0

    # Through the renderer's own interpreter: the check asks each target what it
    # DECLARES it installs, which needs the package importable. Without it the
    # script says "cannot check" and succeeds rather than inventing a verdict.
    #
    # ⛔ WHICH MADE THIS A FALSE GREEN FOR AS LONG AS IT EXISTED. The call here
    # was `tool_python ambition_sprite2d_renderer` — a function that is not
    # defined anywhere, given a bare tool name where the resolver wants a
    # DIRECTORY. It therefore always fell through to the bare `python3` that
    # cannot import the renderer, the checker duly reported "cannot check" and
    # exited 0, and setup logged "every rostered sheet is published" without
    # having checked one. `[ -x "$py" ]` could not catch it either: that test is
    # false for the bare name `python3`, so the guard "corrected" a good path to
    # the same fallback.
    local py
    py="$(ambition_select_tool_python \
        "$repo_root/tools/ambition_sprite2d_renderer" AMBITION_SPRITE_PYTHON)"
    ambition_python_exists "$py" || py="python3"

    local missing
    if missing="$("$py" "$checker" 2>/dev/null)"; then
        log "every rostered sheet is published"
        return 0
    fi

    warn "rostered sheets are MISSING — regenerating them"
    printf '%s\n' "$missing" | sed 's/^/     /'
    local names
    names="$(printf '%s\n' "$missing" | awk '/^ *missing /{print $2}')"
    [ -n "$names" ] || return 0
    local name
    for name in $names; do
        log "  publishing $name"
        "$repo_root/scripts/regen/sprites.sh" --target "$name" --force >/dev/null 2>&1 \
            || warn "  could not publish $name — run sprites.sh --target $name to see why"
    done
    if "$py" "$checker" >/dev/null 2>&1; then
        log "every rostered sheet is published"
    else
        warn "some rostered sheets are still missing; see the list above"
    fi
}

# ⛔ `scripts/agent_query.py` IS STEP 2 OF THE COLD START AGENTS.md MANDATES, AND
# ON A FRESH CLONE IT ANSWERED NOTHING. The `.agent/` packets it queries are
# generated, not committed — `git ls-files .agent` is two files — so a new
# checkout got `⚠ index has no generation stamp` and an empty packet from the
# first command the guide tells an agent to run. Every other kind of generated
# content in this repo is built by this script; this one was left to a person
# knowing to run it.
#
# Reports rather than fails: `source_navigation.sh` ends with the agent-KB
# audit, which is repository DOC hygiene (frontmatter keys, stray files, a
# dangling doc reference) and has nothing to do with whether this checkout is
# runnable. Setup must not refuse a working environment over it.
regenerate_source_navigation() {
    if [ "$skip_assets" -eq 1 ]; then
        log "skipping generated navigation"
        return 0
    fi
    [ -x "$repo_root/scripts/regen/source_navigation.sh" ] || return 0
    # The ECS inventory resolves the dependency graph through cargo, and prints
    # `resolved dependency graph unavailable` and carries on when it cannot —
    # a quietly poorer index rather than an error.
    if [ -f "$HOME/.cargo/env" ]; then
        # shellcheck disable=SC1091
        . "$HOME/.cargo/env"
    fi
    log "regenerating .agent/ navigation"
    if "$repo_root/scripts/regen/source_navigation.sh" >/dev/null 2>&1; then
        log "generated navigation is current"
        return 0
    fi
    # The indexes are written before the audit runs, so a non-zero exit here
    # does not mean they are missing. Say which it was.
    if [ -d "$repo_root/.agent/index" ]; then
        warn "navigation regenerated, but the agent-KB audit reported doc-hygiene problems"
        log "   they do not affect the build:"
        log "     ./scripts/regen/source_navigation.sh   # what it objects to"
    else
        warn "navigation regeneration FAILED; scripts/agent_query.py will answer nothing"
        log "     ./scripts/regen/source_navigation.sh   # see why"
    fi
}

# **`-lstdc++` fails at the END of a cold profiling build, and having `g++`
# installed does not prevent it.** Tracy's client is C++, so `tracy-client-sys`
# emits `cargo:rustc-link-lib=stdc++` and every `--features profile` link ends
# in `-lstdc++`. clang resolves that against the ONE gcc version directory it
# selected — the newest COMPLETE installation under `/usr/lib/gcc/<triple>/` —
# so if that directory has no `libstdc++.so`, the link dies with
# `mold: library not found: stdc++` while `g++` sits installed and innocent,
# because the version that owns the symlink is not the version clang picked.
#
# ⚠ Met on `calculex` 2026-08-29 (Ubuntu 22.04, clang 14): gcc 9 and gcc 11
# both had `libstdc++.so`, `apt install g++` reported "already the newest
# version", and the link still failed after all 537 rlibs had compiled.
#
# ⛔ The message reads like a stale incremental cache or a full disk. It is
# neither, and nothing under `target/` should be deleted for it.
#
# Same bargain as `check_cargo_target_dir_is_reachable` below: turn a linker
# error at the end of a twenty-minute build into a sentence at the start of
# setup, naming the package to install.
check_profiling_cxx_stdlib() {
    [ "$want_profiling" -eq 1 ] || return 0
    have clang || return 0
    # clang echoes the bare name back when it cannot resolve it, so the test is
    # "did I get a path that exists", not "did the command succeed".
    resolved=$(clang -print-file-name=libstdc++.so 2>/dev/null || true)
    [ -e "$resolved" ] && return 0

    # `-v` prints the selection before it fails on the empty translation unit.
    selected=$(clang -v -x c++ /dev/null -o /dev/null 2>&1 \
        | sed -n 's/^Selected GCC installation: //p' | head -1 || true)
    log "⛔ clang cannot resolve libstdc++.so; a --features profile build will"
    log "   fail at the LINK with: mold: library not found: stdc++"
    if [ -n "$selected" ]; then
        log "   clang selected GCC installation: $selected"
        log "   install the C++ stdlib dev package for THAT version:"
        log "     sudo apt install libstdc++-$(basename "$selected")-dev"
    else
        log "   could not read clang's GCC selection; run it by hand:"
        log "     clang -v -x c++ /dev/null -o /dev/null 2>&1 | grep 'GCC installation'"
    fi
    log "   ⚠ \"g++ is already the newest version\" does NOT settle this: the"
    log "   version that owns the symlink need not be the one clang picked."
    for d in /usr/lib/gcc/*/*/; do
        [ -e "$d/libstdc++.so" ] && log "   present in: $d"
    done
    log "   ⛔ this is not a stale cache and not a full disk — do not delete"
    log "   anything under target/ for it."
}

check_desktop_target() {
    if [ "$skip_cargo_check" -eq 1 ]; then
        log "skipping Cargo fetch/check"
        return 0
    fi
    if [ -f "$HOME/.cargo/env" ]; then
        # shellcheck disable=SC1091
        . "$HOME/.cargo/env"
    fi
    have cargo || fatal "cargo is required for the desktop target check"

    log "fetching locked Cargo dependencies"
    cargo fetch --locked
    log "checking the desktop game target"
    cargo check --locked -p ambition_app --bin ambition_game_bin
}

# **The committed `target-dir` is one user's home, and a bare `cargo build` by
# anyone else dies with `Permission denied` at a path they never chose.**
#
# `.cargo/config.toml` sets `target-dir = "/home/joncrall/ambition-target"`, and
# the REASON is sound and documented there: the repo sits on a virtiofs share, so
# an identical path string resolving to different local disks keeps VM and host
# fingerprints from co-mingling. It is only the username that does not travel.
#
# **and it cannot be overridden by a per-user config.** Cargo merges config
# files with the one nearest the working directory winning, so the repo's
# `.cargo/config.toml` beats `~/.cargo/config.toml`. `CARGO_TARGET_DIR` in the
# ENVIRONMENT is the only override — which is why `run_game.sh` and the
# rust-analyzer bridge both export it and work fine.
#
# So this does not change the config, the build, or anyone's cache. It turns an
# unexplained linker-adjacent permission error at the end of a long first build
# into a sentence at the start of setup.
check_cargo_target_dir_is_reachable() {
    if [ -n "${CARGO_TARGET_DIR:-}" ]; then
        return 0
    fi
    configured=$(sed -n 's/^target-dir *= *"\(.*\)"/\1/p' .cargo/config.toml 2>/dev/null | head -1)
    [ -n "$configured" ] || return 0
    parent=$(dirname "$configured")
    if [ -w "$configured" ] 2>/dev/null || [ -w "$parent" ] 2>/dev/null; then
        return 0
    fi
    log "⛔ the committed cargo target-dir is not writable by $(whoami): $configured"
    log "   a bare 'cargo build' will fail with Permission denied at that path."
    log "   export CARGO_TARGET_DIR to somewhere you own, for example:"
    log "     export CARGO_TARGET_DIR=\"\$HOME/ambition-target\""
    log "   ⚠ a per-user ~/.cargo/config.toml will NOT help: cargo lets the"
    log "   config nearest the working directory win, so the repo's beats yours."
    log "   see dev/journals/code_smells.md, 2026-07-26 entry, for why the path"
    log "   is absolute in the first place (virtiofs cache separation)."
}

check_cargo_target_dir_is_reachable
install_system_packages
ensure_rust
ensure_profiling_tools
check_profiling_cxx_stdlib
ensure_resource_tally
ensure_submodules
ensure_python_tools
# Before the assets: scripts/regen/music.sh sources this phase's env.sh to find the
# instruments, so installing them afterwards would render the fallback anyway.
ensure_audio_libraries
regenerate_assets
regenerate_source_navigation
check_desktop_target

echo
if [ "$skip_assets" -eq 0 ] && [ "$skip_cargo_check" -eq 0 ]; then
    log "developer setup complete"
    log "the checkout is ready for: ./run_game.sh"
    # Plain `if`s, not `[ ... ] && log`: this is the last statement in the
    # script, and a short-circuit whose test is false would make a successful
    # setup exit non-zero.
    if [ "$want_profiling" -eq 0 ]; then
        log "not installed (one argument away):"
        log "   --profile          profiling + cargo analysis toolchain"
    fi
    if [ "$want_audio_libraries" -eq 0 ]; then
        warn "sampled instruments were SKIPPED; music cues cannot render here"
        log "   rerun without --skip-audio-libraries before regenerating music"
    fi
else
    log "selected developer setup phases complete"
    log "rerun without skip flags for the zero-to-runnable setup"
fi
