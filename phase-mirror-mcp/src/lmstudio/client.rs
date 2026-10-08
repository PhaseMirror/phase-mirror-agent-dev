use crate::lmstudio::types::{
    ChatCompletionRequest, CompletionRequest, EmbeddingRequest, LmStudioCompletion,
    LmStudioEmbedding, LmStudioHealth, LmStudioModel,
};
use std::process::Stdio;
use std::time::Duration;
use thiserror::Error;
use tokio::process::Command;

#[derive(Error, Debug)]
pub enum LmStudioError {
    #[error("LM Studio server unreachable: {0}")]
    Unreachable(String),

    #[error("HTTP request failed: {0}")]
    RequestFailed(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Invalid response from LM Studio: {0}")]
    InvalidResponse(String),

    #[error("Timed out after {0}s")]
    Timeout(u64),

    #[error("curl not available on PATH")]
    CurlMissing,
}

pub struct LmStudioClient {
    pub base_url: String,
    pub default_model: String,
    pub request_timeout: Duration,
}

impl LmStudioClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        let base = base_url.into().trim_end_matches('/').to_string();
        Self {
            base_url: base.clone(),
            default_model: String::new(),
            request_timeout: Duration::from_secs(120),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    pub async fn health_check(&self) -> Result<LmStudioHealth, LmStudioError> {
        let body = self
            .curl_get(&format!("{}/models", self.base_url), None)
            .await?;

        let models_resp: super::types::ModelsResponse = serde_json::from_str(&body)
            .map_err(|e| LmStudioError::InvalidResponse(e.to_string()))?;

        let first_model = models_resp.data.first().map(|m| m.id.clone());

        Ok(LmStudioHealth {
            status: "ok".to_string(),
            model: first_model,
            backend: Some("lmstudio".to_string()),
        })
    }

    pub async fn list_models(&self) -> Result<Vec<LmStudioModel>, LmStudioError> {
        let body = self
            .curl_get(&format!("{}/models", self.base_url), None)
            .await?;

        let models_resp: super::types::ModelsResponse = serde_json::from_str(&body)
            .map_err(|e| LmStudioError::InvalidResponse(e.to_string()))?;

        Ok(models_resp.data)
    }

    pub async fn generate(
        &self,
        prompt: &str,
        params: &super::types::GenerationParams,
    ) -> Result<LmStudioCompletion, LmStudioError> {
        let model = if params.model.is_some() {
            params.model.as_deref().unwrap()
        } else if !self.default_model.is_empty() {
            self.default_model.as_str()
        } else {
            return Err(LmStudioError::ModelNotFound(
                "No model specified and no default model configured".to_string(),
            ));
        };

        let body = CompletionRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            temperature: params.temperature,
            top_p: params.top_p,
            max_tokens: params.max_tokens,
            stop: params.stop.clone(),
            stream: params.stream,
        };

        let payload = serde_json::to_string(&body)
            .map_err(|e| LmStudioError::RequestFailed(e.to_string()))?;

        let resp_body = self
            .curl_post(
                &format!("{}/completions", self.base_url),
                &payload,
                Some(self.request_timeout.as_secs()),
            )
            .await?;

        let completion: LmStudioCompletion = serde_json::from_str(&resp_body)
            .map_err(|e| LmStudioError::InvalidResponse(e.to_string()))?;

        Ok(completion)
    }

    pub async fn chat(
        &self,
        messages: &[super::types::ChatMessage],
        params: &super::types::GenerationParams,
    ) -> Result<LmStudioCompletion, LmStudioError> {
        let model = if params.model.is_some() {
            params.model.as_deref().unwrap()
        } else if !self.default_model.is_empty() {
            self.default_model.as_str()
        } else {
            return Err(LmStudioError::ModelNotFound(
                "No model specified and no default model configured".to_string(),
            ));
        };

        let body = ChatCompletionRequest {
            model: model.to_string(),
            messages: messages.to_vec(),
            temperature: params.temperature,
            top_p: params.top_p,
            max_tokens: params.max_tokens,
            stop: params.stop.clone(),
            stream: params.stream,
        };

        let payload = serde_json::to_string(&body)
            .map_err(|e| LmStudioError::RequestFailed(e.to_string()))?;

        let resp_body = self
            .curl_post(
                &format!("{}/chat/completions", self.base_url),
                &payload,
                Some(self.request_timeout.as_secs()),
            )
            .await?;

        let completion: LmStudioCompletion = serde_json::from_str(&resp_body)
            .map_err(|e| LmStudioError::InvalidResponse(e.to_string()))?;

        Ok(completion)
    }

