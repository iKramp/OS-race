use std::{mem_utils::{self, VirtAddr}, vec};

use crate::{
    memory::{self, paging::PageTree},
    proc::{MemoryContext, Stack},
};

use super::info::ContextInfo;

const DEFAULT_STACK_SIZE: usize = 0x1000; // 4KB
//contains...?
const KERNEL_DATA_SIZE_PAGES: usize = 0;

pub fn build_generic_memory_context(context: ContextInfo) -> MemoryContext {
    let mut memory_tree = build_generic_memory_tree();

    // map memory regions
    for region in context.mem_regions().iter() {
        //we assume memory regions don't overlap or use same pages
        let start = region.start().0 & !0xFFF;
        let end = start + region.size_pages() as u64 * 0x1000;
        for i in (start..end).step_by(0x1000) {
            memory_tree.allocate_set_virtual(None, VirtAddr(i as u64));
            let page = memory_tree.get_page_table_entry_mut(VirtAddr(i as u64)).unwrap();
            page.set_writeable(region.flags().is_writeable());
            page.set_no_execute(!region.flags().is_executable());
        }
    }
    for init in context.mem_init().iter() {
        //change from virtual in process's address space to virtual in kernel's space
        let start_physical = mem_utils::translate_virt_phys_addr(init.0, memory_tree.root()).unwrap();
        let start_virtual = mem_utils::translate_phys_virt_addr(start_physical);

        unsafe {
            core::ptr::copy_nonoverlapping(init.1.as_ptr(), start_virtual.0 as *mut u8, init.1.len());
        }
    }


    MemoryContext {
        is_32_bit: context.is_32_bit(),
        page_tree: memory_tree,
        default_stack_size_pages: context.stack_size_pages().unwrap_or(DEFAULT_STACK_SIZE as u8),
        stacks: vec![],
    }
}

pub fn build_mem_context_for_new_proc(context: ContextInfo) -> MemoryContext {
    let mut generic_context = build_generic_memory_context(context);
    let stack_size = generic_context.default_stack_size_pages;

    //add stack
   add_stack(&mut generic_context, stack_size);
   generic_context

}

pub fn add_stack(context: &mut MemoryContext, stack_size_pages: u8) {
    let highest_userspace_addr = if context.is_32_bit {
        //highest address is 0xFFFF_FFFF, highest quarter is kernel on 32 bit applications
        //The kernel here will STILL be in higher half of 64(48) bit address space, but maybe
        //applications assume their address can't be in the highest qurter
        0xBFFF_FFFF
    } else {
        //48 bits for addressing, so highest userspace addr is 0x7FFF_FFFF_FFFF
        0x7FFF_FFFF_FFFF
    };

    let stack_reserve_pages = stack_size_pages + 1;
    let mem_tree = &mut context.page_tree;
    let stack_search_page = highest_userspace_addr >> 12;

    'top_loop: for top_page in (0..stack_search_page).rev() {
        for page in (top_page - stack_reserve_pages + 1)..=top_page {
            if mem_tree.get_page_table_entry_mut(VirtAddr((page as u64) << 12)).is_some() {
                //found a page that is already mapped, so we can't use this address
                continue 'top_loop;
            }
        }
        //found a free stack at this address
        for page in (top_page - stack_reserve_pages + 2)..=top_page {
            mem_tree.allocate_set_virtual(None, VirtAddr((page as u64) << 12));
            let entry = mem_tree.get_page_table_entry_mut(VirtAddr((page as u64) << 12)).unwrap();
            entry.set_writeable(true);
            entry.set_no_execute(true);
            entry.set_user_accessible(true);
        }
        //add a non-mapped page to catch stack overflows
        mem_tree.allocate_set_virtual(None, VirtAddr((top_page - stack_reserve_pages + 1) as u64));
        let entry = mem_tree.get_page_table_entry_mut(VirtAddr((top_page - stack_reserve_pages + 1) as u64)).unwrap();
        entry.set_writeable(false);
        entry.set_no_execute(true);
        entry.set_user_accessible(false);

        let stack = Stack {
            stack_base: VirtAddr((top_page as u64) << 12),
            size_pages: stack_size_pages,
        };
        context.stacks.push(stack);
    }
}

pub fn build_generic_memory_tree() -> PageTree {
    let page_tree_root = memory::physical_allocator::allocate_frame();
    let mut new_page_tree = PageTree::new(page_tree_root);

    let existing_page_tree = PageTree::new(PageTree::get_level4_addr());
    existing_page_tree.copy_higher_half(&mut new_page_tree);

    new_page_tree
}
