pub static mut GET_TIME_SINCE_BOOT: fn() -> core::time::Duration = || core::time::Duration::from_secs(0);

pub fn sleep(duration: core::time::Duration) {
    unsafe {
        let start = GET_TIME_SINCE_BOOT();
        loop {
            let now = GET_TIME_SINCE_BOOT();
            if start + duration < now {
                break;
            }
            core::arch::asm!("hlt");
        }
    }
}