    pub async fn embed(
        &self,
        input: &str,
        model: Option<&str>,
    ) -> Result<LmStudioEmbedding, LmStudioError> {
        let model_id = model.unwrap_or_else(|| {
            if !self.default_model.is_empty() {
                self.default_model.as_str()
            } else {
                ""
            }
        });

        if model_id.is_empty() {
            return Err(LmStudioError::ModelNotFound(
                "No model specified and no default model configured".to_string(),
            ));
        }

        let body = EmbeddingRequest {
            model: model_id.to_string(),
            input: input.to_string(),
        };

        let payload = serde_json::to_string(&body)
            .map_err(|e| LmStudioError::RequestFailed(e.to_string()))?;

        let resp_body = self
            .curl_post(
                &format!("{}/embeddings", self.base_url),
                &payload,
                Some(self.request_timeout.as_secs()),
            )
            .await?;

        let embedding: LmStudioEmbedding = serde_json::from_str(&resp_body)
            .map_err(|e| LmStudioError::InvalidResponse(e.to_string()))?;

        Ok(embedding)
    }

    async fn curl_get(
        &self,
        url: &str,
        timeout_secs: Option<u64>,
    ) -> Result<String, LmStudioError> {
        self.curl_request(url, "GET", "", timeout_secs).await
    }

    async fn curl_post(
        &self,
        url: &str,
        payload: &str,
        timeout_secs: Option<u64>,
    ) -> Result<String, LmStudioError> {
        self.curl_request(url, "POST", payload, timeout_secs).await
    }

    async fn curl_request(
        &self,
        url: &str,
        method: &str,
        payload: &str,
        timeout_secs: Option<u64>,
    ) -> Result<String, LmStudioError> {
        let timeout = timeout_secs.unwrap_or(self.request_timeout.as_secs());
        let temp_input = format!("/tmp/lmstudio_req_{}.json", std::process::id());
        let temp_output = format!("/tmp/lmstudio_res_{}.json", std::process::id());

        tokio::fs::write(&temp_input, payload)
            .await
            .map_err(|e| LmStudioError::RequestFailed(format!("write temp: {}", e)))?;

        let status = Command::new("curl")
            .arg("--silent")
            .arg("--show-error")
            .arg("--max-time")
            .arg(timeout.to_string())
            .arg("-X")
            .arg(method)
            .arg("-H")
            .arg("Content-Type: application/json")
            .arg("-d")
            .arg(&temp_input)
            .arg("-o")
            .arg(&temp_output)
            .arg(url)
            .stderr(Stdio::piped())
            .status()
            .await
            .map_err(|_| LmStudioError::CurlMissing)?;

        let _ = tokio::fs::remove_file(&temp_input).await;

        if !status.success() {
            let stderr = Command::new("cat")
                .arg(&temp_output)
                .output()
                .await
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .unwrap_or_default();

            let body_result = tokio::fs::read_to_string(&temp_output).await.ok();
            let _ = tokio::fs::remove_file(&temp_output).await;

            return Err(LmStudioError::RequestFailed(format!(
                "curl exit {}: {}",
                status.code().unwrap_or(-1),
                body_result.unwrap_or(stderr)
            )));
        }

        let body = tokio::fs::read_to_string(&temp_output)
            .await
            .map_err(|e| LmStudioError::RequestFailed(format!("read response: {}", e)))?;

        let _ = tokio::fs::remove_file(&temp_output).await;

        Ok(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_default_model_empty() {
        let client = LmStudioClient::new("http://localhost:1234/v1");
        assert!(client.default_model.is_empty());
    }

    #[test]
    fn test_client_with_default_model() {
        let client =
            LmStudioClient::new("http://localhost:1234/v1").with_default_model("llama-3-8b");
        assert_eq!(client.default_model, "llama-3-8b");
    }

    #[test]
    fn test_client_strips_trailing_slash() {
        let client = LmStudioClient::new("http://localhost:1234/v1/");
        assert_eq!(client.base_url, "http://localhost:1234/v1");
    }
}
