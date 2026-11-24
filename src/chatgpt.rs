use eventsource_stream::{EventStreamError, Eventsource};
use futures_util::StreamExt;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header, jwk::JwkSet};
use reqwest::{Client, Response, StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

pub const RESOURCE: &str = "https://api.openai.com/v1";
pub const ISSUER: &str = "https://auth.openai.com";
pub const AUTHORIZATION: &str = "https://auth.openai.com/api/accounts/authorize";
pub const DYNAMIC_CLIENT: &str = "dynamic_agent_client";
pub const SCOPES: &str =
    "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct";

const TOKEN_URL: &str = "https://auth.openai.com/api/accounts/oauth/token";
const JWKS_URL: &str = "https://auth.openai.com/.well-known/jwks.json";

pub struct Registration {
    pub client_id: String,
    pub subject: String,
    pub access: Option<PlanAccess>,
    pub models: Vec<Model>,
}

#[derive(Clone)]
pub struct PlanAccess {
    pub token: String,
    pub expires_at: Instant,
}

#[derive(Deserialize, Serialize)]
pub struct Model {
    pub slug: String,
    pub display_name: String,
    #[serde(skip_serializing)]
    visibility: String,
}

#[derive(Deserialize)]
struct Catalog {
    models: Vec<Model>,
}

#[derive(Deserialize)]
struct Tokens {
    id_token: String,
    access_token: Option<String>,
    token_type: Option<String>,
    expires_in: Option<u64>,
    #[serde(default)]
    scope: String,
}

#[derive(Deserialize)]
struct Claims {
    sub: String,
    nonce: String,
}

use crate::{Analysis, AnalysisMode, Citation, analysis_instructions};

pub async fn exchange(
    client: &Client,
    client_id: &str,
    code: &str,
    verifier: &str,
    nonce: &str,
    callback: &str,
    verifying_identity: impl FnOnce(),
) -> Result<Registration, &'static str> {
    let tokens = client
        .post(TOKEN_URL)
        .timeout(Duration::from_secs(12))
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", client_id),
            ("code", code),
            ("code_verifier", verifier),
            ("redirect_uri", callback),
            ("resource", RESOURCE),
        ])
        .send()
        .await
        .and_then(Response::error_for_status)
        .map_err(|_| "exchange")?
        .json::<Tokens>()
        .await
        .map_err(|_| "exchange")?;

    verifying_identity();
    let header = decode_header(&tokens.id_token).map_err(|_| "identity")?;
    if header.alg != Algorithm::RS256 {
        return Err("identity");
    }
    let keys = client
        .get(JWKS_URL)
        .timeout(Duration::from_secs(12))
        .send()
        .await
        .and_then(Response::error_for_status)
        .map_err(|_| "identity")?
        .json::<JwkSet>()
        .await
        .map_err(|_| "identity")?;
    let key = header
        .kid
        .as_deref()
        .and_then(|kid| keys.find(kid))
        .ok_or("identity")?;
    let key = DecodingKey::from_jwk(key).map_err(|_| "identity")?;
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&[ISSUER]);
    validation.set_audience(&[client_id]);
    validation.set_required_spec_claims(&["iss", "aud", "exp", "sub"]);
    validation.leeway = 5;
    let claims = decode::<Claims>(&tokens.id_token, &key, &validation)
        .map_err(|_| "identity")?
        .claims;
    if claims.nonce != nonce || claims.sub.is_empty() {
        return Err("identity");
    }

    let access = if grants_plan_usage(&tokens.scope) {
        let token = tokens
            .access_token
            .filter(|token| !token.is_empty())
            .ok_or("exchange")?;
        let bearer = tokens
            .token_type
            .is_some_and(|kind| kind.eq_ignore_ascii_case("bearer"));
        if !bearer {
            return Err("exchange");
        }
        let duration = Duration::from_secs(
            tokens
                .expires_in
                .filter(|value| *value > 0)
                .ok_or("exchange")?,
        );
        let expires_at = Instant::now().checked_add(duration).ok_or("exchange")?;
        Some(PlanAccess { token, expires_at })
    } else {
        None
    };

    Ok(Registration {
        client_id: client_id.to_owned(),
        subject: claims.sub,
        access,
        models: Vec::new(),
    })
}

