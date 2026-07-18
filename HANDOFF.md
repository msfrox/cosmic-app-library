# HANDOFF — cosmic-app-library fork

Resume line: *"Continue cosmic-app-library. Read PLAN.md and HANDOFF.md in
~/Projects/cosmic-app-library and do the next step."* Start with that dir as cwd.

## Current state (2026-07-18)
- ALL FOUR PHASES CODE-COMPLETE on `dev`; everything compiles clean (only the
  two pre-existing unused-import warnings).
- **Phase 1 upstream PR is OPEN:** https://github.com/pop-os/cosmic-app-library/pull/388
  (fixes #336). `feat/library-position` = feature + both position fixes
  cherry-picked (f4bcf5d, c3c9cef), pushed to origin.
- **Phase 2 (0de8981):** home header right of search — Settings icon (launches
  cosmic-settings, dismisses library) + Power icon → inline popover menu.
  Power DBus code copied from cosmic-applet-power into `src/power/`;
  log out/restart/shutdown confirm via cosmic-osd (DBus fallback),
  lock/suspend immediate. New deps: logind-zbus, nix "user" feature.
- **Phase 3 (b91dc02):** Favorites — `favorites: Vec<String>` on config,
  sentinel `FAVORITES_GROUP = usize::MAX` (branches in add_entry/remove_entry/
  filtered), locked group button next to Home, context-menu Add/Remove from
  Favorites, drag-to-add, opens on Favorites when non-empty else Home,
  search in Favorites = global search, empty-state hint.
- **Phase 4:** `packaging/PKGBUILD` (`cosmic-app-library-msfrox`,
  provides/conflicts official pkg, builds from GitHub `dev`). DEPLOY.md updated.

## Next step (owner, on-device)
1. Verify Phase 2+3 in live session: relaunch library; check header buttons,
   power menu actions, favorites add/remove/drag, default-open behavior.
   (A fresh `just build-release` binary + `sudo just install` gets it live, or
   `cd packaging && makepkg -si` for the permanent package.)
2. Comment/respond on PR #388 if pop-os reviews.
3. Optional polish (BACKLOG): position toggle UI, favorites in folder views.

## Gotchas (stable)
- PKGBUILD builds from GitHub `dev` — push before `makepkg`.
- Bare `just install` gets clobbered by pacman updates; the PKGBUILD replaces
  the official package and doesn't. Rollback: `sudo pacman -S cosmic-app-library`.
- Keep `master` = upstream mirror; `feat/library-position` = PR branch only.
- cosmic-config enums store as plain text; missing keys → serde defaults
  (`position` → Auto, `favorites` → empty), so old configs are safe.
- Favorites deliberately NOT in `config.groups` — Home's filter excludes apps
  matched by any group, so a favorites group there would hide apps from Home.
- No secrets in this project.
