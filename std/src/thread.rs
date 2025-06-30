pub static mut SLEEP: fn(core::time::Duration) = |_duration| {};
pub static mut TIMER_ACTIVE: bool = false;

pub fn sleep(duration: core::time::Duration) {
    unsafe {
        SLEEP(duration);
    }
}
