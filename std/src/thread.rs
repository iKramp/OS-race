pub static mut GET_TIME_SINCE_EPOCH: fn() -> core::time::Duration = || core::time::Duration::from_secs(0);
pub static mut TIMER_ACTIVE: bool = false;

pub fn sleep(duration: core::time::Duration) {
    unsafe {
        if !TIMER_ACTIVE {
            return;
        }
        let start = GET_TIME_SINCE_EPOCH();
        loop {
            let now = GET_TIME_SINCE_EPOCH();
            if start + duration < now {
                break;
            }
            core::arch::asm!("hlt");
        }
    }
}
