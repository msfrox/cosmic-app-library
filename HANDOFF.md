# HANDOFF — cosmic-app-library fork

Resume line: *"Continue cosmic-app-library. Read PLAN.md and HANDOFF.md in
~/Projects/cosmic-app-library and do the next step."* Start with that dir as cwd.

## Current state (2026-09-22, Sapphire away session `cosmic-rebase-dev`)
- **`dev` merged current with `upstream/master`** (25 commits it was behind:
  xdgen-based localized desktop-entry/appstream generation replacing the old
  static per-language `.desktop`/`.metainfo.xml` blocks, `justfile` refactored
  to a `cargo.just` module, toolchain bumped `1.93.0` to `1.95.0`
  (`rust-toolchain.toml` now also pins `clippy`/`rustfmt` components), version
  `1.0.12` to `1.9.0`, a `LayerEvent::Focused` match-guard cleanup in `app.rs`,
  plus a pile of `i18n: translation update` commits). Merge commit `6aa1d4c`
  on this task's branch (`away/2026-09/cosmic-rebase-dev` — the runner
  branched it from `master`; reset to `origin/dev` first per the task
  preamble, then merged `upstream/master` in).
- **One real conflict, in `src/widgets/application.rs`**: upstream's "Align
  center for app names" (`d98a389`) added `.align_x(text::Alignment::Center)`
  to the app-tile name text. Our own `9834aed` ("ellipsize long names") had
  already superseded that — it uses `.center()`, which iced_core's
  `Text::center()` impl shows is literally `align_x(Center).align_y(Center)`,
  plus `Length::Fill`, word wrap and 2-line ellipsis, versus upstream's
  `Length::Shrink` + x-only centering. **Kept our side entirely** — a strict
  superset of upstream's change. Every other file (`Cargo.lock`/`.toml`,
  `app.rs`, the i18n `.ftl` files) auto-merged clean; spot-checked `app.rs`'s
  auto-merge (cosmetic match-guard refactor, no logic change) and the xdgen
  data-file changes (upstream's own localization rework, unrelated to
  anything on `dev`).
- **Verified post-merge, this container:** `cargo build` clean (no warnings,
  ~6.5 min cold — the 1.95.0 toolchain auto-installed via `rustup show`, same
  `sapphire-toolchain.md` recipe, nothing new needed), `cargo test` 20/20 pass
  (Phase 16 Home-reorder tests included), `cargo clippy --all-targets` clean,
  `cargo fmt --all -- --check` clean. **Still unverified live** — no display,
  no Wayland, needs RUBY2, same as every prior session.
- `dev` (this branch) is now 0 commits behind `upstream/master`, 26 ahead.
- **`feat/library-position` (PR #389) was NOT touched** — it's a separate
  branch off `master` for the upstream PR, not `dev`. The task's "PR #389
  does not rot further" is about keeping `dev` mergeable in general, not
  about rebasing that submitted PR branch. It's still ~25 commits behind
  `upstream/master`, same gap `dev` had before this merge — left for a
  future call, since rewriting a branch a maintainer may already be
  reviewing wasn't asked for here.

## Previous state (2026-09-21, Sapphire away session `cosmic-toolchain-prep`)
- **`cargo build`/`cargo test`/`cargo clippy` now run on Sapphire's
  `claude-code` container** (Debian 12, no cargo or Wayland headers going
  in) — exact package list and gotchas in
  `docs/away/sapphire-toolchain.md`, summarized in PLAN.md Phase 16. This
  was the bulk of the session; useful independent of the feature below.
- **Phase 16: Home tile reorder** (was BACKLOG). `AppLibraryConfig::home_order`
  + `app_group::apply_home_order`, `Message::ReorderHome` — extends the
  existing `move_within`/reorder-strip pattern Favorites and folders already
  used, rather than inventing a new one. Home's tile also keeps its existing
  combine-into-folder behaviour (now via the same 3-strip shape as
  Favorites/folders, which incidentally gives Home tiles the accent
  combine-hover hint too). 4 new unit tests (20 total, up from 16 — PLAN.md's
  Phase 15 note of "13" was already stale by the time this session started).
  `cargo clippy --all-targets` is fully clean (fixed the 2 pre-existing
  unused-import warnings plus 2 new findings on this toolchain — a redundant
  closure, two derivable `Default` impls — all mechanical).
- **Compile- and unit-test-verified only.** This container has no display —
  Phase 16's drag/drop UI, and everything below from session 5, is
  **unverified, needs RUBY2**.

## Previous state (2026-07-19, session 5)
- Phases 0–15 CODE-COMPLETE on `dev`; release build + `cargo test` (5) pass.
  Phases 11–15 are NOT yet owner-verified live — nothing has been installed
  from this session (sudo needs a password in the agent shell).
- Phase 15: power-menu close-on-outside-click regression fixed (libcosmic's
  modal popover now swallows every mouse event, so the ancestor mouse_area
  `on_release` never fired — switched to `modal(false)` + `on_close`, and app
  tiles go unclickable while the menu is open so the dismissing click can't
  launch anything); COSMIC Settings launcher now uses a bundled
  `cosmic-settings-toggle-symbolic` (ring + toggle) instead of the gear that
  was identical to Library Settings'; folder tiles drag-reorder; apps inside a
  folder drag-reorder via the same strips Favorites uses; `default_page`
  setting (Auto/Home/Favorites).
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
   - (e) In-folder reorder: open a folder, drag an app onto the gap beside
     another → accent bar shows, release reorders, order survives reopen.
     Dropping on a tile's *centre* inside a folder should do nothing (no
     nested folder). Favorites' drop-on-tile-to-combine must still work — that
     branch is now shared between the two views, so re-check it.
   - (d) Settings → "Open on": Home / Favorites / Auto each pick the right
     starting view on next open.
   **Phases 11–14 (still unverified):** favorites drag strips + drop-to-combine;
   settings page position/columns/rows live-apply + ESC; keyboard row nav after
   a column change; header show/hide togglers.
   **Phase 16 (new, unverified):** Home tile reorder — drag a Home tile onto
   the gap beside another → accent bar shows, release reorders, order
   survives reopen. Dropping on a tile's *centre* in Home should still
   combine into a new folder (unchanged behaviour, now via the shared strip
   code — re-check it didn't regress). A never-dragged app should still sit
   wherever alphabetical order puts it.
2. `dev` is caught up with `upstream/master` as of 2026-09-22 (see Current
   state above). Watch for upstream moving again; `feat/library-position`
   (PR #389) itself is still ~25 commits behind and untouched — owner call on
   whether/when to rebase a branch already up for review.
3. BACKLOG: Phase 9 tight blur region (owner deferred — do this next), dock-pin
   sync, upstream picks (#378/#153/#386/#381).

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
