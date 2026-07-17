# HANDOFF — cosmic-app-library fork

Resume line: *"Continue cosmic-app-library. Read PLAN.md and HANDOFF.md in
~/Projects/cosmic-app-library and do the next step."* Start with that dir as cwd.

## Current state (2026-07-17)
- Public fork created: `origin` = github.com/msfrox/cosmic-app-library,
  `upstream` = pop-os. Cloned to `~/Projects/cosmic-app-library`.
- Branches: `master` (mirrors upstream @ ce33b9e), `dev` (integration, current).
- Planning docs written on `dev`, **not yet committed**.
- No code changes yet. No build has been run.

## Next step
Phase 0: `just build-release` on `dev` to confirm the unmodified fork compiles,
then `sudo just install` and verify it runs in place of the system app library.
(First build is long — hundreds of crates.)

## Gotchas (stable)
- `just install` overwrites `/usr/bin/cosmic-app-library` (the packaged binary).
  Rollback = `sudo pacman -S cosmic-app-library`. Pacman updates will clobber the
  fork until Phase 4's PKGBUILD.
- Keep `master` clean (upstream mirror). Phase 1 goes on `feat/library-position`
  off `master` for a tidy upstream PR; everything else on `dev`.
- No secrets in this project (desktop app) — no SECRETS-LOG needed.

## Not done yet
- No commits. First commit should land these docs on `dev` after Phase 0 build
  confirms the toolchain works.
