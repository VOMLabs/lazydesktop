# Release Process

LazyDesktop releases are **tag-driven**: pushing a `v*` tag triggers the
GitHub Actions release workflow, which builds every platform artifact and
publishes a GitHub Release.

## Versioning

- The project uses **semantic versioning** with a `v` prefix: `v0.1.0`,
  `v0.2.0`, …
- The current application version is defined in `xmake.lua`
  (`set_version`), `PKGBUILD` (`pkgver`), and the crate manifests.
- Pre-releases are published as **prereleases** on GitHub (the workflow sets
  `prerelease: true`).

## Creating a release

1. Make sure `main` is green on CI (`.github/workflows/ci.yml`).
2. Check the [roadmap](../ROADMAP.md) to confirm the milestone's items.
3. Tag and push:

   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```

4. The release workflow (`.github/workflows/release.yml`) builds:

   | Job | Artifacts |
   |-----|-----------|
   | `release-linux` | `.deb` + `.AppImage` (ubuntu-24.04) |
   | `release-arch` | `.pkg.tar.zst` (Arch container) |
   | `release-windows` | `.msi` (windows-2022, WiX) |

5. When the Linux job finishes, `create-release` downloads all artifacts,
   generates a `checksums.txt`, and publishes the GitHub Release with
   auto-generated release notes.

## What gets published

- `lazydesktop_<version>_amd64.deb`
- `lazydesktop-<version>-x86_64.AppImage`
- `lazydesktop-<version>-1-x86_64.pkg.tar.zst`
- `lazydesktop-<version>.msi`
- `checksums.txt`

## Local release builds

You can also build artifacts locally with the justfile — see
[packaging](development/packaging.md):

```bash
just release          # all artifacts
```

## Notes / known issues

- The release workflow still uses the **Meson** build system for packaging,
  while development builds use XMake. A `meson.build` is not currently in the
  tree; migrating packaging to XMake is tracked on the
  [roadmap](../ROADMAP.md).
- Releases are published as prereleases until the project reaches a stable
  1.0.
