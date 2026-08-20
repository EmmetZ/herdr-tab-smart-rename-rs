use crate::naming::{ApiResponse, HerdrSnapshot, ProcessInfo, SnapshotResult};
use crate::text::bounded_text;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::ffi::OsStr;
use std::process::Command;

pub trait HerdrApi {
    fn snapshot(&self) -> Result<HerdrSnapshot>;
    fn rename_tab(&self, tab_id: &str, label: &str) -> Result<()>;
    fn pane_read(&self, pane_id: &str, lines: usize) -> Result<String>;
    fn pane_process(&self, pane_id: &str) -> Result<Option<ProcessInfo>>;
    fn notify(&self, title: &str, body: &str, request_sound: bool) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct HerdrCli {
    bin: String,
}

impl HerdrCli {
    pub fn from_env() -> Self {
        Self {
            bin: std::env::var("HERDR_BIN_PATH").unwrap_or_else(|_| "herdr".to_string()),
        }
    }

    fn run<I, S>(&self, args: I) -> Result<String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = Command::new(&self.bin)
            .args(args)
            .output()
            .with_context(|| format!("failed to execute {}", self.bin))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("{}", stderr.trim());
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

impl HerdrApi for HerdrCli {
    fn snapshot(&self) -> Result<HerdrSnapshot> {
        let output = self.run(["api", "snapshot"])?;
        let response: ApiResponse<SnapshotResult> =
            serde_json::from_str(&output).context("failed to parse Herdr snapshot")?;
        Ok(response.result.snapshot)
    }

    fn rename_tab(&self, tab_id: &str, label: &str) -> Result<()> {
        self.run(["tab", "rename", tab_id, label])?;
        Ok(())
    }

    fn pane_read(&self, pane_id: &str, lines: usize) -> Result<String> {
        let output = self.run([
            "pane",
            "read",
            pane_id,
            "--source",
            "recent-unwrapped",
            "--lines",
            &lines.to_string(),
        ])?;
        Ok(bounded_text(output, 2_400))
    }

    fn pane_process(&self, pane_id: &str) -> Result<Option<ProcessInfo>> {
        let output = self.run(["pane", "process-info", "--pane", pane_id])?;
        let response: ApiResponse<ProcessResult> =
            serde_json::from_str(&output).context("failed to parse pane process info")?;
        let Some(item) = response
            .result
            .process_info
            .foreground_processes
            .into_iter()
            .next()
        else {
            return Ok(None);
        };
        Ok(Some(ProcessInfo {
            name: bounded_text(item.argv0.or(item.name).unwrap_or_default(), 80),
            command: bounded_text(
                item.cmdline
                    .or_else(|| item.argv.map(|argv| argv.join(" ")))
                    .unwrap_or_default(),
                500,
            ),
            cwd: bounded_text(item.cwd.unwrap_or_default(), 200),
        }))
    }

    fn notify(&self, title: &str, body: &str, request_sound: bool) -> Result<()> {
        let mut args = vec![
            "notification".to_string(),
            "show".to_string(),
            title.to_string(),
            "--position".to_string(),
            "bottom-right".to_string(),
            "--sound".to_string(),
            if request_sound { "request" } else { "done" }.to_string(),
        ];
        let body = bounded_text(body, 120);
        if !body.is_empty() {
            args.push("--body".to_string());
            args.push(body);
        }
        let _ = self.run(args);
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct ProcessResult {
    process_info: ProcessInfoResponse,
}

#[derive(Debug, Deserialize)]
struct ProcessInfoResponse {
    #[serde(default)]
    foreground_processes: Vec<ProcessItem>,
}

#[derive(Debug, Deserialize)]
struct ProcessItem {
    #[serde(default)]
    argv0: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    cmdline: Option<String>,
    #[serde(default)]
    argv: Option<Vec<String>>,
    #[serde(default)]
    cwd: Option<String>,
}
