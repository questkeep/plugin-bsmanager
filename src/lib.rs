//! Questkeep store plugin for BSManager (<https://github.com/Zagrios/bs-manager>): lists its Beat
//! Saber instances, one game per instance.
//!
//! The file reading lives in [`instances`] and the folder detection logic in [`detect`] (both
//! unit-tested natively); this file is the `Provider` that wires them to what the SDK offers.
//! The plugin finds BSManager by itself (`permissions.filesystem` and `permissions.registry` in
//! `plugin.json`): the instances folder through BSManager's own settings file, the program folder
//! through its uninstall entry. Paths the plugin reports are paths below `/host`, which Questkeep
//! translates to the real paths on this PC.
#![cfg_attr(test, allow(clippy::unwrap_used))]

pub mod detect;
pub mod instances;

use std::cell::RefCell;
use std::path::Path;

use questkeep_plugin_sdk::model::{ImportedGame, PlayAction};
use questkeep_plugin_sdk::{
    Provider, ProviderError, export_provider, host_path, known_folder, log, registry,
};

use instances::{EXECUTABLE, Instance};

/// BSManager's executable in its program folder (the `bsmanager://` handler runs it too).
const LAUNCHER_EXE: &str = "BSManager.exe";
const ICON: &str = include_str!("../store-icon.svg");
/// BSManager's launcher adds this for every instance below `BSInstances` (its `buildBsLaunchArgs`
/// does so when the version is neither the official Steam nor Oculus install).
const LAUNCH_ARGS: &str = "--no-yeet";
/// Shown when no instances folder was found.
const NOT_FOUND: &str = "BSManager's instances folder was not found. Questkeep looks for it through BSManager's own settings (the installation folder you chose in BSManager) and in the usual places. Start BSManager once, or check that its BSInstances folder exists.";
/// BSManager's settings folders below the user's roaming app data folder: `bs-manager` on a real
/// install (the package name); `BSManager` (the product name) is tried second.
const SETTINGS_FOLDERS: [&str; 2] = ["bs-manager", "BSManager"];

/// The game Questkeep gets for an instance. `root` is the instances folder as the plugin sees it
/// (`/host/...`); Questkeep translates the paths to the real ones. An instance folder name is a
/// single path part, so the result never contains `..` (which the host would not translate).
fn to_game(instance: &Instance, root: &str) -> ImportedGame {
    let dir = format!("{root}/{}", instance.folder);
    let mut game = ImportedGame::installed(instance.source_id(), instance.game_name());
    game.play_action = Some(PlayAction {
        args: Some(LAUNCH_ARGS.into()),
        working_dir: Some(dir.clone()),
        ..PlayAction::file(format!("{dir}/{EXECUTABLE}"))
    });
    game.install_dir = Some(dir);
    game
}

/// The action that opens BSManager: its executable below `app` (a path as the plugin sees it;
/// Questkeep translates it to the real one), or `None` when the file is not there. One
/// `is_file` check, so it is cheap enough for the menu.
fn launcher_action(app: &Path) -> Option<PlayAction> {
    app.join(LAUNCHER_EXE).is_file().then(|| PlayAction {
        working_dir: Some(app.display().to_string()),
        ..PlayAction::file(format!("{}/{LAUNCHER_EXE}", app.display()))
    })
}

/// A path as this PC writes it, as the plugin sees it (`None` when it has no view).
fn visible(native: &str) -> Option<String> {
    host_path(native)
}

/// BSManager's settings file, as text.
fn read_settings() -> Option<String> {
    let app_data = known_folder("app-data")?;
    SETTINGS_FOLDERS
        .iter()
        .find_map(|folder| std::fs::read_to_string(format!("{app_data}/{folder}/config.json")).ok())
}

/// The instances folder: the one BSManager's settings name, else BSManager's defaults. The first
/// that exists; its path as the plugin sees it.
fn find_instances_dir() -> Option<String> {
    let mut candidates: Vec<String> = Vec::new();
    if let Some(folder) = read_settings().and_then(|text| detect::installation_folder(&text)) {
        candidates.extend(visible(&detect::instances_dir(&folder)));
    }
    // `known_folder("home")` is already a path as the plugin sees it.
    if let Some(home) = known_folder("home") {
        candidates.extend(detect::default_instances_dirs(&home));
    }
    candidates.into_iter().find(|dir| Path::new(dir).is_dir())
}

/// BSManager's program folder: from its uninstall entry, else where its installer puts it. Its
/// path as the plugin sees it, only when `BSManager.exe` is there.
fn find_program_dir() -> Option<String> {
    let mut candidates: Vec<String> = Vec::new();
    match registry::uninstall_entries() {
        Ok(entries) => candidates.extend(detect::program_dir(&entries).and_then(|d| visible(&d))),
        Err(err) => log::warn(format!("could not read the Apps list: {err}")),
    }
    if let Some(local) = known_folder("local-app-data") {
        candidates.push(detect::default_program_dir(&local));
    }
    candidates
        .into_iter()
        .find(|dir| Path::new(dir).join(LAUNCHER_EXE).is_file())
}

#[derive(Default)]
struct BsManager {
    /// The program folder, found once (the Apps list is long): `configure`, which Questkeep calls
    /// before every scan, makes it look again.
    program_dir: RefCell<Option<Option<String>>>,
}

