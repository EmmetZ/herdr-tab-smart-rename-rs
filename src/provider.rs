use crate::naming::{NameSuggestion, NamingContext, normalize_suggestion};
use crate::text::sanitize_text;
use anyhow::{Context, Result};
use dotenvy::from_read_iter;
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;
use url::Url;

const PROVIDER_EXAMPLE: &str = include_str!("../provider.env.example");
const BUNDLED_PROMPT: &str = include_str!("../docs/naming-policy.md");

pub trait Namer {
    fn suggest(&self, context: &NamingContext) -> Result<NameSuggestion>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderConfig {
    pub provider: String,
    pub base_url: String,
    pub model: String,
    pub reasoning_effort: Option<String>,
    pub timeout_ms: u64,
    pub api_key: String,
}

#[derive(Debug, Clone)]
pub struct OpenAiCompatibleNamer {
    env: HashMap<String, String>,
}

impl OpenAiCompatibleNamer {
    pub fn from_env() -> Self {
        Self {
            env: std::env::vars().collect(),
        }
    }
}

impl Namer for OpenAiCompatibleNamer {
    fn suggest(&self, context: &NamingContext) -> Result<NameSuggestion> {
        let config = load_provider_config(&self.env)?;
        let prompt = load_naming_prompt(&self.env)?;
        let client = Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .context("failed to build HTTP client")?;
        let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));
        let context_json = serde_json::to_string(context)?;
        let token_field = if config.provider == "openai" {
            "max_completion_tokens"
        } else {
            "max_tokens"
        };
        let mut body = json!({
            "model": config.model,
            "messages": [
                {"role": "system", "content": prompt},
                {"role": "user", "content": format!("Suggest one label from this sanitized context:\n{context_json}")}
            ],
            token_field: 1024
        });

        if let Some(object) = body.as_object_mut()
            && let Some(reasoning) = &config.reasoning_effort
        {
            object.insert("reasoning_effort".to_string(), json!(reasoning));
        }

        let response = client
            .post(url)
            .bearer_auth(&config.api_key)
            .json(&body)
            .send()
            .context("provider request failed")?;
        let status = response.status();
        let text = response.text().context("provider response read failed")?;
        if !status.is_success() {
            anyhow::bail!(
                "AI request failed ({}/{}): {}",
                config.provider,
                config.model,
                sanitize_text(text.replace(&config.api_key, "[redacted]"))
            );
        }
        parse_chat_response(&text)
    }
}

pub fn check_ai_config() -> Result<ProviderConfig> {
    let env: HashMap<String, String> = std::env::vars().collect();
    let config = load_provider_config(&env)?;
    let _ = load_naming_prompt(&env)?;
    Ok(config)
}

pub fn ensure_provider_file(config_dir: &Path) -> Result<PathBuf> {
    fs::create_dir_all(config_dir)
        .with_context(|| format!("failed to create {}", config_dir.display()))?;
    set_private_permissions(config_dir, 0o700)?;

    let path = config_dir.join("provider.env");
    match create_private_file(&path) {
        Ok(mut file) => {
            file.write_all(PROVIDER_EXAMPLE.as_bytes())
                .with_context(|| format!("failed to write {}", path.display()))?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => {
            return Err(error).with_context(|| format!("failed to create {}", path.display()));
        }
    }
    anyhow::ensure!(
        fs::symlink_metadata(&path)
            .with_context(|| format!("failed to inspect {}", path.display()))?
            .file_type()
            .is_file(),
        "provider configuration must be a regular file: {}",
        path.display()
    );
    set_private_permissions(&path, 0o600)?;
    Ok(path)
}

#[cfg(unix)]
fn create_private_file(path: &Path) -> std::io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;

    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}

#[cfg(not(unix))]
fn create_private_file(path: &Path) -> std::io::Result<fs::File> {
    OpenOptions::new().write(true).create_new(true).open(path)
}

pub fn ensure_provider_file_from_env() -> Result<PathBuf> {
    let config_dir = std::env::var("HERDR_PLUGIN_CONFIG_DIR")
        .context("HERDR_PLUGIN_CONFIG_DIR is required for AI configuration")?;
    ensure_provider_file(Path::new(&config_dir))
}

#[cfg(unix)]
fn set_private_permissions(path: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .with_context(|| format!("failed to set permissions on {}", path.display()))
}

#[cfg(not(unix))]
fn set_private_permissions(_path: &Path, _mode: u32) -> Result<()> {
    Ok(())
}

