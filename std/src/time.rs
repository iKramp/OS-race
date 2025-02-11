pub use core::time::*;

use crate::thread::GET_TIME_SINCE_BOOT;

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
