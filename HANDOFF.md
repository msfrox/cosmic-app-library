# HANDOFF — cosmic-app-library fork

Resume line: *"Continue cosmic-app-library. Read PLAN.md and HANDOFF.md in
~/Projects/cosmic-app-library and do the next step."* Start with that dir as cwd.

## Current state (2026-07-17)
- Public fork `origin` = github.com/msfrox/cosmic-app-library, `upstream` = pop-os.
- Branches: `master` (upstream mirror @ ce33b9e), `dev` (integration, current),
  `feat/library-position` (Phase 1, merged into dev).
- **Phase 0 done:** unmodified fork builds (`just build-release`, ~1m22s, 45MB bin).
- **Phase 1 code done & committed on `dev`:** top/bottom open position.
  - `LibraryPosition { Auto, Top, Bottom }` added to `AppLibraryConfig`
    (src/app_group.rs). Set via config file
    `~/.config/cosmic/com.system76.CosmicAppLibrary/v1/position` (plain text
    `Top` / `Bottom` / `Auto`). No in-app UI toggle yet.
  - src/app.rs: `bottom_margin` field, `effective_position()`, `handle_overlap()`
    bottom detection, `layer_padding()` + view spacer branch on position.
- Nothing pushed to origin yet. Not installed to the system (owner's live session).

## Next step
1. On-device verify (owner runs — needs sudo + their live desktop):
   `sudo just install`, then trigger the app library. Test `Bottom` override:
   `mkdir -p ~/.config/cosmic/com.system76.CosmicAppLibrary/v1 && \
    printf Bottom > ~/.config/cosmic/com.system76.CosmicAppLibrary/v1/position`
   (delete the file or write `Auto` to revert). Rollback binary:
   `sudo pacman -S cosmic-app-library`.
2. Then Phase 2 (header: Settings + Power) on `dev`.
3. Open the Phase 1 upstream PR (push `feat/library-position`, PR to pop-os, ref #336).

## Gotchas (stable)
- `just install` overwrites `/usr/bin/cosmic-app-library`; pacman updates clobber
  it until Phase 4's PKGBUILD. Rollback = `sudo pacman -S cosmic-app-library`.
- Keep `master` clean (upstream mirror). Phase 1 = `feat/library-position` off
  master for a tidy PR; other features on `dev`.
- cosmic-config unit enums store as plain text files (verified against existing
  configs). Missing `position` key → defaults to `Auto` (safe for old configs).
- Default COSMIC (top panel + bottom dock) → `Auto` resolves to **Top** (no
  regression). Bottom needs the explicit override.
- No secrets in this project.
