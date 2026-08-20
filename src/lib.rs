mod herdr;
mod naming;
mod provider;
mod service;
mod state;
mod text;

pub use herdr::{HerdrApi, HerdrCli};
pub use naming::{
    AgentSession, HerdrPane, HerdrSnapshot, HerdrTab, HerdrWorkspace, NameSuggestion,
    NamingContext, ProcessInfo, RenameResult, is_default_label, validate_tab_label,
};
pub use provider::{
    OpenAiCompatibleNamer, ProviderConfig, check_ai_config, ensure_provider_file_from_env,
};
pub use service::Service;

pub fn notify_failure(title: &str, body: &str) -> anyhow::Result<()> {
    HerdrCli::from_env().notify(title, body, true)
}