fn grants_plan_usage(scopes: &str) -> bool {
    scopes
        .split_ascii_whitespace()
        .any(|scope| scope == "chatgpt.tokens.use.direct")
        && scopes
            .split_ascii_whitespace()
            .any(|scope| scope == "resource.invoke")
}

pub async fn list_models(client: &Client, token: &str) -> Result<Vec<Model>, StatusCode> {
    let response = client
        .get(format!("{RESOURCE}/models"))
        .timeout(Duration::from_secs(12))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    if !response.status().is_success() {
        return Err(response.status());
    }
    let catalog = response
        .json::<Catalog>()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    Ok(catalog
        .models
        .into_iter()
        .filter(|model| model.visibility == "list")
        .collect())
}

pub async fn analyze(
    client: &Client,
    token: &str,
    model: &str,
    text: &str,
    mode: AnalysisMode,
    mut report: impl FnMut(&'static str),
) -> Result<Analysis, (StatusCode, &'static str)> {
    let response = client
        .post(format!("{RESOURCE}/responses"))
        .bearer_auth(token)
        .json(&analysis_request(model, text, mode))
        .send()
        .await
        .map_err(|_| {
            (
                StatusCode::BAD_GATEWAY,
                "OpenAI did not respond. Check plan usage before retrying.",
            )
        })?;

    let request_id = ["x-request-id", "openai-request-id"]
        .iter()
        .find_map(|name| response.headers().get(*name)?.to_str().ok())
        .filter(|id| diagnostic_value(id))
        .unwrap_or("unavailable")
        .to_owned();
    let status = response.status();
    if !status.is_success() {
        let body = response.json::<Value>().await.unwrap_or(Value::Null);
        let code = provider_code(&body);
        log_failure(&request_id, "http", code);
        return Err(provider_failure(code, status, mode));
    }

    let mut events = response.bytes_stream().eventsource();
    let mut progress = 0;
    let mut streamed = StreamedResponse::default();
    while let Some(event) = events.next().await {
        let event = event.map_err(|error| {
            if matches!(&error, EventStreamError::Transport(error) if error.is_timeout()) {
                log_failure(&request_id, "stream-timeout", None);
                (StatusCode::GATEWAY_TIMEOUT, "OpenAI timed out while generating a response. Check usage before retrying.")
            } else {
                log_failure(&request_id, "stream-interrupted", None);
                (StatusCode::BAD_GATEWAY, "The connection to OpenAI was interrupted before the response finished. Check usage before retrying.")
            }
        })?;
        let data = serde_json::from_str::<Value>(&event.data).unwrap_or(Value::Null);
        let kind = response_event_type(&event.event, &data);
        streamed.record(kind, &data);
        if let Some((rank, stage)) = analysis_stage(kind) {
            if rank > progress {
                progress = rank;
                report(stage);
            }
        }
        match kind {
            "response.completed" => {
                let stream_parts = streamed.parts.len();
                let Some(result) = streamed.finish(&data["response"], model) else {
                    log_failure(&request_id, "completed-without-answer", None);
                    eprintln!(
                        "OpenAI response shape: streamed_text_parts={stream_parts} final_output_items={}",
                        data["response"]["output"].as_array().map_or(0, Vec::len)
                    );
                    return Err((
                        StatusCode::BAD_GATEWAY,
                        "OpenAI completed the request without a usable answer.",
                    ));
                };
                if mode == AnalysisMode::WebSearch && !result.searched {
                    log_failure(&request_id, "search-not-completed", None);
                    return Err((
                        StatusCode::BAD_GATEWAY,
                        "OpenAI returned no completed web search. Try another available model.",
                    ));
                }
                return Ok(result);
            }
            "error" | "response.failed" => {
                let code = provider_code(&data);
                log_failure(&request_id, kind, code);
                return Err(provider_failure(code, StatusCode::BAD_GATEWAY, mode));
            }
            "response.incomplete" => {
                let reason = data["response"]["incomplete_details"]["reason"].as_str();
                log_failure(&request_id, kind, reason);
                return Err(incomplete_failure(reason));
            }
            _ => {}
        }
    }
    log_failure(&request_id, "missing-terminal-event", None);
    Err((
        StatusCode::BAD_GATEWAY,
        "The OpenAI connection ended before a final answer arrived. Check usage before retrying.",
    ))
}

fn response_event_type<'a>(name: &'a str, data: &'a Value) -> &'a str {
    let name_is_default = matches!(name, "" | "message");
    if name_is_default {
        data["type"].as_str().unwrap_or(name)
    } else {
        name
    }
}

fn provider_code(body: &Value) -> Option<&str> {
    body["response"]["error"]["code"]
        .as_str()
        .or_else(|| body["error"]["code"].as_str())
        .or_else(|| body["code"].as_str())
}

fn diagnostic_value(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn log_failure(request_id: &str, stage: &str, code: Option<&str>) {
    // Record identifiers and machine codes only. Error bodies may include private input.
    let code = code
        .filter(|code| diagnostic_value(code))
        .unwrap_or("unavailable");
    eprintln!("OpenAI response failure: stage={stage} code={code} request_id={request_id}");
}

fn provider_failure(
    code: Option<&str>,
    status: StatusCode,
    mode: AnalysisMode,
) -> (StatusCode, &'static str) {
    match (code, status, mode) {
        (Some("subscription_sharing_usage_limit_exceeded" | "rate_limit_exceeded"), _, _)
        | (_, StatusCode::TOO_MANY_REQUESTS, _) => (
            StatusCode::TOO_MANY_REQUESTS,
            "ChatGPT reported a usage limit. Open Manage ChatGPT usage before retrying.",
        ),
        (Some("subscription_sharing_usage_unavailable"), _, _)
        | (_, StatusCode::SERVICE_UNAVAILABLE, _) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "ChatGPT plan usage is temporarily unavailable. Try again later.",
        ),
        (Some("subscription_sharing_user_not_eligible"), _, _) | (_, StatusCode::FORBIDDEN, _) => (
            StatusCode::FORBIDDEN,
            "ChatGPT plan usage is unavailable for this account or workspace.",
        ),
        (Some("subscription_sharing_invalid_user" | "invalid_api_key"), _, _)
        | (_, StatusCode::UNAUTHORIZED, _) => (
            StatusCode::UNAUTHORIZED,
            "ChatGPT access expired or was revoked. Sign in again.",
        ),
        (_, StatusCode::PAYMENT_REQUIRED, _) => (
            StatusCode::PAYMENT_REQUIRED,
            "Check your available ChatGPT plan usage and credits.",
        ),
        (
            Some(
                "subscription_sharing_unsupported_capability"
                | "unsupported_tool"
                | "model_not_found",
            ),
            _,
            AnalysisMode::WebSearch,
        )
        | (_, StatusCode::BAD_REQUEST, AnalysisMode::WebSearch) => (
            StatusCode::BAD_REQUEST,
            "OpenAI rejected web search for this request. Try another available model or check workspace settings.",
        ),
        (_, StatusCode::BAD_REQUEST, AnalysisMode::ReviewTriage) => (
            StatusCode::BAD_REQUEST,
            "OpenAI rejected the structured review request. Try another available model.",
        ),
        (_, StatusCode::BAD_REQUEST, AnalysisMode::Summarize) => (
            StatusCode::BAD_REQUEST,
            "OpenAI rejected the analysis request. Check the input and model.",
        ),
        _ => (
            status,
            "OpenAI reported an error before completing the response. Check plan usage before retrying.",
        ),
    }
}

fn incomplete_failure(reason: Option<&str>) -> (StatusCode, &'static str) {
    match reason {
        Some("content_filter") => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "OpenAI stopped the response due to its content filter.",
        ),
        Some("max_output_tokens") => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "OpenAI reached its output limit. Try a narrower question.",
        ),
        _ => (
            StatusCode::BAD_GATEWAY,
            "OpenAI marked the response as incomplete. Check usage before retrying.",
        ),
    }
}

