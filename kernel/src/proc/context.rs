use core::sync::atomic::{AtomicU32, Ordering};
use std::{boxed::Box, mem_utils::{self, VirtAddr}, vec::Vec, PageAllocator};

use super::{ProcessData, ThreadData, ThreadState, Tid};
use crate::{memory::{self, paging::PageTree}, proc::{Pid, ProcessState}};
use bitfield::bitfield;

const DEFAULT_STACK_SIZE: usize = 0x1000; // 4KB
const MAX_STACK_SIZE: usize = 0x2111; // 8KB
//contains...?
const KERNEL_DATA_SIZE_PAGES: usize = 0;

static PROCESS_ID_COUNTER: AtomicU32 = AtomicU32::new(0);


bitfield! {
    #[derive(Copy, Clone)]
    pub struct MemoryRegionFlags(u32);
    impl Debug;
    pub is_writeable, set_is_writeable: 1;
    pub is_executable, set_is_executable: 2;
}

pub struct MemoryRegionDescriptor {
    pub start: usize,
    pub end: usize,
    pub flags: MemoryRegionFlags,
}

pub struct ContextInfo<'a> {
    pub is_32_bit: bool,
    pub stack_size_pages: Option<u8>,
    pub mem_regions: Box<[MemoryRegionDescriptor]>,
    pub mem_init: Box<[(VirtAddr, &'a [u8])]>,
    pub entry_point: VirtAddr,
    pub cmdline: Box<str>,
}

pub fn build_context(context: ContextInfo) -> ProcessData {
    let page_tree_root = memory::physical_allocator::allocate_frame();
    let mut page_tree = PageTree::new(page_tree_root);
    page_tree.init();

    // Map any syscall or interrupt regions. Change IDT and similar structures to be page aligned
    // and have padding, as to not expose any other static kernel data to the process
    // Don't forget to respect the 32 bit option

    for region in context.mem_regions.iter() {
        //we assume memory regions don't overlap or use same pages
        let start = region.start & !0xFFF;
        let end = region.end.div_ceil(0x1000) * 0x1000;
        for i in (start..end).step_by(0x1000) {
            page_tree.allocate_set_virtual(None, VirtAddr(i as u64));
            let page = page_tree.get_page_table_entry_mut(VirtAddr(i as u64)).unwrap();
            page.set_writeable(region.flags.is_writeable());
            page.set_no_execute(!region.flags.is_executable());
        }
    }
    for init in context.mem_init.iter() {
        //change from virtual in process's address space to virtual in kernel's space
        let start_physical = mem_utils::translate_virt_phys_addr(init.0, page_tree_root).unwrap();
        let start_virtual = mem_utils::translate_phys_virt_addr(start_physical);

        unsafe {
            core::ptr::copy_nonoverlapping(init.1.as_ptr(), start_virtual.0 as *mut u8, init.1.len());
        }
    }

    let mut proc_data = ProcessData {
        pid: Pid(PROCESS_ID_COUNTER.fetch_add(1, Ordering::SeqCst)),
        is_32_bit: context.is_32_bit,
        cmdline: context.cmdline,
        page_tree_root,
        threads: Vec::new(),
        state: ProcessState::Running,
    };
    add_thread(&mut proc_data, context.stack_size_pages.unwrap_or(DEFAULT_STACK_SIZE as u8));
    proc_data
}

pub fn add_thread(proc_data: &mut ProcessData, stack_size_pages: u8) -> &ThreadData {
    let stack_search = if proc_data.is_32_bit {
        //highest address is 0xFFFF_FFFF
        0xFFFF_FFFF - KERNEL_DATA_SIZE_PAGES * 0x1000
    } else {
        //just in case, we will NOT use higher half, that is kernel exclusive memory
        //48 bits for addressing, so highest addr is 0x7FFF_FFFF_FFFF
        0x7FFF_FFFF_FFFF - KERNEL_DATA_SIZE_PAGES * 0x1000
    };
    let mut stack_search = VirtAddr(stack_search as u64);

    'outer: loop {
        for thread in proc_data.threads.iter() {
            let thread_start = thread.stack_base;
            //stacks are separated by 1 non-mapped page, to catch stack overflows
            let thread_end = thread_start - (thread.stack_size_pages as u64 + 1) * 0x1000;
            let stack_search_end = stack_search - (stack_size_pages as u64 + 1) * 0x1000;
            let new_overlaps_thread_start = stack_search >= thread_start && stack_search_end < thread_start;
            let new_overlaps_thread_end = stack_search >= thread_start && stack_search < thread_end;
            if new_overlaps_thread_start || new_overlaps_thread_end {
                stack_search = thread_end;
                continue 'outer;
            }
        }
        //we have a free stack at this address
        break;
    }

    let mut page_tree = PageTree::new(proc_data.page_tree_root);
    let lowest_stack = stack_search - (stack_size_pages as u64) * 0x1000;

    for i in (lowest_stack.0..stack_search.0).step_by(0x1000) {
        page_tree.allocate_set_virtual(None, VirtAddr(i));
        let page = page_tree.get_page_table_entry_mut(VirtAddr(i)).unwrap();
        page.set_writeable(true);
        page.set_no_execute(false);
    }

    let thread_id = if proc_data.threads.is_empty() {
        proc_data.pid.0
    } else {
        PROCESS_ID_COUNTER.fetch_add(1, Ordering::SeqCst)
    };

    let thread_data = ThreadData {
        pid: proc_data.pid,
        thread_id: Tid(thread_id),
        stack_pointer: stack_search,
        stack_base: lowest_stack,
        stack_size_pages,
        state: ThreadState::Running,
    };

    proc_data.threads.push(thread_data);

    proc_data.threads.last_mut().unwrap()
}

pub fn remove_thread_context(proc_data: &mut ProcessData, thread_id: Tid) {
    let mut page_tree_root = PageTree::new(proc_data.page_tree_root);
    let thread = proc_data.threads.iter().position(|t| t.thread_id == thread_id);

    //remove stack
    if let Some(thread_index) = thread {
        let thread = proc_data.threads.remove(thread_index);
        let stack_base = thread.stack_base;
        let stack_end = thread.stack_base - (thread.stack_size_pages as u64) * 0x1000;
        for i in (stack_end.0..stack_base.0).step_by(0x1000) {
            page_tree_root.unmap(VirtAddr(i));
        }
    }
}

pub fn remove_proc_context(proc_data: &mut ProcessData) {
    //remove all threads
    let thread_ids = proc_data.threads.iter().map(|t| t.thread_id).collect::<Vec<_>>();
    for thread_id in thread_ids {
        remove_thread_context(proc_data, thread_id)
    }

    //first unmap all pages that still need to be allocated in physical allocator


    //then remove all left over pages
    let mut page_tree = PageTree::new(proc_data.page_tree_root);
}
