# HANDOFF — cosmic-app-library fork

Resume line: *"Continue cosmic-app-library. Read PLAN.md and HANDOFF.md in
~/Projects/cosmic-app-library and do the next step."* Start with that dir as cwd.

## Current state (2026-07-18, session 3)
- Phases 0–11 CODE-COMPLETE on `dev`. Session-2 regressions all fixed and
  OWNER-VERIFIED live (dbd6a0f): favorites drag-reorder no longer deletes the
  app (FinishDrag no-ops in favorites view); power-menu items work again
  (close moved to a dedicated ClosePowerMenu on mouse RELEASE — closing on
  press destroyed the buttons before their release-fired action); frosted
  blur restored by reverting to the f32::MAX rect.
- Phase 9 blur findings (PLAN has detail): our BlurSurface(RESERVED) request
  never reaches the wire (surface-subsystem id remap — frost actually comes
  from libcosmic's automatic EnableBlur); self.size can stay stuck at the
  1920×1080 init default (Opened guard on RESERVED id) — positioning on the
  1200-tall laptop is ~60px off. No visible bleed on cosmic-comp 1.3.0, so
  left as-is; notes are for a future tight-region/#387 attempt.
- Phase 11 W11-style folders implemented (Sonnet agent + index-shift fix in
  add_to_folder): AppFolder{name,apps,in_favorites} + folders vec (serde
  defaults, old configs safe); drop app-on-app in Home creates "Folder";
  folder tile = 2×2 mini icons, drop-on-tile adds; click opens in-place
  folder view (back + rename input + Ungroup button); context menu gains
  "Remove from folder"; auto-dissolve under 2 apps; ESC closes folder view
  first (new EscapePressed). Favorites folders exist in config but are NOT
  creatable by drop yet (nested destination skipped — see PLAN Phase 11).
- Upstream PR #388 CLOSED by owner → replaced by #389 (squashed single
  commit, owner-authored, AI-disclosure + DCO, full template body). Owner
  policy memorized: no Claude trailer on upstream PRs.

## Next step
1. Owner: `just build-release && sudo just install`, verify folders live:
   create (drag icon onto icon in Home), open/rename/ungroup, drop-to-add,
   remove-from-folder menu item, auto-dissolve, ESC behavior, and that
   favorites reorder still works alongside.
2. Watch PR #389; rebase if upstream moves.
3. BACKLOG: favorites drop-to-combine (needs ApplicationButton restructure or
   position-aware on_motion drops); folder tiles in favorites ordering;
   Phase 9 tight blur region (use core.main_window_id() + real output size);
   upstream #378 hide/unhide adapt; #153 fractional scaling; #386 XDG dedupe.

## Gotchas (stable)
- PKGBUILD builds from GitHub `dev` — push before `makepkg`.
- Bare `just install` gets clobbered by pacman updates; the PKGBUILD replaces
  the official package. Rollback: `sudo pacman -S cosmic-app-library`.
- Keep `master` = upstream mirror; `feat/library-position` = PR #389 branch
  (force-pushed rewrite; old #388 commits unreachable).
- cosmic-config: missing keys fall back via manual `impl Default for
  AppLibraryConfig`; AppGroup.keep_in_home and AppFolder/folders use
  `#[serde(default)]`. Old configs safe either way.
- Favorites = separate Vec; DISPLAY order = vec order; filter_apps skips
  alphabetical sort only in favorites view with empty search. Folder-view
  entries likewise keep folder.apps order.
- Drag-id bases: favorites tiles 1_000_000, Home tiles 2_000_000 (create
  folder), 3_000_000 reserved (unused, skipped combine), folder tiles
  4_000_000.
- An app lives in ≤1 folder; add_to_folder recomputes the target index if
  removing the app dissolves an earlier folder (index shift).
- cosmic-session auto-respawns the installed daemon if killed; single
  instance via DBus — a second launch just activates the first.
- No secrets in this project.
