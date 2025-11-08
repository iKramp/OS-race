use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicU8, Ordering::*},
};

pub struct NoIntSpinlock<T: ?Sized> {
    state: AtomicU8,
    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Send for NoIntSpinlock<T> {}
unsafe impl<T: ?Sized + Send> Sync for NoIntSpinlock<T> {}

pub struct NoIntSpinlockGuard<'a, T: ?Sized + 'a> {
    prev_int_enabled: bool,
    lock: &'a NoIntSpinlock<T>,
}
impl<T: ?Sized> !Send for NoIntSpinlockGuard<'_, T> {}
unsafe impl<T: ?Sized + Sync> Sync for NoIntSpinlockGuard<'_, T> {}

impl<T> NoIntSpinlock<T> {
    pub const fn new(t: T) -> Self {
        Self {
            state: AtomicU8::new(0),
            data: UnsafeCell::new(t),
        }
    }
}

impl<T: ?Sized> NoIntSpinlock<T> {
    pub fn lock(&self) -> NoIntSpinlockGuard<'_, T> {
        let prev_rflags: u64;
        unsafe { 
            core::arch::asm!(
                "pushfq",
                "pop {}",
                "cli",
                out(reg) prev_rflags
            );
        }
        let prev_int_state = (prev_rflags & (1 << 9)) != 0;
        while self.state.compare_exchange(0, 1, Acquire, Relaxed).is_err() {
            core::hint::spin_loop();
        }
        NoIntSpinlockGuard { lock: self, prev_int_enabled: prev_int_state  }
    }

    /// only use this in a panic handler
    pub fn force_get_lock(&self) -> NoIntSpinlockGuard<'_, T> {
        let prev_rflags: u64;
        unsafe { 
            core::arch::asm!(
                "pushfq",
                "pop {}",
                "cli",
                out(reg) prev_rflags
            );
        }
        NoIntSpinlockGuard { lock: self, prev_int_enabled: (prev_rflags & (1 << 9)) != 0  }
    }

    pub fn try_lock(&self) -> Option<NoIntSpinlockGuard<'_, T>> {
        let prev_rflags: u64;
        unsafe { 
            core::arch::asm!(
                "pushfq",
                "pop {}",
                "cli",
                out(reg) prev_rflags
            );
        }
        let prev_int_state = (prev_rflags & (1 << 9)) != 0;
        if self.state.compare_exchange(0, 1, Acquire, Relaxed).is_ok() {
            Some(NoIntSpinlockGuard { lock: self, prev_int_enabled: prev_int_state })
        } else {
            if prev_int_state {
                unsafe { core::arch::asm!("sti") };
            }
            None
        }
    }
}

impl<T: ?Sized> Drop for NoIntSpinlockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.state.store(0, Release);
        if self.prev_int_enabled {
            unsafe { core::arch::asm!("sti") };
        }
    }
}

impl<T: ?Sized> DerefMut for NoIntSpinlockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T: ?Sized> Deref for NoIntSpinlockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}
