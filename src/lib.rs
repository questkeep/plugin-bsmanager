//! Questkeep store plugin for BSManager (<https://github.com/Zagrios/bs-manager>): lists its Beat
//! Saber instances, one game per instance.
//!
//! The file reading lives in [`instances`] (unit-tested natively); this file is the `Provider`
//! that wires it to the folder the user chooses (`permissions.folders` in `plugin.json`), which
//! Questkeep mounts read-only at `/instances`. Paths the plugin reports start with that mount
//! path; the host translates them to the real paths on this PC.
#![cfg_attr(test, allow(clippy::unwrap_used))]

pub mod instances;

use std::path::Path;

use questkeep_plugin_sdk::model::{ImportedGame, PlayAction};
use questkeep_plugin_sdk::{Provider, ProviderError, export_provider, log};

use instances::{EXECUTABLE, Instance};

/// `plugin.json` mounts the user's BSManager instances folder here.
const INSTANCES_MOUNT: &str = "/instances";
const ICON: &str = include_str!("../icon.svg");
/// BSManager's launcher adds this for every instance below `BSInstances` (its `buildBsLaunchArgs`
/// does so when the version is neither the official Steam nor Oculus install).
const LAUNCH_ARGS: &str = "--no-yeet";
/// Shown when nothing is mounted at `/instances`.
const NOT_CONFIGURED: &str = "BSManager's instances folder was not found. Choose the BSManager instances folder in Settings > Plugins > BSManager.";

/// The game Questkeep gets for an instance. Its paths are mount paths (`/instances/<name>`):
/// Questkeep translates them to the real ones. An instance folder name is a single path part, so
/// the result never contains `..` (which the host would not translate).
fn to_game(instance: &Instance) -> ImportedGame {
    let dir = format!("{INSTANCES_MOUNT}/{}", instance.folder);
    let mut game = ImportedGame::installed(instance.source_id(), instance.game_name());
    game.play_action = Some(PlayAction {
        args: Some(LAUNCH_ARGS.into()),
        working_dir: Some(dir.clone()),
        ..PlayAction::file(format!("{dir}/{EXECUTABLE}"))
    });
    game.install_dir = Some(dir);
    game
}

#[derive(Default)]
struct BsManager;

impl BsManager {
    fn scan(&self) -> Result<Vec<ImportedGame>, ProviderError> {
        let root = Path::new(INSTANCES_MOUNT);
        if !root.is_dir() {
            return Err(ProviderError::NotConfigured(NOT_CONFIGURED.into()));
        }
        let scan = instances::scan_instances(root)
            .map_err(|e| ProviderError::Io(format!("could not read BSManager's instances: {e}")))?;
        for (name, reason) in &scan.skipped {
            log::warn(format!("skipped {name:?}: {reason}"));
        }
        for (name, note) in &scan.notes {
            log::warn(format!("instance {name:?}: {note}"));
        }
        let games: Vec<ImportedGame> = scan.instances.iter().map(to_game).collect();
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

    /// BSManager counts as installed when its instances folder is mounted (chosen, or the default
    /// exists).
    fn is_installed(&self) -> bool {
        Path::new(INSTANCES_MOUNT).is_dir()
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
    fn paths_are_mount_paths_and_carry_the_launch_arguments() {
        let game = to_game(&instance());
        assert_eq!(game.source_game_id, "abc");
        assert_eq!(game.name, "Beat Saber (1.34.2)");
        assert!(game.installed);
        assert_eq!(game.install_dir.as_deref(), Some("/instances/1.34.2"));
        let action = game.play_action.unwrap();
        assert_eq!(action.target, "/instances/1.34.2/Beat Saber.exe");
        assert_eq!(action.args.as_deref(), Some("--no-yeet"));
        assert_eq!(action.working_dir.as_deref(), Some("/instances/1.34.2"));
    }

    #[test]
    fn no_reported_path_has_a_parent_part() {
        let mut instance = instance();
        instance.folder = "Modded 1.29.1 (a..b)".into();
        let game = to_game(&instance);
        let action = game.play_action.unwrap();
        for path in [&action.target, action.working_dir.as_ref().unwrap()] {
            assert!(path.split('/').all(|part| part != ".."), "{path}");
        }
    }

    #[test]
    fn without_the_mount_the_scan_names_the_setting() {
        // Native tests have no /instances mount.
        if Path::new(INSTANCES_MOUNT).exists() {
            return;
        }
        match BsManager.scan() {
            Err(ProviderError::NotConfigured(message)) => {
                assert!(
                    message.contains("Settings > Plugins > BSManager"),
                    "{message}"
                );
            }
            other => panic!("expected NotConfigured, got {other:?}"),
        }
        assert!(!BsManager.is_installed());
    }

    #[test]
    fn describes_itself() {
        let provider = BsManager;
        assert_eq!(provider.id(), "bsmanager");
        assert!(provider.icon_svg().starts_with("<svg"));
        assert!(provider.icon_svg().contains("currentColor"));
    }
}
