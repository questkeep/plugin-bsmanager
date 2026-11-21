//! Finding BSManager's folders on this PC. Pure logic over strings and the SDK's registry records
//! (unit-tested natively); `lib.rs` feeds it what it reads through the SDK.
//!
//! What this relies on (see the README, "How it works"):
//!
//! - BSManager keeps the folder its data lives in as `installation-folder` in `config.json` in
//!   its Electron `userData` folder (`%APPDATA%\bs-manager`, seen on a real install; the source
//!   builds the store with `new ElectronStore()`). The value is the *parent* of `BSManager`
//!   (`<value>\BSManager\BSInstances\<instance>`).
//! - Its installer (electron-builder NSIS, per user) registers an uninstall entry named
//!   `BSManager <version>` from publisher `Zagrios` whose `DisplayIcon` and `UninstallString` name
//!   the program (`...\BSManager.exe,0`); `InstallLocation` is empty.

use questkeep_plugin_sdk::registry::UninstallEntry;

/// The key under which BSManager stores its installation folder.
pub const INSTALLATION_FOLDER_KEY: &str = "installation-folder";
/// BSManager's own folder inside the installation folder.
const DATA_FOLDER: &str = "BSManager";
/// The folder inside it that holds the instances.
const INSTANCES_FOLDER: &str = "BSInstances";

/// `installation-folder` from BSManager's `config.json`; `None` when the file is not JSON, has no
/// such key, or the value is not a non-blank string.
pub fn installation_folder(config_json: &str) -> Option<String> {
    let value: serde_json::Value =
        serde_json::from_str(config_json.trim_start_matches('\u{feff}')).ok()?;
    let folder = value.get(INSTALLATION_FOLDER_KEY)?.as_str()?.trim();
    (!folder.is_empty()).then(|| folder.to_string())
}

/// Whether the path looks like a Windows path (drive letter or backslashes): joined with `\`.
fn windows_style(path: &str) -> bool {
    path.contains('\\') || path.as_bytes().get(1) == Some(&b':')
}

/// `base` with `parts` below it, using the separator the path already uses.
pub fn join(base: &str, parts: &[&str]) -> String {
    let separator = if windows_style(base) { '\\' } else { '/' };
    let mut path = base.trim_end_matches(['\\', '/']).to_string();
    for part in parts {
        path.push(separator);
        path.push_str(part);
    }
    path
}

/// The instances folder for an installation folder as BSManager stores it. A value that already
/// ends in `BSManager` (BSManager strips that part when it stores the folder, so it should not
/// happen) is accepted too.
pub fn instances_dir(installation_folder: &str) -> String {
    let trimmed = installation_folder.trim_end_matches(['\\', '/']);
    let last = trimmed.rsplit(['\\', '/']).next().unwrap_or("");
    if last.eq_ignore_ascii_case(DATA_FOLDER) {
        join(trimmed, &[INSTANCES_FOLDER])
    } else {
        join(trimmed, &[DATA_FOLDER, INSTANCES_FOLDER])
    }
}

/// BSManager's default instances folders below the user's home folder, in the order BSManager
/// prefers them: `Documents\BSManager` (older versions; used when it exists) before
/// `BSManager` directly in the home folder.
pub fn default_instances_dirs(home: &str) -> [String; 2] {
    [
        join(home, &["Documents", DATA_FOLDER, INSTANCES_FOLDER]),
        join(home, &[DATA_FOLDER, INSTANCES_FOLDER]),
    ]
}

/// Whether an uninstall entry is BSManager's (`BSManager <version>` from Zagrios).
fn is_bsmanager(entry: &UninstallEntry) -> bool {
    let named = entry
        .display_name
        .as_deref()
        .is_some_and(|name| name.trim().to_ascii_lowercase().starts_with("bsmanager"));
    let publisher_fits = entry
        .publisher
        .as_deref()
        .is_none_or(|publisher| publisher.to_ascii_lowercase().contains("zagrios"));
    named && publisher_fits
}

