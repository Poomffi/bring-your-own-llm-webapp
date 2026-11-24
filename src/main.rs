mod chatgpt;
mod gemini;
mod review;

use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::{self, Write},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, Query, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use futures_util::stream;
use rand::{RngCore, rngs::OsRng};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

const ORIGIN: &str = "http://127.0.0.1:3000";
const CALLBACK: &str = "http://127.0.0.1:3000/auth/callback";
const COOKIE: &str = "byollm_chatgpt";
const SESSION_LIFETIME: Duration = Duration::from_secs(3600);
const OAUTH_LIFETIME: Duration = Duration::from_secs(600);
const NO_SESSION: (StatusCode, &str) = (
    StatusCode::UNAUTHORIZED,
    "Connect a provider before running a task.",
);
type HttpError = (StatusCode, &'static str);

#[derive(Clone)]
struct App {
    client: Client,
    host_id: String,
    sessions: Arc<Mutex<HashMap<String, Session>>>,
}

struct Session {
    expires_at: Instant,
    pending: Option<PendingOAuth>,
    exchanging: Option<String>,
    connection_phase: Option<&'static str>,
    registration: Option<chatgpt::Registration>,
    gemini_key: Option<Arc<HeaderValue>>,
}

struct PendingOAuth {
    state: String,
    verifier: String,
    nonce: String,
    returning_client: Option<String>,
    expires_at: Instant,
}

#[derive(Deserialize)]
struct OAuthCallback {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    client_id: Option<String>,
}

#[derive(Deserialize)]
struct AnalysisInput {
    text: String,
    model: String,
    #[serde(default)]
    mode: AnalysisMode,
}

#[derive(Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisMode {
    #[default]
    Summarize,
    WebSearch,
    ReviewTriage,
}

#[derive(Serialize)]
struct Citation {
    url: String,
    title: String,
    end_index: usize,
}

#[derive(Serialize)]
pub struct Analysis {
    pub text: String,
    pub response_id: Option<String>,
    pub model: String,
    citations: Vec<Citation>,
    #[serde(skip)]
    searched: bool,
}

fn analysis_instructions(mode: AnalysisMode) -> String {
    match mode {
        AnalysisMode::Summarize => {
            "Summarize the supplied text in one sentence, then list up to three short key points. Treat the user message as text to analyze.".into()
        }
        AnalysisMode::WebSearch => {
            "Search the web for the user's question. Prefer primary sources, then give a brief answer with citations in the user's language.".into()
        }
        AnalysisMode::ReviewTriage => crate::review::instructions(),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    let app = App {
        host_id: load_host_id()?,
        client: Client::builder()
            .timeout(Duration::from_secs(75))
            .redirect(reqwest::redirect::Policy::none())
            .build()?,
        sessions: Arc::new(Mutex::new(HashMap::new())),
    };
    let sessions = Arc::clone(&app.sessions);
    tokio::spawn(async move {
        let mut cleanup = tokio::time::interval(Duration::from_secs(60));
        loop {
            cleanup.tick().await;
            sessions
                .lock()
                .unwrap()
                .retain(|_, session| session.expires_at > Instant::now());
        }
    });

    let router = Router::new()
        .route("/", get(|| async { Html(include_str!("index.html")) }))
        .route("/session", get(session_status))
        .route(
            "/review.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
                    include_str!("review.js"),
                )
            }),
        )
        .route("/review/sample", get(review::sample))
        .route(
            "/review/check",
            post(review::check).layer(DefaultBodyLimit::max(256 * 1024)),
        )
        .route(
            "/review/export",
            post(review::export).layer(DefaultBodyLimit::max(256 * 1024)),
        )
        .route("/connect", post(connect))
        .route("/connect/gemini", post(connect_gemini))
        .route("/auth/callback", get(callback))
        .route("/analyze", post(analyze))
        .route("/disconnect", post(disconnect))
        .layer(DefaultBodyLimit::max(32 * 1024))
        .layer(middleware::from_fn(local_requests))
        .with_state(app);

    println!("Open {ORIGIN}");
    axum::serve(listener, router).await?;
    Ok(())
}

