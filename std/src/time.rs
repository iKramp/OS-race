pub use core::time::*;

use crate::thread::GET_TIME_SINCE_BOOT;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Default)]
pub struct Instant {
    duration_since_boot: Duration,
}

impl Instant {
    pub fn now() -> Instant {
        Instant {
            duration_since_boot: unsafe { GET_TIME_SINCE_BOOT() },
        }
    }

    pub fn elapsed(&self) -> Duration {
        unsafe { GET_TIME_SINCE_BOOT() - self.duration_since_boot }
    }

    pub fn duration_since(&self, earlier: Instant) -> Duration {
        self.duration_since_boot - earlier.duration_since_boot
    }
}

impl core::ops::Add<Duration> for Instant {
    type Output = Instant;

    fn add(self, rhs: Duration) -> Instant {
        Instant {
            duration_since_boot: self.duration_since_boot + rhs,
        }
    }
}

impl core::ops::Sub<Duration> for Instant {
    type Output = Instant;

    fn sub(self, rhs: Duration) -> Instant {
        Instant {
            duration_since_boot: self.duration_since_boot - rhs,
        }
    }
}

impl core::ops::Sub<Instant> for Instant {
    type Output = Duration;

    fn sub(self, rhs: Instant) -> Duration {
        self.duration_since_boot - rhs.duration_since_boot
    }
}