/// The file a registry command or icon value names: `"C:\x\y.exe" /a`, `C:\x\y.exe,0` and
/// `C:\x\y.exe` all give `C:\x\y.exe`. `None` for values with `%VARIABLES%` (not expanded here).
fn program_file(value: &str) -> Option<String> {
    let value = value.trim();
    let file = if let Some(quoted) = value.strip_prefix('"') {
        quoted.split('"').next()?
    } else {
        let lower = value.to_ascii_lowercase();
        let end = lower
            .find(".exe")
            .map_or(value.len(), |at| at + ".exe".len());
        &value[..end]
    };
    // `file.exe,0` (an icon index) when not quoted.
    let file = file.rsplit_once(',').map_or(file, |(path, index)| {
        if index.trim().chars().all(|c| c.is_ascii_digit() || c == '-') {
            path
        } else {
            file
        }
    });
    let file = file.trim();
    (!file.is_empty() && !file.contains('%')).then(|| file.to_string())
}

/// The folder a file path is in.
fn parent(file: &str) -> Option<String> {
    let (dir, _) = file.rsplit_once(['\\', '/'])?;
    (!dir.is_empty()).then(|| dir.to_string())
}

/// BSManager's program folder, from its uninstall entry: the folder of the exe its icon or
/// uninstall command names, else `InstallLocation`. `None` without such an entry.
pub fn program_dir(entries: &[UninstallEntry]) -> Option<String> {
    entries
        .iter()
        .filter(|e| is_bsmanager(e))
        .find_map(|entry| {
            [&entry.display_icon, &entry.uninstall_string]
                .into_iter()
                .flatten()
                .find_map(|value| program_file(value).and_then(|file| parent(&file)))
                .or_else(|| {
                    entry
                        .install_location
                        .as_deref()
                        .map(str::trim)
                        .filter(|dir| !dir.is_empty() && !dir.contains('%'))
                        .map(str::to_string)
                })
        })
}

