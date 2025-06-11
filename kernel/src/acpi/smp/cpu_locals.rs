use std::mem_utils::VirtAddr;

use crate::interrupts::idt::TablePointer;

#[repr(C)]
pub struct CpuLocals {
    //keep this as first argument for syscall reasons
    pub kernel_stack_base: VirtAddr,
    pub stack_size_pages: u64,
    /// Points to TablePointer with base and limit of GDT
    pub gdt_ptr: TablePointer,
    pub current_process: u32,
    pub apic_id: u8,
    pub processor_id: u8,
}

impl CpuLocals {
    pub fn new(kernel_stack_base: VirtAddr, stack_size_pages: u64, apic_id: u8, processor_id: u8, gdt_ptr: TablePointer) -> Self {
        Self {
            kernel_stack_base,
            stack_size_pages,
            apic_id,
            processor_id,
            gdt_ptr,
            current_process: u32::MAX,
        }
    }

    pub fn get() -> &'static mut Self {
        unsafe {
            let cpu_locals: *mut Self;
            core::arch::asm!(
                "mov {cpu_locals}, gs:0",
                cpu_locals = out(reg) cpu_locals
            );
            &mut *cpu_locals
        }
    }
}

//FS register contains thread local storage of a process