impl BsManager {
    fn program_dir(&self) -> Option<String> {
        self.program_dir
            .borrow_mut()
            .get_or_insert_with(find_program_dir)
            .clone()
    }

    fn scan(&self) -> Result<Vec<ImportedGame>, ProviderError> {
        let root = find_instances_dir().ok_or_else(|| ProviderError::NotFound(NOT_FOUND.into()))?;
        let scan = instances::scan_instances(Path::new(&root))
            .map_err(|e| ProviderError::Io(format!("could not read BSManager's instances: {e}")))?;
        for (name, reason) in &scan.skipped {
            log::warn(format!("skipped {name:?}: {reason}"));
        }
        for (name, note) in &scan.notes {
            log::warn(format!("instance {name:?}: {note}"));
        }
        let games: Vec<ImportedGame> = scan.instances.iter().map(|i| to_game(i, &root)).collect();
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

    /// Looks for BSManager again (it may have been installed or moved since the last sync).
    fn configure(&self, _config: &serde_json::Value) {
        *self.program_dir.borrow_mut() = None;
    }

    /// The title bar's "Open launcher" menu lists BSManager when its `BSManager.exe` is found. No
    /// `launcher_process`: closing BSManager after playing makes no sense.
    fn launcher(&self) -> Option<PlayAction> {
        launcher_action(Path::new(&self.program_dir()?))
    }

    /// BSManager counts as installed when its instances folder is found. Deliberately not
    /// `launcher().is_some()` (the trait's default): "installed" means the store is synced and
    /// offers add-game options, and both need the instances folder (a scan without it fails).
    /// With only the program found, the plugin shows in the launcher menu but is not synced.
    fn is_installed(&self) -> bool {
        find_instances_dir().is_some()
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

    const ROOT: &str = "/host/d/Games/BSManager/BSInstances";

    #[test]
    fn paths_are_paths_below_the_found_folder_and_carry_the_launch_arguments() {
        let game = to_game(&instance(), ROOT);
        assert_eq!(game.source_game_id, "abc");
        assert_eq!(game.name, "Beat Saber (1.34.2)");
        assert!(game.installed);
        let dir = format!("{ROOT}/1.34.2");
        assert_eq!(game.install_dir.as_deref(), Some(dir.as_str()));
        let action = game.play_action.unwrap();
        assert_eq!(action.target, format!("{dir}/Beat Saber.exe"));
        assert_eq!(action.args.as_deref(), Some("--no-yeet"));
        assert_eq!(action.working_dir.as_deref(), Some(dir.as_str()));
    }

    #[test]
    fn no_reported_path_has_a_parent_part() {
        let mut instance = instance();
        instance.folder = "Modded 1.29.1 (a..b)".into();
        let game = to_game(&instance, ROOT);
        let action = game.play_action.unwrap();
        for path in [&action.target, action.working_dir.as_ref().unwrap()] {
            assert!(path.split('/').all(|part| part != ".."), "{path}");
        }
    }

    #[test]
    fn without_a_folder_the_scan_says_where_it_looked() {
        // Native tests see no BSManager settings and no known folders.
        match BsManager::default().scan() {
            Err(ProviderError::NotFound(message)) => {
                assert!(message.contains("BSManager's own settings"), "{message}");
            }
            other => panic!("expected NotFound, got {other:?}"),
        }
        assert!(!BsManager::default().is_installed());
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("bsmanager-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_launcher_is_the_exe_when_it_exists() {
        let dir = temp_dir("with-exe");
        std::fs::write(dir.join(LAUNCHER_EXE), b"").unwrap();
        let action = launcher_action(&dir).unwrap();
        assert_eq!(action.kind, questkeep_plugin_sdk::model::ActionKind::File);
        assert_eq!(action.target, format!("{}/BSManager.exe", dir.display()));
        assert_eq!(action.working_dir, Some(dir.display().to_string()));
        assert!(action.args.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_launcher_without_the_exe_or_the_folder() {
        let dir = temp_dir("no-exe");
        assert!(launcher_action(&dir).is_none());
        // A folder named like the exe is not the program.
        std::fs::create_dir(dir.join(LAUNCHER_EXE)).unwrap();
        assert!(launcher_action(&dir).is_none());
        let _ = std::fs::remove_dir_all(&dir);
        assert!(launcher_action(&dir.join("missing")).is_none());
    }

    #[test]
    fn without_bsmanager_there_is_no_launcher_and_no_process() {
        let provider = BsManager::default();
        assert!(provider.launcher().is_none());
        assert!(provider.launcher_process().is_none());
        // The answer is remembered until `configure` asks again.
        assert!(provider.program_dir.borrow().is_some());
        provider.configure(&serde_json::Value::Null);
        assert!(provider.program_dir.borrow().is_none());
        assert!(!provider.is_installed());
    }

    #[test]
    fn describes_itself() {
        let provider = BsManager::default();
        assert_eq!(provider.id(), "bsmanager");
        assert!(provider.icon_svg().starts_with("<svg"));
        assert!(provider.icon_svg().contains("currentColor"));
        assert!(!provider.has_account());
    }
}