/// Where BSManager's installer puts the program for the current user, below the user's local
/// app data folder (seen on a real install; the fallback when no uninstall entry is found).
pub fn default_program_dir(local_app_data: &str) -> String {
    join(local_app_data, &["Programs", "bs-manager"])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, publisher: Option<&str>) -> UninstallEntry {
        UninstallEntry {
            key: "6c7c7738-e2dc-52b5-9d5f-db14049cad46".into(),
            display_name: Some(name.into()),
            publisher: publisher.map(Into::into),
            ..UninstallEntry::default()
        }
    }

    #[test]
    fn reads_the_installation_folder_from_the_config() {
        let json = r#"{"song-details-cache-etag":"x","installation-folder":"D:\\Games","last-version-launched":"1.6.0"}"#;
        assert_eq!(installation_folder(json).as_deref(), Some(r"D:\Games"));
        // A byte order mark, whitespace.
        let json = "\u{feff}{ \"installation-folder\": \"  E:\\\\VR  \" }";
        assert_eq!(installation_folder(json).as_deref(), Some(r"E:\VR"));
    }

    #[test]
    fn a_config_without_a_usable_folder_gives_none() {
        for json in [
            "",
            "not json",
            "[]",
            "{}",
            r#"{"installation-folder": 5}"#,
            r#"{"installation-folder": ""}"#,
            r#"{"installation-folder": "   "}"#,
            r#"{"installation-folder": null}"#,
        ] {
            assert_eq!(installation_folder(json), None, "{json}");
        }
    }

    #[test]
    fn the_instances_folder_is_below_the_installation_folder() {
        assert_eq!(
            instances_dir(r"D:\Games"),
            r"D:\Games\BSManager\BSInstances"
        );
        assert_eq!(
            instances_dir(r"D:\Games\"),
            r"D:\Games\BSManager\BSInstances"
        );
        assert_eq!(instances_dir("/data/vr"), "/data/vr/BSManager/BSInstances");
        // A value that already names the BSManager folder.
        assert_eq!(
            instances_dir(r"D:\Games\BSManager"),
            r"D:\Games\BSManager\BSInstances"
        );
        assert_eq!(
            instances_dir(r"D:\Games\bsmanager\"),
            r"D:\Games\bsmanager\BSInstances"
        );
        // Only a whole last part counts.
        assert_eq!(
            instances_dir(r"D:\MyBSManager"),
            r"D:\MyBSManager\BSManager\BSInstances"
        );
    }

    #[test]
    fn the_defaults_prefer_documents_when_it_exists() {
        let dirs = default_instances_dirs(r"C:\Users\me");
        assert_eq!(dirs[0], r"C:\Users\me\Documents\BSManager\BSInstances");
        assert_eq!(dirs[1], r"C:\Users\me\BSManager\BSInstances");
    }

    #[test]
    fn the_program_folder_comes_from_the_icon_or_the_uninstall_command() {
        let mut e = entry("BSManager 1.6.0", Some("Zagrios"));
        e.display_icon = Some(r"C:\Apps\bs-manager\BSManager.exe,0".into());
        assert_eq!(
            program_dir(&[e.clone()]).as_deref(),
            Some(r"C:\Apps\bs-manager")
        );

        // Without an icon: the quoted uninstall command.
        e.display_icon = None;
        e.uninstall_string =
            Some(r#""D:\Programs\BSM\Uninstall BSManager.exe" /currentuser"#.into());
        assert_eq!(
            program_dir(&[e.clone()]).as_deref(),
            Some(r"D:\Programs\BSM")
        );

        // Else InstallLocation.
        e.uninstall_string = None;
        e.install_location = Some(r"E:\BSM".into());
        assert_eq!(program_dir(&[e.clone()]).as_deref(), Some(r"E:\BSM"));
        e.install_location = Some("   ".into());
        assert_eq!(program_dir(&[e]), None);
    }

    #[test]
    fn only_bsmanagers_own_entry_counts() {
        let mut other = entry("BSManager Fan Tool", Some("Someone Else"));
        other.display_icon = Some(r"C:\Fan\tool.exe,0".into());
        let mut unrelated = entry("Steam", Some("Valve"));
        unrelated.display_icon = Some(r"C:\Steam\steam.exe".into());
        let mut real = entry("BSManager 1.6.0", None);
        real.display_icon = Some(r"C:\Real\BSManager.exe".into());
        assert_eq!(
            program_dir(&[other, unrelated, real]).as_deref(),
            Some(r"C:\Real")
        );
        assert_eq!(program_dir(&[]), None);
    }

    #[test]
    fn values_with_variables_or_no_exe_path_are_not_trusted() {
        let mut e = entry("BSManager 1.6.0", Some("Zagrios"));
        e.display_icon = Some(r"%LOCALAPPDATA%\Programs\bs-manager\BSManager.exe,0".into());
        assert_eq!(program_dir(&[e.clone()]), None);
        e.display_icon = Some("BSManager.exe".into());
        assert_eq!(program_dir(&[e]), None);
    }

    #[test]
    fn program_files_are_read_from_commands_and_icons() {
        assert_eq!(
            program_file(r#""C:\a b\x.exe" /S"#).as_deref(),
            Some(r"C:\a b\x.exe")
        );
        assert_eq!(
            program_file(r"C:\a\x.exe,0").as_deref(),
            Some(r"C:\a\x.exe")
        );
        assert_eq!(
            program_file(r"C:\a\x.exe /S").as_deref(),
            Some(r"C:\a\x.exe")
        );
        assert_eq!(program_file(""), None);
    }

    #[test]
    fn the_default_program_folder_is_below_local_app_data() {
        assert_eq!(
            default_program_dir(r"C:\Users\me\AppData\Local"),
            r"C:\Users\me\AppData\Local\Programs\bs-manager"
        );
    }
}
