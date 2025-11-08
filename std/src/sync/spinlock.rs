use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicU8, Ordering::*},
};

pub struct Spinlock<T: ?Sized> {
    state: AtomicU8,
    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Send for Spinlock<T> {}
unsafe impl<T: ?Sized + Send> Sync for Spinlock<T> {}

pub struct SpinlockGuard<'a, T: ?Sized + 'a> {
    lock: &'a Spinlock<T>,
}
impl<T: ?Sized> !Send for SpinlockGuard<'_, T> {}
unsafe impl<T: ?Sized + Sync> Sync for SpinlockGuard<'_, T> {}

impl<T> Spinlock<T> {
    pub const fn new(t: T) -> Self {
        Self {
            state: AtomicU8::new(0),
            data: UnsafeCell::new(t),
        }
    }
}

impl<T: ?Sized> Spinlock<T> {
    pub fn lock(&self) -> SpinlockGuard<'_, T> {
        while self.state.compare_exchange(0, 1, Acquire, Relaxed).is_err() {
            core::hint::spin_loop();
        }
        SpinlockGuard { lock: self }
    }

    /// only use this in a panic handler
    pub fn force_get_lock(&self) -> SpinlockGuard<'_, T> {
        SpinlockGuard { lock: self }
    }

    pub fn try_lock(&self) -> Option<SpinlockGuard<'_, T>> {
        if self.state.compare_exchange(0, 1, Acquire, Relaxed).is_ok() {
            Some(SpinlockGuard { lock: self })
        } else {
            None
        }
    }
}

impl<T: ?Sized> Drop for SpinlockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.state.store(0, Release);
    }
}

impl<T: ?Sized> DerefMut for SpinlockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T: ?Sized> Deref for SpinlockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}
