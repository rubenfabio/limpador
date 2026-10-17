use limpador_core::ports::Clock;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

const DAY: Duration = Duration::from_secs(24 * 60 * 60);

/// A clock frozen at a chosen instant that only moves when told to.
#[derive(Debug)]
pub struct FixedClock {
    now: Mutex<SystemTime>,
}

impl FixedClock {
    pub fn at(now: SystemTime) -> Self {
        Self {
            now: Mutex::new(now),
        }
    }

    /// 2026-01-15T12:00:00Z — a fixed, readable reference date for scenarios.
    pub fn reference() -> Self {
        Self::at(SystemTime::UNIX_EPOCH + Duration::from_secs(1_768_478_400))
    }

    pub fn advance(&self, by: Duration) {
        *self.now.lock().unwrap() += by;
    }

    pub fn days_ago(&self, days: u64) -> SystemTime {
        self.now() - DAY * u32::try_from(days).expect("days fits in u32")
    }
}

impl Clock for FixedClock {
    fn now(&self) -> SystemTime {
        *self.now.lock().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stays_frozen_until_advanced() {
        let clock = FixedClock::reference();
        let first = clock.now();
        assert_eq!(clock.now(), first);
        clock.advance(Duration::from_secs(60));
        assert_eq!(clock.now(), first + Duration::from_secs(60));
    }

    #[test]
    fn days_ago_counts_whole_days_back() {
        let clock = FixedClock::reference();
        assert_eq!(clock.days_ago(45), clock.now() - DAY * 45);
        assert_eq!(clock.days_ago(0), clock.now());
    }
}