fn load_provider_config(env: &HashMap<String, String>) -> Result<ProviderConfig> {
    let defaults = parse_env(PROVIDER_EXAMPLE)?;
    let file_env = provider_file(env)
        .filter(|path| path.exists())
        .map(|path| fs::read_to_string(path).context("failed to read provider.env"))
        .transpose()?
        .map(|text| parse_env(&text))
        .transpose()?
        .unwrap_or_default();

    let provider = required_pick(env, &file_env, &defaults, "SMART_RENAME_PROVIDER")?;
    let base_url = required_pick(env, &file_env, &defaults, "SMART_RENAME_BASE_URL")?;
    let model = required_pick(env, &file_env, &defaults, "SMART_RENAME_MODEL")?;
    let configured_reasoning = pick(
        env,
        &file_env,
        &HashMap::new(),
        "SMART_RENAME_REASONING_EFFORT",
    );
    let reasoning_effort = configured_reasoning.or_else(|| {
        (provider
            == defaults
                .get("SMART_RENAME_PROVIDER")
                .map(String::as_str)
                .unwrap_or_default())
        .then(|| {
            defaults
                .get("SMART_RENAME_REASONING_EFFORT")
                .filter(|value| !value.trim().is_empty())
                .cloned()
        })
        .flatten()
    });
    if let Some(reasoning_effort) = &reasoning_effort {
        anyhow::ensure!(
            matches!(reasoning_effort.as_str(), "low" | "medium" | "high"),
            "SMART_RENAME_REASONING_EFFORT must be low, medium, or high"
        );
    }
    let timeout_ms = required_pick(env, &file_env, &defaults, "SMART_RENAME_TIMEOUT_MS")?
        .parse::<u64>()
        .context("SMART_RENAME_TIMEOUT_MS must be a positive integer")?;
    anyhow::ensure!(
        (1_000..=300_000).contains(&timeout_ms),
        "SMART_RENAME_TIMEOUT_MS must be 1000-300000"
    );
    let parsed = Url::parse(&base_url).context("SMART_RENAME_BASE_URL must be a URL")?;
    anyhow::ensure!(
        matches!(parsed.scheme(), "http" | "https")
            && parsed.username().is_empty()
            && parsed.password().is_none(),
        "SMART_RENAME_BASE_URL must be an HTTP(S) URL without credentials"
    );
    let api_key = api_key_for(&provider, env, &file_env);
    anyhow::ensure!(
        !api_key.trim().is_empty(),
        "AI key missing. Set SMART_RENAME_API_KEY or a provider key in provider.env"
    );

    Ok(ProviderConfig {
        provider,
        base_url: base_url.trim_end_matches('/').to_string(),
        model,
        reasoning_effort,
        timeout_ms,
        api_key,
    })
}

fn load_naming_prompt(env: &HashMap<String, String>) -> Result<String> {
    let file_env = provider_file(env)
        .filter(|path| path.exists())
        .map(|path| fs::read_to_string(path).context("failed to read provider.env"))
        .transpose()?
        .map(|text| parse_env(&text))
        .transpose()?
        .unwrap_or_default();
    if let Some(path) = pick(env, &file_env, &HashMap::new(), "SMART_RENAME_PROMPT_PATH") {
        return read_prompt(resolve_config_path(env, &path));
    }
    if let Some(config_dir) = env.get("HERDR_PLUGIN_CONFIG_DIR") {
        let private_prompt = Path::new(config_dir).join("naming-prompt.md");
        if private_prompt.exists() {
            return read_prompt(private_prompt);
        }
    }
    Ok(BUNDLED_PROMPT.trim().to_string())
}

fn read_prompt(path: PathBuf) -> Result<String> {
    let prompt = fs::read_to_string(&path)
        .with_context(|| format!("failed to read naming prompt {}", path.display()))?;
    let prompt = prompt.trim().to_string();
    anyhow::ensure!(!prompt.is_empty(), "naming prompt is empty");
    Ok(prompt)
}

fn provider_file(env: &HashMap<String, String>) -> Option<PathBuf> {
    env.get("HERDR_PLUGIN_CONFIG_DIR")
        .map(|dir| Path::new(dir).join("provider.env"))
}

fn resolve_config_path(env: &HashMap<String, String>, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        env.get("HERDR_PLUGIN_CONFIG_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
            .join(path)
    }
}

fn parse_env(input: &str) -> Result<HashMap<String, String>> {
    from_read_iter(input.as_bytes())
        .map(|item| item.map_err(anyhow::Error::from))
        .collect()
}

fn pick(
    process_env: &HashMap<String, String>,
    file_env: &HashMap<String, String>,
    defaults: &HashMap<String, String>,
    name: &str,
) -> Option<String> {
    process_env
        .get(name)
        .or_else(|| file_env.get(name))
        .or_else(|| defaults.get(name))
        .filter(|value| !value.trim().is_empty())
        .cloned()
}

fn required_pick(
    process_env: &HashMap<String, String>,
    file_env: &HashMap<String, String>,
    defaults: &HashMap<String, String>,
    name: &str,
) -> Result<String> {
    pick(process_env, file_env, defaults, name).with_context(|| format!("{name} is required"))
}