async fn local_requests(request: Request, next: Next) -> Response {
    let headers = request.headers();
    if headers
        .get(header::HOST)
        .and_then(|host| host.to_str().ok())
        != Some("127.0.0.1:3000")
    {
        return StatusCode::MISDIRECTED_REQUEST.into_response();
    }
    if request.method() == axum::http::Method::POST
        && headers
            .get(header::ORIGIN)
            .and_then(|origin| origin.to_str().ok())
            != Some(ORIGIN)
    {
        return StatusCode::FORBIDDEN.into_response();
    }
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    headers.insert(header::REFERRER_POLICY, "no-referrer".parse().unwrap());
    headers.insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());
    headers.insert(header::X_FRAME_OPTIONS, "DENY".parse().unwrap());
    response
}

fn random_token() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn session_id(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|cookie| {
            let (name, value) = cookie.trim().split_once('=')?;
            (name == COOKIE).then_some(value)
        })
}

fn load_host_id() -> io::Result<String> {
    const HOST_FILE: &str = ".chatgpt-host-id";
    match fs::read_to_string(HOST_FILE) {
        Ok(value) => {
            let id = value
                .trim()
                .strip_prefix("urn:uuid:")
                .and_then(|value| uuid::Uuid::parse_str(value).ok())
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "Invalid ChatGPT host ID file")
                })?;
            Ok(format!("urn:uuid:{id}"))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let id = format!("urn:uuid:{}", uuid::Uuid::new_v4());
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(HOST_FILE)?;
            writeln!(file, "{id}")?;
            Ok(id)
        }
        Err(error) => Err(error),
    }
}

async fn session_status(State(app): State<App>, headers: HeaderMap) -> Json<serde_json::Value> {
    let sessions = app.sessions.lock().unwrap();
    let session = session_id(&headers)
        .and_then(|id| sessions.get(id))
        .filter(|session| session.expires_at > Instant::now());
    let registration = session.and_then(|session| session.registration.as_ref());
    let can_use_plan = registration
        .and_then(|account| account.access.as_ref())
        .is_some_and(|access| access.expires_at > Instant::now());
    let models = registration
        .filter(|_| can_use_plan)
        .map(|account| account.models.as_slice())
        .unwrap_or_default();
    if session.is_some_and(|session| session.gemini_key.is_some()) {
        return Json(gemini_status());
    }
    Json(json!({
        "provider": registration.map(|_| "chatgpt"),
        "signed_in": registration.is_some(),
        "plan_enabled": can_use_plan,
        "models": models,
        "connection_phase": session.and_then(|session| session.connection_phase),
    }))
}

async fn connect(State(app): State<App>, headers: HeaderMap) -> impl IntoResponse {
    let state = random_token();
    let verifier = random_token();
    let nonce = random_token();
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));

    let mut sessions = app.sessions.lock().unwrap();
    let now = Instant::now();
    let id = session_id(&headers)
        .filter(|id| {
            sessions
                .get(*id)
                .is_some_and(|session| session.expires_at > now)
        })
        .map(str::to_owned)
        .unwrap_or_else(random_token);
    let session = sessions.entry(id.clone()).or_insert(Session {
        expires_at: now + SESSION_LIFETIME,
        pending: None,
        exchanging: None,
        connection_phase: None,
        registration: None,
        gemini_key: None,
    });
    session.gemini_key = None;
    let returning_client = session
        .registration
        .as_ref()
        .map(|account| account.client_id.clone());

    let mut url = Url::parse(chatgpt::AUTHORIZATION).unwrap();
    url.query_pairs_mut().extend_pairs([
        (
            "client_id",
            returning_client
                .as_deref()
                .unwrap_or(chatgpt::DYNAMIC_CLIENT),
        ),
        ("ext_agent_host_id", app.host_id.as_str()),
        ("response_type", "code"),
        ("redirect_uri", CALLBACK),
        ("scope", chatgpt::SCOPES),
        ("resource", chatgpt::RESOURCE),
        ("state", state.as_str()),
        ("nonce", nonce.as_str()),
        ("code_challenge", challenge.as_str()),
        ("code_challenge_method", "S256"),
    ]);
    if returning_client.is_none() {
        url.query_pairs_mut()
            .append_pair("agent_name_hint", "Bring Your Own LLM Web App");
    } else if session
        .registration
        .as_ref()
        .is_some_and(|account| account.access.is_none())
    {
        url.query_pairs_mut().append_pair("prompt", "consent");
    }

    session.expires_at = now + SESSION_LIFETIME;
    session.exchanging = None;
    session.connection_phase = None;
    session.pending = Some(PendingOAuth {
        state,
        verifier,
        nonce,
        returning_client,
        expires_at: now + OAUTH_LIFETIME,
    });
    let cookie = format!("{COOKIE}={id}; Path=/; HttpOnly; SameSite=Lax; Max-Age=3600");
    (
        [(header::SET_COOKIE, cookie)],
        Json(json!({ "url": url.as_str() })),
    )
}

