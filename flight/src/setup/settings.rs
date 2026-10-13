// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

const VERSION: u32 = 1;
const FILE: &str = "settings.toml";

/// What the person told setup (ADR-012): which agents they use and which one a new workspace
/// starts. No credential, no path to one. Versioned like every persisted file: a newer version
/// or an unknown key is refused and the file is left as found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    version: u32,
    pub agents: AgentChoice,
    pub setup: SetupState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentChoice {
    /// Agent ids the person works with, in their order of preference.
    pub enabled: Vec<String>,
    /// The one a new workspace starts unless told otherwise (one of `enabled`, or empty).
    pub default: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetupState {
    /// Setup ran to the end at least once.
    pub completed: bool,
    /// The person declined the offer made by a plain `flight`; it is not repeated.
    pub declined: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: VERSION,
            agents: AgentChoice {
                enabled: vec!["claude".to_owned()],
                default: "claude".to_owned(),
            },
            setup: SetupState {
                completed: false,
                declined: false,
            },
        }
    }
}

/// Why the settings file could not be used. It is never overwritten in these cases.
#[derive(Debug, PartialEq, Eq)]
pub enum SettingsError {
    Newer(u32),
    Unreadable(String),
}

impl std::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Newer(v) => write!(
                f,
                "settings.toml was written by a newer Flight (version {v}); it is left untouched"
            ),
            Self::Unreadable(why) => write!(
                f,
                "settings.toml cannot be used ({why}); it is left untouched"
            ),
        }
    }
}

/// The settings file in a configuration directory.
pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    pub fn in_dir(dir: &Path) -> Self {
        Self {
            path: dir.join(FILE),
        }
    }

    /// `Ok(None)` when there is no file yet.
    pub fn load(&self) -> Result<Option<Settings>, SettingsError> {
        let text = match fs::read_to_string(&self.path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(SettingsError::Unreadable(e.to_string())),
        };
        // The version is looked at first so that a newer file is reported as newer even when
        // it also has keys this build does not know.
        let probe: toml::Value = text
            .parse()
            .map_err(|e: toml::de::Error| SettingsError::Unreadable(e.message().to_owned()))?;
        if let Some(v) = probe.get("version").and_then(toml::Value::as_integer) {
            if let Ok(v) = u32::try_from(v) {
                if v > VERSION {
                    return Err(SettingsError::Newer(v));
                }
            }
        }
        toml::from_str(&text)
            .map(Some)
            .map_err(|e| SettingsError::Unreadable(e.message().to_owned()))
    }

    /// Replace the file atomically, readable by its owner only.
    pub fn save(&self, settings: &Settings) -> Result<(), String> {
        let text = toml::to_string_pretty(settings).map_err(|e| e.to_string())?;
        let dir = self.path.parent().ok_or("no directory for settings.toml")?;
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let tmp = dir.join(format!(".{FILE}.{}.tmp", std::process::id()));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&tmp)
            .map_err(|e| format!("{}: {e}", tmp.display()))?;
        file.write_all(text.as_bytes())
            .and_then(|()| file.sync_all())
            .map_err(|e| format!("{}: {e}", tmp.display()))?;
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
        fs::rename(&tmp, &self.path).map_err(|e| {
            let _ = fs::remove_file(&tmp);
            format!("{}: {e}", self.path.display())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("flight-settings-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn no_file_is_not_an_error_and_a_saved_file_comes_back_private() {
        let dir = scratch("roundtrip");
        let store = SettingsStore::in_dir(&dir);
        assert_eq!(store.load(), Ok(None));
        let mut s = Settings::default();
        s.agents.enabled = vec!["claude".into(), "codex".into()];
        s.setup.completed = true;
        store.save(&s).unwrap();
        assert_eq!(store.load().unwrap(), Some(s));
        let mode = fs::metadata(dir.join(FILE)).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_newer_version_or_an_unknown_key_is_refused_and_the_file_is_untouched() {
        let dir = scratch("refuse");
        let store = SettingsStore::in_dir(&dir);
        let newer = "version = 2\nsomething = \"new\"\n";
        fs::write(dir.join(FILE), newer).unwrap();
        assert_eq!(store.load(), Err(SettingsError::Newer(2)));
        let unknown = "version = 1\nextra = 1\n[agents]\nenabled = []\ndefault = \"\"\n[setup]\ncompleted = false\ndeclined = false\n";
        fs::write(dir.join(FILE), unknown).unwrap();
        assert!(matches!(store.load(), Err(SettingsError::Unreadable(_))));
        assert_eq!(fs::read_to_string(dir.join(FILE)).unwrap(), unknown);
        let _ = fs::remove_dir_all(&dir);
    }
}
