#!/usr/bin/env bash
# Idempotent apt installation with lazy privilege escalation.
#
# `apt_ensure <pkg>...` installs required packages and returns non-zero on
# failure. `apt_ensure_optional <pkg>...` never fails and never escalates solely
# for an unavailable optional package; it filters candidates with unprivileged
# `apt-cache` first. Already-installed packages require no sudo or apt update.
#
# Installation retries once after `apt-get update`; UPDATE requests that refresh
# up front. At most one update runs per process. APT_ENSURE_LOG_PREFIX controls
# the message prefix.

# Whether `apt-get update` has already run in this process. At most once:
# apt_ensure and apt_ensure_optional are called separately but share metadata.
_apt_ensure_updated=0

_apt_ensure_log() {
    printf '%s %s\n' "${APT_ENSURE_LOG_PREFIX:-[apt-ensure]}" "$*"
}

_apt_ensure_warn() {
    printf '%s warning: %s\n' "${APT_ENSURE_LOG_PREFIX:-[apt-ensure]}" "$*" >&2
}

_apt_ensure_have() {
    command -v "$1" >/dev/null 2>&1
}

# True when apt is usable at all. A non-Debian host is not an error; it just
# means the caller has to get these packages some other way.
apt_ensure_supported() {
    _apt_ensure_have apt-get && _apt_ensure_have dpkg-query
}

apt_ensure_installed() {
    dpkg-query -W -f='${Status}' "$1" 2>/dev/null | grep -q "install ok installed"
}

# Whether the configured repositories carry this package at all. Reads only
# local apt metadata, so it costs no privileges and no network.
apt_ensure_available() {
    apt-cache show "$1" >/dev/null 2>&1
}

# Echo the command prefix for a privileged apt-get, or return non-zero when
# this process cannot get privileges.
#
# sudo reads a password from the terminal, not stdin, so a sudo that needs
# one HANGS in a context with no terminal instead of failing. `sudo -n` asks
# whether privileges are already cached; only when they are not do we care
# whether there is a terminal to type into.
_apt_ensure_sudo_prefix() {
    if [[ "$(id -u)" -eq 0 ]]; then
        return 0
    fi
    _apt_ensure_have sudo || return 1
    if sudo -n true 2>/dev/null; then
        printf 'sudo'
        return 0
    fi
    if [[ -t 0 || -t 1 ]]; then
        printf 'sudo'
        return 0
    fi
    return 1
}

_apt_ensure_update() {
    local -a sudo_prefix=()
    if [[ -n "$1" ]]; then
        sudo_prefix=("$1")
    fi
    _apt_ensure_updated=1
    _apt_ensure_log "refreshing apt metadata"
    "${sudo_prefix[@]}" apt-get update -y || return 1
}

# Install packages, refreshing apt metadata only if a first attempt fails.
_apt_ensure_install() {
    local sudo_prefix="$1"
    shift
    local -a sudo_cmd=()
    if [[ -n "$sudo_prefix" ]]; then
        sudo_cmd=("$sudo_prefix")
    fi

    if [[ -n "${UPDATE:-}" && "$_apt_ensure_updated" -eq 0 ]]; then
        _apt_ensure_update "$sudo_prefix" || _apt_ensure_warn "apt-get update failed; trying the install anyway"
    fi

    if DEBIAN_FRONTEND=noninteractive "${sudo_cmd[@]}" apt-get install -y "$@"; then
        return 0
    fi
    if [[ "$_apt_ensure_updated" -ne 0 ]]; then
        return 1
    fi
    _apt_ensure_log "install failed; refreshing apt metadata and retrying once"
    _apt_ensure_update "$sudo_prefix" || return 1
    DEBIAN_FRONTEND=noninteractive "${sudo_cmd[@]}" apt-get install -y "$@"
}

apt_ensure() {
    local -a requested=("$@")
    if [[ "${#requested[@]}" -eq 0 ]]; then
        return 0
    fi

    if ! apt_ensure_supported; then
        _apt_ensure_warn "apt-get/dpkg-query not found; skipping host packages: ${requested[*]}"
        return 0
    fi

    local -a missing=()
    local pkg
    for pkg in "${requested[@]}"; do
        apt_ensure_installed "$pkg" || missing+=("$pkg")
    done

    if [[ "${#missing[@]}" -eq 0 ]]; then
        _apt_ensure_log "host packages already installed (${#requested[@]}); no sudo needed"
        return 0
    fi

    local sudo_prefix
    if ! sudo_prefix="$(_apt_ensure_sudo_prefix)"; then
        _apt_ensure_warn "cannot obtain privileges to install: ${missing[*]}"
        return 1
    fi

    _apt_ensure_log "installing required host packages: ${missing[*]}"
    _apt_ensure_install "$sudo_prefix" "${missing[@]}"
}

apt_ensure_optional() {
    local -a requested=("$@")
    if [[ "${#requested[@]}" -eq 0 ]]; then
        return 0
    fi

    if ! apt_ensure_supported; then
        return 0
    fi

    local -a missing=()
    local pkg
    for pkg in "${requested[@]}"; do
        apt_ensure_installed "$pkg" || missing+=("$pkg")
    done
    if [[ "${#missing[@]}" -eq 0 ]]; then
        return 0
    fi

    # Drop what the repositories do not carry BEFORE asking for privileges, so
    # a permanently unavailable package cannot make every future run escalate.
    local -a installable=()
    for pkg in "${missing[@]}"; do
        if apt_ensure_available "$pkg"; then
            installable+=("$pkg")
        else
            _apt_ensure_warn "$pkg is unavailable from the configured apt repositories"
        fi
    done
    if [[ "${#installable[@]}" -eq 0 ]]; then
        return 0
    fi

    local sudo_prefix
    if ! sudo_prefix="$(_apt_ensure_sudo_prefix)"; then
        _apt_ensure_warn "skipping optional host packages (no privileges): ${installable[*]}"
        return 0
    fi

    _apt_ensure_log "installing optional host packages: ${installable[*]}"
    if _apt_ensure_install "$sudo_prefix" "${installable[@]}"; then
        return 0
    fi

    # One unsatisfiable package must not cost the caller the rest of the list.
    _apt_ensure_warn "batch install of optional packages failed; retrying individually"
    for pkg in "${installable[@]}"; do
        _apt_ensure_install "$sudo_prefix" "$pkg" \
            || _apt_ensure_warn "optional package could not be installed: $pkg"
    done
    return 0
}
