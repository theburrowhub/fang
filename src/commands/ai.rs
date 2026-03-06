//! AI integration module.
//!
//! Detects available AI providers (Claude Code CLI, Ollama, OpenAI API, Anthropic API,
//! Gemini), loads/saves configuration to `~/.config/fang/config.toml`, constructs
//! contextual prompts, and streams responses back via the internal event channel.

use crate::app::events::Event;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::UnboundedSender;

// ─── Provider types ──────────────────────────────────────────────────────────

/// How to communicate with a provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiProviderType {
    /// Invoke `claude` CLI as a subprocess (OAuth auth via keychain).
    ClaudeCli,
    /// HTTP API to a local Ollama instance.
    Ollama,
    /// OpenAI-compatible HTTP API (api.openai.com).
    OpenAiApi,
    /// Anthropic Messages HTTP API (api.anthropic.com).
    AnthropicApi,
}

impl std::fmt::Display for AiProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClaudeCli => write!(f, "claude"),
            Self::Ollama => write!(f, "ollama"),
            Self::OpenAiApi => write!(f, "openai"),
            Self::AnthropicApi => write!(f, "anthropic"),
        }
    }
}

impl AiProviderType {
    /// Parse from a string stored in config.
    pub fn from_str_config(s: &str) -> Option<Self> {
        match s {
            "claude" => Some(Self::ClaudeCli),
            "ollama" => Some(Self::Ollama),
            "openai" => Some(Self::OpenAiApi),
            "anthropic" => Some(Self::AnthropicApi),
            _ => None,
        }
    }
}

/// A detected AI provider available on the system.
#[derive(Debug, Clone)]
pub struct AiProvider {
    /// Human-readable display name (e.g. "Claude Code (claude-sonnet-4-20250514)").
    pub display_name: String,
    /// How to invoke this provider.
    pub provider_type: AiProviderType,
    /// Model identifier (e.g. "mistral:latest", "gpt-4", etc.).
    pub model: String,
    /// Optional custom endpoint URL (empty = use default).
    pub endpoint: String,
}

/// Persisted AI configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub provider: String,
    pub model: String,
    #[serde(default)]
    pub endpoint: String,
}

/// Runtime-resolved provider configuration (loaded from config or selected interactively).
#[derive(Debug, Clone)]
pub struct AiProviderConfig {
    pub provider_type: AiProviderType,
    pub model: String,
    pub endpoint: String,
}

// ─── Config persistence ──────────────────────────────────────────────────────

/// Wrapper for the TOML config file structure.
#[derive(Debug, Serialize, Deserialize, Default)]
struct ConfigFile {
    #[serde(default)]
    ai: Option<AiConfig>,
}

/// Returns the path to `~/.config/fang/config.toml`.
fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("fang").join("config.toml"))
}

/// Load AI configuration from `~/.config/fang/config.toml`.
pub fn load_config() -> Option<AiProviderConfig> {
    let path = config_path()?;
    let content = std::fs::read_to_string(&path).ok()?;
    let file: ConfigFile = toml::from_str(&content).ok()?;
    let ai = file.ai?;
    let provider_type = AiProviderType::from_str_config(&ai.provider)?;
    Some(AiProviderConfig {
        provider_type,
        model: ai.model,
        endpoint: ai.endpoint,
    })
}

/// Save AI configuration to `~/.config/fang/config.toml`.
/// Creates the directory if it doesn't exist.
pub fn save_config(config: &AiProviderConfig) -> Result<(), String> {
    let path = config_path().ok_or_else(|| "cannot determine config directory".to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("cannot create config dir: {}", e))?;
    }

    // Read existing config to preserve other sections.
    let mut file: ConfigFile = if path.exists() {
        let content = std::fs::read_to_string(&path).map_err(|e| format!("read config: {}", e))?;
        toml::from_str(&content).unwrap_or_default()
    } else {
        ConfigFile::default()
    };

    file.ai = Some(AiConfig {
        provider: config.provider_type.to_string(),
        model: config.model.clone(),
        endpoint: config.endpoint.clone(),
    });

    let toml_str = toml::to_string_pretty(&file).map_err(|e| format!("serialize: {}", e))?;
    std::fs::write(&path, toml_str).map_err(|e| format!("write config: {}", e))?;
    Ok(())
}

