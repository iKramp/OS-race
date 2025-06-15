use core::mem::MaybeUninit;
use std::{boxed::Box, mem_utils::VirtAddr, vec::Vec};

use crate::{
    acpi::platform_info::PlatformInfo,
    interrupts::{self, idt::TablePointer},
    memory::stack::{KERNEL_STACK_SIZE_PAGES, prepare_kernel_stack},
    proc::Pid,
};

pub static mut CPU_LOCALS: MaybeUninit<Box<[VirtAddr]>> = MaybeUninit::uninit();

#[repr(C)]
pub struct CpuLocals {
    //keep this as first argument for syscall reasons
    pub kernel_stack_base: VirtAddr,
    pub stack_size_pages: u64,
    /// Points to TablePointer with base and limit of GDT
    pub gdt_ptr: TablePointer,
    pub current_process: Pid,
    pub apic_id: u8,
    pub processor_id: u8,
}

pub fn init(platform_info: &PlatformInfo) {
    let num_cpus = platform_info.application_processors.len() + 1;
    #[allow(clippy::slow_vector_initialization)] //it's non const ffs
    let mut vec = Vec::with_capacity(num_cpus);
    vec.resize(num_cpus, VirtAddr(0));
    unsafe { CPU_LOCALS = MaybeUninit::new(vec.into_boxed_slice()) }

    let bsp_stack_ptr = prepare_kernel_stack(KERNEL_STACK_SIZE_PAGES);
    let bsp_gdt = interrupts::create_new_gdt(bsp_stack_ptr);
    interrupts::load_gdt(bsp_gdt);
    let bsp_local = super::cpu_locals::CpuLocals::new(
        bsp_stack_ptr,
        KERNEL_STACK_SIZE_PAGES as u64,
        platform_info.boot_processor.apic_id,
        platform_info.boot_processor.processor_id,
        bsp_gdt,
    );
    let bsp_local_ptr = add_cpu_locals(bsp_local);
    crate::msr::set_msr(0xC0000101, bsp_local_ptr.0);
}

pub fn add_cpu_locals(locals: super::cpu_locals::CpuLocals) -> VirtAddr {
    unsafe {
        let apic_id = locals.apic_id;
        let cpu_locals = CPU_LOCALS.assume_init_mut();
        let ptr = std::Box::leak(std::Box::new(locals)) as *const _ as u64;
        cpu_locals[apic_id as usize] = VirtAddr(ptr);
        VirtAddr(&cpu_locals[apic_id as usize] as *const _ as u64)
    }
}

impl CpuLocals {
    pub fn new(kernel_stack_base: VirtAddr, stack_size_pages: u64, apic_id: u8, processor_id: u8, gdt_ptr: TablePointer) -> Self {
        Self {
            kernel_stack_base,
            stack_size_pages,
            apic_id,
            processor_id,
            gdt_ptr,
            current_process: Pid(0),
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
