use color_eyre::eyre::Result;
use serde::Deserialize;
use serde_json::json;

const GEMINI_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent";

#[derive(Clone)]
pub struct GeminiClient {
    api_key: String,
    client: reqwest::Client,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<Candidate>>,
}

#[derive(Deserialize)]
struct Candidate {
    content: Content,
}

#[derive(Deserialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Deserialize)]
struct Part {
    text: String,
}

impl GeminiClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }

    pub async fn explain_mistake(&self, correct: &str, input: &str) -> Result<String> {
        let prompt = format!(
            "You are a Japanese tutor. The user confused '{}' with '{}'. Explain the difference in Chinese (zh-CN) briefly and give one example word.",
            correct, input
        );

        let body = json!({
            "contents": [{
                "parts": [{"text": prompt}]
            }]
        });

        let url = format!("{}?key={}", GEMINI_URL, self.api_key);

        let resp = self.client.post(&url)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let error_text = resp.text().await?;
            return Err(color_eyre::eyre::eyre!("Gemini API Error: {}", error_text));
        }

        let gemini_resp: GeminiResponse = resp.json().await?;

        if let Some(candidates) = gemini_resp.candidates {
            if let Some(first) = candidates.first() {
                if let Some(part) = first.content.parts.first() {
                    return Ok(part.text.clone());
                }
            }
        }

        Ok("无法获取解释。".to_string())
    }
}
