# HANDOFF — cosmic-app-library fork

Resume line: *"Continue cosmic-app-library. Read PLAN.md and HANDOFF.md in
~/Projects/cosmic-app-library and do the next step."* Start with that dir as cwd.

## Current state (2026-07-19, session 4)
- Phases 0–14 CODE-COMPLETE on `dev`; release build passes. Phases 11–14 are
  NOT yet owner-verified live.
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
1. Owner: push is done; `just build-release && sudo just install`, then verify
   live: (a) favorites drag — strips reorder with accent bar, drop-on-tile
   creates a favorites folder with accent highlight, folder tile opens/renames/
   ungroups, auto-dissolve; (b) settings page — gear icon opens it, position
   dropdown moves the library live, columns/rows resize live, ESC backs out;
   (c) keyboard up/down row nav still lands on the right tiles after changing
   columns; (d) Home folders + power menu + blur unregressed; (e) header
   togglers hide/show Settings and Power buttons.
2. Watch PR #389; rebase if upstream moves.
3. BACKLOG: folder-tile reorder, Phase 9 tight blur region, dock-pin sync,
   upstream picks (#378/#153/#386/#381).

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
- An app lives in ≤1 folder; add_to_folder recomputes the target index if
  removing the app dissolves an earlier folder.
- cosmic-session auto-respawns the installed daemon if killed; single
  instance via DBus — a second launch just activates the first.
- Pre-existing cargo warnings (unused ListColumn/Column imports) are known;
  don't "fix" without checking upstream diff noise.
- No secrets in this project.