fn analysis_request(model: &str, text: &str, mode: AnalysisMode) -> serde_json::Value {
    let instructions = analysis_instructions(mode);
    let mut request = json!({
        "model": model,
        "input": [
            { "role": "developer", "content": instructions },
            { "role": "user", "content": text }
        ],
        "store": false,
        "stream": true
    });
    if mode == AnalysisMode::WebSearch {
        request["tools"] = json!([{ "type": "web_search" }]);
        request["tool_choice"] = json!({ "type": "web_search" });
    }
    if mode == AnalysisMode::ReviewTriage {
        request["text"] = json!({ "format": crate::review::response_format() });
    }
    request
}

fn analysis_stage(event: &str) -> Option<(u8, &'static str)> {
    match event {
        "response.created" => Some((1, "received")),
        "response.in_progress" => Some((2, "processing")),
        "response.web_search_call.in_progress" | "response.web_search_call.searching" => {
            Some((3, "searching"))
        }
        "response.web_search_call.completed" => Some((4, "search-complete")),
        "response.output_text.delta" => Some((5, "writing")),
        _ => None,
    }
}

#[derive(Default)]
struct StreamedPart {
    text: String,
    annotations: Vec<Value>,
}

#[derive(Default)]
struct StreamedResponse {
    parts: BTreeMap<(u64, u64), StreamedPart>,
    response_id: Option<String>,
    model: Option<String>,
    searched: bool,
}

