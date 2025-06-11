use crate::interrupts::InterruptProcessorState;
use crate::proc::CpuStateType;
use crate::proc::PROCESS_ID_COUNTER;
use crate::proc::ProcessData;
use crate::proc::SCHEDULER;
use std::string::ToString;
use std::sync::arc::Arc;
use std::{
    mem_utils::{self, VirtAddr, memset_physical_addr},
    println, vec,
};

use crate::{
    memory::{self, paging::PageTree},
    proc::{MemoryContext, Pid, Stack},
};

use super::info::ContextInfo;

const DEFAULT_PROC_STACK_SIZE: usize = 0x1000; // 4KB

pub fn create_process(context_info: ContextInfo) -> Pid {
    let pid = Pid(PROCESS_ID_COUNTER.fetch_add(1, core::sync::atomic::Ordering::Relaxed));
    let is_32_bit = context_info.is_32_bit();
    let cmdline = context_info.cmdline().to_string().into_boxed_str();
    let rip = context_info.entry_point().0;
    let memory_context = build_mem_context_for_new_proc(context_info);
    let rsp = memory_context.stacks.last().unwrap().stack_base.0;

    let cpu_state = InterruptProcessorState::new(rip, rsp);
    println!("rip was set to {:#X}", rip);
    let process_data = ProcessData {
        pid,
        is_32_bit,
        cmdline,
        memory_context: Arc::new(memory_context),
        cpu_state: CpuStateType::Interrupt(cpu_state),
    };

    let mut scheduler_lock = SCHEDULER.lock();
    let scheduler = unsafe { scheduler_lock.assume_init_mut() };
    scheduler.accept_new_process(pid, process_data);
    pid
}

pub fn build_generic_memory_context(context: ContextInfo) -> MemoryContext {
    let mut memory_tree = build_generic_memory_tree();

    let mut mem_init = context.mem_init().iter();
    let mut curr_init_opt = mem_init.next();

    // map memory regions
    for region in context.mem_regions().iter() {
        //we assume memory regions don't overlap or use same pages
        let start = region.start().0 & !0xFFF;
        let end = start + region.size_pages() as u64 * 0x1000;
        for page_addr in (start..end).step_by(0x1000) {
            let phys_addr_map = memory_tree.allocate_set_virtual(None, VirtAddr(page_addr));
            let page = memory_tree.get_page_table_entry_mut(VirtAddr(page_addr)).unwrap();
            page.set_writeable(region.flags().is_writeable());
            page.set_user_accessible(true);

            if region.flags().is_executable() {
                memory_tree.set_execute(VirtAddr(page_addr));
                println!("mapping executable page at {:#X}", page_addr);
            }

            //copy data
            loop {
                let Some(curr_init) = curr_init_opt else {
                    break;
                };
                if curr_init.0.0 >= page_addr + 0x1000 {
                    //no more initializations for this region
                    break;
                }
                if curr_init.0.0 + (curr_init.1.len() as u64) < page_addr {
                    //initialization is before this region, so skip it
                    curr_init_opt = mem_init.next();
                    continue;
                }

                let src_start_index = (page_addr as i64 - curr_init.0.0 as i64).max(0) as u64;
                let dest_start_addr = page_addr.max(curr_init.0.0);
                let buf_copy_len = curr_init.1.len() - src_start_index as usize;
                let dest_copy_len = page_addr + 0x1000 - dest_start_addr;
                let copy_len = buf_copy_len.min(dest_copy_len as usize);
                let copy_buffer = &curr_init.1[src_start_index as usize..src_start_index as usize + copy_len];
                unsafe { mem_utils::memcopy_physical_buffer(phys_addr_map + (dest_start_addr & 0xfff), copy_buffer) };

                if src_start_index + copy_len as u64 >= curr_init.1.len() as u64 {
                    //we copied all data from this init, so go to next
                    curr_init_opt = mem_init.next();
                } else {
                    //we still have data left in this init, so break and continue copying
                    break;
                }
            }
        }
    }

    MemoryContext {
        is_32_bit: context.is_32_bit(),
        page_tree: memory_tree,
        stacks: vec![],
    }
}

pub fn build_mem_context_for_new_proc(context: ContextInfo) -> MemoryContext {
    let mut generic_context = build_generic_memory_context(context);
    let stack_size_pages = DEFAULT_PROC_STACK_SIZE.div_ceil(0x1000) as u8; // convert to pages

    //add stack
    add_stack(&mut generic_context, stack_size_pages);
    generic_context
}

pub fn add_stack(context: &mut MemoryContext, stack_size_pages: u8) {
    let highest_userspace_addr: u64 = if context.is_32_bit {
        //highest address is 0xFFFF_FFFF, highest quarter is kernel on 32 bit applications
        //The kernel here will STILL be in higher half of 64(48) bit address space, but maybe
        //applications assume their address can't be in the highest qurter
        0xC000_0000
    } else {
        //48 bits for addressing, so highest userspace addr is 0x7FFF_FFFF_FFFF
        0x8000_0000_0000
    };

    let stack_reserve_pages = stack_size_pages as u64 + 1;
    let mem_tree = &mut context.page_tree;
    let stack_search_page = (highest_userspace_addr >> 12) - 1;

    let mut top_page = 0;
    'top_loop: for _top_page in (0..stack_search_page).rev() {
        for page in (_top_page - stack_reserve_pages + 1)..=_top_page {
            if mem_tree.get_page_table_entry_mut(VirtAddr(page << 12)).is_some() {
                //found a page that is already mapped, so we can't use this address
                continue 'top_loop;
            }
        }
        top_page = _top_page;
        break;
    }

    //found a free stack at this address
    for page in (top_page - stack_reserve_pages + 2)..=top_page {
        mem_tree.allocate_set_virtual(None, VirtAddr(page << 12));
        println!("allocating stack page at {:#X}", page << 12);
        let entry = mem_tree.get_page_table_entry_mut(VirtAddr(page << 12)).unwrap();
        entry.set_writeable(true);
        entry.set_no_execute(true);
        entry.set_user_accessible(true);
    }
    //add a non-accessible page to catch stack overflows
    let overflow_page = top_page - stack_reserve_pages + 1;
    println!("allocating stack overflow page at {:#X}", overflow_page << 12);
    mem_tree.allocate_set_virtual(None, VirtAddr(overflow_page << 12));
    let entry = mem_tree.get_page_table_entry_mut(VirtAddr(overflow_page << 12)).unwrap();
    entry.set_writeable(true);
    entry.set_no_execute(true);
    entry.set_user_accessible(false);

    let stack = Stack {
        stack_base: VirtAddr((top_page << 12) + 0x1000 - 16),
        size_pages: stack_size_pages,
    };
    context.stacks.push(stack);
}

pub fn build_generic_memory_tree() -> PageTree {
    let page_tree_root = memory::physical_allocator::allocate_frame();
    unsafe { memset_physical_addr(page_tree_root, 0x0, 0x1000) };
    let mut new_page_tree = PageTree::new(page_tree_root);

    let existing_page_tree = PageTree::new(PageTree::get_level4_addr());
    existing_page_tree.copy_higher_half(&mut new_page_tree);

    new_page_tree
}
