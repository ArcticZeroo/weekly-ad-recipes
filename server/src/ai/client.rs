use serde::{Deserialize, Serialize};

use crate::error::AppError;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

#[derive(Debug, Serialize)]
struct MessageRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct MessageResponse {
    content: Vec<ContentBlock>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    text: Option<String>,
}

pub struct AnthropicClient {
    http: reqwest::Client,
    api_key: String,
}

impl AnthropicClient {
    pub fn new(api_key: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            api_key,
        }
    }

    pub async fn send_message(
        &self,
        model: &str,
        max_tokens: u32,
        prompt: &str,
    ) -> Result<String, AppError> {
        if self.api_key.is_empty() {
            return Err(AppError::Ai(
                "ANTHROPIC_API_KEY not configured".to_string(),
            ));
        }

        let request = MessageRequest {
            model: model.to_string(),
            max_tokens,
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        };

        let response = self
            .http
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Ai(format!(
                "Anthropic API returned {status}: {body}"
            )));
        }

        let response: MessageResponse = response.json().await?;

        response
            .content
            .into_iter()
            .find_map(|block| block.text)
            .ok_or_else(|| AppError::Ai("No text in AI response".to_string()))
    }
}
