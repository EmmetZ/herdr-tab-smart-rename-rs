use crate::text::{bounded_text, sanitize_text};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use std::sync::LazyLock;

pub const MAX_TAB_LENGTH: usize = 30;
const MAX_CONTEXT_CHARS: usize = 4_500;

#[derive(Debug, Clone, Deserialize)]
pub struct ApiResponse<T> {
    pub result: T,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SnapshotResult {
    pub snapshot: HerdrSnapshot,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HerdrSnapshot {
    #[serde(default)]
    pub focused_workspace_id: Option<String>,
    #[serde(default)]
    pub focused_tab_id: Option<String>,
    #[serde(default)]
    pub focused_pane_id: Option<String>,
    #[serde(default)]
    pub workspaces: Vec<HerdrWorkspace>,
    #[serde(default)]
    pub tabs: Vec<HerdrTab>,
    #[serde(default)]
    pub panes: Vec<HerdrPane>,
    #[serde(default)]
    pub layouts: Vec<HerdrLayout>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HerdrWorkspace {
    pub workspace_id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub number: Value,
    #[serde(default)]
    pub active_tab_id: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub worktree: Option<Worktree>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Worktree {
    #[serde(default)]
    pub repo_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HerdrTab {
    pub tab_id: String,
    pub workspace_id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub number: Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HerdrPane {
    pub pane_id: String,
    pub tab_id: String,
    pub workspace_id: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub foreground_cwd: Option<String>,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub agent_status: Option<String>,
    #[serde(default)]
    pub agent_session: Option<AgentSession>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentSession {
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HerdrLayout {
    pub tab_id: String,
    #[serde(default)]
    pub focused_pane_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessInfo {
    pub name: String,
    pub command: String,
    pub cwd: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaneContext {
    pub focused: bool,
    pub label: String,
    pub process: Option<ProcessInfo>,
    pub recent_output: String,
    pub user_messages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamingContext {
    pub project: String,
    pub focused_pane: PaneEvidence,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub user_requests: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sibling_panes: Vec<SiblingEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaneEvidence {
    pub process: Option<ProcessInfo>,
    pub recent_output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiblingEvidence {
    pub label: String,
    pub process: Option<ProcessInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NameSuggestion {
    pub tab: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RenameResult {
    pub tab: String,
    pub from: String,
    pub candidate: Option<String>,
    pub reason: String,
    pub used_model: bool,
    pub changed: bool,
    pub skipped: bool,
}

static LABEL_WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Z0-9][A-Za-z0-9+.#/'-]*$").unwrap());

pub fn is_default_label(label: &str, number: &Value) -> bool {
    let value = label.trim();
    value.is_empty() || value.chars().all(|c| c.is_ascii_digit()) || value == number_string(number)
}

fn number_string(number: &Value) -> String {
    match number {
        Value::String(value) => value.clone(),
        Value::Number(value) => value.to_string(),
        _ => String::new(),
    }
}

pub fn title_case(input: impl AsRef<str>) -> String {
    let acronyms = ["api", "cli", "ui", "pr", "var", "rpc", "mvp"];
    input
        .as_ref()
        .replace(['-', '_'], " ")
        .split_whitespace()
        .map(|word| {
            if acronyms.contains(&word.to_ascii_lowercase().as_str()) {
                word.to_ascii_uppercase()
            } else {
                let mut chars = word.chars();
                match chars.next() {
                    Some(first) => {
                        format!(
                            "{}{}",
                            first.to_uppercase(),
                            chars.as_str().to_ascii_lowercase()
                        )
                    }
                    None => String::new(),
                }
            }
        })
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn validate_tab_label(label: &str) -> bool {
    if label.contains('\n') || label.contains('\r') {
        return false;
    }
    let value = sanitize_text(label);
    if value.is_empty() || value.chars().count() > MAX_TAB_LENGTH {
        return false;
    }
    let words: Vec<_> = value.split_whitespace().collect();
    if !(2..=4).contains(&words.len()) {
        return false;
    }
    let connectors = ["a", "an", "and", "for", "in", "of", "on", "to", "with"];
    words.iter().enumerate().all(|(index, word)| {
        LABEL_WORD.is_match(word)
            || (index > 0 && connectors.contains(&word.to_ascii_lowercase().as_str()))
    })
}

pub fn workspace_candidate(workspace: &HerdrWorkspace, pane: Option<&HerdrPane>) -> String {
    let meaningful_label = (!is_default_label(&workspace.label, &workspace.number))
        .then_some(workspace.label.as_str());
    let identity = workspace
        .worktree
        .as_ref()
        .and_then(|worktree| worktree.repo_name.as_deref())
        .or(meaningful_label)
        .or_else(|| pane.and_then(|pane| pane.foreground_cwd.as_deref().or(pane.cwd.as_deref())))
        .or(workspace.cwd.as_deref())
        .unwrap_or(&workspace.label);

    title_case(
        Path::new(identity)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(identity),
    )
}

pub fn focused_pane_for<'a>(tab: &HerdrTab, snap: &'a HerdrSnapshot) -> Option<&'a HerdrPane> {
    let panes: Vec<&HerdrPane> = snap
        .panes
        .iter()
        .filter(|pane| pane.tab_id == tab.tab_id)
        .collect();
    let layout_focus = snap
        .layouts
        .iter()
        .find(|layout| layout.tab_id == tab.tab_id)
        .and_then(|layout| layout.focused_pane_id.as_deref());
    let focused_id = layout_focus.or(snap.focused_pane_id.as_deref());
    let focused = focused_id.and_then(|id| panes.iter().copied().find(|pane| pane.pane_id == id));

    focused
        .filter(|pane| pane.agent.is_some())
        .or_else(|| {
            panes.iter().copied().find(|pane| {
                pane.agent.is_some()
                    && matches!(
                        pane.agent_status.as_deref(),
                        Some("working" | "blocked" | "done")
                    )
            })
        })
        .or(focused)
        .or_else(|| panes.first().copied())
}

pub fn heuristic_title(context: &PaneContext) -> Option<String> {
    let process = context
        .process
        .as_ref()
        .map(|process| format!("{} {}", process.name, process.command).to_ascii_lowercase())
        .unwrap_or_default();
    let output = context.recent_output.to_ascii_lowercase();

    if contains_any(
        &process,
        &[
            "vitest",
            "jest",
            "pytest",
            "rspec",
            "cargo test",
            "go test",
            "node --test",
            "bun test",
        ],
    ) {
        Some("Run Tests".to_string())
    } else if contains_any(
        &process,
        &[
            "next",
            "vite",
            "webpack",
            "astro",
            "rails server",
            "npm run dev",
            "pnpm dev",
            "yarn dev",
        ],
    ) {
        Some("Dev Server".to_string())
    } else if contains_any(&process, &["tail", "journalctl", "docker logs"])
        || output.contains("following logs")
    {
        Some("View Logs".to_string())
    } else if contains_any(&process, &["ssh", "mosh"]) {
        Some("Remote Shell".to_string())
    } else {
        None
    }
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

pub fn build_model_context(
    workspace_name: &str,
    pane_contexts: &[PaneContext],
) -> anyhow::Result<NamingContext> {
    let focused = pane_contexts
        .iter()
        .find(|pane| pane.focused)
        .or_else(|| pane_contexts.first());
    let user_requests = focused
        .map(|pane| {
            pane.user_messages
                .iter()
                .map(|message| bounded_text(message, 700))
                .filter(|message| !message.is_empty())
                .rev()
                .take(6)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();
    let focused_pane = PaneEvidence {
        process: focused.and_then(|pane| pane.process.clone()),
        recent_output: focused
            .map(|pane| bounded_text(&pane.recent_output, 800))
            .unwrap_or_default(),
    };
    let sibling_panes = pane_contexts
        .iter()
        .filter(|pane| !pane.focused)
        .take(4)
        .map(|pane| SiblingEvidence {
            label: bounded_text(&pane.label, 80),
            process: pane.process.clone(),
        })
        .collect();
    let mut context = NamingContext {
        project: bounded_text(workspace_name, 80),
        focused_pane,
        user_requests,
        sibling_panes,
    };

    if serde_json::to_string(&context)?.len() > MAX_CONTEXT_CHARS {
        context.focused_pane.recent_output = bounded_text(&context.focused_pane.recent_output, 350);
        context.user_requests = context
            .user_requests
            .into_iter()
            .rev()
            .take(3)
            .map(|message| bounded_text(message, 350))
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        context.sibling_panes.clear();
    }

    anyhow::ensure!(
        serde_json::to_string(&context)?.len() <= MAX_CONTEXT_CHARS,
        "model context exceeded hard limit"
    );
    Ok(context)
}

pub fn normalize_suggestion(tab: Option<String>, reason: String) -> anyhow::Result<NameSuggestion> {
    let reason = bounded_text(reason, 240);
    match tab {
        Some(label) => {
            let label = sanitize_text(label);
            anyhow::ensure!(validate_tab_label(&label), "invalid tab label: {label:?}");
            Ok(NameSuggestion {
                tab: Some(label),
                reason,
            })
        }
        None => Ok(NameSuggestion { tab: None, reason }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validates_default_and_task_labels() {
        assert!(is_default_label("3", &json!(3)));
        assert!(is_default_label("", &json!(3)));
        assert!(!is_default_label("Manual Task", &json!(3)));

        assert!(validate_tab_label("Repair Tab Ownership"));
        assert!(validate_tab_label("Optimize VAR Review"));
        assert!(!validate_tab_label("bad"));
        assert!(!validate_tab_label("This Label Has Far Too Many Words"));
        assert!(!validate_tab_label("Bad\nLabel"));
    }

    #[test]
    fn picks_agent_before_supporting_command() {
        let snap = HerdrSnapshot {
            focused_workspace_id: Some("w1".into()),
            focused_tab_id: Some("t1".into()),
            focused_pane_id: Some("server".into()),
            workspaces: vec![],
            tabs: vec![HerdrTab {
                tab_id: "t1".into(),
                workspace_id: "w1".into(),
                label: "1".into(),
                number: json!(1),
            }],
            panes: vec![
                HerdrPane {
                    pane_id: "agent".into(),
                    tab_id: "t1".into(),
                    workspace_id: "w1".into(),
                    label: None,
                    cwd: None,
                    foreground_cwd: None,
                    agent: Some("codex".into()),
                    agent_status: Some("working".into()),
                    agent_session: None,
                },
                HerdrPane {
                    pane_id: "server".into(),
                    tab_id: "t1".into(),
                    workspace_id: "w1".into(),
                    label: None,
                    cwd: None,
                    foreground_cwd: None,
                    agent: None,
                    agent_status: None,
                    agent_session: None,
                },
            ],
            layouts: vec![HerdrLayout {
                tab_id: "t1".into(),
                focused_pane_id: Some("server".into()),
            }],
        };

        assert_eq!(
            focused_pane_for(&snap.tabs[0], &snap).unwrap().pane_id,
            "agent"
        );
    }
}
