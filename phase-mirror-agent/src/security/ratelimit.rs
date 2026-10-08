use crate::security::auth::AuthContext;
use axum::{
    extract::{ConnectInfo, Request, State},
    http::{header::RETRY_AFTER, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
}

#[derive(Default)]
struct Buckets {
    by_key: HashMap<String, TokenBucket>,
}

/// Per-client token-bucket limiter (ADR-006 §2.3). Buckets are keyed by the
/// authenticated actor id when present, otherwise by peer IP.
pub struct RateLimitState {
    buckets: Mutex<Buckets>,
    rps: f64,
    burst: f64,
}

impl RateLimitState {
    pub fn new(rps: u32, burst: u32) -> Self {
        Self {
            buckets: Mutex::new(Buckets::default()),
            rps: f64::from(rps.max(1)),
            burst: f64::from(burst.max(1)),
        }
    }

    pub fn take(&self, key: &str) -> Result<(), Duration> {
        let mut buckets = self.buckets.lock().unwrap();
        let now = Instant::now();
        let bucket = buckets
            .by_key
            .entry(key.to_string())
            .or_insert_with(|| TokenBucket {
                tokens: self.burst,
                last_refill: now,
            });
        let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * self.rps).min(self.burst);
        bucket.last_refill = now;
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            Ok(())
        } else {
            Err(Duration::from_secs_f64((1.0 / self.rps).ceil()))
        }
    }

    fn bucket_key(req: &Request) -> String {
        if let Some(auth) = req.extensions().get::<AuthContext>() {
            return format!("actor:{}", auth.actor);
        }
        if let Some(ci) = req.extensions().get::<ConnectInfo<SocketAddr>>() {
            return format!("ip:{}", ci.0.ip());
        }
        "ip:unknown".to_string()
    }
}

/// Rate-limit middleware. Runs after `authenticate` so the actor identity is
/// available for per-client buckets; unauthenticated requests are rejected by
/// authn before reaching this layer.
pub async fn rate_limit(
    State(state): State<Arc<RateLimitState>>,
    req: Request,
    next: Next,
) -> Response {
    let key = RateLimitState::bucket_key(&req);
    match state.take(&key) {
        Ok(()) => next.run(req).await,
        Err(wait) => {
            let retry_after = wait.as_secs().max(1);
            tracing::debug!(key = %key, %retry_after, "rate limit exceeded");
            let mut res = StatusCode::TOO_MANY_REQUESTS.into_response();
            if let Ok(value) = retry_after.to_string().parse::<axum::http::HeaderValue>() {
                res.headers_mut().insert(RETRY_AFTER, value);
            }
            res
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucket_allows_burst_then_rejects() {
        let limiter = RateLimitState::new(1, 2);
        assert!(limiter.take("k").is_ok());
        assert!(limiter.take("k").is_ok());
        let err = limiter.take("k").expect_err("burst exhausted");
        assert_eq!(err.as_secs(), 1);
        std::thread::sleep(Duration::from_millis(1100));
        assert!(limiter.take("k").is_ok(), "bucket refills at 1 rps");
    }

    #[test]
    fn buckets_are_independent_per_key() {
        let limiter = RateLimitState::new(1, 1);
        assert!(limiter.take("actor:alice").is_ok());
        assert!(limiter.take("actor:alice").is_err());
        assert!(limiter.take("actor:bob").is_ok());
        assert!(limiter.take("ip:127.0.0.1").is_ok());
    }
}
