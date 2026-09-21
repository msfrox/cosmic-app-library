# Building cosmic-app-library on Sapphire (the `claude-code` container)

Written 2026-09-21 (away task `cosmic-toolchain-prep`), so the next away
session doesn't re-pay the cold-build cost. This container had neither cargo
nor any Wayland dev headers going in. Total time from nothing to a green
`cargo test`: roughly 45 minutes, most of it the cold `cargo build`.

## Environment

- Debian 12 (bookworm), `claude-code` container, user `node`.
- **Passwordless sudo and `apt` both work here** — this is a newer fact than
  HANDOFF.md's "sudo needs a password in the agent shell" note from the
  2026-07-19 session; that note is now stale for *this* container (it may
  still be true elsewhere on Sapphire — not re-verified against other
  containers).

## 1. Install rustup

`rustup`/`cargo` are not preinstalled. Standard install, then trust
`rust-toolchain.toml` (pins `1.93.0`) to fetch the pinned toolchain the first
time `cargo`/`rustup show` runs inside the repo:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/rustup-init.sh
sh /tmp/rustup-init.sh -y --default-toolchain none
source "$HOME/.cargo/env"
cd /workspace/cosmic-app-library
rustup show   # auto-installs 1.93.0 from rust-toolchain.toml
```

## 2. System packages

🔴 **`apt-get install` aborts the *entire* transaction if even one package
name is wrong or missing from the repos** — it installs nothing, not even the
valid names in the same command. This cost a retry here because `just` isn't
apt-packageable on bookworm (see §3) and was in the first list.

Everything below was needed to get `cargo build` clean (pulled from
`debian/control`'s Build-Depends and `flake.nix`'s `nativeBuildInputs`/
`buildInputs`, then filled out for the rest of the iced/wgpu/x11rb/libinput
dependency tree libcosmic pulls in transitively — the two source-of-truth
files under-list what's actually needed to *link*, not just what upstream's
own CI happens to already have preinstalled):

```bash
sudo apt-get update -qq
sudo apt-get install -y \
  pkg-config \
  libxkbcommon-dev \
  libxkbcommon-x11-dev \
  libwayland-dev \
  wayland-protocols \
  libinput-dev \
  libudev-dev \
  libgtk-4-dev \
  libgtk-3-dev \
  libglib2.0-dev \
  desktop-file-utils \
  libegl1-mesa-dev \
  libgles2-mesa-dev \
  libssl-dev \
  libfontconfig1-dev \
  libfreetype6-dev \
  libx11-dev \
  libxcursor-dev \
  libxrandr-dev \
  libxi-dev \
  libgl1-mesa-dev \
  build-essential \
  cmake \
  clang \
  libclang-dev
```

This list got the *whole* dependency tree to compile in one pass — no
iterative "add a package, retry the linker error" cycle was needed once this
set was installed. If a future upstream bump needs something not here, the
error will be a `pkg-config`/linker failure naming the missing `.pc` file or
`.so`; map that name to its `-dev` package the normal way.

## 3. `just`

Not in the bookworm apt repos (no `just` binary package on Debian 12 as of
2026-09). Install via cargo instead — it's a small crate, builds in well
under a minute:

```bash
cargo install just
just --version   # confirms it's on PATH via ~/.cargo/bin
```

## 4. Cold build

```bash
cd /workspace/cosmic-app-library
cargo fetch   # pulls crates.io + the git deps (libcosmic, cosmic-panel, etc.)
cargo build
```

🔴 **This does not fit inside a single 120s foreground command on this box.**
The first `cargo build` compiles several hundred crates including libcosmic
itself from git — it took roughly 6–10 minutes of wall time here. Run it via
the harness's own backgrounding (or a `run_in_background` Bash call) rather
than fighting the foreground timeout; a `timeout 600 cargo build` wrapper
risks getting killed by its own timeout before cargo finishes; if that
happens the incremental build state on disk is preserved, so simply
re-running `cargo build` afterwards resumes rather than restarts from zero.

Subsequent builds (after the dependency tree is already compiled) are fast:
under a minute for `cargo build`, under 20s for `cargo test` once `cargo
build` has already primed the target dir.

## 5. Verified working here, 2026-09-21

```bash
source "$HOME/.cargo/env"
cd /workspace/cosmic-app-library
cargo build           # clean except the two pre-existing unused-import
                       # warnings HANDOFF.md already documents
cargo test             # all unit tests in src/app_group.rs pass
cargo clippy --all-targets   # see PLAN.md Phase 16 for the result recorded
                             # that session — check there before re-running
                             # cold, since it's the slow one to redo
```

## What this does NOT get you

No display, no Wayland compositor, no way to actually launch the built binary
and see it. `cargo build`/`cargo test`/`cargo clippy` are real, useful
signal — they are not a substitute for owner verification on a COSMIC
desktop (RUBY2). Every phase in `PLAN.md` marked "⬜ (verify)" stays that way
regardless of how green the build is here.
