use std::env;
use std::sync::Arc;

use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
    Router,
};
use rand::Rng;
use reqwest::Client;
use serde::{Deserialize, Serialize};

// ========== Configuration ==========

struct Config {
    supabase_url: String,
    supabase_service_key: String,
    base_url: String,
}

struct AppState {
    config: Config,
    client: Client,
}

// ========== Models ==========

#[derive(Deserialize)]
struct ShortenRequest {
    url: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct UrlRecord {
    id: i64,
    code: String,
    original_url: String,
    clicks: i64,
    created_at: String,
}

// ========== Helpers ==========

fn generate_code() -> String {
    let chars = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::rng();
    (0..6)
        .map(|_| chars[rng.random_range(0..chars.len())] as char)
        .collect()
}

impl AppState {
    fn rest_url(&self, table: &str) -> String {
        format!("{}/rest/v1/{}", self.config.supabase_url, table)
    }

    fn headers(&self) -> reqwest::header::HeaderMap {
        let mut h = reqwest::header::HeaderMap::new();
        let key = &self.config.supabase_service_key;
        h.insert("apikey", key.parse().unwrap());
        h.insert("authorization", format!("Bearer {key}").parse().unwrap());
        h
    }
}

// ========== Error Handling ==========

enum AppError {
    NotFound,
    Internal(String),
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Internal(e.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        match self {
            AppError::NotFound => (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "URL not found" })),
            )
                .into_response(),
            AppError::Internal(msg) => {
                eprintln!("Error: {msg}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": msg })),
                )
                    .into_response()
            }
        }
    }
}

async fn decode<T: serde::de::DeserializeOwned>(
    resp: reqwest::Response,
) -> Result<T, AppError> {
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        let msg = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| v["message"].as_str().map(String::from))
            .unwrap_or_else(|| format!("Supabase error {status}: {body}"));
        return Err(AppError::Internal(msg));
    }
    resp.json::<T>().await.map_err(|e| AppError::Internal(e.to_string()))
}

// ========== Handlers ==========

async fn serve_frontend() -> Html<&'static str> {
    Html(include_str!("../public/index.html"))
}

async fn shorten_url(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ShortenRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let code = generate_code();

    let resp: Vec<UrlRecord> = decode(
        state
            .client
            .post(state.rest_url("urls"))
            .headers(state.headers())
            .header("Prefer", "return=representation")
            .json(&serde_json::json!({ "code": code, "original_url": body.url }))
            .send()
            .await?,
    )
    .await?;

    let record = resp
        .into_iter()
        .next()
        .ok_or_else(|| AppError::Internal("No record returned".into()))?;

    Ok(Json(serde_json::json!({
        "code": record.code,
        "short_url": format!("{}/{}", state.config.base_url, record.code),
        "original_url": record.original_url,
    })))
}

async fn list_urls(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<UrlRecord>>, AppError> {
    let records: Vec<UrlRecord> = decode(
    state
        .client
        .get(format!(
            "{}?select=*&order=created_at.desc",
            state.rest_url("urls")
        ))
        .headers(state.headers())
        .send()
        .await?,
)
.await?;

    Ok(Json(records))
}

async fn redirect_url(
    State(state): State<Arc<AppState>>,
    Path(code): Path<String>,
) -> Result<Redirect, AppError> {
    let records: Vec<UrlRecord> = decode(
    state
        .client
        .get(format!("{}?code=eq.{code}", state.rest_url("urls")))
        .headers(state.headers())
        .send()
        .await?,
)
.await?;

    let record = records.into_iter().next().ok_or(AppError::NotFound)?;
    let url = record.original_url.clone();

    state
        .client
        .patch(format!("{}?code=eq.{code}", state.rest_url("urls")))
        .headers(state.headers())
        .json(&serde_json::json!({ "clicks": record.clicks + 1 }))
        .send()
        .await?;

    Ok(Redirect::temporary(&url))
}

// ========== Main ==========

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let config = Config {
        supabase_url: env::var("SUPABASE_URL").expect("SUPABASE_URL must be set"),
        supabase_service_key: env::var("SUPABASE_SERVICE_KEY")
            .expect("SUPABASE_SERVICE_KEY must be set"),
        base_url: env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".into()),
    };

    let state = Arc::new(AppState {
        config,
        client: Client::new(),
    });

    let app = Router::new()
        .route("/", get(serve_frontend))
        .route("/api/shorten", post(shorten_url))
        .route("/api/urls", get(list_urls))
        .route("/{code}", get(redirect_url))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind port 3000");

    println!("Server running on http://localhost:3000");

    axum::serve(listener, app).await.expect("Server failed");
}
