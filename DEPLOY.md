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

## Update conflict (important)
`just install` writes to the same `/usr/bin` path as the official package, so a
`pacman -Syu` that updates `cosmic-app-library` will overwrite the fork. Options:
- Re-run `sudo just install` after such updates, OR
- Phase 4: build a PKGBUILD (`cosmic-app-library-msfrox`) and add `cosmic-app-library`
  to `IgnorePkg` in `/etc/pacman.conf` (or replace via `provides`/`conflicts`).

## Upstream PR (Phase 1)
- Push `feat/library-position` to `origin`, open PR to `pop-os/cosmic-app-library`
  referencing issue #336. Keep that branch scoped to position only.

## Secrets
None. Desktop app with no credentials — no SECRETS-LOG.md.
