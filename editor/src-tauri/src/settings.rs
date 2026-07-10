//! AI provider settings (fast-follow of milestone M18): lets the editor's
//! Settings panel pick a provider/model/API key without touching
//! environment variables, persisted as a local JSON file in the app's data
//! directory. Environment variables still work and take priority — see
//! `resolve_provider` — so the M18 docs/CI usage is unaffected; the panel
//! is just the practical everyday path now.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::ai::Provider;

fn default_provider() -> String {
    "ollama".to_string()
}
fn default_ollama_base_url() -> String {
    "http://localhost:11434".to_string()
}
fn default_ollama_model() -> String {
    "llama3.2:latest".to_string()
}
fn default_claude_model() -> String {
    "claude-sonnet-5".to_string()
}
fn default_openai_model() -> String {
    "gpt-4o".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiSettings {
    /// `"ollama"`, `"claude"` or `"openai"`.
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default = "default_ollama_base_url")]
    pub ollama_base_url: String,
    #[serde(default = "default_ollama_model")]
    pub ollama_model: String,
    /// Stored in plain text in a local, per-user file — the same trust
    /// level already documented as acceptable for personal use in
    /// `docs/guia-inicio.md` (putting the key in `~/.zshrc`), not an OS
    /// keychain (see `docs/arquitectura.md` for why).
    #[serde(default)]
    pub claude_api_key: String,
    #[serde(default = "default_claude_model")]
    pub claude_model: String,
    #[serde(default)]
    pub openai_api_key: String,
    #[serde(default = "default_openai_model")]
    pub openai_model: String,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            ollama_base_url: default_ollama_base_url(),
            ollama_model: default_ollama_model(),
            claude_api_key: String::new(),
            claude_model: default_claude_model(),
            openai_api_key: String::new(),
            openai_model: default_openai_model(),
        }
    }
}

fn settings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("could not resolve the app data directory: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("ai-settings.json"))
}

/// Loads settings from disk, falling back to `AiSettings::default()` if the
/// file doesn't exist yet or fails to parse — a missing/corrupt settings
/// file is never a hard error, since env vars alone are still enough to
/// run the app (M18's original path).
pub fn load_settings(app: &tauri::AppHandle) -> AiSettings {
    let Ok(path) = settings_path(app) else {
        return AiSettings::default();
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return AiSettings::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

pub fn save_settings(app: &tauri::AppHandle, settings: &AiSettings) -> Result<(), String> {
    let path = settings_path(app)?;
    let text = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| e.to_string())
}

fn env_or(env_key: &str, fallback: &str) -> String {
    std::env::var(env_key)
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

/// Given the provider choice and per-provider config already gathered
/// (each already resolved env-or-settings-or-default), builds the actual
/// `Provider`. Split out from `resolve_provider` so the part that matters
/// — mapping a provider name and its fields to the right `Provider`
/// variant, and rejecting an empty API key — is unit-testable without a
/// real Tauri `AppHandle`.
fn build_provider(
    provider_choice: &str,
    ollama_base_url: String,
    ollama_model: String,
    claude_api_key: String,
    claude_model: String,
    openai_api_key: String,
    openai_model: String,
) -> Result<Provider, String> {
    match provider_choice {
        "ollama" => Ok(Provider::Ollama {
            base_url: ollama_base_url,
            model: ollama_model,
        }),
        "claude" => {
            if claude_api_key.is_empty() {
                return Err(
                    "Claude needs an API key — set it in Ajustes or ANTHROPIC_API_KEY".to_string(),
                );
            }
            Ok(Provider::Claude {
                api_key: claude_api_key,
                model: claude_model,
            })
        }
        "openai" => {
            if openai_api_key.is_empty() {
                return Err(
                    "OpenAI needs an API key — set it in Ajustes or OPENAI_API_KEY".to_string(),
                );
            }
            Ok(Provider::OpenAi {
                api_key: openai_api_key,
                model: openai_model,
            })
        }
        other => Err(format!(
            "unknown provider \"{other}\" (expected \"ollama\", \"claude\" or \"openai\")"
        )),
    }
}

/// Resolves the provider to use for a request: an environment variable, if
/// set, always wins (keeps M18's docs/CI usage working); otherwise falls
/// back to the settings file loaded from `app`'s data directory, with
/// hardcoded defaults under that.
pub fn resolve_provider(app: &tauri::AppHandle) -> Result<Provider, String> {
    let settings = load_settings(app);
    build_provider(
        &env_or("AIGS_AI_PROVIDER", &settings.provider),
        env_or("OLLAMA_BASE_URL", &settings.ollama_base_url),
        env_or("OLLAMA_MODEL", &settings.ollama_model),
        env_or("ANTHROPIC_API_KEY", &settings.claude_api_key),
        env_or("ANTHROPIC_MODEL", &settings.claude_model),
        env_or("OPENAI_API_KEY", &settings.openai_api_key),
        env_or("OPENAI_MODEL", &settings.openai_model),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_round_trip_through_json() {
        let settings = AiSettings::default();
        let json = serde_json::to_string(&settings).unwrap();
        let parsed: AiSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(settings, parsed);
    }

    #[test]
    fn partial_json_fills_in_missing_fields_with_defaults() {
        let settings: AiSettings = serde_json::from_str(r#"{"provider":"claude"}"#).unwrap();
        assert_eq!(settings.provider, "claude");
        assert_eq!(settings.ollama_model, default_ollama_model());
        assert_eq!(settings.claude_model, default_claude_model());
    }

    #[test]
    fn build_provider_defaults_to_ollama_with_given_fields() {
        let provider = build_provider(
            "ollama",
            "http://localhost:11434".to_string(),
            "llama3.2:latest".to_string(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        )
        .unwrap();
        assert!(matches!(provider, Provider::Ollama { .. }));
    }

    #[test]
    fn build_provider_rejects_claude_without_an_api_key() {
        let error = build_provider(
            "claude",
            String::new(),
            String::new(),
            String::new(),
            "claude-sonnet-5".to_string(),
            String::new(),
            String::new(),
        )
        .unwrap_err();
        assert!(error.contains("API key"), "unexpected error: {error}");
    }

    #[test]
    fn build_provider_rejects_openai_without_an_api_key() {
        let error = build_provider(
            "openai",
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            "gpt-4o".to_string(),
        )
        .unwrap_err();
        assert!(error.contains("API key"), "unexpected error: {error}");
    }

    #[test]
    fn build_provider_accepts_claude_and_openai_with_a_key() {
        assert!(matches!(
            build_provider(
                "claude",
                String::new(),
                String::new(),
                "sk-ant-test".to_string(),
                "claude-sonnet-5".to_string(),
                String::new(),
                String::new(),
            )
            .unwrap(),
            Provider::Claude { .. }
        ));
        assert!(matches!(
            build_provider(
                "openai",
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                "sk-test".to_string(),
                "gpt-4o".to_string(),
            )
            .unwrap(),
            Provider::OpenAi { .. }
        ));
    }

    #[test]
    fn build_provider_rejects_an_unknown_provider() {
        let error = build_provider(
            "gemini",
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        )
        .unwrap_err();
        assert!(error.contains("gemini"), "unexpected error: {error}");
    }
}
