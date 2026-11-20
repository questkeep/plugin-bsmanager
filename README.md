# BSManager plugin for Questkeep

A [Questkeep](https://github.com/questkeep/questkeep) store plugin that lists the Beat Saber
instances managed by [BSManager](https://github.com/Zagrios/bs-manager) as games, one game per
instance. It is written in Rust, compiled to WebAssembly (`wasm32-wasip2`) and runs in Questkeep's
plugin sandbox. It doubles as the reference example for community store plugins: parsing lives in
plain modules with fixture-based tests, the `Provider` is a thin layer on top.

## Requirements

- Windows (BSManager's only supported platform for this plugin) with BSManager installed.
- Questkeep 0.6.0 or newer (plugin API version 1).

## Install

- **From the plugin browser:** Questkeep, Settings, Plugins, search for "BSManager", Install.
- **From a zip:** download `bsmanager-<version>.zip` from the
  [releases](https://github.com/questkeep/plugin-bsmanager/releases), then Settings, Plugins,
  "Install from file...".

Then run a library sync. Each instance appears as `Beat Saber (<instance name>)`.

### If BSManager's data is not in your user folder

The sandbox can only read folders `plugin.json` declares. It declares BSManager's settings folder
(`%APPDATA%/bs-manager`) and the default instances folder (`~/BSManager/BSInstances`). If you moved
BSManager's installation folder (BSManager, Settings, Installation folder), the plugin cannot read
the new place until you declare it. The sync then fails with a message naming the folder. To fix it,
install the plugin from a folder (or unzip it), edit its `plugin.json` and change the path of the
`/instances` entry, for example:

```json
{ "path": "F:/Beat Saber stuff/BSManager/BSInstances", "as": "/instances" }
```

then install that folder with "Install from folder...". Plugins installed from a folder can be
reloaded after every edit.

## How it works

What the plugin does, and what it is based on. Facts are from BSManager's source code at
[Zagrios/bs-manager](https://github.com/Zagrios/bs-manager) (version 1.6.0 of `package.json`,
`master` branch) unless noted.

**Established from the source**

- BSManager keeps its instances in `<installation folder>/BSManager/BSInstances/<instance name>`
  (`InstallationLocationService`: `INSTALLATION_FOLDER = "BSManager"`, `VERSIONS_FOLDER = "BSInstances"`).
  The default installation folder on Windows is the user's home folder (`app.getPath("home")`), or
  `Documents` if a `Documents/BSManager` folder already exists from older versions. Users can move
  it; the chosen parent folder is stored as `installation-folder` in BSManager's settings.
- BSManager's settings are an `electron-store` file, `config.json`, in its app-data folder
  (`%APPDATA%/bs-manager`, which also exists on a real installation). Keys used here:
  `installation-folder` and `last-version-launched` (a version object with the full `path`).
- An instance is a folder containing a full Beat Saber install: `Beat Saber.exe` (`BS_EXECUTABLE`)
  and `Beat Saber_Data`. The instance's name is the folder name. BSManager keeps a
  `metadata.config` JSON file in it with an `id` (UUID) and `store` (`steam` or `oculus`).
- BSManager launches an instance by running `<instance>/Beat Saber.exe` with the instance folder
  as the working directory. For versions that are neither the official Steam nor the official
  Oculus install (every instance below `BSInstances`) it adds `--no-yeet`. When it launches through
  its Steam launcher it also sets the environment variables `SteamAppId`, `SteamOverlayGameId` and
  `SteamGameId` (`620980`); which launcher an instance uses is decided from its store (the source
  read for this plugin does not show that choice, so it is not verified for instances). Further arguments (`-vrmode oculus`, `fpfc`,
  `--verbose`, `editor`) come from the launch options the user picks in BSManager.
- BSManager also registers a `bsmanager://launch/?version=<version>&versionIno=<inode>` link (the
  shortcut feature in its wiki), but it needs the file system inode of the instance folder, which a
  plugin cannot read, so the plugin does not use it.

**What the plugin does with it**

- Every direct subfolder of the instances folder that has `Beat Saber.exe` and `Beat Saber_Data` is
  one game named `Beat Saber (<instance name>)`. Anything else (stray files, half-deleted folders)
  is skipped with a warning in Questkeep's log; a broken `metadata.config` is logged and the folder
  name is used as the id instead. The game's id is BSManager's UUID when there is one, which
  survives renaming the instance.
- The official Steam and Oculus installs that BSManager also lists are not reported: the Steam and
  Oculus stores of Questkeep cover them.
- Play action: `Beat Saber.exe` of the instance, with working directory the instance folder and
  the argument `--no-yeet`. No environment can be set from a play action, and no other launch
  options are passed.
- Real paths: Questkeep launches whatever path a plugin reports and does not translate sandbox
  mount paths, so the plugin needs the real Windows path of the instances folder. It takes it from
  `installation-folder` in BSManager's `config.json`, or else from the path of
  `last-version-launched`. If neither exists (default location, no instance ever started from
  BSManager), the games are listed without a play action and the log says to start one instance
  from BSManager once.

**What could not be verified**

- That launching `Beat Saber.exe` directly with `--no-yeet` (and without the Steam environment
  variables) behaves exactly like BSManager's own launch for instances copied from Steam. Nothing
  was run (no BSManager or game was started while building this). `store: "oculus"` instances
  normally need the Oculus app to be running.
- The default location (`~/BSManager`) and `%APPDATA%/bs-manager` were established from the source
  and one real `config.json`; the old `Documents/BSManager` location is not mounted.
- Instance versions are not shown: BSManager reads them from the binary `globalgamemanagers`
  file, which the plugin does not parse. The instance name is usually the version when the user
  did not rename it.

## Development

Needs Rust with the target `wasm32-wasip2` (`rustup target add wasm32-wasip2`) and Node 24 for the
scripts.

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test                      # parsing logic against tests/fixtures
npm test                        # release script and packaging tests
./build.ps1                     # or ./build.sh: builds provider.wasm next to plugin.json
node scripts/pack.mjs           # dist/bsmanager-<version>.zip, prints sha256 and size
```

To try it in Questkeep: Settings, Plugins, "Install from folder..." and pick this folder (after
building). Press Reload after every rebuild.

### Developing against a local Questkeep checkout

`Cargo.toml` depends on the SDK at a release tag:

```toml
questkeep-plugin-sdk = { git = "https://github.com/questkeep/questkeep", tag = "v0.6.0" }
```

To use your own checkout instead, add (to `Cargo.toml`, or a `.cargo/config.toml` that stays
uncommitted):

```toml
[patch."https://github.com/questkeep/questkeep"]
questkeep-plugin-sdk = { path = "../questkeep/crates/questkeep-plugin-sdk" }
```

### Layout

| Path | What |
|---|---|
| `src/lib.rs` | the `Provider`: sandbox mounts, logging, turning instances into games |
| `src/instances.rs` | reading instance folders and `metadata.config` (no SDK types) |
| `src/config.rs` | BSManager's `config.json`: where the instances really are |
| `tests/fixtures/` | made-up instance folders and settings files the tests read |
| `scripts/` | `pack.mjs` (zip), `manifest.mjs` (the host's `plugin.json` rules), `release/` (changesets) |

## Releasing

Every user-visible change adds a changeset (`npm run changeset -- add minor "summary"`). A manual
GitHub Actions run turns them into a version, a tagged commit and a GitHub release with the zip,
and prints the registry entry to paste. See [docs/RELEASING.md](docs/RELEASING.md).

## License

MIT, see [LICENSE](LICENSE).