impl StreamedResponse {
    fn part(&mut self, event: &Value) -> &mut StreamedPart {
        self.parts
            .entry((
                event["output_index"].as_u64().unwrap_or(0),
                event["content_index"].as_u64().unwrap_or(0),
            ))
            .or_default()
    }

    fn record(&mut self, kind: &str, event: &Value) {
        let metadata = &event["response"];
        if self.response_id.is_none() {
            self.response_id = metadata["id"].as_str().map(str::to_owned);
        }
        if self.model.is_none() {
            self.model = metadata["model"].as_str().map(str::to_owned);
        }
        match kind {
            "response.web_search_call.completed" => self.searched = true,
            "response.output_text.delta" => {
                if let Some(delta) = event["delta"].as_str() {
                    self.part(event).text.push_str(delta);
                }
            }
            "response.output_text.done" => {
                if let Some(text) = event["text"].as_str().filter(|text| !text.is_empty()) {
                    self.part(event).text = text.to_owned();
                }
            }
            "response.output_text.annotation.added" => {
                let annotation = &event["annotation"];
                if annotation["type"] == "url_citation" {
                    let annotations = &mut self.part(event).annotations;
                    if !annotations.contains(annotation) {
                        annotations.push(annotation.clone());
                    }
                }
            }
            "response.content_part.done" => {
                let content = &event["part"];
                if content["type"] == "output_text" {
                    apply_final_part(self.part(event), content);
                }
            }
            "response.output_item.done" => {
                let item = &event["item"];
                self.searched |= completed_search(item);
                if item["type"] == "message"
                    && let Some(contents) = item["content"].as_array()
                {
                    let output_index = event["output_index"].as_u64().unwrap_or(0);
                    for (index, content) in contents.iter().enumerate() {
                        if content["type"] == "output_text" {
                            let part = self.parts.entry((output_index, index as u64)).or_default();
                            apply_final_part(part, content);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn finish(self, response: &Value, requested_model: &str) -> Option<Analysis> {
        let mut final_answer = AnswerText::default();
        let mut streamed_answer = AnswerText::default();
        let mut searched = self.searched;
        for item in json_array(&response["output"]) {
            searched |= completed_search(item);
            if item["type"] == "message" {
                for content in json_array(&item["content"]) {
                    if content["type"] == "output_text"
                        && let Some(text) = content["text"].as_str()
                    {
                        final_answer.append(text, json_array(&content["annotations"]));
                    }
                }
            }
        }
        for part in self.parts.into_values() {
            streamed_answer.append(&part.text, &part.annotations);
        }
        let answer = if final_answer.text.trim().is_empty() {
            streamed_answer
        } else {
            if final_answer.citations.is_empty() && final_answer.text == streamed_answer.text {
                final_answer.citations = streamed_answer.citations;
            }
            final_answer
        };
        if answer.text.trim().is_empty() {
            return None;
        }
        Some(Analysis {
            text: answer.text,
            citations: answer.citations,
            response_id: response["id"]
                .as_str()
                .map(str::to_owned)
                .or(self.response_id),
            model: response["model"]
                .as_str()
                .map(str::to_owned)
                .or(self.model)
                .unwrap_or_else(|| requested_model.to_owned()),
            searched,
        })
    }
}

fn apply_final_part(part: &mut StreamedPart, content: &Value) {
    if let Some(text) = content["text"].as_str().filter(|text| !text.is_empty()) {
        part.text = text.to_owned();
    }
    let annotations = json_array(&content["annotations"]);
    if !annotations.is_empty() {
        part.annotations = annotations.to_vec();
    }
}

fn completed_search(item: &Value) -> bool {
    item["type"] == "web_search_call" && item["status"] == "completed"
}

fn json_array(value: &Value) -> &[Value] {
    value.as_array().map(Vec::as_slice).unwrap_or_default()
}

#[derive(Default)]
struct AnswerText {
    text: String,
    citations: Vec<Citation>,
}

impl AnswerText {
    fn append(&mut self, part: &str, annotations: &[Value]) {
        if part.is_empty() {
            return;
        }
        if !self.text.is_empty() {
            self.text.push('\n');
        }
        let offset = self.text.chars().count();
        let part_len = part.chars().count();
        self.citations.extend(
            annotations
                .iter()
                .filter_map(|annotation| citation(annotation, offset, part_len)),
        );
        self.text.push_str(part);
    }
}

fn citation(annotation: &serde_json::Value, offset: usize, part_len: usize) -> Option<Citation> {
    if annotation["type"] != "url_citation" {
        return None;
    }
    let url = Url::parse(annotation["url"].as_str()?).ok()?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    let end = annotation["end_index"]
        .as_u64()
        .and_then(|index| usize::try_from(index).ok())
        .unwrap_or(part_len)
        .min(part_len);
    let title = annotation["title"]
        .as_str()
        .filter(|title| !title.trim().is_empty())
        .unwrap_or(url.host_str().unwrap_or("Source"))
        .to_owned();
    Some(Citation {
        url: url.to_string(),
        title,
        end_index: offset + end,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_scopes_do_not_authorize_plan_usage() {
        assert!(!grants_plan_usage("openid profile email"));
        assert!(!grants_plan_usage(
            "resource.invoke chatgpt.tokens.use.direct.other"
        ));
        assert!(grants_plan_usage(SCOPES));
    }

    #[test]
    fn progress_only_uses_public_response_events() {
        assert_eq!(analysis_stage("response.created"), Some((1, "received")));
        assert_eq!(
            analysis_stage("response.in_progress"),
            Some((2, "processing"))
        );
        assert_eq!(
            analysis_stage("response.output_text.delta"),
            Some((5, "writing"))
        );
        assert_eq!(
            analysis_stage("response.reasoning_summary_text.delta"),
            None
        );
    }

    #[test]
    fn web_search_is_required_only_in_search_mode() {
        let request = analysis_request("test-model", "question", AnalysisMode::WebSearch);
        assert_eq!(request["tool_choice"]["type"], "web_search");
        assert_eq!(request["tools"][0]["type"], "web_search");
        let summary = analysis_request("test-model", "text", AnalysisMode::Summarize);
        assert!(summary.get("tools").is_none());
        assert!(summary.get("tool_choice").is_none());
        assert_eq!(
            analysis_stage("response.web_search_call.searching"),
            Some((3, "searching"))
        );
    }

    #[test]
    fn reads_search_citations_and_filters_unsafe_links() {
        let data = r#"{"response":{"id":"resp_example","model":"test-model","output":[{"type":"web_search_call","status":"completed"},{"type":"message","content":[{"type":"output_text","text":"สรุป","annotations":[{"type":"url_citation","url":"https://example.org/page","title":"Page","end_index":4},{"type":"url_citation","url":"javascript:alert(1)","end_index":4}]}]}]}}"#;
        let event: Value = serde_json::from_str(data).unwrap();
        let result = StreamedResponse::default()
            .finish(&event["response"], "fallback")
            .unwrap();
        assert!(result.searched);
        assert_eq!(result.citations.len(), 1);
        assert_eq!(result.citations[0].end_index, 4);
        assert_eq!(result.citations[0].url, "https://example.org/page");
    }

    #[test]
    fn reads_event_type_from_data_only_streams() {
        let data = json!({ "type": "response.completed" });
        assert_eq!(response_event_type("message", &data), "response.completed");
        assert_eq!(
            response_event_type("response.failed", &data),
            "response.failed"
        );
        assert_eq!(response_event_type("message", &Value::Null), "message");
    }

    #[test]
    fn maps_terminal_provider_errors() {
        let failure = json!({ "response": { "error": { "code": "subscription_sharing_usage_limit_exceeded" } } });
        assert_eq!(
            provider_failure(
                provider_code(&failure),
                StatusCode::BAD_GATEWAY,
                AnalysisMode::WebSearch
            )
            .0,
            StatusCode::TOO_MANY_REQUESTS
        );
        let failure = json!({ "code": "subscription_sharing_unsupported_capability" });
        assert!(
            provider_failure(
                provider_code(&failure),
                StatusCode::BAD_GATEWAY,
                AnalysisMode::WebSearch
            )
            .1
            .contains("web search")
        );
        assert_eq!(
            incomplete_failure(Some("max_output_tokens")).0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert!(!diagnostic_value("user text with spaces"));
    }

    #[test]
    fn recovers_streamed_search_text_when_terminal_snapshot_has_no_output() {
        let mut stream = StreamedResponse::default();
        stream.record(
            "response.created",
            &json!({"response":{"id":"resp_stream","model":"resolved-model"}}),
        );
        stream.record("response.web_search_call.completed", &json!({}));
        stream.record(
            "response.output_text.delta",
            &json!({"output_index":1,"content_index":0,"delta":"First "}),
        );
        stream.record(
            "response.output_text.delta",
            &json!({"output_index":1,"content_index":0,"delta":"answer."}),
        );
        stream.record("response.output_text.annotation.added", &json!({
            "output_index":1,"content_index":0,
            "annotation":{"type":"url_citation","title":"Example","url":"https://example.org/source","end_index":13}
        }));
        let result = stream
            .finish(&json!({"output":[]}), "requested-model")
            .unwrap();
        assert_eq!(result.text, "First answer.");
        assert_eq!(result.response_id.as_deref(), Some("resp_stream"));
        assert_eq!(result.model, "resolved-model");
        assert_eq!(result.citations[0].end_index, 13);
        assert!(result.searched);
    }

    #[test]
    fn uses_finalized_text_without_duplicating_deltas_or_requiring_metadata() {
        let mut stream = StreamedResponse::default();
        stream.record("response.output_text.delta", &json!({"delta":"Partial"}));
        stream.record(
            "response.output_text.done",
            &json!({"text":"Complete answer."}),
        );
        let result = stream.finish(&Value::Null, "requested-model").unwrap();
        assert_eq!(result.text, "Complete answer.");
        assert_eq!(result.response_id, None);
        assert_eq!(result.model, "requested-model");
        assert!(!result.searched);
    }

    #[test]
    fn reads_completed_message_text_and_ignores_reasoning() {
        let data = r#"{"response":{"id":"resp_example","model":"test-model","output":[{"type":"reasoning","content":[{"type":"output_text","text":"ignored"}]},{"type":"message","content":[{"type":"output_text","text":"Summary."},{"type":"output_text","text":"Key point."}]}]}}"#;
        let event: Value = serde_json::from_str(data).unwrap();
        let result = StreamedResponse::default()
            .finish(&event["response"], "fallback")
            .unwrap();
        assert_eq!(result.text, "Summary.\nKey point.");
        assert_eq!(result.response_id.as_deref(), Some("resp_example"));
        assert!(
            StreamedResponse::default()
                .finish(&json!({"output":[]}), "fallback")
                .is_none()
        );
    }
}
