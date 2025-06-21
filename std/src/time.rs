use core::fmt::Debug;
pub use core::time::*;

use crate::thread::GET_TIME_SINCE_EPOCH;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Default)]
pub struct Instant {
    since_epoch: Duration,
}

pub const UNIX_EPOCH: Instant = Instant {
    since_epoch: Duration::from_secs(0),
};

impl Instant {
    pub fn now() -> Instant {
        Instant {
            since_epoch: unsafe { GET_TIME_SINCE_EPOCH() },
        }
    }

    pub fn from_duration_since_epoch(duration: Duration) -> Instant {
        Instant {
            since_epoch: duration,
        }
    }

    pub fn elapsed(&self) -> Duration {
        unsafe { GET_TIME_SINCE_EPOCH() - self.since_epoch }
    }

    pub fn duration_since(&self, earlier: Instant) -> Duration {
        self.since_epoch - earlier.since_epoch
    }
}

impl core::ops::Add<Duration> for Instant {
    type Output = Instant;

    fn add(self, rhs: Duration) -> Instant {
        Instant {
            since_epoch: self.since_epoch + rhs,
        }
    }
}

impl core::ops::Sub<Duration> for Instant {
    type Output = Instant;

    fn sub(self, rhs: Duration) -> Instant {
        Instant {
            since_epoch: self.since_epoch - rhs,
        }
    }
}

impl core::ops::Sub<Instant> for Instant {
    type Output = Duration;

    fn sub(self, rhs: Instant) -> Duration {
        self.since_epoch - rhs.since_epoch
    }
}

impl Debug for Instant {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Instant({:?})", self.since_epoch)
    }
}

impl core::fmt::Display for Instant {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Instant({:?})", self.since_epoch)
    }
}
