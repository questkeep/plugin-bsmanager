//! Questkeep store plugin for BSManager (<https://github.com/Zagrios/bs-manager>): lists its Beat
//! Saber instances, one game per instance.
//!
//! The file reading lives in [`instances`] and [`config`] (unit-tested natively); this file is
//! the `Provider` that wires them to the sandbox mounts declared in `plugin.json`:
//!
//! - `/instances`: BSManager's `BSInstances` folder (read-only)
//! - `/config`: BSManager's app-data folder, for `config.json` (read-only)
#![cfg_attr(test, allow(clippy::unwrap_used))]

pub mod config;
pub mod instances;

use std::path::Path;

use questkeep_plugin_sdk::model::{ImportedGame, PlayAction};
use questkeep_plugin_sdk::{Provider, ProviderError, export_provider, log};

use config::Config;
use instances::{EXECUTABLE, Instance};

/// `plugin.json` mounts BSManager's instances folder here.
const INSTANCES_MOUNT: &str = "/instances";
/// `plugin.json` mounts BSManager's app-data folder at `/config`.
const CONFIG_FILE: &str = "/config/config.json";
const ICON: &str = include_str!("../icon.svg");
/// BSManager's launcher adds this for every instance below `BSInstances` (its `buildBsLaunchArgs`
/// does so when the version is neither the official Steam nor Oculus install).
const LAUNCH_ARGS: &str = "--no-yeet";

/// Reads BSManager's `config.json` when there is one. Problems are logged, never fatal.
fn read_config() -> Option<Config> {
    let text = match std::fs::read_to_string(CONFIG_FILE) {
        Ok(text) => text,
        Err(e) => {
            log::debug(format!("no BSManager config.json ({e})"));
            return None;
        }
    };
    match Config::parse(&text) {
        Ok(config) => Some(config),
        Err(e) => {
            log::warn(format!("BSManager's config.json is not valid JSON: {e}"));
            None
        }
    }
}

/// The game Questkeep gets for an instance. `real_root` is the real Windows path of `BSInstances`
/// (the sandbox's mount path would not start anything), when known.
fn to_game(instance: &Instance, real_root: Option<&str>) -> ImportedGame {
    let mut game = ImportedGame::installed(instance.source_id(), instance.game_name());
    if let Some(root) = real_root {
        let dir = config::join(root, &instance.folder);
        game.play_action = Some(PlayAction {
            args: Some(LAUNCH_ARGS.into()),
            working_dir: Some(dir.clone()),
            ..PlayAction::file(config::join(&dir, EXECUTABLE))
        });
        game.install_dir = Some(dir);
    }
    game
}

#[derive(Default)]
struct BsManager;

impl BsManager {
    fn scan(&self) -> Result<Vec<ImportedGame>, ProviderError> {
        let config = read_config();
        let real_root = config.as_ref().and_then(Config::instances_dir);

        if !Path::new(INSTANCES_MOUNT).is_dir() {
            return match real_root {
                Some(root) => Err(ProviderError::NotConfigured(format!(
                    "BSManager keeps its instances in {root}, which this plugin may not read. Add that folder as a \"permissions.read\" entry named /instances in the plugin's plugin.json (see the plugin's README)."
                ))),
                None => {
                    log::info("BSManager's instances folder was not found");
                    Ok(Vec::new())
                }
            };
        }
        let scan = instances::scan_instances(Path::new(INSTANCES_MOUNT))
            .map_err(|e| ProviderError::Io(format!("could not read BSManager's instances: {e}")))?;
        for (name, reason) in &scan.skipped {
            log::warn(format!("skipped {name:?}: {reason}"));
        }
        for (name, note) in &scan.notes {
            log::warn(format!("instance {name:?}: {note}"));
        }
        if real_root.is_none() && !scan.instances.is_empty() {
            log::warn(
                "could not tell where BSManager's instances really are (no installation-folder or last launched instance in its config.json): the games are listed but cannot be started from Questkeep. Start one instance from BSManager once.",
            );
        }
        let games: Vec<ImportedGame> = scan
            .instances
            .iter()
            .map(|i| to_game(i, real_root.as_deref()))
            .collect();
        log::info(format!("found {} BSManager instances", games.len()));
        Ok(games)
    }
}

impl Provider for BsManager {
    fn id(&self) -> String {
        "bsmanager".into()
    }

    fn name(&self) -> String {
        "BSManager".into()
    }

    fn icon_svg(&self) -> String {
        ICON.into()
    }

    /// BSManager counts as installed when its instances folder or its settings file is visible.
    fn is_installed(&self) -> bool {
        Path::new(INSTANCES_MOUNT).is_dir() || Path::new(CONFIG_FILE).is_file()
    }

    fn scan_installed(&self) -> Result<Vec<ImportedGame>, ProviderError> {
        self.scan()
    }
}

export_provider!(BsManager);

#[cfg(test)]
mod tests {
    use super::*;

    fn instance() -> Instance {
        Instance {
            folder: "1.34.2".into(),
            metadata_id: Some("abc".into()),
            store: Some("steam".into()),
        }
    }

    #[test]
    fn a_known_root_gives_real_paths_and_the_launch_arguments() {
        let game = to_game(
            &instance(),
            Some(r"F:\Beat Saber stuff\BSManager\BSInstances"),
        );
        assert_eq!(game.source_game_id, "abc");
        assert_eq!(game.name, "Beat Saber (1.34.2)");
        assert!(game.installed);
        let dir = r"F:\Beat Saber stuff\BSManager\BSInstances\1.34.2";
        assert_eq!(game.install_dir.as_deref(), Some(dir));
        let action = game.play_action.unwrap();
        assert_eq!(action.target, format!(r"{dir}\Beat Saber.exe"));
        assert_eq!(action.args.as_deref(), Some("--no-yeet"));
        assert_eq!(action.working_dir.as_deref(), Some(dir));
    }

    #[test]
    fn an_unknown_root_lists_the_game_without_a_path() {
        let game = to_game(&instance(), None);
        assert!(game.install_dir.is_none());
        assert!(game.play_action.is_none());
    }

    #[test]
    fn describes_itself() {
        let provider = BsManager;
        assert_eq!(provider.id(), "bsmanager");
        assert!(provider.icon_svg().starts_with("<svg"));
        assert!(provider.icon_svg().contains("currentColor"));
    }
}