fn api_key_for(
    provider: &str,
    process_env: &HashMap<String, String>,
    file_env: &HashMap<String, String>,
) -> String {
    let provider_key = match provider {
        "openai" => Some("OPENAI_API_KEY"),
        "kimi-code" => Some("KIMI_API_KEY"),
        _ => None,
    };
    pick(
        process_env,
        file_env,
        &HashMap::new(),
        "SMART_RENAME_API_KEY",
    )
    .or_else(|| provider_key.and_then(|name| pick(process_env, file_env, &HashMap::new(), name)))
    .unwrap_or_default()
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct Message {
    content: String,
}

#[derive(Debug, Deserialize)]
struct ModelOutput {
    tab: Option<String>,
    reason: String,
}

fn parse_chat_response(text: &str) -> Result<NameSuggestion> {
    let response: ChatResponse =
        serde_json::from_str(text).context("failed to parse chat response")?;
    let content = response
        .choices
        .first()
        .map(|choice| choice.message.content.trim())
        .context("chat response contained no choices")?;
    parse_model_output(content)
}

fn parse_model_output(text: &str) -> Result<NameSuggestion> {
    let cleaned = text
        .trim()
        .strip_prefix("```json")
        .or_else(|| text.trim().strip_prefix("```"))
        .unwrap_or(text.trim())
        .trim()
        .strip_suffix("```")
        .unwrap_or_else(|| {
            text.trim()
                .strip_prefix("```json")
                .or_else(|| text.trim().strip_prefix("```"))
                .unwrap_or(text.trim())
                .trim()
        })
        .trim();
    let output: ModelOutput =
        serde_json::from_str(cleaned).context("model did not return valid JSON")?;
    normalize_suggestion(output.tab, output.reason)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn parses_model_json_and_code_fence() {
        let response = json!({
            "choices": [{
                "message": {
                    "content": "```json\n{\"tab\":\"Repair Tab Ownership\",\"reason\":\"current task\"}\n```"
                }
            }]
        });
        assert_eq!(
            parse_chat_response(&response.to_string()).unwrap(),
            NameSuggestion {
                tab: Some("repair tab ownership".into()),
                reason: "current task".into(),
            }
        );
    }

    #[test]
    fn rejects_invalid_model_label() {
        let response = json!({
            "choices": [{"message": {"content": "{\"tab\":\"bad\",\"reason\":\"bad\"}"}}]
        });
        assert!(parse_chat_response(&response.to_string()).is_err());
    }

    #[test]
    fn initializes_provider_file_without_overwriting_existing_configuration() {
        let directory = tempdir().unwrap();
        let path = ensure_provider_file(directory.path()).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), PROVIDER_EXAMPLE);

        fs::write(&path, "OPENAI_API_KEY=existing-key\n").unwrap();
        assert_eq!(ensure_provider_file(directory.path()).unwrap(), path);
        assert_eq!(
            fs::read_to_string(path).unwrap(),
            "OPENAI_API_KEY=existing-key\n"
        );
    }

    #[test]
    fn rejects_non_file_provider_configuration() {
        let directory = tempdir().unwrap();
        fs::create_dir(directory.path().join("provider.env")).unwrap();

        assert!(ensure_provider_file(directory.path()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn restores_private_permissions_on_existing_provider_file() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempdir().unwrap();
        let path = directory.path().join("provider.env");
        fs::write(&path, "OPENAI_API_KEY=existing-key\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();

        ensure_provider_file(directory.path()).unwrap();

        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[test]
    fn loads_reasoning_effort_from_provider_file() {
        let directory = tempdir().unwrap();
        fs::write(
            directory.path().join("provider.env"),
            "SMART_RENAME_API_KEY=test-key\nSMART_RENAME_REASONING_EFFORT=high\n",
        )
        .unwrap();
        let env = HashMap::from([(
            "HERDR_PLUGIN_CONFIG_DIR".to_string(),
            directory.path().display().to_string(),
        )]);

        assert_eq!(
            load_provider_config(&env)
                .unwrap()
                .reasoning_effort
                .as_deref(),
            Some("high")
        );
    }

    #[test]
    fn rejects_invalid_reasoning_effort() {
        let env = HashMap::from([
            ("SMART_RENAME_API_KEY".to_string(), "test-key".to_string()),
            (
                "SMART_RENAME_REASONING_EFFORT".to_string(),
                "maximum".to_string(),
            ),
        ]);

        assert!(load_provider_config(&env).is_err());
    }

    #[test]
    fn omits_default_reasoning_effort_for_another_provider() {
        let directory = tempdir().unwrap();
        fs::write(
            directory.path().join("provider.env"),
            "SMART_RENAME_PROVIDER=compatible\nSMART_RENAME_API_KEY=test-key\n",
        )
        .unwrap();
        let env = HashMap::from([(
            "HERDR_PLUGIN_CONFIG_DIR".to_string(),
            directory.path().display().to_string(),
        )]);

        assert_eq!(load_provider_config(&env).unwrap().reasoning_effort, None);
    }
}
