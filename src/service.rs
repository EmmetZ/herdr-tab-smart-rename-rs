use crate::herdr::HerdrApi;
use crate::naming::{
    AgentSession, HerdrSnapshot, HerdrTab, NameSuggestion, PaneContext, RenameResult,
    build_model_context, focused_pane_for, heuristic_title, is_default_label, workspace_candidate,
};
use crate::provider::Namer;
use crate::state::{reconcile_tab, state_dir_from_env, with_state};
use crate::text::bounded_text;
use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Service<'a> {
    herdr: &'a dyn HerdrApi,
    namer: &'a dyn Namer,
    state_dir: Option<PathBuf>,
}

impl<'a> Service<'a> {
    pub fn from_env(herdr: &'a dyn HerdrApi, namer: &'a dyn Namer) -> Self {
        Self {
            herdr,
            namer,
            state_dir: state_dir_from_env(),
        }
    }

    pub fn new(herdr: &'a dyn HerdrApi, namer: &'a dyn Namer, state_dir: Option<PathBuf>) -> Self {
        Self {
            herdr,
            namer,
            state_dir,
        }
    }

    pub fn rename_current(&self, force: bool) -> Result<RenameResult> {
        let snap = self.herdr.snapshot()?;
        let tab_id = std::env::var("HERDR_TAB_ID")
            .ok()
            .or_else(|| snap.focused_tab_id.clone())
            .context("No current Herdr tab")?;
        self.evaluate_tab(&tab_id, force, false, false)
    }

    pub fn dry_run_current(&self) -> Result<RenameResult> {
        let snap = self.herdr.snapshot()?;
        let tab_id = std::env::var("HERDR_TAB_ID")
            .ok()
            .or_else(|| snap.focused_tab_id.clone())
            .context("No current Herdr tab")?;
        self.evaluate_tab(&tab_id, true, true, false)
    }

    pub fn handle_agent_status_event(&self) -> Result<Option<RenameResult>> {
        let Some(event) = AgentStatusEvent::from_env()? else {
            return Ok(None);
        };
        if event.event.as_deref() != Some("pane.agent_status_changed") {
            return Ok(None);
        }

        let status = event.new_status();
        let tab_id = event.tab_id();

        if status.as_deref() == Some("working") {
            if let Some(tab_id) = tab_id {
                with_state(self.state_dir.as_deref(), |state| {
                    state.tabs.entry(tab_id).or_default().saw_working = true;
                    Ok(())
                })?;
            }
            return Ok(None);
        }

        if !is_completion_status(status.as_deref()) {
            return Ok(None);
        }

        let snap = self.herdr.snapshot()?;
        let tab_id = tab_id
            .or_else(|| {
                event
                    .pane_id()
                    .and_then(|pane_id| tab_id_for_pane(&snap, &pane_id))
            })
            .context("agent completion event did not identify a tab")?;
        let should_run = with_state(self.state_dir.as_deref(), |state| {
            let record = state.tabs.entry(tab_id.clone()).or_default();
            if record.auto_after_done {
                return Ok(false);
            }
            let saw_work = record.saw_working
                || matches!(event.old_status().as_deref(), Some("working" | "blocked"));
            Ok(saw_work || event.old_status().is_none())
        })?;

        if !should_run {
            return Ok(None);
        }

        Ok(Some(self.evaluate_tab(&tab_id, false, false, true)?))
    }

    pub fn notify_result(&self, result: &RenameResult) {
        let (title, body, request) = if result.changed {
            (
                "Tab renamed",
                format!(
                    "{} -> {}",
                    result.from,
                    result.candidate.as_deref().unwrap_or("")
                ),
                false,
            )
        } else if result.skipped {
            ("Tab not renamed", result.reason.clone(), true)
        } else if let Some(candidate) = &result.candidate {
            (
                "Tab not renamed",
                format!("Already named {candidate}"),
                true,
            )
        } else {
            ("Tab not renamed", result.reason.clone(), true)
        };
        let _ = self.herdr.notify(title, &body, request);
    }

