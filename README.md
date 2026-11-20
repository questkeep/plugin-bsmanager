# BSManager plugin for Questkeep

A [Questkeep](https://github.com/questkeep/questkeep) store plugin that lists the Beat Saber
instances managed by [BSManager](https://github.com/Zagrios/bs-manager) as games, one game per
instance. It is written in Rust, compiled to WebAssembly (`wasm32-wasip2`) and runs in Questkeep's
plugin sandbox. It doubles as the reference example for community store plugins: parsing lives in
plain modules with fixture-based tests, the `Provider` is a thin layer on top.

## Requirements

- Windows (BSManager's only supported platform for this plugin) with BSManager installed.
- Questkeep 0.7.0 or newer (plugin API version 1, and the user-chosen folders this plugin uses; an
  older Questkeep would ignore the folder setting and find nothing).

## Install

- **From the plugin browser:** Questkeep, Settings, Plugins, search for "BSManager", Install.
- **From a zip:** download `bsmanager-<version>.zip` from the
  [releases](https://github.com/questkeep/plugin-bsmanager/releases), then Settings, Plugins,
  "Install from file...".

Then run a library sync. If your instances are not in the default place (see below), choose the
folder first. Each instance appears as `Beat Saber (<instance name>)`.

### Choosing the instances folder

The plugin reads two folders (the second one is described under "Open launcher"). The instances folder is the one that contains your Beat Saber instances: BSManager's
`BSInstances` folder. Until you choose another, Questkeep uses the default, `~/BSManager/BSInstances`
(`C:Users<you>BSManagerBSInstances`), if it exists. If you moved BSManager's installation folder
(BSManager, Settings, Installation folder), or the default does not exist, go to Questkeep, Settings,
Plugins, BSManager, Folders, **Choose...** and pick the `BSInstances` folder (it is the
`BSManagerBSInstances` folder inside the installation folder). **Use default** goes back to the
default. Changing the folder syncs the plugin again. Without a usable folder the sync fails with
"Choose the BSManager instances folder in Settings > Plugins > BSManager"; an existing folder
without instances simply lists no games.

### Open launcher

BSManager also appears in the title bar's **Open launcher** menu (the rocket button) and starts
`BSManager.exe`. For that the plugin needs BSManager's program folder, the second folder setting
(Settings, Plugins, BSManager, Folders, **BSManager program folder**). The default is
`%LOCALAPPDATA%/Programs/bs-manager` (where BSManager's installer puts it for the current user); if
you installed it elsewhere, choose the folder that contains `BSManager.exe`. The menu entry only
appears when that file exists. This folder is mounted read-only and only used to find and start
`BSManager.exe`; nothing in it is read or changed. Questkeep does not close BSManager after playing.

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
- An instance is a folder containing a full Beat Saber install: `Beat Saber.exe` (`BS_EXECUTABLE`)
  and `Beat Saber_Data`. The instance's name is the folder name. BSManager keeps a
  `metadata.config` JSON file in it with an `id` (UUID) and `store` (`steam` or `oculus`).
- BSManager launches an instance by running `<instance>/Beat Saber.exe` with the instance folder
  as the working directory. For versions that are neither the official Steam nor the official
  Oculus install (every instance below `BSInstances`) it adds `--no-yeet`. When it launches through
  its Steam launcher it also sets the environment variables `SteamAppId`, `SteamOverlayGameId` and
  `SteamGameId` (`620980`) only for Steam-store instances; the launcher is chosen from the
  version's Steam or Oculus flag, then from the `store` in `metadata.config`, with Steam as the
  fallback. For an Oculus-store instance it sets none of those variables (like this plugin) but
  first tries to start the Oculus app, which a plugin cannot do: start the Oculus app yourself
  before launching from Questkeep. Further arguments (`-vrmode oculus`, `fpfc`, `--verbose`,
  `editor`) come from the launch options the user picks in BSManager; the plugin passes none.
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
- Paths: the plugin only sees the chosen folders, which Questkeep mounts read-only at
  `/instances` and `/app` in the sandbox. It reports `/instances/<instance name>` and
  `/instances/<instance name>/Beat Saber.exe` (and `/app/BSManager.exe` for the launcher); Questkeep translates such paths below a mount to the
  real ones on your PC when it launches the game or opens the folder. BSManager's own `config.json`
  is not read: the folder you choose replaces guessing the location from it.

**What could not be verified**

- That launching `Beat Saber.exe` directly with `--no-yeet` (and without the Steam environment
  variables) behaves exactly like BSManager's own launch for instances copied from Steam. Nothing
  was run (no BSManager or game was started while building this). `store: "oculus"` instances
  normally need the Oculus app to be running.
- That `BSManager.exe` started directly behaves like BSManager's own shortcut (its `bsmanager://` handler runs the same exe); the launcher was checked against Questkeep's plugin host without starting anything. The default program folder is the NSIS per-user default (`%LOCALAPPDATA%/Programs/bs-manager`), seen on one PC, not taken from BSManager's source.
- The default location (`~/BSManager`) was established from the source; the old
  `Documents/BSManager` location is not the default (choose it as the folder if you still use it).
- The path translation was checked by loading the built `provider.wasm` in Questkeep's plugin host
  against a copy of the test fixtures (not by running the app).
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
| `src/lib.rs` | the `Provider`: the `/instances` and `/app` mounts, `launcher()`, logging, turning instances into games |
| `src/instances.rs` | reading instance folders and `metadata.config` (no SDK types) |
| `tests/fixtures/` | made-up instance folders the tests read |
| `scripts/` | `pack.mjs` (zip), `manifest.mjs` (the host's `plugin.json` rules), `release/` (changesets) |

## Releasing

Every user-visible change adds a changeset (`npm run changeset -- add minor "summary"`). A manual
GitHub Actions run turns them into a version, a tagged commit and a GitHub release with the zip,
and prints the registry entry to paste. See [docs/RELEASING.md](docs/RELEASING.md).

## License

MIT, see [LICENSE](LICENSE).

## Credits

The icon is the BSManager logo from <https://github.com/Zagrios/bs-manager> (GPL-3.0), used to identify the program this plugin works with; see `THIRD_PARTY_NOTICES.md`. This plugin is not affiliated with BSManager or its authors.