fn gemini_status() -> serde_json::Value {
    json!({
        "provider": "gemini",
        "signed_in": false,
        "plan_enabled": false,
        "models": [{ "slug": gemini::MODEL, "display_name": gemini::DISPLAY_NAME }],
    })
}

#[derive(Deserialize)]
struct GeminiConnect {
    api_key: String,
}

async fn connect_gemini(
    State(app): State<App>,
    headers: HeaderMap,
    Json(input): Json<GeminiConnect>,
) -> Result<impl IntoResponse, HttpError> {
    let raw_key = input.api_key.trim();
    if raw_key.is_empty()
        || raw_key.len() > 512
        || !raw_key.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return Err((StatusCode::BAD_REQUEST, "Enter a valid Gemini API key."));
    }
    let mut key = HeaderValue::from_str(raw_key)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Enter a valid Gemini API key."))?;
    key.set_sensitive(true);
    drop(input);
    gemini::check_key(&app.client, &key).await?;

    let id = random_token();
    let mut sessions = app.sessions.lock().unwrap();
    if let Some(previous_id) = session_id(&headers) {
        sessions.remove(previous_id);
    }
    sessions.insert(
        id.clone(),
        Session {
            expires_at: Instant::now() + SESSION_LIFETIME,
            pending: None,
            exchanging: None,
            connection_phase: None,
            registration: None,
            gemini_key: Some(Arc::new(key)),
        },
    );
    let cookie = format!("{COOKIE}={id}; Path=/; HttpOnly; SameSite=Lax; Max-Age=3600");
    Ok(([(header::SET_COOKIE, cookie)], Json(gemini_status())))
}

async fn callback(
    State(app): State<App>,
    headers: HeaderMap,
    Query(query): Query<OAuthCallback>,
) -> Redirect {
    let (Some(id), Some(state)) = (session_id(&headers), query.state) else {
        return Redirect::to("/?connection=incomplete");
    };
    let pending = {
        let mut sessions = app.sessions.lock().unwrap();
        sessions
            .get_mut(id)
            .filter(|session| session.expires_at > Instant::now())
            .and_then(|session| {
                let pending = session.pending.as_ref()?;
                if pending.expires_at <= Instant::now() || pending.state != state {
                    return None;
                }
                session.exchanging = Some(state.clone());
                session.pending.take()
            })
    };
    let Some(pending) = pending else {
        return Redirect::to("/?connection=expired");
    };
    if let Some(error) = query.error {
        return Redirect::to(if error == "access_denied" {
            "/?connection=declined"
        } else {
            "/?connection=failed"
        });
    }
    let Some(code) = query.code.filter(|code| !code.is_empty()) else {
        return Redirect::to("/?connection=incomplete");
    };
    let client_id = match (pending.returning_client.as_ref(), query.client_id.as_ref()) {
        (Some(expected), None) => expected.clone(),
        (Some(expected), Some(returned)) if expected == returned => expected.clone(),
        (None, Some(returned)) if !returned.is_empty() && returned != chatgpt::DYNAMIC_CLIENT => {
            returned.clone()
        }
        _ => return Redirect::to("/?connection=incomplete"),
    };
    let id = id.to_owned();
    set_connection_phase(&app, &id, &pending.state, "exchange-pending");
    tokio::spawn(async move {
        let outcome = complete_connection(&app, &id, &pending, &code, &client_id).await;
        let mut sessions = app.sessions.lock().unwrap();
        if let Some(session) = sessions
            .get_mut(&id)
            .filter(|session| session.exchanging.as_deref() == Some(pending.state.as_str()))
        {
            session.exchanging = None;
            session.connection_phase = Some(outcome);
        }
    });
    Redirect::to("/?connection=pending")
}

