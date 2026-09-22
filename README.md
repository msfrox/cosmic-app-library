# COSMIC App Library

COSMIC App Library is an application launcher for the COSMIC desktop that lists all installed applications in a grid.

## Build & Install

Run `just && sudo just install` to compile a local release build and install it.

Distributions should use `just vendor` during source tarball creation, `just build-vendored` in the build chroot, and `just rootdir=${DESTDIR} install` in their packaging. Use the included [debian packaging rules](./debian/rules) as an example for packaging.
