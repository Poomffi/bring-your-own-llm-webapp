use axum::http::{HeaderValue, StatusCode};
use reqwest::{Client, Response};
use serde_json::{Value, json};

use crate::{Analysis, AnalysisMode, HttpError, analysis_instructions, review};

const API: &str = "https://generativelanguage.googleapis.com/v1beta/models";
pub const MODEL: &str = "gemini-3.1-flash-lite";
pub const DISPLAY_NAME: &str = "Gemini 3.1 Flash-Lite";

pub async fn check_key(client: &Client, key: &HeaderValue) -> Result<(), HttpError> {
    let response = client
        .get(format!("{API}/{MODEL}"))
        .timeout(std::time::Duration::from_secs(15))
        .header("x-goog-api-key", key.clone())
        .send()
        .await
        .map_err(network_error)?;
    if response.status() == StatusCode::BAD_REQUEST {
        return Err((
            StatusCode::BAD_REQUEST,
            "Google rejected the Gemini API key. Check that it is a Gemini Developer API key from Google AI Studio.",
        ));
    }
    let metadata = read_response(response).await?;
    if !metadata["supportedGenerationMethods"]
        .as_array()
        .is_some_and(|methods| methods.iter().any(|method| method == "generateContent"))
    {
        return Err((
            StatusCode::BAD_GATEWAY,
            "Google did not list text generation for Gemini 3.1 Flash-Lite.",
        ));
    }
    Ok(())
}

pub async fn analyze(
    client: &Client,
    key: &HeaderValue,
    text: &str,
    mode: AnalysisMode,
) -> Result<Analysis, HttpError> {
    let mut request = json!({
        "systemInstruction": { "parts": [{ "text": analysis_instructions(mode) }] },
        "contents": [{ "role": "user", "parts": [{ "text": text }] }],
        "generationConfig": { "maxOutputTokens": 8192 },
    });
    if mode == AnalysisMode::ReviewTriage {
        request["generationConfig"]["responseMimeType"] = json!("application/json");
        request["generationConfig"]["responseJsonSchema"] =
            review::response_format()["schema"].clone();
    }
    let response = client
        .post(format!("{API}/{MODEL}:generateContent"))
        .header("x-goog-api-key", key.clone())
        .json(&request)
        .send()
        .await
        .map_err(network_error)?;
    extract_answer(&read_response(response).await?)
}

fn network_error(error: reqwest::Error) -> HttpError {
    if error.is_timeout() {
        (
            StatusCode::GATEWAY_TIMEOUT,
            "Google did not respond in time.",
        )
    } else {
        (StatusCode::BAD_GATEWAY, "Could not reach the Gemini API.")
    }
}

async fn read_response(response: Response) -> Result<Value, HttpError> {
    let status = response.status();
    if status.is_success() {
        return response.json().await.map_err(|_| {
            (
                StatusCode::BAD_GATEWAY,
                "Google returned an unreadable response.",
            )
        });
    }
    if status == StatusCode::BAD_REQUEST
        && response
            .json::<Value>()
            .await
            .ok()
            .and_then(|body| body["error"]["details"].as_array().cloned())
            .is_some_and(|details| {
                details
                    .iter()
                    .any(|detail| detail["reason"] == "API_KEY_INVALID")
            })
    {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Google rejected the Gemini API key. Connect again with a valid key.",
        ));
    }
    Err(match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => (
            StatusCode::UNAUTHORIZED,
            "Google denied access. Check the Gemini API key and its project permissions.",
        ),
        StatusCode::NOT_FOUND => (
            status,
            "Gemini 3.1 Flash-Lite is unavailable for this project.",
        ),
        StatusCode::TOO_MANY_REQUESTS => (
            status,
            "Google's rate limit or project quota was reached. Check Gemini API usage.",
        ),
        StatusCode::BAD_REQUEST => (
            status,
            "Google rejected the request. Check that the project can use the Gemini Developer API.",
        ),
        _ => (
            StatusCode::BAD_GATEWAY,
            "The Gemini API could not complete the request.",
        ),
    })
}

fn extract_answer(response: &Value) -> Result<Analysis, HttpError> {
    let candidate = &response["candidates"][0];
    match candidate["finishReason"].as_str() {
        Some("STOP") => {}
        Some("MAX_TOKENS") => {
            return Err((
                StatusCode::BAD_GATEWAY,
                "Gemini reached its output limit. Try shorter text or fewer reviews.",
            ));
        }
        _ => {
            return Err((
                StatusCode::BAD_GATEWAY,
                "Gemini did not return a complete answer.",
            ));
        }
    }
    let text = candidate["content"]["parts"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|part| part["thought"] != true)
        .filter_map(|part| part["text"].as_str())
        .collect::<String>();
    if text.trim().is_empty() {
        return Err((StatusCode::BAD_GATEWAY, "Gemini returned no usable answer."));
    }
    Ok(Analysis {
        text,
        response_id: response["responseId"].as_str().map(str::to_owned),
        model: response["modelVersion"]
            .as_str()
            .unwrap_or(MODEL)
            .to_owned(),
        citations: Vec::new(),
        searched: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_only_final_text() {
        let result = extract_answer(&json!({"responseId": "sample", "candidates": [{
            "finishReason": "STOP",
            "content": { "parts": [{ "text": "thought", "thought": true }, { "text": "answer" }] }
        }]}))
        .unwrap();
        assert_eq!(result.text, "answer");
        assert_eq!(result.response_id.as_deref(), Some("sample"));
    }

    #[test]
    fn rejects_truncated_or_missing_output() {
        assert!(extract_answer(&json!({"candidates": [{"finishReason": "MAX_TOKENS"}]})).is_err());
        assert!(extract_answer(&json!({"promptFeedback": {"blockReason": "SAFETY"}})).is_err());
    }
}