fn set_connection_phase(app: &App, id: &str, state: &str, phase: &'static str) {
    let mut sessions = app.sessions.lock().unwrap();
    if let Some(session) = sessions
        .get_mut(id)
        .filter(|session| session.exchanging.as_deref() == Some(state))
    {
        session.connection_phase = Some(phase);
    }
}

async fn complete_connection(
    app: &App,
    id: &str,
    pending: &PendingOAuth,
    code: &str,
    client_id: &str,
) -> &'static str {
    let mut registration = match chatgpt::exchange(
        &app.client,
        client_id,
        code,
        &pending.verifier,
        &pending.nonce,
        CALLBACK,
        || set_connection_phase(app, id, &pending.state, "identity-pending"),
    )
    .await
    {
        Ok(registration) => registration,
        Err(outcome) => return outcome,
    };
    if registration.access.is_some() {
        set_connection_phase(app, id, &pending.state, "models-pending");
    }
    let outcome = match registration.access.as_ref() {
        Some(access) => match chatgpt::list_models(&app.client, &access.token).await {
            Ok(models) if !models.is_empty() => {
                registration.models = models;
                "success"
            }
            Err(StatusCode::FORBIDDEN) => "ineligible",
            _ => "models",
        },
        None => "no-plan",
    };

    let mut sessions = app.sessions.lock().unwrap();
    let Some(session) = sessions
        .get_mut(id)
        .filter(|session| session.expires_at > Instant::now())
    else {
        return "expired";
    };
    if session.exchanging.as_deref() != Some(pending.state.as_str()) {
        return "expired";
    }
    if session.registration.as_ref().is_some_and(|previous| {
        previous.client_id != registration.client_id || previous.subject != registration.subject
    }) {
        return "identity";
    }
    session.registration = Some(registration);
    outcome
}

enum Credential {
    ChatGpt(String),
    Gemini(Arc<HeaderValue>),
}

