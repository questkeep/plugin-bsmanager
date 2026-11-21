# AGENTS.md

Guide for humans and coding agents working on the Questkeep BSManager plugin.

## Commands

| Task | Command |
|---|---|
| Format check / clippy / tests | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` |
| Build `provider.wasm` (needs the `wasm32-wasip2` target, set up by `mise install`) | `./build.ps1` or `./build.sh` |
| Script tests (release tooling, zip, manifest rules) | `npm test` |
| Pack `dist/bsmanager-<version>.zip` (prints sha256, size) | `node scripts/pack.mjs` |
| Add a changeset (every user-visible PR) | `npm run changeset -- add <major\|minor\|patch> "summary"` |
| Version, pending changesets, next version | `npm run changeset -- status` |

CI (`.github/workflows/ci.yml`) and the release build run on `ubuntu-latest` (the plugin only returns
mount paths and builds to wasm32-wasip2, so Windows is not needed there). Locally on Windows run
cargo from PowerShell or cmd, not Git Bash (its `link` shadows MSVC's `link.exe`);
`build.ps1` is the Windows convenience script next to `build.sh`. Never invoke `python`.

## Layout

- `src/lib.rs`: the `Provider`. `src/instances.rs`: parsing, no SDK types, tested against
  `tests/fixtures/`.
- `src/detect.rs`: finding BSManager (its `config.json`, its uninstall entry, the defaults); pure
  logic over strings and the SDK's `UninstallEntry`, tested natively.
- `plugin.json`: manifest and sandbox permissions (`filesystem: "read"` and `registry: "read"`: the
  plugin finds the instances and program folders itself, see `detect.rs`; no web access). `icon.svg`: BSManager's logo (registry listing and zip; see
  `THIRD_PARTY_NOTICES.md`); `store-icon.svg`: the one-colour (`currentColor`) version compiled into the store.
- `scripts/`: `pack.mjs`, `manifest.mjs` (the host's `plugin.json` rules), `zip.mjs`,
  `release/` (changesets, registry entry). `docs/RELEASING.md` describes the release.

## Conventions

- **Toolchain pins:** `mise.toml` pins exact versions; `.github/workflows/update-toolchain.yml` (calls
  `questkeep/workflows`) lists every pinned tool in its `tools` input with a version label (`lts`,
  `latest` or a prefix). **When you add or remove a tool in `mise.toml`, update that list in the same
  change**; the workflow fails when a pinned tool is missing from it.
- No `unwrap()`/`expect()` outside tests. A broken or odd instance folder is skipped with
  `log::warn`; it never aborts the scan.
- Paths reported for games are paths below `/host` (`/host/d/Games/.../<name>`): Questkeep
  translates paths below the file system mount to real ones. Never build a path with a `..` part
  (it is not translated). The plugin needs Questkeep 0.7.0 (`MIN_APP` in `release.yml`).
- Do not invent facts about BSManager: cite its source in the README's "How it works" or choose the
  conservative behavior and say so there.
- Conventional Commits. Every user-visible change adds a changeset; docs/CI-only changes do not.
- Never commit `provider.wasm`, `dist/` or `target/`. The SDK is pinned to a Questkeep release tag.