// ─── Provider detection ──────────────────────────────────────────────────────

/// Detect all available AI providers on the system.
///
/// Runs probes concurrently: Claude CLI auth status, Ollama HTTP check,
/// environment variable checks for OpenAI / Anthropic API keys.
pub async fn detect_providers() -> Vec<AiProvider> {
    let (claude, ollama, openai, anthropic) = tokio::join!(
        detect_claude(),
        detect_ollama(),
        detect_openai(),
        detect_anthropic(),
    );

    let mut providers = Vec::new();
    if let Some(p) = claude {
        providers.push(p);
    }
    providers.extend(ollama);
    if let Some(p) = openai {
        providers.push(p);
    }
    if let Some(p) = anthropic {
        providers.push(p);
    }
    providers
}

/// Detect Claude Code CLI.
async fn detect_claude() -> Option<AiProvider> {
    // Check binary exists
    let output = tokio::process::Command::new("claude")
        .args(["auth", "status"])
        .output()
        .await
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // claude auth status outputs JSON with { "loggedIn": true, ... }
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&stdout) {
        if val.get("loggedIn")?.as_bool()? {
            let model = val
                .get("model")
                .and_then(|m| m.as_str())
                .unwrap_or("claude-sonnet-4-20250514")
                .to_string();
            return Some(AiProvider {
                display_name: format!("Claude Code ({})", model),
                provider_type: AiProviderType::ClaudeCli,
                model,
                endpoint: String::new(),
            });
        }
    }
    None
}

/// Detect Ollama — returns one provider per available model.
async fn detect_ollama() -> Vec<AiProvider> {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let resp = match client.get("http://localhost:11434/api/tags").send().await {
        Ok(r) if r.status().is_success() => r,
        _ => return vec![],
    };

    #[derive(Deserialize)]
    struct OllamaTagsResponse {
        models: Option<Vec<OllamaModel>>,
    }
    #[derive(Deserialize)]
    struct OllamaModel {
        name: Option<String>,
    }

    let tags: OllamaTagsResponse = match resp.json().await {
        Ok(t) => t,
        Err(_) => return vec![],
    };

    tags.models
        .unwrap_or_default()
        .into_iter()
        .filter_map(|m| {
            let name = m.name?;
            Some(AiProvider {
                display_name: format!("Ollama - {}", name),
                provider_type: AiProviderType::Ollama,
                model: name,
                endpoint: "http://localhost:11434".to_string(),
            })
        })
        .collect()
}

/// Detect OpenAI API via environment variable.
async fn detect_openai() -> Option<AiProvider> {
    std::env::var("OPENAI_API_KEY").ok().map(|_| AiProvider {
        display_name: "OpenAI API (gpt-4o)".to_string(),
        provider_type: AiProviderType::OpenAiApi,
        model: "gpt-4o".to_string(),
        endpoint: "https://api.openai.com".to_string(),
    })
}

/// Detect Anthropic API via environment variable.
async fn detect_anthropic() -> Option<AiProvider> {
    std::env::var("ANTHROPIC_API_KEY").ok().map(|_| AiProvider {
        display_name: "Anthropic API (claude-sonnet-4-20250514)".to_string(),
        provider_type: AiProviderType::AnthropicApi,
        model: "claude-sonnet-4-20250514".to_string(),
        endpoint: "https://api.anthropic.com".to_string(),
    })
}

// ─── Context builder ─────────────────────────────────────────────────────────