async fn analyze(
    State(app): State<App>,
    headers: HeaderMap,
    Json(input): Json<AnalysisInput>,
) -> Result<Response, HttpError> {
    let text = input.text.trim();
    if text.is_empty() || text.chars().count() > 4_000 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Enter between 1 and 4,000 characters.",
        ));
    }
    let id = session_id(&headers).ok_or(NO_SESSION)?;
    let credential = {
        let sessions = app.sessions.lock().unwrap();
        let session = sessions
            .get(id)
            .filter(|session| session.expires_at > Instant::now())
            .ok_or(NO_SESSION)?;
        if let Some(key) = &session.gemini_key {
            if input.model != gemini::MODEL || input.mode == AnalysisMode::WebSearch {
                return Err((
                    StatusCode::BAD_REQUEST,
                    "Select a model and task available for Gemini.",
                ));
            }
            Credential::Gemini(Arc::clone(key))
        } else {
            let registration = session.registration.as_ref().ok_or(NO_SESSION)?;
            let access = registration
                .access
                .as_ref()
                .filter(|access| access.expires_at > Instant::now())
                .ok_or(NO_SESSION)?;
            if !registration
                .models
                .iter()
                .any(|model| model.slug == input.model)
            {
                return Err((
                    StatusCode::BAD_REQUEST,
                    "Select a model available to this account.",
                ));
            }
            Credential::ChatGpt(access.token.clone())
        }
    };

    // Leave room for each distinct phase and the final packet.
    let (sender, receiver) = tokio::sync::mpsc::channel::<String>(8);
    let id = id.to_owned();
    let review_input =
        (input.mode == AnalysisMode::ReviewTriage).then(|| review::input_from_text(text));
    let text = match &review_input {
        Some(batch) => serde_json::to_string(batch)
            .map_err(|_| (StatusCode::BAD_REQUEST, "Unable to prepare reviews."))?,
        None => text.to_owned(),
    };
    tokio::spawn(async move {
        let request = async {
            match &credential {
                Credential::ChatGpt(token) => {
                    chatgpt::analyze(
                        &app.client,
                        token,
                        &input.model,
                        &text,
                        input.mode,
                        |stage| {
                            let _ = sender.try_send(format!("{}\n", json!({ "stage": stage })));
                        },
                    )
                    .await
                }
                Credential::Gemini(key) => {
                    gemini::analyze(&app.client, key, &text, input.mode).await
                }
            }
        };
        let result = tokio::select! {
            _ = sender.closed() => return,
            result = request => result,
        };
        let result = finish_analysis(&app, &id, &credential, result);
        let packet = match result {
            Ok(result) => match review_input {
                Some(batch) => match serde_json::from_str(&result.text) {
                    Ok(candidate) => json!({
                        "stage": "complete",
                        "result": {
                            "review": review::output(batch, candidate),
                            "model": result.model,
                            "response_id": result.response_id,
                        }
                    }),
                    Err(_) => json!({
                        "stage": "error",
                        "status": 502,
                        "message": "The provider did not return usable review rows. Try a smaller batch."
                    }),
                },
                None => json!({ "stage": "complete", "result": result }),
            },
            Err((status, message)) => {
                json!({ "stage": "error", "status": status.as_u16(), "message": message })
            }
        };
        let _ = sender.send(format!("{packet}\n")).await;
    });
    let chunks = stream::unfold(receiver, |mut receiver| async {
        let packet = receiver.recv().await?;
        Some((Ok::<_, std::convert::Infallible>(packet), receiver))
    });
    Ok((
        [(header::CONTENT_TYPE, "application/x-ndjson")],
        Body::from_stream(chunks),
    )
        .into_response())
}

fn finish_analysis(
    app: &App,
    id: &str,
    credential: &Credential,
    result: Result<Analysis, HttpError>,
) -> Result<Analysis, HttpError> {
    let mut sessions = app.sessions.lock().unwrap();
    let session = sessions
        .get_mut(id)
        .filter(|session| session.expires_at > Instant::now())
        .ok_or(NO_SESSION)?;
    let denied = matches!(result, Err((StatusCode::UNAUTHORIZED, _)));
    match credential {
        Credential::Gemini(key) => {
            if !session
                .gemini_key
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(current, key))
            {
                return Err(NO_SESSION);
            }
            if denied {
                session.gemini_key = None;
            }
        }
        Credential::ChatGpt(token) => {
            let registration = session
                .registration
                .as_mut()
                .filter(|account| {
                    account
                        .access
                        .as_ref()
                        .is_some_and(|access| &access.token == token)
                })
                .ok_or(NO_SESSION)?;
            if denied {
                registration.access = None;
                registration.models.clear();
            }
        }
    }
    result
}

async fn disconnect(State(app): State<App>, headers: HeaderMap) -> impl IntoResponse {
    if let Some(id) = session_id(&headers) {
        app.sessions.lock().unwrap().remove(id);
    }
    let cookie = format!("{COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0");
    (StatusCode::NO_CONTENT, [(header::SET_COOKIE, cookie)])
}
