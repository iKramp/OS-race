use core::{
    sync::atomic::AtomicU64,
    task::{Poll, Waker},
};
use std::{
    alloc::{boxed::Box, sync::Arc, vec::Vec},
    error::KernelError,
    kerror, lock_w_info, r_lock_w_info,
    sync::{no_int_spinlock::NoIntSpinlock, rw_lock::RWSpinlock},
    time::Instant,
    w_lock_w_info,
};

mod input_adapter;

const MAX_QUEUED_EVENTS: usize = 64;

pub(super) fn init_input() {
    todo!()
}

#[derive(PartialEq, Eq)]
enum DeviceType {
    Keyboard,
    Mouse,
    Controller,
    Touchpad,
    _6Dof,
    Other(u8),
}

type DeviceId = u64;
type EventBufferWithType = (Option<DeviceType>, Option<DeviceId>, EventBufferWrapper);

struct InputEventManager {
    queues: RWSpinlock<Vec<EventBufferWithType>>,
    queue_counter: AtomicU64,
}

impl InputEventManager {
    pub const fn new() -> Self {
        Self {
            queues: RWSpinlock::new(Vec::new()),
            queue_counter: AtomicU64::new(0),
        }
    }

    pub fn add_event(&self, device: DeviceType, device_id: DeviceId, event: InputEvent) {
        let queues = r_lock_w_info!(self.queues);
        for queue in queues.iter() {
            let dt_matches = if let Some(dt) = &queue.0 { dt == &device } else { true };
            let did_matches = if let Some(did) = &queue.1 { did == &device_id } else { true };
            if dt_matches && did_matches {
                let _ = queue.2.add_event(event); //whatever, gets dropped
            }
        }
    }

    pub fn add_buffer(&self, device: Option<DeviceType>, device_id: Option<DeviceId>) -> u64 {
        let new_id = self.queue_counter.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        let new_queue = EventBufferWrapper {
            id: new_id,
            inner: Arc::new(NoIntSpinlock::new(EventBuffer::new())),
        };
        let mut queues = w_lock_w_info!(self.queues);
        queues.push((device, device_id, new_queue));
        new_id
    }

    pub fn remove_buffer(&self, buffer_id: u64) {
        let mut queues = w_lock_w_info!(self.queues);
        queues.retain(|e| e.2.id != buffer_id);
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct InputEvent {
    generated_at: Instant,
    code: u32,
    value: u32,
}

struct EventBufferWrapper {
    id: u64,
    inner: Arc<NoIntSpinlock<EventBuffer>>,
}

struct EventBuffer {
    queue: Box<[InputEvent; MAX_QUEUED_EVENTS]>,
    async_waker: Option<Waker>,
    first_valid_event: u32,
    first_empty_space: u32,
}

impl EventBufferWrapper {
    pub fn add_event(&self, event: InputEvent) -> Result<(), KernelError> {
        let mut inner = lock_w_info!(self.inner);
        inner.add_event(event)
    }

    pub fn consume_event(&self) -> Option<InputEvent> {
        let mut inner = lock_w_info!(self.inner);
        inner.consume_event()
    }
}

impl EventBuffer {
    pub fn new() -> Self {
        Self {
            queue: Box::new([InputEvent::default(); MAX_QUEUED_EVENTS]),
            async_waker: None,
            first_valid_event: 0,
            first_empty_space: 0,
        }
    }

    //max queued events is actually 1 less
    pub fn add_event(&mut self, event: InputEvent) -> Result<(), KernelError> {
        if (self.first_empty_space + 1) % (MAX_QUEUED_EVENTS as u32) == self.first_valid_event {
            return kerror!(InsufficientResources);
        }
        let empty_space_index = self.first_empty_space as usize;
        self.queue[empty_space_index] = event;
        self.first_empty_space = (self.first_empty_space + 1) % MAX_QUEUED_EVENTS as u32;
        if let Some(waker) = self.async_waker.take() {
            waker.wake();
        }
        Ok(())
    }

    pub fn consume_event(&mut self) -> Option<InputEvent> {
        if self.first_empty_space == self.first_valid_event {
            return None;
        }
        let event_index = self.first_valid_event;
        self.first_valid_event = (self.first_valid_event + 1) % MAX_QUEUED_EVENTS as u32;
        Some(self.queue[event_index as usize])
    }
}

struct AsyncEventReader {
    event_buffer: Arc<NoIntSpinlock<EventBuffer>>,
}

impl Future for AsyncEventReader {
    type Output = Result<InputEvent, KernelError>;

    fn poll(self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> core::task::Poll<Self::Output> {
        let mut inner = lock_w_info!(self.event_buffer);
        let res = inner.consume_event();
        if let Some(res) = res {
            return Poll::Ready(Ok(res));
        }
        inner.async_waker = Some(cx.waker().clone());
        Poll::Pending
    }
}
