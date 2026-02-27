use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use std::sync::Arc;

#[derive(Debug)]
struct TokenBucketInner {
    rate: f64,           // tokens per second (0 = unlimited)
    tokens: f64,
    last_refill: Instant,
}

#[derive(Clone, Debug)]
pub struct RateLimiter {
    inner: Arc<Mutex<TokenBucketInner>>,
}

impl RateLimiter {
    pub fn new(rate_per_sec: u64) -> Self {
        Self {
            inner: Arc::new(Mutex::new(TokenBucketInner {
                rate: rate_per_sec as f64,
                tokens: rate_per_sec as f64,
                last_refill: Instant::now(),
            })),
        }
    }

    pub async fn acquire(&self) {
        let mut inner = self.inner.lock().await;

        if inner.rate <= 0.0 {
            return;  // unlimited
        }

        let now = Instant::now();
        let elapsed = now.duration_since(inner.last_refill).as_secs_f64();
        inner.tokens = (inner.tokens + elapsed * inner.rate).min(inner.rate);
        inner.last_refill = now;

        if inner.tokens < 1.0 {
            let wait_secs = (1.0 - inner.tokens) / inner.rate;
            let wait = Duration::from_secs_f64(wait_secs);
            drop(inner);  // release lock before sleeping
            tokio::time::sleep(wait).await;
            // re-acquire and consume token
            let mut inner = self.inner.lock().await;
            inner.tokens = 0.0;
        } else {
            inner.tokens -= 1.0;
        }
    }
}
