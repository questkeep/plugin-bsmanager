# Releasing the BSManager plugin

Releases are cut from a manual GitHub Actions run. Versions come from git tags and from
**changesets**: small markdown files that each pull request adds to `.changeset/`. This mirrors
the Questkeep app's process.

## Changesets

```md
---
bump: minor   # major | minor | patch
---
Show the instance's Beat Saber version in the game's name.
```

```sh
npm run changeset -- add <major|minor|patch> "<summary>"   # creates .changeset/<random>.md
npm run changeset -- status                                 # current version, pending files, next version
```

- Add one per user-visible change. Docs/CI-only changes need none. CI posts a warning (never a
  failure) on pull requests that change code without adding one.
- **Current version** = the highest `vX.Y.Z` git tag (no tag means `0.0.0`).
- **Next version** = current bumped by the highest pending bump. Pre-1.0 is plain semver: `major`
  on 0.x gives 1.0.0, `minor` gives 0.(x+1).0.
- No pending changesets means nothing to release; the workflow ends successfully with a notice.
- The repository has no tag yet, so the first release computes from `0.0.0`: the pending `minor`
  changeset ("First release") gives 0.1.0.

## Cutting a release

1. Make sure `main` is green and contains the changesets you want to ship.
2. GitHub, Actions, **Release**, **Run workflow** on `main`. Tick **dry_run** to compute the
   version, render the notes, build and pack without committing, tagging or publishing anything
   (the run summary shows the notes and the registry entry). A dry run may run from any branch; a
   real release only from `main`.
3. The workflow:
   1. `prepare` computes current/next version and the notes. No changesets: stop.
   2. `build` (Windows) writes the version into `Cargo.toml` (`[package]`), `Cargo.lock` and
      `plugin.json`, builds `provider.wasm`, packs `bsmanager-X.Y.Z.zip` (`plugin.json`,
      `provider.wasm`, `icon.svg`, `README.md`, `LICENSE` at the top level), and computes the
      sha256 and size. The notes and the job summary end with the registry `versions.json` entry.
   3. `release` (not on dry runs) fails if `main` moved during the build, then commits
      `chore(release): vX.Y.Z` (version files and deleted changesets) as `github-actions[bot]`,
      pushes it and the annotated tag `vX.Y.Z`, and runs `gh release create` with the notes and
      the zip.
4. Only one release runs at a time. If `release` fails after the tag was pushed, re-run the failed
   job: it detects the existing tag and only publishes the release (`--clobber` if it exists).

The release commit is pushed with `GITHUB_TOKEN`, so GitHub does not start CI on it. If `main` has
branch protection, allow `github-actions[bot]` to push.

## Listing the release in the plugin registry

The registry ([`questkeep/plugins`](https://github.com/questkeep/plugins), contract in the app
repository's `docs/PLUGIN_REGISTRY.md`) points at the release zip and pins its sha256.

- **By hand:** copy the JSON entry at the end of the release notes (or the run summary) and append
  it to `plugins/bsmanager/versions.json` in a pull request there. `minApp` is `0.7.0`
  (`MIN_APP` in `release.yml`); raise it when a release needs a newer Questkeep.
- **Automatically:** the organization's GitHub App (variable `PLUGIN_RELEASE_APP_ID`, secret
  `PLUGIN_RELEASE_APP_PRIVATE_KEY`, contents and pull requests write on `questkeep/plugins`) is used
  to open that pull request itself. The release workflow creates a short-lived token for it, pushes a
  branch `bsmanager-vX.Y.Z` and opens the PR. If the variable or secret is missing, or the step
  fails, the release still succeeds and says so in the log.

The listing itself (`registry/meta.json` and `icon.svg` of this repository) is copied into
`plugins/bsmanager/` by the same pull request on every release, so the first release creates the
registry entry and later ones keep it in sync. Edit `registry/meta.json` here, not in the registry.

## Without GitHub

`npm run build` (or `./build.ps1`) then `node scripts/pack.mjs` produces the same zip locally in
`dist/`.
