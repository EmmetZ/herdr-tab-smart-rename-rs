use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use herdr_tab_smart_rename_rs::{
    HerdrCli, OpenAiCompatibleNamer, Service, check_ai_config, ensure_provider_file_from_env,
    notify_failure,
};
use std::process::{Command as ProcessCommand, Stdio};

#[derive(Parser)]
#[command(name = "herdr-tab-smart-rename-rs")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    RenameNow,
    AgentStatusEvent,
    CheckAi,
    ConfigureAi,
    ConfigureAiEditor,
    DryRun,
}

fn main() {
    if let Err(error) = run() {
        let message = format!("{error:#}");
        let _ = notify_failure("Smart Rename failed", &message);
        eprintln!("Smart Rename: {message}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let herdr = HerdrCli::from_env();
    let namer = OpenAiCompatibleNamer::from_env();
    let service = Service::from_env(&herdr, &namer);

    match cli.command {
        Command::RenameNow => {
            let result = service
                .rename_current(true)
                .context("failed to rename current tab")?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            service.notify_result(&result);
        }
        Command::AgentStatusEvent => {
            let result = service
                .handle_agent_status_event()
                .context("failed to handle agent status event")?;
            if let Some(result) = result {
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }
        Command::CheckAi => {
            let config = check_ai_config().context("AI configuration is invalid")?;
            println!("{}/{}", config.provider, config.model);
        }
        Command::ConfigureAi => {
            herdr.open_plugin_pane("tab-smart-rename", "provider-config", "overlay")?;
        }
        Command::ConfigureAiEditor => {
            let path = ensure_provider_file_from_env()?;
            let editor = std::env::var("VISUAL")
                .or_else(|_| std::env::var("EDITOR"))
                .unwrap_or_else(|_| "vi".to_string());
            let status = ProcessCommand::new(&editor)
                .arg(&path)
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .with_context(|| format!("failed to start editor {editor}"))?;
            anyhow::ensure!(status.success(), "editor {editor} exited with {status}");
        }
        Command::DryRun => {
            let result = service
                .dry_run_current()
                .context("failed to evaluate current tab")?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
    }

    Ok(())
}
