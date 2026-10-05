//! Client-side rate limiting that mirrors GameTorch's published limits.
//!
//! GameTorch rate limits every account per route and returns `429` when a limit
//! is exceeded. To be a good citizen by default, the SDK throttles requests
//! locally before they are sent:
//!
//! * **Tier 1** — 1 request/second per route: generation creates, frame
//!   generation and animation exports.
//! * **Tier 2** — 2 requests/second per route: content, usage, search and
//!   single-item reads.
//! * **Writes** — one shared token bucket (100-request burst, 5 requests/second
//!   refill) for creation, archive/unarchive and metadata writes.
//! * **Concurrency** — at most 25 in-flight hold-creating requests and 5
//!   concurrent frame generations.
//!
//! The limiter is enabled by default and can be disabled with
//! [`crate::ClientBuilder::rate_limit`].

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// The per-account concurrency caps published by GameTorch.
pub(crate) const MAX_OUTSTANDING_HOLDS: usize = 25;
/// Maximum concurrent animation-frame generations per account.
pub(crate) const MAX_FRAME_GENERATIONS: usize = 5;

/// The rate-limit bucket an operation belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RateClass {
    /// 1 request/second per route.
    Tier1,
    /// 2 requests/second per route.
    Tier2,
    /// Shared token bucket across unbounded writes.
    Writes,
    /// Not rate limited (for example `/health`).
    Unlimited,
}

/// Concurrency caps that apply to an operation.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Concurrency {
    /// Acquire one of the 25 outstanding-hold slots.
    pub hold: bool,
    /// Acquire one of the 5 frame-generation slots.
    pub frame_generation: bool,
}

impl Concurrency {
    pub(crate) const NONE: Concurrency = Concurrency {
        hold: false,
        frame_generation: false,
    };
    pub(crate) const HOLD: Concurrency = Concurrency {
        hold: true,
        frame_generation: false,
    };
    pub(crate) const FRAME_GENERATION: Concurrency = Concurrency {
        hold: true,
        frame_generation: true,
    };
}

#[derive(Debug)]
struct Bucket {
    tokens: f64,
    last: Instant,
    capacity: f64,
    refill_per_sec: f64,
}

impl Bucket {
    fn new(capacity: f64, refill_per_sec: f64) -> Self {
        Bucket {
            tokens: capacity,
            last: Instant::now(),
            capacity,
            refill_per_sec,
        }
    }

    fn refill(&mut self, now: Instant) {
        let elapsed = now.saturating_duration_since(self.last).as_secs_f64();
        if elapsed > 0.0 {
            self.tokens = (self.tokens + elapsed * self.refill_per_sec).min(self.capacity);
            self.last = now;
        }
    }

    /// Consumes a token if one is available, otherwise returns how long to wait.
    fn poll(&mut self, now: Instant) -> Option<Duration> {
        self.refill(now);
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            None
        } else {
            let deficit = 1.0 - self.tokens;
            Some(Duration::from_secs_f64(deficit / self.refill_per_sec))
        }
    }
}

/// Owned concurrency permits held for the lifetime of a request.
#[derive(Debug, Default)]
pub(crate) struct Permits {
    _hold: Option<OwnedSemaphorePermit>,
    _frame: Option<OwnedSemaphorePermit>,
}

/// The SDK's shared rate limiter.
#[derive(Debug)]
pub(crate) struct RateLimiter {
    enabled: bool,
    per_route: Mutex<HashMap<&'static str, Bucket>>,
    writes: Mutex<Bucket>,
    holds: std::sync::Arc<Semaphore>,
    frame_generations: std::sync::Arc<Semaphore>,
}

impl RateLimiter {
    pub(crate) fn new(enabled: bool) -> Self {
        RateLimiter {
            enabled,
            per_route: Mutex::new(HashMap::new()),
            writes: Mutex::new(Bucket::new(100.0, 5.0)),
            holds: std::sync::Arc::new(Semaphore::new(MAX_OUTSTANDING_HOLDS)),
            frame_generations: std::sync::Arc::new(Semaphore::new(MAX_FRAME_GENERATIONS)),
        }
    }

    fn bucket_params(class: RateClass) -> (f64, f64) {
        match class {
            RateClass::Tier1 => (1.0, 1.0),
            RateClass::Tier2 => (1.0, 2.0),
            RateClass::Writes => (100.0, 5.0),
            RateClass::Unlimited => (f64::INFINITY, f64::INFINITY),
        }
    }

    /// Waits until the operation's rate bucket allows another request.
    pub(crate) async fn acquire(&self, class: RateClass, route: &'static str) {
        if !self.enabled || class == RateClass::Unlimited {
            return;
        }

        loop {
            let now = Instant::now();
            let wait = if class == RateClass::Writes {
                let mut bucket = self.writes.lock().expect("rate limiter mutex poisoned");
                bucket.poll(now)
            } else {
                let (capacity, refill) = Self::bucket_params(class);
                let mut routes = self.per_route.lock().expect("rate limiter mutex poisoned");
                routes
                    .entry(route)
                    .or_insert_with(|| Bucket::new(capacity, refill))
                    .poll(now)
            };

            match wait {
                None => return,
                Some(delay) => tokio::time::sleep(delay).await,
            }
        }
    }

    /// Acquires the concurrency permits required by an operation.
    pub(crate) async fn acquire_concurrency(&self, concurrency: Concurrency) -> Permits {
        if !self.enabled {
            return Permits::default();
        }

        let hold = if concurrency.hold {
            Some(
                self.holds
                    .clone()
                    .acquire_owned()
                    .await
                    .expect("hold semaphore closed"),
            )
        } else {
            None
        };

        let frame = if concurrency.frame_generation {
            Some(
                self.frame_generations
                    .clone()
                    .acquire_owned()
                    .await
                    .expect("frame semaphore closed"),
            )
        } else {
            None
        };

        Permits {
            _hold: hold,
            _frame: frame,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucket_allows_burst_then_throttles() {
        let mut bucket = Bucket::new(2.0, 1.0);
        let now = Instant::now();
        assert!(bucket.poll(now).is_none());
        assert!(bucket.poll(now).is_none());
        // Third request in the same instant must wait roughly one refill period.
        let wait = bucket.poll(now).expect("should throttle");
        assert!(wait >= Duration::from_millis(900), "wait was {wait:?}");
    }

    #[test]
    fn writes_bucket_has_large_burst() {
        let mut bucket = Bucket::new(100.0, 5.0);
        let now = Instant::now();
        for _ in 0..100 {
            assert!(bucket.poll(now).is_none());
        }
        assert!(bucket.poll(now).is_some());
    }

    #[tokio::test]
    async fn disabled_limiter_never_blocks() {
        let limiter = RateLimiter::new(false);
        for _ in 0..1000 {
            limiter.acquire(RateClass::Tier1, "route").await;
        }
        let permits = limiter
            .acquire_concurrency(Concurrency::FRAME_GENERATION)
            .await;
        assert!(permits._hold.is_none());
        assert!(permits._frame.is_none());
    }
}