/// Build the system context string from the current state.
///
/// Includes: current directory, directory listing, selected file metadata,
/// file content (if text and < 10 KB), and prior conversation history.
pub fn build_context(
    current_dir: &Path,
    selected_file: Option<&crate::app::state::FileEntry>,
    conversation: &[crate::app::state::AiMessage],
) -> String {
    let mut ctx = String::new();
    ctx.push_str("You are an AI assistant integrated into a terminal file explorer called fang.\n");
    ctx.push_str(&format!("Current directory: {}\n", current_dir.display()));

    // Directory listing (first 50 entries to avoid bloating the context).
    if let Ok(entries) = std::fs::read_dir(current_dir) {
        ctx.push_str("\nDirectory contents:\n");
        for (count, entry) in entries.flatten().enumerate() {
            if count >= 50 {
                ctx.push_str("  ... (truncated)\n");
                break;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            if is_dir {
                ctx.push_str(&format!("  {}/\n", name));
            } else {
                ctx.push_str(&format!("  {}\n", name));
            }
        }
    }

    if let Some(entry) = selected_file {
        ctx.push_str(&format!(
            "\nSelected file: {} ({:?}, {} bytes)\n",
            entry.name, entry.file_type, entry.size
        ));

        // Include file content if it's a small text file
        if !entry.is_dir && entry.size < 10_240 {
            if let Ok(content) = std::fs::read_to_string(&entry.path) {
                let ext = entry.extension.as_deref().unwrap_or("txt");
                ctx.push_str(&format!("\nFile content:\n```{}\n{}\n```\n", ext, content));
            }
        }
    }

    // Include prior conversation history so the AI has context of the session.
    if !conversation.is_empty() {
        ctx.push_str("\n--- Conversation history ---\n");
        for msg in conversation {
            match msg.role {
                crate::app::state::AiRole::User => {
                    ctx.push_str(&format!("User: {}\n", msg.text));
                }
                crate::app::state::AiRole::Assistant => {
                    // Truncate long assistant responses to avoid token bloat.
                    let text = if msg.text.len() > 2000 {
                        format!("{}... [truncated]", &msg.text[..2000])
                    } else {
                        msg.text.clone()
                    };
                    ctx.push_str(&format!("Assistant: {}\n", text));
                }
                crate::app::state::AiRole::Status => {}
            }
        }
        ctx.push_str("--- End of history ---\n");
    }

    ctx
}

// ─── AI invocation ───────────────────────────────────────────────────────────

/// Run an AI prompt against the configured provider.
///
/// Streams response lines as `Event::AiOutputLine` and sends `Event::AiDone`
/// when complete.
pub async fn run_ai_prompt(
    config: &AiProviderConfig,
    user_prompt: &str,
    context: &str,
    tx: UnboundedSender<Event>,
) {
    tracing::info!(
        "AI prompt: provider={}, model={}, prompt_len={}, context_len={}",
        config.provider_type,
        config.model,
        user_prompt.len(),
        context.len()
    );

    let result = match config.provider_type {
        AiProviderType::ClaudeCli => run_claude_cli(&config.model, user_prompt, context, &tx).await,
        AiProviderType::Ollama => {
            run_ollama(&config.endpoint, &config.model, user_prompt, context, &tx).await
        }
        AiProviderType::OpenAiApi => run_openai_api(&config.model, user_prompt, context, &tx).await,
        AiProviderType::AnthropicApi => {
            run_anthropic_api(&config.model, user_prompt, context, &tx).await
        }
    };

    match &result {
        Ok(()) => tracing::info!("AI prompt completed successfully"),
        Err(e) => tracing::error!("AI prompt failed: {}", e),
    }

    if let Err(e) = result {
        let _ = tx.send(Event::AiOutputLine(format!("\n[error: {}]", e)));
    }
    let _ = tx.send(Event::AiDone);
}

/// Invoke Claude Code CLI as a subprocess.
async fn run_claude_cli(
    _model: &str,
    user_prompt: &str,
    context: &str,
    tx: &UnboundedSender<Event>,
) -> Result<(), String> {
    use tokio::io::{AsyncBufReadExt, BufReader};

    let full_prompt = format!("{}\n\nUser request: {}", context, user_prompt);

    tracing::debug!(
        "Spawning claude CLI with prompt length {}",
        full_prompt.len()
    );

    let mut child = tokio::process::Command::new("claude")
        .args(["-p", "--output-format", "text", &full_prompt])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn claude: {}", e))?;

    // Also capture stderr for error reporting.
    let stderr = child.stderr.take();
    let stdout = child.stdout.take().ok_or("no stdout")?;
    let mut reader = BufReader::new(stdout).lines();

    while let Ok(Some(line)) = reader.next_line().await {
        tracing::debug!("Claude line: {}", line);
        // BufReader::lines() strips \n; we send each line with a trailing \n
        // so the AiOutputLine handler creates separate display lines.
        if tx.send(Event::AiOutputLine(format!("{}\n", line))).is_err() {
            break;
        }
    }

    // Read stderr if any
    if let Some(stderr) = stderr {
        let mut stderr_reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = stderr_reader.next_line().await {
            tracing::warn!("Claude stderr: {}", line);
        }
    }

    let status = child.wait().await.map_err(|e| format!("wait: {}", e))?;
    if !status.success() {
        return Err(format!(
            "claude exited with code {}",
            status.code().unwrap_or(-1)
        ));
    }
    Ok(())
}

/// Invoke Ollama HTTP API with streaming.
async fn run_ollama(
    endpoint: &str,
    model: &str,
    user_prompt: &str,
    context: &str,
    tx: &UnboundedSender<Event>,
) -> Result<(), String> {
    use futures::StreamExt;

    let client = reqwest::Client::new();
    let url = format!("{}/api/generate", endpoint);

    let full_prompt = format!("{}\n\nUser request: {}", context, user_prompt);

    let body = serde_json::json!({
        "model": model,
        "prompt": full_prompt,
        "stream": true,
    });

    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("ollama request: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("ollama returned {}", resp.status()));
    }

    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("stream: {}", e))?;
        let text = String::from_utf8_lossy(&chunk);

        // Ollama streams newline-delimited JSON objects.
        buffer.push_str(&text);
        while let Some(newline_pos) = buffer.find('\n') {
            let json_line = buffer[..newline_pos].to_string();
            buffer = buffer[newline_pos + 1..].to_string();

            if json_line.trim().is_empty() {
                continue;
            }

            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_line) {
                if let Some(response) = val.get("response").and_then(|r| r.as_str()) {
                    if !response.is_empty() {
                        // Ollama sends token-by-token; we accumulate and send per-line.
                        let _ = tx.send(Event::AiOutputLine(response.to_string()));
                    }
                }
                // Check if generation is done.
                if val.get("done").and_then(|d| d.as_bool()).unwrap_or(false) {
                    break;
                }
            }
        }
    }

    Ok(())
}