    fn evaluate_tab(
        &self,
        tab_id: &str,
        force: bool,
        dry_run: bool,
        automatic_after_done: bool,
    ) -> Result<RenameResult> {
        let snap = self.herdr.snapshot()?;
        let tab = find_tab(&snap, tab_id)?;
        let skip_reason = with_state(self.state_dir.as_deref(), |state| {
            let record = state.tabs.entry(tab.tab_id.clone()).or_default();
            if automatic_after_done && record.auto_after_done {
                return Ok(Some("already handled first agent completion".to_string()));
            }
            reconcile_tab(record, tab, force);
            if !force && record.manual {
                Ok(Some("manual tab name".to_string()))
            } else {
                Ok(None)
            }
        })?;

        if let Some(reason) = skip_reason {
            if automatic_after_done {
                self.mark_after_done(tab_id, None)?;
            }
            return Ok(RenameResult {
                tab: tab.tab_id.clone(),
                from: tab.label.clone(),
                candidate: None,
                reason,
                used_model: false,
                changed: false,
                skipped: true,
            });
        }

        let suggestion = self.suggest_for_tab(&snap, tab)?;
        let candidate = suggestion.suggestion.tab.clone();
        let changed = candidate.as_ref().is_some_and(|label| label != &tab.label);
        if changed && !dry_run {
            self.herdr
                .rename_tab(&tab.tab_id, candidate.as_ref().unwrap())?;
        }

        if !dry_run {
            self.remember_result(tab, candidate.as_deref(), automatic_after_done)?;
        }

        Ok(RenameResult {
            tab: tab.tab_id.clone(),
            from: tab.label.clone(),
            candidate,
            reason: suggestion.suggestion.reason,
            used_model: suggestion.used_model,
            changed,
            skipped: false,
        })
    }

    fn suggest_for_tab(&self, snap: &HerdrSnapshot, tab: &HerdrTab) -> Result<SuggestionResult> {
        let workspace = snap
            .workspaces
            .iter()
            .find(|workspace| workspace.workspace_id == tab.workspace_id)
            .context("tab workspace not found")?;
        let focused_pane = focused_pane_for(tab, snap);
        let workspace_name = workspace_candidate(workspace, focused_pane);
        let pane_contexts =
            self.pane_contexts(tab, snap, focused_pane.map(|pane| pane.pane_id.as_str()))?;
        let focused = pane_contexts
            .iter()
            .find(|pane| pane.focused)
            .or_else(|| pane_contexts.first());

        if let Some(context) = focused
            && context.user_messages.is_empty()
            && let Some(label) = heuristic_title(context)
        {
            return Ok(SuggestionResult {
                suggestion: NameSuggestion {
                    tab: Some(label),
                    reason: "process heuristic".to_string(),
                },
                used_model: false,
            });
        }

        let model_context = build_model_context(&workspace_name, &pane_contexts)?;
        match self.namer.suggest(&model_context) {
            Ok(suggestion) => Ok(SuggestionResult {
                suggestion,
                used_model: true,
            }),
            Err(error) => Ok(SuggestionResult {
                suggestion: NameSuggestion {
                    tab: None,
                    reason: bounded_text(format!("{error:#}"), 240),
                },
                used_model: true,
            }),
        }
    }

    fn pane_contexts(
        &self,
        tab: &HerdrTab,
        snap: &HerdrSnapshot,
        focused_pane_id: Option<&str>,
    ) -> Result<Vec<PaneContext>> {
        let panes = snap
            .panes
            .iter()
            .filter(|pane| pane.tab_id == tab.tab_id)
            .collect::<Vec<_>>();
        let mut contexts = Vec::with_capacity(panes.len());
        for pane in panes {
            let focused = Some(pane.pane_id.as_str()) == focused_pane_id;
            let process = self.herdr.pane_process(&pane.pane_id).unwrap_or(None);
            let recent_output = if focused {
                self.herdr.pane_read(&pane.pane_id, 120).unwrap_or_default()
            } else {
                String::new()
            };
            let user_messages = if focused {
                session_user_messages(pane.agent_session.as_ref())
            } else {
                vec![]
            };
            contexts.push(PaneContext {
                focused,
                label: bounded_text(pane.label.as_deref().unwrap_or_default(), 80),
                process,
                recent_output,
                user_messages,
            });
        }
        Ok(contexts)
    }

    fn remember_result(
        &self,
        tab: &HerdrTab,
        candidate: Option<&str>,
        automatic_after_done: bool,
    ) -> Result<()> {
        with_state(self.state_dir.as_deref(), |state| {
            let record = state.tabs.entry(tab.tab_id.clone()).or_default();
            if let Some(label) = candidate {
                record.auto_label = Some(label.to_string());
                record.observed_label = Some(label.to_string());
                record.manual = false;
            } else {
                record.observed_label = Some(tab.label.clone());
            }
            if automatic_after_done {
                record.auto_after_done = true;
            }
            Ok(())
        })
    }

    fn mark_after_done(&self, tab_id: &str, candidate: Option<&str>) -> Result<()> {
        with_state(self.state_dir.as_deref(), |state| {
            let record = state.tabs.entry(tab_id.to_string()).or_default();
            if let Some(candidate) = candidate {
                record.auto_label = Some(candidate.to_string());
            }
            record.auto_after_done = true;
            Ok(())
        })
    }
}

