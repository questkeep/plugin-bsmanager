//! Reading BSManager's instance folders. Pure file-system logic (no SDK types), so it is
//! unit-tested natively against `tests/fixtures/instances`.
//!
//! What BSManager's source establishes (see the README, "How it works"): every instance is a
//! folder directly below `BSInstances`, named after the instance; it holds a full Beat Saber
//! install (`Beat Saber.exe`, `Beat Saber_Data`) and, once BSManager has looked at it, a
//! `metadata.config` JSON file with `id` (a UUID) and `store` (`steam` or `oculus`).

use std::fs;
use std::path::Path;

use serde::Deserialize;

/// The game executable inside every instance folder (`BS_EXECUTABLE` in BSManager).
pub const EXECUTABLE: &str = "Beat Saber.exe";
/// The game's data folder; together with the executable it makes a folder an instance.
pub const DATA_DIR: &str = "Beat Saber_Data";
/// BSManager's per-instance metadata file (`METADATA_FILE`).
pub const METADATA_FILE: &str = "metadata.config";

/// One instance folder that looks like a Beat Saber install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instance {
    /// The folder name, which is the instance's name in BSManager.
    pub folder: String,
    /// BSManager's UUID for the instance, when `metadata.config` has one.
    pub metadata_id: Option<String>,
    /// The store the instance was copied from (`steam` or `oculus`), when known.
    pub store: Option<String>,
}

impl Instance {
    /// The id Questkeep stores: BSManager's UUID (it survives renaming the instance in
    /// BSManager), else the folder name.
    pub fn source_id(&self) -> String {
        self.metadata_id
            .clone()
            .unwrap_or_else(|| format!("folder:{}", self.folder))
    }

    /// The game's name in Questkeep: `Beat Saber (<instance name>)`.
    pub fn game_name(&self) -> String {
        format!("Beat Saber ({})", self.folder)
    }
}

/// The result of looking through the instances folder.
#[derive(Debug, Default)]
pub struct Scan {
    pub instances: Vec<Instance>,
    /// Entries that are not usable instances, with the reason (for the log).
    pub skipped: Vec<(String, String)>,
    /// Instances that were read, but with something odd (for the log).
    pub notes: Vec<(String, String)>,
}

#[derive(Debug, Deserialize)]
struct Metadata {
    id: Option<String>,
    store: Option<String>,
}

/// Reads one instance folder. `Err` carries the reason it is not an instance; the `Ok` side also
/// returns a note about anything odd that did not prevent using it.
pub fn read_instance(dir: &Path) -> Result<(Instance, Option<String>), String> {
    let folder = dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "the folder name is not valid text".to_string())?
        .to_string();
    if !dir.join(EXECUTABLE).is_file() {
        return Err(format!("no {EXECUTABLE}"));
    }
    if !dir.join(DATA_DIR).is_dir() {
        return Err(format!("no {DATA_DIR} folder"));
    }
    let mut note = None;
    let mut metadata = Metadata {
        id: None,
        store: None,
    };
    match fs::read_to_string(dir.join(METADATA_FILE)) {
        Ok(text) => match serde_json::from_str::<Metadata>(&text) {
            Ok(parsed) => metadata = parsed,
            Err(e) => {
                note = Some(format!(
                    "{METADATA_FILE} is not valid ({e}); using the folder name"
                ));
            }
        },
        // BSManager creates the file when it first reads an instance; until then it is absent.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            note = Some(format!(
                "could not read {METADATA_FILE} ({e}); using the folder name"
            ));
        }
    }
    let id = metadata.id.filter(|id| !id.trim().is_empty());
    Ok((
        Instance {
            folder,
            metadata_id: id,
            store: metadata.store.filter(|s| !s.trim().is_empty()),
        },
        note,
    ))
}

/// Looks through `root` (BSManager's `BSInstances` folder). Only an unreadable `root` is an
/// error; a folder that cannot be used is skipped and listed in [`Scan::skipped`].
pub fn scan_instances(root: &Path) -> std::io::Result<Scan> {
    let mut scan = Scan::default();
    let mut dirs = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                scan.skipped
                    .push(("?".into(), format!("unreadable entry ({e})")));
                continue;
            }
        };
        let path = entry.path();
        if path.is_dir() {
            dirs.push(path);
        }
    }
    dirs.sort_by_key(|p| p.file_name().map(|n| n.to_ascii_lowercase()));
    for dir in dirs {
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        match read_instance(&dir) {
            Ok((instance, note)) => {
                if let Some(note) = note {
                    scan.notes.push((name, note));
                }
                scan.instances.push(instance);
            }
            Err(reason) => scan.skipped.push((name, reason)),
        }
    }
    Ok(scan)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/instances")
    }

    #[test]
    fn finds_the_valid_instances_in_name_order() {
        let scan = scan_instances(&fixtures()).unwrap();
        let folders: Vec<&str> = scan.instances.iter().map(|i| i.folder.as_str()).collect();
        assert_eq!(
            folders,
            ["1.34.2", "broken-metadata", "Modded 1.29.1", "no-metadata"]
        );
    }

    #[test]
    fn skips_odd_folders_with_a_reason() {
        let scan = scan_instances(&fixtures()).unwrap();
        let skipped: Vec<(&str, &str)> = scan
            .skipped
            .iter()
            .map(|(n, r)| (n.as_str(), r.as_str()))
            .collect();
        assert_eq!(
            skipped,
            [
                ("incomplete", "no Beat Saber.exe"),
                ("not-an-instance", "no Beat Saber.exe"),
            ]
        );
    }

    #[test]
    fn reads_the_metadata() {
        let scan = scan_instances(&fixtures()).unwrap();
        let by = |name: &str| scan.instances.iter().find(|i| i.folder == name).unwrap();
        let steam = by("1.34.2");
        assert_eq!(
            steam.metadata_id.as_deref(),
            Some("11111111-1111-4111-8111-111111111111")
        );
        assert_eq!(steam.store.as_deref(), Some("steam"));
        assert_eq!(steam.source_id(), "11111111-1111-4111-8111-111111111111");
        assert_eq!(by("Modded 1.29.1").store.as_deref(), Some("oculus"));
    }

    #[test]
    fn missing_or_broken_metadata_falls_back_to_the_folder_name() {
        let scan = scan_instances(&fixtures()).unwrap();
        let by = |name: &str| scan.instances.iter().find(|i| i.folder == name).unwrap();
        assert_eq!(by("no-metadata").source_id(), "folder:no-metadata");
        assert_eq!(by("broken-metadata").source_id(), "folder:broken-metadata");
        let notes: Vec<&str> = scan.notes.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            notes,
            ["broken-metadata"],
            "only the broken file is worth a note"
        );
    }

    #[test]
    fn names_the_game_after_the_instance() {
        let scan = scan_instances(&fixtures()).unwrap();
        let names: Vec<String> = scan.instances.iter().map(Instance::game_name).collect();
        assert_eq!(names[0], "Beat Saber (1.34.2)");
        assert_eq!(names[2], "Beat Saber (Modded 1.29.1)");
    }

    #[test]
    fn a_missing_root_is_an_error() {
        assert!(scan_instances(&fixtures().join("does-not-exist")).is_err());
    }
}
