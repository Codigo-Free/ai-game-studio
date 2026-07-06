//! AI Core (milestone M18): lets the editor's Chat panel answer questions
//! about the currently open project, backed by a local model (Ollama) or a
//! cloud one (Claude for now — implemented against the public Messages API
//! but not locally verified, since doing so needs the user's own API key).
//!
//! Provider/model selection is by environment variable for now
//! (`AIGS_AI_PROVIDER`, `OLLAMA_MODEL`, `ANTHROPIC_API_KEY`) — no settings
//! panel yet, see `docs/plan.md` M18.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// `"system"`, `"user"` or `"assistant"`.
    pub role: String,
    pub content: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("request to {provider} failed: {source}")]
    Request {
        provider: &'static str,
        source: reqwest::Error,
    },
    #[error("{provider} returned {status}: {body}")]
    Status {
        provider: &'static str,
        status: reqwest::StatusCode,
        body: String,
    },
    #[error("could not parse the provider's response: {0}")]
    Parse(String),
}

/// One model provider, chosen at request time from the environment. An
/// `enum` rather than `dyn Trait`: with two or three variants and async
/// calls, a trait object would need `async-trait` (boxed futures) just to
/// get the same dynamic dispatch a `match` already gives us for free.
pub enum Provider {
    Ollama { base_url: String, model: String },
    Claude { api_key: String, model: String },
}

impl Provider {
    /// Picks a provider from `AIGS_AI_PROVIDER` (`"ollama"`, the default, or
    /// `"claude"`), reading whatever other environment variables that
    /// provider needs.
    pub fn from_env() -> Result<Self, String> {
        let choice = std::env::var("AIGS_AI_PROVIDER").unwrap_or_else(|_| "ollama".to_string());
        match choice.as_str() {
            "ollama" => Ok(Provider::Ollama {
                base_url: std::env::var("OLLAMA_BASE_URL")
                    .unwrap_or_else(|_| "http://localhost:11434".to_string()),
                model: std::env::var("OLLAMA_MODEL")
                    .unwrap_or_else(|_| "llama3.2:latest".to_string()),
            }),
            "claude" => {
                let api_key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| {
                    "AIGS_AI_PROVIDER=claude needs ANTHROPIC_API_KEY set".to_string()
                })?;
                Ok(Provider::Claude {
                    api_key,
                    model: std::env::var("ANTHROPIC_MODEL")
                        .unwrap_or_else(|_| "claude-sonnet-5".to_string()),
                })
            }
            other => Err(format!(
                "unknown AIGS_AI_PROVIDER \"{other}\" (expected \"ollama\" or \"claude\")"
            )),
        }
    }

    pub async fn chat(&self, messages: &[ChatMessage]) -> Result<String, ProviderError> {
        match self {
            Provider::Ollama { base_url, model } => ollama_chat(base_url, model, messages).await,
            Provider::Claude { api_key, model } => claude_chat(api_key, model, messages).await,
        }
    }
}

#[derive(Serialize)]
struct OllamaRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaResponse {
    message: OllamaResponseMessage,
}

#[derive(Deserialize)]
struct OllamaResponseMessage {
    content: String,
}

async fn ollama_chat(
    base_url: &str,
    model: &str,
    messages: &[ChatMessage],
) -> Result<String, ProviderError> {
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{base_url}/api/chat"))
        .json(&OllamaRequest {
            model,
            messages,
            stream: false,
        })
        .send()
        .await
        .map_err(|source| ProviderError::Request {
            provider: "Ollama",
            source,
        })?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(ProviderError::Status {
            provider: "Ollama",
            status,
            body,
        });
    }
    let parsed: OllamaResponse = response
        .json()
        .await
        .map_err(|error| ProviderError::Parse(error.to_string()))?;
    Ok(parsed.message.content)
}

/// Anthropic's Messages API: the system prompt is a top-level field, not a
/// `"system"`-role message in the array.
#[derive(Serialize)]
struct ClaudeRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    system: &'a str,
    messages: Vec<ClaudeMessage<'a>>,
}

#[derive(Serialize)]
struct ClaudeMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContentBlock>,
}

#[derive(Deserialize)]
struct ClaudeContentBlock {
    text: String,
}

async fn claude_chat(
    api_key: &str,
    model: &str,
    messages: &[ChatMessage],
) -> Result<String, ProviderError> {
    let system = messages
        .iter()
        .find(|m| m.role == "system")
        .map(|m| m.content.as_str())
        .unwrap_or_default();
    let turns: Vec<ClaudeMessage> = messages
        .iter()
        .filter(|m| m.role != "system")
        .map(|m| ClaudeMessage {
            role: &m.role,
            content: &m.content,
        })
        .collect();

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&ClaudeRequest {
            model,
            max_tokens: 1024,
            system,
            messages: turns,
        })
        .send()
        .await
        .map_err(|source| ProviderError::Request {
            provider: "Claude",
            source,
        })?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(ProviderError::Status {
            provider: "Claude",
            status,
            body,
        });
    }
    let parsed: ClaudeResponse = response
        .json()
        .await
        .map_err(|error| ProviderError::Parse(error.to_string()))?;
    Ok(parsed
        .content
        .into_iter()
        .map(|block| block.text)
        .collect::<Vec<_>>()
        .join(""))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real call to a local Ollama (no mocking) — skipped, not failed, if
    /// nothing is listening on the default port, matching how the audio
    /// player degrades to a no-op without a device instead of erroring.
    #[tokio::test]
    async fn ollama_answers_a_simple_question_when_available() {
        let base_url = "http://localhost:11434".to_string();
        if reqwest::Client::new()
            .get(format!("{base_url}/api/tags"))
            .send()
            .await
            .is_err()
        {
            eprintln!("skipping: no Ollama reachable at {base_url}");
            return;
        }
        let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2:latest".to_string());
        let provider = Provider::Ollama { base_url, model };
        let answer = provider
            .chat(&[ChatMessage {
                role: "user".to_string(),
                content: "Reply with exactly the word: pong".to_string(),
            }])
            .await
            .unwrap();
        assert!(
            !answer.trim().is_empty(),
            "expected a non-empty answer, got {answer:?}"
        );
    }

    // A single test, not three: `std::env::set_var` mutates *process-wide*
    // state, and Rust runs test functions in parallel by default — three
    // separate tests each touching AIGS_AI_PROVIDER raced and flaked. One
    // test body is inherently sequential, so the scenarios can't interleave.
    #[test]
    fn from_env_reads_the_provider_and_validates_its_requirements() {
        std::env::remove_var("AIGS_AI_PROVIDER");
        assert!(matches!(
            Provider::from_env().unwrap(),
            Provider::Ollama { .. }
        ));

        std::env::set_var("AIGS_AI_PROVIDER", "not-a-real-provider");
        assert!(Provider::from_env().is_err());

        std::env::set_var("AIGS_AI_PROVIDER", "claude");
        std::env::remove_var("ANTHROPIC_API_KEY");
        assert!(Provider::from_env().is_err());

        std::env::remove_var("AIGS_AI_PROVIDER");
    }
}
