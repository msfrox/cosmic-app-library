# DEPLOY — cosmic-app-library fork

This is a native COSMIC desktop app, not a web service. "Deploy" = build and
install the binary so COSMIC launches it instead of the packaged one.

## Build & install (dev machine, CachyOS/COSMIC)
```
cd ~/Projects/cosmic-app-library
just build-release            # long first build; hundreds of crates
sudo just install             # installs to /usr/bin/cosmic-app-library (APPID com.system76.CosmicAppLibrary)
```
Relaunch the app library (Super key / panel button) to pick up the new binary.

## Rollback
```
sudo pacman -S cosmic-app-library     # restores the official binary
```

## Permanent install (recommended): PKGBUILD
`packaging/PKGBUILD` builds `cosmic-app-library-msfrox` from the GitHub fork's
`dev` branch. It `provides`/`conflicts` the official `cosmic-app-library`, so
installing it REPLACES the official package and `pacman -Syu` never clobbers
the fork (different pkgname). Push `dev` first — it builds from GitHub.
```
cd ~/Projects/cosmic-app-library/packaging
makepkg -si          # builds and installs, replacing cosmic-app-library
```
To update later: push `dev`, then re-run `makepkg -si` (pkgver auto-bumps from
git). To go back to the official package:
```
sudo pacman -S cosmic-app-library     # pacman swaps the fork back out
```

## Update conflict (if using bare `just install` instead)
`just install` writes to the same `/usr/bin` path as the official package, so a
`pacman -Syu` that updates `cosmic-app-library` will overwrite the fork —
re-run `sudo just install` after such updates, or use the PKGBUILD above.

## Upstream PR (Phase 1)
Open: https://github.com/pop-os/cosmic-app-library/pull/388 (fixes #336, from
`feat/library-position`). Keep that branch scoped to position only; rebase on
upstream if requested by reviewers.

## Secrets
None. Desktop app with no credentials — no SECRETS-LOG.md.
