# COSMIC App Library — Fork Plan

Fork of [pop-os/cosmic-app-library](https://github.com/pop-os/cosmic-app-library)
adding: (1) top/bottom open position, (2) a header with a Settings button + inline
Power menu (Andromeda-launcher style), (3) a built-in **Favorites** group that opens
by default with right-click "Add to Favorites" everywhere.

Stack note: this is **Rust + libcosmic/iced**, not the web stack in the playbook.
Playbook parts that still apply: `~/Projects` location, GitHub fork = source of
truth, PLAN/HANDOFF context-transfer, phased checkboxes, Sonnet-agent delegation,
token-window pacing. Parts that don't: Cloudflare/Workers/D1/Hono deploy pipeline.

## Owner decisions (locked 2026-07-17)

- **Fork = public + upstream PR.** Phase 1 (position) written cleanly to submit to
  pop-os for issue #336. Personal features stay on a branch.
- **Position logic = auto-follow dock, with manual Top/Bottom override.** Default
  follows the COSMIC panel/dock; a config key can force Top or Bottom.
- **Power menu = reuse applet logic in-process.** Copy cosmic-applet-power's DBus
  session code in; render an inline menu. No second process.

## Branch model

- `master` — mirror of `upstream/master`. Never commit here except upstream merges.
- `dev` — personal integration branch. Holds these docs + ALL features. **This is
  what gets built/installed and run.**
- `feat/library-position` — branched from `master`, contains ONLY the Phase 1
  position feature, kept clean for the upstream PR. Merged into `dev` after.

Remotes: `origin` = msfrox/cosmic-app-library (fork), `upstream` = pop-os.

## Key code facts (verified against source, 2026-07-17)

- **Positioning** lives in `src/app.rs`:
  - `margin: f32` field (`:263`), set in `activate()` (`:409–419`) from the panel
    output geometry — currently the bottom edge of the top panel.
  - `layer_padding()` (`:454`): `top = margin+16`, `bottom = size.height-690-16-margin`.
    Content is a ~690px block pinned near the top. **Bottom-open = swap the top/bottom
    formulas** and flip the content column's vertical alignment.
  - Main surface uses `Anchor::all()` (`:393`); no position config exists today =
    issue [#336](https://github.com/pop-os/cosmic-app-library/issues/336).
- **Context menu already exists**: `Message::OpenContextMenu`/`CloseContextMenu`
  (`:512–513`), xdg popup positioner (`:1040`), `remove_entry` on menu action
  (`:1086`). Favorites reuses this — just add menu items + an action.
- **Groups**: `AppLibraryConfig` (`src/app_group.rs:169`) has `add(name)`,
  `remove(i)`, `add_entry(group, id)` (`:223`), `remove_entry(group, id)` (`:206`),
  `filtered(group,…)` (`:260`), and a special built-in `HOME` group (`:182`, and
  `filtered` treats `None` as HOME `:267`). Persisted via `cosmic_config`.
  Favorites = a second special group, selected on open.
- **Deps**: `zbus = "5.12"` is **already present** — reusing the power applet's
  DBus code adds no heavy new dependency. `tokio` has the `process` feature (for
  spawning `cosmic-settings`).
- **Install**: `just install` writes the binary to `/usr/bin/cosmic-app-library`
  (prefix `/usr`, APPID `com.system76.CosmicAppLibrary`) — same path as the
  packaged binary, so it overrides the system one. (Pacman will overwrite on the
  next `cosmic-app-library` package update — see DEPLOY.md.)

## Power actions reference (from cosmic-applet-power)

- log out / restart / shut down → session-manager DBus (`session_manager.rs`).
- lock / suspend → logind (`org.freedesktop.login1`) / cosmic-session
  (`cosmic_session.rs`). Copy both modules into `src/power/`.

---

## Phases

### Phase 0 — Fork & baseline build ✅ (build) / ⬜ (on-device verify)
- [x] Fork public → clone to `~/Projects/cosmic-app-library`, add `upstream`.
- [x] Branch model: `master` (upstream mirror) / `dev` (integration).
- [x] `just build-release` — unmodified fork compiles (1m22s, cache warm; 45MB bin).
- [ ] `sudo just install` + relaunch to confirm the fork runs in place of the
      system binary. Deferred: owner's live COSMIC session; owner to run when ready.
      Rollback: `sudo pacman -S cosmic-app-library`.

### Phase 1 — Top/bottom position (issue #336) ✅
Config key `Auto|Top|Bottom`, auto-follow dock, bottom-margin overlap fix.
PR #388 closed by owner; replaced by
https://github.com/pop-os/cosmic-app-library/pull/389 — same tree squashed to
ONE commit, no Claude trailer, AI-disclosure + DCO sign-off in the message,
full template checklist in the body (owner policy, see memory
upstream-pr-style). Verified on owner's desktop.

### Phase 2 — Header: Settings + Power ✅
Home header right of search: Settings icon (launches cosmic-settings via
activation token, dismisses library) + Power icon → inline popover menu
(lock/suspend/log out/restart/shut down). Power actions = applet's DBus code in
`src/power/`; confirmations via cosmic-osd with direct-DBus fallback.
New dep: logind-zbus; nix "user" feature.

### Phase 3 — Favorites ✅
`favorites: Vec<String>` on config (separate from groups so Home still shows
favorited apps), sentinel index `FAVORITES_GROUP = usize::MAX`, locked group
button next to Home (drag-to-add works), context-menu Add/Remove from
Favorites, opens on Favorites when non-empty else Home, search inside
Favorites searches all apps, empty-state hint.

### Phase 4 — Polish & packaging ✅ (code+docs) / ⬜ (owner: makepkg + verify)
- [x] `packaging/PKGBUILD` (`cosmic-app-library-msfrox`, provides/conflicts the
      official pkg, builds from GitHub `dev`); DEPLOY.md documents it.
- [x] Final HANDOFF.md + DEPLOY.md pass.
- [ ] Owner: on-device verify Phases 2+3, run `makepkg -si`, screenshots.
- [ ] Watch Phase 1 PR; rebase if upstream moves.

### Phase 5 — Power menu closes on outside click ✅
One-liner: `Message::CloseContextMenu` (fires via the window-wide `mouse_area`,
app.rs:1976) now also sets `power_menu_open = false`. Same flow the main window
uses; backdrop clicks already closed it via `hide()`.

### Phase 6 — Center position + configurable window size ✅
- `LibraryPosition` gets `Center` (manual-only; Auto still resolves Top/Bottom
  from dock). `layer_padding()` (app.rs:499–510): Center = symmetric
  `((size.height − H)/2)` top+bottom. View `positioned` match (app.rs:1977–87):
  Center = `space Fill` above AND below.
- Config keys `window_width`/`window_height` (f32, defaults 1200/690, clamped
  ≥ 600×400 and ≤ screen). Replace hardcoded 690/1200 at app.rs:499–508,
  1954–57, 1975 and anywhere else. No settings UI — config-file only for now.

### Phase 7 — Reorder favorites by drag ✅
- Order = `config.favorites` Vec order. Bug to fix first: `filtered()`'s
  FAVORITES branch (app_group.rs:322–28) returns global entry order — must sort
  by position in `self.favorites`.
- Drag-reorder: reuse existing dnd (`dnd_destination_for_data::<AppletString>`,
  app.rs:1850, as used for drag-to-group). In favorites view each tile is also
  a drop destination: dropping app id X on tile at index i ⇒ move X before i
  (`Message::ReorderFavorite`). Trailing drop zone appends to end.

### Phase 8 — Custom-named folders that keep apps in Home ✅
Groups already have custom names; what's missing is "favorites-style" =
non-exclusive. Add `keep_in_home: bool` (serde default false) to `AppGroup`;
create-group dialog gets a toggle. HOME's filter must exclude ONLY apps in
groups with `keep_in_home == false` (HOME.filtered branch, app_group.rs:321).
Old configs safe via serde default. Unlimited such folders = "multiple
favorites".

### Phase 9 — Frosted-blur bleed (upstream #387) ⚠️ REOPENED
bd929d9 (window-sized blur rect) killed the frost on-device → reverted to the
f32::MAX rect (dbd6a0f); bleed accepted for now. Live findings (2026-07-18):
compositor (cosmic-comp 1.3.0) DOES support partial regions end-to-end
(ext_background_effect_manager_v1 advertised, wl_region respected, renderer
clips); WAYLAND_DEBUG showed OUR BlurSurface(RESERVED) request never reaches
the wire — id likely remapped by the surface subsystem (frost actually comes
from libcosmic's automatic EnableBlur path, full-surface MAX rect, which also
explains the #387 bleed). Secondary bug: blur rect was computed from
self.size = 1920x1080 while the laptop output is 1920x1200 logical (~60px
offset; check the Size::new(1920,1080) fallback near app.rs:2257). Fix
candidates: use core.main_window_id() as the BlurSurface id + real output
size; verify with the kill-daemon/activate/screenshot loop.

### Phase 10 — Upstream cherry-picks ✅ (partial)
Done: PR #385 stable entry ids (46339fe). Batch: #338 accent highlight,
#306 clickable area, #235/335 long-name ellipsis (re-derived from draft PR
#360). Skipped: #378 hide/unhide (conflicts, revisit), #381 translations,
#179/66 stale. Upstream master fully synced as of 2026-07-18 — no rebase due.

### Phase 11 — Windows-11-style folders (drag icon onto icon) ✅ (code) / ⬜ (verify)
Implemented per spec below with two adaptations: Ungroup lives in the folder
view header (not a tile context menu); the favorites nested combine
destination was skipped (ApplicationButton's hand-rolled layout makes a
nested icon-area drop zone a restructure — favorites folders exist in
config/messages but aren't creatable by drop yet). ESC routes through a new
EscapePressed message so the folder view closes before the library hides.
Owner clarified Phase 8 intent: folders are TILES inside Home/Favorites (like
W11 Start), not bottom-bar groups. Phase 8's keep_in_home groups stay (harmless).
- Config (`app_group.rs`): `AppFolder { name, apps: Vec<String>, in_favorites: bool }`,
  `folders: Vec<AppFolder>` on AppLibraryConfig, `#[serde(default)]` both. An app
  lives in ≤1 folder (adding moves it). Home grid hides apps in home-folders
  (favorites-folder apps stay in Home — favorites never hide from Home). Adding a
  favorites app to a favorites-folder removes the id from the `favorites` vec.
  Folder auto-dissolves under 2 apps (survivor returns to host view).
- View: folder tile = same footprint as app tile; rounded "folder" container
  with 2×2 mini-icons of first 4 apps, ellipsized name below. Folder tiles
  prepend the grid of their host view (search empty only). Click → in-place
  folder view (same grid infra, entries = folder apps in order) with back
  button + name text_input (buffer, persist on submit/close); ESC = back, not
  hide. Right-click folder tile → menu: Ungroup. Inside folder view app
  context menu gains "Remove from folder".
- DnD: Home tiles become drop destinations (drop A on B = create folder
  [B, A], default name "Folder", drag_id base 2_000_000). Folder tile = drop
  destination (add to folder, base 4_000_000). Favorites tiles keep whole-tile
  reorder; nested inner destination on the icon area = combine (base
  3_000_000, EXPERIMENTAL — if nested destinations misbehave live, keep
  reorder only). FinishDrag/dnd source must no-op removal inside folder view.

### Phase 12 — Favorites positional drops: between = reorder, on top = combine ✅ (code) / ⬜ (verify)
Replaces the whole-tile-reorder-only favorites dnd. Each favorites app tile cell
becomes `row![left_zone, tile, right_zone]` — three SIBLING dnd destinations (no
nesting, no overlap, so none of the Phase 11 nested-destination risk):
- Zones: `Fixed(24)` wide × `Fixed(120)` tall. Left zone drop ⇒
  `ReorderFavorite(id, i)`; right zone ⇒ `ReorderFavorite(id, i+1)`; tile center
  drop ⇒ `CreateFolderFromDrop { target, dropped, in_favorites: true }` (config
  plumbing already handles favorites-vec removal + dissolve/rejoin).
- Drag ids: left `3_000_000 + 2i`, right `3_000_000 + 2i + 1` (base finally
  used); tile keeps `FAVORITE_TILE_DRAG_ID_BASE + i`.
- Hover hints via `fav_drop_hint: Option<(usize, FavDropZone)>` state set by
  `on_enter`/cleared by `on_leave` (+ FinishDrag/CancelDrag/Hide): active strip
  renders a 4px accent vertical insertion bar; active tile center renders the
  accent selected style (reuses ApplicationButton `selected`).
- The wrapping row keeps `FillPortion(1)` so the 7-column grid layout is
  unchanged; trailing append_zone stays. Folder tiles keep whole-tile
  add-to-folder (no reorder strips — folder order = folders vec order).

### Phase 13 — Grid size (rows × columns) + in-app settings page ✅ (code) / ⬜ (verify)
Owner picked settings-page rows/columns over drag-to-resize (layer-shell drag
resize is fragile; rows/cols is the unit the user thinks in). Config-file-only
sizing (Phase 6 keys) is replaced by derived sizing:
- Config: `grid_columns: u32` (default 7, clamp 4..=12), `grid_rows: u32`
  (default 3, clamp 2..=8), both `#[serde(default)]`-style fns. REMOVE
  `window_width`/`window_height` fields (stale per-field config keys are simply
  ignored; defaults reproduce today's exact 1200×690/444 sizes).
- Derived: `window_width() = cols·160 + 80`, `window_height() = rows·148 + 246`,
  `grid_max_height() = rows·148` (replaces both `max_height(444.0)` call
  sites); `chunks(7)` → `chunks(grid_columns)`. Screen-size clamps stay.
- Settings page: in-place view (like folder view) — `settings_view: bool`,
  opened by a new header icon button (emblem-system-symbolic, tooltip
  "Library Settings") next to the cosmic-settings launcher. Content: back
  button + title; Position dropdown (Auto/Top/Bottom/Center); Columns and
  Rows rows with −/value/+ buttons. Every change writes config AND pushes
  `set_padding(RESERVED, layer_padding())` so position/size apply live.
  ESC priority: settings → folder → hide.
- i18n (en): library-settings, position, position-auto/top/bottom/center,
  grid-columns, grid-rows.

### Phase 14 — Header-button visibility toggles ✅ (code) / ⬜ (verify)
`show_settings_button`/`show_power_button` config bools (serde default true),
togglers on the settings page, header row uses `push_maybe`. Hiding the power
button force-closes an open power menu; the library-settings gear is always
visible so the page stays reachable.

### Phase 15 — Power-menu regression, distinct icons, folder reorder, default page ✅ (code) / ⬜ (verify)
- **Power menu close-on-outside-click (regression).** Cause was upstream, not
  ours: libcosmic's `popover::update` now does `shell.capture_event()` for
  *every* mouse event while a **modal** popup is open, so the release could no
  longer bubble to the ancestor `mouse_area(window).on_release(ClosePowerMenu)`
  added in dbd6a0f — and `on_close` is in the `else` arm, so a modal popover
  never publishes it either. The menu became dismissable only via its own
  button. Fix: `modal(false)` + `on_close`, which publishes on any press
  outside the button's bounds; the dead `on_release` is removed. Because a
  non-modal popup no longer blocks the content behind it, app tiles take
  `on_press = None` while `power_menu_open`, so a dismissing click can't also
  launch the app underneath.
- **Header icons.** `preferences-system-symbolic` (COSMIC Settings launcher)
  and `emblem-system-symbolic` (Library Settings) are the *same gear glyph* —
  verified by rendering both. The launcher now uses a purpose-drawn
  `cosmic-settings-toggle-symbolic`: an outer ring around a filled toggle pill
  with a hollow knob, echoing COSMIC Settings' own app icon. No icon theme
  ships a symbolic toggle glyph, so it's bundled via `icon_cache`'s `bundle!`
  (compile-time `include_bytes!`, like the app-source icons) rather than
  installed — nothing to add to `data/icons/justfile`, and it can't go missing
  on a themed system. Being `-symbolic`-suffixed it recolours with the theme.
- **Folder reorder (was BACKLOG).** Folder tiles become `dnd_source`s as well
  as destinations; dropping folder A on folder B moves A to B's index.
  Everything on the wire is one `AppletString` mime, so the payload can't say
  "this is a folder" — `dragging_folder: Option<usize>` (set by `on_start`,
  cleared on finish/cancel/hide/drop) is read at view-build time by every
  destination closure instead. Folder drops on app tiles / favourites strips /
  the append zone resolve to `Message::IgnoredDrop`. `reorder_folder(from, to)`
  moves within the global `folders` vec; since a view's tiles are just that vec
  filtered by `in_favorites`, the other view's relative order is untouched
  (unit-tested).
- **In-folder app reorder (was BACKLOG).** The folder view now gets the same
  flanking reorder strips as Favorites — the whole `if favorites_view` tile
  branch became `if favorites_view || folder_view`, picking `ReorderFolderApp`
  vs `ReorderFavorite` per view. Inside a folder the tile centre stays a plain
  button (nothing to combine into, and a destination there would only steal
  drops from the strips); Favorites keeps its combine-on-tile. The duplicated
  insert-before index-shift is now one tested `app_group::move_within` helper
  used by both paths. `reorder_folder_app` refuses ids the folder doesn't
  already hold, so a stray drop can't smuggle an app in behind
  `add_to_folder`'s back.
- **Default opening page.** `DefaultPage { Auto, Home, Favorites }` config enum
  (serde default `Auto` = the old favorites-if-any behaviour), dropdown on the
  settings page above Columns/Rows. i18n: `default-page`, `default-page-auto`.
- First unit tests in the repo (`src/app_group.rs`): 13 covering
  `move_within`, `reorder_folder` / `reorder_folder_app` index-shifting and the
  `DefaultPage` default. `move_within_moves_forwards` pins the exact
  insert-before semantics the inline favourites code had, so the de-duplication
  is provably behaviour-preserving.
- NOT addressed (owner deferred): frosted-blur tight region.

## Delegation plan (per playbook §5)
- **Main thread:** Phase 1 positioning math, Phase 2 power/DBus integration,
  anything touching layer-shell/iced surfaces. Judgment-heavy on unfamiliar libcosmic.
- **Sonnet agent:** config-key plumbing, context-menu item wiring, PKGBUILD, once
  the pattern is set by the main thread. Tight specs + build-must-pass.
- **Local-GPU/CCR:** skip — libcosmic is too unfamiliar for the 14b model to be
  reliable here.
