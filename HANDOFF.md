# HANDOFF — cosmic-app-library fork

Resume line: *"Continue cosmic-app-library. Read PLAN.md and HANDOFF.md in
~/Projects/cosmic-app-library and do the next step."* Start with that dir as cwd.

## Current state (2026-07-19, session 5)
- Phases 0–15 CODE-COMPLETE on `dev`; release build + `cargo test` (5) pass.
  Phases 11–15 are NOT yet owner-verified live — nothing has been installed
  from this session (sudo needs a password in the agent shell).
- Phase 15: power-menu close-on-outside-click regression fixed (libcosmic's
  modal popover now swallows every mouse event, so the ancestor mouse_area
  `on_release` never fired — switched to `modal(false)` + `on_close`, and app
  tiles go unclickable while the menu is open so the dismissing click can't
  launch anything); COSMIC Settings launcher icon changed to
  `com.system76.CosmicSettings` (it was the identical gear to Library
  Settings); folder tiles drag-reorder; `default_page` setting (Auto/Home/
  Favorites).
- Phase 14 (dd36357): settings-page togglers hide the header Settings/Power
  buttons (`show_settings_button`/`show_power_button`, serde default true;
  gear icon always visible; hiding power closes an open power menu).
- Phase 12 (5e6951c): favorites positional drops. Each favorites cell = three
  sibling dnd destinations: 24px strips flanking the tile insert-reorder
  before/after (accent insertion-bar hover hint); the tile itself creates or
  joins a FAVORITES folder (accent selected-style hint). Favorites folders are
  finally creatable by drag. `fav_drop_hint` state drives hints; cleared on
  leave/finish/cancel/hide. Semantic change: dropping ON a favorite now
  combines (was: reorder-before) — reorder uses the strips between icons.
- Phase 13 (b7f517f, Sonnet agent + main-thread review): window size now
  derives from `grid_columns` (4–12, default 7) × `grid_rows` (2–8, default
  3); `window_width`/`window_height` config keys REMOVED (stale key files
  ignored; defaults reproduce the old 1200×690). Keyboard row-nav + grid
  chunking share the column count. In-place settings page: gear icon
  (emblem-system-symbolic) in the Home/Favorites header, ESC-closable; has
  Position dropdown (Auto/Top/Bottom/Center) + Columns/Rows steppers; changes
  write config and re-apply layer padding live via set_padding.
- Phase 9 blur findings unchanged (see PLAN): full-surface blur kept; our
  BlurSurface(RESERVED) never reaches the wire; self.size can stay 1920×1080.

## Next step
1. Owner: `just build-release && sudo just install`, relaunch, then verify live.
   **Phase 15 (new, highest risk):**
   - (a) Power menu: open it, click on empty library background → closes. Click
     an app tile while it's open → menu closes and the app does NOT launch.
     Click the power button again → toggles shut. Menu items still work.
     *This is the one to watch — the fix depends on iced's overlay/capture
     ordering, which couldn't be exercised without a live session.*
   - (b) Header: Library Settings gear and the COSMIC Settings toggle icon are
     now visually distinct; the latter still launches cosmic-settings.
   - (c) Folder reorder: drag one folder tile onto another → it moves to that
     position, order survives a reopen. Dragging a folder onto an app tile or
     a favorites gap does nothing (no folder created, no favorite inserted).
     Dragging an *app* onto a folder still adds it to the folder.
   - (d) Settings → "Open on": Home / Favorites / Auto each pick the right
     starting view on next open.
   **Phases 11–14 (still unverified):** favorites drag strips + drop-to-combine;
   settings page position/columns/rows live-apply + ESC; keyboard row nav after
   a column change; header show/hide togglers.
2. Watch PR #389; rebase if upstream moves.
3. BACKLOG: Phase 9 tight blur region (owner deferred — do this next), dock-pin
   sync, Home/in-folder app reorder, upstream picks (#378/#153/#386/#381).

## Gotchas (stable)
- PKGBUILD builds from GitHub `dev` — push before `makepkg`.
- Bare `just install` gets clobbered by pacman updates; the PKGBUILD replaces
  the official package. Rollback: `sudo pacman -S cosmic-app-library`.
- Keep `master` = upstream mirror; `feat/library-position` = PR #389 branch.
- cosmic-config: missing keys fall back via manual `impl Default`; AppFolder/
  folders/grid_columns/grid_rows use serde defaults. Old configs safe; old
  window_width/window_height key files are silently ignored now.
- Favorites = separate Vec; DISPLAY order = vec order; foldered favorites live
  in the folder (not the vec). filter_apps skips alphabetical sort only in
  favorites view with empty search.
- Drag-id bases: favorites tiles 1_000_000, Home tiles 2_000_000 (create
  folder), favorites reorder strips 3_000_000 (+2i left, +2i+1 right), folder
  tiles 4_000_000.
- Every dnd payload here is the same `AppletString` (text/uri-list) mime, so a
  drop can't tell from its data whether a folder or an app was dragged. That's
  what `dragging_folder: Option<usize>` is for — destination closures capture
  it at view-build time. If you add a drop destination, decide what it should
  do when a folder is being dragged (usually `Message::IgnoredDrop`).
- Don't make the power popover `modal(true)` again: libcosmic captures all
  mouse events for modal popups, which kills both click-outside-to-close and
  `on_close`. See PLAN Phase 15.
- An app lives in ≤1 folder; add_to_folder recomputes the target index if
  removing the app dissolves an earlier folder.
- cosmic-session auto-respawns the installed daemon if killed; single
  instance via DBus — a second launch just activates the first.
- Pre-existing cargo warnings (unused ListColumn/Column imports) are known;
  don't "fix" without checking upstream diff noise.
- No secrets in this project.
