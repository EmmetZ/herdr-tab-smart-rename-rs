use crate::naming::{HerdrTab, is_default_label};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub version: u8,
    #[serde(default)]
    pub tabs: HashMap<String, TabState>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            version: 1,
            tabs: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TabState {
    #[serde(default)]
    pub manual: bool,
    #[serde(default)]
    pub auto_label: Option<String>,
    #[serde(default)]
    pub observed_label: Option<String>,
    #[serde(default)]
    pub saw_working: bool,
    #[serde(default)]
    pub auto_after_done: bool,
}

pub fn state_dir_from_env() -> Option<PathBuf> {
    std::env::var("HERDR_PLUGIN_STATE_DIR")
        .ok()
        .map(PathBuf::from)
}

pub fn with_state<T>(
    state_dir: Option<&Path>,
    operation: impl FnOnce(&mut State) -> Result<T>,
) -> Result<T> {
    let Some(state_dir) = state_dir else {
        let mut state = State::default();
        return operation(&mut state);
    };
    fs::create_dir_all(state_dir)
        .with_context(|| format!("failed to create {}", state_dir.display()))?;
    let lock = acquire_lock(&state_dir.join("state.lock"))?;
    let state_file = state_dir.join("state.json");
    let mut state = load_state(&state_file)?;
    let result = operation(&mut state);
    if result.is_ok() {
        save_state(&state_file, &state)?;
    }
    drop(lock);
    result
}

pub fn reconcile_tab(record: &mut TabState, tab: &HerdrTab, force: bool) {
    if force {
        record.manual = false;
        record.auto_label = None;
        record.observed_label = Some(tab.label.clone());
        return;
    }
    if let Some(auto_label) = &record.auto_label {
        if tab.label != *auto_label {
            record.manual = true;
        }
    } else if let Some(observed) = &record.observed_label {
        if tab.label != *observed {
            record.manual = true;
        }
    } else if !is_default_label(&tab.label, &tab.number) {
        record.manual = true;
    }
    record.observed_label = Some(tab.label.clone());
}

fn load_state(path: &Path) -> Result<State> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(serde_json::from_str(&text).context("failed to parse state.json")?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(State::default()),
        Err(error) => Err(error).with_context(|| format!("failed to read {}", path.display())),
    }
}

fn save_state(path: &Path, state: &State) -> Result<()> {
    let tmp = path.with_extension(format!("json.{}.tmp", std::process::id()));
    fs::write(&tmp, format!("{}\n", serde_json::to_string_pretty(state)?))
        .with_context(|| format!("failed to write {}", tmp.display()))?;
    fs::rename(&tmp, path).with_context(|| format!("failed to replace {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).ok();
    }
    Ok(())
}

struct Lock {
    path: PathBuf,
}

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn acquire_lock(path: &Path) -> Result<Lock> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match OpenOptions::new().create_new(true).write(true).open(path) {
            Ok(_) => {
                return Ok(Lock {
                    path: path.to_path_buf(),
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if Instant::now() >= deadline {
                    anyhow::bail!("timed out waiting for {}", path.display());
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(error) => {
                return Err(error).with_context(|| format!("failed to lock {}", path.display()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn manual_labels_are_detected_for_automatic_paths() {
        let tab = HerdrTab {
            tab_id: "t1".into(),
            workspace_id: "w1".into(),
            label: "Manual Task".into(),
            number: json!(1),
        };
        let mut record = TabState::default();
        reconcile_tab(&mut record, &tab, false);
        assert!(record.manual);
    }

    #[test]
    fn explicit_force_reclaims_manual_label() {
        let tab = HerdrTab {
            tab_id: "t1".into(),
            workspace_id: "w1".into(),
            label: "Manual Task".into(),
            number: json!(1),
        };
        let mut record = TabState {
            manual: true,
            ..Default::default()
        };
        reconcile_tab(&mut record, &tab, true);
        assert!(!record.manual);
    }
}