/// Invoke OpenAI-compatible API with streaming (SSE).
async fn run_openai_api(
    model: &str,
    user_prompt: &str,
    context: &str,
    tx: &UnboundedSender<Event>,
) -> Result<(), String> {
    use futures::StreamExt;

    let api_key =
        std::env::var("OPENAI_API_KEY").map_err(|_| "OPENAI_API_KEY not set".to_string())?;

    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": model,
        "messages": [
            { "role": "system", "content": context },
            { "role": "user", "content": user_prompt },
        ],
        "stream": true,
    });

    let resp = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("openai request: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err(format!("openai returned {} - {}", status, body_text));
    }

    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("stream: {}", e))?;
        let text = String::from_utf8_lossy(&chunk);
        buffer.push_str(&text);

        // SSE format: "data: {json}\n\n"
        while let Some(pos) = buffer.find("\n\n") {
            let event_block = buffer[..pos].to_string();
            buffer = buffer[pos + 2..].to_string();

            for line in event_block.lines() {
                let line = line.trim();
                if line == "data: [DONE]" {
                    return Ok(());
                }
                if let Some(json_str) = line.strip_prefix("data: ") {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
                        if let Some(content) = val
                            .pointer("/choices/0/delta/content")
                            .and_then(|c| c.as_str())
                        {
                            if !content.is_empty() {
                                let _ = tx.send(Event::AiOutputLine(content.to_string()));
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Invoke Anthropic Messages API with streaming (SSE).
async fn run_anthropic_api(
    model: &str,
    user_prompt: &str,
    context: &str,
    tx: &UnboundedSender<Event>,
) -> Result<(), String> {
    use futures::StreamExt;

    let api_key =
        std::env::var("ANTHROPIC_API_KEY").map_err(|_| "ANTHROPIC_API_KEY not set".to_string())?;

    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": model,
        "max_tokens": 4096,
        "system": context,
        "messages": [
            { "role": "user", "content": user_prompt },
        ],
        "stream": true,
    });

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("anthropic request: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err(format!("anthropic returned {} - {}", status, body_text));
    }

    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("stream: {}", e))?;
        let text = String::from_utf8_lossy(&chunk);
        buffer.push_str(&text);

        // SSE format: "event: ...\ndata: {json}\n\n"
        while let Some(pos) = buffer.find("\n\n") {
            let event_block = buffer[..pos].to_string();
            buffer = buffer[pos + 2..].to_string();

            for line in event_block.lines() {
                let line = line.trim();
                if let Some(json_str) = line.strip_prefix("data: ") {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
                        // content_block_delta events carry the text
                        if val.get("type").and_then(|t| t.as_str()) == Some("content_block_delta") {
                            if let Some(text) = val.pointer("/delta/text").and_then(|t| t.as_str())
                            {
                                if !text.is_empty() {
                                    let _ = tx.send(Event::AiOutputLine(text.to_string()));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_type_roundtrip() {
        for (s, expected) in [
            ("claude", AiProviderType::ClaudeCli),
            ("ollama", AiProviderType::Ollama),
            ("openai", AiProviderType::OpenAiApi),
            ("anthropic", AiProviderType::AnthropicApi),
        ] {
            let parsed = AiProviderType::from_str_config(s).unwrap();
            assert_eq!(parsed, expected);
            assert_eq!(parsed.to_string(), s);
        }
    }

    #[test]
    fn test_provider_type_unknown() {
        assert!(AiProviderType::from_str_config("unknown").is_none());
    }

    #[test]
    fn test_config_serialization() {
        let file = ConfigFile {
            ai: Some(AiConfig {
                provider: "ollama".to_string(),
                model: "mistral:latest".to_string(),
                endpoint: "http://localhost:11434".to_string(),
            }),
        };
        let toml_str = toml::to_string_pretty(&file).unwrap();
        assert!(toml_str.contains("[ai]"));
        assert!(toml_str.contains("provider = \"ollama\""));

        let parsed: ConfigFile = toml::from_str(&toml_str).unwrap();
        let ai = parsed.ai.unwrap();
        assert_eq!(ai.provider, "ollama");
        assert_eq!(ai.model, "mistral:latest");
    }

    #[test]
    fn test_build_context_no_file() {
        let ctx = build_context(Path::new("/tmp/test"), None, &[]);
        assert!(ctx.contains("Current directory: /tmp/test"));
        assert!(ctx.contains("fang"));
    }

    #[test]
    fn test_build_context_with_file() {
        use crate::fs::metadata::{FileEntry, FileType};

        let entry = FileEntry {
            name: "test.rs".to_string(),
            path: PathBuf::from("/nonexistent/test.rs"),
            is_dir: false,
            is_symlink: false,
            size: 100,
            is_executable: false,
            extension: Some("rs".to_string()),
            file_type: FileType::Code,
            modified: None,
        };
        let ctx = build_context(Path::new("/tmp"), Some(&entry), &[]);
        assert!(ctx.contains("test.rs"));
        assert!(ctx.contains("Code"));
    }

    #[test]
    fn test_build_context_with_conversation() {
        use crate::app::state::{AiMessage, AiRole};

        let conversation = vec![
            AiMessage {
                role: AiRole::User,
                text: "What is this file?".to_string(),
            },
            AiMessage {
                role: AiRole::Assistant,
                text: "It's a Rust source file.".to_string(),
            },
        ];
        let ctx = build_context(Path::new("/tmp"), None, &conversation);
        assert!(ctx.contains("Conversation history"));
        assert!(ctx.contains("What is this file?"));
        assert!(ctx.contains("It's a Rust source file."));
    }

    #[test]
    fn test_ai_provider_display() {
        let p = AiProvider {
            display_name: "Test Provider".to_string(),
            provider_type: AiProviderType::Ollama,
            model: "test:latest".to_string(),
            endpoint: "http://localhost:11434".to_string(),
        };
        assert_eq!(p.display_name, "Test Provider");
        assert_eq!(p.provider_type.to_string(), "ollama");
    }
}
