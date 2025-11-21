use std::{boxed::Box, mem_utils::PhysAddr, sync::arc::Arc, vec::Vec};

use crate::{proc::{syscall::SyscallArgs, ProcessData}, task_runner};


pub fn fread(args: &mut SyscallArgs, proc: &Arc<ProcessData>) -> bool {
    let fd = args.arg1;
    let buffer_ptr = args.arg2 as *mut u8;
    let size = args.arg3;
    let proc = proc.clone();
    let pid = proc.pid();

    let file_handle = {
        let mut proc_mut = proc.get_mutable();
        if let Some(f_handle) = proc_mut.take_file_handle(fd) {
            f_handle
        } else {
            args.syscall_number = u64::MAX;
            return false;
        }
    };

    let task = async move {
        let mut f_handle = file_handle; //get to local
        let size_rounded = size.div_ceil(4096) * 4096;
        let buffer_alloc = crate::memory::physical_allocator::allocate_contiguius_high(size_rounded as u64);
        let buffers = (0..size_rounded).map(|i| buffer_alloc + (i as u64 * 4096)).collect::<Vec<PhysAddr>>();

        let read_result = crate::vfs::read_file(&mut f_handle, &buffers, size).await;
        let Some(proc) = crate::proc::get_proc(proc.pid()) else {
            return; //proc was killed
        };
        if let Err(_) = read_result {
            let proc_lock = proc.get();
            proc_lock.set_syscall_return(u64::MAX, 1).unwrap();
            return;
        }
        //copy to user buffer
        let dst = buffer_ptr;
        let src = std::mem_utils::translate_phys_virt_addr(buffer_alloc).0 as *const u8;
        unsafe { core::ptr::copy_nonoverlapping(src, dst, size as usize) };

        //free
        for i in 0..size_rounded {
            unsafe { crate::memory::physical_allocator::deallocate_frame(buffer_alloc + (i as u64 * 4096)) };
        }

        //return fd
        proc.get_mutable().insert_file_handle(fd, f_handle);

        //return
        proc.set_syscall_return(size, 0).unwrap();
        crate::proc::wake_process(proc.pid())
    };

    task_runner::add_task(Box::pin(task), Some(pid));
    true
}
