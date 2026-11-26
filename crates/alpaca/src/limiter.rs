//! Sliding-window rate limiter with an injected clock and sleeper.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::error::AlpacaError;

/// Length of the sliding window.
const WINDOW: Duration = Duration::from_secs(60);

/// A monotonic clock, as a duration since an arbitrary fixed origin.
pub trait MonoClock: Send + Sync {
    fn now(&self) -> Duration;
}

/// Something that can block the current thread.
pub trait Sleeper: Send + Sync {
    fn sleep(&self, d: Duration);
}

/// The real clock, measured from its creation.
#[derive(Debug)]
pub struct SystemClock {
    origin: Instant,
}

impl SystemClock {
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl MonoClock for SystemClock {
    fn now(&self) -> Duration {
        self.origin.elapsed()
    }
}

/// The real sleeper.
#[derive(Debug, Default)]
pub struct ThreadSleeper;

impl Sleeper for ThreadSleeper {
    fn sleep(&self, d: Duration) {
        std::thread::sleep(d);
    }
}

/// Allows at most `max` calls in any 60 second window.
pub struct RateLimiter {
    max: usize,
    clock: Arc<dyn MonoClock>,
    sleeper: Arc<dyn Sleeper>,
    calls: Mutex<VecDeque<Duration>>,
}

impl RateLimiter {
    /// A limit of 0 would never let a call through, so it is refused.
    pub fn new(
        max: u32,
        clock: Arc<dyn MonoClock>,
        sleeper: Arc<dyn Sleeper>,
    ) -> Result<Self, AlpacaError> {
        if max == 0 {
            return Err(AlpacaError::Invalid(
                "rate limit must be at least 1 request per minute".to_string(),
            ));
        }
        Ok(Self {
            max: max as usize,
            clock,
            sleeper,
            calls: Mutex::new(VecDeque::new()),
        })
    }

    /// Record a call, sleeping first if the window is full.
    pub fn acquire(&self) {
        let mut calls = self.calls.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            let now = self.clock.now();
            while calls
                .front()
                .is_some_and(|t| now.saturating_sub(*t) >= WINDOW)
            {
                calls.pop_front();
            }
            if calls.len() < self.max {
                calls.push_back(now);
                return;
            }
            let oldest = calls[0];
            self.sleeper.sleep(WINDOW - now.saturating_sub(oldest));
        }
    }
}
