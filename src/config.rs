//! BSManager's own settings file (`config.json` in its app-data folder, written by
//! `electron-store`). Only two keys matter here, both found in BSManager's source:
//!
//! - `installation-folder`: set when the user moved BSManager's data. It is the *parent* of the
//!   `BSManager` folder (BSManager strips a trailing `BSManager` before saving it).
//! - `last-version-launched`: the last instance started from BSManager, a `BSVersion` object whose
//!   `path` is the full path of the game folder.
//!
//! They are what tells the plugin the real Windows path of the instances folder: the sandbox only
//! shows mount paths, and Questkeep does not translate those when it launches a game.
//! Paths are handled as text with Windows separators, so this also runs in native tests anywhere.

use serde::Deserialize;

/// The folder BSManager creates inside the installation folder.
const DATA_FOLDER: &str = "BSManager";
/// The folder holding the instances.
const INSTANCES_FOLDER: &str = "BSInstances";

#[derive(Debug, Default, Deserialize)]
struct Raw {
    #[serde(rename = "installation-folder")]
    installation_folder: Option<String>,
    #[serde(rename = "last-version-launched")]
    last_version_launched: Option<LastLaunched>,
}

#[derive(Debug, Deserialize)]
struct LastLaunched {
    path: Option<String>,
}

/// The parts of BSManager's `config.json` the plugin uses.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Config {
    pub installation_folder: Option<String>,
    pub last_launched_path: Option<String>,
}

impl Config {
    pub fn parse(text: &str) -> Result<Self, String> {
        let raw: Raw = serde_json::from_str(text).map_err(|e| e.to_string())?;
        Ok(Self {
            installation_folder: raw.installation_folder.filter(|s| !s.trim().is_empty()),
            last_launched_path: raw
                .last_version_launched
                .and_then(|l| l.path)
                .filter(|s| !s.trim().is_empty()),
        })
    }

    /// The real path of the `BSInstances` folder, as far as the config says: the configured
    /// installation folder first, then where the last launched instance lives. `None` when BSManager
    /// has never been moved or used to start an instance (the default location is the user's home,
    /// which the sandbox cannot see).
    pub fn instances_dir(&self) -> Option<String> {
        if let Some(parent) = &self.installation_folder {
            let parent = parent.trim_end_matches(['\\', '/']);
            let parent = strip_data_folder(parent);
            return Some(join(&join(parent, DATA_FOLDER), INSTANCES_FOLDER));
        }
        let last = self.last_launched_path.as_deref()?;
        let parts: Vec<&str> = last.split(['\\', '/']).collect();
        let at = parts
            .iter()
            .position(|p| p.eq_ignore_ascii_case(INSTANCES_FOLDER))?;
        // `BSInstances` must have something above it, and the instance below it.
        (at > 0 && at + 1 < parts.len()).then(|| parts[..=at].join("\\"))
    }
}

/// `D:\Games\BSManager` becomes `D:\Games` (BSManager does the same with a chosen folder).
fn strip_data_folder(parent: &str) -> &str {
    match parent.rsplit_once(['\\', '/']) {
        Some((rest, last)) if last.eq_ignore_ascii_case(DATA_FOLDER) => rest,
        _ => parent,
    }
}

/// Joins two Windows path parts.
pub fn join(base: &str, name: &str) -> String {
    format!("{}\\{}", base.trim_end_matches(['\\', '/']), name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Config {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/config")
            .join(name);
        Config::parse(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn a_configured_installation_folder_wins() {
        assert_eq!(
            fixture("custom-folder.json").instances_dir().as_deref(),
            Some(r"F:\Beat Saber stuff\BSManager\BSInstances")
        );
    }

    #[test]
    fn the_last_launched_instance_shows_where_the_instances_are() {
        assert_eq!(
            fixture("last-launched.json").instances_dir().as_deref(),
            Some(r"D:\Games\BSManager\BSInstances")
        );
    }

    #[test]
    fn an_official_install_says_nothing_about_the_instances() {
        let config = Config::parse(
            r#"{"last-version-launched":{"path":"C:\\Program Files (x86)\\Steam\\steamapps\\common\\Beat Saber"}}"#,
        )
        .unwrap();
        assert_eq!(config.instances_dir(), None);
    }

    #[test]
    fn nothing_known_means_no_path() {
        assert_eq!(fixture("empty.json").instances_dir(), None);
        assert_eq!(Config::parse("{}").unwrap().instances_dir(), None);
    }

    #[test]
    fn the_installation_folder_may_already_end_in_bsmanager() {
        let config = Config::parse(r#"{"installation-folder":"D:/Games/BSManager/"}"#).unwrap();
        assert_eq!(
            config.instances_dir().as_deref(),
            Some(r"D:/Games\BSManager\BSInstances")
        );
    }

    #[test]
    fn invalid_json_is_an_error() {
        assert!(Config::parse("{nope").is_err());
    }
}
