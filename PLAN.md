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

### Phase 1 — Top/bottom position (issue #336) ✅ (code) / ⬜ (verify + PR)
Branch `feat/library-position` off `master`; merged into `dev`.
- [x] `LibraryPosition` config key `Auto | Top | Bottom` (default `Auto`) on
      `AppLibraryConfig`. Set via `~/.config/cosmic/com.system76.CosmicAppLibrary/v1/position`
      (plain text: `Top`/`Bottom`/`Auto`).
- [x] `Auto`: bottom only when a bottom dock exists AND no top panel (else top),
      so default top-panel layouts are unchanged.
- [x] `handle_overlap()` computes `bottom_margin`; `layer_padding()` + view spacer
      branch on `effective_position()`.
- [x] Compiles clean (only pre-existing warnings).
- [ ] On-device verify: top panel + bottom override; and a bottom-dock-only setup.
- [ ] Open PR to pop-os referencing #336.
- **Scope contract:** position only. Minimal, upstream-friendly diff. ✅ held.

### Phase 2 — Header: Settings + Power ⬜
On `dev`.
- [ ] Add a header row to the right of the 96px search bar (top_row) with a
      Settings icon and a Power icon.
- [ ] Settings icon → spawn `cosmic-settings` (tokio process) and dismiss library.
- [ ] Copy `session_manager.rs` + `cosmic_session.rs` into `src/power/`; expose an
      inline power menu (lock, suspend, restart, shut down, log out) with the same
      confirmation behavior as the applet.
- [ ] Match spacing/iconography to the COSMIC theme (not pixel-copy Andromeda).
- **Scope contract:** header + power menu. Favorites is Phase 3.

### Phase 3 — Favorites ⬜
On `dev`.
- [ ] Add a special built-in `Favorites` group alongside `HOME`.
- [ ] Select Favorites by default on open (adjust `activate()` `cur_group`).
- [ ] Add context-menu items "Add to Favorites" / "Remove from Favorites" on app
      icons in every view (reuse existing OpenContextMenu; call add/remove_entry).
- [ ] Persist via existing config; handle empty-favorites empty state.
- **Scope contract:** favorites group + context action + default view.

### Phase 4 — Polish & packaging ⬜
- [ ] PKGBUILD for CachyOS/Arch (`cosmic-app-library-msfrox`) so updates don't get
      clobbered by the official package; document pin/override in DEPLOY.md.
- [ ] Final HANDOFF.md + DEPLOY.md pass; screenshots.
- [ ] Confirm Phase 1 PR status; rebase if upstream moved.

## Delegation plan (per playbook §5)
- **Main thread:** Phase 1 positioning math, Phase 2 power/DBus integration,
  anything touching layer-shell/iced surfaces. Judgment-heavy on unfamiliar libcosmic.
- **Sonnet agent:** config-key plumbing, context-menu item wiring, PKGBUILD, once
  the pattern is set by the main thread. Tight specs + build-must-pass.
- **Local-GPU/CCR:** skip — libcosmic is too unfamiliar for the 14b model to be
  reliable here.
