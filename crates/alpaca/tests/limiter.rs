use std::sync::{Arc, Mutex};
use std::time::Duration;

use pt_alpaca::limiter::{MonoClock, RateLimiter, Sleeper};

/// A fake clock whose sleeper advances the clock and records each sleep.
#[derive(Default)]
struct Fake {
    now: Mutex<Duration>,
    sleeps: Mutex<Vec<Duration>>,
}

impl Fake {
    fn advance(&self, d: Duration) {
        *self.now.lock().unwrap() += d;
    }
    fn sleeps(&self) -> Vec<Duration> {
        self.sleeps.lock().unwrap().clone()
    }
}

impl MonoClock for Fake {
    fn now(&self) -> Duration {
        *self.now.lock().unwrap()
    }
}

impl Sleeper for Fake {
    fn sleep(&self, d: Duration) {
        self.sleeps.lock().unwrap().push(d);
        self.advance(d);
    }
}

fn limiter(max: u32) -> (RateLimiter, Arc<Fake>) {
    let fake = Arc::new(Fake::default());
    let clock: Arc<dyn MonoClock> = fake.clone();
    let sleeper: Arc<dyn Sleeper> = fake.clone();
    (RateLimiter::new(max, clock, sleeper), fake)
}

// R-084: three acquires at time 0 with a limit of 3 cause no sleep.
#[test]
fn calls_within_the_limit_do_not_sleep() {
    let (lim, fake) = limiter(3);
    for _ in 0..3 {
        lim.acquire();
    }
    assert!(fake.sleeps().is_empty());
}

// R-084: the fourth call in the same minute sleeps exactly 60 seconds, once.
#[test]
fn fourth_call_sleeps_exactly_sixty_seconds() {
    let (lim, fake) = limiter(3);
    for _ in 0..4 {
        lim.acquire();
    }
    assert_eq!(fake.sleeps(), vec![Duration::from_secs(60)]);
}

// R-084: after 61 more seconds the window is empty, so three calls do not sleep.
#[test]
fn window_clears_after_the_minute_passes() {
    let (lim, fake) = limiter(3);
    for _ in 0..4 {
        lim.acquire();
    }
    fake.advance(Duration::from_secs(61));
    for _ in 0..3 {
        lim.acquire();
    }
    assert_eq!(fake.sleeps().len(), 1, "no sleep beyond the first");
}

// R-084: the wait is only the time left until the oldest call leaves the window.
#[test]
fn sleep_is_the_time_until_the_oldest_call_expires() {
    let (lim, fake) = limiter(3);
    for _ in 0..3 {
        lim.acquire();
    }
    fake.advance(Duration::from_secs(10));
    lim.acquire();
    assert_eq!(fake.sleeps(), vec![Duration::from_secs(50)]);
}
