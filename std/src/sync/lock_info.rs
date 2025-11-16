use core::sync::atomic::{AtomicBool, AtomicU16};

static GLOBAL_LOCK_INFO: LockInfo = LockInfo::new();

//fn to get lock info
pub(super) static mut GET_LOCK_INFO: fn() -> &'static LockInfo = || &GLOBAL_LOCK_INFO;

#[derive(Debug)]
pub struct LockInfo {
    num_no_int_spinlocks: AtomicU16,
    prev_int_state: AtomicBool,
    blocking_task: AtomicBool,
}

impl LockInfo {
    pub const fn new() -> Self {
        Self {
            num_no_int_spinlocks: AtomicU16::new(0),
            prev_int_state: AtomicBool::new(true),
            blocking_task: AtomicBool::new(false),
        }
    }

    pub fn inc_spinlocks(&self, prev_int_state: bool) {
        let prev_val = self.num_no_int_spinlocks.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        if prev_val == 0 {
            self.prev_int_state.store(prev_int_state, core::sync::atomic::Ordering::SeqCst);
        }
    }

    //returns whether interrupts should be re-enabled
    pub fn dec_spinlocks(&self) -> bool {
        let new_val = self.num_no_int_spinlocks.fetch_sub(1, core::sync::atomic::Ordering::SeqCst) - 1;
        if new_val == 0 {
            self.prev_int_state.load(core::sync::atomic::Ordering::SeqCst)
        } else {
            false
        }
    }

    pub fn no_locks(&self) -> bool {
        self.num_no_int_spinlocks.load(core::sync::atomic::Ordering::SeqCst) == 0
    }

    pub fn num_locks(&self) -> u16 {
        self.num_no_int_spinlocks.load(core::sync::atomic::Ordering::SeqCst)
    }

    pub fn blocking_task(&self) {
        self.blocking_task.store(true, core::sync::atomic::Ordering::SeqCst);
    }

    pub fn unblocking_task(&self) {
        self.blocking_task.store(false, core::sync::atomic::Ordering::SeqCst);
    }

    pub fn is_blocking_task(&self) -> bool {
        self.blocking_task.load(core::sync::atomic::Ordering::SeqCst)
    }
}

impl Default for LockInfo {
    fn default() -> Self {
        Self::new()
    }
}

pub fn set_lock_info_func(f: fn() -> &'static LockInfo) {
    assert!(GLOBAL_LOCK_INFO.num_locks() == 0);
    assert!(f().num_locks() == 0);
    unsafe {
        GET_LOCK_INFO = f;
    }
}
