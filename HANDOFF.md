# HANDOFF — cosmic-app-library fork

Resume line: *"Continue cosmic-app-library. Read PLAN.md and HANDOFF.md in
~/Projects/cosmic-app-library and do the next step."* Start with that dir as cwd.

## Current state (2026-07-18, session 2)
- Phases 0–10 CODE-COMPLETE on `dev`, pushed to origin through 02e4cbf.
  `cargo check` clean (only the two pre-existing unused-import warnings).
- This session added (each its own commit):
  - 0cfc96a power menu closes on any click outside it (CloseContextMenu also
    resets `power_menu_open`).
  - fe59b73 Phase 6: `LibraryPosition::Center` + `window_width`/`window_height`
    config keys (defaults 1200×690, clamp 600×400..screen).
  - bd929d9 Phase 9: frosted-blur bleed (upstream #387) — blur rect now matches
    the window instead of f32::MAX. **Upstream PR candidate** (needs a
    master-based variant with literal 1200/690).
  - e51573b Phase 7: favorites drag-reorder (order = config vec; tiles are drop
    targets, trailing append zone; fixed two ordering bugs incl. filter_apps
    alphabetical re-sort).
  - 46339fe cherry-pick upstream PR #385: stable entry widget ids (arrow-key
    nav after search).
  - 02e4cbf Phase 8: `keep_in_home` folders — create-dialog toggle "Also show
    apps in Home"; Home filter + add-to-Home strip loop respect the flag.
- IN FLIGHT at session end: Sonnet agent implementing #338 (accent-color
  selected-group highlight), #306 (group button clickable area), #235/335
  (long-name ellipsis, re-derived from upstream draft PR #360). If its work
  is uncommitted, check `git status` / `git diff` and finish/commit it.
- Phase 1 upstream PR still open: pop-os/cosmic-app-library#388.

## Next step
1. Owner on-device verify (nothing verified live yet this session):
   power-menu outside-click, Center position + window size config keys,
   favorites drag-reorder (esp. drop targets vs drag sources), keep-in-home
   folder toggle, blur bleed with frosted glass on, arrow keys after search.
   `just build-release && sudo just install` or `cd packaging && makepkg -si`.
2. Commit/verify the in-flight fixes batch if not already committed.
3. Open upstream PR for the #387 blur fix (rebase a master-based variant).
4. BACKLOG candidates from research: upstream PR #378 hide/unhide apps
   (conflicts with our tree, adapt manually), #153 fractional-scaling size,
   #386 XDG dedupe, #164 all-apps section, #177 icon-only mode.

## Gotchas (stable)
- PKGBUILD builds from GitHub `dev` — push before `makepkg`.
- Bare `just install` gets clobbered by pacman updates; the PKGBUILD replaces
  the official package. Rollback: `sudo pacman -S cosmic-app-library`.
- Keep `master` = upstream mirror (synced 2026-07-18, nothing missing);
  `feat/library-position` = PR #388 branch only.
- cosmic-config: missing keys fall back via the manual `impl Default for
  AppLibraryConfig` (derive get_entry starts from Self::default()); AppGroup's
  new `keep_in_home` uses `#[serde(default)]`. Old configs safe either way.
- Favorites = separate Vec (NOT in groups) so Home still shows them; favorites
  DISPLAY order = vec order, and filter_apps deliberately skips its
  alphabetical sort only in favorites view with empty search.
- Favorites tile drag_ids offset by FAVORITE_TILE_DRAG_ID_BASE (1_000_000) to
  avoid colliding with group-row drag ids.
- No secrets in this project.
