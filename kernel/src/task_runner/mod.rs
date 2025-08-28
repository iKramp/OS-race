use core::{
    pin::Pin,
    sync::atomic::AtomicPtr,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};
use std::boxed::Box;

use crate::acpi::cpu_locals::CpuLocals;

//probably won't change return type, tasks should modify process state or other things themselves (through
//a pointer)
pub type AsyncTask = dyn Future<Output = ()>;
pub struct AsyncTaskHolder {
    task: Pin<Box<AsyncTask>>,
    next_task: AtomicPtr<AsyncTaskHolder>,
}

pub fn add_task(task: Pin<Box<AsyncTask>>) {
    let task = AsyncTaskHolder {
        task,
        next_task: AtomicPtr::new(core::ptr::null_mut()),
    };

    add_task_holder(Box::new(task));
}

fn add_task_holder(task: Box<AsyncTaskHolder>) {
    let task_ptr = Box::into_raw(task);
    let locals = CpuLocals::get();

    //inserts into the beginning and gets ptr to the next node
    //any interrupting stores will point to this one and not affect anything following
    let new_ptr = locals.async_task_list.swap(task_ptr, core::sync::atomic::Ordering::AcqRel);
    if !new_ptr.is_null() {
        unsafe { (*task_ptr).next_task.store(new_ptr, core::sync::atomic::Ordering::Release) };
    }
}

pub fn process_tasks() {
    let locals = CpuLocals::get();
    loop {
        let mut list = locals
            .async_task_list
            .swap(core::ptr::null_mut(), core::sync::atomic::Ordering::AcqRel);

        if list.is_null() {
            break;
        }

        loop {
            let task = unsafe { Box::from_raw(list) };
            list = task.next_task.load(core::sync::atomic::Ordering::Acquire);
            process_single_task(task);

            if list.is_null() {
                break;
            }
        }
    }
}

fn noop(_: *const ()) {}
fn clone(_: *const ()) -> RawWaker {
    dummy_raw_waker()
}

fn dummy_raw_waker() -> RawWaker {
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);
    RawWaker::new(core::ptr::null(), &VTABLE)
}
fn dummy_waker() -> Waker {
    // SAFETY: VTABLE functions are no-ops, so this is safe
    unsafe { Waker::from_raw(dummy_raw_waker()) }
}

fn process_single_task(mut task: Box<AsyncTaskHolder>) {
    task.next_task
        .store(core::ptr::null_mut(), core::sync::atomic::Ordering::Release);

    let result = task.task.as_mut().poll(&mut Context::from_waker(&dummy_waker()));

    match result {
        Poll::Pending => {
            add_task_holder(task);
        }
        Poll::Ready(_) => {
        }
    }
}