struct SuggestionResult {
    suggestion: NameSuggestion,
    used_model: bool,
}

fn find_tab<'a>(snap: &'a HerdrSnapshot, tab_id: &str) -> Result<&'a HerdrTab> {
    snap.tabs
        .iter()
        .find(|tab| tab.tab_id == tab_id)
        .with_context(|| format!("tab not found: {tab_id}"))
}

fn tab_id_for_pane(snap: &HerdrSnapshot, pane_id: &str) -> Option<String> {
    snap.panes
        .iter()
        .find(|pane| pane.pane_id == pane_id)
        .map(|pane| pane.tab_id.clone())
}

fn is_completion_status(status: Option<&str>) -> bool {
    matches!(status, Some("done" | "idle"))
}

fn session_user_messages(session: Option<&AgentSession>) -> Vec<String> {
    let Some(session) = session else {
        return vec![];
    };
    if session.kind != "path" {
        return vec![];
    }
    let path = Path::new(&session.value);
    if !path.is_absolute() {
        return vec![];
    }
    let Ok(metadata) = fs::metadata(path) else {
        return vec![];
    };
    if !metadata.is_file() {
        return vec![];
    }
    let Ok(bytes) = fs::read(path) else {
        return vec![];
    };
    let start = bytes.len().saturating_sub(512 * 1024);
    let text = String::from_utf8_lossy(&bytes[start..]);
    text.lines()
        .filter_map(user_message_from_line)
        .rev()
        .take(6)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

fn user_message_from_line(line: &str) -> Option<String> {
    let value: Value = serde_json::from_str(line).ok()?;
    let message = value.get("message")?;
    if message.get("role")?.as_str()? != "user" {
        return None;
    }
    let content = message.get("content")?;
    let text = match content {
        Value::String(value) => value.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter(|part| part.get("type").and_then(Value::as_str) == Some("text"))
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(" "),
        _ => String::new(),
    };
    let text = bounded_text(text, 1_000);
    (!text.is_empty()).then_some(text)
}

#[derive(Debug, Deserialize)]
struct AgentStatusEvent {
    #[serde(default)]
    event: Option<String>,
    #[serde(default)]
    data: Value,
}

impl AgentStatusEvent {
    fn from_env() -> Result<Option<Self>> {
        let Ok(raw) = std::env::var("HERDR_PLUGIN_EVENT_JSON") else {
            return Ok(None);
        };
        Ok(Some(
            serde_json::from_str(&raw).context("failed to parse HERDR_PLUGIN_EVENT_JSON")?,
        ))
    }

    fn new_status(&self) -> Option<String> {
        self.first_string(&["status", "agent_status", "new_status", "to"])
    }

    fn old_status(&self) -> Option<String> {
        self.first_string(&["old_status", "previous_status", "from"])
    }

    fn tab_id(&self) -> Option<String> {
        self.first_string(&["tab_id"])
    }

    fn pane_id(&self) -> Option<String> {
        self.first_string(&["pane_id"])
    }

    fn first_string(&self, keys: &[&str]) -> Option<String> {
        first_string(&self.data, keys)
    }
}

fn first_string(value: &Value, keys: &[&str]) -> Option<String> {
    let object = value.as_object()?;
    for key in keys {
        if let Some(value) = object.get(*key).and_then(Value::as_str) {
            return Some(value.to_string());
        }
    }
    for nested in ["pane", "tab", "agent"] {
        if let Some(found) = object
            .get(nested)
            .and_then(|value| first_string(value, keys))
        {
            return Some(found);
        }
    }
    None
}

#[allow(dead_code)]
fn auto_eligible(tab: &HerdrTab) -> bool {
    is_default_label(&tab.label, &tab.number)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::naming::{
        AgentSession, HerdrLayout, HerdrPane, HerdrWorkspace, NamingContext, ProcessInfo, Worktree,
    };
    use serde_json::json;
    use std::cell::RefCell;
    use tempfile::TempDir;

    struct FakeHerdr {
        snap: RefCell<HerdrSnapshot>,
        read: String,
        process: Option<ProcessInfo>,
        renames: RefCell<Vec<(String, String)>>,
    }

    impl HerdrApi for FakeHerdr {
        fn snapshot(&self) -> Result<HerdrSnapshot> {
            Ok(self.snap.borrow().clone())
        }

        fn rename_tab(&self, tab_id: &str, label: &str) -> Result<()> {
            self.renames
                .borrow_mut()
                .push((tab_id.to_string(), label.to_string()));
            if let Some(tab) = self
                .snap
                .borrow_mut()
                .tabs
                .iter_mut()
                .find(|tab| tab.tab_id == tab_id)
            {
                tab.label = label.to_string();
            }
            Ok(())
        }

        fn pane_read(&self, _pane_id: &str, _lines: usize) -> Result<String> {
            Ok(self.read.clone())
        }

        fn pane_process(&self, _pane_id: &str) -> Result<Option<ProcessInfo>> {
            Ok(self.process.clone())
        }

        fn notify(&self, _title: &str, _body: &str, _request_sound: bool) -> Result<()> {
            Ok(())
        }
    }

    struct FixedNamer(NameSuggestion);

    impl Namer for FixedNamer {
        fn suggest(&self, _context: &NamingContext) -> Result<NameSuggestion> {
            Ok(self.0.clone())
        }
    }

    fn snapshot(tab_label: &str) -> HerdrSnapshot {
        HerdrSnapshot {
            focused_workspace_id: Some("w1".into()),
            focused_tab_id: Some("t1".into()),
            focused_pane_id: Some("p1".into()),
            workspaces: vec![HerdrWorkspace {
                workspace_id: "w1".into(),
                label: "Project".into(),
                number: json!(1),
                active_tab_id: Some("t1".into()),
                cwd: None,
                worktree: Some(Worktree {
                    repo_name: Some("Project".into()),
                }),
            }],
            tabs: vec![HerdrTab {
                tab_id: "t1".into(),
                workspace_id: "w1".into(),
                label: tab_label.into(),
                number: json!(1),
            }],
            panes: vec![HerdrPane {
                pane_id: "p1".into(),
                tab_id: "t1".into(),
                workspace_id: "w1".into(),
                label: None,
                cwd: None,
                foreground_cwd: None,
                agent: Some("codex".into()),
                agent_status: Some("done".into()),
                agent_session: None::<AgentSession>,
            }],
            layouts: vec![HerdrLayout {
                tab_id: "t1".into(),
                focused_pane_id: Some("p1".into()),
            }],
        }
    }

    #[test]
    fn manual_action_forces_rename() {
        let fake = FakeHerdr {
            snap: RefCell::new(snapshot("Manual Label")),
            read: "Implemented Rust rewrite".into(),
            process: None,
            renames: RefCell::new(vec![]),
        };
        let namer = FixedNamer(NameSuggestion {
            tab: Some("Implement Rust Rewrite".into()),
            reason: "current task".into(),
        });
        let tmp = TempDir::new().unwrap();
        let service = Service::new(&fake, &namer, Some(tmp.path().to_path_buf()));

        let result = service.evaluate_tab("t1", true, false, false).unwrap();
        assert!(result.changed);
        assert_eq!(fake.renames.borrow().len(), 1);
    }

    #[test]
    fn automatic_completion_skips_user_named_tabs() {
        let fake = FakeHerdr {
            snap: RefCell::new(snapshot("Manual Label")),
            read: "Implemented Rust rewrite".into(),
            process: None,
            renames: RefCell::new(vec![]),
        };
        let namer = FixedNamer(NameSuggestion {
            tab: Some("Implement Rust Rewrite".into()),
            reason: "current task".into(),
        });
        let tmp = TempDir::new().unwrap();
        let service = Service::new(&fake, &namer, Some(tmp.path().to_path_buf()));

        let result = service.evaluate_tab("t1", false, false, true).unwrap();
        assert!(result.skipped);
        assert!(fake.renames.borrow().is_empty());
    }

    #[test]
    fn automatic_completion_renames_default_tab_once() {
        let fake = FakeHerdr {
            snap: RefCell::new(snapshot("1")),
            read: "Implemented Rust rewrite".into(),
            process: None,
            renames: RefCell::new(vec![]),
        };
        let namer = FixedNamer(NameSuggestion {
            tab: Some("Implement Rust Rewrite".into()),
            reason: "current task".into(),
        });
        let tmp = TempDir::new().unwrap();
        let service = Service::new(&fake, &namer, Some(tmp.path().to_path_buf()));

        let result = service.evaluate_tab("t1", false, false, true).unwrap();
        assert!(result.changed);
        let second = service.evaluate_tab("t1", false, false, true).unwrap();
        assert!(second.skipped);
        assert_eq!(fake.renames.borrow().len(), 1);
    }

    #[test]
    fn idle_is_a_completion_status_for_codex() {
        assert!(is_completion_status(Some("done")));
        assert!(is_completion_status(Some("idle")));
        assert!(!is_completion_status(Some("working")));
        assert!(!is_completion_status(Some("blocked")));
        assert!(!is_completion_status(None));
    }
}
