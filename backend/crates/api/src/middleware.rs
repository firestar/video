//! Middleware: JWT auth extraction + in-memory rate limiting.
//!
//! Rate limiting uses a small per-IP token-bucket (no external crate —
//! `govern` drags in Solana deps). Good enough for Phase 1.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use axum::extract::{Request, State};
use axum::http::header;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::state::AppState;

// ---------- Auth ----------

/// Auth middleware: validates `Authorization: Bearer <jwt>` and stashes claims.
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, Response> {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string());

    let claims = match token {
        Some(t) => auth::decode_token(&state.config.jwt_secret, &t).map_err(|e| {
            (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": {"message": e.to_string()}})),
            )
                .into_response()
        })?,
        None => {
            return Err((
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": {"message": "missing token"}})),
            )
                .into_response());
        }
    };

    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}

// ---------- Rate limiting (token bucket) ----------

struct Bucket {
    tokens: f64,
    last: Instant,
}

/// Optional auth middleware: if a valid Bearer token is present, decode
/// and stash `JwtClaims` in the request extensions. No token → continue
/// anonymously. Invalid token → 401 (fail-closed for malformed credentials).
pub async fn optional_auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, Response> {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string());

    match token {
        Some(t) => {
            let claims = auth::decode_token(&state.config.jwt_secret, &t)
                .map_err(|e| {
                    (
                        axum::http::StatusCode::UNAUTHORIZED,
                        Json(serde_json::json!({"error": {"message": e.to_string()}})),
                    )
                        .into_response()
                })?;
            req.extensions_mut().insert(claims);
        }
        None => {} // anonymous
    }

    Ok(next.run(req).await)
}

struct Limiter {
    buckets: HashMap<String, Bucket>,
    rate_per_sec: f64,
    burst: f64,
}

impl Limiter {
    fn new(rate_per_sec: f64, burst: f64) -> Self {
        Self {
            buckets: HashMap::new(),
            rate_per_sec,
            burst,
        }
    }

    /// Try to consume one token for `key`. Returns true if allowed.
    fn try_consume(&mut self, key: &str) -> bool {
        let now = Instant::now();
        let entry = self.buckets.entry(key.to_string()).or_insert(Bucket {
            tokens: self.burst,
            last: now,
        });
        let elapsed = now.duration_since(entry.last).as_secs_f64();
        entry.tokens = (entry.tokens + elapsed * self.rate_per_sec).min(self.burst);
        entry.last = now;
        if entry.tokens >= 1.0 {
            entry.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

static LIMITER: OnceLock<Mutex<Limiter>> = OnceLock::new();

fn limiter() -> &'static Mutex<Limiter> {
    // 10 req/sec sustained, burst of 50.
    LIMITER.get_or_init(|| Mutex::new(Limiter::new(10.0, 50.0)))
}

/// Extract the first client IP from x-forwarded-for or x-real-ip.
fn client_ip(req: &Request) -> String {
    for h in ["x-forwarded-for", "x-real-ip"] {
        if let Some(v) = req.headers().get(h) {
            if let Ok(s) = v.to_str() {
                return s.split(',').next().unwrap_or("").trim().to_string();
            }
        }
    }
    "local".to_string()
}

/// Apply rate limiting keyed on client IP. Returns 429 when over limit.
pub async fn rate_limit(req: Request, next: Next) -> Result<Response, Response> {
    let key = client_ip(&req);
    let allowed = limiter().lock().unwrap().try_consume(&key);
    if !allowed {
        return Err((
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({"error": {"message": "rate limited"}})),
        )
            .into_response());
    }
    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_bucket_allows_burst_then_throttles() {
        let mut lim = Limiter::new(1.0, 3.0); // 1/sec, burst 3
        assert!(lim.try_consume("a"));
        assert!(lim.try_consume("a"));
        assert!(lim.try_consume("a"));
        // Burst exhausted; next should be denied (no time elapsed).
        assert!(!lim.try_consume("a"));
    }

    #[test]
    fn separate_keys_independent() {
        let mut lim = Limiter::new(1.0, 1.0);
        assert!(lim.try_consume("a"));
        assert!(!lim.try_consume("a"));
        assert!(lim.try_consume("b"));
    }
}
