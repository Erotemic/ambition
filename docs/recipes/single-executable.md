---
status: current
last_verified: 2026-10-09
---

# One file to give a player

`scripts/package_single_exe.py` builds the game as one executable file with
its asset tree inside, for Linux and for Windows. The player needs no other
file and no directory beside it. The script's `--help` is the authority for
its options.

## Prerequisites

The Linux file needs only the toolchain of the workspace and `strace` (the
script uses it to prove the file stands alone).

The Windows file is a cross build from Linux. Install the linker and the Rust
target one time:

```bash
sudo apt-get install gcc-mingw-w64-x86-64 g++-mingw-w64-x86-64
rustup target add x86_64-pc-windows-gnu
```

The package holds the art and the music of the checkout that builds it. Run
`scripts/regen/sprites.sh` and `scripts/regen/music.sh` first on a checkout
that does not have them: an asset that is absent from the checkout is absent
from the file, and the game then draws a placeholder or plays no track.

## Common workflows

```bash
# Build both files and verify them.
python3 scripts/package_single_exe.py

# One platform.
python3 scripts/package_single_exe.py --platform linux
python3 scripts/package_single_exe.py --platform windows

# Package the programs the last build wrote (no cargo).
python3 scripts/package_single_exe.py --skip-build

# Each quality tier, as the Steam Deck package has.
python3 scripts/package_single_exe.py --assets all
```

The files are `target/single_exe/ambition-linux-x86_64` and
`target/single_exe/ambition-windows-x86_64.exe`.

Each build uses the `ship` profile (optimization level 3, link-time
optimization across the whole program, symbols stripped, no debug info) and
the `desktop_player` feature set of `ambition_app`: the visible game with no
development tools, and with the worlds and the content data compiled in.
The last step of a `ship` build is one long link-time optimization on one
core. One build took 49 minutes on a 30-core machine (2026-10-09).

## What is in the file

```text
[ the program ][ each asset file ][ index ][ footer, 32 bytes ]
```

The operating system loads the program from the front of the file and
ignores the rest. The game opens its own file, reads the footer, and serves
each asset from the bytes between
(`crates/ambition_asset_manager/src/exe_bundle.rs` has the layout). The
default asset source and `game://` are then the same reader.

A program that carries its assets reads no asset from the disk. The asset
catalog asks the bundle only, and a world loads from its compiled-in copy.
The directory of the checkout is an absolute path of the machine that built
the program: a file found there is a file the player does not have.

`--assets` selects how much of the asset tree goes in:

| Choice | Contents |
| --- | --- |
| `full` (default) | The full-resolution art. The smaller quality tiers stay out, and the game falls back to the full-resolution file when a tier is absent. The shared sprite pack keeps only the pages a consumer reaches (`scripts/measure_pack_reachability.py`). |
| `all` | Each file of each tier, as the Steam Deck package has. |

With `full`, a machine with a weak GPU selects a lower quality and gets
full-resolution textures. It draws the same game and uses more video memory
than the tier would.

## What the script checks

- Each file reads back: the program at its front is the bytes cargo wrote,
  and each asset is the bytes of its source file.
- The Linux file stands alone. The script runs it with no window from an
  empty directory, into the first room of the campaign, under `strace`. It
  refuses the file if the game opened an asset file outside itself, asked
  for an asset it does not carry, or did not reach the room.

The script does not run the Windows file: it has no Windows. Start that file
on Windows to prove it.

The script does not open a window. To see the game from the file, run it.
To run it as a player's machine does, hide the checkout from it (a private
mount namespace with `crates/` and `game/` empty).

## What the Linux file needs from the machine

The Linux file is one file, and it is not a static program. A game that
draws with the GPU loads the driver of the machine while it runs, and a
static program cannot load a library. The file names these libraries in its
header:

```text
libasound.so.2  libudev.so.1  libwayland-client.so.0
libgcc_s.so.1   libm.so.6     libc.so.6
```

`--print-needs` prints that list for the file it built. While it runs, the
game also loads the window system (X11 with `libxkbcommon`, or Wayland), and
`libvulkan.so.1` with the GPU driver. A desktop Linux that runs other games
has each of them.

The C library sets which machines the file starts on. A file built on Ubuntu
24.04 needs glibc 2.39 or newer (`objdump -T <file> | grep -o 'GLIBC_[0-9.]*'
| sort -V | tail -1`), so it does not start on Ubuntu 22.04. Build on the
oldest system the players have.

## What the Windows file needs from the machine

The program imports only libraries that Windows has (`KERNEL32`, `USER32`,
`msvcrt`, and others of the system). Read the list with
`x86_64-w64-mingw32-objdump -p <file> | grep 'DLL Name'`. It is a console
program: Windows opens a console window beside the game window, and the
game writes its log there.

## Known limits

- The on-screen touch controls show on the desktop, as they do in a
  development build. *Settings → Touch Overlay* hides them.
- Saves and settings go to the data directory of the player's account
  (`%APPDATA%` on Windows, the XDG data directory on Linux), not beside the
  file.
- `scripts/package_asset_guard.py compose` did not pass on the checkout that
  wrote this page (2026-10-09). Its scan reads the section paths of
  `audio/music_cues.ron` as paths from the asset root (they are paths from
  the cue's `asset_root`), and `data/vanity_card.ron` names the frames of the
  old full-frame card, which that checkout does not have. This script takes
  the file set only (`collect_source_files`) and does not run that scan.
