use axum::{
    body::Body,
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use http::{HeaderMap, Method, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use std::time::Instant;

use crate::config::APP_CONFIG;

fn should_ignore_path(path: &str) -> bool {
    matches!(path, "/health" | "/health/")
}

fn filter_sensitive_data(mut value: Value) -> Value {
    if let Value::Object(ref mut map) = value {
        let sensitive_fields = [
            "password",
            "token",
            "jwt",
            "access_token",
            "refresh_token",
            "authorization",
            "auth",
            "secret",
            "key",
            "api_key",
            "apikey",
            "private_key",
            "privatekey",
            "credential",
            "credentials",
        ];

        for field in sensitive_fields {
            if map.contains_key(field) {
                map.insert(field.to_string(), Value::String("[REDACTED]".to_string()));
            }
        }
    }
    value
}

fn filter_sensitive_headers(headers: &HeaderMap) -> HeaderMap {
    let mut filtered_headers = headers.clone();

    let sensitive_headers = [
        "authorization",
        "cookie",
        "x-api-key",
        "x-auth-token",
        "bearer",
        "jwt",
        "access-token",
        "refresh-token",
    ];

    for header_name in sensitive_headers {
        if let Ok(name) = header_name.parse::<http::HeaderName>() {
            if filtered_headers.contains_key(&name) {
                filtered_headers.insert(name, "[REDACTED]".parse().unwrap());
            }
        }
    }

    filtered_headers
}

pub async fn http_logger(
    req: Request,
    next: Next,
) -> std::result::Result<impl IntoResponse, (StatusCode, String)> {
    // Log immediately to verify middleware is called (can be removed later)
    let path = req.uri().path();
    tracing::debug!("HTTP logger middleware called: {} {}", req.method(), path);

    let start_time = Instant::now();

    let method = req.method().clone();
    let uri = req.uri().clone();
    let path = uri.path();
    let version = req.version();
    let req_headers = req.headers().clone();
    let x_request_id = req_headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if should_ignore_path(path) {
        return Ok(next.run(req).await);
    }

    // Check if request is a file upload
    let is_file_upload = req_headers
        .get("content-type")
        .and_then(|ct| ct.to_str().ok())
        .map(|ct| ct.starts_with("multipart/form-data"))
        .unwrap_or(false);

    let (parts, body) = req.into_parts();
    let bytes = buffer_body("request", body).await?;
    let bytes_clone = bytes.clone();

    // Only log body if not a file upload
    let req_body = if !is_file_upload {
        let body_str = String::from_utf8_lossy(bytes_clone.as_ref());
        match serde_json::from_str::<Value>(&body_str) {
            Ok(json) => filter_sensitive_data(json),
            Err(_) => Value::Object(serde_json::Map::new()),
        }
    } else {
        Value::Object(serde_json::Map::new())
    };

    // Reconstruct request with original body
    let req = Request::from_parts(parts, Body::from(bytes));

    let mut response = next.run(req).await;

    let latency = start_time.elapsed();

    let status = response.status();
    let res_headers = response.headers().clone();

    let should_log_body = matches!(method.as_str(), "POST" | "PUT" | "PATCH");
    let res_body = if should_log_body {
        let (parts, body) = response.into_parts();
        let bytes = buffer_body("response", body).await?;
        let body_str = String::from_utf8_lossy(&bytes);
        let json_body = match serde_json::from_str::<Value>(&body_str) {
            Ok(json) => filter_sensitive_data(json),
            Err(_) => Value::Object(serde_json::Map::new()),
        };
        response = Response::from_parts(parts, Body::from(bytes));
        json_body
    } else {
        Value::Object(serde_json::Map::new())
    };

    if method == Method::OPTIONS {
        // ignore OPTIONS requests
        return Ok(response);
    }

    let filtered_req_headers = filter_sensitive_headers(&req_headers);
    let filtered_res_headers = filter_sensitive_headers(&res_headers);

    // Log HTTP request with detailed information
    // Use direct tracing::info! instead of span for better compatibility
    tracing::info!(
        method = ?method,
        uri = ?uri,
        path = %path,
        x_request_id = %x_request_id,
        version = ?version,
        req_headers = ?filtered_req_headers,
        req_body = %req_body,
        status = ?status,
        latency_ms = latency.as_millis(),
        latency_micros = latency.as_micros(),
        res_headers = ?filtered_res_headers,
        res_body = %res_body,
        app_env = %APP_CONFIG.app_env,
        "HTTP request completed"
    );

    Ok(response)
}

pub async fn buffer_body<B>(
    direction: &str,
    body: B,
) -> std::result::Result<Bytes, (StatusCode, String)>
where
    B: BodyExt,
    B::Error: std::fmt::Display,
{
    let bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(err) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("failed to read {direction} body: {err}"),
            ));
        }
    };

    Ok(bytes)
}
